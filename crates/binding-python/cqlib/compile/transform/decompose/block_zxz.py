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

"""Block-ZXZ synthesis of arbitrary unitary matrices into circuits."""

from ...._native import compile as _compile_module

_block_zxz_module = _compile_module.transform.decompose.block_zxz

BlockZxzOptions = _block_zxz_module.BlockZxzOptions
BlockZxzResult = _block_zxz_module.BlockZxzResult
decompose_block_zxz = _block_zxz_module.decompose_block_zxz

__all__ = [
    "BlockZxzOptions",
    "BlockZxzResult",
    "decompose_block_zxz",
]
