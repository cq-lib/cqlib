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

from numpy.typing import ArrayLike

from cqlib.circuit import Circuit

class BlockZxzDecomposeOptions:
    def __init__(
        self, *, opt_a1: bool = True, opt_a2: bool = True, opt_a2_force: bool = False
    ) -> None: ...
    @property
    def opt_a1(self) -> bool: ...
    @property
    def opt_a2(self) -> bool: ...
    @property
    def opt_a2_force(self) -> bool: ...
    @staticmethod
    def none() -> BlockZxzDecomposeOptions: ...
    def __copy__(self) -> BlockZxzDecomposeOptions: ...
    def __deepcopy__(self, memo: dict[int, object]) -> BlockZxzDecomposeOptions: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

def decompose_block_zxz(matrix: ArrayLike, atol: float = 1e-10) -> Circuit: ...
def decompose_block_zxz_with_options(
    matrix: ArrayLike,
    atol: float = 1e-10,
    *,
    options: BlockZxzDecomposeOptions | None = None,
) -> Circuit: ...
