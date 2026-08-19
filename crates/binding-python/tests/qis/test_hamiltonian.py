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
from cqlib.qis import Hamiltonian, PauliString, Phase, TrotterMode


def test_hamiltonian_initialization():
    h = Hamiltonian(2)
    assert h.num_qubits == 2
    assert h.num_terms == 0

    # From Pauli
    h_pauli = Hamiltonian.from_pauli(PauliString.from_str("ZZ"))
    assert h_pauli.num_qubits == 2
    assert h_pauli.num_terms == 1

    # From List
    terms = [
        (PauliString.from_str("XX"), 0.5),
        (PauliString.from_str("YY"), -0.3 + 0.0j),  # Support complex input
        (PauliString.from_str("ZZ"), (0.0, 0.2)),  # Tuple (re, im) support
    ]
    h_list = Hamiltonian.from_list(terms)
    assert h_list.num_qubits == 2
    assert h_list.num_terms == 3

    # Dimension mismatch
    bad_terms = [(PauliString.from_str("X"), 1.0)]
    with pytest.raises(ValueError):
        Hamiltonian.from_list(bad_terms + terms)


def test_hamiltonian_add_term():
    h = Hamiltonian(3)
    h.add_term(PauliString.from_str("XZI"), 1.5)
    assert h.num_terms == 1

    # Add mismatched dimension
    with pytest.raises(ValueError):
        h.add_term(PauliString.from_str("XX"), 0.5)


def test_hamiltonian_scale():
    h = Hamiltonian(1)
    h.add_term(PauliString.from_str("X"), 1.0)

    h.scale(2.5)
    terms = h.terms
    assert math.isclose(terms[0][1].real, 2.5)

    h.scale(1j)
    terms = h.terms
    assert math.isclose(terms[0][1].imag, 2.5)
    assert math.isclose(terms[0][1].real, 0.0)


def test_hamiltonian_simplify():
    h = Hamiltonian(2)

    # Add duplicate terms
    h.add_term(PauliString.from_str("XX"), 0.5)
    h.add_term(PauliString.from_str("XX"), 0.3)

    # Add terms that cancel out
    h.add_term(PauliString.from_str("ZZ"), 1.0)
    h.add_term(PauliString.from_str("ZZ"), -1.0)

    # Add term with internal phase: iY
    # Internally iY * 0.5 -> Y * 0.5j
    h.add_term(PauliString.from_str("+iYY"), 0.5)

    assert h.num_terms == 5
    h.simplify()

    # After simplify:
    # XX should have coeff 0.8
    # ZZ should be removed (coeff 0.0)
    # YY should have coeff 0.5j
    assert h.num_terms == 2
    terms = h.terms

    # Terms might be sorted, let's find them
    xx_term = next(t for t in terms if str(t[0]) == "+XX")
    yy_term = next(t for t in terms if str(t[0]) == "+YY")

    assert math.isclose(xx_term[1].real, 0.8)
    assert math.isclose(xx_term[1].imag, 0.0)

    assert math.isclose(yy_term[1].real, 0.0)
    assert math.isclose(yy_term[1].imag, 0.5)


def test_hamiltonian_addition():
    h1 = Hamiltonian(2)
    h1.add_term(PauliString.from_str("XX"), 1.0)

    h2 = Hamiltonian(2)
    h2.add_term(PauliString.from_str("ZZ"), 0.5)

    h3 = h1 + h2
    assert h3.num_terms == 2

    # In-place add
    h1 += h2
    assert h1.num_terms == 2

    # Different dimensions
    h_bad = Hamiltonian(3)
    with pytest.raises(ValueError):
        _ = h1 + h_bad
    with pytest.raises(ValueError):
        h1 += h_bad


def test_trotter_circuit():
    h = Hamiltonian(2)
    h.add_term(PauliString.from_str("ZZ"), 0.5)
    h.add_term(PauliString.from_str("XX"), 0.3)

    # First Order: 1 step contains 2 terms, each term generates rotation gates
    # A single Pauli rotation generally requires CNOTs and an RZ.
    # We just ensure the circuit generates and has expected structure.
    circ = h.to_trotter_circuit(1.0, 10, TrotterMode.first_order())
    assert circ.num_qubits == 2
    assert len(circ) > 0

    # Non-hermitian case should raise error
    h_non_herm = Hamiltonian(2)
    h_non_herm.add_term(
        PauliString.from_str("+iXX"), 1.0
    )  # After simplify, coeff is 1.0j
    with pytest.raises(ValueError, match="Hermitian"):
        h_non_herm.to_trotter_circuit(1.0, 1, TrotterMode.first_order())


