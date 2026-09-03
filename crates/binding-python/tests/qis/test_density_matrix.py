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

"""Tests for DensityMatrix quantum state simulation."""

import inspect

import pytest
import math
import numpy as np
from cqlib.circuit import Circuit, StandardGate
from cqlib.qis import DensityMatrix, PauliString, Hamiltonian


class TestDensityMatrixConstruction:
    """Test DensityMatrix constructors."""

    def test_new(self):
        """Test basic constructor creates |0...0><0...0|."""
        dm = DensityMatrix(2)
        assert dm.num_qubits == 2
        probs = dm.probabilities()
        assert len(probs) == 4
        assert math.isclose(probs[0], 1.0)
        assert math.isclose(probs[1], 0.0)
        assert math.isclose(probs[2], 0.0)
        assert math.isclose(probs[3], 0.0)

    def test_from_state(self):
        """Test construction from statevector."""
        # Create |+> state
        amps = np.array([1 / np.sqrt(2), 1 / np.sqrt(2)], dtype=complex)
        dm = DensityMatrix.from_state(1, amps)
        assert dm.num_qubits == 1
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.5)
        assert math.isclose(probs[1], 0.5)

    def test_from_state_invalid_length(self):
        """Test from_state with invalid state length."""
        amps = np.array([1.0, 0.0, 0.0], dtype=complex)  # Wrong length for 1 qubit
        with pytest.raises(ValueError):
            DensityMatrix.from_state(1, amps)

    def test_from_density_matrix(self):
        """Test construction from density matrix data."""
        # Create |1><1| density matrix
        data = np.array([0, 0, 0, 1], dtype=complex)
        dm = DensityMatrix.from_density_matrix(1, data)
        assert dm.num_qubits == 1
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.0)
        assert math.isclose(probs[1], 1.0)

    def test_from_density_matrix_invalid_trace(self):
        """Test from_density_matrix with invalid trace."""
        # Matrix with trace != 1
        data = np.array([1, 0, 0, 1], dtype=complex)
        with pytest.raises(ValueError):
            DensityMatrix.from_density_matrix(1, data)

    def test_from_circuit(self):
        """Test construction from circuit simulation."""
        circuit = Circuit(2)
        circuit.h(0)
        circuit.cx(0, 1)

        dm = DensityMatrix.from_circuit(circuit)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.5)
        assert math.isclose(probs[1], 0.0)
        assert math.isclose(probs[2], 0.0)
        assert math.isclose(probs[3], 0.5)


class TestDensityMatrixProperties:
    """Test DensityMatrix properties."""

    def test_num_qubits(self):
        """Test num_qubits property."""
        dm = DensityMatrix(3)
        assert dm.num_qubits == 3

    def test_data(self):
        """Test data property returns correct shape."""
        dm = DensityMatrix(2)
        data = dm.data
        assert data.shape == (4, 4)
        # Check it's a density matrix of |00><00|
        assert data[0, 0] == 1.0
        assert np.allclose(data[0, 1:], 0)
        assert np.allclose(data[1:, 0], 0)

    def test_probabilities(self):
        """Test probabilities extraction."""
        dm = DensityMatrix(1)
        dm.apply_h(0)
        probs = dm.probabilities()
        assert len(probs) == 2
        assert math.isclose(probs[0], 0.5)
        assert math.isclose(probs[1], 0.5)

    def test_trace(self):
        """Test trace calculation."""
        dm = DensityMatrix(2)
        assert math.isclose(dm.trace(), 1.0)

        # After operations, trace should still be 1
        dm.apply_h(0)
        dm.apply_cx(0, 1)
        assert math.isclose(dm.trace(), 1.0, abs_tol=1e-10)


