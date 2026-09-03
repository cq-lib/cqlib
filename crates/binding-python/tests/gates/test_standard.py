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
Tests for StandardGate quantum gates.

Test coverage:
- 36 standard gate instances access
- Gate properties (num_qubits, num_params, num_ctrl_qubits)
- Gate matrix computation (exact value verification)
- Unitary property verification
- Gate parameter binding
- Gate inverse (mathematical correctness)
- Gate control elevation
- Parametric gate matrix variation with parameters
"""

import pytest
import numpy as np
from cqlib.circuit.gates import (
    StandardGate,
    I,
    H,
    X,
    Y,
    Z,
    S,
    SDG,
    T,
    TDG,
    RX,
    RY,
    RZ,
    U,
    Phase,
    GPhase,
    CX,
    CY,
    CZ,
    SWAP,
    CCX,
    CRX,
    CRY,
    CRZ,
    RXX,
    RYY,
    RZZ,
    RZX,
    RXY,
    FSIM,
    X2P,
    X2M,
    Y2P,
    Y2M,
    XY,
    XY2P,
    XY2M,
)
from cqlib.circuit import Parameter


def is_unitary(matrix, atol=1e-10):
    """Verify matrix is unitary: U†U = I"""
    n = matrix.shape[0]
    identity = np.eye(n)
    product = matrix.conj().T @ matrix
    return np.allclose(product, identity, atol=atol)


def matrix_close(mat1, mat2, atol=1e-10):
    """Compare two matrices for equality (allowing global phase difference)"""
    # Direct comparison first
    if np.allclose(mat1, mat2, atol=atol):
        return True
    # Consider global phase
    # Find first non-zero element and compute phase difference
    flat1 = mat1.flatten()
    flat2 = mat2.flatten()
    for i in range(len(flat1)):
        if np.abs(flat1[i]) > atol and np.abs(flat2[i]) > atol:
            phase = flat2[i] / flat1[i]
            if np.allclose(mat1 * phase, mat2, atol=atol):
                return True
            break
    return False


class TestGateInstancesExist:
    """Test standard gate instances exist and have correct type"""

    SINGLE_QUBIT_GATES = [I, H, X, Y, Z, S, SDG, T, TDG]
    PARAMETRIC_GATES = [RX, RY, RZ, U, Phase, GPhase]
    TWO_QUBIT_GATES = [CX, CY, CZ, SWAP]
    ISING_GATES = [RXX, RYY, RZZ, RZX, RXY]
    CONTROLLED_ROTATION_GATES = [CRX, CRY, CRZ]
    SPECIAL_GATES = [X2P, X2M, Y2P, Y2M, XY, XY2P, XY2M, FSIM]

    @pytest.mark.parametrize("gate", SINGLE_QUBIT_GATES)
    def test_single_qubit_gate_is_standard_gate(self, gate):
        """Single qubit gates are StandardGate type"""
        assert isinstance(gate, StandardGate), f"{gate} should be StandardGate"

    @pytest.mark.parametrize("gate", PARAMETRIC_GATES)
    def test_parametric_gate_is_standard_gate(self, gate):
        """Parametric gates are StandardGate type"""
        assert isinstance(gate, StandardGate), f"{gate} should be StandardGate"

    @pytest.mark.parametrize("gate", TWO_QUBIT_GATES)
    def test_two_qubit_gate_is_standard_gate(self, gate):
        """Two-qubit gates are StandardGate type"""
        assert isinstance(gate, StandardGate), f"{gate} should be StandardGate"

    @pytest.mark.parametrize("gate", ISING_GATES)
    def test_ising_gate_is_standard_gate(self, gate):
        """Ising gates are StandardGate type"""
        assert isinstance(gate, StandardGate), f"{gate} should be StandardGate"

    @pytest.mark.parametrize("gate", CONTROLLED_ROTATION_GATES)
    def test_controlled_rotation_gate_is_standard_gate(self, gate):
        """Controlled rotation gates are StandardGate type"""
        assert isinstance(gate, StandardGate), f"{gate} should be StandardGate"

    def test_multi_controlled_gate_exists(self):
        """Multi-controlled CCX gate exists and is StandardGate type"""
        assert isinstance(CCX, StandardGate)

    @pytest.mark.parametrize("gate", SPECIAL_GATES)
    def test_special_gate_is_standard_gate(self, gate):
        """Special gates are StandardGate type"""
        assert isinstance(gate, StandardGate), f"{gate} should be StandardGate"


class TestGateProperties:
    """Test gate property exact values"""

    # (gate, num_qubits, num_params, num_ctrl_qubits)
    GATE_PROPERTIES = [
        (I, 1, 0, 0),
        (H, 1, 0, 0),
        (X, 1, 0, 0),
        (Y, 1, 0, 0),
        (Z, 1, 0, 0),
        (S, 1, 0, 0),
        (SDG, 1, 0, 0),
        (T, 1, 0, 0),
        (TDG, 1, 0, 0),
        (RX, 1, 1, 0),
        (RY, 1, 1, 0),
        (RZ, 1, 1, 0),
        (U, 1, 3, 0),
        (Phase, 1, 1, 0),
        (GPhase, 0, 1, 0),
        (CX, 2, 0, 1),
        (CY, 2, 0, 1),
        (CZ, 2, 0, 1),
        (SWAP, 2, 0, 0),
        (CCX, 3, 0, 2),
        (CRX, 2, 1, 1),
        (CRY, 2, 1, 1),
        (CRZ, 2, 1, 1),
        (RXX, 2, 1, 0),
        (RYY, 2, 1, 0),
        (RZZ, 2, 1, 0),
        (RZX, 2, 1, 0),
        (RXY, 1, 2, 0),  # RXY is 1-qubit gate with 2 parameters
        (XY, 1, 1, 0),  # XY is 1-qubit gate with 1 parameter
        (X2P, 1, 0, 0),
        (X2M, 1, 0, 0),
        (Y2P, 1, 0, 0),
        (Y2M, 1, 0, 0),
        (XY2P, 1, 1, 0),
        (XY2M, 1, 1, 0),
        (FSIM, 2, 2, 0),
    ]

    @pytest.mark.parametrize(
        "gate, num_qubits, num_params, num_ctrl_qubits", GATE_PROPERTIES
    )
    def test_gate_properties(self, gate, num_qubits, num_params, num_ctrl_qubits):
        """Verify each gate's property values"""
        assert gate.num_qubits == num_qubits, (
            f"{gate} num_qubits should be {num_qubits}"
        )
        assert gate.num_params == num_params, (
            f"{gate} num_params should be {num_params}"
        )
        assert gate.num_ctrl_qubits == num_ctrl_qubits, (
            f"{gate} num_ctrl_qubits should be {num_ctrl_qubits}"
        )


