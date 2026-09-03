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

"""Tests for DensityMatrixNoise quantum simulator with noise."""

import pytest
import math
import numpy as np
from cqlib.circuit import Circuit, StandardGate
from cqlib.qis.state import DensityMatrixNoise
from cqlib.qis import PauliString, Hamiltonian
from cqlib.device import NoiseModel, SingleQubitNoise, TwoQubitNoise, ReadoutError
from cqlib.qis import Pauli


class TestDensityMatrixNoiseConstruction:
    """Test DensityMatrixNoise constructors."""

    def test_new_no_noise(self):
        """Test basic constructor without noise model."""
        sim = DensityMatrixNoise(2)
        assert sim.num_qubits == 2
        probs = sim.probabilities()
        assert len(probs) == 4
        assert math.isclose(probs[0], 1.0)

    def test_new_with_noise_model(self):
        """Test constructor with noise model."""
        noise_model = NoiseModel()
        sim = DensityMatrixNoise(2, noise_model)
        assert sim.num_qubits == 2

    def test_from_circuit_no_noise(self):
        """Test from_circuit without noise."""
        circuit = Circuit(2)
        circuit.h(0)
        circuit.cx(0, 1)

        sim = DensityMatrixNoise.from_circuit(circuit)
        assert sim.num_qubits == 2
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[3], 0.5, abs_tol=1e-10)

    def test_from_circuit_with_noise(self):
        """Test from_circuit with noise model."""
        circuit = Circuit(2)
        circuit.h(0)
        circuit.cx(0, 1)

        noise_model = NoiseModel()
        sim = DensityMatrixNoise.from_circuit(circuit, noise_model)
        assert sim.num_qubits == 2


class TestDensityMatrixNoiseProperties:
    """Test DensityMatrixNoise properties."""

    def test_num_qubits(self):
        """Test num_qubits property."""
        sim = DensityMatrixNoise(3)
        assert sim.num_qubits == 3

    def test_state(self):
        """Test state property returns correct shape and initial state is |00><00|."""
        sim = DensityMatrixNoise(2)
        state = sim.state
        assert state.shape == (4, 4)
        # Verify initial state is |00><00| (pure state)
        assert math.isclose(state[0, 0].real, 1.0, abs_tol=1e-10)
        assert math.isclose(state[0, 0].imag, 0.0, abs_tol=1e-10)
        # All other elements should be zero
        assert np.allclose(state[1:, :], 0.0, atol=1e-10)
        assert np.allclose(state[:, 1:], 0.0, atol=1e-10)
        # Verify trace is 1
        assert math.isclose(np.trace(state).real, 1.0, abs_tol=1e-10)

    def test_probabilities(self):
        """Test probabilities extraction."""
        sim = DensityMatrixNoise(1)
        sim.apply_h(0)
        probs = sim.probabilities()
        assert len(probs) == 2
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[1], 0.5, abs_tol=1e-10)

    def test_probabilities_with_readout(self):
        """Test probabilities with readout error modeling."""
        sim = DensityMatrixNoise(2)
        sim.apply_h(0)
        probs = sim.probabilities_with_readout([0, 1])
        assert len(probs) == 4
        assert abs(sum(probs) - 1.0) < 1e-10


