# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
#
# Any modifications or derivative works of this code must retain this
# copyright notice, and modified files need to carry a notice indicating
# that they have been altered from the originals.

"""Knowledge-based local circuit rewrite type contracts."""

from __future__ import annotations

from collections.abc import Sequence

from cqlib.circuit import Circuit, Instruction
from cqlib.compile.knowledge import RuleKind

class RewriteMode:
    """High-level knowledge-rule application mode."""

    @staticmethod
    def optimize() -> RewriteMode:
        """Return conservative optimization mode."""
        ...
    @staticmethod
    def lowering() -> RewriteMode:
        """Return explicit lowering mode."""
        ...
    @property
    def name(self) -> str:
        """Stable lowercase mode name."""
        ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __copy__(self) -> RewriteMode: ...
    def __deepcopy__(self, memo: dict[int, object]) -> RewriteMode: ...

class RewriteConfig:
    """Immutable configuration for knowledge-based local circuit rewrite.

    When ``mode`` is omitted or is :meth:`RewriteMode.optimize`, construction
    starts from production defaults. Lowering mode starts from lowering
    defaults, including decomposition and hardware-native rule categories.
    Explicit ``enabled_kinds`` replace the selected preset categories.
    """

    def __init__(
        self,
        *,
        max_rounds: int = 8,
        max_window_ops: int = 16,
        max_pattern_len: int = 8,
        recurse_control_flow: bool = True,
        skip_labeled_ops: bool = True,
        preserve_two_qubit_connectivity: bool = False,
        enabled_kinds: Sequence[RuleKind] | None = None,
        mode: RewriteMode | None = None,
        target_instructions: Sequence[Instruction] | None = None,
    ) -> None:
        """Create a rewrite configuration.

        Raises:
            CompilerConfigError: If the target basis is empty or contains a
                non-gate instruction.
        """
        ...
    @staticmethod
    def production() -> RewriteConfig:
        """Return conservative production defaults."""
        ...
    @staticmethod
    def lowering() -> RewriteConfig:
        """Return explicit lowering defaults."""
        ...
    @property
    def max_rounds(self) -> int: ...
    @property
    def max_window_ops(self) -> int: ...
    @property
    def max_pattern_len(self) -> int: ...
    @property
    def recurse_control_flow(self) -> bool: ...
    @property
    def skip_labeled_ops(self) -> bool: ...
    @property
    def preserve_two_qubit_connectivity(self) -> bool:
        """Whether rewrites must avoid introducing new two-qubit pairs."""
        ...
    @property
    def enabled_kinds(self) -> list[RuleKind]:
        """Copies of the enabled rule categories in selection order."""
        ...
    @property
    def mode(self) -> RewriteMode: ...
    @property
    def target_instructions(self) -> list[Instruction] | None:
        """A copy of the deduplicated target basis, if configured."""
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> RewriteConfig: ...
    def __deepcopy__(self, memo: dict[int, object]) -> RewriteConfig: ...

class KnowledgeRewriteStats:
    """Aggregate statistics produced by one rewrite run."""

    @property
    def rounds_executed(self) -> int: ...
    @property
    def rules_applied(self) -> int: ...
    @property
    def changed_sequences(self) -> int: ...
    @property
    def reached_fixpoint(self) -> bool: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> KnowledgeRewriteStats: ...
    def __deepcopy__(self, memo: dict[int, object]) -> KnowledgeRewriteStats: ...

class KnowledgeRewriteResult:
    """Rewritten circuit and fixed-point run metadata."""

    @property
    def circuit(self) -> Circuit: ...
    @property
    def changed(self) -> bool: ...
    @property
    def stats(self) -> KnowledgeRewriteStats: ...
    def __repr__(self) -> str: ...
    def __copy__(self) -> KnowledgeRewriteResult: ...
    def __deepcopy__(self, memo: dict[int, object]) -> KnowledgeRewriteResult: ...

class KnowledgeRewriter:
    """Configurable local rewriter using Cqlib's built-in knowledge rules."""

    def __init__(self, config: RewriteConfig | None = None) -> None:
        """Create a rewriter, using production defaults when omitted."""
        ...
    @staticmethod
    def production() -> KnowledgeRewriter: ...
    @staticmethod
    def lowering() -> KnowledgeRewriter: ...
    @property
    def config(self) -> RewriteConfig: ...
    def run(self, circuit: Circuit) -> KnowledgeRewriteResult:
        """Rewrite ``circuit`` without modifying it.

        Raises:
            CompilerConfigError: If configuration or target-basis validation
                fails.
            CircuitError: If circuit rebuilding fails.
        """
        ...
    def __repr__(self) -> str: ...
    def __copy__(self) -> KnowledgeRewriter: ...
    def __deepcopy__(self, memo: dict[int, object]) -> KnowledgeRewriter: ...

def rewrite_circuit(
    circuit: Circuit,
    config: RewriteConfig | None = None,
) -> KnowledgeRewriteResult:
    """Rewrite a circuit using production defaults or an explicit config.

    Raises:
        CompilerConfigError: If configuration or target-basis validation
            fails.
        CircuitError: If circuit rebuilding fails.
    """
    ...

__all__ = [
    "RewriteMode",
    "RewriteConfig",
    "KnowledgeRewriter",
    "KnowledgeRewriteStats",
    "KnowledgeRewriteResult",
    "rewrite_circuit",
]
