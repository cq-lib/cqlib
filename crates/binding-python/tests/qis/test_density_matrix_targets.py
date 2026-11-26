# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# Licensed under the Apache License, Version 2.0. See LICENSE.txt or
# http://www.apache.org/licenses/LICENSE-2.0.

"""Target validation must precede any density matrix mutation."""

import numpy as np
import pytest

from cqlib.qis import DensityMatrix, DensityMatrixNoise


def initial_state():
    amplitudes = np.arange(1, 9) + 1j * np.arange(8, 0, -1)
    return amplitudes / np.linalg.norm(amplitudes)


def householder(dim):
    vector = np.arange(1, dim + 1, dtype=float)
    return (np.eye(dim) - 2 * np.outer(vector, vector) / (vector @ vector)).astype(
        complex
    )


def state_data(state):
    return state.state if isinstance(state, DensityMatrixNoise) else state.data


@pytest.mark.parametrize("simulator", [DensityMatrix, DensityMatrixNoise])
@pytest.mark.parametrize("target", [0, 1, 2])
@pytest.mark.parametrize(
    "method,params",
    [
        ("apply_cx", []),
        ("apply_cz", []),
        ("apply_cy", []),
        ("apply_swap", []),
        ("apply_rxx", [0.37]),
        ("apply_ryy", [0.37]),
        ("apply_rzz", [0.37]),
        ("apply_rzx", [0.37]),
        ("apply_crx", [0.37]),
        ("apply_cry", [0.37]),
        ("apply_crz", [0.37]),
        ("apply_fsim", [0.37, -0.21]),
    ],
)
def test_duplicate_two_qubit_targets(simulator, target, method, params):
    state = simulator(3)
    state.apply_h(0)
    state.apply_s(0)
    before = state_data(state).copy()
    with pytest.raises(ValueError, match=rf"(?i)duplicate qubit index {target}\b"):
        getattr(state, method)(target, target, *params)
    np.testing.assert_array_equal(state_data(state), before)


@pytest.mark.parametrize("simulator", [DensityMatrix, DensityMatrixNoise])
@pytest.mark.parametrize("targets", [(0, 0, 2), (0, 2, 0), (2, 0, 0)])
def test_duplicate_ccx_targets(simulator, targets):
    state = simulator(3)
    state.apply_h(0)
    before = state_data(state).copy()
    with pytest.raises(ValueError, match=r"(?i)duplicate qubit index 0\b"):
        state.apply_ccx(*targets)
    np.testing.assert_array_equal(state_data(state), before)


@pytest.mark.parametrize("target", [0, 1, 2])
def test_duplicate_custom_gate_targets(target):
    state = DensityMatrix.from_state(3, initial_state())
    before = state.data.copy()
    with pytest.raises(ValueError, match=rf"(?i)duplicate qubit index {target}\b"):
        state.apply_double_qubits_gate(target, target, householder(4))
    np.testing.assert_array_equal(state.data, before)


@pytest.mark.parametrize(
    "targets,duplicate",
    [
        ([0, 0], 0),
        ([1, 1], 1),
        ([2, 2], 2),
        ([0, 2, 0], 0),
        ([0, 0, 2], 0),
        ([2, 0, 0], 0),
    ],
)
def test_duplicate_kraus_targets(targets, duplicate):
    state = DensityMatrix.from_state(3, initial_state())
    before = state.data.copy()
    with pytest.raises(ValueError, match=rf"(?i)duplicate qubit index {duplicate}\b"):
        state.apply_kraus(targets, [householder(1 << len(targets)).ravel()])
    np.testing.assert_array_equal(state.data, before)


def embed_operator(operator, targets, num_qubits):
    """Build the full operator independently, preserving target argument order."""
    dim = 1 << num_qubits
    result = np.zeros((dim, dim), dtype=complex)
    mask = sum(1 << target for target in targets)
    for column in range(dim):
        local_column = 0
        for target in targets:
            local_column = (local_column << 1) | ((column >> target) & 1)
        for local_row in range(1 << len(targets)):
            row = column & ~mask
            for offset, target in enumerate(reversed(targets)):
                row |= ((local_row >> offset) & 1) << target
            result[row, column] = operator[local_row, local_column]
    return result


@pytest.mark.parametrize("targets", [[0, 1], [1, 0], [0, 2], [2, 0]])
@pytest.mark.parametrize("method", ["custom", "kraus", "unitary", "mixed_channel"])
def test_distinct_targets_preserve_matrix_order(targets, method):
    amplitudes = initial_state()
    state = DensityMatrix.from_state(3, amplitudes)
    operator = householder(4)
    full_operator = embed_operator(operator, targets, 3)
    evolved = full_operator @ amplitudes
    expected = np.outer(evolved, evolved.conj())
    if method == "custom":
        state.apply_double_qubits_gate(*targets, operator)
    elif method == "unitary":
        state.apply_unitary_gate(targets, operator)
    elif method == "kraus":
        state.apply_kraus(targets, [operator.ravel()])
    else:
        state.apply_kraus(
            targets,
            [
                np.sqrt(0.7) * operator.ravel(),
                np.sqrt(0.3) * np.eye(4, dtype=complex).ravel(),
            ],
        )
        expected = 0.7 * expected + 0.3 * np.outer(amplitudes, amplitudes.conj())
    np.testing.assert_allclose(state.data, expected, rtol=0, atol=1e-12)