class TestDensityMatrixSingleQubitGates:
    """Test single-qubit gate operations."""

    def test_apply_standard_gate(self):
        """Test applying gates via the general standard gate interface."""
        dm = DensityMatrix(1)
        dm.apply_standard_gate(StandardGate.X, [0])
        probs = dm.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

        dm = DensityMatrix(1)
        dm.apply_standard_gate(StandardGate.RX, [0], [np.pi])
        probs = dm.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

        dm = DensityMatrix(2)
        dm.apply_standard_gate(StandardGate.X, [0])
        dm.apply_standard_gate(StandardGate.CX, [0, 1])
        probs = dm.probabilities()
        assert math.isclose(probs[3], 1.0, abs_tol=1e-10)

    def test_apply_x(self):
        """Test Pauli-X gate."""
        dm = DensityMatrix(1)
        dm.apply_x(0)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.0)
        assert math.isclose(probs[1], 1.0)

    def test_apply_y(self):
        """Test Pauli-Y gate."""
        dm = DensityMatrix(1)
        dm.apply_y(0)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.0)
        assert math.isclose(probs[1], 1.0)

    def test_apply_z(self):
        """Test Pauli-Z gate (no effect on |0>)."""
        dm = DensityMatrix(1)
        dm.apply_z(0)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 1.0)
        assert math.isclose(probs[1], 0.0)

    def test_apply_h(self):
        """Test Hadamard gate."""
        dm = DensityMatrix(1)
        dm.apply_h(0)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.5)
        assert math.isclose(probs[1], 0.5)

    def test_apply_s_and_sdg(self):
        """Test S and S-dagger gates."""
        dm = DensityMatrix(1)
        dm.apply_h(0)
        dm.apply_s(0)
        dm.apply_sdg(0)
        # S * Sdg = I, so back to |+> state
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[1], 0.5, abs_tol=1e-10)

    def test_apply_t_and_tdg(self):
        """Test T and T-dagger gates."""
        dm = DensityMatrix(1)
        dm.apply_h(0)
        dm.apply_t(0)
        dm.apply_tdg(0)
        # T * Tdg = I
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)

    def test_apply_rx(self):
        """Test RX rotation."""
        dm = DensityMatrix(1)
        dm.apply_rx(0, np.pi)
        # RX(pi) = -iX, rotates |0> to |1> (up to phase)
        probs = dm.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_ry(self):
        """Test RY rotation."""
        dm = DensityMatrix(1)
        dm.apply_ry(0, np.pi / 2)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[1], 0.5, abs_tol=1e-10)

    def test_apply_rz(self):
        """Test RZ rotation (no effect on |0>)."""
        dm = DensityMatrix(1)
        dm.apply_rz(0, np.pi / 4)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 1.0)

    def test_apply_p(self):
        """Test phase gate (no effect on |0>)."""
        dm = DensityMatrix(1)
        dm.apply_phase(0, np.pi / 4)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 1.0)

    def test_apply_x2p_x2m(self):
        """Test X2P and X2M gates."""
        dm = DensityMatrix(1)
        dm.apply_x2p(0)
        dm.apply_x2m(0)
        # Should return to |0> (approximately)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 1.0, abs_tol=1e-10)

    def test_apply_y2p_y2m(self):
        """Test Y2P and Y2M gates."""
        dm = DensityMatrix(1)
        dm.apply_y2p(0)
        dm.apply_y2m(0)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 1.0, abs_tol=1e-10)

    def test_apply_u(self):
        """Test general U gate."""
        dm = DensityMatrix(1)
        # U(pi, 0, 0) = Ry(pi) = X (up to phase)
        dm.apply_u(0, np.pi, 0, 0)
        probs = dm.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_gphase(self):
        """Test global phase (no observable effect)."""
        dm = DensityMatrix(1)
        dm.apply_gphase(np.pi / 4)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 1.0)


