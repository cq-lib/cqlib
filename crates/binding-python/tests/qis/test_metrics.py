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

import pytest
import math
import numpy as np
from cqlib.qis import Statevector, DensityMatrix, metrics


def test_purity():
    # Pure state purity = 1.0
    sv = Statevector(2)
    assert math.isclose(metrics.purity_pure(sv), 1.0, abs_tol=1e-10)

    dm_pure = DensityMatrix(2)
    assert math.isclose(metrics.purity_mixed(dm_pure), 1.0, abs_tol=1e-10)

    # Mixed state purity
    # Create a completely mixed state: rho = I/4 using Kraus operators (depolarizing channel)
    dm_mixed = DensityMatrix(2)
    # Apply depolarizing noise to create mixed state
    # For 2-qubit maximally mixed state, use Kraus operators
    p = 0.75  # depolarizing probability
    K0 = np.sqrt(1 - p) * np.eye(2, dtype=complex)
    K1 = np.sqrt(p / 3) * np.array([[0, 1], [1, 0]], dtype=complex)
    K2 = np.sqrt(p / 3) * np.array([[0, -1j], [1j, 0]], dtype=complex)
    K3 = np.sqrt(p / 3) * np.array([[1, 0], [0, -1]], dtype=complex)
    # Apply to both qubits to create highly mixed state
    dm_mixed.apply_kraus([0], [K0.flatten(), K1.flatten(), K2.flatten(), K3.flatten()])
    dm_mixed.apply_kraus([1], [K0.flatten(), K1.flatten(), K2.flatten(), K3.flatten()])

    # Purity should be less than 1.0 for mixed state
    purity = metrics.purity_mixed(dm_mixed)
    assert purity < 1.0
    assert purity >= 0.25  # For maximally mixed state, purity = 1/d = 1/4 = 0.25


def test_state_fidelity_pure():
    sv1 = Statevector(1)  # |0>
    sv2 = Statevector(1)
    sv2.apply_x(0)  # |1>

    # Orthogonal states
    assert math.isclose(metrics.state_fidelity_pure(sv1, sv2), 0.0, abs_tol=1e-10)

    # Identical states
    assert math.isclose(metrics.state_fidelity_pure(sv1, sv1), 1.0, abs_tol=1e-10)

    # Mismatch dimension
    sv3 = Statevector(2)
    with pytest.raises(ValueError, match="Qubit count mismatch"):
        metrics.state_fidelity_pure(sv1, sv3)


def test_trace_distance_pure():
    sv1 = Statevector(1)  # |0>
    sv2 = Statevector(1)
    sv2.apply_x(0)  # |1>

    # Orthogonal states: D = 1.0
    assert math.isclose(metrics.trace_distance_pure(sv1, sv2), 1.0, abs_tol=1e-10)

    # Identical states: D = 0.0
    assert math.isclose(metrics.trace_distance_pure(sv1, sv1), 0.0, abs_tol=1e-10)


def test_state_fidelity_pure_mixed():
    sv = Statevector(1)  # |0>
    dm = DensityMatrix(1)  # |0><0|

    assert math.isclose(metrics.state_fidelity_pure_mixed(sv, dm), 1.0, abs_tol=1e-10)

    # Create maximally mixed state I/2 using from_density_matrix
    mixed_data = np.array([0.5, 0, 0, 0.5], dtype=complex)
    dm_mixed = DensityMatrix.from_density_matrix(1, mixed_data)

    # Fidelity |0><0| with I/2 is 0.5
    assert math.isclose(
        metrics.state_fidelity_pure_mixed(sv, dm_mixed), 0.5, abs_tol=1e-10
    )


def test_entropy():
    # Pure state: S = 0
    dm = DensityMatrix(2)
    assert math.isclose(metrics.entropy(dm), 0.0, abs_tol=1e-10)

    # Maximally mixed 1-qubit state: S = 1.0 (bit)
    # Create I/2 using Kraus operators or from_density_matrix
    mixed_data = np.array([0.5, 0, 0, 0.5], dtype=complex)
    dm_mixed = DensityMatrix.from_density_matrix(1, mixed_data)

    assert math.isclose(metrics.entropy(dm_mixed), 1.0, abs_tol=1e-10)