def test_hamiltonian_all_terms_commute():
    h = Hamiltonian(2)
    h.add_term(PauliString.from_str("ZZ"), 0.5)
    h.add_term(PauliString.from_str("IZ"), 0.3)
    assert h.all_terms_commute() is True

    h_non_commuting = Hamiltonian(1)
    h_non_commuting.add_term(PauliString.from_str("X"), 1.0)
    h_non_commuting.add_term(PauliString.from_str("Z"), 1.0)
    assert h_non_commuting.all_terms_commute() is False


def test_hamiltonian_to_evolution_circuit():
    h_commuting = Hamiltonian(2)
    h_commuting.add_term(PauliString.from_str("ZZ"), 0.5)
    h_commuting.add_term(PauliString.from_str("IZ"), 0.3)

    exact = h_commuting.to_evolution_circuit(1.0, 1, TrotterMode.first_order())
    assert exact.num_qubits == 2
    assert len(exact) > 0

    h_non_commuting = Hamiltonian(1)
    h_non_commuting.add_term(PauliString.from_str("X"), 1.0)
    h_non_commuting.add_term(PauliString.from_str("Z"), 1.0)

    trotterized = h_non_commuting.to_evolution_circuit(
        0.5, 4, TrotterMode.second_order()
    )
    assert trotterized.num_qubits == 1
    assert len(trotterized) > 0


def test_hamiltonian_to_evolution_circuit_rejects_invalid_inputs():
    h = Hamiltonian(1)

    with pytest.raises(ValueError):
        h.to_evolution_circuit(1.0, 1, TrotterMode.first_order())

    h.add_term(PauliString.from_str("X"), 1.0)
    with pytest.raises(ValueError, match="greater than 0"):
        h.to_evolution_circuit(1.0, 0, TrotterMode.first_order())

    h_non_herm = Hamiltonian(1)
    h_non_herm.add_term(PauliString.from_str("+iX"), 1.0)
    with pytest.raises(ValueError, match="Hermitian"):
        h_non_herm.to_evolution_circuit(1.0, 1, TrotterMode.first_order())


def test_hamiltonian_variance_statevector():
    from cqlib.qis import Statevector

    h_z = Hamiltonian.from_pauli(PauliString.from_str("Z"))

    sv_zero = Statevector(1)
    assert math.isclose(h_z.variance_statevector(sv_zero), 0.0, abs_tol=1e-10)

    sv_plus = Statevector(1)
    sv_plus.apply_h(0)
    assert math.isclose(h_z.variance_statevector(sv_plus), 1.0, abs_tol=1e-10)

    h_non_herm = Hamiltonian.from_pauli(PauliString.from_str("+iZ"))
    with pytest.raises(ValueError, match="Hermitian"):
        h_non_herm.variance_statevector(sv_zero)


def test_hamiltonian_terms_property():
    """Test terms property."""
    h = Hamiltonian(2)
    h.add_term(PauliString.from_str("ZZ"), 0.5)
    h.add_term(PauliString.from_str("XX"), 0.3)

    terms = h.terms
    assert len(terms) == 2
    # Each term is a tuple (PauliString, complex_coeff)
    assert isinstance(terms[0][0], PauliString)
    assert isinstance(terms[0][1], complex)


def test_hamiltonian_str_repr():
    """Test string representations."""
    h = Hamiltonian(2)
    h.add_term(PauliString.from_str("ZZ"), 0.5)

    str_val = str(h)
    assert isinstance(str_val, str)

    repr_val = repr(h)
    assert "Hamiltonian" in repr_val
    assert "num_qubits=2" in repr_val or "2" in repr_val


def test_hamiltonian_equality():
    """Test Hamiltonian equality."""
    h1 = Hamiltonian(2)
    h1.add_term(PauliString.from_str("ZZ"), 0.5)

    h2 = Hamiltonian(2)
    h2.add_term(PauliString.from_str("ZZ"), 0.5)

    h3 = Hamiltonian(2)
    h3.add_term(PauliString.from_str("XX"), 0.5)

    assert h1 == h2
    assert h1 != h3


