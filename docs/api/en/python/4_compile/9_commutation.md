# Commutation

`cqlib.compile.commutation` provides conservative proofs for the exchange of quantum operations. This module proves whether the semantics of a circuit are preserved after two concrete `ValueOperation` objects are exchanged. Returning `None` means the configured proof sources cannot establish commutation; it does not prove that the two operations do not commute.

## Import

```python
from cqlib.compile.commutation import (
    Commutation,
    CommutationConfig,
    CommutationChecker,
    check_commutation,
    algebraic_commutation,
)
```

---

## Functions

### check_commutation(lhs, rhs) -> Commutation | None

Use the shared built-in checker to decide whether two self-contained operations commute.

### algebraic_commutation(lhs, rhs) -> Commutation | None

Decide commutation using only the symbolic algebra proof sources.

Example:

```python
from cqlib import Circuit, Qubit
from cqlib.circuit import StandardGate, ValueOperation
from cqlib.compile.commutation import check_commutation

lhs = ValueOperation.from_standard_gate(StandardGate.H, [Qubit(0)])
rhs = ValueOperation.from_standard_gate(StandardGate.X, [Qubit(1)])

proof = check_commutation(lhs, rhs)
assert proof is not None and proof.is_exact()
```

---

## Commutation

Commutation proof result.

### Static methods

- `Commutation.exact()`: an exact commutation proof.
- `Commutation.up_to_global_phase(phase)`: a commutation proof up to a global phase.

### Attributes and methods

- `phase -> Parameter`: the phase carried by the proof.
- `is_exact() -> bool`: whether the commutation is exact.

---

## CommutationConfig

Commutation checker configuration.

### CommutationConfig(*, enable_rule_oracle=true, enable_matrix_fallback=true, max_matrix_qubits=4)

Parameters:

- `enable_rule_oracle` (`bool`): enable the rule oracle (the commute rules from the knowledge library).
- `enable_matrix_fallback` (`bool`): enable the matrix fallback.
- `max_matrix_qubits` (`int`): the maximum number of qubits allowed for the matrix fallback.

### Attributes

`enable_rule_oracle`, `enable_matrix_fallback`, `max_matrix_qubits`, corresponding one-to-one with the constructor parameters.

---

## CommutationChecker

Configurable commutation checker.

### Static methods

- `CommutationChecker.builtin()`: built-in rules and the default configuration.
- `CommutationChecker.with_config(config)`: built-in rules and an explicit configuration.
- `CommutationChecker.from_library(library, config=None)`: use the commute rules of a loaded rule library (that is, the `RuleKind.commute()` category) in proofs. `config=None` uses the default configuration, consistent with `builtin()`. Raises `ValueError` when `enable_rule_oracle` is disabled; otherwise the rule library is silently ignored.

### Attributes and methods

- `config -> CommutationConfig`: a copy of the current configuration.
- `check(lhs, rhs) -> Commutation | None`: decide whether two self-contained operation values commute.

---

## Notes

- Proof results are conservative: `None` only means that a proof cannot be established, not that the operations do not commute.
- Operations related to classical control flow are outside the proof scope of this checker.
- Operations with symbolic parameters keep their symbolic parameters unchanged when participating in a proof.
