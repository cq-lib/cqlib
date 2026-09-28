# This code is part of Cqlib.
# Modified to verify seeded sampling across state backends.
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

"""Direct circuit sampling, output selection, and supported execution semantics."""

from collections import Counter
import copy
import os
import subprocess
import sys

import numpy as np
import pytest

from cqlib import Circuit, ExecutionResult, Parameter, Qubit, sample
from cqlib.circuit import ClassicalExpr, ClassicalType
from cqlib.qis import (
    DensityMatrix,
    DensityMatrixNoise,
    StabilizerState,
    Statevector,
    sample as qis_sample,
)


SIMULATORS = ["statevector", "density_matrix", "stabilizer"]
STATE_TYPES = [Statevector, DensityMatrix, DensityMatrixNoise, StabilizerState]


@pytest.mark.parametrize("simulator", SIMULATORS)
def test_bell_sampling_returns_completed_result(simulator):
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)
    before = copy.copy(circuit)

    result = sample(circuit, shots=1000, seed=42, simulator=simulator)

    assert qis_sample is sample
    assert isinstance(result, ExecutionResult)
    assert result.status.kind == "completed"
    assert result.shots == sum(result.counts.values()) == 1000
    assert result.backend == simulator
    assert result.qubits == [Qubit(0), Qubit(1)]
    assert set(result.counts) == {"00", "11"}
    assert 400 < result.counts["00"] < 600
    assert result.probabilities == pytest.approx(
        {key: count / 1000 for key, count in result.counts.items()}
    )
    assert (
        sample(circuit, shots=1000, seed=42, simulator=simulator).counts
        == result.counts
    )
    assert circuit == before


def test_defaults_include_idle_qubits_in_circuit_order():
    circuit = Circuit([9, 5, 2])
    circuit.x(9)

    result = sample(circuit)

    assert result.backend == "statevector"
    assert result.counts == {"001": 1000}
    assert result.qubits == circuit.qubits


@pytest.mark.parametrize("simulator", SIMULATORS)
def test_terminal_declarations_and_explicit_output_order(simulator):
    circuit = Circuit([9, 5, 2])
    circuit.x(9)
    circuit.measure_into(9, circuit.var(ClassicalType.bit()))
    circuit.barrier([9, 5, 2])
    circuit.delay(9, 1.0)
    circuit.x(2)  # Independent gates after a measurement are supported.
    circuit.measure_bits([5, 9])
    before = copy.copy(circuit)

    result = sample(circuit, shots=8, simulator=simulator)
    assert result.counts == {"01": 8}
    assert result.qubits == [Qubit(9), Qubit(5)]
    assert result.num_qubits == 2

    reverse = sample(circuit, shots=8, simulator=simulator, qubits=[5, 9])
    assert reverse.counts == {"10": 8}
    assert reverse.qubits == [Qubit(5), Qubit(9)]
    subset = sample(circuit, shots=8, simulator=simulator, qubits=[Qubit(2)])
    assert subset.counts == {"1": 8}
    assert circuit == before


@pytest.mark.parametrize("simulator", SIMULATORS)
def test_mid_circuit_measurement_cannot_be_bypassed_by_output_selection(simulator):
    circuit = Circuit(2)
    circuit.measure(0)
    child = Circuit(1)
    child.h(0)
    circuit.append_circuit_gate(child.to_gate("Hadamard"), [0])

    with pytest.raises(ValueError, match="mid-circuit measurement"):
        sample(circuit, simulator=simulator, qubits=[1])


@pytest.mark.parametrize("simulator", SIMULATORS)
def test_control_flow_is_rejected(simulator):
    circuit = Circuit(1)
    circuit.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))
    with pytest.raises(ValueError, match="classical control flow"):
        sample(circuit, simulator=simulator)


def test_reset_samples_the_mixed_ensemble():
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)
    circuit.reset(0)

    result = sample(circuit, shots=1000, seed=42, simulator="density_matrix")
    assert set(result.counts) == {"00", "10"}
    assert 400 < result.counts["00"] < 600
    for simulator in ["statevector", "stabilizer"]:
        with pytest.raises(ValueError, match="reset requires the density_matrix"):
            sample(circuit, simulator=simulator)


