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

"""Circuit gate conversion tests for the Python circuit API."""

import numpy as np

from cqlib import Circuit, Parameter
from cqlib.circuit import CircuitGate, FrozenCircuit


def test_to_gate_creates_named_circuit_gate():
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)

    gate = circuit.to_gate("Bell")

    assert isinstance(gate, CircuitGate)
    assert gate.name == "Bell"
    assert gate.num_qubits == 2
    assert gate.num_params == 0
    assert gate.circuit.num_operations == 2


def test_append_circuit_gate_reuses_subcircuit():
    subcircuit = Circuit(2)
    subcircuit.h(0)
    subcircuit.cx(0, 1)
    gate = subcircuit.to_gate("Bell")

    circuit = Circuit(4)
    circuit.append_circuit_gate(gate, [0, 1])
    circuit.append_circuit_gate(gate, [2, 3])

    assert [op.instruction.instruction.name for op in circuit.operations] == [
        "Bell",
        "Bell",
    ]
    assert [qubit.index for qubit in circuit[1].qubits] == [2, 3]


def test_circuit_gate_accepts_parameter_bindings():
    theta = Parameter("theta")
    subcircuit = Circuit(1)
    subcircuit.rx(0, theta)
    gate = subcircuit.to_gate("Rotate")

    circuit = Circuit(1)
    circuit.append_circuit_gate(gate, [0], [0.5])

    decomposed = circuit.decompose()
    assert decomposed[0].instruction.instruction.name == "RX"
    assert list(decomposed[0].params) == [0.5]


def test_manual_frozen_circuit_gate_construction():
    subcircuit = Circuit(1)
    subcircuit.h(0)
    frozen = FrozenCircuit(subcircuit.qubits, subcircuit.operations)
    gate = CircuitGate("HadamardBlock", frozen)

    circuit = Circuit(1)
    circuit.append_circuit_gate(gate, [0])

    assert circuit[0].instruction.instruction.name == "HadamardBlock"
    assert np.allclose(circuit.decompose().to_matrix(), subcircuit.to_matrix())


def test_inverse_circuit_gate_can_be_appended():
    subcircuit = Circuit(1)
    subcircuit.rx(0, 0.25)
    gate = subcircuit.to_gate("RxBlock")
    inverse_gate = gate.inverse()

    circuit = Circuit(1)
    circuit.append_circuit_gate(gate, [0])
    circuit.append_circuit_gate(inverse_gate, [0])

    assert inverse_gate.name == "RxBlock_dg"
    assert inverse_gate.num_qubits == 1
    assert np.allclose(circuit.decompose().to_matrix(), np.eye(2), atol=1e-10)
