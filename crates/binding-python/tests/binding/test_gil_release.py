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

import sys

import pytest

import numpy as np

from cqlib import Circuit
from cqlib.qis.state import (
    DensityMatrix,
    DensityMatrixNoise,
    StabilizerState,
    Statevector,
)


def test_gil_probe_rejects_an_action_that_holds_the_gil(assert_releases_gil) -> None:
    switch_interval = sys.getswitchinterval()
    with pytest.raises(pytest.fail.Exception, match="No Python thread progress"):
        assert_releases_gil(lambda: None, timeout=0.05)
    assert sys.getswitchinterval() == switch_interval


def test_circuit_to_matrix_releases_gil(assert_releases_gil):
    circuit = Circuit(10)
    for _ in range(4):
        for qubit in range(10):
            circuit.h(qubit)
        for qubit in range(9):
            circuit.cx(qubit, qubit + 1)

    assert_releases_gil(circuit.to_matrix)


def test_statevector_apply_circuit_releases_gil(assert_releases_gil):
    circuit = Circuit(20)
    for _ in range(6):
        for qubit in range(20):
            circuit.h(qubit)
        for qubit in range(19):
            circuit.cx(qubit, qubit + 1)
    state = Statevector(20)

    assert_releases_gil(lambda: state.apply_circuit(circuit))


def test_statevector_probabilities_releases_gil(assert_releases_gil):
    state = Statevector(20)

    assert_releases_gil(state.probabilities)


def test_density_matrix_unitary_releases_gil(assert_releases_gil):
    state = DensityMatrix(9)
    matrix = np.array([[0.0, 1.0], [1.0, 0.0]], dtype=complex)

    assert_releases_gil(lambda: state.apply_unitary_gate([0], matrix))


def test_noise_simulator_unitary_releases_gil(assert_releases_gil):
    state = DensityMatrixNoise(9)
    matrix = np.array([[0.0, 1.0], [1.0, 0.0]], dtype=complex)

    assert_releases_gil(lambda: state.apply_unitary_gate([0], matrix))


def test_stabilizer_probabilities_releases_gil(assert_releases_gil):
    state = StabilizerState(18)

    assert_releases_gil(state.probabilities)
