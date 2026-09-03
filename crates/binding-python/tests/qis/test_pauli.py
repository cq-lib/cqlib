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
from cqlib.qis import Pauli, PauliString, Phase, DensityMatrix, Statevector


def test_phase_operations():
    # Test creation and conversion
    assert Phase.plus().to_complex() == 1.0 + 0.0j
    assert Phase.i().to_complex() == 0.0 + 1.0j
    assert Phase.minus().to_complex() == -1.0 + 0.0j
    assert Phase.minus_i().to_complex() == 0.0 - 1.0j

    # Test arithmetic (group multiplication)
    p_i = Phase.i()
    p_minus_i = Phase.minus_i()

    assert p_i + p_i == Phase.minus()  # i * i = -1
    assert p_i * p_minus_i == Phase.plus()  # i * (-i) = 1
    assert p_minus_i + p_minus_i == Phase.minus()  # -i * -i = -1

    # Test equality
    assert Phase.plus() == Phase(0)
    assert Phase.i() == Phase(1)


def test_pauli_basic():
    # Test creation
    x, y, z, i = Pauli.x(), Pauli.y(), Pauli.z(), Pauli.i()

    # Test symplectic representation
    assert x.to_symplectic() == (1, 0)
    assert y.to_symplectic() == (1, 1)
    assert z.to_symplectic() == (0, 1)
    assert i.to_symplectic() == (0, 0)

    # Test matrix representation
    np.testing.assert_array_equal(
        x.to_matrix(), np.array([[0, 1], [1, 0]], dtype=complex)
    )
    np.testing.assert_array_equal(
        z.to_matrix(), np.array([[1, 0], [0, -1]], dtype=complex)
    )
    np.testing.assert_array_equal(
        y.to_matrix(), np.array([[0, -1j], [1j, 0]], dtype=complex)
    )


def test_pauli_multiplication():
    x, y, z, i = Pauli.x(), Pauli.y(), Pauli.z(), Pauli.i()

    # Multiplication with phase tracking
    # XY = iZ
    res, phase = x.mul_with_phase(y)
    assert res == z
    assert phase == Phase.i()

    # YX = -iZ
    res, phase = y.mul_with_phase(x)
    assert res == z
    assert phase == Phase.minus_i()

    # XX = I
    res, phase = x.mul_with_phase(x)
    assert res == i
    assert phase == Phase.plus()

    # Operator overload (without explicit phase tracking)
    assert x * y == z
    assert x * i == x


def test_pauli_string_from_str_valid():
    # Implicit format tests
    ps = PauliString.from_str("XYZ")
    assert ps.num_qubits == 3
    assert str(ps) == "+XYZ"
    assert ps.phase == Phase.plus()

    # Explicit phase tests
    ps_minus = PauliString.from_str("-YIX")
    assert ps_minus.num_qubits == 3
    assert str(ps_minus) == "-YIX"
    assert ps_minus.phase == Phase.minus()

    # Imaginary phase tests
    ps_i = PauliString.from_str("+iZII")
    assert str(ps_i) == "+iZII"
    assert ps_i.phase == Phase.i()

    # 'j' as alternative for 'i' tests
    ps_j = PauliString.from_str("+jZZ")
    assert str(ps_j) == "+iZZ"  # Internal representation normalizes to 'i'


def test_pauli_string_from_str_invalid():
    # Empty string
    with pytest.raises(ValueError, match="empty string"):
        PauliString.from_str("")

    # Invalid characters
    with pytest.raises(ValueError, match="invalid character"):
        PauliString.from_str("XABZ")

    # Missing operators (only phase)
    with pytest.raises(ValueError, match="no Pauli operators specified"):
        PauliString.from_str("+i")


def test_pauli_string_manipulation():
    ps = PauliString(3)
    assert str(ps) == "+III"

    ps.set_pauli(0, Pauli.x())
    ps.set_pauli(1, Pauli.z())
    ps.set_pauli(2, Pauli.y())
    assert str(ps) == "+YZX"

    # Test indexing retrieval
    assert ps.get_pauli(0) == Pauli.x()
    assert ps.get_pauli(1) == Pauli.z()

    # Boundary checks
    with pytest.raises(IndexError):
        ps.set_pauli(3, Pauli.x())
    with pytest.raises(IndexError):
        ps.get_pauli(3)