class TestDensityMatrixNoiseSingleQubitGates:
    """Test single-qubit gate operations with noise."""

    def test_apply_standard_gate_noise(self):
        """Test applying gates via the general standard gate noise interface."""
        # Single qubit ideal
        sim = DensityMatrixNoise(1)
        sim.apply_standard_gate_noise(StandardGate.X, [0])
        probs = sim.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

        # Single qubit parametric
        sim = DensityMatrixNoise(1)
        sim.apply_standard_gate_noise(StandardGate.RX, [0], [np.pi])
        probs = sim.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

        # Two qubit
        sim = DensityMatrixNoise(2)
        sim.apply_standard_gate_noise(StandardGate.X, [0])
        sim.apply_standard_gate_noise(StandardGate.CX, [0, 1])
        probs = sim.probabilities()
        assert math.isclose(probs[3], 1.0, abs_tol=1e-10)

        # With noise
        noise_model = NoiseModel()
        noise = SingleQubitNoise.bit_flip(0.1)
        noise_model.add_single_qubit_error(StandardGate.X, 0, noise)
        sim = DensityMatrixNoise(1, noise_model)

        sim.apply_standard_gate_noise(StandardGate.X, [0])
        probs = sim.probabilities()
        assert math.isclose(probs[1], 0.9, abs_tol=1e-10)
        assert math.isclose(probs[0], 0.1, abs_tol=1e-10)

    def test_apply_x(self):
        """Test Pauli-X gate."""
        sim = DensityMatrixNoise(1)
        sim.apply_x(0)
        probs = sim.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_y(self):
        """Test Pauli-Y gate."""
        sim = DensityMatrixNoise(1)
        sim.apply_y(0)
        probs = sim.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_z(self):
        """Test Pauli-Z gate."""
        sim = DensityMatrixNoise(1)
        sim.apply_z(0)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 1.0, abs_tol=1e-10)

    def test_apply_h(self):
        """Test Hadamard gate."""
        sim = DensityMatrixNoise(1)
        sim.apply_h(0)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[1], 0.5, abs_tol=1e-10)

    def test_apply_s_and_sdg(self):
        """Test S and S-dagger gates."""
        sim = DensityMatrixNoise(1)
        sim.apply_h(0)
        sim.apply_s(0)
        sim.apply_sdg(0)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)

    def test_apply_t_and_tdg(self):
        """Test T and T-dagger gates."""
        sim = DensityMatrixNoise(1)
        sim.apply_h(0)
        sim.apply_t(0)
        sim.apply_tdg(0)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)

    def test_apply_rx(self):
        """Test RX rotation."""
        sim = DensityMatrixNoise(1)
        sim.apply_rx(0, np.pi)
        probs = sim.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_ry(self):
        """Test RY rotation."""
        sim = DensityMatrixNoise(1)
        sim.apply_ry(0, np.pi / 2)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[1], 0.5, abs_tol=1e-10)

    def test_apply_rz(self):
        """Test RZ rotation."""
        sim = DensityMatrixNoise(1)
        sim.apply_rz(0, np.pi / 4)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 1.0, abs_tol=1e-10)

    def test_apply_p(self):
        """Test phase gate."""
        sim = DensityMatrixNoise(1)
        sim.apply_phase(0, np.pi / 4)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 1.0, abs_tol=1e-10)

    def test_apply_gphase(self):
        """Test global phase gate."""
        sim = DensityMatrixNoise(1)
        sim.apply_gphase(np.pi / 4)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 1.0, abs_tol=1e-10)

    def test_apply_x2p_x2m(self):
        """Test X2P and X2M gates."""
        sim = DensityMatrixNoise(1)
        sim.apply_x2p(0)
        sim.apply_x2m(0)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 1.0, abs_tol=1e-10)

    def test_apply_y2p_y2m(self):
        """Test Y2P and Y2M gates."""
        sim = DensityMatrixNoise(1)
        sim.apply_y2p(0)
        sim.apply_y2m(0)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 1.0, abs_tol=1e-10)

    def test_apply_rxy(self):
        """Test RXY gate."""
        sim = DensityMatrixNoise(1)
        sim.apply_rxy(0, np.pi / 2, 0)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[1], 0.5, abs_tol=1e-10)

    def test_apply_u(self):
        """Test general U gate."""
        sim = DensityMatrixNoise(1)
        sim.apply_u(0, np.pi, 0, 0)
        probs = sim.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)


class TestDensityMatrixNoiseXYGates:
    """Test XY family of gates with noise."""

    def test_apply_xy(self):
        """Test XY gate."""
        sim = DensityMatrixNoise(1)
        sim.apply_xy(0, np.pi / 4)
        probs = sim.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_apply_xy2p(self):
        """Test XY2P gate."""
        sim = DensityMatrixNoise(1)
        sim.apply_xy2p(0, np.pi / 4)
        probs = sim.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_apply_xy2m(self):
        """Test XY2M gate."""
        sim = DensityMatrixNoise(1)
        sim.apply_xy2m(0, np.pi / 4)
        probs = sim.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10


class TestDensityMatrixNoiseTwoQubitGates:
    """Test two-qubit gate operations with noise."""

    def test_apply_cx(self):
        """Test CNOT gate."""
        sim = DensityMatrixNoise(2)
        sim.apply_h(0)
        sim.apply_cx(0, 1)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[3], 0.5, abs_tol=1e-10)
        # Verify coherence: Bell state has off-diagonal elements
        rho = sim.state
        assert math.isclose(abs(rho[0, 3]), 0.5, abs_tol=1e-10)
        assert math.isclose(abs(rho[3, 0]), 0.5, abs_tol=1e-10)

    def test_apply_cy(self):
        """Test CY gate."""
        sim = DensityMatrixNoise(2)
        sim.apply_h(0)
        sim.apply_cy(0, 1)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)

    def test_apply_cz(self):
        """Test CZ gate."""
        sim = DensityMatrixNoise(2)
        sim.apply_h(0)
        sim.apply_h(1)
        sim.apply_cz(0, 1)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.25, abs_tol=1e-10)

    def test_apply_swap(self):
        """Test SWAP gate."""
        sim = DensityMatrixNoise(2)
        sim.apply_x(1)  # |10> (qubit 1 is high-order bit)
        sim.apply_swap(0, 1)  # |01>
        probs = sim.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_apply_crx(self):
        """Test controlled-RX."""
        sim = DensityMatrixNoise(2)
        sim.apply_h(0)
        sim.apply_crx(0, 1, np.pi)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)

    def test_apply_cry(self):
        """Test controlled-RY."""
        sim = DensityMatrixNoise(2)
        sim.apply_h(0)
        sim.apply_cry(0, 1, np.pi / 2)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)

    def test_apply_crz(self):
        """Test controlled-RZ."""
        sim = DensityMatrixNoise(2)
        sim.apply_h(0)
        sim.apply_h(1)
        sim.apply_crz(0, 1, np.pi / 4)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.25, abs_tol=1e-10)

    def test_apply_rxx(self):
        """Test RXX gate."""
        sim = DensityMatrixNoise(2)
        sim.apply_rxx(0, 1, np.pi / 2)
        probs = sim.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_apply_ryy(self):
        """Test RYY gate."""
        sim = DensityMatrixNoise(2)
        sim.apply_ryy(0, 1, np.pi / 2)
        probs = sim.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_apply_rzz(self):
        """Test RZZ gate."""
        sim = DensityMatrixNoise(2)
        sim.apply_h(0)
        sim.apply_h(1)
        sim.apply_rzz(0, 1, np.pi)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.25, abs_tol=1e-10)

    def test_apply_rzx(self):
        """Test RZX gate."""
        sim = DensityMatrixNoise(2)
        sim.apply_h(0)
        sim.apply_rzx(0, 1, np.pi / 4)
        probs = sim.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10


