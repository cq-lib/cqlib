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
Tests for MCGate (Multi-Controlled Gate).

Test coverage:
- MCGate creation with different control counts
- MCGate properties (num_qubits, num_ctrl_qubits, num_params)
- MCGate matrix computation
- MCGate with different base gates
- MCGate inverse
- MCGate unitary property
"""

import numpy as np
from cqlib.circuit.gates import MCGate, StandardGate, X, H, RX, RZ, Phase, CX
from cqlib.circuit import Parameter


def is_unitary(matrix, atol=1e-10):
    """Verify matrix is unitary: U†U = I"""
    n = matrix.shape[0]
    identity = np.eye(n)
    product = matrix.conj().T @ matrix
    return np.allclose(product, identity, atol=atol)


class TestMcGateCreation:
    """Test MCGate creation"""

    def test_create_single_control_gate(self):
        """Create single-controlled gate"""
        mc_gate = MCGate(1, X)
        assert mc_gate.num_ctrl_qubits == 1
        assert mc_gate.num_qubits == 2
        assert mc_gate.base_gate == X

    def test_create_multi_control_gate(self):
        """Create multi-controlled gate"""
        mc_gate = MCGate(2, X)
        assert mc_gate.num_ctrl_qubits == 2
        assert mc_gate.num_qubits == 3

    def test_create_triple_control_gate(self):
        """Create triple-controlled gate"""
        mc_gate = MCGate(3, X)
        assert mc_gate.num_ctrl_qubits == 3
        assert mc_gate.num_qubits == 4

    def test_mc_gate_with_hadamard(self):
        """Create controlled-Hadamard gate"""
        ch = MCGate(1, H)
        assert ch.num_ctrl_qubits == 1
        assert ch.num_qubits == 2
        assert ch.base_gate == H

    def test_mc_gate_zero_controls(self):
        """MCGate with 0 controls is equivalent to base gate"""
        mc_gate = MCGate(0, X)
        assert mc_gate.num_ctrl_qubits == 0
        assert mc_gate.num_qubits == 1


class TestMcGateProperties:
    """Test MCGate properties"""

    def test_num_params_with_non_parametric_gate(self):
        """MCGate with non-parametric base gate has 0 params"""
        mc_gate = MCGate(2, X)
        assert mc_gate.num_params == 0

    def test_num_params_with_parametric_gate(self):
        """MCGate inherits params from parametric base gate"""
        mc_gate = MCGate(1, RX)
        assert mc_gate.num_params == 1

        mc_gate_rz = MCGate(2, RZ)
        assert mc_gate_rz.num_params == 1

    def test_base_gate_property(self):
        """base_gate property returns the wrapped gate"""
        mc_gate = MCGate(1, Phase)
        assert mc_gate.base_gate == Phase


class TestMcGateMatrix:
    """Test MCGate matrix computation"""

    def test_cnot_matrix(self):
        """CNOT matrix (X with 1 control)"""
        cnot = MCGate(1, X)
        mat = cnot.matrix([])
        expected = np.array(
            [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 0, 1], [0, 0, 1, 0]], dtype=complex
        )
        assert np.allclose(mat, expected), "CNOT matrix mismatch"

    def test_ccx_matrix(self):
        """CCX (Toffoli) matrix (X with 2 controls)"""
        ccx = MCGate(2, X)
        mat = ccx.matrix([])
        # CCX is 8x8 matrix
        assert mat.shape == (8, 8)
        assert is_unitary(mat), "CCX should be unitary"

    def test_controlled_hadamard_matrix(self):
        """Controlled-Hadamard matrix"""
        ch = MCGate(1, H)
        mat = ch.matrix([])
        assert mat.shape == (4, 4)
        assert is_unitary(mat), "CH should be unitary"

    def test_mc_rx_matrix(self):
        """Controlled-RX matrix"""
        crx = MCGate(1, RX)
        theta = np.pi / 2
        mat = crx.matrix([theta])
        assert mat.shape == (4, 4)
        assert is_unitary(mat), "CRX should be unitary"

    def test_mc_rx_matrix_values(self):
        """Controlled-RX matrix values verification"""
        crx = MCGate(1, RX)
        theta = np.pi / 2
        mat = crx.matrix([theta])
        cos_half = np.cos(theta / 2)
        sin_half = np.sin(theta / 2)
        # |00⟩|00⟩, |01⟩|01⟩, |10⟩ stays, |11⟩ applies RX
        expected = np.array(
            [
                [1, 0, 0, 0],
                [0, 1, 0, 0],
                [0, 0, cos_half, -1j * sin_half],
                [0, 0, -1j * sin_half, cos_half],
            ],
            dtype=complex,
        )
        assert np.allclose(mat, expected), "CRX matrix mismatch"


class TestMcGateUnitary:
    """Test MCGate unitary property"""

    def test_mc_x_is_unitary(self):
        """Multi-controlled X is unitary"""
        for n_ctrls in [1, 2, 3]:
            mc_x = MCGate(n_ctrls, X)
            mat = mc_x.matrix([])
            assert is_unitary(mat), f"MC-X with {n_ctrls} controls should be unitary"

    def test_mc_h_is_unitary(self):
        """Multi-controlled H is unitary"""
        mc_h = MCGate(1, H)
        mat = mc_h.matrix([])
        assert is_unitary(mat), "CH should be unitary"

    def test_mc_parametric_is_unitary(self):
        """Multi-controlled parametric gates are unitary"""
        test_cases = [
            (1, RX, [np.pi / 3]),
            (2, RX, [np.pi / 4]),
            (1, RZ, [np.pi / 6]),
        ]
        for n_ctrls, gate, params in test_cases:
            mc_gate = MCGate(n_ctrls, gate)
            mat = mc_gate.matrix(params)
            assert is_unitary(mat), (
                f"MC-{gate} with {n_ctrls} controls should be unitary"
            )


class TestMcGateInverse:
    """Test MCGate inverse"""

    def test_mc_x_inverse(self):
        """Multi-controlled X is self-inverse"""
        ccx = MCGate(2, X)
        inv_gate = ccx.inverse()
        # Inverse should be another MCGate with same controls
        assert inv_gate.num_ctrl_qubits == 2
        # X is self-inverse, so inverse is itself
        assert inv_gate.base_gate == X
        assert inv_gate.params == []

    def test_mc_rx_inverse_negates_angle(self):
        """Controlled-RX inverse negates the angle"""
        crx = MCGate(1, RX(Parameter("theta")))
        inv_gate = crx.inverse()
        assert inv_gate.num_ctrl_qubits == 1
        # Verify parameter is negated
        assert len(inv_gate.params) == 1
        assert str(inv_gate.params[0]) == "-theta"

    def test_mc_h_inverse(self):
        """Controlled-H inverse is itself (H is self-inverse)"""
        ch = MCGate(1, H)
        inv_gate = ch.inverse()
        assert inv_gate.base_gate == H
        assert inv_gate.params == []


class TestMcGateMatrixShape:
    """Test MCGate matrix dimensions"""

    def test_matrix_dimensions(self):
        """Matrix dimensions match num_qubits"""
        test_cases = [
            (1, X, 4),  # 2^2 = 4
            (2, X, 8),  # 2^3 = 8
            (3, X, 16),  # 2^4 = 16
            (1, H, 4),  # 2^2 = 4
        ]
        for n_ctrls, gate, expected_dim in test_cases:
            mc_gate = MCGate(n_ctrls, gate)
            mat = mc_gate.matrix([])
            assert mat.shape == (expected_dim, expected_dim), (
                f"MC-{gate} with {n_ctrls} controls should have {expected_dim}x{expected_dim} matrix"
            )


class TestMcGateEdgeCases:
    """Test MCGate edge cases"""

    def test_mc_gate_with_zero_controls_equals_base(self):
        """MCGate with 0 controls has same matrix as base gate"""
        mc_x = MCGate(0, X)
        mat = mc_x.matrix([])
        base_mat = X.matrix()
        assert np.allclose(mat, base_mat), "MC-X with 0 controls should equal X"

    def test_mc_gate_with_pre_controlled_base(self):
        """MCGate can wrap already-controlled gates"""
        # CX is already controlled, wrapping with 1 more control gives CCX
        ccx_via_cx = MCGate(1, CX)
        assert ccx_via_cx.num_ctrl_qubits == 2
        assert ccx_via_cx.num_qubits == 3


class TestMcGateRepr:
    """Test MCGate repr reconstructability"""

    def test_repr_round_trips_through_eval(self):
        namespace = {"MCGate": MCGate, "StandardGate": StandardGate}

        gate = MCGate(2, X)
        assert repr(gate) == "MCGate(2, StandardGate.X)"
        assert eval(repr(gate), namespace) == gate

    def test_parametric_repr_round_trips_through_eval(self):
        namespace = {"MCGate": MCGate, "StandardGate": StandardGate}

        gate = MCGate(1, RX(1.5))
        assert repr(gate) == "MCGate(1, StandardGate.RX(1.5))"
        rebuilt = eval(repr(gate), namespace)
        assert rebuilt == gate
        assert [str(p) for p in rebuilt.params] == [str(p) for p in gate.params]
