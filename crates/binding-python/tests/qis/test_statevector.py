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

"""Tests for Statevector quantum state simulation."""

import pytest
import math
import numpy as np
from cqlib.circuit import Circuit, StandardGate
from cqlib.qis import Statevector, Hamiltonian, PauliString


class TestStatevectorConstruction:
    """Test Statevector constructors."""

    def test_new(self):
        """Test basic constructor creates |0...0>."""
        sv = Statevector(2)
        assert sv.num_qubits == 2
        probs = sv.probabilities()
        assert len(probs) == 4
        assert math.isclose(probs[0], 1.0)
        assert math.isclose(probs[1], 0.0)
        assert math.isclose(probs[2], 0.0)
        assert math.isclose(probs[3], 0.0)

    def test_from_state(self):
        """Test construction from initial amplitudes."""
        # Create |+> state
        amps = np.array([1 / np.sqrt(2), 1 / np.sqrt(2)], dtype=complex)
        sv = Statevector.from_state(1, amps)
        assert sv.num_qubits == 1
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.5)
        assert math.isclose(probs[1], 0.5)

    def test_from_state_with_list(self):
        """Test construction from list of complex numbers."""
        amps = [1 / np.sqrt(2) + 0j, 1 / np.sqrt(2) + 0j]
        sv = Statevector.from_state(1, amps)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.5)

    def test_from_state_invalid_length(self):
        """Test from_state with invalid state length."""
        amps = np.array([1.0, 0.0, 0.0], dtype=complex)
        with pytest.raises(ValueError):
            Statevector.from_state(1, amps)

    def test_from_circuit(self):
        """Test construction from circuit simulation."""
        circuit = Circuit(2)
        circuit.h(0)
        circuit.cx(0, 1)

        sv = Statevector.from_circuit(circuit)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[1], 0.0, abs_tol=1e-10)
        assert math.isclose(probs[2], 0.0, abs_tol=1e-10)
        assert math.isclose(probs[3], 0.5, abs_tol=1e-10)


class TestStatevectorProperties:
    """Test Statevector properties."""

    def test_num_qubits(self):
        """Test num_qubits property."""
        sv = Statevector(3)
        assert sv.num_qubits == 3

    def test_data(self):
        """Test data property returns correct shape."""
        sv = Statevector(2)
        data = sv.data
        assert len(data) == 4
        # Check |00> state
        assert data[0] == 1.0
        assert np.allclose(data[1:], 0)

    def test_probabilities(self):
        """Test probabilities extraction."""
        sv = Statevector(1)
        sv.apply_h(0)
        probs = sv.probabilities()
        assert len(probs) == 2
        assert math.isclose(probs[0], 0.5)
        assert math.isclose(probs[1], 0.5)

    def test_probabilities_sum_to_one(self):
        """Test that probabilities always sum to 1."""
        sv = Statevector(3)
        sv.apply_h(0)
        sv.apply_h(1)
        sv.apply_h(2)
        probs = sv.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10


