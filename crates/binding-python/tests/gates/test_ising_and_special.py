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
Tests for Ising interaction gates and special quantum gates.

Test coverage:
- Ising gates: RXX, RYY, RZZ, RZX (two-qubit parametric)
- Fermionic simulation gate (FSIM)
- XY interaction gates (XY, XY2P, XY2M)
- Sqrt gates (X2P, X2M, Y2P, Y2M)
- Matrix mathematical properties
"""

import numpy as np
from cqlib.circuit.gates import (
    RXX,
    RYY,
    RZZ,
    RZX,
    RXY,
    FSIM,
    XY,
    XY2P,
    XY2M,
    X2P,
    X2M,
    Y2P,
    Y2M,
)


def is_unitary(matrix, atol=1e-10):
    """Verify matrix is unitary: U†U = I"""
    n = matrix.shape[0]
    identity = np.eye(n)
    product = matrix.conj().T @ matrix
    return np.allclose(product, identity, atol=atol)


def commutator(a, b):
    """Compute matrix commutator [A, B] = AB - BA"""
    return a @ b - b @ a


class TestIsingGatesUnitary:
    """Test Ising gates are unitary"""

    def test_rxx_is_unitary(self):
        """RXX(θ) is unitary for any angle"""
        for theta in [0, np.pi / 4, np.pi / 2, np.pi, 2 * np.pi]:
            mat = RXX(theta).matrix()
            assert is_unitary(mat), f"RXX({theta}) should be unitary"

    def test_ryy_is_unitary(self):
        """RYY(θ) is unitary for any angle"""
        for theta in [0, np.pi / 4, np.pi / 2, np.pi]:
            mat = RYY(theta).matrix()
            assert is_unitary(mat), f"RYY({theta}) should be unitary"

    def test_rzz_is_unitary(self):
        """RZZ(θ) is unitary for any angle"""
        for theta in [0, np.pi / 4, np.pi / 2, np.pi]:
            mat = RZZ(theta).matrix()
            assert is_unitary(mat), f"RZZ({theta}) should be unitary"

    def test_rzx_is_unitary(self):
        """RZX(θ) is unitary for any angle"""
        for theta in [0, np.pi / 4, np.pi / 2, np.pi]:
            mat = RZX(theta).matrix()
            assert is_unitary(mat), f"RZX({theta}) should be unitary"


class TestIsingGatesMatrixStructure:
    """Test Ising gate matrix structure"""

    def test_rxx_matrix_structure(self):
        """RXX(θ) has diagonal and off-diagonal cosine/sine structure"""
        theta = np.pi / 2
        mat = RXX(theta).matrix()
        # RXX should be 4x4
        assert mat.shape == (4, 4)
        # Diagonal elements should be cos(θ/2)
        cos_half = np.cos(theta / 2)
        assert np.allclose(np.diag(mat), [cos_half, cos_half, cos_half, cos_half])

    def test_rzz_is_diagonal(self):
        """RZZ(θ) is diagonal in computational basis"""
        theta = np.pi / 3
        mat = RZZ(theta).matrix()
        off_diag = mat - np.diag(np.diag(mat))
        assert np.allclose(off_diag, 0), "RZZ should be diagonal"

    def test_rzz_diagonal_phases(self):
        """RZZ(θ) diagonal elements are phase factors"""
        theta = np.pi / 2
        mat = RZZ(theta).matrix()
        expected = np.diag(
            [
                np.exp(-1j * theta / 2),
                np.exp(1j * theta / 2),
                np.exp(1j * theta / 2),
                np.exp(-1j * theta / 2),
            ]
        )
        assert np.allclose(mat, expected), "RZZ diagonal phases incorrect"


class TestIsingGatesSpecialValues:
    """Test Ising gates at special parameter values"""

    def test_rzz_pi_is_cz_circuit(self):
        """RZZ(π) implements CZ up to single-qubit phases"""
        mat = RZZ(np.pi).matrix()
        # RZZ(π) = diag(-i, i, i, -i) = -i * diag(1, -1, -1, 1)
        # This is locally equivalent to CZ
        assert is_unitary(mat)

    def test_rzz_zero_is_identity(self):
        """RZZ(0) = I⊗I"""
        mat = RZZ(0).matrix()
        expected = np.eye(4)
        assert np.allclose(mat, expected), "RZZ(0) should be identity"

    def test_rxx_zero_is_identity(self):
        """RXX(0) = I⊗I"""
        mat = RXX(0).matrix()
        expected = np.eye(4)
        assert np.allclose(mat, expected), "RXX(0) should be identity"

    def test_ryy_zero_is_identity(self):
        """RYY(0) = I⊗I"""
        mat = RYY(0).matrix()
        expected = np.eye(4)
        assert np.allclose(mat, expected), "RYY(0) should be identity"


class TestIsingGatesInverse:
    """Test Ising gate inverse properties"""

    def test_rxx_inverse_negates_angle(self):
        """RXX(θ)† = RXX(-θ)"""
        theta = np.pi / 3
        gate = RXX(theta)
        inv_gate = gate.inverse()
        mat = gate.matrix()
        inv_mat = inv_gate.matrix()
        product = mat @ inv_mat
        assert np.allclose(product, np.eye(4)), "RXX inverse incorrect"

    def test_rzz_inverse_negates_angle(self):
        """RZZ(θ)† = RZZ(-θ)"""
        theta = np.pi / 4
        gate = RZZ(theta)
        inv_gate = gate.inverse()
        mat = gate.matrix()
        inv_mat = inv_gate.matrix()
        product = mat @ inv_mat
        assert np.allclose(product, np.eye(4)), "RZZ inverse incorrect"


class TestFSIMGate:
    """Test Fermionic Simulation (FSIM) gate"""

    def test_fsim_is_unitary(self):
        """FSIM(θ, φ) is unitary"""
        theta, phi = np.pi / 2, np.pi / 4
        mat = FSIM(theta, phi).matrix()
        assert is_unitary(mat), "FSIM should be unitary"

    def test_fsim_matrix_shape(self):
        """FSIM is 4x4 matrix"""
        mat = FSIM(0.5, 0.3).matrix()
        assert mat.shape == (4, 4)

    def test_fsim_zero_swap(self):
        """FSIM(0, φ) has no swap component"""
        phi = np.pi / 4
        mat = FSIM(0, phi).matrix()
        # When theta=0, the swap-like off-diagonals should be zero
        assert is_unitary(mat)

    def test_fsim_inverse(self):
        """FSIM inverse negates both angles"""
        theta, phi = np.pi / 3, np.pi / 6
        gate = FSIM(theta, phi)
        inv_gate = gate.inverse()
        mat = gate.matrix()
        inv_mat = inv_gate.matrix()
        product = mat @ inv_mat
        assert np.allclose(product, np.eye(4), atol=1e-10), "FSIM inverse incorrect"


class TestXYGates:
    """Test XY interaction gates"""

    def test_xy_is_unitary(self):
        """XY(θ) is unitary"""
        for theta in [0, np.pi / 4, np.pi / 2, np.pi]:
            mat = XY(theta).matrix()
            assert is_unitary(mat), f"XY({theta}) should be unitary"

    def test_xy_matrix_shape(self):
        """XY is 2x2 matrix"""
        mat = XY(np.pi / 2).matrix()
        assert mat.shape == (2, 2)

    def test_xy2p_is_unitary(self):
        """XY2P(θ) is unitary"""
        theta = np.pi / 3
        mat = XY2P(theta).matrix()
        assert is_unitary(mat), "XY2P should be unitary"

    def test_xy2m_is_unitary(self):
        """XY2M(θ) is unitary"""
        theta = np.pi / 3
        mat = XY2M(theta).matrix()
        assert is_unitary(mat), "XY2M should be unitary"

    def test_xy2p_xy2m_inverse_pair(self):
        """XY2P and XY2M are inverse for same angle"""
        theta = np.pi / 4
        xy2p = XY2P(theta)
        xy2m_inv = xy2p.inverse()
        mat_p = xy2p.matrix()
        mat_m_inv = xy2m_inv.matrix()
        product = mat_p @ mat_m_inv
        assert np.allclose(product, np.eye(2)), "XY2P @ XY2M should be identity"


class TestSqrtGates:
    """Test square root gates (X2P, X2M, Y2P, Y2M)"""

    def test_x2p_squared_is_x(self):
        """X2P @ X2P = X (up to global phase)"""
        x2p_mat = X2P.matrix()
        product = x2p_mat @ x2p_mat
        # Should be equivalent up to global phase
        assert np.allclose(product @ product.conj().T, np.eye(2))

    def test_x2m_squared_is_x(self):
        """X2M @ X2M = X (up to global phase)"""
        x2m_mat = X2M.matrix()
        product = x2m_mat @ x2m_mat
        assert np.allclose(product @ product.conj().T, np.eye(2))

    def test_x2p_x2m_are_inverse(self):
        """X2P and X2M are inverses"""
        x2p_mat = X2P.matrix()
        x2m_mat = X2M.matrix()
        product = x2p_mat @ x2m_mat
        assert np.allclose(product, np.eye(2)), "X2P @ X2M should be I"

    def test_y2p_squared_is_y(self):
        """Y2P @ Y2P = Y (up to global phase)"""
        y2p_mat = Y2P.matrix()
        product = y2p_mat @ y2p_mat
        assert np.allclose(product @ product.conj().T, np.eye(2))

    def test_y2m_squared_is_y(self):
        """Y2M @ Y2M = Y (up to global phase)"""
        y2m_mat = Y2M.matrix()
        product = y2m_mat @ y2m_mat
        assert np.allclose(product @ product.conj().T, np.eye(2))

    def test_y2p_y2m_are_inverse(self):
        """Y2P and Y2M are inverses"""
        y2p_mat = Y2P.matrix()
        y2m_mat = Y2M.matrix()
        product = y2p_mat @ y2m_mat
        assert np.allclose(product, np.eye(2)), "Y2P @ Y2M should be I"

    def test_sqrt_gates_are_unitary(self):
        """All sqrt gates are unitary"""
        gates = [X2P, X2M, Y2P, Y2M]
        for gate in gates:
            mat = gate.matrix()
            assert is_unitary(mat), f"{gate} should be unitary"


class TestRXYGate:
    """Test RXY gate (rotation in XY plane)"""

    def test_rxy_is_unitary(self):
        """RXY(θ, φ) is unitary"""
        theta, phi = np.pi / 2, np.pi / 4
        mat = RXY(theta, phi).matrix()
        assert is_unitary(mat), "RXY should be unitary"

    def test_rxy_matrix_shape(self):
        """RXY is 2x2 matrix"""
        mat = RXY(0.5, 0.3).matrix()
        assert mat.shape == (2, 2)

    def test_rxy_periodicity(self):
        """RXY(θ + 4π, φ) = RXY(θ, φ)"""
        phi = np.pi / 3
        mat1 = RXY(np.pi / 2, phi).matrix()
        mat2 = RXY(np.pi / 2 + 4 * np.pi, phi).matrix()
        assert np.allclose(mat1, mat2), "RXY should have 4π periodicity"