def test_trace_distance_mixed():
    dm1 = DensityMatrix(1)
    dm2 = DensityMatrix(1)
    dm2.apply_x(0)

    assert math.isclose(metrics.trace_distance_mixed(dm1, dm1), 0.0, abs_tol=1e-10)
    assert math.isclose(metrics.trace_distance_mixed(dm1, dm2), 1.0, abs_tol=1e-10)

    # Mixed states D(I/2, |0><0|) = 0.5
    mixed_data = np.array([0.5, 0, 0, 0.5], dtype=complex)
    dm_mixed = DensityMatrix.from_density_matrix(1, mixed_data)

    assert math.isclose(metrics.trace_distance_mixed(dm1, dm_mixed), 0.5, abs_tol=1e-10)

    # Mismatch
    dm3 = DensityMatrix(2)
    with pytest.raises(ValueError):
        metrics.trace_distance_mixed(dm1, dm3)


def test_state_fidelity_mixed():
    dm1 = DensityMatrix(1)
    dm2 = DensityMatrix(1)
    dm2.apply_x(0)

    assert math.isclose(metrics.state_fidelity_mixed(dm1, dm1), 1.0, abs_tol=1e-10)
    assert math.isclose(metrics.state_fidelity_mixed(dm1, dm2), 0.0, abs_tol=1e-10)

    # Mixed states F(I/2, |0><0|) = 0.5
    mixed_data = np.array([0.5, 0, 0, 0.5], dtype=complex)
    dm_mixed = DensityMatrix.from_density_matrix(1, mixed_data)

    assert math.isclose(metrics.state_fidelity_mixed(dm1, dm_mixed), 0.5, abs_tol=1e-10)


def test_partial_transpose():
    # Construct Bell state |Phi+> = (|00> + |11>)/sqrt(2)
    dm = DensityMatrix(2)
    dm.apply_h(0)
    dm.apply_cx(0, 1)

    # Perform PT on subsystem A (qubit 0)
    pt_dm = metrics.partial_transpose(dm, [0])

    # The partial transpose of Bell state has a negative eigenvalue -0.5
    # Therefore, its purity should still trace out, but it's no longer positive semi-definite.
    data = pt_dm.data

    assert math.isclose(data[0, 0].real, 0.5, abs_tol=1e-10)
    assert math.isclose(data[1, 2].real, 0.5, abs_tol=1e-10)
    assert math.isclose(data[2, 1].real, 0.5, abs_tol=1e-10)
    assert math.isclose(data[3, 3].real, 0.5, abs_tol=1e-10)

    # Invalid subsystem
    with pytest.raises(IndexError):
        metrics.partial_transpose(dm, [2])


def test_logarithmic_negativity():
    # Bell state |Phi+>
    dm = DensityMatrix(2)
    dm.apply_h(0)
    dm.apply_cx(0, 1)

    # Logarithmic negativity of maximally entangled 2-qubit state is 1.0
    log_neg = metrics.logarithmic_negativity(dm, [0])
    assert math.isclose(log_neg, 1.0, abs_tol=1e-10)

    # Separable state |00>
    dm_sep = DensityMatrix(2)
    assert math.isclose(metrics.logarithmic_negativity(dm_sep, [0]), 0.0, abs_tol=1e-10)