class TestStatevectorSingleQubitGates:
    """Test single-qubit gate operations."""

    def test_apply_standard_gate(self):
        """Test applying gates via the general standard gate interface."""
        # Single qubit no params
        sv = Statevector(1)
        sv.apply_standard_gate(StandardGate.X, [0])
        probs = sv.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

        # Single qubit with params
        sv = Statevector(1)
        sv.apply_standard_gate(StandardGate.RX, [0], [np.pi])
        probs = sv.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

        # Two qubit
        sv = Statevector(2)
        sv.apply_standard_gate(StandardGate.X, [0])
        sv.apply_standard_gate(StandardGate.CX, [0, 1])
        probs = sv.probabilities()
        assert math.isclose(probs[3], 1.0, abs_tol=1e-10)

    def test_apply_x(self):
        """Test Pauli-X gate."""
        sv = Statevector(1)
        sv.apply_x(0)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.0)
        assert math.isclose(probs[1], 1.0)

    def test_apply_y(self):
        """Test Pauli-Y gate."""
        sv = Statevector(1)
        sv.apply_y(0)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.0)
        assert math.isclose(probs[1], 1.0)

    def test_apply_z(self):
        """Test Pauli-Z gate (no effect on |0>)."""
        sv = Statevector(1)
        sv.apply_z(0)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 1.0)
        assert math.isclose(probs[1], 0.0)

    def test_apply_z_on_superposition(self):
        """Test Pauli-Z gate on |+> state."""
        sv = Statevector(1)
        sv.apply_h(0)  # |+>
        sv.apply_z(0)  # |->
        # Measure in X basis via Hadamard
        sv.apply_h(0)
        probs = sv.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_h(self):
        """Test Hadamard gate."""
        sv = Statevector(1)
        sv.apply_h(0)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.5)
        assert math.isclose(probs[1], 0.5)

    def test_apply_s_and_sdg(self):
        """Test S and S-dagger gates."""
        sv = Statevector(1)
        sv.apply_h(0)
        sv.apply_s(0)
        sv.apply_sdg(0)
        # S * Sdg = I
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)

    def test_apply_t_and_tdg(self):
        """Test T and T-dagger gates."""
        sv = Statevector(1)
        sv.apply_h(0)
        sv.apply_t(0)
        sv.apply_tdg(0)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)

    def test_apply_rx(self):
        """Test RX rotation."""
        sv = Statevector(1)
        sv.apply_rx(0, np.pi)
        # RX(pi) = -iX, rotates |0> to |1> (up to phase)
        probs = sv.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_rx_pi_half(self):
        """Test RX(pi/2) rotation."""
        sv = Statevector(1)
        sv.apply_rx(0, np.pi / 2)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[1], 0.5, abs_tol=1e-10)

    def test_apply_ry(self):
        """Test RY rotation."""
        sv = Statevector(1)
        sv.apply_ry(0, np.pi / 2)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[1], 0.5, abs_tol=1e-10)

    def test_apply_rz(self):
        """Test RZ rotation (no effect on |0>)."""
        sv = Statevector(1)
        sv.apply_rz(0, np.pi / 4)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 1.0)

    def test_apply_p(self):
        """Test phase gate (no effect on |0>)."""
        sv = Statevector(1)
        sv.apply_phase(0, np.pi / 4)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 1.0)

    def test_apply_x2p(self):
        """Test X2P (sqrt(X)) gate."""
        sv = Statevector(1)
        sv.apply_x2p(0)
        sv.apply_x2p(0)
        # X2P * X2P = X
        probs = sv.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_x2m(self):
        """Test X2M (sqrt(X) dagger) gate."""
        sv = Statevector(1)
        sv.apply_x2m(0)
        sv.apply_x2m(0)
        # X2M * X2M = X
        probs = sv.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_y2p(self):
        """Test Y2P (sqrt(Y)) gate."""
        sv = Statevector(1)
        sv.apply_y2p(0)
        sv.apply_y2p(0)
        # Y2P * Y2P = Y
        probs = sv.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_y2m(self):
        """Test Y2M (sqrt(Y) dagger) gate."""
        sv = Statevector(1)
        sv.apply_y2m(0)
        sv.apply_y2m(0)
        probs = sv.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_u(self):
        """Test general U gate."""
        sv = Statevector(1)
        # U(pi, 0, 0) = Ry(pi) = X (up to phase)
        sv.apply_u(0, np.pi, 0, 0)
        probs = sv.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_gphase(self):
        """Test global phase (affects statevector but not probabilities)."""
        sv = Statevector(1)
        sv.apply_h(0)
        sv.apply_gphase(np.pi / 4)
        # Global phase doesn't affect probabilities
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)


