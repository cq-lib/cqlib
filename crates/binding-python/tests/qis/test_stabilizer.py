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

"""Tests for StabilizerState Clifford simulation."""

import math

import pytest

from cqlib.circuit import Circuit, ClassicalExpr, ClassicalType
from cqlib.device import Outcome
from cqlib.qis import (
    PauliString,
    RuntimeValue,
    StabilizerCircuitResult,
    StabilizerState,
)
from cqlib.qis.state import StabilizerState as StateModuleStabilizerState


def assert_distribution_close(actual, expected, tol=1e-10):
    """Assert a full probability distribution matches sparse expected values."""
    assert len(actual) == max(expected.keys(), default=0) + 1 or len(actual) >= len(
        expected
    )
    for index, value in expected.items():
        assert math.isclose(actual[index], value, abs_tol=tol), (
            f"probability[{index}] expected {value}, got {actual[index]}"
        )
    unexpected_mass = sum(
        value for index, value in enumerate(actual) if index not in expected
    )
    assert math.isclose(unexpected_mass, 0.0, abs_tol=tol)


class TestStabilizerConstruction:
    """Test StabilizerState construction and exports."""

    def test_new_initializes_zero_state(self):
        state = StabilizerState(3)

        assert state.num_qubits == 3
        assert_distribution_close(state.probabilities(), {0: 1.0})
        assert "StabilizerState" in repr(state)

    def test_imports_from_qis_and_state_module_match(self):
        assert StabilizerState is StateModuleStabilizerState

    def test_from_circuit_bell_state(self):
        circuit = Circuit(2)
        circuit.h(0)
        circuit.cx(0, 1)

        state = StabilizerState.from_circuit(circuit)

        assert state.num_qubits == 2
        assert_distribution_close(state.probabilities(), {0b00: 0.5, 0b11: 0.5})

    def test_from_circuit_rejects_non_clifford_gate(self):
        circuit = Circuit(1)
        circuit.t(0)

        with pytest.raises(ValueError):
            StabilizerState.from_circuit(circuit)


class TestStabilizerGates:
    """Test supported Clifford gates."""

    def test_single_qubit_clifford_gates(self):
        state = StabilizerState(1)
        state.apply_x(0)
        assert_distribution_close(state.probabilities(), {1: 1.0})

        state.apply_y(0)
        assert_distribution_close(state.probabilities(), {0: 1.0})

        state.apply_h(0)
        assert_distribution_close(state.probabilities(), {0: 0.5, 1: 0.5})

        state.apply_s(0)
        state.apply_sdg(0)
        assert_distribution_close(state.probabilities(), {0: 0.5, 1: 0.5})

    @pytest.mark.parametrize("gate", ["x2p", "x2m", "y2p", "y2m"])
    def test_half_rotation_clifford_gates_create_balanced_state(self, gate):
        state = StabilizerState(1)

        getattr(state, f"apply_{gate}")(0)

        assert_distribution_close(state.probabilities(), {0: 0.5, 1: 0.5})

    def test_two_qubit_clifford_gates(self):
        state = StabilizerState(2)
        state.apply_x(0)
        state.apply_cx(0, 1)
        assert_distribution_close(state.probabilities(), {0b11: 1.0})

        state.apply_swap(0, 1)
        assert_distribution_close(state.probabilities(), {0b11: 1.0})

        state.apply_z(0)
        state.apply_cz(0, 1)
        state.apply_cy(0, 1)
        assert math.isclose(sum(state.probabilities()), 1.0, abs_tol=1e-10)

    def test_copy_is_independent(self):
        original = StabilizerState(2)
        original.apply_h(0)
        original.apply_cx(0, 1)

        copied = original.copy()
        original.apply_x(0)

        assert_distribution_close(copied.probabilities(), {0b00: 0.5, 0b11: 0.5})


