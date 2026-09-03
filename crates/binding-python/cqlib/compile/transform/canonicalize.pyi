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

from __future__ import annotations

from cqlib.circuit import Circuit

class CanonicalizeConfig:
    """Configuration for representation-level circuit canonicalization.

    All options are immutable after construction. Production defaults recurse
    through classical control flow, fold scope-local global phase, normalize
    instruction and barrier forms, and remove strict no-ops.
    """

    def __init__(
        self,
        *,
        round_limit: int = 8,
        recurse_control_flow: bool = True,
        fold_gphase: bool = True,
        canonicalize_instruction_form: bool = True,
        drop_noops: bool = True,
        canonicalize_barriers: bool = True,
    ) -> None:
        """Create a canonicalization configuration."""
        ...
    @staticmethod
    def production() -> CanonicalizeConfig:
        """Return the production canonicalization configuration."""
        ...
    @property
    def round_limit(self) -> int:
        """Maximum number of canonicalization rounds."""
        ...
    @property
    def recurse_control_flow(self) -> bool:
        """Whether control-flow bodies are recursively canonicalized."""
        ...
    @property
    def fold_gphase(self) -> bool:
        """Whether ``GPhase`` operations are folded into scope-local phase."""
        ...
    @property
    def canonicalize_instruction_form(self) -> bool:
        """Whether multi-controlled instructions use canonical standard forms."""
        ...
    @property
    def drop_noops(self) -> bool:
        """Whether strict no-op operations are removed."""
        ...
    @property
    def canonicalize_barriers(self) -> bool:
        """Whether barrier scopes are normalized and adjacent barriers merged."""
        ...
    def __repr__(self) -> str:
        """Return a self-documenting configuration representation."""
        ...
    def __eq__(self, other: object) -> bool:
        """Return whether two configurations contain the same options."""
        ...
    def __copy__(self) -> CanonicalizeConfig:
        """Return a shallow copy of this configuration."""
        ...
    def __deepcopy__(self, memo: dict) -> CanonicalizeConfig:
        """Return a deep copy of this configuration."""
        ...

class Canonicalizer:
    """Configurable circuit canonicalizer.

    Canonicalization rebuilds a stable logical representation. It does not
    perform approximate optimization, decomposition, routing, or target-basis
    lowering.
    """

    def __init__(self, config: CanonicalizeConfig | None = None) -> None:
        """Create a canonicalizer, using production defaults when omitted."""
        ...
    @staticmethod
    def production() -> Canonicalizer:
        """Return a canonicalizer using production defaults."""
        ...
    @property
    def config(self) -> CanonicalizeConfig:
        """Configuration used by this canonicalizer."""
        ...
    def run(self, circuit: Circuit) -> CanonicalizeResult:
        """Canonicalize ``circuit`` without modifying it.

        Raises:
            CompilerConfigError: If the configuration is invalid or
                canonicalization does not reach its declared fixed point.
            CircuitError: If circuit rebuilding fails.
        """
        ...
    def __repr__(self) -> str:
        """Return a self-documenting canonicalizer representation."""
        ...
    def __copy__(self) -> Canonicalizer:
        """Return a shallow copy of this canonicalizer."""
        ...
    def __deepcopy__(self, memo: dict) -> Canonicalizer:
        """Return a deep copy of this canonicalizer."""
        ...

class CanonicalizeResult:
    """Canonicalized circuit and fixed-point run metadata."""

    @property
    def circuit(self) -> Circuit:
        """Canonicalized circuit owned by this result."""
        ...
    @property
    def changed(self) -> bool:
        """Whether the output representation differs from the input."""
        ...
    @property
    def rounds(self) -> int:
        """Number of canonicalization rounds executed."""
        ...
    def __repr__(self) -> str:
        """Return a compact result representation."""
        ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> CanonicalizeResult:
        """Return a shallow copy of this result."""
        ...
    def __deepcopy__(self, memo: dict) -> CanonicalizeResult:
        """Return a deep copy of this result."""
        ...

def canonicalize_circuit(circuit: Circuit) -> CanonicalizeResult:
    """Canonicalize a circuit using production defaults.

    Args:
        circuit: Logical input circuit. The function does not modify it.

    Returns:
        The canonical circuit and fixed-point metadata.

    Raises:
        CompilerConfigError: If canonicalization does not reach its declared
            fixed point.
        CircuitError: If circuit rebuilding fails.
    """
    ...

__all__ = [
    "CanonicalizeConfig",
    "Canonicalizer",
    "CanonicalizeResult",
    "canonicalize_circuit",
]
