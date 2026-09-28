# Compilation and optimization

The recommended entry point for the Cqlib 2.0 Python binding is **`cqlib.compile`**: a single call completes canonicalization, knowledge rule optimization, decomposition, optional device layout and SABRE routing, and target gate set translation, plus the native instruction lowering, fixed-point optimization and validation required by a strict device target.

---

## Common entry points

```python
from cqlib import Circuit
from cqlib.compile import CompileConfig, CompileMode, compile
from cqlib.compile.transform.layout import vf2_perfect_layout, sabre_layout
from cqlib.compile.transform.routing import route_sabre
from cqlib.compile.transform import KnowledgeRewriter, RewriteConfig
```

---

## Recommended compilation pipeline

```python
from cqlib import Circuit
from cqlib.compile import CompileMode, compile
from cqlib.device import Device

circuit = Circuit(3)
circuit.h(0)
circuit.cx(0, 2)

device = Device.line("line-3", 3)

result = compile(
    circuit,
    mode=CompileMode.enhanced(),
    device=device,
    target_basis=["H", "CX", "RZ"],
    seed=42,
)

print("changed:", result.changed)
for step in result.steps:
    if step.changed or not step.skipped:
        print(step.stage, step.name, step.reason)

compiled = result.circuit
```

The example above passes both `device` and `target_basis`, so `TopologyBasis` semantics are selected: the device is used for capacity, layout and routing, and the explicit gate set constrains the output, but the output is not promised to satisfy the native instruction capabilities of the device. Only a strict `Device` target configured with native capabilities runs the SABRE Pareto beam described below, and only in enhanced mode.

---

## Step-by-step pipeline debugging

When layout, routing or rule optimization needs to be observed separately, the calls can be split apart:

```python
from cqlib.compile.transform.layout import LayoutObjective, vf2_perfect_layout
from cqlib.compile.transform.routing import route_sabre
from cqlib.compile.sabre import SabreConfig

objective = LayoutObjective.topology_only()
config = SabreConfig.deterministic_seeded(42)

# 1) layout only (no SWAP insertion)
layout_result = vf2_perfect_layout(circuit, device, objective)

# 2) layout + routing (with SWAP insertion)
route_result = route_sabre(circuit, device, objective, config)
print("swap_count:", route_result.swap_count)
```

---

## Compilation goals

1. Adapt the logical circuit to the target device topology (layout + routing, inserting SWAPs when necessary);
2. Reduce the number of two-qubit gates, the circuit depth and redundant single-qubit gates;
3. Lower the gate sequence to the target native gate set.

---

## Pipeline overview

```text
Logical circuit (Circuit)
  ->canonicalize.input
  ->decompose.definitions
  ->optimize.pre_decomposition
  ->decompose.unitary / decompose.mc_gates
  ->canonicalize.after_decomposition
  ->optimize.post_decomposition
  -> decompose.routing_basis                        [physical target]
  -> route.sabre                                    [physical target]
  -> post-routing resynthesis / cleanup             [Enhanced + physical target]
  -> translate.target_basis / target cleanup        [explicit target gate set]
  ->canonicalize.output
  -> lower.device_instructions                      [strict Device target]
  -> native-input canonicalization / fixed point    [strict Device target]
  -> validate.device / validate.topology            [by target type]
  -> select.sabre_pareto_beam                       [Enhanced + strict Device target]
  ->result.circuit
```

`canonicalize.output` only ends the tidying of the generic output representation. For a strict `Device` target,
exact native instruction lowering, `optimize.native_fixed_point` and `validate.device` still run afterwards.

### Pareto selection for enhanced strict devices

Enhanced strict device compilation saves an immutable pre-routing prefix before the first routing, and takes the
ordinary complete compilation result as candidate 0:

```text
Save the pre-routing prefix
  -> candidate 0: route + full suffix + validate.device
  ->bounded SABRE Pareto beam
      -> each explored candidate starts from the same prefix
      -> route + the same full suffix + validate.device
  ->select.sabre_pareto_beam
      ├─ an improved candidate satisfying the contract exists: select the validated winner
      └─ otherwise: keep the validated candidate 0
  ->result.circuit
```