class TestStabilizerMeasurement:
    """Test measurement, reset, and sampling behavior."""

    def test_measure_deterministic_state_collapses(self):
        state = StabilizerState(1)
        state.apply_x(0)

        assert state.measure(0) is True
        assert_distribution_close(state.probabilities(), {1: 1.0})

    def test_measure_all_returns_outcome(self):
        state = StabilizerState(3)
        state.apply_x(0)
        state.apply_x(2)

        outcome = state.measure_all()

        assert isinstance(outcome, Outcome)
        assert outcome.to_bitstring(3) == "101"
        assert_distribution_close(state.probabilities(), {0b101: 1.0})

    def test_reset_returns_qubit_to_zero(self):
        state = StabilizerState(1)
        state.apply_x(0)
        state.reset(0)

        assert_distribution_close(state.probabilities(), {0: 1.0})

    def test_sample_shots_preserves_bell_correlations_and_does_not_mutate(self):
        state = StabilizerState(2)
        state.apply_h(0)
        state.apply_cx(0, 1)

        shots = state.sample_shots(64)

        assert len(shots) == 64
        assert all(isinstance(outcome, Outcome) for outcome in shots)
        assert {outcome.to_bitstring(2) for outcome in shots} <= {"00", "11"}
        assert_distribution_close(state.probabilities(), {0b00: 0.5, 0b11: 0.5})


class TestStabilizerProbabilitiesAndObservables:
    """Test probability and Pauli observable APIs."""

    def test_probability_of_bell_state(self):
        state = StabilizerState(2)
        state.apply_h(0)
        state.apply_cx(0, 1)

        assert math.isclose(state.probability_of([False, False]), 0.5, abs_tol=1e-10)
        assert math.isclose(state.probability_of([True, True]), 0.5, abs_tol=1e-10)
        assert math.isclose(state.probability_of([True, False]), 0.0, abs_tol=1e-10)

    def test_probability_of_rejects_wrong_bit_count(self):
        state = StabilizerState(2)

        with pytest.raises(ValueError):
            state.probability_of([False])

    def test_pauli_expectation_for_bell_state(self):
        state = StabilizerState(2)
        state.apply_h(0)
        state.apply_cx(0, 1)

        assert state.pauli_expectation(PauliString.from_str("ZZ")) == 1
        assert state.pauli_expectation(PauliString.from_str("XX")) == 1
        assert state.pauli_expectation(PauliString.from_str("ZI")) == 0

    def test_stabilizer_generators_and_stim_format(self):
        state = StabilizerState(2)
        state.apply_h(0)
        state.apply_cx(0, 1)

        stabilizers = state.get_stabilizers()
        destabilizers = state.get_destabilizers()
        stim_text = state.to_stim_format()

        assert len(stabilizers) == 2
        assert len(destabilizers) == 2
        assert all(isinstance(pauli, PauliString) for pauli in stabilizers)
        assert stim_text.count("\n") == 2
        assert any(str(pauli) in stim_text for pauli in stabilizers)


class TestStabilizerCircuitExecution:
    """Test Clifford circuit execution with mid-circuit directives."""

    def test_apply_circuit_returns_measurement_register(self):
        circuit = Circuit(2)
        circuit.x(0)
        measurement = circuit.measure(0)
        circuit.reset(0)
        circuit.h(1)

        result = StabilizerState.run_circuit(circuit)

        assert isinstance(result, StabilizerCircuitResult)
        measured = result.classical.value(measurement.value)
        assert isinstance(measured, RuntimeValue)
        assert measured.kind == "bit"
        assert measured.as_bit() is True
        assert measured.to_bitstring() == "1"
        assert_distribution_close(result.state.probabilities(), {0b00: 0.5, 0b10: 0.5})
        assert "StabilizerCircuitResult" in repr(result)

    def test_apply_circuit_returns_stored_classical_var(self):
        circuit = Circuit(1)
        flag = circuit.var(ClassicalType.bool())
        circuit.store(flag, ClassicalExpr.bool_literal(True))

        result = StabilizerState.run_circuit(circuit)

        stored = result.classical.var(flag)
        assert isinstance(stored, RuntimeValue)
        assert stored.kind == "bool"
        assert stored.as_bool() is True
        assert stored.to_bitstring() is None

    def test_apply_circuit_rejects_control_flow(self):
        circuit = Circuit(1)
        circuit.h(0)
        circuit.measure(0)
        circuit.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))

        with pytest.raises(ValueError):
            StabilizerState.run_circuit(circuit)


class TestStabilizerErrors:
    """Test error handling for invalid operations."""

    def test_qubit_out_of_bounds(self):
        state = StabilizerState(2)

        with pytest.raises(IndexError):
            state.apply_h(2)

        with pytest.raises(IndexError):
            state.measure(2)

    def test_two_qubit_gates_reject_duplicate_qubits(self):
        state = StabilizerState(2)

        with pytest.raises(ValueError):
            state.apply_cx(0, 0)

        with pytest.raises(ValueError):
            state.apply_swap(1, 1)