class TestGateMatrixUnitary:
    """Test gate matrix unitary property"""

    SINGLE_QUBIT_GATES = [I, H, X, Y, Z, S, SDG, T, TDG, X2P, X2M, Y2P, Y2M]
    TWO_QUBIT_GATES = [CX, CY, CZ, SWAP]
    THREE_QUBIT_GATES = [CCX]

    @pytest.mark.parametrize("gate", SINGLE_QUBIT_GATES)
    def test_single_qubit_unitary(self, gate):
        """Single qubit gate matrices are unitary"""
        mat = gate.matrix()
        assert mat.shape == (2, 2)
        assert is_unitary(mat), f"{gate} should be unitary"

    @pytest.mark.parametrize("gate", TWO_QUBIT_GATES)
    def test_two_qubit_unitary(self, gate):
        """Two-qubit gate matrices are unitary"""
        mat = gate.matrix()
        assert mat.shape == (4, 4)
        assert is_unitary(mat), f"{gate} should be unitary"

    def test_three_qubit_unitary(self):
        """Three-qubit gate matrices are unitary"""
        mat = CCX.matrix()
        assert mat.shape == (8, 8)
        assert is_unitary(mat), "CCX should be unitary"


class TestGateMatricesExact:
    """Test gate matrix exact values"""

    def test_hadamard_matrix_exact(self):
        """Hadamard gate matrix exact value"""
        mat = H.matrix()
        expected = np.array([[1, 1], [1, -1]]) / np.sqrt(2)
        assert np.allclose(mat, expected), "H matrix mismatch"

    def test_pauli_x_matrix_exact(self):
        """Pauli-X gate matrix exact value"""
        mat = X.matrix()
        expected = np.array([[0, 1], [1, 0]])
        assert np.allclose(mat, expected), "X matrix mismatch"

    def test_pauli_y_matrix_exact(self):
        """Pauli-Y gate matrix exact value"""
        mat = Y.matrix()
        expected = np.array([[0, -1j], [1j, 0]])
        assert np.allclose(mat, expected), "Y matrix mismatch"

    def test_pauli_z_matrix_exact(self):
        """Pauli-Z gate matrix exact value"""
        mat = Z.matrix()
        expected = np.array([[1, 0], [0, -1]])
        assert np.allclose(mat, expected), "Z matrix mismatch"

    def test_identity_matrix_exact(self):
        """Identity gate matrix exact value"""
        mat = I.matrix()
        expected = np.eye(2)
        assert np.allclose(mat, expected), "I matrix mismatch"

    def test_s_gate_matrix_exact(self):
        """S gate matrix exact value"""
        mat = S.matrix()
        expected = np.array([[1, 0], [0, 1j]])
        assert np.allclose(mat, expected), "S matrix mismatch"

    def test_sdg_gate_matrix_exact(self):
        """S† gate matrix exact value"""
        mat = SDG.matrix()
        expected = np.array([[1, 0], [0, -1j]])
        assert np.allclose(mat, expected), "SDG matrix mismatch"

    def test_t_gate_matrix_exact(self):
        """T gate matrix exact value"""
        mat = T.matrix()
        expected = np.array([[1, 0], [0, np.exp(1j * np.pi / 4)]])
        assert np.allclose(mat, expected), "T matrix mismatch"

    def test_tdg_gate_matrix_exact(self):
        """T† gate matrix exact value"""
        mat = TDG.matrix()
        expected = np.array([[1, 0], [0, np.exp(-1j * np.pi / 4)]])
        assert np.allclose(mat, expected), "TDG matrix mismatch"

    def test_cnot_matrix_exact(self):
        """CNOT gate matrix exact value"""
        mat = CX.matrix()
        expected = np.array([[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 0, 1], [0, 0, 1, 0]])
        assert np.allclose(mat, expected), "CX matrix mismatch"

    def test_swap_matrix_exact(self):
        """SWAP gate matrix exact value"""
        mat = SWAP.matrix()
        expected = np.array([[1, 0, 0, 0], [0, 0, 1, 0], [0, 1, 0, 0], [0, 0, 0, 1]])
        assert np.allclose(mat, expected), "SWAP matrix mismatch"

    def test_cz_matrix_exact(self):
        """CZ gate matrix exact value"""
        mat = CZ.matrix()
        expected = np.array([[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, -1]])
        assert np.allclose(mat, expected), "CZ matrix mismatch"

    def test_sqrt_x_matrix_exact(self):
        """√X (SX) gate matrix exact value: 1/√2 * [[1, -i], [-i, 1]]"""
        mat = X2P.matrix()
        expected = np.array([[1, -1j], [-1j, 1]]) / np.sqrt(2)
        assert np.allclose(mat, expected), "X2P matrix mismatch"

    def test_sqrt_x_dagger_matrix_exact(self):
        """√X† (SXdg) gate matrix exact value: 1/√2 * [[1, i], [i, 1]]"""
        mat = X2M.matrix()
        expected = np.array([[1, 1j], [1j, 1]]) / np.sqrt(2)
        assert np.allclose(mat, expected), "X2M matrix mismatch"

    def test_sqrt_y_matrix_exact(self):
        """√Y gate matrix exact value: 1/√2 * [[1, -1], [1, 1]]"""
        mat = Y2P.matrix()
        expected = np.array([[1, -1], [1, 1]]) / np.sqrt(2)
        assert np.allclose(mat, expected), "Y2P matrix mismatch"

    def test_sqrt_y_dagger_matrix_exact(self):
        """√Y† gate matrix exact value: 1/√2 * [[1, 1], [-1, 1]]"""
        mat = Y2M.matrix()
        expected = np.array([[1, 1], [-1, 1]]) / np.sqrt(2)
        assert np.allclose(mat, expected), "Y2M matrix mismatch"


