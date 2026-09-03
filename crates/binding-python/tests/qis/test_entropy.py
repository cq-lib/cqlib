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
from cqlib.qis import Statevector, DensityMatrix, entropy


def test_linear_entropy():
    # Pure state: linear entropy = 0
    dm_pure = DensityMatrix(2)
    assert abs(entropy.linear_entropy(dm_pure) - 0.0) < 1e-10

    # Maximally mixed state: linear entropy = 1 - 1/d = 0.75 for 2 qubits
    # Simulate mixed state manually (or apply operations to create one)
    # Since we can't easily create mixed states via pure state operations,
    # we just test the pure case for now.


def test_renyi_entropy():
    dm = DensityMatrix(2)
    # Pure state: all Renyi entropies are 0
    assert abs(entropy.renyi_entropy(dm, 2.0) - 0.0) < 1e-10
    assert abs(entropy.renyi_entropy(dm, 0.5) - 0.0) < 1e-10
    assert abs(entropy.renyi_entropy(dm, 1.0) - 0.0) < 1e-10  # Fallback to Von Neumann

    # Edge case: alpha <= 0
    with pytest.raises(ValueError, match="alpha must be positive"):
        entropy.renyi_entropy(dm, -1.0)
    with pytest.raises(ValueError, match="alpha must be positive"):
        entropy.renyi_entropy(dm, 0.0)


def test_entanglement_entropy_pure():
    # Create Bell state |Phi+> = (|00> + |11>) / sqrt(2)
    sv = Statevector(2)
    sv.apply_h(0)
    sv.apply_cx(0, 1)

    # Entanglement entropy should be 1.0 for maximally entangled state
    ee = entropy.entanglement_entropy_pure(sv, [0])
    assert abs(ee - 1.0) < 1e-10

    # Product state |00>: EE = 0
    sv_prod = Statevector(2)
    assert abs(entropy.entanglement_entropy_pure(sv_prod, [0]) - 0.0) < 1e-10

    # Edge cases: Invalid subsystems
    with pytest.raises(ValueError):
        entropy.entanglement_entropy_pure(sv, [])  # Empty subsystem
    with pytest.raises(ValueError):
        entropy.entanglement_entropy_pure(sv, [0, 1])  # Subsystem equals full system
    with pytest.raises(ValueError):
        entropy.entanglement_entropy_pure(sv, [0, 0])  # Duplicates
    with pytest.raises(IndexError):
        entropy.entanglement_entropy_pure(sv, [2])  # Out of bounds


def test_negativity():
    # Bell state |Phi+>
    dm = DensityMatrix(2)
    dm.apply_h(0)
    dm.apply_cx(0, 1)

    # Negativity of Bell state is 0.5
    neg = entropy.negativity(dm, [0])
    assert abs(neg - 0.5) < 1e-10

    # Separable state |00><00|
    dm_sep = DensityMatrix(2)
    assert abs(entropy.negativity(dm_sep, [0]) - 0.0) < 1e-10

    # Edge cases
    with pytest.raises(IndexError):
        entropy.negativity(dm, [2])  # Out of bounds


def test_concurrence():
    # Bell state
    dm = DensityMatrix(2)
    dm.apply_h(0)
    dm.apply_cx(0, 1)

    assert abs(entropy.concurrence(dm) - 1.0) < 1e-10

    # Separable state
    dm_sep = DensityMatrix(2)
    assert abs(entropy.concurrence(dm_sep) - 0.0) < 1e-10

    # Edge case: not 2 qubits
    dm_3 = DensityMatrix(3)
    with pytest.raises(ValueError, match="Unsupported dimension"):
        entropy.concurrence(dm_3)


def test_entanglement_of_formation():
    # Bell state
    dm = DensityMatrix(2)
    dm.apply_h(0)
    dm.apply_cx(0, 1)

    assert abs(entropy.entanglement_of_formation(dm) - 1.0) < 1e-10

    # Separable state
    dm_sep = DensityMatrix(2)
    assert abs(entropy.entanglement_of_formation(dm_sep) - 0.0) < 1e-10

    # Edge case: not 2 qubits
    dm_3 = DensityMatrix(3)
    with pytest.raises(ValueError, match="Unsupported dimension"):
        entropy.entanglement_of_formation(dm_3)