def test_stabilizer_samples_large_outputs_without_probability_enumeration():
    circuit = Circuit(128)
    circuit.h(0)
    for qubit in range(1, 128):
        circuit.cx(0, qubit)

    result = sample(circuit, shots=32, seed=42, simulator="stabilizer")
    assert set(result.counts) == {"0" * 128, "1" * 128}
    assert sum(result.counts.values()) == 32


@pytest.mark.parametrize("simulator", SIMULATORS)
def test_unbound_parameters_are_rejected(simulator):
    circuit = Circuit(1)
    circuit.ry(0, Parameter("theta"))
    with pytest.raises(ValueError):
        sample(circuit, simulator=simulator)


def test_non_clifford_gates_require_a_general_simulator():
    circuit = Circuit(1)
    circuit.ry(0, 0.3)
    for simulator in ["statevector", "density_matrix"]:
        assert (
            sum(sample(circuit, shots=16, seed=42, simulator=simulator).counts.values())
            == 16
        )
    with pytest.raises(ValueError):
        sample(circuit, simulator="stabilizer")


@pytest.mark.parametrize("angle", [0.2, 2.0])
def test_density_matrix_sampling_tolerates_roundoff(angle):
    circuit = Circuit(1)
    circuit.ry(0, angle)
    circuit.ry(0, -angle)
    assert sample(circuit, simulator="density_matrix", seed=42).counts == {"0": 1000}


@pytest.mark.parametrize(
    "options, error",
    [
        ({"shots": 0}, ValueError),
        ({"shots": -1}, OverflowError),
        ({"shots": 1.5}, TypeError),
        ({"seed": -1}, OverflowError),
        ({"seed": 2**64}, OverflowError),
        ({"simulator": "unknown"}, ValueError),
        ({"qubits": []}, ValueError),
        ({"qubits": [0, 0]}, ValueError),
        ({"qubits": [2]}, ValueError),
    ],
)
def test_invalid_sampling_options(options, error):
    with pytest.raises(error):
        sample(Circuit(1), **options)


def test_zero_qubit_circuit_is_rejected():
    with pytest.raises(ValueError, match="at least one output qubit"):
        sample(Circuit(0))


def test_seed_is_independent_of_worker_count():
    script = """
import json
from cqlib import Circuit, sample
circuit = Circuit(4)
for qubit in range(4):
    circuit.h(qubit)
print(json.dumps({
    simulator: sample(circuit, shots=256, seed=42, simulator=simulator).counts
    for simulator in ['statevector', 'density_matrix', 'stabilizer']
}, sort_keys=True))
"""
    results = [
        subprocess.check_output(
            [sys.executable, "-c", script],
            env={**os.environ, "RAYON_NUM_THREADS": str(workers)},
            text=True,
        )
        for workers in [1, 4]
    ]
    assert results[0] == results[1]


def test_state_sampling_seed_is_independent_of_workers_and_processes():
    script = """
import json
from cqlib import Circuit
from cqlib.qis import Statevector, DensityMatrix, DensityMatrixNoise, StabilizerState
circuit = Circuit(4)
for qubit in range(4):
    circuit.h(qubit)
measurement = circuit.measure_bits([3, 0, 2])
results = {}
for state_type in [Statevector, DensityMatrix, DensityMatrixNoise, StabilizerState]:
    state = state_type.from_circuit(circuit)
    results[state_type.__name__] = {
        "shots": [outcome.to_bitstring(4) for outcome in state.sample_shots(256, seed=42)],
        "counts": state.sample(measurement, 256, seed=42).counts,
    }
print(json.dumps(results, sort_keys=True))
"""
    results = [
        subprocess.check_output(
            [sys.executable, "-c", script],
            env={**os.environ, "RAYON_NUM_THREADS": str(workers)},
            text=True,
            timeout=45,
        )
        for workers in [1, 2, 4, 1]
    ]
    assert all(result == results[0] for result in results[1:])