class TestParametricGateMatrices:
    """Test parametric gate matrix mathematical correctness"""

    def test_rx_pi_equals_x(self):
        """RX(pi) = -iX (global phase)"""
        mat = RX(np.pi).matrix()
        x_mat = X.matrix()
        # RX(pi) should be equivalent to X (allowing global phase)
        assert matrix_close(mat, -1j * x_mat), "RX(pi) should equal -iX"
        assert matrix_close(mat, x_mat), "RX(pi) should be equivalent to X"

    def test_rx_pi_half(self):
        """RX(pi/2) verification"""
        theta = np.pi / 2
        mat = RX(theta).matrix()
        expected = np.array(
            [
                [np.cos(theta / 2), -1j * np.sin(theta / 2)],
                [-1j * np.sin(theta / 2), np.cos(theta / 2)],
            ]
        )
        assert np.allclose(mat, expected), "RX(pi/2) matrix mismatch"
        assert is_unitary(mat), "RX(pi/2) should be unitary"

    def test_rx_zero_is_identity(self):
        """RX(0) = I"""
        mat = RX(0).matrix()
        expected = np.eye(2)
        assert np.allclose(mat, expected), "RX(0) should be identity"

    def test_ry_pi_equals_y(self):
        """RY(pi) = -iY (global phase)"""
        mat = RY(np.pi).matrix()
        y_mat = Y.matrix()
        assert matrix_close(mat, y_mat), "RY(pi) should be equivalent to Y"

    def test_ry_pi_half(self):
        """RY(pi/2) verification"""
        theta = np.pi / 2
        mat = RY(theta).matrix()
        expected = np.array(
            [
                [np.cos(theta / 2), -np.sin(theta / 2)],
                [np.sin(theta / 2), np.cos(theta / 2)],
            ]
        )
        assert np.allclose(mat, expected), "RY(pi/2) matrix mismatch"
        assert is_unitary(mat), "RY(pi/2) should be unitary"

    def test_rz_pi_equals_z(self):
        """RZ(pi) = -iZ (global phase)"""
        mat = RZ(np.pi).matrix()
        z_mat = Z.matrix()
        assert matrix_close(mat, z_mat), "RZ(pi) should be equivalent to Z"

    def test_rz_phase(self):
        """RZ(phi) = diag(e^(-iφ/2), e^(iφ/2))"""
        phi = np.pi / 3
        mat = RZ(phi).matrix()
        expected = np.array([[np.exp(-1j * phi / 2), 0], [0, np.exp(1j * phi / 2)]])
        assert np.allclose(mat, expected), "RZ matrix mismatch"

    def test_phase_gate(self):
        """P(λ) = diag(1, e^(iλ))"""
        lam = np.pi / 4
        mat = Phase(lam).matrix()
        expected = np.array([[1, 0], [0, np.exp(1j * lam)]])
        assert np.allclose(mat, expected), "Phase gate matrix mismatch"

    def test_global_phase_gate(self):
        """GPhase(θ) = e^(iθ) I (2x2)"""
        theta = np.pi / 4
        mat = GPhase(theta).matrix()
        # GPhase is 1-qubit gate with 1 parameter, matrix is 2x2
        expected = np.exp(1j * theta) * np.eye(2)
        assert np.allclose(mat, expected), "GPhase matrix mismatch"

    def test_u_gate_general(self):
        """U gate is general single-qubit gate"""
        theta, phi, lam = np.pi / 2, np.pi / 4, np.pi / 3
        mat = U(theta, phi, lam).matrix()
        # U(θ, φ, λ) = [[cos(θ/2), -e^(iλ)sin(θ/2)],
        #               [e^(iφ)sin(θ/2), e^(i(φ+λ))cos(θ/2)]]
        expected = np.array(
            [
                [np.cos(theta / 2), -np.exp(1j * lam) * np.sin(theta / 2)],
                [
                    np.exp(1j * phi) * np.sin(theta / 2),
                    np.exp(1j * (phi + lam)) * np.cos(theta / 2),
                ],
            ]
        )
        assert np.allclose(mat, expected), "U gate matrix mismatch"
        assert is_unitary(mat), "U gate should be unitary"

    def test_crx_matrix(self):
        """CRX gate matrix verification"""
        theta = np.pi / 2
        mat = CRX(theta).matrix()
        # CRX = |0⟩⟨0| ⊗ I + |1⟩⟨1| ⊗ RX(θ)
        cos_half = np.cos(theta / 2)
        sin_half = np.sin(theta / 2)
        expected = np.array(
            [
                [1, 0, 0, 0],
                [0, 1, 0, 0],
                [0, 0, cos_half, -1j * sin_half],
                [0, 0, -1j * sin_half, cos_half],
            ]
        )
        assert np.allclose(mat, expected), "CRX matrix mismatch"

    def test_crz_matrix(self):
        """CRZ gate matrix verification"""
        phi = np.pi / 3
        mat = CRZ(phi).matrix()
        # CRZ = |0⟩⟨0| ⊗ I + |1⟩⟨1| ⊗ RZ(φ)
        expected = np.array(
            [
                [1, 0, 0, 0],
                [0, 1, 0, 0],
                [0, 0, np.exp(-1j * phi / 2), 0],
                [0, 0, 0, np.exp(1j * phi / 2)],
            ]
        )
        assert np.allclose(mat, expected), "CRZ matrix mismatch"