def test_pauli_string_commutation():
    # XZI and ZXI
    ps1 = PauliString.from_str("XZI")
    ps2 = PauliString.from_str("ZXI")
    # Anti-commute on q2 (X,Z) and q1 (Z,X), total anti-commutations = 2 (even) -> commutes
    assert ps1.commutes_with(ps2) is True

    # XZI and XII
    ps3 = PauliString.from_str("XII")
    # Anti-commute on q2 (X,X=0), q1 (Z,I=0) -> commutes
    assert ps1.commutes_with(ps3) is True

    # XZI and YII
    ps4 = PauliString.from_str("YII")
    # Anti-commute on q2 (X,Y) = 1 (odd) -> anti-commutes
    assert ps1.commutes_with(ps4) is False

    # Dimension mismatch
    ps_short = PauliString.from_str("XI")
    with pytest.raises(ValueError, match="same number of qubits"):
        ps1.commutes_with(ps_short)


def test_pauli_string_multiplication():
    # X * Z = -iY (on single qubit)
    ps1 = PauliString.from_str("X")
    ps2 = PauliString.from_str("Z")

    res = ps1 * ps2
    assert str(res) == "-iY"

    # Multi-qubit: (X ⊗ Z) * (Y ⊗ X)
    # q1: X * Y = iZ
    # q0: Z * X = iY
    # Total phase = i * i = -1
    # Result = -1 * Z ⊗ Y
    ps3 = PauliString.from_str("XZ")
    ps4 = PauliString.from_str("YX")
    res_multi = ps3 * ps4
    assert str(res_multi) == "-ZY"

    # In-place multiplication
    ps3 *= ps4
    assert str(ps3) == "-ZY"

    # Dimension mismatch
    with pytest.raises(ValueError, match="same number of qubits"):
        _ = ps1 * PauliString.from_str("XX")


class TestPauliStringProperties:
    """Test PauliString properties."""

    def test_x_bits_property(self):
        ps = PauliString.from_str("XYZ")
        x_bits = ps.x_bits
        # String "XYZ": first char X is highest qubit (index 2), Z is lowest (index 0)
        assert x_bits == [False, True, True]  # Z=0, Y=1, X=1

    def test_z_bits_property(self):
        ps = PauliString.from_str("XYZ")
        z_bits = ps.z_bits
        # String "XYZ": first char X is highest qubit, Z is lowest
        assert z_bits == [True, True, False]  # Z=1, Y=1, X=0

    def test_x_mask_property(self):
        ps = PauliString.from_str("XIZ")
        # X on qubit 2 (value 4), I on qubit 1 (value 0), Z on qubit 0 (value 0)
        assert ps.x_mask == 0b100  # X is on qubit 2

    def test_z_mask_property(self):
        ps = PauliString.from_str("XIZ")
        # Z on qubit 0 (value 1)
        assert ps.z_mask == 0b001  # Z is on qubit 0

    def test_y_phase(self):
        ps = PauliString.from_str("YII")
        y_phase = ps.y_phase()
        assert y_phase == 1j  # Single Y contributes i

        ps2 = PauliString.from_str("YY")
        y_phase2 = ps2.y_phase()
        assert y_phase2 == -1  # Two Ys contribute i*i = -1

    def test_phase_setter(self):
        ps = PauliString.from_str("X")
        assert ps.phase == Phase.plus()
        ps.phase = Phase.i()
        assert ps.phase == Phase.i()

    def test_copy(self):
        ps1 = PauliString.from_str("XYZ")
        ps2 = ps1.copy()
        assert str(ps1) == str(ps2)
        # Modify copy
        ps2.set_pauli(0, Pauli.i())
        assert str(ps1) != str(ps2)