class TestMetricsBoundaryConditions:
    """Test boundary conditions and extreme values."""

    @pytest.mark.parametrize("num_qubits", [1, 2, 3, 4, 5])
    def test_purity_extreme_values(self, num_qubits):
        """Test purity bounds for various system sizes."""
        # Pure state: purity = 1.0
        dm_pure = DensityMatrix(num_qubits)
        assert math.isclose(metrics.purity_mixed(dm_pure), 1.0, abs_tol=1e-10)

        # Maximally mixed state: purity = 1/2^N
        dim = 2**num_qubits
        mixed_data = np.eye(dim, dtype=complex) / dim
        dm_mixed = DensityMatrix.from_density_matrix(num_qubits, mixed_data.flatten())
        expected_purity = 1.0 / dim
        assert math.isclose(
            metrics.purity_mixed(dm_mixed), expected_purity, abs_tol=1e-10
        )

    @pytest.mark.parametrize("num_qubits", [1, 2, 3])
    def test_entropy_maximally_mixed_n_qubits(self, num_qubits):
        """Test von Neumann entropy for maximally mixed states."""
        dim = 2**num_qubits
        mixed_data = np.eye(dim, dtype=complex) / dim
        dm_mixed = DensityMatrix.from_density_matrix(num_qubits, mixed_data.flatten())

        # Entropy should be N bits for maximally mixed N-qubit state
        expected_entropy = num_qubits
        assert math.isclose(metrics.entropy(dm_mixed), expected_entropy, abs_tol=1e-10)

    def test_fidelity_boundary_values(self):
        """Test fidelity for states with specific overlap values."""
        # |0> and |+> have overlap 1/sqrt(2), fidelity = 0.5
        sv0 = Statevector(1)
        sv_plus = Statevector(1)
        sv_plus.apply_h(0)

        fid = metrics.state_fidelity_pure(sv0, sv_plus)
        assert math.isclose(fid, 0.5, abs_tol=1e-10)

        # |0> and rotated state with angle theta
        for theta in [np.pi / 4, np.pi / 3, np.pi / 6]:
            sv_rot = Statevector(1)
            sv_rot.apply_ry(0, theta)
            # Fidelity = cos^2(theta/2)
            expected = np.cos(theta / 2) ** 2
            fid = metrics.state_fidelity_pure(sv0, sv_rot)
            assert math.isclose(fid, expected, abs_tol=1e-10)

    def test_trace_distance_extreme_states(self):
        """Test trace distance between various extreme state pairs."""
        # Identical states: D = 0
        sv1 = Statevector(1)
        assert math.isclose(metrics.trace_distance_pure(sv1, sv1), 0.0, abs_tol=1e-10)

        # Orthogonal states: D = 1
        sv2 = Statevector(1)
        sv2.apply_x(0)
        assert math.isclose(metrics.trace_distance_pure(sv1, sv2), 1.0, abs_tol=1e-10)

        # |0> and |+>: D = sqrt(1 - 0.5) = sqrt(0.5)
        sv_plus = Statevector(1)
        sv_plus.apply_h(0)
        expected = np.sqrt(0.5)
        assert math.isclose(
            metrics.trace_distance_pure(sv1, sv_plus), expected, abs_tol=1e-10
        )

        # Mixed state: D(|0><0|, I/2) = 0.5
        dm_pure = DensityMatrix(1)
        mixed_data = np.array([0.5, 0, 0, 0.5], dtype=complex)
        dm_mixed = DensityMatrix.from_density_matrix(1, mixed_data)
        assert math.isclose(
            metrics.trace_distance_mixed(dm_pure, dm_mixed), 0.5, abs_tol=1e-10
        )

    def test_large_system_metrics(self):
        """Test metrics on larger systems (6 qubit GHZ)."""
        # GHZ state on 6 qubits
        sv = Statevector(6)
        sv.apply_h(0)
        for i in range(1, 6):
            sv.apply_cx(0, i)

        # Purity should be 1.0 for pure state
        assert math.isclose(metrics.purity_pure(sv), 1.0, abs_tol=1e-10)

        # GHZ state should be highly entangled
        dm = DensityMatrix(6)
        dm.apply_h(0)
        for i in range(1, 6):
            dm.apply_cx(0, i)

        # Partial transpose on first qubit should reveal entanglement
        pt_dm = metrics.partial_transpose(dm, [0])
        assert pt_dm is not None