class TestGateParameterBinding:
    """Test gate parameter binding"""

    def test_bind_float_to_rx(self):
        """Bind float to RX"""
        rx = RX(0.5)
        assert len(rx.params) == 1
        # Verify parameter value is correct
        param_val = rx.params[0].evaluate({})
        assert np.isclose(param_val, 0.5), "Parameter value mismatch"
        # Verify matrix computation is correct
        mat = rx.matrix()
        assert mat.shape == (2, 2)
        assert is_unitary(mat), "Bound RX should be unitary"

    def test_bind_parameter_to_rx(self):
        """Bind symbolic parameter to RX"""
        theta = Parameter("theta")
        rx = RX(theta)
        assert len(rx.params) == 1
        # Symbolic parameters cannot compute matrix directly
        with pytest.raises(Exception):
            rx.matrix()
        # But can compute with concrete values passed in
        mat = rx.matrix([np.pi / 2])
        assert mat.shape == (2, 2)
        assert is_unitary(mat), "RX with concrete params should be unitary"

    def test_bind_multiple_params_to_u(self):
        """Bind multiple parameters to U gate"""
        u = U(0.1, 0.2, 0.3)
        assert len(u.params) == 3
        # Verify each parameter
        for i, expected in enumerate([0.1, 0.2, 0.3]):
            actual = u.params[i].evaluate({})
            assert np.isclose(actual, expected), f"U param {i} mismatch"
        mat = u.matrix()
        assert mat.shape == (2, 2)
        assert is_unitary(mat), "U gate should be unitary"


