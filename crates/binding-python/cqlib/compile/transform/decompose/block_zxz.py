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

"""Block-ZXZ decomposition of general n-qubit unitary matrices.

Implements the recursive Block-ZXZ decomposition of Krol & Al-Ars,
"Beyond Quantum Shannon: Circuit Construction for General n-Qubit Gates
Based on Block ZXZ-Decomposition" (arXiv:2403.13692), including the
paper's Section 5 CNOT-count optimizations:

- A.1 (``opt_a1``): each recursion level's two UCRz side CNOTs are folded
  into the middle multiplexer via the ``H @ CX = CZ @ H`` identities,
  saving 2 CNOTs per level.
- A.2 (``opt_a2``): every two-qubit leaf but the temporally last one is
  synthesized with 2 CNOTs plus a rolling diagonal remainder
  (Shende-Markov-Bullock "up to diagonal"), saving ``4^(n-2) - 1`` CNOTs.

With both optimizations enabled (the default) the CNOT count meets the
paper bound ``22/48 * 4^n - 3/2 * 2^n + 5/3`` (n=3 -> 19, n=4 -> 95).

Example::

    import numpy as np
    from cqlib.compile.transform.decompose.block_zxz import (
        decompose_block_zxz,
        block_zxz_cx_count,
    )

    unitary = np.eye(8, dtype=complex)
    circuit = decompose_block_zxz(unitary)
    assert block_zxz_cx_count(circuit) <= 19
"""

from ...._native import compile as _compile_module

_block_zxz_module = _compile_module.transform.decompose.block_zxz

BlockZxzDecomposeOptions = _block_zxz_module.BlockZxzDecomposeOptions
decompose_block_zxz = _block_zxz_module.decompose_block_zxz
decompose_block_zxz_with_options = _block_zxz_module.decompose_block_zxz_with_options
block_zxz_step1 = _block_zxz_module.block_zxz_step1
block_zxz_demux_pair = _block_zxz_module.block_zxz_demux_pair
block_zxz_step3_ucrz_to_circuit = _block_zxz_module.block_zxz_step3_ucrz_to_circuit
block_zxz_is_block_diagonal = _block_zxz_module.block_zxz_is_block_diagonal
block_zxz_cx_count = _block_zxz_module.block_zxz_cx_count

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