class TestDensityMatrixTwoQubitGates:
    """Test two-qubit gate operations."""

    def test_apply_cx(self):
        """Test CNOT gate."""
        dm = DensityMatrix(2)
        dm.apply_h(0)
        dm.apply_cx(0, 1)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.5)
        assert math.isclose(probs[3], 0.5)

    def test_apply_cy(self):
        """Test CY gate."""
        dm = DensityMatrix(2)
        dm.apply_h(0)
        dm.apply_cy(0, 1)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.5)
        assert math.isclose(probs[3], 0.5)

    def test_apply_cz(self):
        """Test CZ gate."""
        dm = DensityMatrix(2)
        dm.apply_h(0)
        dm.apply_h(1)
        dm.apply_cz(0, 1)
        # Creates phase on |11>
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.25)
        assert math.isclose(probs[3], 0.25)

    def test_apply_swap(self):
        """Test SWAP gate."""
        dm = DensityMatrix(2)
        dm.apply_x(1)  # |10> (qubit 1 is high-order bit)
        dm.apply_swap(0, 1)  # |01> (qubit 0 is high-order bit after swap)
        probs = dm.probabilities()
        # |01> is index 1
        assert math.isclose(probs[1], 1.0)

    def test_apply_crx(self):
        """Test controlled-RX."""
        dm = DensityMatrix(2)
        dm.apply_h(0)
        dm.apply_crx(0, 1, np.pi)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.5)
        assert math.isclose(probs[3], 0.5)

    def test_apply_cry(self):
        """Test controlled-RY."""
        dm = DensityMatrix(2)
        dm.apply_h(0)
        dm.apply_cry(0, 1, np.pi / 2)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.5)

    def test_apply_crz(self):
        """Test controlled-RZ."""
        dm = DensityMatrix(2)
        dm.apply_h(0)
        dm.apply_h(1)
        dm.apply_crz(0, 1, np.pi / 4)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.25)

    def test_apply_rxx(self):
        """Test RXX gate."""
        dm = DensityMatrix(2)
        dm.apply_rxx(0, 1, np.pi / 2)
        probs = dm.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_apply_ryy(self):
        """Test RYY gate."""
        dm = DensityMatrix(2)
        dm.apply_ryy(0, 1, np.pi / 2)
        probs = dm.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_apply_rzz(self):
        """Test RZZ gate."""
        dm = DensityMatrix(2)
        dm.apply_h(0)
        dm.apply_h(1)
        dm.apply_rzz(0, 1, np.pi)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.25)

    def test_apply_rzx(self):
        """Test RZX gate."""
        dm = DensityMatrix(2)
        dm.apply_h(0)
        dm.apply_rzx(0, 1, np.pi / 4)
        probs = dm.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10


class TestDensityMatrixThreeQubitGates:
    """Test three-qubit gate operations."""

    def test_apply_ccx(self):
        """Test Toffoli (CCX) gate."""
        dm = DensityMatrix(3)
        dm.apply_x(0)
        dm.apply_x(1)
        dm.apply_ccx(0, 1, 2)
        # |110> -> |111>
        probs = dm.probabilities()
        assert math.isclose(probs[6], 0.0)  # |110> is index 6
        assert math.isclose(probs[7], 1.0)  # |111> is index 7


class TestDensityMatrixXYGates:
    """Test XY family of gates."""

    def test_apply_xy(self):
        """Test XY gate."""
        dm = DensityMatrix(1)
        dm.apply_xy(0, np.pi / 4)
        probs = dm.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_apply_xy2p(self):
        """Test XY2P gate."""
        dm = DensityMatrix(1)
        dm.apply_xy2p(0, np.pi / 4)
        probs = dm.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_apply_xy2m(self):
        """Test XY2M gate."""
        dm = DensityMatrix(1)
        dm.apply_xy2m(0, np.pi / 4)
        probs = dm.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_apply_rxy(self):
        """Test RXY gate."""
        dm = DensityMatrix(1)
        dm.apply_rxy(0, np.pi / 2, 0)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[1], 0.5, abs_tol=1e-10)


class TestDensityMatrixFSim:
    """Test fSim gate."""

    def test_apply_fsim(self):
        """Test Fermionic Simulation gate."""
        dm = DensityMatrix(2)
        dm.apply_x(0)  # |01>
        dm.apply_fsim(0, 1, np.pi / 4, np.pi / 2)
        probs = dm.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10


