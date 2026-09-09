# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# Licensed under the Apache License, Version 2.0. See LICENSE.txt or
# http://www.apache.org/licenses/LICENSE-2.0.

"""Regression tests for measurement declarations, execution, and reset."""

import numpy as np
import pytest

from cqlib.circuit import Circuit, ClassicalExpr, ClassicalType
from cqlib.qis import DensityMatrix, DensityMatrixNoise, StabilizerState, Statevector


SIMULATORS = [Statevector, DensityMatrix, DensityMatrixNoise, StabilizerState]


def state_data(state):
    if isinstance(state, StabilizerState):
        return state.to_stim_format()
    if isinstance(state, DensityMatrixNoise):
        return state.state
    return state.data


@pytest.mark.parametrize("simulator", SIMULATORS)
@pytest.mark.parametrize("measurement", ["measure", "measure_bits", "measure_into"])
@pytest.mark.parametrize("following", ["h", "cx", "reset", "nested"])
def test_mid_circuit_measurement_rejected_before_evolution(
    simulator, measurement, following
):
    circuit = Circuit(2)
    circuit.h(0)
    if measurement == "measure_bits":
        circuit.measure_bits([1, 0])
    elif measurement == "measure_into":
        circuit.measure_into(0, circuit.var(ClassicalType.bit()))
    else:
        circuit.measure(0)
    circuit.barrier([0, 1])
    circuit.delay(0, 1.0)
    if following == "nested":
        child = Circuit(1)
        child.h(0)
        circuit.append_circuit_gate(child.to_gate("Hadamard"), [0])
    elif following == "cx":
        circuit.cx(1, 0)
    else:
        getattr(circuit, following)(0)

    with pytest.raises(ValueError, match="mid-circuit measurement on qubit"):
        simulator.from_circuit(circuit)

    state = simulator(2)
    state.apply_h(1)
    before = state_data(state)
    with pytest.raises(ValueError, match="mid-circuit measurement"):
        state.apply_circuit(circuit)
    np.testing.assert_equal(state_data(state), before)


@pytest.mark.parametrize("simulator", SIMULATORS)
def test_terminal_declarations_allow_independent_gates_stores_and_repeated_measurements(
    simulator,
):
    circuit = Circuit(2)
    circuit.h(0)
    circuit.measure_into(0, circuit.var(ClassicalType.bit()))
    circuit.barrier([0, 1])
    circuit.delay(0, 1.0)
    circuit.h(1)
    circuit.measure_bits([1, 0])
    flag = circuit.var(ClassicalType.bool())
    circuit.store(flag, ClassicalExpr.bool_literal(True))

    state = simulator.from_circuit(circuit)
    applied = simulator(2)
    applied.apply_circuit(circuit)
    for result in [state, applied]:
        np.testing.assert_allclose(result.probabilities(), [0.25] * 4, atol=1e-12)
        # The declarations did not collapse either |+> state.
        result.apply_h(0)
        result.apply_h(1)
        np.testing.assert_allclose(result.probabilities(), [1, 0, 0, 0], atol=1e-12)


@pytest.mark.parametrize("simulator", SIMULATORS)
def test_classical_control_flow_remains_unsupported(simulator):
    circuit = Circuit(1)
    circuit.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))
    with pytest.raises(ValueError, match="classical control flow"):
        simulator.from_circuit(circuit)


def test_stabilizer_execution_keeps_mid_circuit_result_and_conditional_state():
    circuit = Circuit(1)
    circuit.h(0)
    measured = circuit.measure(0)
    circuit.h(0)

    result = StabilizerState.run_circuit(circuit)
    bit = result.classical.value(measured.value).as_bit()
    state = result.state
    np.testing.assert_allclose(state.probabilities(), [0.5, 0.5], atol=1e-12)
    state.apply_h(0)
    np.testing.assert_allclose(state.probabilities(), [not bit, bit], atol=1e-12)


@pytest.mark.parametrize("simulator", [DensityMatrix, DensityMatrixNoise])
@pytest.mark.parametrize(
    "qubit, expected", [(0, [0.5, 0, 0.5, 0]), (1, [0.5, 0.5, 0, 0])]
)
def test_circuit_reset_keeps_bell_partner_mixed(simulator, qubit, expected):
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)
    circuit.reset(qubit)
    circuit.measure_bits([0, 1])

    state = simulator.from_circuit(circuit)
    applied = simulator(2)
    applied.apply_circuit(circuit)
    for result in [state, applied]:
        np.testing.assert_allclose(state_data(result), np.diag(expected), atol=1e-12)


def test_density_matrix_reset_preserves_complex_coherence_of_other_qubit():
    state = DensityMatrix(2)
    state.apply_h(0)
    state.apply_h(1)
    state.apply_s(1)
    before = state.partial_trace([1]).data

    state.reset(0)

    np.testing.assert_allclose(state.partial_trace([1]).data, before, atol=1e-12)
    np.testing.assert_allclose(
        state.data, np.kron(before, [[1, 0], [0, 0]]), atol=1e-12
    )


@pytest.mark.parametrize("simulator", [Statevector, StabilizerState])
def test_pure_state_reset_remains_a_single_trajectory(simulator):
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)
    circuit.reset(0)

    state = simulator.from_circuit(circuit)
    probs = state.probabilities()
    assert np.isclose(probs[0], 1.0) or np.isclose(probs[2], 1.0)
    np.testing.assert_allclose([probs[1], probs[3]], [0, 0], atol=1e-12)