class TestEntropyBoundaryConditions:
    """Test boundary conditions and edge cases."""

    def test_linear_entropy_pure_vs_mixed(self):
        """Test linear entropy distinguishes pure and mixed states."""
        from cqlib.qis.state import DensityMatrixNoise
        from cqlib.device import SingleQubitNoise, NoiseModel
        from cqlib.circuit import StandardGate

        # Pure state
        dm_pure = DensityMatrix(1)
        dm_pure.apply_h(0)
        le_pure = entropy.linear_entropy(dm_pure)
        assert abs(le_pure - 0.0) < 1e-10

        # Mixed state via depolarizing noise using DensityMatrixNoise
        noise_model = NoiseModel()
        depol_noise = SingleQubitNoise.depolarizing(p=0.5)
        noise_model.add_single_qubit_error(StandardGate.H, 0, depol_noise)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_h(0)
        dm_mixed = DensityMatrix.from_density_matrix(1, sim.state.flatten().tolist())
        le_mixed = entropy.linear_entropy(dm_mixed)
        # Mixed state should have linear entropy > 0
        assert le_mixed > 0.1

    @pytest.mark.parametrize("alpha", [0.5, 1.0, 2.0, 10.0])
    def test_renyi_entropy_various_alpha(self, alpha):
        """Test Renyi entropy with various alpha values."""
        dm = DensityMatrix(2)
        # Pure state: all Renyi entropies should be 0
        re = entropy.renyi_entropy(dm, alpha)
        assert abs(re - 0.0) < 1e-10

    def test_renyi_entropy_maximally_mixed(self):
        """Test Renyi entropy for maximally mixed state."""
        # Create maximally mixed state via completely mixed density matrix
        # Use bit-flip channel with p=0.5 on |0><0|
        from cqlib.qis.state import DensityMatrixNoise
        from cqlib.device import SingleQubitNoise, NoiseModel
        from cqlib.circuit import StandardGate

        noise_model = NoiseModel()
        bf_noise = SingleQubitNoise.bit_flip(p=0.5)
        noise_model.add_single_qubit_error(StandardGate.X, 0, bf_noise)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_x(0)  # This creates mixed state due to noise
        dm_mixed = DensityMatrix.from_density_matrix(1, sim.state.flatten().tolist())

        # Renyi entropy of mixed state should be > 0
        re = entropy.renyi_entropy(dm_mixed, 2.0)
        assert re > 0.0

    def test_entanglement_entropy_large_system(self):
        """Test entanglement entropy for larger systems."""
        # GHZ state on 4 qubits: |0000> + |1111>
        sv = Statevector(4)
        sv.apply_h(0)
        sv.apply_cx(0, 1)
        sv.apply_cx(0, 2)
        sv.apply_cx(0, 3)

        # Entanglement entropy of single qubit should be 1.0 (maximal)
        ee = entropy.entanglement_entropy_pure(sv, [0])
        assert abs(ee - 1.0) < 1e-10

        # Entanglement entropy of two qubits
        ee_two = entropy.entanglement_entropy_pure(sv, [0, 1])
        assert abs(ee_two - 1.0) < 1e-10

    def test_entanglement_entropy_different_subsystems(self):
        """Test entanglement entropy with different subsystem choices."""
        # Bell state
        sv = Statevector(2)
        sv.apply_h(0)
        sv.apply_cx(0, 1)

        # Subsystem A = {0}
        ee_0 = entropy.entanglement_entropy_pure(sv, [0])
        assert abs(ee_0 - 1.0) < 1e-10

        # Subsystem A = {1}
        ee_1 = entropy.entanglement_entropy_pure(sv, [1])
        assert abs(ee_1 - 1.0) < 1e-10

    def test_negativity_multi_qubit(self):
        """Test negativity for multi-qubit systems."""
        # 3-qubit GHZ state
        dm = DensityMatrix(3)
        dm.apply_h(0)
        dm.apply_cx(0, 1)
        dm.apply_cx(0, 2)

        # Negativity for different bipartitions
        neg_0 = entropy.negativity(dm, [0])
        assert neg_0 > 0.0  # Should be entangled

        neg_01 = entropy.negativity(dm, [0, 1])
        assert neg_01 > 0.0

    def test_concurrence_eof_relationship(self):
        """Test relationship between concurrence and entanglement of formation."""
        # For Bell state: C = 1, EoF = 1
        dm_bell = DensityMatrix(2)
        dm_bell.apply_h(0)
        dm_bell.apply_cx(0, 1)

        c = entropy.concurrence(dm_bell)
        eof = entropy.entanglement_of_formation(dm_bell)

        # Both should be 1 for Bell state
        assert abs(c - 1.0) < 1e-10
        assert abs(eof - 1.0) < 1e-10

        # For separable state: C = 0, EoF = 0
        dm_sep = DensityMatrix(2)
        c_sep = entropy.concurrence(dm_sep)
        eof_sep = entropy.entanglement_of_formation(dm_sep)

        assert abs(c_sep - 0.0) < 1e-10
        assert abs(eof_sep - 0.0) < 1e-10