class TestDensityMatrixNoiseThreeQubitGates:
    """Test three-qubit gate operations with noise."""

    def test_apply_ccx(self):
        """Test Toffoli (CCX) gate."""
        sim = DensityMatrixNoise(3)
        sim.apply_x(0)
        sim.apply_x(1)
        sim.apply_ccx(0, 1, 2)
        probs = sim.probabilities()
        assert math.isclose(probs[7], 1.0, abs_tol=1e-10)


class TestDensityMatrixNoiseFSim:
    """Test fSim gate with noise."""

    def test_apply_fsim(self):
        """Test Fermionic Simulation gate."""
        sim = DensityMatrixNoise(2)
        sim.apply_x(0)
        sim.apply_fsim(0, 1, np.pi / 4, np.pi / 2)
        probs = sim.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10


class TestDensityMatrixNoiseUnitary:
    """Test custom unitary application with noise."""

    def test_apply_unitary_gate(self):
        """Test arbitrary unitary gate."""
        sim = DensityMatrixNoise(2)
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
        sim.apply_unitary_gate([0, 1], h_cx)
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[3], 0.5, abs_tol=1e-10)
        # Verify coherence: Bell state has off-diagonal elements rho[0,3] and rho[3,0] = 0.5
        rho = sim.state
        assert math.isclose(abs(rho[0, 3]), 0.5, abs_tol=1e-10)
        assert math.isclose(abs(rho[3, 0]), 0.5, abs_tol=1e-10)
        # Verify this is NOT a mixed state (coherence exists)
        assert math.isclose(np.trace(rho @ rho).real, 1.0, abs_tol=1e-10)


class TestDensityMatrixNoiseExpectation:
    """Test expectation value calculations."""

    def test_expectation_pauli_string(self):
        """Test expectation with PauliString."""
        sim = DensityMatrixNoise(1)
        ps_z = PauliString.from_str("Z")
        assert math.isclose(sim.expectation(ps_z), 1.0, abs_tol=1e-10)

        sim.apply_h(0)
        assert math.isclose(sim.expectation(ps_z), 0.0, abs_tol=1e-10)

    def test_expectation_hamiltonian(self):
        """Test expectation with Hamiltonian."""
        sim = DensityMatrixNoise(2)
        sim.apply_h(0)
        sim.apply_cx(0, 1)

        h = Hamiltonian(2)
        h.add_term(PauliString.from_str("ZZ"), 0.5)
        h.add_term(PauliString.from_str("XX"), 0.5)
        h.simplify()

        exp = sim.expectation(h)
        assert math.isclose(exp, 1.0, abs_tol=1e-10)


class TestDensityMatrixNoiseWithNoiseModel:
    """Test operations with actual noise models."""

    def test_bit_flip_noise(self):
        """Test simulation with bit-flip noise."""
        from cqlib.circuit import StandardGate

        noise_model = NoiseModel()
        # Add bit-flip noise after X gates
        noise = SingleQubitNoise.bit_flip(p=0.1)
        noise_model.add_single_qubit_error(StandardGate.X, 0, noise)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_x(0)

        # With 10% bit-flip noise, P(|1>) should be ~0.9, P(|0>) should be ~0.1
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.1, abs_tol=0.01)
        assert math.isclose(probs[1], 0.9, abs_tol=0.01)
        # Verify probability sum
        assert math.isclose(sum(probs), 1.0, abs_tol=1e-10)

    def test_depolarizing_noise(self):
        """Test simulation with depolarizing noise."""

        noise_model = NoiseModel()
        # Add depolarizing noise after H gates
        p = 0.1
        noise = SingleQubitNoise.depolarizing(p=p)
        noise_model.add_single_qubit_error(StandardGate.H, 0, noise)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_h(0)

        # With depolarizing noise, state becomes mixed:
        # rho = (1-p) * |+><+| + p * I/2
        # For p=0.1: rho = 0.9 * |+><+| + 0.1 * I/2
        probs = sim.probabilities()
        # For |+> state with depolarizing:
        # P(|0>) = P(|1>) = 0.5 (depolarizing doesn't change diagonal in computational basis)
        assert math.isclose(probs[0], 0.5, abs_tol=0.01)
        assert math.isclose(probs[1], 0.5, abs_tol=0.01)
        # Verify probability sum
        assert math.isclose(sum(probs), 1.0, abs_tol=1e-10)

        # More importantly, verify off-diagonal elements decay
        rho = sim.state
        # Ideal |+> has off-diagonal = 0.5, with depolarizing it becomes (1-p)*0.5 = 0.45
        expected_off_diag = 0.5 * (1 - p)
        assert math.isclose(abs(rho[0, 1]), expected_off_diag, abs_tol=0.02)
        assert math.isclose(abs(rho[1, 0]), expected_off_diag, abs_tol=0.02)


