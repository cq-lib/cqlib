# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

"""Validated rule-library indexes and runtime-rule DSL I/O."""

from __future__ import annotations
from collections.abc import Sequence
from os import PathLike
from cqlib.circuit import Instruction
from .rule import Rule

class RuleId:
    """Immutable insertion index local to one :class:`RuleLibrary`.

    An ID returned by one library must not be used with a different library,
    even when both IDs expose the same numeric ``index``.
    """
    @property
    def index(self) -> int: ...
    def __index__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __copy__(self) -> RuleId: ...
    def __deepcopy__(self, memo: dict) -> RuleId: ...

class RuleKind:
    """Immutable coarse use-case used by compiler rule selection."""
    @staticmethod
    def simplify() -> RuleKind: ...
    @staticmethod
    def cancel() -> RuleKind: ...
    @staticmethod
    def merge() -> RuleKind: ...
    @staticmethod
    def commute() -> RuleKind: ...
    @staticmethod
    def decompose() -> RuleKind: ...
    @staticmethod
    def canonicalize() -> RuleKind: ...
    @staticmethod
    def hardware_native() -> RuleKind: ...
    @staticmethod
    def other() -> RuleKind: ...
    @property
    def name(self) -> str:
        """Lowercase stable category name."""
        ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __copy__(self) -> RuleKind: ...
    def __deepcopy__(self, memo: dict) -> RuleKind: ...

class RuleMetadata:
    """Read-only precomputed rule selection and diagnostic metadata."""
    @property
    def id(self) -> RuleId: ...
    @property
    def kind(self) -> RuleKind: ...
    @property
    def pattern_len(self) -> int: ...
    @property
    def rewrite_len(self) -> int: ...
    @property
    def qubit_count(self) -> int: ...
    @property
    def first_instruction(self) -> Instruction: ...
    @property
    def cost_delta(self) -> int:
        """Static operation-count delta ``rewrite_len - pattern_len``."""
        ...
    @property
    def has_conditions(self) -> bool: ...
    def __copy__(self) -> RuleMetadata: ...
    def __deepcopy__(self, memo: dict) -> RuleMetadata: ...

class RuleLibrary:
    """Validated rule collection with stable IDs and selection indexes.

    Insertion is atomic: validation, duplicate-name, or indexing failure leaves
    the original library unchanged. All returned rules and metadata are copies.

    Example::

        library = RuleLibrary.from_dsl(
            "rule cancel_x { match { X 0, X 0 } rewrite {} }",
            RuleKind.cancel(),
        )
        rule_id = library.id_by_name("cancel_x")
        assert rule_id is not None
        assert library.metadata(rule_id).cost_delta == -2
    """
    def __init__(self) -> None:
        """Create an empty library."""
        ...
    @staticmethod
    def builtin() -> RuleLibrary:
        """Return an owned copy of all embedded, validated compiler rules."""
        ...
    @staticmethod
    def from_rules(rules: Sequence[Rule], kind: RuleKind) -> RuleLibrary:
        """Validate and index constructed rules under one kind.

        Raises:
            ValueError: If validation, duplicate checking, or indexing fails.
        """
        ...
    @staticmethod
    def from_dsl(source: str, kind: RuleKind) -> RuleLibrary:
        """Parse, validate, classify, and index DSL source rules."""
        ...
    @staticmethod
    def from_dsl_file(path: str | PathLike[str], kind: RuleKind) -> RuleLibrary:
        """Build a library from a DSL file.

        Raises:
            OSError: If reading fails.
            ValueError: If parsing, lowering, validation, or indexing fails.
        """
        ...
    def add_rule(self, rule: Rule, kind: RuleKind) -> RuleId:
        """Validate and append one rule, returning its assigned ID."""
        ...
    def extend_rules(self, rules: Sequence[Rule], kind: RuleKind) -> list[RuleId]:
        """Atomically validate and append multiple rules."""
        ...
    def rules(self) -> list[Rule]:
        """Return all rules in insertion order."""
        ...
    def get(self, id: RuleId) -> Rule | None:
        """Return a rule by library-local ID, or ``None`` if out of range."""
        ...
    def metadata(self, id: RuleId) -> RuleMetadata | None: ...
    def id_by_name(self, name: str) -> RuleId | None: ...
    def get_by_name(self, name: str) -> Rule | None: ...
    def candidates_for_first_instruction(self, instruction: Instruction) -> list[RuleId]:
        """Return rules indexed under an instruction's matcher key.

        Raises:
            ValueError: If the instruction is unsupported by knowledge matching.
        """
        ...
    def rules_by_kind(self, kind: RuleKind) -> list[RuleId]: ...
    def filter_rule_ids_by_instruction_keys(self, op_instructions: Sequence[Instruction], target_instructions: Sequence[Instruction]) -> list[RuleId]:
        """Select rules whose complete match and target instruction sets fit the supplied bases."""
        ...
    def __contains__(self, name: str) -> bool: ...
    def __len__(self) -> int: ...
    def __bool__(self) -> bool: ...
    def __copy__(self) -> RuleLibrary: ...
    def __deepcopy__(self, memo: dict) -> RuleLibrary: ...

def loads(source: str) -> list[Rule]:
    """Parse and lower runtime rules from DSL text.

    Rules contain ``match``, optional ``require``, and ``rewrite`` blocks.
    Parameters occur inside parentheses and rule-local qubit labels follow the
    gate, for example ``RZ(theta) 0``. ``//`` line comments are supported.

    Raises:
        ValueError: For syntax, lowering, arity, or duplicate-name errors.
    """
    ...
def load(path: str | PathLike[str]) -> list[Rule]:
    """Read and parse a DSL file.

    Raises:
        OSError: If reading fails.
        ValueError: If DSL parsing or lowering fails.
    """
    ...
def dumps(rule: Rule) -> str:
    """Serialize one runtime rule to canonical DSL text."""
    ...
def dump(rule_or_rules: Rule | Sequence[Rule], path: str | PathLike[str]) -> None:
    """Write one or more rules; multiple rules are separated by a blank line.

    Raises:
        TypeError: If the first argument is not a rule or rule sequence.
        OSError: If writing fails.
    """
    ...

__all__ = [
    "RuleId",
    "RuleKind",
    "RuleMetadata",
    "RuleLibrary",
    "loads",
    "load",
    "dumps",
    "dump",
]