class TestDensityMatrixCustomGates:
    """Test custom gate application."""

    def test_apply_single_qubit_gate(self):
        """Test custom single-qubit gate."""
        dm = DensityMatrix(1)
        # X gate matrix
        x_matrix = np.array([[0, 1], [1, 0]], dtype=complex)
        dm.apply_single_qubit_gate(0, x_matrix)
        probs = dm.probabilities()
        assert math.isclose(probs[1], 1.0)

    def test_apply_single_qubit_gate_invalid(self):
        """Test invalid single-qubit gate dimensions."""
        dm = DensityMatrix(1)
        invalid_matrix = np.array([[1, 0, 0], [0, 1, 0]], dtype=complex)  # 2x3
        with pytest.raises(ValueError):
            dm.apply_single_qubit_gate(0, invalid_matrix)

    def test_apply_double_qubits_gate(self):
        """Test custom two-qubit gate."""
        dm = DensityMatrix(2)
        # SWAP gate as 4x4 matrix
        swap_matrix = np.array(
            [[1, 0, 0, 0], [0, 0, 1, 0], [0, 1, 0, 0], [0, 0, 0, 1]], dtype=complex
        )
        dm.apply_x(1)  # |10> (qubit 1 is high-order bit)
        dm.apply_double_qubits_gate(0, 1, swap_matrix)  # |01>
        probs = dm.probabilities()
        assert math.isclose(probs[1], 1.0)

    def test_apply_unitary_gate(self):
        """Test arbitrary n-qubit unitary."""
        dm = DensityMatrix(2)
        # Create Bell state using unitary
        h_cx = np.array(
            [
                [1 / np.sqrt(2), 0, 1 / np.sqrt(2), 0],
                [0, 1 / np.sqrt(2), 0, 1 / np.sqrt(2)],
                [0, 1 / np.sqrt(2), 0, -1 / np.sqrt(2)],
                [1 / np.sqrt(2), 0, -1 / np.sqrt(2), 0],
            ],
            dtype=complex,
        )
        dm.apply_unitary_gate([0, 1], h_cx)
        probs = dm.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[3], 0.5, abs_tol=1e-10)


class TestDensityMatrixKraus:
    """Test Kraus operator (quantum channel) application."""

    def test_apply_kraus_bit_flip(self):
        """Test bit-flip channel."""
        dm = DensityMatrix(1)
        dm.apply_h(0)  # |+>
        # Bit-flip channel with p=0.1
        p = 0.1
        K0 = np.sqrt(1 - p) * np.eye(2, dtype=complex)
        K1 = np.sqrt(p) * np.array([[0, 1], [1, 0]], dtype=complex)
        dm.apply_kraus([0], [K0.flatten(), K1.flatten()])
        probs = dm.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_apply_kraus_depolarizing(self):
        """Test depolarizing channel."""
        dm = DensityMatrix(1)
        p = 0.3
        K0 = np.sqrt(1 - p) * np.eye(2, dtype=complex)
        K1 = np.sqrt(p / 3) * np.array([[0, 1], [1, 0]], dtype=complex)
        K2 = np.sqrt(p / 3) * np.array([[0, -1j], [1j, 0]], dtype=complex)
        K3 = np.sqrt(p / 3) * np.array([[1, 0], [0, -1]], dtype=complex)
        dm.apply_kraus([0], [K0.flatten(), K1.flatten(), K2.flatten(), K3.flatten()])
        probs = dm.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10


class TestDensityMatrixPartialTrace:
    """Test partial trace operation."""

    def test_partial_trace_bell_state(self):
        """Test partial trace on Bell state."""
        dm = DensityMatrix(2)
        dm.apply_h(0)
        dm.apply_cx(0, 1)

        # Trace out qubit 1, keep qubit 0
        reduced = dm.partial_trace([0])
        assert reduced.num_qubits == 1
        # Bell state reduced is I/2 (maximally mixed)
        probs = reduced.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[1], 0.5, abs_tol=1e-10)

    def test_partial_trace_product_state(self):
        """Test partial trace on product state."""
        dm = DensityMatrix(2)
        dm.apply_x(0)  # |10>

        reduced = dm.partial_trace([0])
        assert reduced.num_qubits == 1
        probs = reduced.probabilities()
        assert math.isclose(probs[1], 1.0)

    def test_partial_trace_invalid_qubit(self):
        """Test partial trace with invalid qubit index."""
        dm = DensityMatrix(2)
        with pytest.raises(IndexError):
            dm.partial_trace([2])  # Out of bounds