class TestStatevectorTwoQubitGates:
    """Test two-qubit gate operations."""

    def test_apply_cx(self):
        """Test CNOT gate."""
        sv = Statevector(2)
        sv.apply_h(0)
        sv.apply_cx(0, 1)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.5)
        assert math.isclose(probs[3], 0.5)

    def test_apply_cx_reverse(self):
        """Test CNOT with control and target swapped."""
        sv = Statevector(2)
        sv.apply_x(0)  # |01>
        sv.apply_h(1)  # |0+>
        sv.apply_cx(1, 0)
        probs = sv.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_apply_cy(self):
        """Test CY gate."""
        sv = Statevector(2)
        sv.apply_h(0)
        sv.apply_cy(0, 1)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[3], 0.5, abs_tol=1e-10)

    def test_apply_cz(self):
        """Test CZ gate."""
        sv = Statevector(2)
        sv.apply_h(0)
        sv.apply_h(1)
        sv.apply_cz(0, 1)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.25, abs_tol=1e-10)
        assert math.isclose(probs[3], 0.25, abs_tol=1e-10)

    def test_apply_swap(self):
        """Test SWAP gate."""
        sv = Statevector(2)
        sv.apply_x(1)  # |10> (qubit 1 is high-order bit)
        sv.apply_swap(0, 1)  # |01>
        probs = sv.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_crx(self):
        """Test controlled-RX."""
        sv = Statevector(2)
        sv.apply_h(0)
        sv.apply_crx(0, 1, np.pi)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[3], 0.5, abs_tol=1e-10)

    def test_apply_cry(self):
        """Test controlled-RY."""
        sv = Statevector(2)
        sv.apply_h(0)
        sv.apply_cry(0, 1, np.pi / 2)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)

    def test_apply_crz(self):
        """Test controlled-RZ."""
        sv = Statevector(2)
        sv.apply_h(0)
        sv.apply_h(1)
        sv.apply_crz(0, 1, np.pi / 4)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.25, abs_tol=1e-10)

    def test_apply_rxx(self):
        """Test RXX gate."""
        sv = Statevector(2)
        sv.apply_rxx(0, 1, np.pi / 2)
        probs = sv.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_apply_ryy(self):
        """Test RYY gate."""
        sv = Statevector(2)
        sv.apply_ryy(0, 1, np.pi / 2)
        probs = sv.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_apply_rzz(self):
        """Test RZZ gate."""
        sv = Statevector(2)
        sv.apply_h(0)
        sv.apply_h(1)
        sv.apply_rzz(0, 1, np.pi)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.25, abs_tol=1e-10)

    def test_apply_rzx(self):
        """Test RZX gate."""
        sv = Statevector(2)
        sv.apply_h(0)
        sv.apply_rzx(0, 1, np.pi / 4)
        probs = sv.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10


class TestStatevectorThreeQubitGates:
    """Test three-qubit gate operations."""

    def test_apply_ccx(self):
        """Test Toffoli (CCX) gate."""
        sv = Statevector(3)
        sv.apply_x(0)
        sv.apply_x(1)
        sv.apply_ccx(0, 1, 2)
        probs = sv.probabilities()
        assert math.isclose(probs[7], 1.0, abs_tol=1e-10)

    def test_apply_ccx_not_triggered(self):
        """Test Toffoli when not all controls are 1."""
        sv = Statevector(3)
        sv.apply_x(0)  # Only one control is 1
        sv.apply_ccx(0, 1, 2)
        probs = sv.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)


class TestStatevectorXYGates:
    """Test XY family of gates."""

    def test_apply_xy(self):
        """Test XY gate."""
        sv = Statevector(1)
        sv.apply_xy(0, np.pi / 4)
        probs = sv.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_apply_xy2p(self):
        """Test XY2P gate."""
        sv = Statevector(1)
        theta = np.pi / 4
        sv.apply_xy2p(0, theta)

        k = 1.0 / math.sqrt(2)
        expected_0 = k * 1.0
        expected_1 = k * (-1j * np.exp(1j * theta))

        data = sv.data
        assert np.allclose(data[0], expected_0)
        assert np.allclose(data[1], expected_1)

    def test_apply_xy2m(self):
        """Test XY2M gate."""
        sv = Statevector(1)
        theta = np.pi / 4
        sv.apply_xy2m(0, theta)

        k = 1.0 / math.sqrt(2)
        expected_0 = k * 1.0
        expected_1 = k * (1j * np.exp(1j * theta))

        data = sv.data
        assert np.allclose(data[0], expected_0)
        assert np.allclose(data[1], expected_1)

    def test_apply_rxy(self):
        """Test RXY gate."""
        sv = Statevector(1)
        sv.apply_rxy(0, np.pi / 2, 0)
        probs = sv.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[1], 0.5, abs_tol=1e-10)


