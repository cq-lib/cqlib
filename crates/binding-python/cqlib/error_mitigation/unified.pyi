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

from collections.abc import Callable, Sequence

from cqlib.circuit import Circuit, CqlibError, Instruction
from cqlib.qis import Hamiltonian

from .virtual_distillation import VirtualDistillationConfig
from .zne import ExtrapolateMethod, ZneConfig

Estimator = Callable[[Circuit, Hamiltonian | None, int | None], tuple[float, float]]

class ErrorMitigationError(CqlibError):
    """Base exception for error-mitigation API failures."""
    ...

class MitigationMethod:
    """Configured mitigation method for the unified pipeline."""

    @staticmethod
    def zne(config: ZneConfig) -> MitigationMethod:
        """Configure zero-noise extrapolation."""
        ...
    @staticmethod
    def virtual_distillation(config: VirtualDistillationConfig) -> MitigationMethod:
        """Configure virtual distillation."""
        ...
    @property
    def method_type(self) -> str:
        """Method discriminator: ``"zne"`` or ``"virtual_distillation"``."""
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> MitigationMethod: ...
    def __deepcopy__(self, memo: dict) -> MitigationMethod: ...

class RunArgs:
    """Runtime arguments for one mitigation pipeline run."""

    @staticmethod
    def zne(
        gate_set: Sequence[Instruction] | None = None,
        shots: int | None = None,
    ) -> RunArgs:
        """Runtime arguments for a ZNE run."""
        ...
    @staticmethod
    def virtual_distillation(shots_numerator: int, shots_denominator: int) -> RunArgs:
        """Runtime arguments for a virtual distillation run."""
        ...
    @property
    def method_type(self) -> str:
        """Method discriminator for these run arguments."""
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> RunArgs: ...
    def __deepcopy__(self, memo: dict) -> RunArgs: ...

class ProcessArgs:
    """Post-processing arguments for a completed mitigation run."""

    @staticmethod
    def zne(method: ExtrapolateMethod, degree: int | None = None) -> ProcessArgs:
        """Post-processing arguments for a ZNE run."""
        ...
    @staticmethod
    def virtual_distillation() -> ProcessArgs:
        """Post-processing arguments for a virtual distillation run."""
        ...
    @property
    def method_type(self) -> str:
        """Method discriminator for these processing arguments."""
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> ProcessArgs: ...
    def __deepcopy__(self, memo: dict) -> ProcessArgs: ...

class MitigatedResult:
    """Final mitigated observable estimate."""

    @property
    def expectation(self) -> float:
        """Mitigated expectation value."""
        ...
    @property
    def variance(self) -> float | None:
        """Mitigated variance when the selected method provides one."""
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> MitigatedResult: ...
    def __deepcopy__(self, memo: dict) -> MitigatedResult: ...

class ErrorMitigation:
    """Unified sequential mitigation pipeline.

    The workflow is ``run(...)`` followed by ``get_mitigated(...)``. Each
    instance can be run and post-processed once, matching the core state
    machine.
    """

    def __init__(self, circuit: Circuit, method: MitigationMethod) -> None:
        """Create a mitigation pipeline for ``circuit`` and ``method``."""
        ...
    def run(
        self,
        hamiltonian: Hamiltonian,
        run_args: RunArgs,
        estimator: Estimator,
    ) -> None:
        """Execute the method-specific circuits with ``estimator``."""
        ...
    def get_mitigated(self, process_args: ProcessArgs) -> MitigatedResult:
        """Post-process stored run outputs and return the final estimate."""
        ...
    def __repr__(self) -> str: ...
    def __copy__(self) -> ErrorMitigation: ...
    def __deepcopy__(self, memo: dict) -> ErrorMitigation: ...

__all__ = [
    "Estimator",
    "ErrorMitigationError",
    "MitigationMethod",
    "RunArgs",
    "ProcessArgs",
    "MitigatedResult",
    "ErrorMitigation",
]
