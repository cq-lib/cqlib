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

"""Tests for the Block-ZXZ arbitrary-unitary synthesis binding."""

import copy
import sys

import numpy as np
import pytest

from cqlib.compile.transform.decompose import (
    BlockZxzOptions,
    BlockZxzResult,
    decompose_block_zxz,
)
from cqlib.compile.transform.decompose import block_zxz as block_zxz_module


def random_unitary(num_qubits: int, seed: int) -> np.ndarray:
    rng = np.random.default_rng((seed, num_qubits))
    dim = 1 << num_qubits
    z = rng.standard_normal((dim, dim)) + 1j * rng.standard_normal((dim, dim))
    q, r = np.linalg.qr(z)
    return q * np.exp(-1j * np.angle(np.diag(r)) / 2)


def phase_aligned_error(a: np.ndarray, b: np.ndarray) -> float:
    overlap = np.vdot(b, a)
    if abs(overlap) < 1e-8:
        return float(np.abs(a - b).max())
    return float(np.abs(a - b * np.exp(1j * np.angle(overlap))).max())


def test_block_zxz_module_and_types_are_registered() -> None:
    assert "cqlib._native.compile.transform.decompose.block_zxz" in sys.modules
    assert block_zxz_module.decompose_block_zxz is decompose_block_zxz
    assert BlockZxzOptions.__module__ == "cqlib.compile.transform.decompose.block_zxz"
    assert BlockZxzResult.__module__ == "cqlib.compile.transform.decompose.block_zxz"


def test_block_zxz_options_are_copyable_value_objects() -> None:
    full = BlockZxzOptions()
    assert full == BlockZxzOptions(opt_a1=True, opt_a2=True)
    assert full != BlockZxzOptions.none()
    assert repr(full) == "BlockZxzOptions(opt_a1=True, opt_a2=True, opt_a2_force=False)"

    none = BlockZxzOptions.none()
    assert not none.opt_a1
    assert not none.opt_a2
    assert not none.opt_a2_force
    assert repr(none) == "BlockZxzOptions(opt_a1=False, opt_a2=False, opt_a2_force=False)"

    forced = BlockZxzOptions(opt_a1=False, opt_a2=True, opt_a2_force=True)
    assert forced.opt_a2_force

    assert copy.copy(full) == full
    assert copy.deepcopy(full) == full


@pytest.mark.parametrize(
    "num_qubits, expected_cx",
    [(3, 19), (4, 95)],
)
def test_decompose_block_zxz_paper_table1_counts(num_qubits: int, expected_cx: int) -> None:
    """Fully optimized counts match the paper's Table 1 (arXiv:2403.13692)."""
    unitary = random_unitary(num_qubits, seed=7)
    result = decompose_block_zxz(unitary)

    assert isinstance(result, BlockZxzResult)
    assert result.num_qubits == num_qubits
    assert result.cx_count == expected_cx
    assert result.circuit.num_qubits == num_qubits
    assert phase_aligned_error(unitary, result.circuit.to_matrix()) < 1e-9


@pytest.mark.parametrize(
    "num_qubits, expected_cx",
    [(2, 3), (3, 24), (4, 120)],
)
def test_decompose_block_zxz_unoptimized_baseline(num_qubits: int, expected_cx: int) -> None:
    """Flag-free options reproduce the unoptimized paper baseline counts."""
    unitary = random_unitary(num_qubits, seed=7)
    result = decompose_block_zxz(unitary, options=BlockZxzOptions.none())

    assert result.cx_count == expected_cx
    assert phase_aligned_error(unitary, result.circuit.to_matrix()) < 1e-9


def test_decompose_block_zxz_block_diagonal_disables_a2() -> None:
    """Controlled-gate inputs auto-disable A.2 (paper Sec. 6.2)."""
    u0 = random_unitary(2, seed=21)
    u1 = random_unitary(2, seed=22)
    unitary = np.zeros((8, 8), dtype=np.complex128)
    unitary[:4, :4] = u0
    unitary[4:, 4:] = u1

    full = decompose_block_zxz(unitary)
    a1_only = decompose_block_zxz(
        unitary, options=BlockZxzOptions(opt_a1=True, opt_a2=False)
    )

    assert full.cx_count == a1_only.cx_count
    assert full.cx_count < decompose_block_zxz(
        unitary, options=BlockZxzOptions.none()
    ).cx_count
    assert phase_aligned_error(unitary, full.circuit.to_matrix()) < 1e-9


def test_decompose_block_zxz_accepts_atol_override() -> None:
    unitary = random_unitary(2, seed=3)
    result = decompose_block_zxz(unitary, atol=1e-9)
    assert result.num_qubits == 2
    assert phase_aligned_error(unitary, result.circuit.to_matrix()) < 1e-9


def test_decompose_block_zxz_raises_on_invalid_input() -> None:
    with pytest.raises(ValueError):
        decompose_block_zxz([[1, 2], [3, 4]])  # not a unitary matrix
    with pytest.raises(ValueError):
        decompose_block_zxz(np.eye(3, dtype=np.complex128))  # not a power-of-two dim
    with pytest.raises(ValueError):
        decompose_block_zxz("not a matrix")
    with pytest.raises(ValueError):
        decompose_block_zxz(random_unitary(2, seed=7), atol=-1.0)
    with pytest.raises(ValueError):
        decompose_block_zxz(random_unitary(2, seed=7), atol=float("nan"))