class TestStatevectorFSim:
    """Test fSim gate."""

    def test_apply_fsim_from_01(self):
        """Test fSim gate from |01> state."""
        sv = Statevector(2)
        sv.apply_x(0)  # |01>
        theta = np.pi / 4
        phi = np.pi / 2
        sv.apply_fsim(0, 1, theta, phi)

        data = sv.data
        assert np.allclose(data[1], np.cos(theta), atol=1e-10)
        assert np.allclose(data[2], -1j * np.sin(theta), atol=1e-10)

    def test_apply_fsim_from_11(self):
        """Test fSim gate from |11> state."""
        sv = Statevector(2)
        sv.apply_x(0)
        sv.apply_x(1)  # |11>
        theta = np.pi / 4
        phi = np.pi / 2
        sv.apply_fsim(0, 1, theta, phi)

        data = sv.data
        assert np.allclose(data[3], np.exp(-1j * phi), atol=1e-10)


class TestStatevectorCustomGates:
    """Test custom gate application."""

    def test_apply_single_qubit_gate(self):
        """Test custom single-qubit gate."""
        sv = Statevector(1)
        # X gate matrix
        x_matrix = np.array([[0, 1], [1, 0]], dtype=complex)
        sv.apply_single_qubit_gate(0, x_matrix)
        probs = sv.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_single_qubit_gate_invalid(self):
        """Test invalid single-qubit gate dimensions."""
        sv = Statevector(1)
        invalid_matrix = np.array([[1, 0, 0], [0, 1, 0]], dtype=complex)
        with pytest.raises(ValueError):
            sv.apply_single_qubit_gate(0, invalid_matrix)

    def test_apply_double_qubits_gate(self):
        """Test custom two-qubit gate."""
        sv = Statevector(2)
        # SWAP gate as 4x4 matrix
        swap_matrix = np.array(
            [[1, 0, 0, 0], [0, 0, 1, 0], [0, 1, 0, 0], [0, 0, 0, 1]], dtype=complex
        )
        sv.apply_x(1)  # |10> (qubit 1 is high-order bit)
        sv.apply_double_qubits_gate(0, 1, swap_matrix)  # |01>
        probs = sv.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_double_qubits_gate_invalid(self):
        """Test invalid two-qubit gate dimensions."""
        sv = Statevector(2)
        invalid_matrix = np.eye(4, 5, dtype=complex)
        with pytest.raises(ValueError):
            sv.apply_double_qubits_gate(0, 1, invalid_matrix)


class TestStatevectorExpectation:
    """Test expectation value calculations."""

    def test_expectation_pauli_string(self):
        """Test expectation with PauliString."""
        sv = Statevector(1)
        ps_z = PauliString.from_str("Z")
        assert math.isclose(sv.expectation(ps_z), 1.0, abs_tol=1e-10)

        sv.apply_h(0)
        assert math.isclose(sv.expectation(ps_z), 0.0, abs_tol=1e-10)

    def test_expectation_hamiltonian(self):
        """Test expectation with Hamiltonian."""
        sv = Statevector(2)
        sv.apply_h(0)
        sv.apply_cx(0, 1)

        h = Hamiltonian(2)
        h.add_term(PauliString.from_str("ZZ"), 0.5)
        h.add_term(PauliString.from_str("XX"), 0.5)
        h.simplify()

        exp = sv.expectation(h)
        assert math.isclose(exp, 1.0, abs_tol=1e-10)

    def test_expectation_mismatched_qubits(self):
        """Test expectation with mismatched qubit count."""
        sv = Statevector(1)
        h = Hamiltonian(2)
        with pytest.raises(ValueError):
            sv.expectation(h)

    def test_expectation_invalid_observable(self):
        """Test expectation with invalid observable type."""
        sv = Statevector(1)
        with pytest.raises(ValueError):
            sv.expectation("invalid")


class TestStatevectorCopy:
    """Test copy functionality."""

    def test_copy(self):
        """Test that copy creates independent instance."""
        sv1 = Statevector(1)
        sv1.apply_h(0)
        sv2 = sv1.copy()

        # Modify original
        sv1.apply_x(0)

        # Copy should be unchanged
        probs = sv2.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)

    def test_repr(self):
        """Test string representation."""
        sv = Statevector(2)
        repr_str = repr(sv)
        assert "Statevector" in repr_str


