# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

"""Numeric two-qubit block resynthesis transforms."""

from ..._native import compile as _compile_module

_resynthesis_module = _compile_module.transform.resynthesis

TwoQubitBlockResynthesisConfig = _resynthesis_module.TwoQubitBlockResynthesisConfig
ResynthesizeTwoQubitBlocks = _resynthesis_module.ResynthesizeTwoQubitBlocks
resynthesize_two_qubit_blocks = _resynthesis_module.resynthesize_two_qubit_blocks
resynthesize_two_qubit_blocks_for_device = (
    _resynthesis_module.resynthesize_two_qubit_blocks_for_device
)

__all__ = [
    "TwoQubitBlockResynthesisConfig",
    "ResynthesizeTwoQubitBlocks",
    "resynthesize_two_qubit_blocks",
    "resynthesize_two_qubit_blocks_for_device",
]
