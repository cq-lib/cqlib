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

"""QIS Python binding integration tests."""

import importlib

import pytest

from cqlib.circuit import Circuit, ClassicalExpr, ClassicalType, Parameter
from cqlib.device import NoiseModel, Outcome, ReadoutError
from cqlib.qis import (
    DensityMatrix,
    DensityMatrixNoise,
    RuntimeValue,
    StabilizerState,
    Statevector,
)


def test_public_qis_submodules_are_importable():
    for module in [
        "cqlib.qis.pauli",
        "cqlib.qis.hamiltonian",
        "cqlib.qis.evolution",
        "cqlib.qis.entropy",
        "cqlib.qis.metrics",
        "cqlib.qis.state.statevector",
        "cqlib.qis.state.density_matrix",
        "cqlib.qis.state.density_matrix_noise",
        "cqlib.qis.state.classical",
        "cqlib.qis.state.stabilizer",
    ]:
        importlib.import_module(module)


def test_state_measurement_probs_and_sample_bindings():
    circuit = Circuit(2)
    measurement = circuit.measure_bits([1, 0])

    for state in [
        Statevector(2),
        DensityMatrix(2),
        StabilizerState(2),
        DensityMatrixNoise(2),
    ]:
        probs = state.probs(measurement)
        assert probs == {Outcome("00"): 1.0}

        result = state.sample(measurement, 8)
        assert result.shots == 8
        assert result.num_qubits == 2
        assert result.counts == {"00": 8}


@pytest.mark.parametrize(
    "state_type", [Statevector, DensityMatrix, StabilizerState, DensityMatrixNoise]
)
@pytest.mark.parametrize("qubits", [[0, 1], [1, 0], [5, 9], [9, 5]])
@pytest.mark.parametrize("apply_in_place", [False, True])
def test_state_measurement_resolves_circuit_qubit_positions(
    state_type, qubits, apply_in_place
):
    circuit = Circuit(qubits)
    circuit.x(qubits[0])
    measurements = [
        (circuit.measure_bits(qubits), "01"),
        (circuit.measure_bits(qubits[::-1]), "10"),
        (circuit.measure(qubits[0]), "1"),
        (circuit.measure(qubits[1]), "0"),
    ]
    if apply_in_place:
        state = state_type(2)
        state.apply_circuit(circuit)
    else:
        state = state_type.from_circuit(circuit)

    assert state.probabilities() == [0.0, 1.0, 0.0, 0.0]
    for candidate in [state, state.copy()]:
        for measurement, expected in measurements:
            assert candidate.sample(measurement, 4).counts == {expected: 4}
            assert candidate.probs(measurement) == {Outcome(expected): 1.0}


@pytest.mark.parametrize(
    "state_type", [Statevector, DensityMatrix, StabilizerState, DensityMatrixNoise]
)
def test_state_measurement_rejects_ids_absent_from_circuit(state_type):
    state = state_type.from_circuit(Circuit([5, 9]))
    measurement = Circuit(2).measure(0)
    with pytest.raises(IndexError):
        state.sample(measurement, 4)
    with pytest.raises(IndexError):
        state.probs(measurement)


@pytest.mark.parametrize(
    "state_type", [Statevector, DensityMatrix, StabilizerState, DensityMatrixNoise]
)
@pytest.mark.parametrize("incoming_qubits", [[9, 5], [5, 7]])
@pytest.mark.parametrize("with_gate", [False, True])
def test_apply_circuit_rejects_incompatible_mapping(
    state_type, incoming_qubits, with_gate
):
    circuit = Circuit([5, 9])
    circuit.x(5)
    measurement = circuit.measure_bits([5, 9])
    state = state_type.from_circuit(circuit)
    incoming = Circuit(incoming_qubits)
    if with_gate:
        incoming.x(incoming_qubits[0])

    with pytest.raises(ValueError, match="qubit IDs and order"):
        state.apply_circuit(incoming)

    assert state.probabilities() == [0.0, 1.0, 0.0, 0.0]
    assert state.sample(measurement, 4).counts == {"01": 4}
    assert state.probs(measurement) == {Outcome("01"): 1.0}

    compatible = Circuit([5, 9])
    compatible.x(9)
    state.apply_circuit(compatible)
    assert state.probs(measurement) == {Outcome("11"): 1.0}


@pytest.mark.parametrize(
    "state_type", [Statevector, DensityMatrix, StabilizerState, DensityMatrixNoise]
)
@pytest.mark.parametrize("qubits", [[1, 0], [5, 9]])
@pytest.mark.parametrize("bound", [False, True])
def test_failed_apply_circuit_preserves_measurement_positions(
    state_type, qubits, bound
):
    state = state_type.from_circuit(Circuit(qubits)) if bound else state_type(2)
    state.apply_x(0)
    measurement_qubits = qubits if bound else [0, 1]
    measurement = Circuit(measurement_qubits).measure_bits(measurement_qubits)
    invalid = Circuit(qubits)
    invalid.rx(qubits[0], Parameter("theta"))

    with pytest.raises(ValueError):
        state.apply_circuit(invalid)

    assert state.probabilities() == [0.0, 1.0, 0.0, 0.0]
    assert state.sample(measurement, 4).counts == {"01": 4}
    assert state.probs(measurement) == {Outcome("01"): 1.0}


