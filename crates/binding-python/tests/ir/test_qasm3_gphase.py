# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# Licensed under the Apache License, Version 2.0. See LICENSE.txt or
# http://www.apache.org/licenses/LICENSE-2.0.

"""Global phase must survive serialization of reusable gate definitions."""

import numpy as np
import pytest

from cqlib import Circuit, Parameter
from cqlib.circuit import UnitaryGate
from cqlib.circuit.gates import GPhase
from cqlib.ir import qasm3
from cqlib.qis import Statevector


def assert_round_trip(circuit, expected, bindings=None):
    output = qasm3.dumps(circuit)
    loaded = qasm3.loads(output)
    original = circuit.assign_parameters(bindings) if bindings else circuit
    loaded = loaded.assign_parameters(bindings) if bindings else loaded
    for candidate in [original, loaded]:
        np.testing.assert_allclose(candidate.to_matrix(), expected, rtol=0, atol=1e-12)
        np.testing.assert_allclose(
            Statevector.from_circuit(candidate).data,
            expected[:, 0],
            rtol=0,
            atol=1e-12,
        )
    return output


@pytest.mark.parametrize("symbolic", [False, True])
@pytest.mark.parametrize("explicit_phase", [False, True])
@pytest.mark.parametrize("nested", [False, True])
def test_nested_gate_global_phase_round_trip(symbolic, explicit_phase, nested):
    theta = Parameter("theta") if symbolic else 0.37
    inner = Circuit([12])
    inner.h(12)
    inner.set_global_phase(theta)
    if explicit_phase:
        inner.append_gate(GPhase(-0.21), [])
    gate = inner.to_gate("fragment")
    if nested:
        middle = Circuit([5])
        middle.set_global_phase(0.11)
        middle.append_circuit_gate(gate, [5], [theta] if symbolic else [])
        gate = middle.to_gate("middle")
    outer = Circuit(1)
    outer.set_global_phase(-0.07)
    outer.append_circuit_gate(gate, [0], [Parameter("outer_theta")] if symbolic else [])
    phase = 0.37 - 0.07 + (0.11 if nested else 0) - (0.21 if explicit_phase else 0)
    expected = np.exp(1j * phase) * np.array([[1, 1], [1, -1]]) / np.sqrt(2)
    assert_round_trip(outer, expected, {"outer_theta": 0.37} if symbolic else None)


def test_phase_only_parameter_and_repeated_gate_calls():
    inner = Circuit(1)
    inner.h(0)
    inner.set_global_phase(Parameter("theta"))
    gate = inner.to_gate("fragment")
    outer = Circuit(1)
    outer.append_circuit_gate(gate, [0], [0.37])
    outer.append_circuit_gate(gate, [0], [-0.51])
    output = assert_round_trip(outer, np.exp(-0.14j) * np.eye(2))
    assert output.count("gphase(theta);") == 1


def test_unitary_gate_definition_global_phase_round_trip():
    inner = Circuit(1)
    inner.h(0)
    inner.set_global_phase(0.37)
    inner.append_gate(GPhase(-0.21), [])
    expected = np.exp(0.16j) * np.array([[1, 1], [1, -1]]) / np.sqrt(2)
    gate = (
        UnitaryGate("phase_h", 1)
        .with_matrix(expected)
        .with_circuit(inner.to_gate("definition").circuit)
    )
    outer = Circuit(1)
    outer.append_unitary_gate(gate, [0])
    assert_round_trip(outer, expected)
