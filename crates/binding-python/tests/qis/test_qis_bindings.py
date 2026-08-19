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

from cqlib.circuit import Circuit, ClassicalExpr, ClassicalType
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