class TestDensityMatrixExpectation:
    """Test expectation value calculations."""

    def test_expectation_pauli_string(self):
        """Test expectation with PauliString."""
        dm = DensityMatrix(1)
        ps_z = PauliString.from_str("Z")
        assert math.isclose(dm.expectation(ps_z), 1.0)

        dm.apply_h(0)
        assert math.isclose(dm.expectation(ps_z), 0.0, abs_tol=1e-10)

    def test_expectation_hamiltonian(self):
        """Test expectation with Hamiltonian."""
        dm = DensityMatrix(2)
        dm.apply_h(0)
        dm.apply_cx(0, 1)

        h = Hamiltonian(2)
        h.add_term(PauliString.from_str("ZZ"), 0.5)
        h.add_term(PauliString.from_str("XX"), 0.5)
        h.simplify()

        exp = dm.expectation(h)
        assert math.isclose(exp, 1.0, abs_tol=1e-10)

    def test_expectation_mismatched_qubits(self):
        """Test expectation with mismatched qubit count."""
        dm = DensityMatrix(1)
        h = Hamiltonian(2)
        with pytest.raises(ValueError):
            dm.expectation(h)


class TestDensityMatrixCopy:
    """Test copy functionality."""

    def test_copy(self):
        """Test that copy creates independent instance."""
        dm1 = DensityMatrix(1)
        dm1.apply_h(0)
        dm2 = dm1.copy()

        # Modify original
        dm1.apply_x(0)

        # Copy should be unchanged
        probs = dm2.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)

    def test_repr(self):
        """Test string representation."""
        dm = DensityMatrix(2)
        repr_str = repr(dm)
        assert "DensityMatrix" in repr_str
        assert "2" in repr_str or "4" in repr_str  # num_qubits or shape


class TestDensityMatrixBoundaryConditions:
    """Test boundary conditions and edge cases."""

    @pytest.mark.parametrize("num_qubits", [1, 2, 5, 8])
    def test_large_qubit_systems_initialize_correctly(self, num_qubits):
        """Test that large density matrix systems can be created."""
        dm = DensityMatrix(num_qubits)
        assert dm.num_qubits == num_qubits
        assert dm.data.shape == (2**num_qubits, 2**num_qubits)
        assert np.isclose(dm.trace(), 1.0)

    def test_from_density_matrix_with_non_hermitian_matrix(self):
        """Test that creating a density matrix with non-Hermitian input raises ValueError."""
        # Create a non-Hermitian matrix with trace 1
        non_hermitian = np.array([[1, 0.5], [-0.5, 0]], dtype=complex)
        with pytest.raises(ValueError):
            DensityMatrix.from_density_matrix(1, non_hermitian.flatten())

    def test_from_density_matrix_with_negative_eigenvalues(self):
        """Test that creating a density matrix with negative eigenvalues raises ValueError."""
        # Create a Hermitian matrix with trace 1 but negative eigenvalue
        neg_eig_matrix = np.array([[1.5, 0], [0, -0.5]], dtype=complex)
        with pytest.raises(ValueError):
            DensityMatrix.from_density_matrix(1, neg_eig_matrix.flatten())

    @pytest.mark.parametrize(
        "invalid_input",
        [
            "string",
            None,
            {"dict": "value"},
            3.14,
        ],
    )
    def test_from_state_with_invalid_type_raises_error(self, invalid_input):
        """Test that from_state rejects non-array/list inputs."""
        with pytest.raises((ValueError, TypeError)):
            DensityMatrix.from_state(1, invalid_input)

    @pytest.mark.parametrize(
        "invalid_input",
        [
            "string",
            None,
            3.14,
        ],
    )
    def test_from_density_matrix_with_invalid_type_raises_error(self, invalid_input):
        """Test that from_density_matrix rejects invalid inputs."""
        with pytest.raises((ValueError, TypeError)):
            DensityMatrix.from_density_matrix(1, invalid_input)

    @pytest.mark.parametrize("theta", [0.0, 1e-10, np.pi, 1e10 * np.pi])
    def test_rotation_gate_with_extreme_angles(self, theta):
        """Test rotation gates with extreme angle values."""
        dm = DensityMatrix(1)
        dm.apply_rx(0, theta)
        probs = dm.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10
        assert all(0 <= p <= 1 for p in probs)


