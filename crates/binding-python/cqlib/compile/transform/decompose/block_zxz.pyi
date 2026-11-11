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

"""Block-ZXZ decomposition of general n-qubit unitary matrices."""

from __future__ import annotations

import numpy as np
from numpy.typing import ArrayLike, NDArray

from cqlib.circuit import Circuit

class BlockZxzDecomposeOptions:
    """Optimization switches of the paper's Section 5 reductions.

    Args:
        opt_a1: Fold each recursion level's two UCRz side CNOTs into the
            middle multiplexer (paper 5.2 / Eq. 11; saves 2 CNOTs per
            level).
        opt_a2: Two-qubit leaf 2-CNOT synthesis plus rolling diagonal
            remainders (paper 5.1; saves ``4^(n-2) - 1`` CNOTs).
        opt_a2_force: Re-enable A.2 on block-diagonal (controlled) inputs,
            where the paper disables it by default.
    """

    def __init__(
        self,
        *,
        opt_a1: bool = True,
        opt_a2: bool = True,
        opt_a2_force: bool = False,
    ) -> None: ...
    @property
    def opt_a1(self) -> bool: ...
    @property
    def opt_a2(self) -> bool: ...
    @property
    def opt_a2_force(self) -> bool: ...
    @staticmethod
    def none() -> BlockZxzDecomposeOptions:
        """Options with every optimization disabled (unoptimized pipeline)."""
        ...

    def __copy__(self) -> BlockZxzDecomposeOptions: ...
    def __deepcopy__(self, memo: dict[int, object]) -> BlockZxzDecomposeOptions: ...
    def __eq__(self, other: object) -> bool: ...

def decompose_block_zxz(matrix: ArrayLike, atol: float = 1e-10) -> Circuit:
    """Decompose a general n-qubit unitary into single-qubit rotations and
    CX gates via the recursive Block-ZXZ method.

    Both paper Section 5 optimizations (A.1 + A.2) are enabled by default;
    the CNOT count meets the paper bound
    ``22/48 * 4^n - 3/2 * 2^n + 5/3`` (n=3 -> 19, n=4 -> 95).

    Numerically degenerate inputs can exceed that bound by at most two CX:
    when folding the accumulated A.2 diagonal remainder into a two-qubit
    leaf is not numerically feasible, the remainder is emitted as an
    explicit diagonal gate instead of failing the decomposition.

    Args:
        matrix: Square power-of-two unitary matrix.
        atol: Absolute tolerance of the unitarity and invariant checks;
            must be a finite non-negative number.

    Returns:
        The assembled :class:`~cqlib.circuit.Circuit`.

    Raises:
        ValueError: If ``matrix`` is not a square power-of-two unitary (a
            NaN or infinite entry is rejected up front), if ``atol`` is
            not a finite non-negative number, or if a decomposition
            invariant fails.
    """
    ...

def decompose_block_zxz_with_options(
    matrix: ArrayLike,
    atol: float = 1e-10,
    *,
    options: BlockZxzDecomposeOptions | None = None,
) -> Circuit:
    """Decompose a general n-qubit unitary with explicit optimization
    switches.

    A.2 is auto-disabled on block-diagonal (controlled-gate) inputs unless
    ``options.opt_a2_force`` is set (paper Sec. 6.2 constraint).

    Args:
        matrix: Square power-of-two unitary matrix.
        atol: Absolute tolerance of the unitarity and invariant checks;
            must be a finite non-negative number.
        options: Optimization switches; ``None`` selects the fully
            optimized default.

    Returns:
        The assembled :class:`~cqlib.circuit.Circuit`.

    Raises:
        ValueError: If ``matrix`` is invalid (including NaN or infinite
            entries), if ``atol`` is not a finite non-negative number, or
            if a decomposition invariant fails.
    """
    ...

def block_zxz_step1(
    matrix: ArrayLike, atol: float = 1e-10
) -> tuple[
    NDArray[np.complex128],
    NDArray[np.complex128],
    NDArray[np.complex128],
    NDArray[np.complex128],
]:
    """Run Block-ZXZ step 1 and return the factors ``(A1, A2, B, C)``.

    Raises:
        ValueError: If ``matrix`` is not a valid unitary of at least two
            qubits.
    """
    ...

def block_zxz_demux_pair(
    u0: ArrayLike, u1: ArrayLike, atol: float = 1e-10
) -> tuple[
    NDArray[np.complex128],
    NDArray[np.complex128],
    NDArray[np.complex128],
]:
    """Demultiplex a block pair ``(U0, U1)`` into ``(V, D, W)``.

    The factors satisfy ``U0 = V @ D @ W`` and
    ``U1 = V @ D.conj().T @ W``.

    Raises:
        ValueError: If the inputs are invalid or the reconstruction check
            fails.
    """
    ...

def block_zxz_step3_ucrz_to_circuit(
    d_matrix: ArrayLike, atol: float = 1e-10
) -> Circuit:
    """Expand a diagonal unitary ``D`` into its uniformly controlled Rz
    circuit, enacting ``diag(D, D†)``.

    Raises:
        ValueError: If ``d_matrix`` is not a diagonal unitary.
    """
    ...

def block_zxz_is_block_diagonal(matrix: ArrayLike, atol: float = 1e-10) -> bool:
    """Return whether ``matrix`` is block-diagonal (controlled-gate shape)."""
    ...

def block_zxz_cx_count(circuit: Circuit) -> int:
    """Count the CX gates of ``circuit`` (nested composite blocks included)."""
    ...

__all__ = [
    "BlockZxzDecomposeOptions",
    "decompose_block_zxz",
    "decompose_block_zxz_with_options",
    "block_zxz_step1",
    "block_zxz_demux_pair",
    "block_zxz_step3_ucrz_to_circuit",
    "block_zxz_is_block_diagonal",
    "block_zxz_cx_count",
]