@pytest.mark.parametrize(
    "state_type", [Statevector, DensityMatrix, StabilizerState, DensityMatrixNoise]
)
@pytest.mark.parametrize("bound", [False, True])
def test_failed_apply_circuit_keeps_completed_gates(state_type, bound):
    state = state_type.from_circuit(Circuit([5, 9])) if bound else state_type(2)
    measurement_qubits = [5, 9] if bound else [0, 1]
    measurement = Circuit(measurement_qubits).measure_bits(measurement_qubits)
    invalid = Circuit([5, 9])
    invalid.x(5)
    invalid.rx(9, Parameter("theta"))

    with pytest.raises(ValueError):
        state.apply_circuit(invalid)

    assert state.sample(measurement, 4).counts == {"01": 4}
    assert state.probs(measurement) == {Outcome("01"): 1.0}


def test_partial_trace_retains_circuit_qubit_identifiers():
    circuit = Circuit([5, 9])
    circuit.x(9)
    measurement = circuit.measure(9)
    reduced = DensityMatrix.from_circuit(circuit).partial_trace([1])
    assert reduced.sample(measurement, 4).counts == {"1": 4}
    assert reduced.probs(measurement) == {Outcome("1"): 1.0}


def test_stabilizer_run_retains_circuit_qubit_identifiers():
    circuit = Circuit([5, 9])
    circuit.x(5)
    measurement = circuit.measure_bits([5, 9])
    state = StabilizerState.run_circuit(circuit).state
    assert state.sample(measurement, 4).counts == {"01": 4}
    assert state.probs(measurement) == {Outcome("01"): 1.0}


@pytest.mark.parametrize(
    "state_type", [Statevector, DensityMatrix, StabilizerState, DensityMatrixNoise]
)
def test_state_measurement_on_routed_sparse_physical_qubits(state_type):
    from cqlib.compile.sabre import SabreConfig
    from cqlib.compile.transform import route_with_layout
    from cqlib.device import Device, Layout, Topology

    logical_qubits = [1, 0, 2]
    circuit = Circuit(logical_qubits)
    circuit.x(1)
    circuit.cx(1, 2)
    device = Device("sparse", [5, 7, 9], Topology.line([5, 7, 9]))
    routed = route_with_layout(
        circuit,
        device,
        Layout(logical_qubits, [5, 7, 9], {1: 5, 0: 7, 2: 9}),
        SabreConfig.deterministic_seeded(7),
    )
    assert routed.swap_count > 0
    assert routed.final_layout.l2p_map != routed.initial_layout.l2p_map
    physical = routed.circuit
    measurement = physical.measure_bits(
        [routed.final_layout.get_physical(q).index for q in logical_qubits]
    )
    state = state_type.from_circuit(physical)
    assert state.sample(measurement, 4).counts == {"101": 4}
    assert state.probs(measurement) == {Outcome("101"): 1.0}


def test_density_matrix_noise_readout_errors_raise_index_error():
    noise_model = NoiseModel()
    noise_model.add_readout_error(0, ReadoutError(0.1, 0.2))
    sim = DensityMatrixNoise(1, noise_model)

    with pytest.raises(IndexError):
        sim.probabilities_with_readout([99])


def test_density_matrix_noise_probs_with_readout_binding():
    circuit = Circuit(1)
    measurement = circuit.measure(0)

    noise_model = NoiseModel()
    noise_model.add_readout_error(0, ReadoutError(0.0, 1.0))
    sim = DensityMatrixNoise(1, noise_model)

    assert sim.probs(measurement) == {Outcome("0"): 1.0}
    assert sim.probs_with_readout(measurement) == {Outcome("1"): 1.0}


@pytest.mark.parametrize("qubits", [[0, 1], [1, 0], [5, 9]])
@pytest.mark.parametrize("target", [0, 1])
def test_density_matrix_readout_uses_ids_for_noise_and_positions_for_probabilities(
    qubits, target
):
    circuit = Circuit(qubits)
    measurements = [
        (circuit.measure(qubits[target]), "1"),
        (circuit.measure(qubits[1 - target]), "0"),
        (circuit.measure_bits(qubits), "01" if target == 0 else "10"),
        (circuit.measure_bits(qubits[::-1]), "10" if target == 0 else "01"),
    ]
    noise = NoiseModel()
    noise.add_readout_error(qubits[target], ReadoutError(1.0, 1.0))
    state = DensityMatrixNoise.from_circuit(circuit, noise)

    for measurement, expected in measurements:
        assert state.probs_with_readout(measurement) == {Outcome(expected): 1.0}
        assert state.probs(measurement) == {Outcome("0" * len(expected)): 1.0}


def test_runtime_value_equality_and_hash():
    """Runtime values compare by value and deduplicate in sets."""

    def measured_bit():
        circuit = Circuit(1)
        circuit.x(0)
        measurement = circuit.measure(0)
        return StabilizerState.run_circuit(circuit).classical.value(measurement.value)

    first = measured_bit()
    second = measured_bit()
    assert isinstance(first, RuntimeValue)
    assert first == second
    assert hash(first) == hash(second)
    assert len({first, second}) == 1
    assert (first == 42) is False


def test_runtime_value_different_kinds_are_unequal():
    """A measured Bit(True) is distinct from a stored Bool(True)."""
    circuit = Circuit(1)
    circuit.x(0)
    measurement = circuit.measure(0)
    bit = StabilizerState.run_circuit(circuit).classical.value(measurement.value)

    var_circuit = Circuit(1)
    flag = var_circuit.var(ClassicalType.bool())
    var_circuit.store(flag, ClassicalExpr.bool_literal(True))
    stored = StabilizerState.run_circuit(var_circuit).classical.var(flag)

    assert bit.as_bit() is True
    assert stored.as_bool() is True
    assert bit != stored