class TestDensityMatrixQuantumInvariants:
    """Test quantum physical invariants and properties."""

    def test_density_matrix_is_hermitian(self):
        """Test that density matrix is Hermitian (rho = rho^dagger).

        This is a fundamental requirement for physical density matrices.
        """
        dm = DensityMatrix(2)
        dm.apply_h(0)
        dm.apply_cx(0, 1)

        data = dm.data
        # Check Hermitian: rho == rho^dagger
        assert np.allclose(data, data.conj().T, atol=1e-10), (
            "Density matrix is not Hermitian"
        )

    def test_density_matrix_is_positive_semidefinite(self):
        """Test that density matrix has non-negative eigenvalues.

        A valid density matrix must be positive semidefinite.
        """
        dm = DensityMatrix(2)
        dm.apply_h(0)
        dm.apply_cx(0, 1)

        data = dm.data
        eigenvalues = np.linalg.eigvalsh(data)  # eigvalsh for Hermitian matrices

        # All eigenvalues should be >= 0 (within numerical tolerance)
        assert np.all(eigenvalues >= -1e-10), (
            f"Density matrix has negative eigenvalues: {eigenvalues}"
        )

    def test_cptp_channel_preserves_trace(self):
        """Test that CPTP quantum channels preserve the trace.

        Completely Positive Trace-Preserving (CPTP) maps must preserve Tr(rho) = 1.
        """
        dm = DensityMatrix(1)
        dm.apply_h(0)

        # Apply depolarizing channel (CPTP)
        p = 0.3
        K0 = np.sqrt(1 - p) * np.eye(2, dtype=complex)
        K1 = np.sqrt(p / 3) * np.array([[0, 1], [1, 0]], dtype=complex)
        K2 = np.sqrt(p / 3) * np.array([[0, -1j], [1j, 0]], dtype=complex)
        K3 = np.sqrt(p / 3) * np.array([[1, 0], [0, -1]], dtype=complex)
        dm.apply_kraus([0], [K0.flatten(), K1.flatten(), K2.flatten(), K3.flatten()])

        # Trace must still be 1
        assert np.isclose(dm.trace(), 1.0, atol=1e-10), (
            "CPTP channel did not preserve trace"
        )

    def test_purity_of_pure_vs_mixed_state(self):
        """Test that pure states have Tr(rho^2) = 1 and mixed states have Tr(rho^2) < 1.

        Purity is a measure of how "quantum" a state is.
        """
        # Pure state: |+><+|
        dm_pure = DensityMatrix(1)
        dm_pure.apply_h(0)

        data_pure = dm_pure.data
        purity_pure = np.trace(data_pure @ data_pure).real
        assert np.isclose(purity_pure, 1.0, atol=1e-10), (
            f"Pure state purity should be 1, got {purity_pure}"
        )

        # Mixed state: apply bit-flip channel to |0>
        # This creates rho = (1-p)|0><0| + p|1><1| = [[1-p, 0], [0, p]]
        dm_mixed = DensityMatrix(1)
        p = 0.5
        K0 = np.sqrt(1 - p) * np.eye(2, dtype=complex)
        K1 = np.sqrt(p) * np.array([[0, 1], [1, 0]], dtype=complex)
        dm_mixed.apply_kraus([0], [K0.flatten(), K1.flatten()])

        data_mixed = dm_mixed.data
        purity_mixed = np.trace(data_mixed @ data_mixed).real
        # For p=0.5, purity should be Tr((I/2)^2) = Tr(I/4) = 0.5
        assert purity_mixed < 1.0, (
            f"Mixed state purity should be < 1, got {purity_mixed}"
        )
        assert np.isclose(purity_mixed, 0.5, atol=1e-10), (
            f"Maximally mixed state purity should be 0.5, got {purity_mixed}"
        )

    def test_bell_state_maximally_entangled(self):
        """Test that Bell state has correct entanglement entropy.

        Maximally entangled states should have reduced density matrix = I/2.
        """
        dm = DensityMatrix(2)
        dm.apply_h(0)
        dm.apply_cx(0, 1)

        # Reduced density matrix of either qubit should be maximally mixed
        reduced = dm.partial_trace([0])
        assert reduced.num_qubits == 1

        data = reduced.data
        # Should be close to I/2
        expected = 0.5 * np.eye(2, dtype=complex)
        assert np.allclose(data, expected, atol=1e-10), (
            "Reduced density matrix of Bell state is not maximally mixed"
        )

    def test_partial_trace_preserves_hermiticity_and_trace(self):
        """Test that partial trace preserves Hermiticity and trace = 1."""
        dm = DensityMatrix(3)
        dm.apply_h(0)
        dm.apply_h(1)
        dm.apply_h(2)

        # Partial trace over qubit 2, keep qubits 0 and 1
        reduced = dm.partial_trace([0, 1])
        assert reduced.num_qubits == 2

        # Check trace is preserved
        assert np.isclose(reduced.trace(), 1.0, atol=1e-10), (
            "Partial trace did not preserve trace"
        )

        # Check Hermiticity is preserved
        data = reduced.data
        assert np.allclose(data, data.conj().T, atol=1e-10), (
            "Partial trace did not preserve Hermiticity"
        )

    def test_partial_trace_on_product_state(self):
        """Test partial trace on product state gives correct pure state."""
        # Create |1+> state
        dm = DensityMatrix(2)
        dm.apply_x(0)  # |10> -> qubit 0 is |1>
        dm.apply_h(1)  # qubit 1 is |+>

        # Trace out qubit 1
        reduced = dm.partial_trace([0])
        assert reduced.num_qubits == 1

        # Should be |1><1|
        probs = reduced.probabilities()
        assert np.isclose(probs[1], 1.0, atol=1e-10)

    def test_mixed_state_expectation_values(self):
        """Test expectation values for mixed states.

        For mixed state rho = 0.5|0><0| + 0.5|1><1| (maximally mixed),
        <Z> should be 0.
        """
        # Create maximally mixed state via depolarizing
        dm = DensityMatrix(1)
        # Complete depolarization with p = 1.0
        K0 = np.sqrt(0.5) * np.eye(2, dtype=complex)
        K1 = np.sqrt(0.5) * np.array([[0, 1], [1, 0]], dtype=complex)
        dm.apply_kraus([0], [K0.flatten(), K1.flatten()])

        # For maximally mixed state, <Z> = 0
        ps_z = PauliString.from_str("Z")
        exp_z = dm.expectation(ps_z)
        assert np.isclose(exp_z, 0.0, atol=1e-10), (
            f"Maximally mixed state <Z> should be 0, got {exp_z}"
        )

        # <X> should also be 0
        ps_x = PauliString.from_str("X")
        exp_x = dm.expectation(ps_x)
        assert np.isclose(exp_x, 0.0, atol=1e-10), (
            f"Maximally mixed state <X> should be 0, got {exp_x}"
        )


