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

"""Virtual distillation bindings.

Virtual distillation builds Hadamard-test copy-swap circuits from a base
circuit: ``copies`` disjoint copies of the base circuit, a Hadamard gate on a
trailing ancillary qubit, and a ladder of ancilla-controlled SWAP gates
between adjacent copies. Numerator and denominator estimates are combined
into a mitigated expectation value; both estimator calls receive a
``Hamiltonian`` observable (the denominator observable is a single X on the
ancillary qubit).
"""

from collections.abc import Callable

from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian

from .._native import error_mitigation as _error_mitigation_module

# Estimators receive the (widened) circuit, an optional Hamiltonian, and an
# optional shot count, and return ``(mean, variance)``. Virtual distillation
# passes a Hamiltonian on both the numerator call (the observable expanded
# with X on the ancillary qubit) and the denominator call (a single X on the
# ancillary qubit); distinguish the two by observable content, not by
# ``observable is None``.
Estimator = Callable[[Circuit, Hamiltonian | None, int | None], tuple[float, float]]
VirtualDistillationConfig = _error_mitigation_module.VirtualDistillationConfig
VirtualDistillation = _error_mitigation_module.VirtualDistillation

__all__ = [
    "Estimator",
    "VirtualDistillationConfig",
    "VirtualDistillation",
]