def test_hamiltonian_copy():
    """Test Hamiltonian copy."""
    h1 = Hamiltonian(2)
    h1.add_term(PauliString.from_str("ZZ"), 0.5)

    h2 = h1.copy()
    assert h1 == h2

    # Modify copy
    h2.add_term(PauliString.from_str("XX"), 0.3)
    assert h1.num_terms == 1
    assert h2.num_terms == 2


def test_trotter_second_order():
    """Test second-order Trotter decomposition."""
    h = Hamiltonian(2)
    h.add_term(PauliString.from_str("ZZ"), 0.5)
    h.add_term(PauliString.from_str("XX"), 0.3)

    circ = h.to_trotter_circuit(1.0, 10, TrotterMode.second_order())
    assert circ.num_qubits == 2
    assert len(circ) > 0


def test_trotter_randomized():
    """Test randomized Trotter decomposition."""
    h = Hamiltonian(2)
    h.add_term(PauliString.from_str("ZZ"), 0.5)
    h.add_term(PauliString.from_str("XX"), 0.3)

    circ = h.to_trotter_circuit(1.0, 10, TrotterMode.randomized(42))
    assert circ.num_qubits == 2
    assert len(circ) > 0


class TestHamiltonianBoundaryConditions:
    """Test boundary conditions and edge cases."""

    def test_large_hamiltonian_20_qubits(self):
        """Test large 20-qubit Hamiltonian."""
        h = Hamiltonian(20)
        assert h.num_qubits == 20
        assert h.num_terms == 0

        # Add a term
        h.add_term(PauliString.from_str("Z" + "I" * 19), 1.0)
        assert h.num_terms == 1

    @pytest.mark.parametrize("coeff", [1e-15, 1e15, -1e15, 0.0, -0.0])
    def test_extreme_coefficient_values(self, coeff):
        """Test extreme coefficient values."""
        h = Hamiltonian(1)
        h.add_term(PauliString.from_str("X"), coeff)
        terms = h.terms
        assert len(terms) == 1
        assert math.isclose(terms[0][1].real, coeff, rel_tol=1e-10, abs_tol=1e-10)

    def test_zero_coefficient_handling(self):
        """Test zero coefficient term handling."""
        h = Hamiltonian(2)
        h.add_term(PauliString.from_str("XX"), 0.0)
        h.add_term(PauliString.from_str("ZZ"), 1.0)

        assert h.num_terms == 2
        h.simplify()
        # Zero coefficient term should be removed
        assert h.num_terms == 1

    def test_empty_hamiltonian_operations(self):
        """Test operations on empty Hamiltonian."""
        h = Hamiltonian(3)
        assert h.num_terms == 0

        # Scale empty Hamiltonian
        h.scale(2.0)
        assert h.num_terms == 0

        # Simplify empty Hamiltonian
        h.simplify()
        assert h.num_terms == 0

        # Copy empty Hamiltonian
        h_copy = h.copy()
        assert h_copy.num_terms == 0
        assert h_copy.num_qubits == 3

    def test_single_qubit_hamiltonian(self):
        """Test minimum 1-qubit Hamiltonian."""
        h = Hamiltonian(1)
        h.add_term(PauliString.from_str("X"), 0.5)
        h.add_term(PauliString.from_str("Z"), 0.3)

        assert h.num_qubits == 1
        assert h.num_terms == 2

    @pytest.mark.parametrize(
        "coeff",
        [
            1.0,  # float
            2,  # int
            1.0 + 0.5j,  # complex
            (0.3, 0.4),  # tuple (re, im)
        ],
    )
    def test_complex_coefficient_types(self, coeff):
        """Test various coefficient input types."""
        h = Hamiltonian(1)
        h.add_term(PauliString.from_str("X"), coeff)

        terms = h.terms
        if isinstance(coeff, tuple):
            expected = complex(*coeff)
        else:
            expected = complex(coeff)
        assert math.isclose(terms[0][1].real, expected.real, abs_tol=1e-10)
        assert math.isclose(terms[0][1].imag, expected.imag, abs_tol=1e-10)


