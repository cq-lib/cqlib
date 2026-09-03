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

from cqlib.circuit import Circuit, Instruction
from cqlib.qis import Hamiltonian

Estimator = Callable[[Circuit, Hamiltonian | None, int | None], tuple[float, float]]

class ExtrapolateMethod:
    """Zero-noise extrapolation fit method.

    Use :meth:`polynomial` for polynomial regression or :meth:`exponential`
    for a log-space exponential-decay fit. Instances are immutable value
    objects understood by :class:`ZNEMitigation` and the unified pipeline.
    """

    @staticmethod
    def polynomial() -> ExtrapolateMethod:
        """Return the polynomial extrapolation method."""
        ...
    @staticmethod
    def exponential() -> ExtrapolateMethod:
        """Return the exponential extrapolation method."""
        ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __copy__(self) -> ExtrapolateMethod: ...
    def __deepcopy__(self, memo: dict) -> ExtrapolateMethod: ...

class ZneConfig:
    """Configuration for zero-noise extrapolation.

    Args:
        fold_levels: Non-negative unitary-folding levels. Each level maps to
            a noise factor ``2 * level + 1``.
    """

    def __init__(self, fold_levels: Sequence[int]) -> None: ...
    @property
    def fold_levels(self) -> list[int]:
        """Configured circuit-folding levels."""
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> ZneConfig: ...
    def __deepcopy__(self, memo: dict) -> ZneConfig: ...

class ZNEMitigation:
    """Low-level zero-noise extrapolation helper.

    The helper owns a copy of the base circuit, produces folded circuits, and
    extrapolates expectation values collected from those folded circuits.
    """

    def __init__(self, circuit: Circuit, fold_levels: Sequence[int]) -> None:
        """Create a ZNE helper for ``circuit`` and ``fold_levels``."""
        ...
    @property
    def circuit(self) -> Circuit:
        """Copy of the original unfurled circuit."""
        ...
    @property
    def fold_levels(self) -> list[int]:
        """Configured circuit-folding levels."""
        ...
    @property
    def noise_factors(self) -> list[int]:
        """Noise factors derived from ``fold_levels``."""
        ...
    def fold_circuits(
        self,
        gate_set: Sequence[Instruction] | None = None,
    ) -> list[Circuit]:
        """Return folded circuits for all configured levels.

        ``gate_set=None`` performs global folding. Passing a gate set folds
        only operations whose instruction names match one of the supplied
        instructions.
        """
        ...
    def run_em_sequence(
        self,
        gate_set: Sequence[Instruction] | None,
        hamiltonian: Hamiltonian,
        estimator: Estimator,
    ) -> list[float]:
        """Run folded circuits with ``estimator`` and return expectations."""
        ...
    def run_em_sequence_with_shots(
        self,
        gate_set: Sequence[Instruction] | None,
        hamiltonian: Hamiltonian,
        shots: int | None,
        estimator: Estimator,
    ) -> list[float]:
        """Run folded circuits with an optional shot count."""
        ...
    def extrapolate(
        self,
        noisy_results: Sequence[float],
        method: ExtrapolateMethod,
        degree: int,
    ) -> float:
        """Extrapolate noisy expectations to the zero-noise point."""
        ...
    def poly_extrapolate(self, noisy_results: Sequence[float], degree: int) -> float:
        """Extrapolate with a polynomial fit of ``degree``."""
        ...
    def exp_extrapolate(self, noisy_results: Sequence[float]) -> float:
        """Extrapolate with an exponential-decay fit."""
        ...
    def __repr__(self) -> str: ...
    def __copy__(self) -> ZNEMitigation: ...
    def __deepcopy__(self, memo: dict) -> ZNEMitigation: ...

__all__ = [
    "Estimator",
    "ExtrapolateMethod",
    "ZneConfig",
    "ZNEMitigation",
]
