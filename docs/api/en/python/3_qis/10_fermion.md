# Fermionic Operators and Jordan–Wigner Mapping

`cqlib.qis.fermion` exposes general fermionic algebra and sparse mappings backed by Rust Core. Public types are also exported from `cqlib.qis`.

```python
from cqlib.qis import FermionicLadder, FermionicOp, JordanWignerMapper

a = FermionicOp.annihilation(num_modes=4, mode=0)
assert (a * a.adjoint() + a.adjoint() * a).equiv(FermionicOp.identity(4))
term = [FermionicLadder.create(2), FermionicLadder.annihilate(0)]
op = FermionicOp.from_term(num_modes=4, term=term, coefficient=0.5j)
hamiltonian = JordanWignerMapper().map_operator(op)
```

## Operators and Arithmetic

`FermionicLadder.create(mode)` and `annihilate(mode)` construct immutable ladders with read-only `mode` and `kind`, `adjoint()`, value equality and hashing. The operator containing a ladder validates its mode against the declared mode space.

`FermionicOp(num_modes)` constructs zero; `identity(num_modes)` constructs identity. `from_term(num_modes, term, coefficient=1.0)` accepts an ordered product. `from_terms(num_modes, terms)` accepts an iterable of `(term, coefficient)` tuples and merges duplicate products. An empty product is identity; an empty sum is zero. Both retain their declared mode count, including zero modes.

| Interface | Behavior |
| --- | --- |
| `creation` / `annihilation` / `number` | Arguments are `num_modes, mode`; construct creation, annihilation or mode-number operators. |
| `total_number(num_modes)` | Construct total particle number. |
| `A + B` / `A - B` / `-A` | Return new operators; addition and subtraction require matching mode counts. |
| `A * B` | Operator product AB: B acts first, without implicit normal ordering. |
| `c * A` / `A * c` / `A.scaled(c)` | Scale by a finite integer, float or complex coefficient. |
| `adjoint()` | Conjugate coefficients, reverse products and exchange ladder kinds. |
| `normal_order()` | Return a new expansion: creators first, ascending modes within each group, including contractions and signs. |
| `add_term(term, coefficient=1.0)` | Mutate atomically and return `None`. |
| `chop(*, atol=0.0)` | Remove coefficients with complex magnitude <= tolerance in place; return `None`. |
| `equiv(other, *, atol=0.0)` | Compare algebraically after normal ordering; mismatched mode counts raise an error. |
| `is_hermitian(*, atol=0.0)` | Compare algebraically with the adjoint. |

Read-only `num_modes` and `num_terms` describe storage. `terms` returns an independent snapshot of `(tuple[FermionicLadder, ...], complex)` pairs. Equality compares stored structure; `is_zero()` tests empty storage. Use `equiv()` for algebraic equality or zero.

Only exact zeros are removed by default. Tolerances must be finite and nonnegative. `copy()` and Python copy protocols create independent objects. Operators are mutable and unhashable. `A += B` rebinds the result without modifying aliases of A. Use `sum(operators, start=FermionicOp(num_modes))` to sum operators.

## Excitation Generators and Evolution

`excitation_generator(num_modes, sources, targets)` constructs G = i(T − T†), with target creators in input order followed by source annihilators in reverse input order. Thus `[0, 1] -> [2, 3]` means T = a†₂ a†₃ a₁ a₀. Lists must be nonempty, equally sized, individually unique and disjoint. No implicit sorting or factorial normalization is applied.

```python
from cqlib.circuit.ansatz import BasisEncoding, EvolutionStrategy, PauliEvolutionAnsatz

mapper = JordanWignerMapper()
circuit = BasisEncoding(mapper.map_occupation([1, 1, 0, 0])).build_circuit("hf")
for name, sources, targets in [
    ("theta_double", [0, 1], [2, 3]),
    ("theta_single", [0], [2]),
]:
    generator = FermionicOp.excitation_generator(4, sources, targets)
    block = (PauliEvolutionAnsatz(mapper.map_operator(generator))
             .with_strategy(EvolutionStrategy.exact())
             .with_time_param_name(name).build_circuit("excitation"))
    circuit.compose(block)
bound = circuit.assign_parameters({"theta_double": 0.2, "theta_single": -0.3})
```

Each block implements exp(−iθG) = exp(θ(T − T†)). Build and name separate blocks for independent parameters; evolving a sum of generators uses one shared parameter.

## Mappings and Extension

`JordanWignerMapper()` provides `map_operator(op) -> Hamiltonian`, `num_qubits(num_modes) -> int` and `map_occupation(occupation) -> list[bool]`. Operator mapping accepts non-Hermitian, identity and zero operators; downstream evolution and expectation consumers impose Hermiticity requirements.

Mode p maps to qubit p, with a Z chain on lower-numbered modes. Qubit 0 is the least significant bit. Occupations must be integer zero/one or booleans; returned `bits[p]` addresses qubit p. Empty input returns an empty list. Core specifies no chemical or alpha/beta orbital ordering.

`FermionToQubitMapper` and `OccupationStateMapper` are independent runtime-checkable `typing.Protocol` interfaces. Custom implementations need matching methods without inheriting a binding class. Runtime checks establish member presence only, not signatures or correctness. Algorithms call Python mappers directly; no automatic Rust trait callback bridge is provided.

Wrong types raise `TypeError`; negative or out-of-range modes raise `IndexError`. Negative mode counts, mismatched spaces, nonfinite coefficients, invalid occupations or tolerances raise `ValueError`. Mode integers outside the Rust integer range raise `OverflowError`.