class TestStatevectorBoundaryConditions:
    """Test boundary conditions and edge cases."""

    @pytest.mark.parametrize("num_qubits", [1, 2, 5, 10])
    def test_large_qubit_systems_initialize_correctly(self, num_qubits):
        """Test that large qubit systems can be created and initialized."""
        sv = Statevector(num_qubits)
        assert sv.num_qubits == num_qubits
        assert len(sv.data) == 2**num_qubits
        assert sv.data[0] == 1.0 + 0j
        assert np.allclose(sv.data[1:], 0j)

    @pytest.mark.parametrize("theta", [0.0, 1e-10, np.pi, 1e10 * np.pi, -1e10 * np.pi])
    def test_rotation_gate_with_extreme_angles(self, theta):
        """Test rotation gates with extreme angle values for numerical stability."""
        sv = Statevector(1)
        sv.apply_rx(0, theta)
        probs = sv.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10
        assert all(0 <= p <= 1 for p in probs)

    def test_from_state_with_zero_amplitudes_raises(self):
        """Test that all-zero initial state raises error (not normalized)."""
        amps = np.zeros(4, dtype=complex)
        with pytest.raises(ValueError):
            Statevector.from_state(2, amps)

    def test_from_state_with_non_normalized_state_raises(self):
        """Test that non-normalized initial state raises error."""
        amps = np.array([1.0, 1.0, 0.0, 0.0], dtype=complex)  # norm = sqrt(2)
        with pytest.raises(ValueError):
            Statevector.from_state(2, amps)

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
            Statevector.from_state(1, invalid_input)


class TestStatevectorQuantumInvariants:
    """Test quantum physical invariants and properties."""

    @pytest.mark.parametrize(
        "gate_sequence",
        [
            [lambda sv: sv.apply_h(0)],
            [
                lambda sv: sv.apply_x(0),
                lambda sv: sv.apply_y(0),
                lambda sv: sv.apply_z(0),
            ],
            [
                lambda sv: sv.apply_rx(0, np.pi / 4),
                lambda sv: sv.apply_ry(0, np.pi / 3),
            ],
            [lambda sv: sv.apply_h(0), lambda sv: sv.apply_cx(0, 1)],
            [
                lambda sv: sv.apply_h(0),
                lambda sv: sv.apply_h(1),
                lambda sv: sv.apply_cz(0, 1),
            ],
        ],
    )
    def test_normalization_preserved_after_gate_sequence(self, gate_sequence):
        """Test that statevector norm is preserved after any gate sequence."""
        sv = Statevector(2)

        for gate in gate_sequence:
            gate(sv)

        norm_squared = np.sum(np.abs(sv.data) ** 2)
        assert np.isclose(norm_squared, 1.0, atol=1e-10)

    def test_probability_distribution_properties(self):
        """Test that probabilities satisfy mathematical requirements."""
        sv = Statevector(2)
        sv.apply_h(0)
        sv.apply_h(1)

        probs = sv.probabilities()
        data = sv.data

        assert len(probs) == 2**sv.num_qubits

        for i, p in enumerate(probs):
            assert 0 <= p <= 1, f"Probability {p} at index {i} out of range"
            expected_p = abs(data[i]) ** 2
            assert np.isclose(p, expected_p, atol=1e-10)

        assert np.isclose(sum(probs), 1.0, atol=1e-10)

    def test_global_phase_invariance_of_probabilities(self):
        """Test that global phase doesn't affect measurement probabilities."""
        sv1 = Statevector(1)
        sv1.apply_h(0)

        sv2 = Statevector(1)
        sv2.apply_h(0)
        sv2.apply_gphase(np.pi / 4)

        assert np.allclose(sv1.probabilities(), sv2.probabilities(), atol=1e-10)

    def test_global_phase_affects_statevector_data(self):
        """Test that global phase is correctly applied to statevector."""
        sv = Statevector(1)
        sv.apply_h(0)

        original_data = sv.data.copy()
        sv.apply_gphase(np.pi / 2)
        phased_data = sv.data

        expected = original_data * 1j
        assert np.allclose(phased_data, expected, atol=1e-10)

    @pytest.mark.parametrize(
        "pauli_str,state_prep,expected_expectation",
        [
            ("Z", lambda sv: None, 1.0),
            ("Z", lambda sv: sv.apply_x(0), -1.0),
            ("Z", lambda sv: sv.apply_h(0), 0.0),
            ("X", lambda sv: None, 0.0),
            ("X", lambda sv: sv.apply_h(0), 1.0),
            ("Y", lambda sv: None, 0.0),
            ("I", lambda sv: sv.apply_h(0), 1.0),
        ],
    )
    def test_pauli_expectation_values(
        self, pauli_str, state_prep, expected_expectation
    ):
        """Test expectation values of Pauli operators on prepared states."""
        sv = Statevector(1)
        state_prep(sv)

        ps = PauliString.from_str(pauli_str)
        exp_val = sv.expectation(ps)

        assert np.isclose(exp_val, expected_expectation, atol=1e-10)

    def test_bell_state_correlations(self):
        """Test that Bell state shows perfect correlations."""
        sv = Statevector(2)
        sv.apply_h(0)
        sv.apply_cx(0, 1)

        zz = PauliString.from_str("ZZ")
        assert np.isclose(sv.expectation(zz), 1.0, atol=1e-10)

        xx = PauliString.from_str("XX")
        assert np.isclose(sv.expectation(xx), 1.0, atol=1e-10)

        zi = PauliString.from_str("ZI")
        assert np.isclose(sv.expectation(zi), 0.0, atol=1e-10)

    def test_hermitian_expectation_real_value(self):
        """Test that expectation values of Hermitian operators are real."""
        sv = Statevector(2)
        sv.apply_h(0)
        sv.apply_h(1)

        h = Hamiltonian(2)
        h.add_term(PauliString.from_str("ZZ"), 0.5)
        h.add_term(PauliString.from_str("XX"), 0.3)
        h.simplify()

        exp_val = sv.expectation(h)
        assert np.isreal(exp_val) or abs(np.imag(exp_val)) < 1e-10