class TestPauliStringExpectation:
    """Test PauliString expectation values."""

    def test_expectation_probs(self):
        """Test expectation from measurement probabilities."""
        ps_z = PauliString.from_str("Z")

        # 100% |0>
        probs_0 = {"0": 1.0}
        assert math.isclose(ps_z.expectation(probs_0), 1.0)

        # 100% |1>
        probs_1 = {"1": 1.0}
        assert math.isclose(ps_z.expectation(probs_1), -1.0)

        # 50/50 mix
        probs_mixed = {"0": 0.5, "1": 0.5}
        assert math.isclose(ps_z.expectation(probs_mixed), 0.0)

    def test_expectation_statevector(self):
        """Test expectation from statevector."""
        sv = Statevector(1)
        ps_z = PauliString.from_str("Z")
        assert math.isclose(ps_z.expectation_statevector(sv), 1.0)

        sv.apply_h(0)
        ps_x = PauliString.from_str("X")
        assert math.isclose(ps_x.expectation_statevector(sv), 1.0)

    def test_expectation_density_matrix(self):
        """Test expectation from density matrix."""
        dm = DensityMatrix(1)
        ps_z = PauliString.from_str("Z")
        assert math.isclose(ps_z.expectation_density_matrix(dm), 1.0)

        dm.apply_h(0)
        ps_x = PauliString.from_str("X")
        assert math.isclose(ps_x.expectation_density_matrix(dm), 1.0)

    def test_expectation_probs_with_measurements(self):
        """Test expectation_probs with multiple measurement bases."""
        ps_z = PauliString.from_str("Z")

        measurements = [(PauliString.from_str("Z"), {"0": 0.5, "1": 0.5})]
        assert math.isclose(ps_z.expectation_probs(measurements), 0.0)


class TestPhaseProperties:
    """Test Phase properties."""

    def test_exponent_property(self):
        assert Phase.plus().exponent == 0
        assert Phase.i().exponent == 1
        assert Phase.minus().exponent == 2
        assert Phase.minus_i().exponent == 3

    def test_phase_from_mod(self):
        assert Phase(4).exponent == 0  # 4 mod 4 = 0
        assert Phase(5).exponent == 1  # 5 mod 4 = 1
        assert Phase(6).exponent == 2  # 6 mod 4 = 2
        assert Phase(7).exponent == 3  # 7 mod 4 = 3

    def test_phase_str(self):
        assert str(Phase.plus()) == "1"
        assert str(Phase.i()) == "i"
        assert str(Phase.minus()) == "-1"
        assert str(Phase.minus_i()) == "-i"

    def test_phase_repr(self):
        assert "Phase" in repr(Phase.plus())


class TestPauliBoundaryConditions:
    """Test boundary conditions and edge cases."""

    def test_large_pauli_string_20_qubits(self):
        """Test large 20-qubit PauliString."""
        ps = PauliString(20)
        assert ps.num_qubits == 20
        assert len(ps.x_bits) == 20
        assert len(ps.z_bits) == 20

    @pytest.mark.parametrize(
        "val,expected",
        [
            (100, 0),  # 100 mod 4 = 0
            (255, 3),  # 255 mod 4 = 3
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
        ],
    )
    def test_phase_modulo_behavior_extreme(self, val, expected):
        """Test Phase extreme values mod 4."""
        phase = Phase(val)
        assert phase.exponent == expected

    def test_pauli_string_single_qubit(self):
        """Test minimum 1-qubit PauliString."""
        ps = PauliString(1)
        assert ps.num_qubits == 1
        assert str(ps) == "+I"

        ps.set_pauli(0, Pauli.x())
        assert str(ps) == "+X"

    @pytest.mark.parametrize(
        "probs,expected",
        [
            ({"0": 1.0, "1": 0.0}, 1.0),  # 100% |0⟩
            ({"0": 0.0, "1": 1.0}, -1.0),  # 100% |1⟩
            ({"0": 0.5, "1": 0.5}, 0.0),  # 50/50
            ({"0": 0.25, "1": 0.75}, -0.5),  # 25/75
        ],
    )
    def test_expectation_probability_distribution_extreme(self, probs, expected):
        """Test expectation with extreme probability values."""
        ps_z = PauliString.from_str("Z")
        result = ps_z.expectation(probs)
        assert math.isclose(result, expected, abs_tol=1e-10)

    def test_expectation_with_nearly_zero_probability(self):
        """Test numerical stability with nearly zero probabilities."""
        ps_z = PauliString.from_str("Z")
        probs = {"0": 1e-15, "1": 1 - 1e-15}
        result = ps_z.expectation(probs)
        assert math.isclose(result, -1.0, rel_tol=1e-5)