class TestDensityMatrixNumericalPrecision:
    """Test numerical precision for density matrix operations."""

    @pytest.mark.parametrize("num_applications", [1, 10, 50])
    def test_repeated_operations_preserve_trace(self, num_applications):
        """Test that repeated operations maintain trace = 1."""
        dm = DensityMatrix(2)

        for _ in range(num_applications):
            dm.apply_h(0)
            dm.apply_cx(0, 1)
            dm.apply_h(0)  # Undo Hadamard
            dm.apply_cx(0, 1)  # Undo CNOT

        assert np.isclose(dm.trace(), 1.0, atol=1e-10), (
            f"Trace drift after {num_applications} operations: {dm.trace()}"
        )

    def test_small_angle_rotation_preserves_hermiticity(self):
        """Test that small angle rotations maintain Hermiticity."""
        dm = DensityMatrix(1)
        dm.apply_h(0)

        # Very small rotation
        dm.apply_rx(0, 1e-12)

        data = dm.data
        assert np.allclose(data, data.conj().T, atol=1e-10), (
            "Hermiticity lost after small angle rotation"
        )


class TestDensityMatrixToleranceDefaults:
    """Test that physical-constraint checks default tol=1e-10 when omitted.

    The Rust bindings declare ``f64`` parameters with
    ``#[pyo3(signature = (tol=1e-10))]``, so no-arg calls must work and the
    Python-visible default matches ``density_matrix.pyi``'s ``tol: float = 1e-10``.
    """

    @pytest.fixture
    def valid_dm(self):
        dm = DensityMatrix(1)
        dm.apply_h(0)
        return dm

    def test_signatures_expose_1e_10_default(self):
        """The Python-visible default must be 1e-10, not required or None."""
        for method in (
            DensityMatrix.is_hermitian,
            DensityMatrix.is_positive_semidefinite,
            DensityMatrix.validate_physical,
        ):
            assert inspect.signature(method).parameters["tol"].default == 1e-10

    def test_is_hermitian_no_arg_defaults_to_1e_10(self, valid_dm):
        """is_hermitian() without arguments must not raise TypeError."""
        assert valid_dm.is_hermitian() is True
        # Explicit tolerance must keep working, positional and keyword.
        assert valid_dm.is_hermitian(1e-10) is True
        assert valid_dm.is_hermitian(tol=1e-12) is True

    def test_is_hermitian_with_custom_tol(self, valid_dm):
        """Tight tolerance still passes for an exactly prepared state."""
        assert valid_dm.is_hermitian(tol=1e-15) is True

    def test_is_positive_semidefinite_no_arg_defaults_to_1e_10(self, valid_dm):
        """is_positive_semidefinite() without arguments must not raise TypeError."""
        assert valid_dm.is_positive_semidefinite() is True
        assert valid_dm.is_positive_semidefinite(1e-10) is True
        assert valid_dm.is_positive_semidefinite(tol=1e-8) is True

    def test_validate_physical_no_arg_defaults_to_1e_10(self, valid_dm):
        """validate_physical() without arguments must not raise TypeError."""
        assert valid_dm.validate_physical() is None
        assert valid_dm.validate_physical(1e-10) is None
        assert valid_dm.validate_physical(tol=1e-8) is None

    def test_tol_rejects_none(self, valid_dm):
        """tol is typed float; passing None must raise TypeError, matching the stub."""
        with pytest.raises(TypeError):
            valid_dm.is_hermitian(None)
        with pytest.raises(TypeError):
            valid_dm.validate_physical(None)


