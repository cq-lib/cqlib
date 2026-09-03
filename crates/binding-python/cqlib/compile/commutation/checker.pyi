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

"""Gate-commutation proof types and checker interfaces."""

from __future__ import annotations

from cqlib.circuit import Parameter, ValueOperation
from cqlib.compile.knowledge import RuleLibrary

class Commutation:
    """A proven relationship between two commuting operations.

    An exact proof establishes ``lhs * rhs == rhs * lhs``. A global-phase
    proof establishes
    ``lhs * rhs == exp(1j * phase) * rhs * lhs``. The phase is represented by
    :class:`~cqlib.circuit.Parameter`, so a proof may retain symbolic values.

    Instances are normally returned by :func:`check_commutation`,
    :func:`algebraic_commutation`, or :meth:`CommutationChecker.check`.

    Example::

        proof = Commutation.up_to_global_phase(Parameter.pi())
        assert not proof.is_exact()
        assert proof.phase == Parameter.pi()
    """

    @staticmethod
    def exact() -> Commutation:
        """Create an exact commutation proof with zero global phase."""
        ...
    @staticmethod
    def up_to_global_phase(phase: Parameter) -> Commutation:
        """Create a proof that is valid up to ``exp(1j * phase)``.

        Args:
            phase: Symbolic or concrete global phase angle in radians.
        """
        ...
    def is_exact(self) -> bool:
        """Return whether this proof introduces no global phase."""
        ...
    @property
    def phase(self) -> Parameter:
        """Global phase angle; exact proofs return a constant zero parameter."""
        ...
    def __copy__(self) -> Commutation:
        """Return a shallow copy of this proof."""
        ...
    def __deepcopy__(self, memo: dict) -> Commutation:
        """Return a deep copy of this proof."""
        ...
    def __eq__(self, other: object) -> bool:
        """Return whether two proofs describe the same relationship."""
        ...

class CommutationConfig:
    """Configuration for :class:`CommutationChecker`.

    The checker always applies cheap structural facts and symbolic algebra.
    The options control the two later, potentially more expensive proof
    sources.

    Args:
        enable_rule_oracle: Match explicit exchange rules from the builtin
            compiler knowledge library.
        enable_matrix_fallback: Compare small concrete local matrices when
            earlier proof sources do not decide the query.
        max_matrix_qubits: Maximum size of the union of both operations'
            qubit supports for matrix fallback. This is not a per-operation
            limit.

    Example::

        config = CommutationConfig(
            enable_rule_oracle=False,
            enable_matrix_fallback=True,
            max_matrix_qubits=2,
        )
    """

    def __init__(
        self,
        *,
        enable_rule_oracle: bool = True,
        enable_matrix_fallback: bool = True,
        max_matrix_qubits: int = 4,
    ) -> None: ...
    @property
    def enable_rule_oracle(self) -> bool:
        """Whether builtin knowledge-library rules are enabled."""
        ...
    @property
    def enable_matrix_fallback(self) -> bool:
        """Whether concrete local-matrix comparison is enabled."""
        ...
    @property
    def max_matrix_qubits(self) -> int:
        """Maximum union-support size accepted by matrix fallback."""
        ...
    def __copy__(self) -> CommutationConfig:
        """Return a shallow copy of this configuration."""
        ...
    def __deepcopy__(self, memo: dict) -> CommutationConfig:
        """Return a deep copy of this configuration."""
        ...
    def __eq__(self, other: object) -> bool:
        """Return whether every configuration option is equal."""
        ...