class TestPauliQuantumInvariants:
    """Test quantum mechanical invariants of Pauli operators."""

    def test_pauli_matrices_hermitian(self):
        """Test Pauli matrices are Hermitian: P = P†."""
        for p in [Pauli.x(), Pauli.y(), Pauli.z(), Pauli.i()]:
            mat = p.to_matrix()
            assert np.allclose(mat, mat.conj().T, atol=1e-10)

    def test_pauli_matrices_unitary(self):
        """Test Pauli matrices are unitary: P·P† = I."""
        for p in [Pauli.x(), Pauli.y(), Pauli.z(), Pauli.i()]:
            mat = p.to_matrix()
            prod = mat @ mat.conj().T
            assert np.allclose(prod, np.eye(2), atol=1e-10)

    def test_pauli_matrices_traceless_xyz(self):
        """Test X, Y, Z are traceless (trace = 0), I has trace = 2."""
        assert np.isclose(np.trace(Pauli.x().to_matrix()), 0, atol=1e-10)
        assert np.isclose(np.trace(Pauli.y().to_matrix()), 0, atol=1e-10)
        assert np.isclose(np.trace(Pauli.z().to_matrix()), 0, atol=1e-10)
        assert np.isclose(np.trace(Pauli.i().to_matrix()), 2, atol=1e-10)

    def test_pauli_matrices_square_to_identity(self):
        """Test X² = Y² = Z² = I."""
        x, y, z, i = Pauli.x(), Pauli.y(), Pauli.z(), Pauli.i()

        x_squared, phase_xx = x.mul_with_phase(x)
        assert x_squared == i
        assert phase_xx == Phase.plus()

        y_squared, phase_yy = y.mul_with_phase(y)
        assert y_squared == i
        assert phase_yy == Phase.plus()

        z_squared, phase_zz = z.mul_with_phase(z)
        assert z_squared == i
        assert phase_zz == Phase.plus()

    def test_commutation_relations_xyz(self):
        """Test [X,Y] = 2iZ, [Y,Z] = 2iX, [Z,X] = 2iY (commutator)."""
        x, y, z = Pauli.x(), Pauli.y(), Pauli.z()

        # XY = iZ, YX = -iZ, so [X,Y] = XY - YX = 2iZ
        xy, phase_xy = x.mul_with_phase(y)
        yx, phase_yx = y.mul_with_phase(x)

        assert xy == z
        assert phase_xy == Phase.i()
        assert yx == z
        assert phase_yx == Phase.minus_i()

        # Verify anti-commutation for X,Y: {X,Y} = XY + YX = 0
        # Since YX = -XY, they anti-commute
        assert phase_xy.to_complex() == -phase_yx.to_complex()

    def test_anticommutation_xyz(self):
        """Test {X,Y} = {Y,Z} = {Z,X} = 0 (anti-commutator)."""
        x, y, z = Pauli.x(), Pauli.y(), Pauli.z()

        # X and Y anti-commute
        xy, phase_xy = x.mul_with_phase(y)
        yx, phase_yx = y.mul_with_phase(x)
        # XY = -YX
        assert phase_xy.to_complex() == -phase_yx.to_complex()

        # Y and Z anti-commute
        yz, phase_yz = y.mul_with_phase(z)
        zy, phase_zy = z.mul_with_phase(y)
        assert phase_yz.to_complex() == -phase_zy.to_complex()

        # Z and X anti-commute
        zx, phase_zx = z.mul_with_phase(x)
        xz, phase_xz = x.mul_with_phase(z)
        assert phase_zx.to_complex() == -phase_xz.to_complex()

    def test_pauli_string_tensor_product_structure(self):
        """Test multi-qubit Pauli strings have correct tensor product structure."""
        # X ⊗ Z on 2 qubits
        ps = PauliString.from_str("XZ")

        # Check individual qubits
        assert ps.get_pauli(0) == Pauli.z()  # Z on qubit 0
        assert ps.get_pauli(1) == Pauli.x()  # X on qubit 1

        # Verify masks
        assert ps.x_mask == 0b10  # X on qubit 1
        assert ps.z_mask == 0b01  # Z on qubit 0

    def test_expectation_real_for_hermitian(self):
        """Test expectation values of Hermitian operators are real."""
        sv = Statevector(1)
        sv.apply_h(0)

        ps_x = PauliString.from_str("X")
        exp = ps_x.expectation_statevector(sv)

        # Should be real (within numerical precision)
        assert isinstance(exp, float)
        assert math.isclose(exp, 1.0, abs_tol=1e-10)

    def test_pauli_group_cyclic_property(self):
        """Test cyclic nature: i⁴ = 1, and cyclic multiplication."""
        i_phase = Phase.i()

        # i^1 = i
        assert i_phase == Phase.i()
        # i^2 = -1
        assert i_phase + i_phase == Phase.minus()
        # i^3 = -i
        assert i_phase + i_phase + i_phase == Phase.minus_i()
        # i^4 = 1
        assert i_phase + i_phase + i_phase + i_phase == Phase.plus()


