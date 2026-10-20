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

from __future__ import annotations

import numpy as np
from numpy.typing import ArrayLike

from cqlib.circuit import Circuit


class BlockZxzOptions:
    """Optimization switches for the Block-ZXZ paper's Section 5 reductions.

    ``opt_a1`` folds each recursion level's two UCRz side CNOTs into the
    middle multiplexor (saves two CNOTs per level); ``opt_a2`` decomposes
    every two-qubit leaf but the temporally last one into a 2-CNOT circuit
    plus a rolling diagonal remainder (saves ``4^(n-2) - 1`` CNOTs).
    ``opt_a2_force`` re-enables A.2 on block-diagonal (controlled) inputs,
    where it is disabled by default.
    """

    def __init__(
        self,
        *,
        opt_a1: bool = True,
        opt_a2: bool = True,
        opt_a2_force: bool = False,
    ) -> None: ...
    @staticmethod
    def none() -> BlockZxzOptions:
        """All optimizations off (matches the unoptimized paper baseline)."""
        ...
    @property
    def opt_a1(self) -> bool:
        """Whether the A.1 middle-multiplexor CNOT absorption is enabled."""
        ...
    @property
    def opt_a2(self) -> bool:
        """Whether the A.2 two-qubit-leaf 2-CNOT chaining is enabled."""
        ...
    @property
    def opt_a2_force(self) -> bool:
        """Whether A.2 stays enabled on block-diagonal (controlled) inputs."""
        ...
    def __copy__(self) -> BlockZxzOptions: ...
    def __deepcopy__(self, memo: dict[int, object]) -> BlockZxzOptions: ...
    def __eq__(self, other: object) -> bool: ...


class BlockZxzResult:
    """Circuit synthesized from an arbitrary unitary via Block-ZXZ."""

    @property
    def circuit(self) -> Circuit:
        """Circuit implementing the input unitary up to global phase."""
        ...
    @property
    def num_qubits(self) -> int:
        """Number of qubits of the synthesized circuit."""
        ...
    @property
    def cx_count(self) -> int:
        """Number of CX gates in the assembled circuit."""
        ...


def decompose_block_zxz(
    matrix: ArrayLike,
    atol: float = 1e-10,
    options: BlockZxzOptions | None = None,
) -> BlockZxzResult:
    """Synthesize an arbitrary ``2^n x 2^n`` unitary into a circuit.

    Uses Block-ZXZ decomposition (arXiv:2403.13692) with both Section 5
    CNOT optimizations enabled by default: generic random unitaries yield
    19 CX gates for 3 qubits and 95 for 4 qubits (paper Table 1), versus
    the unoptimized baseline of 24 / 120 (:meth:`BlockZxzOptions.none`).

    Block-diagonal (controlled-gate) inputs disable A.2 automatically,
    per paper Sec. 6.2.

    Raises:
        ValueError: If ``matrix`` cannot be converted to a two-dimensional
            complex128 array, is not a square power-of-two unitary, or
            decomposition fails.
    """
    ...


__all__ = [
    "BlockZxzOptions",
    "BlockZxzResult",
    "decompose_block_zxz",
]