class TestEntropyQuantumInvariants:
    """Test quantum information invariants."""

    def test_pure_state_renyi_independence(self):
        """Test that pure state Renyi entropy is independent of alpha."""
        dm = DensityMatrix(2)
        dm.apply_h(0)
        dm.apply_cx(0, 1)  # Bell state

        alphas = [0.5, 1.0, 2.0, 5.0, 10.0]
        entropies = [entropy.renyi_entropy(dm, a) for a in alphas]

        # All should be approximately 0 for pure state
        for e in entropies:
            assert abs(e) < 1e-10

    def test_concurrence_bounds(self):
        """Test concurrence is bounded in [0, 1]."""
        # Separable state
        dm_sep = DensityMatrix(2)
        c_sep = entropy.concurrence(dm_sep)
        assert 0.0 <= c_sep <= 1.0

        # Bell state
        dm_bell = DensityMatrix(2)
        dm_bell.apply_h(0)
        dm_bell.apply_cx(0, 1)
        c_bell = entropy.concurrence(dm_bell)
        assert 0.0 <= c_bell <= 1.0
        assert abs(c_bell - 1.0) < 1e-10

    def test_eof_bounds(self):
        """Test entanglement of formation is bounded."""
        # Separable state: EoF = 0
        dm_sep = DensityMatrix(2)
        eof_sep = entropy.entanglement_of_formation(dm_sep)
        assert eof_sep >= 0.0

        # Bell state: EoF = 1 (maximal for 2 qubits)
        dm_bell = DensityMatrix(2)
        dm_bell.apply_h(0)
        dm_bell.apply_cx(0, 1)
        eof_bell = entropy.entanglement_of_formation(dm_bell)
        assert eof_bell >= 0.0
        assert abs(eof_bell - 1.0) < 1e-10

    def test_linear_entropy_pure_state_zero(self):
        """Test linear entropy is 0 for any pure state."""
        test_states = [
            lambda dm: None,  # |00>
            lambda dm: dm.apply_h(0),  # |+0>
            lambda dm: (dm.apply_h(0), dm.apply_cx(0, 1))[0],  # Bell
        ]

        for prep in test_states:
            dm = DensityMatrix(2)
            prep(dm)
            le = entropy.linear_entropy(dm)
            assert abs(le) < 1e-10, "Linear entropy should be 0 for pure state"

    def test_negativity_non_negative(self):
        """Test negativity is always non-negative."""
        # Test various states
        states = [
            DensityMatrix(2),  # |00>
        ]

        # Add Bell state
        dm_bell = DensityMatrix(2)
        dm_bell.apply_h(0)
        dm_bell.apply_cx(0, 1)
        states.append(dm_bell)

        for dm in states:
            neg = entropy.negativity(dm, [0])
            assert neg >= 0.0, "Negativity must be non-negative"


class TestEntropyNumericalPrecision:
    """Test numerical precision and stability."""

    def test_near_pure_state_entropy(self):
        """Test entropy for states near pure."""
        from cqlib.qis.state import DensityMatrixNoise
        from cqlib.device import SingleQubitNoise, NoiseModel
        from cqlib.circuit import StandardGate

        # Very small noise
        noise_model = NoiseModel()
        noise = SingleQubitNoise.depolarizing(p=1e-6)
        noise_model.add_single_qubit_error(StandardGate.H, 0, noise)

        sim = DensityMatrixNoise(1, noise_model)
        sim.apply_h(0)
        dm = DensityMatrix.from_density_matrix(1, sim.state.flatten().tolist())

        # Linear entropy should be very small but positive
        le = entropy.linear_entropy(dm)
        assert 0.0 <= le < 1e-4

    def test_renyi_alpha_near_one(self):
        """Test Renyi entropy stability when alpha is close to 1."""
        dm = DensityMatrix(1)
        dm.apply_h(0)

        # Alpha very close to 1
        alphas = [0.999, 1.001, 0.9999, 1.0001]
        for alpha in alphas:
            re = entropy.renyi_entropy(dm, alpha)
            # Should be close to 0 for pure state
            assert abs(re) < 1e-6

    def test_small_entanglement_detection(self):
        """Test detection of small entanglement."""
        # Create slightly entangled state
        sv = Statevector(2)
        sv.apply_rx(0, 0.1)  # Small rotation
        sv.apply_cx(0, 1)

        # Should still have some entanglement
        ee = entropy.entanglement_entropy_pure(sv, [0])
        assert ee > 0.0
        assert ee < 1.0  # But not maximal


class TestEntropyErrorHandling:
    """Test error handling for invalid inputs."""

    def test_renyi_alpha_extreme_values(self):
        """Test Renyi entropy with extreme alpha values."""
        dm = DensityMatrix(2)

        # Very small positive alpha
        re_small = entropy.renyi_entropy(dm, 1e-10)
        assert abs(re_small) < 1e-10  # Pure state

        # Very large alpha
        re_large = entropy.renyi_entropy(dm, 1e6)
        assert abs(re_large) < 1e-10  # Pure state

    def test_entanglement_entropy_invalid_partitions(self):
        """Test entanglement entropy with invalid subsystem partitions."""
        sv = Statevector(3)

        # Overlapping/invalid partitions should raise ValueError
        with pytest.raises(ValueError):
            entropy.entanglement_entropy_pure(sv, [0, 1, 2, 3])  # Out of bounds

        # Empty subsystem should raise ValueError
        with pytest.raises(ValueError):
            entropy.entanglement_entropy_pure(sv, [])  # Empty subsystem

    def test_negativity_invalid_subsystem(self):
        """Test negativity with invalid subsystem."""
        dm = DensityMatrix(2)

        with pytest.raises(IndexError):
            entropy.negativity(dm, [0, 1, 2])  # Out of bounds