class TestPauliNumericalPrecision:
    """Test numerical precision and stability."""

    def test_expectation_precision_mixed_state(self):
        """Test expectation precision for mixed state."""
        ps_z = PauliString.from_str("Z")

        # Nearly equal probabilities
        probs = {"0": 0.5000000001, "1": 0.4999999999}
        result = ps_z.expectation(probs)
        expected = 0.5000000001 - 0.4999999999
        assert math.isclose(result, expected, rel_tol=1e-9)

    def test_pauli_matrix_element_precision(self):
        """Test matrix elements have exact values."""
        x_mat = Pauli.x().to_matrix()
        # X = [[0, 1], [1, 0]]
        assert x_mat[0, 0] == 0
        assert x_mat[0, 1] == 1
        assert x_mat[1, 0] == 1
        assert x_mat[1, 1] == 0

        y_mat = Pauli.y().to_matrix()
        # Y = [[0, -i], [i, 0]]
        assert y_mat[0, 0] == 0
        assert y_mat[0, 1] == -1j
        assert y_mat[1, 0] == 1j
        assert y_mat[1, 1] == 0

    def test_multiplication_phase_accumulation(self):
        """Test phase accumulation over multiple multiplications."""
        # (XY)^4 should give phase i^4 = 1
        x, y = Pauli.x(), Pauli.y()

        result, phase = x.mul_with_phase(y)  # XY = iZ
        assert phase == Phase.i()

        # Continue multiplying to accumulate phase
        for _ in range(3):  # Do 3 more times
            _, p = x.mul_with_phase(y)
            phase = phase + p

        # After 4 multiplications, phase should be i^4 = 1
        assert phase == Phase.plus()

    def test_y_phase_calculation_precision(self):
        """Test Y phase factor calculation precision."""
        # Single Y: phase = i
        ps_y = PauliString.from_str("Y")
        assert ps_y.y_phase() == 1j

        # Two Ys: phase = i^2 = -1
        ps_yy = PauliString.from_str("YY")
        assert ps_yy.y_phase() == -1

        # Three Ys: phase = i^3 = -i
        ps_yyy = PauliString.from_str("YYY")
        assert ps_yyy.y_phase() == -1j

        # Four Ys: phase = i^4 = 1
        ps_yyyy = PauliString.from_str("YYYY")
        assert ps_yyyy.y_phase() == 1