@pytest.mark.parametrize("state_type", STATE_TYPES)
def test_seeded_shot_sequence_replays_and_keeps_prefix(state_type, bell_state_circuit):
    state = state_type.from_circuit(bell_state_circuit)
    before = state.probabilities()

    def draw(shots, **kwargs):
        return [o.to_bitstring(2) for o in state.sample_shots(shots, **kwargs)]

    first = draw(512, seed=42)
    assert len(draw(17)) == 17
    assert len(draw(17, seed=None)) == 17
    assert draw(512, seed=42) == first
    assert draw(128, seed=42) == first[:128]
    assert len(first) == 512 and set(first) <= {"00", "11"}
    np.testing.assert_array_equal(state.probabilities(), before)


@pytest.mark.parametrize("state_type", STATE_TYPES)
def test_seeded_measurement_counts_replay_with_sparse_qubit_order(state_type):
    circuit = Circuit([9, 5, 12])
    circuit.h(9)
    circuit.cx(9, 5)
    measurement = circuit.measure_bits([12, 9])
    state = state_type.from_circuit(circuit)
    before = state.probabilities()
    result = state.sample(measurement, 512, seed=42)
    assert result.counts == state.sample(measurement, 512, seed=42).counts
    assert sum(result.counts.values()) == 512
    assert set(result.counts) <= {"00", "10"}
    # Full shots and measurement counts use the same seeded stream.
    full = state.sample_shots(512, seed=42)
    projected = Counter(
        "10" if shot.to_bitstring(3)[-1] == "1" else "00" for shot in full
    )
    assert result.counts == projected
    np.testing.assert_array_equal(state.probabilities(), before)


@pytest.mark.parametrize("state_type", STATE_TYPES)
@pytest.mark.parametrize("seed", [0, 42, 2**64 - 1])
def test_seed_boundaries_and_concurrent_calls(state_type, seed, bell_state_circuit):
    from concurrent.futures import ThreadPoolExecutor

    state = state_type.from_circuit(bell_state_circuit)

    def draw(value):
        return [o.to_bitstring(2) for o in state.sample_shots(128, seed=value)]

    seeds = [seed, 17, seed, 23]
    expected = [draw(s) for s in seeds]
    with ThreadPoolExecutor(4) as executor:
        assert list(executor.map(draw, seeds)) == expected
    assert state.sample_shots(0, seed=seed) == []


@pytest.mark.parametrize("state_type", STATE_TYPES)
@pytest.mark.parametrize(
    "seed,error", [(-1, OverflowError), (2**64, OverflowError), (1.5, TypeError)]
)
def test_invalid_sampling_seed_is_rejected(state_type, seed, error, bell_state_circuit):
    state = state_type.from_circuit(bell_state_circuit)
    measurement = bell_state_circuit.measure_bits([0, 1])
    with pytest.raises(error):
        state.sample_shots(8, seed=seed)
    with pytest.raises(error):
        state.sample(measurement, 8, seed=seed)


@pytest.mark.parametrize("state_type", STATE_TYPES)
@pytest.mark.parametrize("seed", [0, 42])
def test_seed_controls_the_random_sequence(state_type, seed, bell_state_circuit):
    if sys.maxsize <= 2**32:
        pytest.skip("SmallRng uses a different algorithm on 32-bit targets")
    expected = {
        0: "00011111100000000000011011110011",
        42: "11010011100010000010111011100010",
    }
    stabilizer_expected = {
        0: "11100000011111111111100100001100",
        42: "00101100011101111101000100011101",
    }
    state = state_type.from_circuit(bell_state_circuit)
    sequence = "".join(
        str(int(o.to_bitstring(2) == "11")) for o in state.sample_shots(32, seed=seed)
    )
    # Golden sequences for the current seeded sampler, not a promise across
    # package versions. Detects a binding that accepts but ignores the seed.
    assert (
        sequence
        == (
            stabilizer_expected
            if state_type.__name__ == "StabilizerState"
            else expected
        )[seed]
    )
