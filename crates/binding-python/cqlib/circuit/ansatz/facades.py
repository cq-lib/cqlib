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

"""Runtime exports for ansatz convenience constructors."""

from ..._native import circuit as _circuit_module

_ansatz_module = _circuit_module.ansatz

real_amplitudes = _ansatz_module.real_amplitudes
efficient_su2 = _ansatz_module.efficient_su2
zz_feature_map = _ansatz_module.zz_feature_map
pauli_feature_map = _ansatz_module.pauli_feature_map

__all__ = [
    "real_amplitudes",
    "efficient_su2",
    "zz_feature_map",
    "pauli_feature_map",
]