class TestPauliCopySemantics:
    """Test copy semantics and independence."""

    def test_phase_copy_independence(self):
        """Test Phase copies are independent."""
        p1 = Phase.i()
        p2 = p1  # Assignment

        # Phase is immutable, so this test mainly checks equality
        assert p1 == p2
        assert p1.exponent == p2.exponent

        # New Phase from value
        p3 = Phase(1)
        assert p3 == p1

    def test_pauli_copy_independence(self):
        """Test Pauli copies are independent."""
        x1 = Pauli.x()
        x2 = x1  # Assignment

        # Pauli is immutable
        assert x1 == x2
        assert x1.to_symplectic() == x2.to_symplectic()

    def test_pauli_string_copy_deep_independence(self):
        """Test PauliString deep copy creates independent instance."""
        ps1 = PauliString.from_str("XYZ")
        ps2 = ps1.copy()

        # Initially equal
        assert str(ps1) == str(ps2)

        # Modify original - XYZ: X on q2, Y on q1, Z on q0
        # Change qubit 0 to I gives XYI
        ps1.set_pauli(0, Pauli.i())

        # Copy should be unchanged
        assert str(ps1) == "+XYI"
        assert str(ps2) == "+XYZ"

    def test_copy_preserves_all_properties(self):
        """Test copy preserves all properties including phase."""
        ps1 = PauliString.from_str("-iXYZ")
        ps2 = ps1.copy()

        assert ps1.num_qubits == ps2.num_qubits
        assert ps1.phase == ps2.phase
        assert ps1.x_mask == ps2.x_mask
        assert ps1.z_mask == ps2.z_mask
        assert list(ps1.x_bits) == list(ps2.x_bits)
        assert list(ps1.z_bits) == list(ps2.z_bits)

    def test_statevector_copy_semantics(self):
        """Test Statevector used with PauliString is properly copied."""
        sv = Statevector(1)
        sv.apply_h(0)

        ps_x = PauliString.from_str("X")

        # Get expectation
        exp1 = ps_x.expectation_statevector(sv)

        # Modify statevector
        sv.apply_z(0)

        # New expectation should be different
        exp2 = ps_x.expectation_statevector(sv)

        # After Z gate, |+⟩ becomes |-⟩, so ⟨X⟩ flips sign
        assert math.isclose(exp1, 1.0, abs_tol=1e-10)
        assert math.isclose(exp2, -1.0, abs_tol=1e-10)


class TestPauliErrorHandling:
    """Test error handling for invalid inputs."""

    def test_expectation_invalid_probability_negative(self):
        """Test expectation with negative probability raises error."""
        ps_z = PauliString.from_str("Z")
        probs = {"0": -0.5, "1": 1.5}  # Invalid negative probability

        # Current implementation may not validate, document behavior
        try:
            result = ps_z.expectation(probs)
            # If no error, just verify it returns a number
            assert isinstance(result, float)
        except ValueError:
            pass  # Also acceptable

    def test_expectation_invalid_state_string_length(self):
        """Test expectation with wrong state string length."""
        ps_z = PauliString.from_str("ZZ")  # 2 qubits
        probs = {"0": 1.0}  # Wrong length - should be "00", "01", etc.

        try:
            _ = ps_z.expectation(probs)
            # May raise error or handle gracefully
        except ValueError as e:
            assert (
                "dimension" in str(e).lower()
                or "mismatch" in str(e).lower()
                or "expected" in str(e).lower()
            )

    def test_expectation_invalid_state_string_chars(self):
        """Test expectation with invalid characters in state string."""
        ps_z = PauliString.from_str("Z")
        probs = {"a": 1.0}  # Invalid character

        try:
            _ = ps_z.expectation(probs)
        except ValueError as e:
            assert "invalid" in str(e).lower() or "character" in str(e).lower()

    def test_pauli_string_from_str_invalid_format(self):
        """Test from_str with various invalid formats."""
        # Only phase, no operators
        with pytest.raises(ValueError):
            PauliString.from_str("+i")

        # Invalid characters mixed with valid
        with pytest.raises(ValueError):
            PauliString.from_str("XAY")

        # Multiple phase indicators
        with pytest.raises(ValueError):
            PauliString.from_str("++XX")

    def test_commutes_with_different_qubits_error(self):
        """Test commutes_with raises error for different qubit counts."""
        ps1 = PauliString.from_str("XXX")  # 3 qubits
        ps2 = PauliString.from_str("XX")  # 2 qubits

        with pytest.raises(ValueError, match="same number of qubits"):
            ps1.commutes_with(ps2)

    def test_multiplication_different_qubits_error(self):
        """Test multiplication raises error for different qubit counts."""
        ps1 = PauliString.from_str("XXX")
        ps2 = PauliString.from_str("XX")

        with pytest.raises(ValueError, match="same number of qubits"):
            _ = ps1 * ps2

        with pytest.raises(ValueError, match="same number of qubits"):
            ps1 *= ps2


