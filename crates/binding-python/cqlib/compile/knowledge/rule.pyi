# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

"""Rule model and equivalence-verification type contracts."""

from __future__ import annotations
from collections.abc import Sequence
from typing import Literal
from cqlib.circuit import Instruction, MCGate, Parameter, StandardGate

class RuleItem:
    """One gate-like item in a match or rewrite block.

    Parameters already bound to ``gate`` are preserved. Qubit integers are
    rule-local labels rather than concrete circuit qubits. Returned collection
    properties are copies and cannot mutate the native item.

    Example::

        theta = Parameter("theta")
        item = RuleItem.standard(StandardGate.RZ(theta), [0])
        item.validate()
        assert item.symbols() == ["theta"]
    """
    @staticmethod
    def standard(gate: StandardGate, qubits: Sequence[int]) -> RuleItem:
        """Create a standard-gate item in instruction operand order."""
        ...
    @staticmethod
    def mc_gate(gate: MCGate, qubits: Sequence[int]) -> RuleItem:
        """Create a multi-controlled item; control labels precede targets."""
        ...
    @property
    def instruction(self) -> Instruction:
        """Gate instruction without the separately owned parameter values."""
        ...
    @property
    def qubits(self) -> list[int]:
        """Rule-local qubit labels in operand order."""
        ...
    @property
    def params(self) -> list[Parameter]:
        """Fixed and symbolic parameters represented uniformly as expressions."""
        ...
    def symbols(self) -> list[str]:
        """Return free-symbol names in deterministic sorted order."""
        ...
    def validate(self) -> None:
        """Check instruction support, arities, and duplicate labels.

        Raises:
            ValueError: If any item-level invariant is violated.
        """
        ...
    def equivalent_to(self, other: RuleItem) -> bool:
        """Compare instruction, labels, and provably equal parameter expressions."""
        ...
    def __copy__(self) -> RuleItem: ...
    def __deepcopy__(self, memo: dict) -> RuleItem: ...

class Condition:
    """A symbolic requirement evaluated after a match has bound parameters."""
    @staticmethod
    def equal(lhs: Parameter, rhs: Parameter) -> Condition:
        """Require two expressions to be provably equal."""
        ...
    @staticmethod
    def equal_mod(lhs: Parameter, rhs: Parameter, modulus: Parameter) -> Condition:
        """Require ``lhs == rhs`` modulo ``modulus``."""
        ...
    @property
    def kind(self) -> Literal["equal", "equal_mod"]: ...
    @property
    def lhs(self) -> Parameter: ...
    @property
    def rhs(self) -> Parameter: ...
    @property
    def modulus(self) -> Parameter | None:
        """The modulus for modular equality, otherwise ``None``."""
        ...
    def symbols(self) -> list[str]:
        """Return referenced symbols in sorted order."""
        ...
    def __copy__(self) -> Condition: ...
    def __deepcopy__(self, memo: dict) -> Condition: ...

class VerifyResult:
    """Diagnostic result of layered semantic-equivalence verification.

    ``equivalent`` is a symbolic or deterministic proof. ``sampled_equal``
    records a successful sampling fallback. ``not_equivalent`` is a failed
    comparison. ``inconclusive`` means satisfying samples could not be built.
    """
    @property
    def status(
        self,
    ) -> Literal["equivalent", "sampled_equal", "not_equivalent", "inconclusive"]: ...
    @property
    def passed(self) -> bool:
        """Whether status is ``equivalent`` or ``sampled_equal``."""
        ...
    def is_verified(self) -> bool:
        """Return whether equivalence verification succeeded."""
        ...
    @property
    def num_bindings(self) -> int | None:
        """Accepted sample count for ``sampled_equal``."""
        ...
    @property
    def reason(self) -> str | None:
        """Explanation attached to an ``inconclusive`` result."""
        ...
    def __copy__(self) -> VerifyResult: ...
    def __deepcopy__(self, memo: dict) -> VerifyResult: ...

class Rule:
    """Runtime rewrite rule containing match, require, and target blocks.

    Args:
        name: Stable name, unique inside one rule library.
        operations: Non-empty adjacent pattern to match.
        target: Replacement sequence; empty means deletion.
        conditions: Optional constraints over symbols bound by ``operations``.

    Construction is intentionally separate from validation. Call
    :meth:`validate`, or add the rule to a library, before using it.
    """
    def __init__(
        self,
        name: str,
        operations: Sequence[RuleItem],
        target: Sequence[RuleItem],
        conditions: Sequence[Condition] | None = None,
    ) -> None: ...
    @property
    def name(self) -> str: ...
    @property
    def operations(self) -> list[RuleItem]:
        """Copies of match items in order."""
        ...
    @property
    def conditions(self) -> list[Condition]:
        """Copies of conditions; absent conditions produce an empty list."""
        ...
    @property
    def target(self) -> list[RuleItem]:
        """Copies of replacement items in emission order."""
        ...
    @property
    def num_qubits(self) -> int: ...
    def validate(self) -> None:
        """Validate all structural invariants required by matching.

        Raises:
            ValueError: For empty matches, bad arity, unsupported instructions,
                duplicate/non-dense labels, or unbound target/condition symbols.
        """
        ...
    def verify(self) -> VerifyResult:
        """Verify symbolically and automatically sample when required.

        Raises:
            ValueError: If circuit or matrix construction fails.
        """
        ...
    def verify_by_sampling(self, num_bindings: int, tolerance: float) -> VerifyResult:
        """Verify with explicit positive finite sampling settings.

        Raises:
            ValueError: For invalid settings or verification setup failures.
        """
        ...
    def needs_sampling_fallback(self) -> bool: ...
    def free_symbols(self) -> list[str]: ...
    def operation_qubits(self) -> list[int]: ...
    def target_qubits(self) -> list[int]: ...
    def __copy__(self) -> Rule: ...
    def __deepcopy__(self, memo: dict) -> Rule: ...

__all__ = [
    "RuleItem",
    "Condition",
    "Rule",
    "VerifyResult",
]