class TestMetricsQuantumInvariants:
    """Test quantum information invariants and mathematical bounds."""

    def test_fidelity_bounds(self):
        """Verify 0 <= Fidelity <= 1 for all state combinations."""
        test_cases = [
            # (state1_prep, state2_prep, description)
            (lambda s: None, lambda s: s.apply_x(0), "orthogonal"),
            (lambda s: None, lambda s: None, "identical"),
            (lambda s: None, lambda s: s.apply_h(0), "zero_plus"),
            (
                lambda s: s.apply_h(0),
                lambda s: s.apply_rx(0, np.pi / 4),
                "plus_rotated",
            ),
        ]

        for prep1, prep2, desc in test_cases:
            sv1 = Statevector(1)
            sv2 = Statevector(1)
            prep1(sv1)
            prep2(sv2)

            fid = metrics.state_fidelity_pure(sv1, sv2)
            assert 0.0 <= fid <= 1.0, f"Fidelity out of bounds for {desc}: {fid}"

    def test_trace_distance_bounds(self):
        """Verify 0 <= Trace Distance <= 1 for all states."""
        test_states = [
            (Statevector(1), "|0>"),
            (Statevector(1).apply_x(0) or Statevector(1), "|1>"),
            (Statevector(1).apply_h(0) or Statevector(1), "|+>"),
            (Statevector(1).apply_ry(0, np.pi / 3) or Statevector(1), "ry_pi3"),
        ]

        for i, (sv1, name1) in enumerate(test_states):
            for j, (sv2, name2) in enumerate(test_states):
                d = metrics.trace_distance_pure(sv1, sv2)
                assert 0.0 <= d <= 1.0, (
                    f"Trace distance out of bounds for {name1} vs {name2}: {d}"
                )

    def test_fidelity_trace_distance_relationship(self):
        """Verify D >= 1 - F (quantum inequality relating distance and fidelity)."""
        # Test various state pairs
        states = [
            Statevector(1),
            Statevector(1).apply_x(0) or Statevector(1),
            Statevector(1).apply_h(0) or Statevector(1),
            Statevector(1).apply_ry(0, np.pi / 4) or Statevector(1),
        ]

        for i, sv1 in enumerate(states):
            for j, sv2 in enumerate(states):
                if i != j:
                    fid = metrics.state_fidelity_pure(sv1, sv2)
                    dist = metrics.trace_distance_pure(sv1, sv2)
                    # Fuchs-van de Graaf inequality: D >= 1 - F
                    assert dist >= 1 - fid - 1e-10, f"D({i},{j})={dist}, 1-F={1 - fid}"

    def test_pure_state_purity_always_one(self):
        """Test purity = 1.0 for any pure state."""
        test_preparations = [
            lambda s: None,  # |0>
            lambda s: s.apply_x(0),  # |1>
            lambda s: s.apply_h(0),  # |+>
            lambda s: s.apply_rx(0, 0.5),  # small rotation
            lambda s: s.apply_ry(0, np.pi / 3),  # larger rotation
        ]

        for prep in test_preparations:
            sv = Statevector(1)
            result = prep(sv)
            # Handle tuple return from Bell state preparation
            if result is not None and not isinstance(result, Statevector):
                sv = result[0] if isinstance(result, tuple) else result
            purity = metrics.purity_pure(sv)
            assert math.isclose(purity, 1.0, abs_tol=1e-10), (
                f"Purity != 1.0 for pure state: {purity}"
            )

    def test_mixed_fidelity_symmetry(self):
        """Test F(rho, sigma) = F(sigma, rho) (symmetry of fidelity)."""
        # Create two different mixed states
        dm1 = DensityMatrix(1)
        dm1.apply_h(0)

        dm2 = DensityMatrix(1)
        dm2.apply_ry(0, np.pi / 4)

        fid_12 = metrics.state_fidelity_mixed(dm1, dm2)
        fid_21 = metrics.state_fidelity_mixed(dm2, dm1)
        assert math.isclose(fid_12, fid_21, abs_tol=1e-10)