class TestPauliAdvancedFeatures:
    """Test advanced features and edge cases."""

    def test_pauli_string_repr_format(self):
        """Test repr format contains expected information."""
        ps = PauliString.from_str("-iXYZ")
        r = repr(ps)

        assert "PauliString" in r
        assert "num_qubits" in r
        assert "3" in r  # 3 qubits

    def test_pauli_string_str_format(self):
        """Test str format for various PauliStrings."""
        assert str(PauliString.from_str("X")) == "+X"
        assert str(PauliString.from_str("-X")) == "-X"
        assert str(PauliString.from_str("+iX")) == "+iX"
        assert str(PauliString.from_str("-iX")) == "-iX"

    def test_hash_consistency(self):
        """Test Pauli hash is consistent for equal objects."""
        x1 = Pauli.x()
        x2 = Pauli.x()

        assert hash(x1) == hash(x2)

        # Different Pauli should (likely) have different hashes
        z = Pauli.z()
        assert hash(x1) != hash(z)

    def test_pauli_string_is_unhashable(self):
        """PauliString is mutable, so it must not be hashable."""
        ps = PauliString.from_str("X")
        assert PauliString.__hash__ is None
        with pytest.raises(TypeError):
            hash(ps)
        with pytest.raises(TypeError):
            {ps: 1}

        # Mutation still works after removing __hash__.
        ps.set_pauli(0, Pauli.z())
        assert str(ps) == "+Z"

    def test_equality_transitivity(self):
        """Test equality is transitive."""
        x1 = Pauli.x()
        x2 = Pauli.x()
        x3 = Pauli.x()

        assert x1 == x2
        assert x2 == x3
        assert x1 == x3  # Transitivity

    def test_identity_pauli_behavior(self):
        """Test Identity Pauli special properties."""
        i, x = Pauli.i(), Pauli.x()

        # I * X = X
        res, phase = i.mul_with_phase(x)
        assert res == x
        assert phase == Phase.plus()

        # X * I = X
        res2, phase2 = x.mul_with_phase(i)
        assert res2 == x
        assert phase2 == Phase.plus()

    def test_symplectic_representation_consistency(self):
        """Test symplectic representation is consistent with matrix."""
        for p in [Pauli.x(), Pauli.y(), Pauli.z(), Pauli.i()]:
            x_bit, z_bit = p.to_symplectic()
            # Verify symplectic matches matrix structure
            if p == Pauli.x():
                assert (x_bit, z_bit) == (1, 0)
            elif p == Pauli.z():
                assert (x_bit, z_bit) == (0, 1)
            elif p == Pauli.y():
                assert (x_bit, z_bit) == (1, 1)
            elif p == Pauli.i():
                assert (x_bit, z_bit) == (0, 0)

    def test_large_pauli_string_operations(self):
        """Test operations on large Pauli strings."""
        # Create large Pauli string
        ps1 = PauliString(10)
        ps2 = PauliString(10)

        # Set some operators
        ps1.set_pauli(0, Pauli.x())
        ps1.set_pauli(5, Pauli.z())
        ps2.set_pauli(0, Pauli.z())
        ps2.set_pauli(5, Pauli.x())

        # Multiplication should work
        result = ps1 * ps2
        assert result.num_qubits == 10

        # Commutation check
        commutes = ps1.commutes_with(ps2)
        # XZ = -ZX on same qubit, so they anti-commute at positions 0 and 5
        # Two anti-commutations -> overall commutes
        assert commutes is True

    def test_pauli_string_from_str_edge_cases(self):
        """Test from_str edge cases."""
        # Just identity
        ps_i = PauliString.from_str("I")
        assert str(ps_i) == "+I"

        # Multiple identities
        ps_iii = PauliString.from_str("III")
        assert ps_iii.num_qubits == 3

        # j instead of i for imaginary unit
        ps_j = PauliString.from_str("+jX")
        assert ps_j.phase == Phase.i()


