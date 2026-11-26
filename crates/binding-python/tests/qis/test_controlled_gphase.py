# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# Licensed under the Apache License, Version 2.0. See LICENSE.txt or
# http://www.apache.org/licenses/LICENSE-2.0.

"""Controlled zero-target phase evolution in the numerical simulators."""

import numpy as np
import pytest

from cqlib import Circuit, Parameter
from cqlib.circuit import StandardGate
from cqlib.circuit.gates import GPhase
from cqlib.device import NoiseModel, SingleQubitNoise
from cqlib.qis import DensityMatrix, DensityMatrixNoise, Statevector


SIMULATORS = [Statevector, DensityMatrix, DensityMatrixNoise]


def assert_state(state, amplitudes):
    if isinstance(state, Statevector):
        actual, expected = state.data, amplitudes
    else:
        actual = state.state if isinstance(state, DensityMatrixNoise) else state.data
        expected = np.outer(amplitudes, amplitudes.conj())
    np.testing.assert_allclose(actual, expected, rtol=0, atol=1e-12)


@pytest.mark.parametrize("simulator", SIMULATORS)
@pytest.mark.parametrize("num_qubits", [0, 1, 3])
def test_controlled_gphase_preserves_uncontrolled_phase(simulator, num_qubits):
    circuit = Circuit(num_qubits)
    expected = np.zeros(1 << num_qubits, dtype=complex)
    expected[0] = 1
    if num_qubits:
        circuit.h(0)
        expected[:2] = 1 / np.sqrt(2)
    circuit.append_gate(GPhase(0.37), [])
    circuit.set_global_phase(-0.21)
    expected *= np.exp(0.16j)
    assert_state(simulator.from_circuit(circuit), expected)


@pytest.mark.parametrize("simulator", SIMULATORS)
@pytest.mark.parametrize("theta", [0.0, 0.37, -0.51, np.pi])
@pytest.mark.parametrize("initial", ["zero", "one", "plus"])
@pytest.mark.parametrize("incremental", [False, True])
def test_controlled_gphase_single_qubit(simulator, theta, initial, incremental):
    circuit = Circuit(1)
    expected = np.array([1, 0], dtype=complex)
    if initial == "one":
        circuit.x(0)
        expected = np.array([0, 1], dtype=complex)
    elif initial == "plus":
        circuit.h(0)
        expected = np.ones(2, dtype=complex) / np.sqrt(2)
    if incremental:
        state = simulator.from_circuit(circuit)
        circuit = Circuit(1)
    circuit.append_mc_gate(GPhase(theta).control(1), [0])
    expected[1] *= np.exp(1j * theta)
    if incremental:
        state.apply_circuit(circuit)
    else:
        state = simulator.from_circuit(circuit)
    assert_state(state, expected)


@pytest.mark.parametrize("simulator", SIMULATORS)
@pytest.mark.parametrize("labels", [[0, 1, 2], [12, 5, 31]])
@pytest.mark.parametrize("positions", [[2], [2, 0], [0, 2]])
def test_controlled_gphase_labels_spectators_and_inverse(simulator, labels, positions):
    circuit = Circuit(labels)
    for qubit in labels:
        circuit.h(qubit)
    circuit.s(labels[1])
    controls = [labels[position] for position in positions]
    phase = Circuit(labels)
    gate = GPhase(0.37).control(len(controls))
    phase.append_mc_gate(gate, controls)
    incremental = simulator.from_circuit(circuit)
    incremental.apply_circuit(phase)
    circuit.append_mc_gate(gate, controls)
    direct = simulator.from_circuit(circuit)
    indices = np.arange(8)
    baseline = np.where(indices & 2, 1j, 1) / np.sqrt(8)
    mask = sum(1 << position for position in positions)
    expected = baseline * np.where(indices & mask == mask, np.exp(0.37j), 1)
    assert_state(direct, expected)
    assert_state(incremental, expected)
    inverse = Circuit(labels)
    inverse.append_mc_gate(gate.inverse(), controls)
    incremental.apply_circuit(inverse)
    assert_state(incremental, baseline)


@pytest.mark.parametrize("simulator", SIMULATORS)
def test_controlled_gphase_parameter_resolution(simulator):
    circuit = Circuit(1)
    circuit.h(0)
    circuit.append_mc_gate(GPhase(Parameter("theta")).control(1), [0])
    with pytest.raises(ValueError, match="(?i)symbolic"):
        simulator.from_circuit(circuit)
    state = simulator(1)
    with pytest.raises(ValueError, match="(?i)symbolic"):
        state.apply_circuit(circuit)
    bound = circuit.assign_parameters({"theta": -0.51})
    expected = np.array([1, np.exp(-0.51j)]) / np.sqrt(2)
    assert_state(simulator.from_circuit(bound), expected)


