# Commutation and Clifford-RZ Optimization

Commutation analysis is used to determine whether two specific operations can exchange order. It does not necessarily reduce the gate count directly, but it creates the conditions for subsequent rule matching, rotation merging, cancellation of redundant gates and target gate set cleanup. Clifford-RZ optimization is common in circuit fragments that contain `H`, `S`, `X`, `Z`, `CX`, `CZ` together with phase gates such as `RZ`, `Phase` and `T`.

In everyday use, calling `compile()` directly is recommended. The compiler automatically applies commutation checks, canonicalization and knowledge rule rewriting at the appropriate stages. `CommutationChecker` needs to be used directly only when writing custom analysis or debugging optimization rules.

Neither `compile()` nor the commutation checking interfaces modify the input circuit; they return a new result object or proof object. A commutation check is only responsible for answering "whether these two operations can be exchanged safely", and is not an optimization step that directly rewrites an entire circuit.

---

## Commutation checking entry points

In Python, a commutation check takes `ValueOperation` objects as input. Each `ValueOperation` is a complete operation application, containing a gate, the qubits it acts on and parameters.

```python
from cqlib.circuit import Parameter, Qubit, StandardGate, ValueOperation
from cqlib.compile.commutation import CommutationChecker

lhs = ValueOperation.from_standard_gate(
    StandardGate.CX,
    [Qubit(0), Qubit(1)],
)
rhs = ValueOperation.from_standard_gate(
    StandardGate.RZ(Parameter("theta")),
    [Qubit(0)],
)

checker = CommutationChecker.builtin()
proof = checker.check(lhs, rhs)

if proof is not None:
    print("exact:", proof.is_exact())
    print("phase:", proof.phase)
```

A shared functional entry point is also available:

```python
from cqlib.compile.commutation import check_commutation

proof = check_commutation(lhs, rhs)
```

The return value may be:

- an exact commutation proof: the two operations can be exchanged exactly;
- a commutation proof with a global phase: the exchange introduces a global phase;
- `None`: the current checker cannot prove commutation, which is not equivalent to having proved non-commutation.

This conservative semantics is important. The compiler uses a commutation conclusion only when a proof exists, so that insufficient proof coverage does not break the circuit semantics.

---

## Check order

The built-in checker attempts proofs in a fixed order:

1. fast local facts: identity gates, global phases, gates acting on disjoint qubit sets, and identical operations;
2. algebraic proofs: Pauli axes, Pauli rotations, diagonal gates, controlled single-axis gates and some symmetric two-qubit gates;
3. rule library proofs: explicit exchange rules of the form `A; B -> B; A` extracted from the built-in knowledge rules;
4. small-scale matrix checks: within a qubit-count limit, local matrices are constructed to verify whether `AB` and `BA` are equal or differ only by a global phase.

The default configuration enables rule library proofs and the matrix fallback check, and limits the joint support of the matrix check to 4 qubits.

```python
from cqlib.compile.commutation import CommutationChecker, CommutationConfig

config = CommutationConfig(
    enable_rule_oracle=True,
    enable_matrix_fallback=True,
    max_matrix_qubits=4,
)

checker = CommutationChecker.with_config(config)
proof = checker.check(lhs, rhs)
```

If speed matters more in large-scale static analysis, the matrix fallback check can be turned off; if full reliance on structural rules is preferred, rule library proofs and matrix checks can both be turned off, keeping only the basic structural facts and the algebraic proofs.

---

## Common commutation relations

The compiler recognizes several classes of common relations:

- two gates acting on disjoint qubit sets always commute exactly;
- diagonal gates commute with each other, for example `RZ`, `Phase`, `S`, `T`, `CZ`, `RZZ`;
- rotations about the same axis commute, for example consecutive `RZ(a)` and `RZ(b)`;
- Pauli string rotations commute when the number of anticommuting positions is even;
- `CX` commutes with Z-axis operations on the control qubit;
- `CX` commutes with X-axis operations on the target qubit;
- `CZ` commutes with Z-axis operations on either endpoint;
- some two-qubit gate families such as `SWAP`, `FSIM` and symmetric Pauli rotations have additional structural commutation facts when applied to the same unordered qubit pair.

These conclusions help the rule rewriter move mergeable gates together. For example:

```text
RZ(a) q; S q; RZ(b) q
```

Since `RZ` and `S` both belong to the Z-axis phase family, this can be rearranged as:

```text
S q; RZ(a) q; RZ(b) q
```

The rotation merging rule can then merge the two `RZ` gates into `RZ(a + b)`.

---

## Optimization approach for Clifford-RZ fragments

Clifford-RZ circuits usually consist of alternating discrete Clifford gates and Z-axis rotations. The optimization goal is not simply to reorder all gates, but to perform local rearrangement while preserving semantics:

- cancel adjacent self-inverse gates, for example `H H`, `X X`, `CX CX`;
- cancel inverse gate pairs, for example `S SDG`, `T TDG`;
- merge same-axis rotations, for example `RZ(a) RZ(b) -> RZ(a + b)`;
- remove zero-angle rotations and identity gates;
- rewrite Clifford/phase combinations such as `S S` and `T T` into shorter forms;
- preserve or fold the necessary `GPhase` before and after target gate set translation.

These optimizations are handled by the canonicalizer and the knowledge rule rewriter respectively. They are generally used indirectly through `compile()`, so there is no need to look for a separate `CliffordRzOptimization` class.

```python
from cqlib.circuit import Circuit
from cqlib.compile import CompileMode, compile

circuit = Circuit(1)
circuit.rz(0, 0.1)
circuit.rz(0, 0.2)
circuit.rz(0, -0.3)

result = compile(circuit, mode=CompileMode.enhanced())
optimized = result.circuit

print("changed:", result.changed)
print("before:", len(circuit.operations))
print("after:", len(optimized.operations))
```

`CompileMode.enhanced()` uses higher budgets for rule search, resynthesis, native fixed-point optimization and SABRE search, and adds cleanup steps after routing and target gate set translation. For a strict `Device` target it also lets several routing candidates each complete the subsequent lowering, optimization and validation, and then selects the final result through a bounded Pareto beam. For circuits with many Clifford-RZ fragments, where new adjacent gates are easily exposed after routing, the enhanced mode usually finds additional merging or cancellation opportunities more easily.

---

## Interaction with template rules

The relationship between commutation analysis and template optimization can be understood as follows:

1. a commutation check proves that two adjacent operations can be exchanged;
2. after the exchange, a local pattern that was previously separated becomes an adjacent pattern;
3. the knowledge rule library matches that pattern;
4. the rewriter applies cancellation, merging or normalization rules;
5. the canonicalizer cleans up parameters, global phase and representation details.

For example, the commutation rules of the Z-axis gate family can arrange `RZ`, `S`, `T` and `Phase` into positions where merging or cancellation is easier. The commutation rules of `CX` and `CZ` can move single-qubit phase gates around a two-qubit gate, reducing redundancy in the subsequent target gate set translation.

---

## Global phase

Some exchanges or cancellations hold only up to a global phase. Cqlib represents such cases explicitly with the phase information of `Commutation` and with `GPhase`. A top-level `GPhase` is merged into the `global_phase` of the circuit during canonicalization; a phase inside a control flow body cannot be promoted to a global phase freely, so it is kept in the body representation.

In algorithm verification, statevector comparison and gate matrix comparison, it must be made explicit whether the global phase is ignored. For sampling probabilities, the global phase is usually unobservable; for controlled sub-circuits, phase estimation and scenarios where the fragment is further wrapped as a composite gate, it should be preserved with care.

---

## Usage recommendations

- for user-level compilation, prefer `compile()` and let the workflow schedule commutation, rule rewriting and canonicalization automatically;
- in custom analysis, use `CommutationChecker.builtin()` and treat `None` as "not proved" rather than "non-commuting";
- for batch analysis of large circuits, the matrix fallback check can be turned off to avoid the overhead of local matrix construction;
- after Clifford-RZ optimization, record the gate count, the two-qubit gate count, the circuit depth and the change in global phase;
- for critical circuits, it is recommended to use matrix equivalence or state equivalence tests to confirm that the semantics are unchanged before and after optimization.

---

## Next steps

- [Compilation and optimization](0_overview.md): review the complete `compile()` pipeline and the configuration essentials of each stage.
- [Template matching and knowledge rule optimization](3_template_optimization.md): learn how knowledge rule rewriting works together with commutation analysis.