class TestDensityMatrixNoiseCopy:
    """Test copy functionality."""

    def test_copy(self):
        """Test that copy creates independent instance."""
        sim1 = DensityMatrixNoise(1)
        sim1.apply_h(0)
        sim2 = sim1.copy()

        # Modify original
        sim1.apply_x(0)

        # Copy should be unchanged
        probs = sim2.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)

    def test_repr(self):
        """Test string representation."""
        sim = DensityMatrixNoise(2)
        repr_str = repr(sim)
        assert "DensityMatrixNoise" in repr_str


class TestDensityMatrixNoiseBoundaryConditions:
    """Test boundary conditions and edge cases."""

    def test_single_qubit_system(self):
        """Test minimum system with 1 qubit."""
        sim = DensityMatrixNoise(1)
        assert sim.num_qubits == 1
        assert sim.state.shape == (2, 2)
        probs = sim.probabilities()
        assert len(probs) == 2
        assert math.isclose(probs[0], 1.0, abs_tol=1e-10)

    def test_large_system_8_qubits(self):
        """Test large system with 8 qubits (256x256 matrix)."""
        sim = DensityMatrixNoise(8)
        assert sim.num_qubits == 8
        assert sim.state.shape == (256, 256)
        probs = sim.probabilities()
        assert len(probs) == 256
        assert math.isclose(probs[0], 1.0, abs_tol=1e-10)

    @pytest.mark.parametrize("angle", [0, 2 * np.pi, 4 * np.pi, -np.pi / 2])
    def test_extreme_rotation_angles(self, angle):
        """Test extreme rotation angles."""
        sim = DensityMatrixNoise(1)
        sim.apply_rx(0, angle)
        probs = sim.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_zero_noise_probability(self):
        """Test p=0 noise is equivalent to no noise."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.bit_flip(p=0.0)
        noise_model.add_single_qubit_error(StandardGate.X, 0, noise)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_x(0)

        probs = sim.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_maximum_bit_flip_noise(self):
        """Test p=1 bit-flip noise (complete flip)."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.bit_flip(p=1.0)
        noise_model.add_single_qubit_error(StandardGate.X, 0, noise)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_x(0)

        probs = sim.probabilities()
        assert math.isclose(probs[0], 1.0, abs_tol=1e-10)

    def test_amplitude_damping_extreme_gamma_zero(self):
        """Test amplitude damping with γ=0 (no damping)."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.amplitude_damping(gamma=0.0)
        noise_model.add_single_qubit_error(StandardGate.X, 0, noise)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_x(0)

        probs = sim.probabilities()
        assert math.isclose(probs[1], 1.0, abs_tol=1e-10)

    def test_amplitude_damping_extreme_gamma_one(self):
        """Test amplitude damping with γ=1 (complete damping to |0⟩)."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.amplitude_damping(gamma=1.0)
        noise_model.add_single_qubit_error(StandardGate.X, 0, noise)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_x(0)

        probs = sim.probabilities()
        assert math.isclose(probs[0], 1.0, abs_tol=1e-10)

    def test_phase_damping_extreme_lambda_zero(self):
        """Test phase damping with λ=0 (no dephasing)."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.phase_damping(lambda_=0.0)
        noise_model.add_single_qubit_error(StandardGate.H, 0, noise)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_h(0)

        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[1], 0.5, abs_tol=1e-10)


class TestDensityMatrixNoiseQuantumInvariants:
    """Test quantum invariants under noise."""

    def test_density_matrix_is_hermitian(self):
        """Test density matrix remains Hermitian under noise."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.depolarizing(p=0.1)
        noise_model.add_single_qubit_error(StandardGate.H, 0, noise)

        sim = DensityMatrixNoise(2, noise_model)
        sim.apply_h(0)
        sim.apply_cx(0, 1)

        rho = sim.state
        assert np.allclose(rho, rho.conj().T, atol=1e-10)

    def test_density_matrix_is_positive_semidefinite(self):
        """Test density matrix remains positive semidefinite."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.depolarizing(p=0.2)
        noise_model.add_single_qubit_error(StandardGate.H, 0, noise)

        sim = DensityMatrixNoise(2, noise_model)
        sim.apply_h(0)
        sim.apply_cx(0, 1)

        rho = sim.state
        eigenvalues = np.linalg.eigvalsh(rho)
        assert np.all(eigenvalues >= -1e-10)

    def test_trace_preservation_under_noise(self):
        """Test trace is preserved under noisy evolution."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.bit_flip(p=0.3)
        noise_model.add_single_qubit_error(StandardGate.X, 0, noise)
        noise_model.add_single_qubit_error(StandardGate.H, 0, noise)

        sim = DensityMatrixNoise(2, noise_model)
        sim.apply_h(0)
        sim.apply_cx(0, 1)
        sim.apply_x(0)

        rho = sim.state
        trace = np.trace(rho).real
        assert math.isclose(trace, 1.0, abs_tol=1e-10)

    def test_purity_decreases_with_depolarizing(self):
        """Test purity decreases with depolarizing noise."""
        # Pure state without noise
        sim_ideal = DensityMatrixNoise(1)
        sim_ideal.apply_h(0)
        rho_ideal = sim_ideal.state
        purity_ideal = np.trace(rho_ideal @ rho_ideal).real

        # With depolarizing noise
        noise_model = NoiseModel()
        noise = SingleQubitNoise.depolarizing(p=0.1)
        noise_model.add_single_qubit_error(StandardGate.H, 0, noise)

        sim_noisy = DensityMatrixNoise(1, noise_model)
        sim_noisy.apply_h(0)
        rho_noisy = sim_noisy.state
        purity_noisy = np.trace(rho_noisy @ rho_noisy).real

        assert purity_noisy < purity_ideal
        assert purity_ideal <= 1.0 + 1e-10
        assert purity_noisy >= 0.5 - 1e-10

    def test_amplitude_damping_to_ground_state(self):
        """Test amplitude damping drives system to |0⟩."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.amplitude_damping(gamma=0.5)
        noise_model.add_single_qubit_error(StandardGate.X, 0, noise)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_x(0)  # Start in |1⟩

        probs = sim.probabilities()
        # With amplitude damping, excited state decays
        assert probs[1] < 1.0
        assert probs[0] > 0.0

    def test_bell_state_decoherence_under_noise(self):
        """Test Bell state decoherence under bit-flip noise."""
        # Ideal Bell state
        sim_ideal = DensityMatrixNoise(2)
        sim_ideal.apply_h(0)
        sim_ideal.apply_cx(0, 1)

        # Reduced density matrix of qubit 0 should be I/2 (maximally mixed)
        rho_ideal = sim_ideal.state
        rho_0_ideal = np.trace(rho_ideal.reshape(2, 2, 2, 2), axis1=1, axis2=3)

        # With noise, off-diagonal elements decay
        noise_model = NoiseModel()
        noise = SingleQubitNoise.bit_flip(p=0.1)
        noise_model.add_single_qubit_error(StandardGate.H, 0, noise)

        sim_noisy = DensityMatrixNoise(2, noise_model)
        sim_noisy.apply_h(0)
        sim_noisy.apply_cx(0, 1)

        # Both reduced density matrices should be close to maximally mixed
        # (I/2 for Bell state, affected by noise)
        assert np.allclose(rho_0_ideal, np.eye(2) / 2, atol=1e-10)

    def test_combined_noise_channels(self):
        """Test combined single and two-qubit noise."""
        noise_model = NoiseModel()

        # Single-qubit noise
        sq_noise = SingleQubitNoise.depolarizing(p=0.05)
        noise_model.add_single_qubit_error(StandardGate.H, 0, sq_noise)

        # Two-qubit noise
        tq_noise = TwoQubitNoise.depolarizing(p=0.05)
        noise_model.add_two_qubit_error(StandardGate.CX, 0, 1, tq_noise)

        sim = DensityMatrixNoise(2, noise_model)
        sim.apply_h(0)
        sim.apply_cx(0, 1)

        probs = sim.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10


class TestDensityMatrixNoiseNumericalPrecision:
    """Test numerical precision and stability."""

    def test_repeated_hadamard_precision(self):
        """Test repeated Hadamard gates maintain precision."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.depolarizing(p=0.001)  # Very small noise
        noise_model.add_single_qubit_error(StandardGate.H, 0, noise)

        sim = DensityMatrixNoise(1, noise_model)

        # Apply H multiple times: H^4 = I (approximately with small noise)
        for _ in range(4):
            sim.apply_h(0)

        probs = sim.probabilities()
        # Should be close to |0⟩ with small deviation due to noise
        assert probs[0] > 0.99

    def test_small_angle_rotation_precision(self):
        """Test small angle rotations have correct effect."""
        sim = DensityMatrixNoise(1)
        sim.apply_rx(0, 0.001)

        probs = sim.probabilities()
        # For small θ, P(|1⟩) ≈ (θ/2)^2
        expected_p1 = (0.001 / 2) ** 2
        assert math.isclose(probs[1], expected_p1, rel_tol=0.1)

    def test_probability_sum_precision_with_noise(self):
        """Test probabilities sum to 1.0 with various noise."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.pauli(px=0.1, py=0.05, pz=0.05)
        noise_model.add_single_qubit_error(StandardGate.H, 0, noise)
        noise_model.add_single_qubit_error(StandardGate.X, 0, noise)

        sim = DensityMatrixNoise(2, noise_model)
        sim.apply_h(0)
        sim.apply_cx(0, 1)
        sim.apply_x(0)

        probs = sim.probabilities()
        assert math.isclose(sum(probs), 1.0, abs_tol=1e-10)

    def test_identity_under_depolarizing(self):
        """Test state evolves correctly under depolarizing."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.depolarizing(p=0.1)
        noise_model.add_single_qubit_error(StandardGate.X, 0, noise)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_x(0)
        sim.apply_x(0)  # X^2 = I

        # With depolarizing, state should be mixed
        rho = sim.state
        trace_rho2 = np.trace(rho @ rho).real
        # After depolarizing on both X gates, purity decreases
        assert trace_rho2 < 1.0