@pytest.mark.parametrize("empty_model", [False, True])
def test_controlled_gphase_noisy_simulator_without_errors(empty_model):
    circuit = Circuit(1)
    circuit.h(0)
    circuit.append_mc_gate(GPhase(0.37).control(1), [0])
    model = NoiseModel() if empty_model else None
    state = DensityMatrixNoise.from_circuit(circuit, model)
    assert_state(state, np.array([1, np.exp(0.37j)]) / np.sqrt(2))


@pytest.mark.parametrize("incremental", [False, True])
def test_controlled_gphase_noise_follows_unitary_once_with_original_labels(incremental):
    labels = [12, 5]
    circuit = Circuit(labels)
    circuit.x(12)
    circuit.h(5)
    model = NoiseModel()
    probability = 0.23
    model.add_single_qubit_error(
        StandardGate.GPhase, 5, SingleQubitNoise.bit_flip(probability)
    )
    model.add_single_qubit_error(
        StandardGate.GPhase, 12, SingleQubitNoise.bit_flip(1.0)
    )
    if incremental:
        state = DensityMatrixNoise.from_circuit(circuit, model)
        circuit = Circuit(labels)
    circuit.append_mc_gate(GPhase(0.37).control(1), [5])
    if incremental:
        state.apply_circuit(circuit)
    else:
        state = DensityMatrixNoise.from_circuit(circuit, model)
    amplitudes = np.array([0, 1, 0, np.exp(0.37j)]) / np.sqrt(2)
    rho = np.outer(amplitudes, amplitudes.conj())
    flipped = np.arange(4) ^ 2
    expected = (1 - probability) * rho + probability * rho[np.ix_(flipped, flipped)]
    np.testing.assert_allclose(state.state, expected, rtol=0, atol=1e-12)


@pytest.mark.parametrize("simulator", SIMULATORS)
@pytest.mark.parametrize("num_qubits", [0, 1, 3])
@pytest.mark.parametrize("theta", [0.0, 0.37, -0.51, np.pi])
@pytest.mark.parametrize("incremental", [False, True])
def test_zero_control_gphase_evolution(simulator, num_qubits, theta, incremental):
    preparation = Circuit(num_qubits)
    expected = np.zeros(1 << num_qubits, dtype=complex)
    expected[0] = 1
    if num_qubits:
        preparation.h(0)
        expected[:2] = 1 / np.sqrt(2)
    circuit = Circuit(num_qubits) if incremental else preparation
    circuit.append_mc_gate(GPhase(theta).control(0), [])
    circuit.append_mc_gate(GPhase(-0.21).control(0), [])
    circuit.set_global_phase(0.11)
    expected *= np.exp(1j * (theta - 0.10))
    if incremental:
        state = simulator.from_circuit(preparation)
        state.apply_circuit(circuit)
    else:
        state = simulator.from_circuit(circuit)
    assert_state(state, expected)
    inverse = Circuit(num_qubits)
    inverse.append_mc_gate(GPhase(theta - 0.10).control(0).inverse(), [])
    state.apply_circuit(inverse)
    assert_state(state, expected * np.exp(-1j * (theta - 0.10)))


@pytest.mark.parametrize("simulator", SIMULATORS)
def test_zero_control_gphase_parameter_resolution(simulator):
    circuit = Circuit([12, 5])
    circuit.h(5)
    circuit.append_mc_gate(GPhase(Parameter("theta")).control(0), [])
    with pytest.raises(ValueError, match="(?i)symbolic"):
        simulator.from_circuit(circuit)
    with pytest.raises(ValueError, match="(?i)symbolic"):
        simulator(2).apply_circuit(circuit)
    bound = circuit.assign_parameters({"theta": -0.51})
    expected = np.array([1, 0, 1, 0], dtype=complex) * np.exp(-0.51j) / np.sqrt(2)
    assert_state(simulator.from_circuit(bound), expected)


@pytest.mark.parametrize("empty_model", [False, True])
@pytest.mark.parametrize("incremental", [False, True])
def test_zero_control_gphase_noise_matches_plain_gate(empty_model, incremental):
    model = NoiseModel()
    if not empty_model:
        model.add_single_qubit_error(
            StandardGate.GPhase, 12, SingleQubitNoise.bit_flip(1.0)
        )
        model.add_single_qubit_error(
            StandardGate.H, 5, SingleQubitNoise.depolarizing(0.23)
        )
    preparation = Circuit([12, 5])
    preparation.x(12)
    preparation.h(5)
    results = []
    for wrapped in [False, True]:
        circuit = Circuit([12, 5])
        if not incremental:
            circuit.x(12)
            circuit.h(5)
        if wrapped:
            circuit.append_mc_gate(GPhase(0.37).control(0), [])
        else:
            circuit.append_gate(GPhase(0.37), [])
        if incremental:
            state = DensityMatrixNoise.from_circuit(preparation, model)
            state.apply_circuit(circuit)
        else:
            state = DensityMatrixNoise.from_circuit(circuit, model)
        results.append(state.state)
    np.testing.assert_allclose(results[0], results[1], rtol=0, atol=1e-12)
