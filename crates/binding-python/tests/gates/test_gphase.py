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

"""Zero-qubit phase and controlled-phase matrix contracts."""

import numpy as np
import pytest

from cqlib import Circuit, Parameter
from cqlib.circuit.gates import GPhase


@pytest.mark.parametrize("theta", [0.0, 0.37, -0.51, np.pi])
@pytest.mark.parametrize("controls", [None, 0, 1, 2])
def test_gphase_matrix_metadata_and_inverse(theta, controls):
    base = GPhase(theta)
    gate = base if controls is None else base.control(controls)
    count = controls or 0
    assert gate.num_qubits == count
    assert gate.num_ctrl_qubits == count
    assert gate.num_params == 1
    expected = np.eye(1 << count, dtype=complex)
    expected[-1, -1] = np.exp(1j * theta)
    actual = gate.matrix()
    assert actual.shape == expected.shape
    np.testing.assert_allclose(actual, expected, rtol=0, atol=1e-12)
    inverse = gate.inverse().matrix()
    assert inverse.shape == expected.shape
    np.testing.assert_allclose(inverse, expected.conj().T, rtol=0, atol=1e-12)


@pytest.mark.parametrize("num_qubits", [0, 1, 3])
@pytest.mark.parametrize("symbolic", [False, True])
def test_gphase_markers_accumulate_in_circuit_matrices(num_qubits, symbolic):
    circuit = Circuit(num_qubits)
    if num_qubits:
        circuit.h(0)
    baseline = circuit.to_matrix()
    circuit.append_gate(GPhase(-0.21), [])
    circuit.append_gate(GPhase(Parameter("theta") if symbolic else 0.37), [])
    circuit.append_gate(GPhase(0.12), [])
    circuit.set_global_phase(0.17)
    expected = np.exp(1j * (-0.21 + 0.37 + 0.12 + 0.17)) * baseline
    if not symbolic:
        actual = circuit.to_matrix()
        assert actual.shape == expected.shape
        np.testing.assert_allclose(actual, expected, rtol=0, atol=1e-12)
    expression = circuit.to_symbolic_matrix()
    assert expression.shape == expected.shape
    np.testing.assert_allclose(
        expression.evaluate({"theta": 0.37}), expected, rtol=0, atol=1e-12
    )


@pytest.mark.parametrize("controls", [[], [2], [2, 0]])
@pytest.mark.parametrize("order", [[0, 1, 2], [2, 0, 1]])
@pytest.mark.parametrize("symbolic", [False, True])
def test_gphase_controlled_circuit_matrices(controls, order, symbolic):
    circuit = Circuit(3)
    circuit.h(1)
    theta = Parameter("theta") if symbolic else 0.37
    circuit.append_mc_gate(GPhase(theta).control(len(controls)), controls)
    spectator_bit = order.index(1)
    mask = sum(1 << order.index(control) for control in controls)
    expected = np.zeros((8, 8), dtype=complex)
    for row in range(8):
        for col in range(8):
            if (row ^ col) & ~(1 << spectator_bit) == 0:
                sign = -1 if row & col & (1 << spectator_bit) else 1
                phase = np.exp(0.37j) if row & mask == mask else 1
                expected[row, col] = phase * sign / np.sqrt(2)
    if not symbolic:
        actual = circuit.to_matrix(order)
        assert actual.shape == (8, 8)
        np.testing.assert_allclose(actual, expected, rtol=0, atol=1e-12)
    expression = circuit.to_symbolic_matrix(order)
    assert expression.shape == (8, 8)
    np.testing.assert_allclose(
        expression.evaluate({"theta": 0.37}), expected, rtol=0, atol=1e-12
    )