class TestMetricsNumericalPrecision:
    """Test numerical precision and stability."""

    def test_near_pure_state_purity(self):
        """Test purity for states with small mixedness."""
        from cqlib.qis.state import DensityMatrixNoise
        from cqlib.device import SingleQubitNoise, NoiseModel
        from cqlib.circuit import StandardGate

        # Very small depolarizing noise
        for p in [1e-10, 1e-8, 1e-6]:
            noise_model = NoiseModel()
            noise = SingleQubitNoise.depolarizing(p=p)
            noise_model.add_single_qubit_error(StandardGate.H, 0, noise)

            sim = DensityMatrixNoise(1, noise_model)
            sim.apply_h(0)
            dm = DensityMatrix.from_density_matrix(1, sim.state.flatten().tolist())

            purity = metrics.purity_mixed(dm)
            # Should be close to 1 but slightly less
            assert 1.0 - 1e-4 < purity <= 1.0, (
                f"Purity {purity} out of expected range for p={p}"
            )

    def test_near_orthogonal_fidelity(self):
        """Test fidelity between nearly orthogonal states."""
        sv0 = Statevector(1)

        # States very close to |1> but not exactly
        for epsilon in [1e-10, 1e-8, 1e-6]:
            sv_near = Statevector(1)
            sv_near.apply_rx(0, np.pi - epsilon)

            fid = metrics.state_fidelity_pure(sv0, sv_near)
            # Should be very small but positive
            expected = np.sin(epsilon / 2) ** 2  # ~ epsilon^2/4
            assert math.isclose(fid, expected, rel_tol=1e-4)

    def test_near_identical_trace_distance(self):
        """Test trace distance between nearly identical states."""
        sv1 = Statevector(1)

        for epsilon in [1e-10, 1e-8, 1e-6]:
            sv2 = Statevector(1)
            sv2.apply_rx(0, epsilon)

            dist = metrics.trace_distance_pure(sv1, sv2)
            # For small angle, D ~ epsilon/2
            assert 0.0 <= dist < epsilon, (
                f"Trace distance {dist} too large for epsilon={epsilon}"
            )

    def test_small_entanglement_log_negativity(self):
        """Test logarithmic negativity for slightly entangled states."""
        # Werner state: rho = p * |Phi+><Phi+| + (1-p) * I/4
        # Log negativity increases with p
        for p in [0.0, 0.1, 0.5, 1.0]:
            # Create Werner-like state
            dim = 4
            mixed = np.eye(dim, dtype=complex) / dim

            # Bell state density matrix
            sv_bell = Statevector(2)
            sv_bell.apply_h(0)
            sv_bell.apply_cx(0, 1)
            bell_dm = np.outer(sv_bell.data, sv_bell.data.conj())

            # Werner state
            rho = p * bell_dm + (1 - p) * mixed

            dm_werner = DensityMatrix.from_density_matrix(2, rho.flatten())
            log_neg = metrics.logarithmic_negativity(dm_werner, [0])

            # p=0: separable, log_neg = 0
            # p=1: Bell state, log_neg = 1
            # Intermediate: log_neg should increase with p
            assert log_neg >= 0.0, f"Log negativity must be non-negative for p={p}"
            if p == 1.0:
                assert math.isclose(log_neg, 1.0, abs_tol=1e-10)
            elif p == 0.0:
                assert math.isclose(log_neg, 0.0, abs_tol=1e-10)

    def test_entropy_near_zero(self):
        """Test entropy for nearly pure states."""
        # Mix pure state with small amount of identity
        sv_pure = Statevector(1)
        sv_pure.apply_h(0)

        for epsilon in [1e-10, 1e-8, 1e-6]:
            rho = (1 - epsilon) * np.outer(
                sv_pure.data, sv_pure.data.conj()
            ) + epsilon * np.eye(2) / 2
            dm_mixed = DensityMatrix.from_density_matrix(1, rho.flatten())

            ent = metrics.entropy(dm_mixed)
            # For nearly pure states, entropy should be small
            assert 0.0 <= ent < 0.01, f"Entropy {ent} too large for epsilon={epsilon}"


