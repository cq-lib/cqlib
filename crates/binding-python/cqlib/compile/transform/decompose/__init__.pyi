# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

"""Circuit decomposition transforms and their configurations."""

from __future__ import annotations

from cqlib.circuit import Circuit, Instruction
from cqlib.compile.resource import ResourceLimits, ResourcePolicy
from cqlib.device import Device
from .. import TransformResult
from . import mc_gate as mc_gate
from . import unitary as unitary

class TwoQubitUnitaryDecomposeBasis:
    """Output interaction basis for numeric two-qubit unitary synthesis."""

    @staticmethod
    def pauli_rotations() -> TwoQubitUnitaryDecomposeBasis:
        """Emit local U gates plus RXX, RYY, and RZZ interactions."""
        ...
    @staticmethod
    def cx() -> TwoQubitUnitaryDecomposeBasis:
        """Emit local U gates plus optimized CX templates."""
        ...
    @staticmethod
    def cy() -> TwoQubitUnitaryDecomposeBasis:
        """Emit local U gates plus optimized CY templates."""
        ...
    @staticmethod
    def cz() -> TwoQubitUnitaryDecomposeBasis:
        """Emit local U gates plus optimized CZ templates."""
        ...
    @staticmethod
    def rzz() -> TwoQubitUnitaryDecomposeBasis:
        """Emit local U gates plus RZZ interactions for the Cartan core."""
        ...
    def __copy__(self) -> TwoQubitUnitaryDecomposeBasis: ...
    def __deepcopy__(
        self, memo: dict[int, object]
    ) -> TwoQubitUnitaryDecomposeBasis: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class UnitaryDecomposeConfig:
    """Configuration for matrix-backed unitary decomposition."""

    def __init__(
        self,
        *,
        two_qubit_basis: TwoQubitUnitaryDecomposeBasis | None = None,
        target_basis: list[Instruction | str] | None = None,
        recurse_control_flow: bool = True,
    ) -> None: ...
    @property
    def two_qubit_basis(self) -> TwoQubitUnitaryDecomposeBasis | None:
        """Basis used for synthesized two-qubit interaction gates.

        ``None`` when the config uses the unconstrained core default target.
        """
        ...
    @property
    def target_basis(self) -> list[Instruction] | None:
        """Explicit target basis used for two-qubit synthesis candidate choice.

        ``None`` when the config uses the unconstrained core default target or
        the legacy ``two_qubit_basis`` selection. Mutually exclusive with
        ``two_qubit_basis``.
        """
        ...
    @property
    def recurse_control_flow(self) -> bool:
        """Whether unitary gates in control-flow bodies are synthesized."""
        ...
    def __copy__(self) -> UnitaryDecomposeConfig: ...
    def __deepcopy__(self, memo: dict[int, object]) -> UnitaryDecomposeConfig: ...
    def __eq__(self, other: object) -> bool: ...

class McGateDecomposeConfig:
    """Configuration for resource-aware multi-controlled-gate decomposition."""

    def __init__(
        self,
        *,
        resource_policy: ResourcePolicy | None = None,
        resource_limits: ResourceLimits | None = None,
    ) -> None: ...
    @property
    def resource_policy(self) -> ResourcePolicy:
        """Copy of the ancillary-resource permissions."""
        ...
    @property
    def resource_limits(self) -> ResourceLimits:
        """Copy of the hard logical-qubit limits."""
        ...
    def __copy__(self) -> McGateDecomposeConfig: ...
    def __deepcopy__(self, memo: dict[int, object]) -> McGateDecomposeConfig: ...
    def __eq__(self, other: object) -> bool: ...

class DecompositionRuleStats:
    """Pass-local runtime decomposition-rule cache counters."""

    @property
    def hits(self) -> int:
        """Number of runtime-rule cache hits."""
        ...
    @property
    def misses(self) -> int:
        """Number of runtime-rule cache misses."""
        ...
    @property
    def inserts(self) -> int:
        """Number of runtime rules inserted during the pass."""
        ...
    def __copy__(self) -> DecompositionRuleStats: ...
    def __deepcopy__(self, memo: dict[int, object]) -> DecompositionRuleStats: ...
    def __eq__(self, other: object) -> bool: ...

def expand_definitions(circuit: Circuit) -> TransformResult:
    """Expand circuit-backed definitions without modifying the input.

    Raises:
        CompilerConfigError: If a definition is malformed or exceeds recursion
            limits.
        CircuitError: If circuit rebuilding fails.
    """
    ...

def decompose_unitaries(
    circuit: Circuit,
    config: UnitaryDecomposeConfig | None = None,
) -> TransformResult:
    """Synthesize matrix-backed one- and two-qubit unitary gates.

    Raises:
        CompilerConfigError: If a unitary is unresolved, invalid, or
            unsupported.
        CircuitError: If circuit rebuilding fails.
    """
    ...

def decompose_unitaries_with_rule_stats(
    circuit: Circuit,
    config: UnitaryDecomposeConfig | None = None,
) -> tuple[TransformResult, DecompositionRuleStats]: ...
def decompose_mc_gates(
    circuit: Circuit,
    config: McGateDecomposeConfig | None = None,
) -> TransformResult:
    """Decompose multi-controlled gates using configured resources.

    This entry point does not enforce device capacity. Use
    :func:`decompose_mc_gates_for_device` when decomposition must fit a device.

    Raises:
        CompilerConfigError: If decomposition or ancillary-resource validation
            fails.
        CircuitError: If circuit rebuilding fails.
    """
    ...

def decompose_mc_gates_with_rule_stats(
    circuit: Circuit,
    config: McGateDecomposeConfig | None = None,
) -> tuple[TransformResult, DecompositionRuleStats]:
    """Decompose gates without device-capacity checks and return cache stats."""
    ...

def decompose_mc_gates_for_device(
    circuit: Circuit,
    device: Device,
    resource_policy: ResourcePolicy | None = None,
) -> TransformResult:
    """Decompose multi-controlled gates while enforcing usable device capacity.

    Raises:
        CompilerConfigError: If the circuit exceeds device capacity or
            decomposition fails.
        CircuitError: If circuit rebuilding fails.
    """
    ...

__all__ = [
    "mc_gate",
    "unitary",
    "TwoQubitUnitaryDecomposeBasis",
    "UnitaryDecomposeConfig",
    "McGateDecomposeConfig",
    "DecompositionRuleStats",
    "expand_definitions",
    "decompose_unitaries",
    "decompose_unitaries_with_rule_stats",
    "decompose_mc_gates",
    "decompose_mc_gates_with_rule_stats",
    "decompose_mc_gates_for_device",
]
