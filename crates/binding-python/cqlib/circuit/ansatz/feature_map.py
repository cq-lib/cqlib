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

"""Runtime exports for feature-map ansatze."""

from ..._native import circuit as _circuit_module

_ansatz_module = _circuit_module.ansatz

AngleEncoding = _ansatz_module.AngleEncoding
BasisEncoding = _ansatz_module.BasisEncoding
ZFeatureMap = _ansatz_module.ZFeatureMap
IQPFeatureMap = _ansatz_module.IQPFeatureMap
ZZFeatureMap = _ansatz_module.ZZFeatureMap
PauliFeatureMap = _ansatz_module.PauliFeatureMap

__all__ = [
    "AngleEncoding",
    "BasisEncoding",
    "ZFeatureMap",
    "IQPFeatureMap",
    "ZZFeatureMap",
    "PauliFeatureMap",
]