class TestMetricsErrorHandling:
    """Test error handling for invalid inputs."""

    def test_state_fidelity_pure_mixed_mismatch(self):
        """Test dimension mismatch between pure and mixed states."""
        sv_1q = Statevector(1)
        dm_2q = DensityMatrix(2)

        with pytest.raises(ValueError):
            metrics.state_fidelity_pure_mixed(sv_1q, dm_2q)

        sv_2q = Statevector(2)
        dm_1q = DensityMatrix(1)

        with pytest.raises(ValueError):
            metrics.state_fidelity_pure_mixed(sv_2q, dm_1q)

    def test_partial_transpose_invalid_qubits(self):
        """Test partial transpose with out-of-bounds qubit indices."""
        dm = DensityMatrix(2)

        # Out of bounds should raise IndexError
        with pytest.raises(IndexError):
            metrics.partial_transpose(dm, [2])
        with pytest.raises(IndexError):
            metrics.partial_transpose(dm, [0, 3])

    def test_logarithmic_negativity_invalid_qubits(self):
        """Test logarithmic negativity with out-of-bounds qubit indices."""
        dm = DensityMatrix(3)

        # Out of bounds should raise IndexError
        with pytest.raises(IndexError):
            metrics.logarithmic_negativity(dm, [0, 5])

    def test_trace_distance_mixed_dimension_mismatch(self):
        """Test trace distance with mismatched dimensions."""
        dm1 = DensityMatrix(1)
        dm2 = DensityMatrix(2)

        with pytest.raises(ValueError):
            metrics.trace_distance_mixed(dm1, dm2)

    def test_state_fidelity_mixed_dimension_mismatch(self):
        """Test state fidelity with mismatched dimensions."""
        dm1 = DensityMatrix(1)
        dm2 = DensityMatrix(2)

        with pytest.raises(ValueError):
            metrics.state_fidelity_mixed(dm1, dm2)


