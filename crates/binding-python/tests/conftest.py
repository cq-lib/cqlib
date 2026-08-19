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

"""
Pytest fixtures and utilities for cqlib tests.
"""

import pytest
import numpy as np
from cqlib.circuit import Circuit, Parameter


@pytest.fixture
def empty_circuit():
    """Returns an empty circuit with no qubits."""
    return Circuit(0)


@pytest.fixture
def single_qubit_circuit():
    """Returns a simple 1-qubit circuit."""
    return Circuit(1)


@pytest.fixture
def two_qubit_circuit():
    """Returns a simple 2-qubit circuit."""
    return Circuit(2)


@pytest.fixture
def three_qubit_circuit():
    """Returns a simple 3-qubit circuit."""
    return Circuit(3)


@pytest.fixture
def bell_state_circuit():
    """Returns a circuit that creates a Bell state."""
    c = Circuit(2)
    c.h(0)
    c.cx(0, 1)
    return c


@pytest.fixture
def ghz_state_circuit():
    """Returns a circuit that creates a GHZ state."""
    c = Circuit(3)
    c.h(0)
    c.cx(0, 1)
    c.cx(1, 2)
    return c


@pytest.fixture
def qft_3qubit_circuit():
    """Returns a 3-qubit QFT circuit."""
    c = Circuit(3)
    c.h(0)
    c.cz(1, 0)
    c.cz(2, 0)
    c.h(1)
    c.cz(2, 1)
    c.h(2)
    return c


@pytest.fixture
def theta_param():
    """Returns a symbolic parameter named 'theta'."""
    return Parameter("theta")


@pytest.fixture
def phi_param():
    """Returns a symbolic parameter named 'phi'."""
    return Parameter("phi")


@pytest.fixture
def alpha_param():
    """Returns a symbolic parameter named 'alpha'."""
    return Parameter("alpha")


@pytest.fixture
def beta_param():
    """Returns a symbolic parameter named 'beta'."""
    return Parameter("beta")


@pytest.fixture
def constant_param():
    """Returns a constant parameter with value 3.14."""
    return Parameter.from_float(3.14)


@pytest.fixture
def single_param_circuit(theta_param):
    """Returns a circuit with a single parameter."""
    c = Circuit(1)
    c.rx(0, theta_param)
    return c


@pytest.fixture
def multi_param_circuit(theta_param, phi_param):
    """Returns a circuit with multiple parameters."""
    c = Circuit(2)
    c.rx(0, theta_param)
    c.ry(1, phi_param)
    c.cx(0, 1)
    return c


@pytest.fixture
def variational_circuit(theta_param, phi_param):
    """Returns a variational circuit with entanglement."""
    c = Circuit(2)
    c.rx(0, theta_param)
    c.ry(1, phi_param)
    c.cz(0, 1)
    c.rx(0, theta_param)
    c.ry(1, phi_param)
    return c


@pytest.fixture
def hadamard_gate():
    """Returns a Hadamard gate instance."""
    from cqlib.circuit.gates import H

    return H


@pytest.fixture
def cnot_gate():
    """Returns a CNOT gate instance."""
    from cqlib.circuit.gates import CX

    return CX


@pytest.fixture
def is_close():
    """Returns a function to check if two arrays are close."""

    def _is_close(a, b, rtol=1e-10, atol=1e-10):
        return np.allclose(a, b, rtol=rtol, atol=atol)

    return _is_close


@pytest.fixture
def is_unitary():
    """Returns a function to check if a matrix is unitary."""

    def _is_unitary(mat, rtol=1e-10, atol=1e-10):
        identity = mat @ mat.conj().T
        return np.allclose(identity, np.eye(len(mat)), rtol=rtol, atol=atol)

    return _is_unitary


@pytest.fixture
def qasm_bell_state():
    """Returns QASM2 string for a Bell state circuit."""
    return """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
h q[0];
cx q[0],q[1];
"""


@pytest.fixture
def qasm_ghz_state():
    """Returns QASM2 string for a GHZ state circuit."""
    return """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[3];
h q[0];
cx q[0],q[1];
cx q[1],q[2];
"""


@pytest.fixture
def qasm_qft():
    """Returns QASM2 string for a 2-qubit QFT circuit."""
    return """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
h q[0];
cu1(pi/2) q[1],q[0];
h q[1];
swap q[0],q[1];
"""


@pytest.fixture
def qcis_bell_state():
    """Returns QCIS string for a Bell state circuit."""
    return """
H Q0
CNOT Q0 Q1
"""


@pytest.fixture
def qcis_ghz_state():
    """Returns QCIS string for a GHZ state circuit."""
    return """
H Q0
CNOT Q0 Q1
CNOT Q1 Q2
"""