A candidate must satisfy the exact Pareto contract in every control-flow scope and must strictly improve either
the native two-qubit gate count or the depth before it can replace candidate 0. Therefore, `selection` appearing
after `validation` does not mean the output bypassed validation: selection does not transform the circuit again,
it only decides between candidates that have already completed the same validation suffix.

On a successful return, `result.steps` records the common prefix, the finally kept baseline or winner path, and
the trailing `select.sabre_pareto_beam`; it is not a complete execution log of every explored branch. The number
of candidate attempts, the number of discards, the first recoverable error, and the quality summaries of
candidate 0 and the winner are recorded in the `reason` of the selection.

---

## CompileMode

| Mode | Description |
|------|------|
| `CompileMode.normal()` | Predictable production default: conservative rewrite budget and SABRE trials |
| `CompileMode.enhanced()` | Higher rewrite, resynthesis, native fixed-point optimization and SABRE search budgets, plus post-routing / target gate set cleanup; a strict `Device` target also runs a bounded SABRE Pareto beam across validated candidates |

---

## CompileConfig essentials

| Parameter | Where it is set | Purpose |
|------|---------|------|
| `mode` | `CompileConfig` | `normal` / `enhanced` |
| `target` | `CompileConfig` | Target constraint; see [`CompileTarget`](../../../../api/en/python/4_compile/1_compiler.md) |
| `resource_policy` | `CompileConfig` | Ancilla policy for the decomposition stage |
| `device` | `compile()` | Usable qubits, topology, native gates and optional calibration; used alone it selects a strict `Device` target |
| `target_basis` | `compile()` | Explicit target gate set; used together with `device` it selects `TopologyBasis`, which does not promise device native compatibility and does not run the Pareto beam |
| `initial_layout` | `compile()` or a `CompileTarget` constructor | Skip automatic layout and use the given mapping directly for SABRE routing |
| `seed` | `compile()` or a `CompileTarget` constructor | Random trials for heuristic layout / routing |

---

## CompileConfig and CompilerWorkflow

To compile multiple circuits with the same configuration, use `CompilerWorkflow`:

```python
from cqlib import Circuit
from cqlib.compile import CompileConfig, CompileMode, CompileTarget, CompilerWorkflow
from cqlib.device import Device

circuits = []
c1 = Circuit(2)
c1.cx(0, 1)
circuits.append(c1)

c2 = Circuit(3)
c2.h(0)
c2.cx(0, 2)
circuits.append(c2)

config = CompileConfig(
    mode=CompileMode.enhanced(),
    target=CompileTarget.topology_basis(
        Device.line("line-8", 8),
        ["H", "CX", "RZ"],
        seed=42,
    ),
)
workflow = CompilerWorkflow(config)

for circuit in circuits:
    result = workflow.run(circuit)
    print(result.changed, len(result.circuit.operations))
```

---

## Decomposition and resource policy

The `decompose.definitions` / `decompose.unitary` / `decompose.mc_gates` stages in the workflow can also be called separately (useful when debugging multi-controlled gate decomposition):

```python
from cqlib.compile.resource import ResourcePolicy
from cqlib.compile.transform.decompose import (
    decompose_mc_gates_for_device,
    decompose_unitaries,
)

# multi-controlled gate decomposition (constrained by device capacity)
result = decompose_mc_gates_for_device(
    circuit,
    device,
    resource_policy=ResourcePolicy(max_pre_layout_clean_ancillas=2),
)

# matrix unitary gate decomposition
unitary_result = decompose_unitaries(circuit)
```

`ResourcePolicy` controls the number of **clean ancillas** the compiler may create; the **hard capacity** of the device is determined by `device.num_usable_qubits`, and the two are independent.

---

## Next steps

- [Initial layout (Layout)](1_layout.md): learn initial mapping algorithms such as VF2, greedy and sabre_layout.
- [SABRE routing mapping](2_sabre_mapping.md): learn how heuristic SWAPs route a circuit onto the device topology.
- [Template matching and knowledge rule optimization](3_template_optimization.md): master the local optimization capabilities of `compile()` and `KnowledgeRewriter`.
- [Commutation and Clifford-RZ optimization](4_commutative_and_clifford.md): understand how commutation checking supports rotation merging and rule reordering.