class TestDensityMatrixCopySemantics:
    """Test copy behavior and independence."""

    def test_copy_creates_independent_instance(self):
        """Test that copy() creates a fully independent density matrix."""
        dm1 = DensityMatrix(2)
        dm1.apply_h(0)
        dm1.apply_cx(0, 1)

        dm2 = dm1.copy()

        # Modify dm1
        dm1.apply_x(0)
        dm1.apply_ry(1, np.pi / 4)

        # Verify dm2 is unchanged (still in Bell state)
        probs = dm2.probabilities()
        assert np.isclose(probs[0], 0.5, atol=1e-10)
        assert np.isclose(probs[3], 0.5, atol=1e-10)

    def test_data_array_is_copy_not_reference(self):
        """Test that data property returns a copy, not internal reference."""
        dm = DensityMatrix(1)
        dm.apply_h(0)

        data1 = dm.data
        original_val = data1[0, 0]
        data1[0, 0] = 999.0

        data2 = dm.data
        assert np.isclose(data2[0, 0], original_val, atol=1e-10), (
            "Modifying returned data array affected density matrix"
        )


class TestDensityMatrixMaximallyMixed:
    """Test the maximally mixed state constructor."""

    @pytest.mark.parametrize("num_qubits", [0, 1, 2])
    def test_matrix_is_identity_over_dim(self, num_qubits):
        """Test the matrix equals I / 2^N."""
        dim = 2**num_qubits
        dm = DensityMatrix.maximally_mixed(num_qubits)
        assert dm.num_qubits == num_qubits
        data = dm.data
        assert data.shape == (dim, dim)
        np.testing.assert_allclose(data, np.eye(dim, dtype=complex) / dim, atol=1e-12)

    def test_trace_and_probabilities(self):
        """Test physical-state invariants: trace 1, uniform probabilities."""
        dm = DensityMatrix.maximally_mixed(2)
        assert np.isclose(dm.trace(), 1.0)
        assert dm.probabilities() == pytest.approx([0.25, 0.25, 0.25, 0.25])
        assert dm.is_hermitian(1e-10)