class TestDensityMatrixNoiseCopySemantics:
    """Test copy semantics and independence."""

    def test_copy_independence_state(self):
        """Test copy creates independent state."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.depolarizing(p=0.1)
        noise_model.add_single_qubit_error(StandardGate.RY, 0, noise)

        sim1 = DensityMatrixNoise(1, noise_model)
        sim1.apply_ry(0, np.pi / 4)

        sim2 = sim1.copy()

        # Modify sim1
        sim1.apply_ry(0, np.pi / 4)

        # sim2 should be unchanged
        probs1 = sim1.probabilities()
        probs2 = sim2.probabilities()

        assert not math.isclose(probs1[0], probs2[0], abs_tol=1e-6)

    def test_state_array_copy_semantics(self):
        """Test state property returns array that can be modified safely."""
        sim = DensityMatrixNoise(1)
        sim.apply_h(0)

        state1 = sim.state
        state1_copy = state1.copy()

        # Modify the returned array
        state1[0, 0] = 999.0

        # Get state again - should be unchanged
        state2 = sim.state
        assert np.allclose(state2, state1_copy)

    def test_deep_copy_with_noise_model(self):
        """Test deep copy preserves noise model."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.bit_flip(p=0.1)
        noise_model.add_single_qubit_error(StandardGate.X, 0, noise)

        sim1 = DensityMatrixNoise(1, noise_model)
        sim1.apply_x(0)

        sim2 = sim1.copy()

        # Both should have same noisy behavior
        # Apply X to both: sim1 with noise, sim2 with same noise
        probs1 = sim1.probabilities()
        probs2 = sim2.probabilities()

        assert math.isclose(probs1[0], probs2[0], abs_tol=1e-10)
        assert math.isclose(probs1[1], probs2[1], abs_tol=1e-10)


