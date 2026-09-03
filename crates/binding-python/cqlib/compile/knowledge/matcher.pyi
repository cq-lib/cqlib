# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

"""High-level structural matcher contracts using self-contained operations."""

from __future__ import annotations
from collections.abc import Sequence
from cqlib.circuit import Parameter, Qubit, ValueOperation
from .rule import Condition, Rule, RuleItem

class MatchBindings:
    """Mutable qubit and symbolic-parameter bindings from matching.

    Rule-local qubit labels map one-to-one onto concrete qubits. Symbol names
    map to immutable parameter expressions. Dictionary properties are copies;
    :func:`match_rule_item` is the supported mutation operation.
    """
    def __init__(self) -> None: ...
    @property
    def qubits(self) -> dict[int, Qubit]: ...
    def qubit(self, rule_qubit: int) -> Qubit | None: ...
    @property
    def params(self) -> dict[str, Parameter]: ...
    def param(self, symbol: str) -> Parameter | None: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> MatchBindings: ...
    def __deepcopy__(self, memo: dict) -> MatchBindings: ...

def match_rule_item(item: RuleItem, operation: ValueOperation, bindings: MatchBindings) -> bool:
    """Match one item and transactionally update ``bindings``.

    Instruction identity, arity, one-to-one qubit mapping, and symbolic
    parameters must agree. ``False`` leaves bindings unchanged. Unsupported
    concrete operation kinds return ``False``.

    Raises:
        ValueError: If the rule item itself uses an unsupported instruction.

    Example::

        bindings = MatchBindings()
        if match_rule_item(item, operation, bindings):
            print(bindings.qubits, bindings.params)
    """
    ...
def conditions_hold(conditions: Sequence[Condition], bindings: MatchBindings) -> bool:
    """Return whether every condition is fully bound and provably satisfied."""
    ...
def instantiate_target(target: Sequence[RuleItem], bindings: MatchBindings) -> list[ValueOperation]:
    """Create self-contained replacement operations from target items.

    Output labels are ``None``. Fixed parameters remain numeric and symbolic
    expressions remain ``Parameter`` values.

    Raises:
        ValueError: For unsupported instructions or unbound labels/symbols.
    """
    ...
def rule_matches_operations(rule: Rule, operations: Sequence[ValueOperation]) -> MatchBindings | None:
    """Match a complete rule against one adjacent operation sequence.

    This helper performs no circuit search, commutation, cost comparison, or
    patching. The sequence length must equal the match-block length. ``None``
    means the pattern or its conditions did not match.

    Example::

        bindings = rule_matches_operations(rule, adjacent_operations)
        if bindings is not None:
            replacements = instantiate_target(rule.target, bindings)
    """
    ...

__all__ = [
    "MatchBindings",
    "match_rule_item",
    "conditions_hold",
    "instantiate_target",
    "rule_matches_operations",
]