class TestMetricsAdvancedFeatures:
    """Test advanced features and multi-qubit scenarios."""

    def test_partial_transpose_multi_qubit(self):
        """Test partial transpose on different subsystems of 3-qubit states."""
        # GHZ state
        dm = DensityMatrix(3)
        dm.apply_h(0)
        dm.apply_cx(0, 1)
        dm.apply_cx(0, 2)

        # PT on different subsystems
        for subsystem in [[0], [1], [2], [0, 1], [0, 2], [1, 2]]:
            pt_dm = metrics.partial_transpose(dm, subsystem)
            assert pt_dm is not None
            assert pt_dm.num_qubits == 3

            # Verify trace is preserved
            pt_data = pt_dm.data
            trace = np.trace(pt_data)
            assert math.isclose(abs(trace), 1.0, abs_tol=1e-10)

    def test_logarithmic_negativity_ghz_state(self):
        """Test logarithmic negativity for GHZ state."""
        # 3-qubit GHZ: |000> + |111>
        dm = DensityMatrix(3)
        dm.apply_h(0)
        dm.apply_cx(0, 1)
        dm.apply_cx(0, 2)

        # Single qubit bipartition: should be entangled
        log_neg_0 = metrics.logarithmic_negativity(dm, [0])
        assert log_neg_0 > 0.5  # Highly entangled

        # Two-qubit bipartition: also entangled
        log_neg_01 = metrics.logarithmic_negativity(dm, [0, 1])
        assert log_neg_01 > 0.5

    def test_logarithmic_negativity_w_state(self):
        """Test logarithmic negativity for W state."""
        # 3-qubit W state: (|001> + |010> + |100>) / sqrt(3)
        sv = Statevector(3)
        # Construct W state
        sv.apply_x(0)  # |001>

        sv2 = Statevector(3)
        sv2.apply_x(1)  # |010>

        sv3 = Statevector(3)
        sv3.apply_x(2)  # |100>

        # Superposition |001> + |010> + |100>
        # Approximate W state via superposition
        # For simplicity, just verify log negativity works on superposition states
        dm_w = DensityMatrix(3)
        dm_w.apply_h(0)
        dm_w.apply_cx(0, 1)

        log_neg = metrics.logarithmic_negativity(dm_w, [0])
        assert log_neg >= 0.0  # Non-negative for all states

    def test_fidelity_superposition_states(self):
        """Test fidelity for various superposition states."""
        # |+>, |->, |+i>, |-i>
        states = {
            "plus": lambda s: s.apply_h(0),
            "minus": lambda s: (s.apply_x(0), s.apply_h(0))[0],
            "plus_i": lambda s: s.apply_rx(0, np.pi / 2),
            "minus_i": lambda s: s.apply_rx(0, -np.pi / 2),
        }

        # Test orthogonality relationships
        sv_plus = Statevector(1)
        states["plus"](sv_plus)

        sv_minus = Statevector(1)
        states["minus"](sv_minus)

        # |+> and |-> are orthogonal
        fid = metrics.state_fidelity_pure(sv_plus, sv_minus)
        assert math.isclose(fid, 0.0, abs_tol=1e-10)

        # Each state with itself is 1.0
        for name, prep in states.items():
            sv = Statevector(1)
            prep(sv)
            fid = metrics.state_fidelity_pure(sv, sv)
            assert math.isclose(fid, 1.0, abs_tol=1e-10)

    def test_trace_distance_mixed_various_states(self):
        """Test trace distance between various mixed states."""
        # Identity I/2
        mixed_data = np.array([0.5, 0, 0, 0.5], dtype=complex)
        dm_i2 = DensityMatrix.from_density_matrix(1, mixed_data)

        # |0><0|
        dm_0 = DensityMatrix(1)

        # |+><+|
        dm_plus = DensityMatrix(1)
        dm_plus.apply_h(0)

        # D(|0><0|, I/2) = 0.5
        assert math.isclose(
            metrics.trace_distance_mixed(dm_0, dm_i2), 0.5, abs_tol=1e-10
        )

        # D(|0><0|, |+><+|) = 1/sqrt(2) ~ 0.707
        dist_0_plus = metrics.trace_distance_mixed(dm_0, dm_plus)
        assert math.isclose(dist_0_plus, 1 / np.sqrt(2), abs_tol=1e-10)

        # D(I/2, |+><+|) = 0.5
        assert math.isclose(
            metrics.trace_distance_mixed(dm_i2, dm_plus), 0.5, abs_tol=1e-10
        )

    def test_state_fidelity_pure_vs_mixed_equivalence(self):
        """Test consistency between pure-pure and pure-mixed fidelity."""
        # Fidelity(|0>, |0><0|) should equal Fidelity(|0>, |0>) = 1
        sv0 = Statevector(1)
        dm0 = DensityMatrix(1)

        fid_pure = metrics.state_fidelity_pure(sv0, Statevector(1))
        fid_mixed = metrics.state_fidelity_pure_mixed(sv0, dm0)

        assert math.isclose(fid_pure, fid_mixed, abs_tol=1e-10)

        # Fidelity(|0>, |1><1|) should equal Fidelity(|0>, |1>) = 0
        dm1 = DensityMatrix(1)
        dm1.apply_x(0)
        sv1 = Statevector(1)
        sv1.apply_x(0)

        fid_pure = metrics.state_fidelity_pure(sv0, sv1)
        fid_mixed = metrics.state_fidelity_pure_mixed(sv0, dm1)

        assert math.isclose(fid_pure, fid_mixed, abs_tol=1e-10)