class TestDensityMatrixNoiseAdvancedNoise:
    """Test advanced noise channel types."""

    def test_phase_flip_noise(self):
        """Test phase-flip noise channel."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.phase_flip(p=0.5)
        noise_model.add_single_qubit_error(StandardGate.H, 0, noise)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_h(0)

        # Phase flip in Z basis doesn't affect diagonal elements
        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=0.1)
        assert math.isclose(probs[1], 0.5, abs_tol=0.1)

    def test_pauli_noise_all_channels(self):
        """Test general Pauli noise with all error types."""
        noise_model = NoiseModel()
        noise = SingleQubitNoise.pauli(px=0.1, py=0.05, pz=0.05)
        noise_model.add_single_qubit_error(StandardGate.H, 0, noise)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_h(0)

        probs = sim.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_two_qubit_depolarizing_noise(self):
        """Test two-qubit depolarizing noise."""
        noise_model = NoiseModel()
        p = 0.1
        noise = TwoQubitNoise.depolarizing(p=p)
        noise_model.add_two_qubit_error(StandardGate.CX, 0, 1, noise)

        sim = DensityMatrixNoise(2, noise_model)
        sim.apply_h(0)
        sim.apply_cx(0, 1)

        probs = sim.probabilities()
        # Two-qubit depolarizing: rho' = (1-p) * rho + p * I/4
        # For Bell state: rho = |Φ+><Φ+|
        # After depolarizing: P(|00>) = P(|11>) = (1-p)*0.5 + p*0.25 = 0.45 + 0.025 = 0.475
        #                      P(|01>) = P(|10>) = p*0.25 = 0.025
        expected_p00 = (1 - p) * 0.5 + p * 0.25
        expected_p01 = p * 0.25

        assert math.isclose(probs[0], expected_p00, abs_tol=0.02)
        assert math.isclose(probs[3], expected_p00, abs_tol=0.02)
        assert math.isclose(probs[1], expected_p01, abs_tol=0.02)
        assert math.isclose(probs[2], expected_p01, abs_tol=0.02)
        assert math.isclose(sum(probs), 1.0, abs_tol=1e-10)

        # Verify off-diagonal coherence elements decay
        rho = sim.state
        # Ideal Bell state has |rho[0,3]| = 0.5, with depolarizing: (1-p)*0.5
        expected_coherence = 0.5 * (1 - p)
        assert math.isclose(abs(rho[0, 3]), expected_coherence, abs_tol=0.02)
        assert math.isclose(abs(rho[3, 0]), expected_coherence, abs_tol=0.02)

    def test_correlated_pauli_noise(self):
        """Test correlated Pauli noise."""
        noise_model = NoiseModel()
        noise = TwoQubitNoise.correlated_pauli(Pauli.x(), Pauli.x(), p=0.5)
        noise_model.add_two_qubit_error(StandardGate.CX, 0, 1, noise)

        sim = DensityMatrixNoise(2, noise_model)
        sim.apply_h(0)
        sim.apply_cx(0, 1)

        probs = sim.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_asymmetric_readout_error(self):
        """Test asymmetric readout error."""
        noise_model = NoiseModel()
        p_0_given_1 = 0.1  # Probability of measuring 0 when actually in |1⟩
        p_1_given_0 = 0.2  # Probability of measuring 1 when actually in |0⟩
        readout_error = ReadoutError(p_0_given_1=p_0_given_1, p_1_given_0=p_1_given_0)
        noise_model.add_readout_error(0, readout_error)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_x(0)  # Prepare |1⟩

        # Actual probabilities should be P(|1⟩) = 1.0
        probs_actual = sim.probabilities()
        assert math.isclose(probs_actual[0], 0.0, abs_tol=1e-10)
        assert math.isclose(probs_actual[1], 1.0, abs_tol=1e-10)

        # With readout error: we prepared |1⟩, so P(measured 0) = p_0_given_1, P(measured 1) = 1 - p_0_given_1
        probs_with_readout = sim.probabilities_with_readout([0])
        assert math.isclose(probs_with_readout[0], p_0_given_1, abs_tol=1e-10)
        assert math.isclose(probs_with_readout[1], 1.0 - p_0_given_1, abs_tol=1e-10)
        assert math.isclose(sum(probs_with_readout), 1.0, abs_tol=1e-10)

        # Test with |0⟩ preparation
        sim2 = DensityMatrixNoise(1, noise_model)
        # Stay in |0⟩
        probs_with_readout_0 = sim2.probabilities_with_readout([0])
        # P(measured 1) = p_1_given_0, P(measured 0) = 1 - p_1_given_0
        assert math.isclose(probs_with_readout_0[1], p_1_given_0, abs_tol=1e-10)
        assert math.isclose(probs_with_readout_0[0], 1.0 - p_1_given_0, abs_tol=1e-10)

    def test_multiple_qubits_different_noise(self):
        """Test different noise on different qubits."""
        noise_model = NoiseModel()

        # Qubit 0: bit-flip
        bf_noise = SingleQubitNoise.bit_flip(p=0.1)
        noise_model.add_single_qubit_error(StandardGate.X, 0, bf_noise)

        # Qubit 1: phase-flip
        pf_noise = SingleQubitNoise.phase_flip(p=0.1)
        noise_model.add_single_qubit_error(StandardGate.H, 1, pf_noise)

        sim = DensityMatrixNoise(2, noise_model)
        sim.apply_x(0)
        sim.apply_h(1)

        probs = sim.probabilities()
        assert abs(sum(probs) - 1.0) < 1e-10

    def test_noise_model_gate_specific(self):
        """Test noise model with gate-specific errors."""
        noise_model = NoiseModel()

        # Only H gates on qubit 0 have noise
        noise = SingleQubitNoise.depolarizing(p=0.1)
        noise_model.add_single_qubit_error(StandardGate.H, 0, noise)

        sim = DensityMatrixNoise(1, noise_model)

        # Apply X (no noise)
        sim.apply_x(0)
        # Note: probs_after_x would be [0.0, 1.0] (pure |1>)

        # Apply H (with noise)
        sim.apply_h(0)
        probs_after_h = sim.probabilities()

        # Results should differ from ideal due to noise
        assert abs(sum(probs_after_h) - 1.0) < 1e-10

    def test_no_noise_without_noise_model(self):
        """Test that operations without noise model are ideal."""
        sim = DensityMatrixNoise(1)  # No noise model
        sim.apply_h(0)

        probs = sim.probabilities()
        assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
        assert math.isclose(probs[1], 0.5, abs_tol=1e-10)


class TestDensityMatrixNoiseErrorHandling:
    """Test error handling and invalid inputs."""

    def test_qubit_out_of_bounds_single_qubit_gate(self):
        """Test qubit index out of bounds for single-qubit gates raises IndexError."""
        sim = DensityMatrixNoise(2)

        with pytest.raises(IndexError, match="out of bounds"):
            sim.apply_x(2)  # Qubit 2 doesn't exist (valid: 0, 1)

    def test_qubit_out_of_bounds_two_qubit_gate(self):
        """Test qubit index out of bounds for two-qubit gates raises IndexError."""
        sim = DensityMatrixNoise(2)

        with pytest.raises(IndexError, match="out of bounds"):
            sim.apply_cx(0, 2)  # Target qubit 2 doesn't exist

    def test_unitary_dimension_mismatch_single_qubit(self):
        """Test unitary matrix dimension mismatch for single qubit."""
        sim = DensityMatrixNoise(2)

        # 4x4 matrix applied to single qubit should fail
        wrong_unitary = np.eye(4, dtype=complex)
        with pytest.raises((ValueError, RuntimeError)):
            sim.apply_unitary_gate([0], wrong_unitary)

    def test_unitary_dimension_mismatch_two_qubit(self):
        """Test unitary matrix dimension mismatch for two qubits."""
        sim = DensityMatrixNoise(3)

        # 2x2 matrix applied to two qubits should fail
        wrong_unitary = np.eye(2, dtype=complex)
        with pytest.raises((ValueError, RuntimeError)):
            sim.apply_unitary_gate([0, 1], wrong_unitary)

        # 8x8 matrix applied to two qubits should fail
        wrong_unitary = np.eye(8, dtype=complex)
        with pytest.raises((ValueError, RuntimeError)):
            sim.apply_unitary_gate([0, 1], wrong_unitary)

    def test_unitary_not_square(self):
        """Test non-square unitary matrix."""
        sim = DensityMatrixNoise(1)

        # Non-square matrix
        non_square = np.array([[1, 0, 0], [0, 1, 0]], dtype=complex)
        with pytest.raises((ValueError, RuntimeError)):
            sim.apply_unitary_gate([0], non_square)

    def test_invalid_num_qubits_negative(self):
        """Test invalid number of qubits (negative)."""
        with pytest.raises((ValueError, RuntimeError, OverflowError)):
            DensityMatrixNoise(-1)


class TestDensityMatrixNoiseCoherenceVerification:
    """Test coherence (off-diagonal elements) preservation and decay."""

    def test_bell_state_coherence(self):
        """Test Bell state has correct off-diagonal coherence elements."""
        sim = DensityMatrixNoise(2)
        sim.apply_h(0)
        sim.apply_cx(0, 1)

        rho = sim.state
        # For Bell state |Φ+> = (|00> + |11>)/√2:
        # rho = 0.5 * (|00><00| + |00><11| + |11><00| + |11><11|)
        # rho[0,0] = rho[3,3] = 0.5 (diagonal)
        assert math.isclose(rho[0, 0].real, 0.5, abs_tol=1e-10)
        assert math.isclose(rho[3, 3].real, 0.5, abs_tol=1e-10)
        # rho[0,3] = rho[3,0] = 0.5 (coherence)
        assert math.isclose(rho[0, 3].real, 0.5, abs_tol=1e-10)
        assert math.isclose(rho[3, 0].real, 0.5, abs_tol=1e-10)
        # All other elements should be 0
        assert math.isclose(abs(rho[0, 1]), 0.0, abs_tol=1e-10)
        assert math.isclose(abs(rho[0, 2]), 0.0, abs_tol=1e-10)
        assert math.isclose(abs(rho[1, 0]), 0.0, abs_tol=1e-10)
        assert math.isclose(abs(rho[1, 2]), 0.0, abs_tol=1e-10)
        assert math.isclose(abs(rho[1, 3]), 0.0, abs_tol=1e-10)
        assert math.isclose(abs(rho[2, 0]), 0.0, abs_tol=1e-10)
        assert math.isclose(abs(rho[2, 1]), 0.0, abs_tol=1e-10)
        assert math.isclose(abs(rho[2, 3]), 0.0, abs_tol=1e-10)

        # Purity should be 1 (pure state)
        purity = np.trace(rho @ rho).real
        assert math.isclose(purity, 1.0, abs_tol=1e-10)

    def test_plus_state_coherence(self):
        """Test |+> state has correct coherence elements."""
        sim = DensityMatrixNoise(1)
        sim.apply_h(0)

        rho = sim.state
        # For |+> = (|0> + |1>)/√2:
        # rho = 0.5 * (|0><0| + |0><1| + |1><0| + |1><1|)
        assert math.isclose(rho[0, 0].real, 0.5, abs_tol=1e-10)
        assert math.isclose(rho[1, 1].real, 0.5, abs_tol=1e-10)
        assert math.isclose(rho[0, 1].real, 0.5, abs_tol=1e-10)
        assert math.isclose(rho[1, 0].real, 0.5, abs_tol=1e-10)

    def test_phase_flip_affects_coherence_not_population(self):
        """Test phase flip noise destroys coherence but preserves populations."""
        noise_model = NoiseModel()
        p = 0.5
        noise = SingleQubitNoise.phase_flip(p=p)
        noise_model.add_single_qubit_error(StandardGate.H, 0, noise)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_h(0)

        rho = sim.state
        # Phase flip in computational basis doesn't affect diagonal
        # But destroys off-diagonal elements: rho[0,1] = (1-2p) * 0.5
        assert math.isclose(rho[0, 0].real, 0.5, abs_tol=0.1)
        assert math.isclose(rho[1, 1].real, 0.5, abs_tol=0.1)

        # Off-diagonal elements should decay
        expected_coherence = 0.5 * (1 - 2 * p)
        assert math.isclose(abs(rho[0, 1]), abs(expected_coherence), abs_tol=0.1)

    def test_cnot_preserves_bell_state_coherence(self):
        """Test CNOT preserves coherence in Bell state preparation."""
        sim = DensityMatrixNoise(2)

        # Step by step Bell state preparation
        sim.apply_h(0)
        # After H on |00>: |+0> = (|00> + |01>)/√2 (little-endian indexing)
        # State is |0>(|0>+|1>)/√2 = (|00> + |01>)/√2
        # Index: |00>=0, |01>=1, |10>=2, |11>=3
        rho_after_h = sim.state.copy()
        # This is a product state, not entangled yet
        assert math.isclose(rho_after_h[0, 0].real, 0.5, abs_tol=1e-10)
        assert math.isclose(rho_after_h[1, 1].real, 0.5, abs_tol=1e-10)
        # Verify coherence between |00> and |01>
        assert math.isclose(rho_after_h[0, 1].real, 0.5, abs_tol=1e-10)

        sim.apply_cx(0, 1)
        # After CNOT: Bell state |Φ+> = (|00> + |11>)/√2
        rho_after_cx = sim.state

        # Now we have coherence between |00> and |11>
        assert math.isclose(rho_after_cx[0, 3].real, 0.5, abs_tol=1e-10)
        assert math.isclose(rho_after_cx[3, 0].real, 0.5, abs_tol=1e-10)
        # Diagonal elements
        assert math.isclose(rho_after_cx[0, 0].real, 0.5, abs_tol=1e-10)
        assert math.isclose(rho_after_cx[3, 3].real, 0.5, abs_tol=1e-10)
        # But no coherence between other states
        assert math.isclose(abs(rho_after_cx[0, 1]), 0.0, abs_tol=1e-10)
        assert math.isclose(abs(rho_after_cx[0, 2]), 0.0, abs_tol=1e-10)