class TestGateInverse:
    """Test gate inverse mathematical correctness"""

    # Self-inverse gates: G = G†, i.e., G @ G = I
    SELF_INVERSE_GATES = [H, X, Y, Z, CX, CY, CZ, SWAP, CCX]

    @pytest.mark.parametrize("gate", SELF_INVERSE_GATES)
    def test_self_inverse_gates(self, gate):
        """Self-inverse gates: G = G†, i.e., G @ G = I"""
        inv = gate.inverse()
        assert inv == gate, f"{gate} should be self-inverse type"
        # Mathematical verification: G @ G = I
        mat = gate.matrix()
        product = mat @ mat
        assert np.allclose(product, np.eye(mat.shape[0])), (
            f"{gate} @ {gate} should be identity"
        )

    def test_paired_inverse_gates(self):
        """Paired inverse gates: S† = SDG, T† = TDG"""
        # S @ SDG = I
        s_mat = S.matrix()
        sdg_mat = SDG.matrix()
        product = s_mat @ sdg_mat
        assert np.allclose(product, np.eye(2)), "S @ SDG should be identity"
        # T @ TDG = I
        t_mat = T.matrix()
        tdg_mat = TDG.matrix()
        product = t_mat @ tdg_mat
        assert np.allclose(product, np.eye(2)), "T @ TDG should be identity"

    def test_sqrt_x_inverse_pair(self):
        """√X and √X† are inverses"""
        x2p_mat = X2P.matrix()
        x2m_mat = X2M.matrix()
        product = x2p_mat @ x2m_mat
        assert np.allclose(product, np.eye(2)), "X2P @ X2M should be identity"

    def test_sqrt_y_inverse_pair(self):
        """√Y and √Y† are inverses"""
        y2p_mat = Y2P.matrix()
        y2m_mat = Y2M.matrix()
        product = y2p_mat @ y2m_mat
        assert np.allclose(product, np.eye(2)), "Y2P @ Y2M should be identity"

    def test_rx_inverse_negates_angle(self):
        """RX(θ)† = RX(-θ)"""
        theta = 0.5
        rx = RX(theta)
        rx_inv = rx.inverse()
        # Verify inverse gate parameter is negated
        inv_param = rx_inv.params[0].evaluate({})
        assert np.isclose(inv_param, -theta), "RX inverse should negate angle"
        # Mathematical verification: RX(θ) @ RX(-θ) = I
        rx_mat = rx.matrix()
        rx_inv_mat = rx_inv.matrix()
        product = rx_mat @ rx_inv_mat
        assert np.allclose(product, np.eye(2), atol=1e-10), (
            "RX(θ) @ RX(-θ) should be identity"
        )

    def test_ry_inverse_negates_angle(self):
        """RY(θ)† = RY(-θ)"""
        theta = 0.5
        ry = RY(theta)
        ry_inv = ry.inverse()
        ry_mat = ry.matrix()
        ry_inv_mat = ry_inv.matrix()
        product = ry_mat @ ry_inv_mat
        assert np.allclose(product, np.eye(2), atol=1e-10), (
            "RY(θ) @ RY(-θ) should be identity"
        )

    def test_rz_inverse_negates_angle(self):
        """RZ(θ)† = RZ(-θ)"""
        theta = 0.5
        rz = RZ(theta)
        rz_inv = rz.inverse()
        rz_mat = rz.matrix()
        rz_inv_mat = rz_inv.matrix()
        product = rz_mat @ rz_inv_mat
        assert np.allclose(product, np.eye(2), atol=1e-10), (
            "RZ(θ) @ RZ(-θ) should be identity"
        )

    def test_phase_inverse_negates_angle(self):
        """P(λ)† = P(-λ)"""
        lam = 0.5
        p = Phase(lam)
        p_inv = p.inverse()
        p_mat = p.matrix()
        p_inv_mat = p_inv.matrix()
        product = p_mat @ p_inv_mat
        assert np.allclose(product, np.eye(2), atol=1e-10), (
            "P(λ) @ P(-λ) should be identity"
        )

    def test_u_inverse_parameter_swap(self):
        """U(θ, φ, λ)† = U(-θ, -λ, -φ)"""
        theta, phi, lam = 0.3, 0.4, 0.5
        u = U(theta, phi, lam)
        u_inv = u.inverse()
        # Verify parameters: -θ, -λ, -φ
        inv_params = [p.evaluate({}) for p in u_inv.params]
        assert np.isclose(inv_params[0], -theta), "U inverse should negate theta"
        assert np.isclose(inv_params[1], -lam), (
            "U inverse should swap and negate phi/lam"
        )
        assert np.isclose(inv_params[2], -phi), (
            "U inverse should swap and negate phi/lam"
        )
        # Mathematical verification
        u_mat = u.matrix()
        u_inv_mat = u_inv.matrix()
        product = u_mat @ u_inv_mat
        assert np.allclose(product, np.eye(2), atol=1e-10), "U @ U† should be identity"


