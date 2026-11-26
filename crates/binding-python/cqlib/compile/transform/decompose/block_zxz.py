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

"""Recursive Block-ZXZ synthesis of finite numeric unitary matrices."""

from ...._native import compile as _compile

_module = _compile.transform.decompose.block_zxz
BlockZxzDecomposeOptions = _module.BlockZxzDecomposeOptions
decompose_block_zxz = _module.decompose_block_zxz
decompose_block_zxz_with_options = _module.decompose_block_zxz_with_options

__all__ = [
    "BlockZxzDecomposeOptions",
    "decompose_block_zxz",
    "decompose_block_zxz_with_options",
]
