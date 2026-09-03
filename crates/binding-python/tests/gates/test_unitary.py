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
Tests for UnitaryGate custom unitary gates.

Test coverage:
- Custom gate creation
- Gate definition via matrix
- Single/multi-qubit custom gates
- Gate property access
- Gate application in circuits
- Unitary property verification
"""

import pytest
import numpy as np
from cqlib.circuit import Circuit, UnitaryGate


def is_unitary(matrix, atol=1e-10):
    """Verify matrix is unitary: U†U = I"""
    n = matrix.shape[0]
    identity = np.eye(n)
    product = matrix.conj().T @ matrix
    return np.allclose(product, identity, atol=atol)


class TestUnitaryGateCreation:
    """Test custom gate creation"""

    def test_create_single_qubit_gate(self):
        """Create single qubit gate"""
        gate = UnitaryGate("MyGate", 1)
        assert gate.label == "MyGate"
        assert gate.num_qubits == 1

    def test_create_two_qubit_gate(self):
        """Create two-qubit gate"""
        gate = UnitaryGate("TwoQubitGate", 2)
        assert gate.label == "TwoQubitGate"
        assert gate.num_qubits == 2

    def test_create_multi_qubit_gate(self):
        """Create multi-qubit gate"""
        gate = UnitaryGate("ThreeQubitGate", 3)
        assert gate.label == "ThreeQubitGate"
        assert gate.num_qubits == 3


class TestUnitaryGateWithMatrix:
    """Test gate definition via matrix"""

    def test_identity_matrix(self):
        """Identity matrix"""
        gate = UnitaryGate("Identity", 1).with_matrix(np.eye(2))
        mat = gate.matrix()
        assert np.allclose(mat, np.eye(2)), "Identity matrix mismatch"
        assert is_unitary(mat), "Identity should be unitary"

    def test_pauli_x_matrix(self):
        """Pauli-X matrix"""
        x_mat = np.array([[0, 1], [1, 0]], dtype=complex)
        gate = UnitaryGate("CustomX", 1).with_matrix(x_mat)
        mat = gate.matrix()
        assert np.allclose(mat, x_mat), "X matrix mismatch"
        assert is_unitary(mat), "X should be unitary"

    def test_pauli_y_matrix(self):
        """Pauli-Y matrix"""
        y_mat = np.array([[0, -1j], [1j, 0]], dtype=complex)
        gate = UnitaryGate("CustomY", 1).with_matrix(y_mat)
        mat = gate.matrix()
        assert np.allclose(mat, y_mat), "Y matrix mismatch"
        assert is_unitary(mat), "Y should be unitary"

    def test_pauli_z_matrix(self):
        """Pauli-Z matrix"""
        z_mat = np.array([[1, 0], [0, -1]], dtype=complex)
        gate = UnitaryGate("CustomZ", 1).with_matrix(z_mat)
        mat = gate.matrix()
        assert np.allclose(mat, z_mat), "Z matrix mismatch"
        assert is_unitary(mat), "Z should be unitary"

    def test_hadamard_matrix(self):
        """Hadamard matrix"""
        h_mat = np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2)
        gate = UnitaryGate("CustomH", 1).with_matrix(h_mat)
        mat = gate.matrix()
        assert np.allclose(mat, h_mat), "H matrix mismatch"
        assert is_unitary(mat), "H should be unitary"

    def test_two_qubit_gate_matrix(self):
        """Two-qubit matrix - CNOT"""
        cnot_mat = np.array(
            [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 0, 1], [0, 0, 1, 0]], dtype=complex
        )
        gate = UnitaryGate("CustomCNOT", 2).with_matrix(cnot_mat)
        mat = gate.matrix()
        assert np.allclose(mat, cnot_mat), "CNOT matrix mismatch"
        assert is_unitary(mat), "CNOT should be unitary"

    def test_swap_matrix(self):
        """SWAP gate matrix"""
        swap_mat = np.array(
            [[1, 0, 0, 0], [0, 0, 1, 0], [0, 1, 0, 0], [0, 0, 0, 1]], dtype=complex
        )
        gate = UnitaryGate("CustomSWAP", 2).with_matrix(swap_mat)
        mat = gate.matrix()
        assert np.allclose(mat, swap_mat), "SWAP matrix mismatch"
        assert is_unitary(mat), "SWAP should be unitary"

    def test_matrix_from_list(self):
        """Create matrix from list"""
        mat_list = [[1, 0], [0, 1j]]
        gate = UnitaryGate("S", 1).with_matrix(mat_list)
        mat = gate.matrix()
        expected = np.array([[1, 0], [0, 1j]], dtype=complex)
        assert np.allclose(mat, expected), "Matrix from list mismatch"
        assert is_unitary(mat), "Matrix should be unitary"

    def test_matrix_with_complex_dtype(self):
        """Matrix with complex128 dtype"""
        h_mat = np.array([[1, 1], [1, -1]], dtype=np.complex128) / np.sqrt(2)
        gate = UnitaryGate("CustomH", 1).with_matrix(h_mat)
        mat = gate.matrix()
        assert np.allclose(mat, h_mat), "Complex128 matrix mismatch"


class TestUnitaryGateUnitaryProperty:
    """Test custom gate unitary property verification"""

    def test_hadamard_square_is_identity(self):
        """H² = I"""
        h_mat = np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2)
        gate = UnitaryGate("H", 1).with_matrix(h_mat)
        mat = gate.matrix()
        product = mat @ mat
        assert np.allclose(product, np.eye(2)), "H² should be identity"

    def test_cnot_square_is_identity(self):
        """CNOT² = I"""
        cnot_mat = np.array(
            [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 0, 1], [0, 0, 1, 0]], dtype=complex
        )
        gate = UnitaryGate("CNOT", 2).with_matrix(cnot_mat)
        mat = gate.matrix()
        product = mat @ mat
        assert np.allclose(product, np.eye(4)), "CNOT² should be identity"

    def test_random_unitary_is_unitary(self):
        """Random unitary matrix is recognized as unitary"""
        # Create a random unitary via QR decomposition
        np.random.seed(42)
        a = np.random.randn(2, 2) + 1j * np.random.randn(2, 2)
        q, r = np.linalg.qr(a)
        # Adjust phases to make it truly unitary
        d = np.diag(r)
        ph = d / np.abs(d)
        unitary = q * ph
        gate = UnitaryGate("RandomU", 1).with_matrix(unitary)
        mat = gate.matrix()
        assert is_unitary(mat), "Random unitary should be unitary"


class TestUnitaryGateInCircuit:
    """Test custom gate application in circuits"""

    def test_apply_single_qubit_unitary(self):
        """Apply single-qubit gate to circuit"""
        c = Circuit(2)
        h_mat = np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2)
        gate = UnitaryGate("CustomH", 1).with_matrix(h_mat)
        c.append_unitary_gate(gate, [0])
        assert len(c) == 1
        # Verify operation name is the gate label
        op = c[0]
        assert op.name == "CustomH"

    def test_apply_two_qubit_unitary(self):
        """Apply two-qubit gate to circuit"""
        c = Circuit(3)
        cnot_mat = np.array(
            [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 0, 1], [0, 0, 1, 0]], dtype=complex
        )
        gate = UnitaryGate("CustomCNOT", 2).with_matrix(cnot_mat)
        c.append_unitary_gate(gate, [0, 1])
        assert len(c) == 1

    def test_apply_multiple_unitaries(self):
        """Apply multiple custom gates to circuit"""
        c = Circuit(2)
        x_mat = np.array([[0, 1], [1, 0]], dtype=complex)
        h_mat = np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2)

        x_gate = UnitaryGate("X", 1).with_matrix(x_mat)
        h_gate = UnitaryGate("H", 1).with_matrix(h_mat)

        c.append_unitary_gate(x_gate, [0])
        c.append_unitary_gate(h_gate, [1])

        assert len(c) == 2

    def test_apply_unitary_to_different_qubits(self):
        """Apply unitary gate to different qubit indices"""
        c = Circuit(3)
        h_mat = np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2)
        gate = UnitaryGate("CustomH", 1).with_matrix(h_mat)

        # Apply to qubit 2
        c.append_unitary_gate(gate, [2])
        assert len(c) == 1
        assert c[0].qubits[0].index == 2


class TestUnitaryGateProperties:
    """Test gate properties"""

    def test_gate_label(self):
        """Gate label property"""
        gate = UnitaryGate("TestGate", 1)
        assert gate.label == "TestGate"

    def test_gate_num_qubits(self):
        """Gate qubit count property"""
        gate1 = UnitaryGate("G1", 1)
        gate2 = UnitaryGate("G2", 2)
        assert gate1.num_qubits == 1
        assert gate2.num_qubits == 2

    def test_gate_matrix_before_definition(self):
        """Accessing matrix before definition raises error"""
        gate = UnitaryGate("Undefined", 1)
        with pytest.raises(Exception):
            _ = gate.matrix()

    def test_gate_matrix_after_definition(self):
        """Matrix accessible after definition"""
        identity = np.eye(2, dtype=complex)
        gate = UnitaryGate("I", 1).with_matrix(identity)
        mat = gate.matrix()
        assert np.allclose(mat, identity)


class TestUnitaryGateErrors:
    """Test error handling"""

    def test_matrix_without_definition(self):
        """Accessing matrix without definition raises error"""
        gate = UnitaryGate("Undefined", 1)
        with pytest.raises(Exception):
            _ = gate.matrix()

    def test_invalid_matrix_size_too_large(self):
        """Matrix size larger than gate dimension raises error"""
        gate = UnitaryGate("Test", 1)
        with pytest.raises(Exception):
            gate.with_matrix(np.eye(4))

    def test_invalid_matrix_size_too_small(self):
        """Matrix size smaller than gate dimension raises error"""
        gate = UnitaryGate("Test", 2)
        with pytest.raises(Exception):
            gate.with_matrix(np.eye(2))


class TestUnitaryGateNumpyArray:
    """Test NumPy array protocol compatibility"""

    def test_numpy_array_protocol(self):
        """__array__ protocol - compatible with NumPy 2.0+"""
        h_mat = np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2)
        gate = UnitaryGate("CustomH", 1).with_matrix(h_mat)
        # Use np.array to call __array__
        arr = np.array(gate)
        assert np.allclose(arr, h_mat), "__array__ should return correct matrix"

    def test_numpy_array_protocol_with_dtype(self):
        """__array__ protocol supports dtype parameter"""
        h_mat = np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2)
        gate = UnitaryGate("CustomH", 1).with_matrix(h_mat)
        arr = np.array(gate, dtype=complex)
        assert np.allclose(arr, h_mat), "__array__ with dtype should work"

    def test_numpy_array_protocol_with_copy(self):
        """__array__ protocol supports copy parameter (NumPy 2.0+) - partial support"""
        h_mat = np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2)
        gate = UnitaryGate("CustomH", 1).with_matrix(h_mat)
        # Note: current implementation supports copy parameter signature, but NumPy 2.0 full
        # compatibility requires pyo3/rust-numpy updates
        # Test copy=True (should work)
        arr_copy = np.array(gate, copy=True)
        assert np.allclose(arr_copy, h_mat), "__array__ with copy=True should work"
        # Test asarray (recommended, no forced copy)
        arr = np.asarray(gate, dtype=complex)
        assert np.allclose(arr, h_mat), "np.asarray should work"

    def test_numpy_asarray_without_copy(self):
        """np.asarray creates array view without copy when possible"""
        h_mat = np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2)
        gate = UnitaryGate("CustomH", 1).with_matrix(h_mat)
        arr = np.asarray(gate)
        assert arr.shape == (2, 2)
        assert np.allclose(arr, h_mat)