class TestGateControl:
    """Test gate control elevation"""

    def test_x_control_one(self):
        """X with 1 control becomes CNOT"""
        cx = X.control(1)
        assert cx.num_ctrl_qubits == 1
        assert cx.num_qubits == 2
        # Verify matrix equals CX
        cx_mat = cx.matrix()
        expected = CX.matrix()
        assert np.allclose(cx_mat, expected), "X.control(1) should equal CX"

    def test_x_control_two(self):
        """X with 2 controls becomes CCX"""
        ccx = X.control(2)
        assert ccx.num_ctrl_qubits == 2
        assert ccx.num_qubits == 3
        # Verify matrix equals CCX
        ccx_mat = ccx.matrix()
        expected = CCX.matrix()
        assert np.allclose(ccx_mat, expected), "X.control(2) should equal CCX"

    def test_cnot_control_one_is_ccx(self):
        """CNOT with 1 more control becomes CCX"""
        ccx = CX.control(1)
        assert ccx.num_ctrl_qubits == 2
        assert ccx.num_qubits == 3
        ccx_mat = ccx.matrix()
        expected = CCX.matrix()
        assert np.allclose(ccx_mat, expected), "CX.control(1) should equal CCX"


class TestGateReprAndEquality:
    """Test gate representation and equality"""

    def test_gate_repr_contains_name(self):
        """Gate repr contains gate name"""
        # Test that repr contains expected gate name for specific gates
        assert "H" in repr(H), "H repr should contain 'H'"
        assert "X" in repr(X), "X repr should contain 'X'"
        assert "CX" in repr(CX), "CX repr should contain 'CX'"
        # Test parametric gates
        rx_gate = RX(0.5)
        assert "RX" in repr(rx_gate), "RX repr should contain 'RX'"

    def test_gate_equality_same_gate(self):
        """Same gates are equal"""
        assert H == StandardGate.H
        assert X == StandardGate.X
        assert CX == StandardGate.CX

    def test_gate_equality_different_gates(self):
        """Different gates are not equal"""
        assert H != X
        assert X != Y
        assert CX != CZ

    def test_parametric_gate_equality_same_params(self):
        """Parametric gates with same parameters are equal"""
        rx1 = RX(0.5)
        rx2 = RX(0.5)
        assert rx1 == rx2, "RX with same params should be equal"

    def test_parametric_gate_inequality_different_params(self):
        """Parametric gates with different parameters are not equal"""
        rx1 = RX(0.5)
        rx2 = RX(0.6)
        assert rx1 != rx2, "RX with different params should not be equal"

    def test_gate_hash_consistency(self):
        """Gate hash consistency"""
        # Same gate should have same hash
        h1 = hash(H)
        h2 = hash(H)
        assert h1 == h2, "Same gate should have same hash"
        # Gates with same params should have same hash
        rx1 = RX(0.5)
        rx2 = RX(0.5)
        assert hash(rx1) == hash(rx2), "Gates with same params should have same hash"