class TestHamiltonianQuantumInvariants:
    """Test quantum mechanical invariants."""

    def test_hermitian_hamiltonian_real_expectation(self):
        """Test Hermitian Hamiltonian gives real expectation value."""
        from cqlib.qis import Statevector

        h = Hamiltonian(1)
        h.add_term(PauliString.from_str("X"), 0.5)
        h.add_term(PauliString.from_str("Z"), 0.3)

        sv = Statevector(1)
        sv.apply_h(0)

        exp = h.expectation_statevector(sv)
        # Expectation should be real (check imag is effectively 0)
        assert isinstance(exp, float)
        assert abs(exp) < 10  # Sanity check

    def test_identity_hamiltonian_expectation(self):
        """Test identity Hamiltonian gives expectation = 1."""
        from cqlib.qis import Statevector

        h = Hamiltonian.from_pauli(PauliString.from_str("I"))

        sv = Statevector(1)
        exp = h.expectation_statevector(sv)
        assert math.isclose(exp, 1.0, abs_tol=1e-10)

    def test_pauli_decomposition_completeness(self):
        """Test Pauli string decomposition of simple Hamiltonian."""
        h = Hamiltonian(2)
        h.add_term(PauliString.from_str("ZZ"), 0.5)
        h.add_term(PauliString.from_str("XX"), 0.3)
        h.add_term(PauliString.from_str("YY"), 0.2)

        # Verify all terms present
        assert h.num_terms == 3

        # Verify coefficients
        terms = h.terms
        paulis = [str(t[0]) for t in terms]
        assert "+ZZ" in paulis or "-ZZ" in paulis
        assert "+XX" in paulis or "-XX" in paulis
        assert "+YY" in paulis or "-YY" in paulis

    def test_trotter_circuit_generates_gates(self):
        """Test Trotter circuit generates valid quantum gates."""
        h = Hamiltonian(2)
        h.add_term(PauliString.from_str("ZZ"), 0.5)
        h.add_term(PauliString.from_str("XX"), 0.3)

        circ = h.to_trotter_circuit(1.0, 5, TrotterMode.first_order())

        # Circuit should have gates
        assert len(circ) > 0

        # More steps = more gates
        circ2 = h.to_trotter_circuit(1.0, 10, TrotterMode.first_order())
        assert len(circ2) > len(circ)

    def test_energy_additivity(self):
        """Test that energy expectation is additive for commuting terms."""
        from cqlib.qis import Statevector

        # H1 = ZZ, H2 = ZI, both diagonal in computational basis
        h1 = Hamiltonian(2)
        h1.add_term(PauliString.from_str("ZZ"), 1.0)

        h2 = Hamiltonian(2)
        h2.add_term(PauliString.from_str("ZI"), 0.5)

        h_total = h1 + h2

        sv = Statevector(2)  # |00⟩

        exp1 = h1.expectation_statevector(sv)
        exp2 = h2.expectation_statevector(sv)
        exp_total = h_total.expectation_statevector(sv)

        assert math.isclose(exp_total, exp1 + exp2, abs_tol=1e-10)


class TestHamiltonianNumericalPrecision:
    """Test numerical precision and stability."""

    def test_simplify_coefficient_precision(self):
        """Test simplify preserves coefficient precision."""
        h = Hamiltonian(2)
        h.add_term(PauliString.from_str("XX"), 0.1)
        h.add_term(PauliString.from_str("XX"), 0.2)

        h.simplify()

        terms = h.terms
        assert len(terms) == 1
        assert math.isclose(terms[0][1].real, 0.3, abs_tol=1e-10)

    def test_addition_concatenation(self):
        """Test Hamiltonian addition concatenates terms (lazy)."""
        h1 = Hamiltonian(1)
        h1.add_term(PauliString.from_str("X"), 0.5)

        h2 = Hamiltonian(1)
        h2.add_term(PauliString.from_str("Z"), 0.3)

        h3 = h1 + h2

        # Addition is lazy - terms are concatenated, not merged
        assert h3.num_terms == 2

    def test_scale_by_small_factor_precision(self):
        """Test scaling by small factor maintains precision."""
        h = Hamiltonian(1)
        h.add_term(PauliString.from_str("X"), 1.0)

        h.scale(1e-10)

        terms = h.terms
        assert math.isclose(terms[0][1].real, 1e-10, rel_tol=1e-5)

    def test_scale_by_complex_factor(self):
        """Test scaling by complex factor."""
        h = Hamiltonian(1)
        h.add_term(PauliString.from_str("X"), 1.0)

        h.scale(1j)

        terms = h.terms
        assert math.isclose(terms[0][1].real, 0.0, abs_tol=1e-10)
        assert math.isclose(terms[0][1].imag, 1.0, abs_tol=1e-10)