class TestStatevectorNumericalPrecision:
    """Test numerical precision and floating-point behavior."""

    @pytest.mark.parametrize("num_applications", [1, 10, 100])
    def test_repeated_hadamard_precision(self, num_applications):
        """Test that repeated Hadamard applications maintain precision."""
        sv = Statevector(1)
        original = sv.copy()

        for _ in range(num_applications):
            sv.apply_h(0)

        if num_applications % 2 == 0:
            assert np.allclose(sv.data, original.data, atol=1e-10)

        norm = np.sum(np.abs(sv.data) ** 2)
        assert np.isclose(norm, 1.0, atol=1e-10)

    def test_small_angle_rotation_precision(self):
        """Test that very small rotation angles don't cause numerical issues."""
        sv = Statevector(1)
        sv.apply_rx(0, 1e-12)

        assert np.isclose(abs(sv.data[0]), 1.0, atol=1e-10)
        norm = np.sum(np.abs(sv.data) ** 2)
        assert np.isclose(norm, 1.0, atol=1e-10)


class TestStatevectorCopySemantics:
    """Test copy behavior and independence of copies."""

    def test_copy_creates_independent_instance(self):
        """Test that copy() creates a fully independent statevector."""
        sv1 = Statevector(1)
        sv1.apply_h(0)

        sv2 = sv1.copy()

        # Apply RY rotation to modify sv1 to a different state
        sv1.apply_ry(0, np.pi / 4)

        # Verify sv2 is unchanged (still |+>)
        probs = sv2.probabilities()
        assert np.isclose(probs[0], 0.5, atol=1e-10)
        assert np.isclose(probs[1], 0.5, atol=1e-10)

        # Verify sv1 has changed (not equal to |+>)
        probs1 = sv1.probabilities()
        # RY(pi/4) rotates |+> to a state with different probabilities
        assert not np.isclose(probs1[0], 0.5, atol=1e-10) or not np.isclose(
            probs1[1], 0.5, atol=1e-10
        )

    def test_data_array_is_copy_not_reference(self):
        """Test that data property returns a copy, not internal reference."""
        sv = Statevector(1)
        sv.apply_h(0)

        data1 = sv.data
        original_val = data1[0]
        data1[0] = 999.0

        data2 = sv.data
        assert np.isclose(data2[0], original_val, atol=1e-10)