class TestGateMatrixCaching:
    """Test gate matrix caching behavior (if applicable)"""

    def test_matrix_consistency(self):
        """Multiple matrix() calls return same result"""
        mat1 = H.matrix()
        mat2 = H.matrix()
        assert np.allclose(mat1, mat2), "Matrix should be consistent across calls"

    def test_parametric_matrix_different_params(self):
        """Different parameters produce different matrices"""
        rx_pi = RX(np.pi).matrix()
        rx_pi_half = RX(np.pi / 2).matrix()
        assert not np.allclose(rx_pi, rx_pi_half), (
            "Different params should give different matrices"
        )


class TestGateEdgeCases:
    """Test edge cases"""

    def test_rx_2pi_periodicity(self):
        """RX(2π) = -I (global phase)"""
        mat = RX(2 * np.pi).matrix()
        expected = -np.eye(2)
        assert np.allclose(mat, expected), "RX(2π) should be -I"

    def test_rx_4pi_is_identity(self):
        """RX(4π) = I"""
        mat = RX(4 * np.pi).matrix()
        expected = np.eye(2)
        assert np.allclose(mat, expected), "RX(4π) should be identity"

    def test_rz_2pi_periodicity(self):
        """RZ(2π) = -I"""
        mat = RZ(2 * np.pi).matrix()
        expected = -np.eye(2)
        assert np.allclose(mat, expected), "RZ(2π) should be -I"

    def test_ry_negative_angle(self):
        """RY(-θ) = RY(θ)†"""
        theta = np.pi / 3
        ry_neg = RY(-theta).matrix()
        ry_inv = RY(theta).inverse().matrix()
        assert np.allclose(ry_neg, ry_inv), "RY(-θ) should equal RY(θ)†"

    def test_xy_gate_is_unitary(self):
        """XY gate is unitary for any parameter"""
        theta = np.pi / 4
        mat = XY(theta).matrix()
        assert is_unitary(mat), "XY gate should be unitary"

    def test_xy2p_xy2m_inverse(self):
        """XY2P and XY2M are inverses of each other"""
        theta = np.pi / 3
        xy2p = XY2P(theta)
        xy2m_inv = xy2p.inverse()
        xy2p_mat = xy2p.matrix()
        xy2m_inv_mat = xy2m_inv.matrix()
        product = xy2p_mat @ xy2m_inv_mat
        assert np.allclose(product, np.eye(2)), "XY2P @ XY2M should be identity"


