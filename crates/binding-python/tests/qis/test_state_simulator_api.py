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

"""Cross-simulator tests for shared state simulation APIs."""

import math

import numpy as np
import pytest

from cqlib.circuit import Circuit
from cqlib.device import Outcome
from cqlib.qis import DensityMatrix, DensityMatrixNoise, Statevector


IDEAL_SIMULATORS = [Statevector, DensityMatrix, DensityMatrixNoise]


def bell_circuit():
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)
    return circuit


@pytest.mark.parametrize("simulator", IDEAL_SIMULATORS)
def test_apply_circuit_in_place_creates_bell_state(simulator):
    state = simulator(2)

    state.apply_circuit(bell_circuit())

    probs = state.probabilities()
    assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
    assert math.isclose(probs[1], 0.0, abs_tol=1e-10)
    assert math.isclose(probs[2], 0.0, abs_tol=1e-10)
    assert math.isclose(probs[3], 0.5, abs_tol=1e-10)


@pytest.mark.parametrize("simulator", IDEAL_SIMULATORS)
def test_apply_circuit_rejects_dimension_mismatch(simulator):
    state = simulator(1)

    with pytest.raises((ValueError, IndexError)):
        state.apply_circuit(bell_circuit())


@pytest.mark.parametrize("simulator", IDEAL_SIMULATORS)
def test_measure_all_returns_outcome_and_collapses_state(simulator):
    state = simulator(3)
    state.apply_x(0)
    state.apply_x(2)

    outcome = state.measure_all()

    assert isinstance(outcome, Outcome)
    assert outcome.to_bitstring(3) == "101"
    probs = state.probabilities()
    assert math.isclose(probs[0b101], 1.0, abs_tol=1e-10)


@pytest.mark.parametrize("simulator", IDEAL_SIMULATORS)
def test_sample_shots_preserves_bell_correlations_without_mutating(simulator):
    state = simulator(2)
    state.apply_circuit(bell_circuit())

    shots = state.sample_shots(64)

    assert len(shots) == 64
    assert all(isinstance(outcome, Outcome) for outcome in shots)
    assert {outcome.to_bitstring(2) for outcome in shots} <= {"00", "11"}
    probs = state.probabilities()
    assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
    assert math.isclose(probs[3], 0.5, abs_tol=1e-10)


@pytest.mark.parametrize("simulator", IDEAL_SIMULATORS)
def test_measure_deterministic_one_qubit_state(simulator):
    state = simulator(1)
    state.apply_x(0)

    assert state.measure(0) is True
    assert math.isclose(state.probabilities()[1], 1.0, abs_tol=1e-10)


def test_statevector_apply_unitary_gate_multi_qubit():
    state = Statevector(2)
    state.apply_x(0)

    swap = np.array(
        [
            [1, 0, 0, 0],
            [0, 0, 1, 0],
            [0, 1, 0, 0],
            [0, 0, 0, 1],
        ],
        dtype=complex,
    )
    state.apply_unitary_gate([0, 1], swap)

    probs = state.probabilities()
    assert math.isclose(probs[0b10], 1.0, abs_tol=1e-10)