class TestStatevectorErrorHandling:
    """Test error handling for Statevector operations."""

    def test_qubit_out_of_bounds_single_qubit_gate(self):
        """Test qubit index out of bounds for single-qubit gates raises IndexError."""
        sv = Statevector(2)

        with pytest.raises(IndexError, match="out of bounds"):
            sv.apply_x(2)

    def test_qubit_out_of_bounds_negative_effective(self):
        """Test that negative qubit indices (converted to large usize) raise IndexError."""
        sv = Statevector(2)

        # In Python, -1 would be converted to a very large usize in Rust,
        # which would then be caught by the bounds check
        with pytest.raises((IndexError, OverflowError)):
            sv.apply_h(-1)

    def test_two_qubit_gates_out_of_bounds(self):
        """Test two-qubit gates with out of bounds indices."""
        sv = Statevector(2)

        with pytest.raises(IndexError, match="out of bounds"):
            sv.apply_cx(0, 3)

        with pytest.raises(IndexError, match="out of bounds"):
            sv.apply_cx(3, 0)

        with pytest.raises(IndexError, match="out of bounds"):
            sv.apply_swap(0, 5)

    def test_two_qubit_gates_same_qubit(self):
        """Test two-qubit gates with same qubit raises error."""
        sv = Statevector(2)

        with pytest.raises(ValueError, match="distinct|same"):
            sv.apply_cx(0, 0)

        with pytest.raises(ValueError, match="distinct|same"):
            sv.apply_swap(1, 1)

        with pytest.raises(ValueError, match="distinct|same"):
            sv.apply_cz(0, 0)

    def test_three_qubit_gate_out_of_bounds(self):
        """Test CCX with out of bounds qubit index."""
        sv = Statevector(3)

        with pytest.raises(IndexError, match="out of bounds"):
            sv.apply_ccx(0, 1, 5)

    def test_three_qubit_gate_duplicate_qubits(self):
        """Test CCX with duplicate qubits raises error."""
        sv = Statevector(3)

        with pytest.raises(ValueError, match="(?i)duplicate|distinct|same"):
            sv.apply_ccx(0, 0, 1)

        with pytest.raises(ValueError, match="(?i)duplicate|distinct|same"):
            sv.apply_ccx(0, 1, 1)

        with pytest.raises(ValueError, match="(?i)duplicate|distinct|same"):
            sv.apply_ccx(0, 1, 0)

    def test_custom_single_qubit_gate_out_of_bounds(self):
        """Test custom single-qubit gate with out of bounds index."""
        sv = Statevector(2)
        matrix = np.array([[0, 1], [1, 0]], dtype=complex)

        with pytest.raises(IndexError, match="out of bounds"):
            sv.apply_single_qubit_gate(5, matrix)

    def test_custom_two_qubit_gate_out_of_bounds(self):
        """Test custom two-qubit gate with out of bounds index."""
        sv = Statevector(2)
        matrix = np.eye(4, dtype=complex)

        with pytest.raises(IndexError, match="out of bounds"):
            sv.apply_double_qubits_gate(0, 5, matrix)

    def test_custom_two_qubit_gate_same_qubit(self):
        """Test custom two-qubit gate with same qubit raises error."""
        sv = Statevector(2)
        matrix = np.eye(4, dtype=complex)

        with pytest.raises(ValueError, match="distinct|same"):
            sv.apply_double_qubits_gate(0, 0, matrix)

    def test_valid_operations_after_error(self):
        """Test that valid operations still work after an error."""
        sv = Statevector(2)

        # Trigger an error
        with pytest.raises(IndexError):
            sv.apply_x(5)

        # Verify the statevector is still functional
        sv.apply_h(0)
        sv.apply_cx(0, 1)
        probs = sv.probabilities()
        assert np.isclose(probs[0], 0.5, atol=1e-10)
        assert np.isclose(probs[3], 0.5, atol=1e-10)