class TestHamiltonianCopySemantics:
    """Test copy semantics and independence."""

    def test_copy_independence_terms(self):
        """Test copy creates independent terms list."""
        h1 = Hamiltonian(2)
        h1.add_term(PauliString.from_str("XX"), 0.5)

        h2 = h1.copy()

        # Modify original
        h1.add_term(PauliString.from_str("ZZ"), 0.3)

        # Copy should be unchanged
        assert h1.num_terms == 2
        assert h2.num_terms == 1

    def test_copy_preserves_all_properties(self):
        """Test copy preserves all Hamiltonian properties."""
        h1 = Hamiltonian(2)
        h1.add_term(PauliString.from_str("ZZ"), 0.5)
        h1.add_term(PauliString.from_str("XX"), 0.3)

        h2 = h1.copy()

        assert h1.num_qubits == h2.num_qubits
        assert h1.num_terms == h2.num_terms

        # Check terms are equivalent
        for t1, t2 in zip(h1.terms, h2.terms):
            assert str(t1[0]) == str(t2[0])
            assert math.isclose(t1[1].real, t2[1].real, abs_tol=1e-10)
            assert math.isclose(t1[1].imag, t2[1].imag, abs_tol=1e-10)

    def test_terms_list_copy_semantics(self):
        """Test terms property returns consistent data."""
        h = Hamiltonian(1)
        h.add_term(PauliString.from_str("X"), 0.5)

        terms1 = h.terms
        terms2 = h.terms

        # Both calls should return equivalent data
        assert len(terms1) == len(terms2)
        for t1, t2 in zip(terms1, terms2):
            assert str(t1[0]) == str(t2[0])
            assert math.isclose(t1[1].real, t2[1].real, abs_tol=1e-10)