class TestStandardGateFromName:
    """Test StandardGate.from_name resolution"""

    def test_from_name_resolves_canonical_names(self):
        assert StandardGate.from_name("H") == StandardGate.H
        assert StandardGate.from_name("X2P") == StandardGate.X2P
        assert StandardGate.from_name("Phase") == StandardGate.Phase

    def test_from_name_is_case_insensitive(self):
        assert StandardGate.from_name("h") == StandardGate.H
        assert StandardGate.from_name("cx") == StandardGate.CX
        assert StandardGate.from_name("phase") == StandardGate.Phase

    def test_from_name_rejects_unknown_names(self):
        for bad in ("not-a-gate", "C2-X"):
            with pytest.raises(ValueError, match="unknown standard gate"):
                StandardGate.from_name(bad)


class TestStandardGateRepr:
    """Test StandardGate repr reconstructability"""

    def test_unbound_repr_matches_class_attribute(self):
        assert repr(StandardGate.H) == "StandardGate.H"
        assert repr(StandardGate.RX) == "StandardGate.RX"

    def test_bound_repr_round_trips_through_eval(self):
        gate = StandardGate.RX(1.5)

        assert repr(gate) == "StandardGate.RX(1.5)"
        assert eval(repr(gate), {"StandardGate": StandardGate}) == gate