class CommutationChecker:
    """Reusable conservative commutation checker for compiler passes.

    Proofs are attempted in this order: structural facts, symbolic algebra,
    optional builtin rules, and optional local-matrix comparison. The input
    operations must contain gate instructions with valid qubit and parameter
    arities and no repeated qubit within either operation.

    Unsupported instructions, malformed applications, and undecided queries
    return ``None`` rather than raising an exception.

    Example::

        from cqlib.circuit import Qubit, StandardGate, ValueOperation

        checker = CommutationChecker.with_config(
            CommutationConfig(enable_matrix_fallback=False)
        )
        lhs = ValueOperation.from_standard_gate(StandardGate.RZ(0.2), [Qubit(0)])
        rhs = ValueOperation.from_standard_gate(StandardGate.RZ(0.7), [Qubit(0)])
        proof = checker.check(lhs, rhs)
        assert proof is not None and proof.is_exact()
    """

    @staticmethod
    def builtin() -> CommutationChecker:
        """Build a checker using builtin rules and default configuration."""
        ...
    @staticmethod
    def with_config(config: CommutationConfig) -> CommutationChecker:
        """Build a checker using builtin rules and an explicit configuration."""
        ...
    @staticmethod
    def from_library(
        library: RuleLibrary, config: CommutationConfig | None = None
    ) -> CommutationChecker:
        """Build a checker using commute rules from an already loaded library.

        Only rules registered under ``RuleKind.commute()`` participate in
        proofs; builtin rules are not included unless the library contains
        them (see :meth:`RuleLibrary.builtin`). ``config=None`` uses the
        default configuration, matching :meth:`builtin`.

        Args:
            library: Rule library supplying the commutation rules.
            config: Checker configuration; defaults are applied when omitted.

        Returns:
            A checker whose rule oracle was built from ``library``.

        Raises:
            ValueError: If ``config.enable_rule_oracle`` is ``False``, because
                the library would otherwise be silently ignored.

        Example::

            from cqlib.circuit import Qubit, StandardGate, ValueOperation
            from cqlib.compile.commutation import CommutationChecker
            from cqlib.compile.knowledge import RuleKind, RuleLibrary

            library = RuleLibrary.from_dsl(
                "rule comm_s_t { match { S 0, T 0 } rewrite { T 0, S 0 } }",
                RuleKind.commute(),
            )
            checker = CommutationChecker.from_library(library)
            lhs = ValueOperation.from_standard_gate(StandardGate.S, [Qubit(0)])
            rhs = ValueOperation.from_standard_gate(StandardGate.T, [Qubit(0)])
            proof = checker.check(lhs, rhs)
            assert proof is not None and proof.is_exact()
        """
        ...
    @property
    def config(self) -> CommutationConfig:
        """Return a copy of the active checker configuration."""
        ...
    def check(
        self, lhs: ValueOperation, rhs: ValueOperation
    ) -> Commutation | None:
        """Prove whether two concrete operation applications commute.

        ``ValueOperation`` parameters are passed through without numeric
        evaluation: fixed values become constant parameters and symbolic
        expressions remain symbolic. Operation labels do not affect the proof.

        Returns:
            A proof when commutation is established, otherwise ``None``.
        """
        ...
    def __copy__(self) -> CommutationChecker:
        """Return an independent shallow copy of this checker."""
        ...
    def __deepcopy__(self, memo: dict) -> CommutationChecker:
        """Return an independent deep copy of this checker."""
        ...

def check_commutation(
    lhs: ValueOperation, rhs: ValueOperation
) -> Commutation | None:
    """Check commutation using the shared builtin checker.

    Args:
        lhs: First self-contained operation application.
        rhs: Second self-contained operation application.

    Returns:
        A proof when the operations can be exchanged safely. ``None`` means
        that commutation was not proven; it does not prove non-commutation.

    Example::

        from cqlib.circuit import Qubit, StandardGate, ValueOperation
        from cqlib.compile.commutation import check_commutation

        lhs = ValueOperation.from_standard_gate(StandardGate.H, [Qubit(0)])
        rhs = ValueOperation.from_standard_gate(StandardGate.X, [Qubit(1)])
        proof = check_commutation(lhs, rhs)
        assert proof is not None and proof.is_exact()
    """
    ...

def algebraic_commutation(
    lhs: ValueOperation, rhs: ValueOperation
) -> Commutation | None:
    """Check commutation using only the symbolic algebra oracle.

    This lower-level function bypasses knowledge-library rules and matrix
    comparison. It is useful when a compiler pass requires a cheap,
    deterministic symbolic proof source.

    Returns:
        An algebraic proof when supported, otherwise ``None``.
    """
    ...

__all__ = [
    "Commutation",
    "CommutationConfig",
    "CommutationChecker",
    "check_commutation",
    "algebraic_commutation",
]
