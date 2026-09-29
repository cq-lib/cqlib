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

from collections.abc import Callable

from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian

# Estimators receive the (widened) circuit, an optional Hamiltonian, and an
# optional shot count, and return ``(mean, variance)``. Virtual distillation
# passes a Hamiltonian on both the numerator call (the observable expanded
# with X on the ancillary qubit) and the denominator call (a single X on the
# ancillary qubit); distinguish the two by observable content, not by
# ``observable is None``.
Estimator = Callable[[Circuit, Hamiltonian | None, int | None], tuple[float, float]]

class VirtualDistillationConfig:
    """Configuration for virtual distillation.

    Args:
        copies: Number of density-matrix copies. The core algorithm requires
            at least two copies.
    """

    def __init__(self, copies: int) -> None: ...
    @property
    def copies(self) -> int:
        """Configured number of copies."""
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> VirtualDistillationConfig: ...
    def __deepcopy__(self, memo: dict) -> VirtualDistillationConfig: ...

class VirtualDistillation:
    """Low-level virtual distillation helper.

    The helper builds Hadamard-test copy-swap circuits and combines numerator
    and denominator estimator outputs into a mitigated expectation value and
    variance. Both estimator calls receive a ``Hamiltonian`` observable; the
    denominator observable is a single X on the ancillary (last) qubit.
    """

    def __init__(self, circuit: Circuit, copies: int) -> None:
        """Create a virtual distillation helper for ``circuit``."""
        ...
    @property
    def copies(self) -> int:
        """Current number of configured copies."""
        ...
    def set_copies(self, copies: int) -> None:
        """Update the configured number of copies."""
        ...
    def build_copy_swap_circuit(self) -> Circuit:
        """Build the Hadamard-test copy-swap circuit used by virtual distillation.

        The returned circuit has width ``copies * base_width + 1``, where the
        last qubit is the ancillary qubit. It contains, in order: the
        ``copies`` copies of the base circuit, a Hadamard gate on the
        ancillary qubit, and ``(copies - 1) * base_width`` ancilla-controlled
        SWAP gates (exposed as ``C1-SWAP``) forming a permutation ladder
        between adjacent copies.
        """
        ...
    def run_denominator_circuit(
        self,
        shots: int,
        estimator: Estimator,
    ) -> tuple[float, float]:
        """Run the denominator circuit and return ``(mean, variance)``.

        ``estimator`` receives the copy-swap circuit together with the
        denominator observable: a single X on the ancillary (last) qubit and
        identity on every other qubit.
        """
        ...
    def run_numerator_circuit(
        self,
        hamiltonian: Hamiltonian,
        shots: int,
        estimator: Estimator,
    ) -> tuple[float, float]:
        """Run the numerator circuit and return ``(mean, variance)``.

        ``estimator`` receives the copy-swap circuit together with
        ``hamiltonian`` expanded to the full circuit width: the original
        Pauli terms on the first copy's qubits, identity on the remaining
        copies, and X on the ancillary (last) qubit.
        """
        ...
    def run_vd(
        self,
        hamiltonian: Hamiltonian,
        shots_numerator: int,
        shots_denominator: int,
        estimator: Estimator,
    ) -> tuple[float, float]:
        """Run virtual distillation and return ``(mitigated_mean, variance)``.

        The mitigated mean is the numerator estimate divided by the
        denominator estimate.
        """
        ...
    def __repr__(self) -> str: ...
    def __copy__(self) -> VirtualDistillation: ...
    def __deepcopy__(self, memo: dict) -> VirtualDistillation: ...

__all__ = [
    "Estimator",
    "VirtualDistillationConfig",
    "VirtualDistillation",
]
