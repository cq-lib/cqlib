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

"""Deterministic target-basis lowering and exact cost evaluation."""

from ..._native import compile as _compile_module

_target_basis_module = _compile_module.transform.target_basis

TargetBasisLowerer = _target_basis_module.TargetBasisLowerer
TargetBasisSignature = _target_basis_module.TargetBasisSignature
TargetBasisCost = _target_basis_module.TargetBasisCost
TargetBasisCostModel = _target_basis_module.TargetBasisCostModel

__all__ = [
    "TargetBasisLowerer",
    "TargetBasisSignature",
    "TargetBasisCost",
    "TargetBasisCostModel",
]