class TestHamiltonianAdvancedFeatures:
    """Test advanced features and edge cases."""

    def test_simplify_phase_normalization(self):
        """Test simplify normalizes phases correctly."""
        h = Hamiltonian(1)
        # Add term with internal phase
        h.add_term(PauliString.from_str("+iX"), 1.0)

        h.simplify()

        terms = h.terms
        assert len(terms) == 1
        # After simplify: iX with coeff 1.0 -> X with coeff i
        assert str(terms[0][0]) == "+X"
        assert math.isclose(terms[0][1].real, 0.0, abs_tol=1e-10)
        assert math.isclose(terms[0][1].imag, 1.0, abs_tol=1e-10)

    def test_simplify_cancellation_with_precision(self):
        """Test simplify handles near-zero coefficients."""
        h = Hamiltonian(2)
        h.add_term(PauliString.from_str("XX"), 1.0)
        h.add_term(PauliString.from_str("XX"), -1.0 + 1e-15)  # Near cancellation

        h.simplify()

        # Should keep or remove depending on tolerance
        assert h.num_terms <= 1

    def test_ising_model_hamiltonian(self):
        """Test constructing Ising model Hamiltonian."""
        n = 4
        h = Hamiltonian(n)

        # Add ZZ interactions for nearest neighbors
        for i in range(n - 1):
            pauli_str = "I" * (n - i - 2) + "ZZ" + "I" * i
            h.add_term(PauliString.from_str(pauli_str), -1.0)

        # Add transverse field X terms
        for i in range(n):
            pauli_str = "I" * (n - i - 1) + "X" + "I" * i
            h.add_term(PauliString.from_str(pauli_str), 0.5)

        assert h.num_qubits == n
        assert h.num_terms == (n - 1) + n  # n-1 ZZ + n X

    def test_heisenberg_model_hamiltonian(self):
        """Test constructing Heisenberg model Hamiltonian."""
        n = 3
        h = Hamiltonian(n)

        # Add XX, YY, ZZ interactions for nearest neighbors
        for i in range(n - 1):
            base = "I" * (n - i - 2)
            suffix = "I" * i
            h.add_term(PauliString.from_str(base + "XX" + suffix), 1.0)
            h.add_term(PauliString.from_str(base + "YY" + suffix), 1.0)
            h.add_term(PauliString.from_str(base + "ZZ" + suffix), 1.0)

        h.simplify()
        assert h.num_qubits == n

    def test_hamiltonian_chain_addition(self):
        """Test chain of Hamiltonian additions."""
        h1 = Hamiltonian(2)
        h1.add_term(PauliString.from_str("XX"), 0.1)

        h2 = Hamiltonian(2)
        h2.add_term(PauliString.from_str("YY"), 0.2)

        h3 = Hamiltonian(2)
        h3.add_term(PauliString.from_str("ZZ"), 0.3)

        h_total = h1 + h2 + h3
        assert h_total.num_terms == 3

        # Chain in-place addition
        h1 += h2
        h1 += h3
        assert h1.num_terms == 3

    def test_repr_format_detailed(self):
        """Test repr format contains expected information."""
        h = Hamiltonian(3)
        h.add_term(PauliString.from_str("ZZZ"), 1.0)

        r = repr(h)
        assert "Hamiltonian" in r
        assert "num_qubits" in r
        assert "num_terms" in r
        assert "3" in r  # num_qubits = 3

    def test_str_format_various_terms(self):
        """Test str format for various term configurations."""
        # Single term
        h1 = Hamiltonian(1)
        h1.add_term(PauliString.from_str("X"), 1.0)
        s1 = str(h1)
        assert isinstance(s1, str)
        assert len(s1) > 0

        # Multiple terms
        h2 = Hamiltonian(2)
        h2.add_term(PauliString.from_str("ZZ"), 0.5)
        h2.add_term(PauliString.from_str("XX"), 0.3)
        s2 = str(h2)
        assert isinstance(s2, str)

    def test_hamiltonian_equality_after_simplify(self):
        """Test equality before and after simplify."""
        h1 = Hamiltonian(2)
        h1.add_term(PauliString.from_str("XX"), 0.5)
        h1.add_term(PauliString.from_str("XX"), 0.5)

        h2 = Hamiltonian(2)
        h2.add_term(PauliString.from_str("XX"), 1.0)

        # Before simplify, they are different
        assert h1.num_terms == 2
        assert h2.num_terms == 1

        # After simplify, they should be equivalent
        h1.simplify()
        h2.simplify()
        assert h1 == h2


def test_hamiltonian_to_matrix_empty_is_zero():
    """Test an empty Hamiltonian evaluates to the zero matrix."""
    matrix = Hamiltonian(2).to_matrix()
    assert matrix.shape == (4, 4)
    assert matrix.dtype == np.complex128
    np.testing.assert_array_equal(matrix, np.zeros((4, 4), dtype=complex))


def test_hamiltonian_to_matrix_sums_coefficients_and_pauli_phases():
    """Test coefficients and Pauli phases are both applied: -2X + iZ."""
    x = PauliString.from_str("X")
    x.phase = Phase.minus()
    z = PauliString.from_str("Z")
    h = Hamiltonian.from_list([(x, 2.0), (z, 1j)])
    expected = np.array([[1j, -2], [-2, -1j]], dtype=complex)
    np.testing.assert_array_equal(h.to_matrix(), expected)


def test_hamiltonian_to_matrix_zero_qubits_is_scalar():
    """Test a zero-qubit Hamiltonian yields a 1x1 matrix."""
    pauli = PauliString(0)
    pauli.phase = Phase.i()
    h = Hamiltonian.from_list([(pauli, 2.0)])
    matrix = h.to_matrix()
    assert matrix.shape == (1, 1)
    np.testing.assert_array_equal(matrix, np.array([[2j]], dtype=complex))


def test_hamiltonian_from_list_rejects_mixed_qubit_counts():
    """Test the qubit-mismatch invariant is enforced at construction.

    to_matrix's own mismatch error path is therefore unreachable through the
    public Python API.
    """
    with pytest.raises(ValueError):
        Hamiltonian.from_list(
            [
                (PauliString.from_str("Z"), 1.0),
                (PauliString.from_str("ZZ"), 1.0),
            ]
        )