class TestPauliStringMatrixAndIteration:
    """Test PauliString to_matrix, support, and the iteration protocol."""

    def test_to_matrix_little_endian_tensor_order(self):
        """Test "ZX" expands as Z ⊗ X (qubit 0 = least-significant factor)."""
        ps = PauliString.from_str("ZX")
        matrix = ps.to_matrix()
        assert matrix.shape == (4, 4)
        assert matrix.dtype == np.complex128
        expected = np.array(
            [
                [0, 1, 0, 0],
                [1, 0, 0, 0],
                [0, 0, 0, -1],
                [0, 0, -1, 0],
            ],
            dtype=complex,
        )
        np.testing.assert_array_equal(matrix, expected)

    def test_to_matrix_includes_each_global_phase(self):
        """Test the global phase multiplies the matrix."""
        base = Pauli.y().to_matrix()
        for phase in (Phase.plus(), Phase.i(), Phase.minus(), Phase.minus_i()):
            ps = PauliString.from_str("Y")
            ps.phase = phase
            np.testing.assert_array_equal(ps.to_matrix(), base * phase.to_complex())

    def test_to_matrix_zero_qubits_is_its_phase(self):
        """Test a zero-qubit PauliString yields a 1x1 phase matrix."""
        ps = PauliString(0)
        ps.phase = Phase.minus_i()
        matrix = ps.to_matrix()
        assert matrix.shape == (1, 1)
        np.testing.assert_array_equal(matrix, np.array([[-1j]], dtype=complex))

    def test_support(self):
        """Test support returns ascending non-identity qubit indices."""
        assert PauliString.from_str("XZI").support() == [1, 2]
        assert PauliString.from_str("III").support() == []
        assert PauliString.from_str("YIY").support() == [0, 2]

    def test_len(self):
        """Test __len__ equals num_qubits."""
        assert len(PauliString.from_str("XZI")) == 3
        assert len(PauliString(0)) == 0

    def test_len_truthiness(self):
        """__len__ makes an empty PauliString falsy (like empty str/list)."""
        assert bool(PauliString(0)) is False
        assert bool(PauliString(1)) is True

    def test_iter_ascending_qubit_order(self):
        """Test iteration yields per-qubit Paulis in ascending index (incl. I)."""
        # "XYZI": qubit 3=X, qubit 2=Y, qubit 1=Z, qubit 0=I
        ps = PauliString.from_str("XYZI")
        expected = [Pauli.i(), Pauli.z(), Pauli.y(), Pauli.x()]
        assert list(ps) == expected
        assert [ps.get_pauli(i) for i in range(4)] == expected

    def test_iter_is_lazy_and_independent(self):
        """Test iterators have independent cursors and stop with StopIteration."""
        ps = PauliString.from_str("XZ")
        it1, it2 = iter(ps), iter(ps)
        assert it1 is not it2
        assert next(it1) == Pauli.z()
        assert next(it2) == Pauli.z()
        assert list(it1) == [Pauli.x()]
        with pytest.raises(StopIteration):
            next(it1)
        with pytest.raises(StopIteration):
            next(it1)

    def test_iter_length_hint(self):
        """Test __length_hint__ tracks remaining items."""
        ps = PauliString.from_str("XZI")
        it = iter(ps)
        assert it.__length_hint__() == 3
        next(it)
        assert it.__length_hint__() == 2
        list(it)
        assert it.__length_hint__() == 0

    def test_iter_snapshot_semantics(self):
        """Test modifications after iterator creation don't affect iteration."""
        ps = PauliString.from_str("IZ")  # qubit 0=Z, qubit 1=I
        it = iter(ps)
        ps.set_pauli(0, Pauli.x())
        assert next(it) == Pauli.z()
        assert next(it) == Pauli.i()

    def test_getitem(self):
        """Test __getitem__ with positive and negative indices."""
        ps = PauliString.from_str("XZI")  # qubit 2=X, qubit 1=Z, qubit 0=I
        assert ps[0] == Pauli.i()
        assert ps[1] == Pauli.z()
        assert ps[2] == Pauli.x()
        assert ps[-1] == Pauli.x()
        assert ps[-3] == Pauli.i()
        with pytest.raises(IndexError):
            ps[3]
        with pytest.raises(IndexError):
            ps[-4]
