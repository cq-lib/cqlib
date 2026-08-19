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

"""Tests for Trotter evolution modes and Hamiltonian evolution ansatz metadata."""

import pytest
from cqlib.circuit.ansatz import EvolutionStrategy, PauliEvolutionAnsatz
from cqlib.qis import Hamiltonian, PauliString, TrotterMode


class TestTrotterMode:
    """Test TrotterMode class."""

    def test_first_order(self):
        """Test first-order Trotter mode creation."""
        mode = TrotterMode.first_order()
        assert mode is not None
        assert str(mode) == "first-order"
        assert "FirstOrder" in repr(mode)

    def test_second_order(self):
        """Test second-order Trotter mode creation."""
        mode = TrotterMode.second_order()
        assert mode is not None
        assert str(mode) == "second-order"
        assert "SecondOrder" in repr(mode)

    def test_randomized(self):
        """Test randomized Trotter mode creation."""
        mode = TrotterMode.randomized(42)
        assert mode is not None
        assert "randomized" in str(mode)
        assert "Randomized" in repr(mode)
        assert "seed=42" in repr(mode)

    def test_randomized_different_seeds(self):
        """Test that different seeds create different modes."""
        mode1 = TrotterMode.randomized(42)
        mode2 = TrotterMode.randomized(123)
        # Different seeds should not be equal
        assert mode1 != mode2

    def test_same_mode_equality(self):
        """Test equality of same modes."""
        mode1 = TrotterMode.first_order()
        mode2 = TrotterMode.first_order()
        assert mode1 == mode2

        mode3 = TrotterMode.second_order()
        mode4 = TrotterMode.second_order()
        assert mode3 == mode4

        mode5 = TrotterMode.randomized(42)
        mode6 = TrotterMode.randomized(42)
        assert mode5 == mode6

    def test_different_modes_inequality(self):
        """Test inequality of different modes."""
        mode_first = TrotterMode.first_order()
        mode_second = TrotterMode.second_order()
        mode_rand = TrotterMode.randomized(42)

        assert mode_first != mode_second
        assert mode_first != mode_rand
        assert mode_second != mode_rand

    def test_trotter_mode_repr(self):
        """Test repr format."""
        mode = TrotterMode.first_order()
        repr_str = repr(mode)
        assert "TrotterMode" in repr_str

    def test_trotter_mode_str(self):
        """Test str format."""
        mode = TrotterMode.first_order()
        str_val = str(mode)
        assert isinstance(str_val, str)


class TestTrotterModeBoundaryConditions:
    """Test boundary conditions and edge cases."""

    @pytest.mark.parametrize("seed", [0, 1, 42, 2**32, 2**63 - 1])
    def test_randomized_extreme_seeds(self, seed):
        """Test randomized mode with various seed values."""
        mode = TrotterMode.randomized(seed)
        assert mode is not None
        assert "Randomized" in repr(mode)
        assert f"seed={seed}" in repr(mode)

    def test_randomized_zero_seed(self):
        """Test randomized mode with zero seed."""
        mode = TrotterMode.randomized(0)
        assert mode is not None
        # Zero seed should work like any other seed
        repr_str = repr(mode)
        assert "seed=0" in repr_str


class TestTrotterModeCopySemantics:
    """Test copy semantics and independence."""

    def test_copy_independence(self):
        """Test TrotterMode copies are independent."""
        mode1 = TrotterMode.first_order()
        mode2 = mode1  # Assignment

        # TrotterMode is immutable/value type
        assert mode1 == mode2

    def test_copy_equality(self):
        """Test copied modes are equal."""
        mode_orig = TrotterMode.randomized(12345)
        # Since TrotterMode is immutable, we just verify equality
        mode_copy = mode_orig
        assert mode_orig == mode_copy
        assert str(mode_orig) == str(mode_copy)
        assert repr(mode_orig) == repr(mode_copy)


class TestTrotterModeAdvancedFeatures:
    """Test advanced features and properties."""

    def test_equality_transitivity(self):
        """Test equality is transitive."""
        mode1 = TrotterMode.first_order()
        mode2 = TrotterMode.first_order()
        mode3 = TrotterMode.first_order()

        assert mode1 == mode2
        assert mode2 == mode3
        assert mode1 == mode3  # Transitivity

    def test_repr_contains_all_info(self):
        """Test repr contains all necessary information."""
        # FirstOrder
        mode1 = TrotterMode.first_order()
        r1 = repr(mode1)
        assert "TrotterMode" in r1
        assert "FirstOrder" in r1

        # SecondOrder
        mode2 = TrotterMode.second_order()
        r2 = repr(mode2)
        assert "TrotterMode" in r2
        assert "SecondOrder" in r2

        # Randomized with seed
        mode3 = TrotterMode.randomized(42)
        r3 = repr(mode3)
        assert "TrotterMode" in r3
        assert "Randomized" in r3
        assert "seed=42" in r3

    def test_randomized_same_seed_reproducibility(self):
        """Test same seed produces equivalent modes."""
        seed = 12345
        mode1 = TrotterMode.randomized(seed)
        mode2 = TrotterMode.randomized(seed)

        # Same seed should produce equal modes
        assert mode1 == mode2
        assert str(mode1) == str(mode2)
        assert repr(mode1) == repr(mode2)

    def test_str_format_all_modes(self):
        """Test str format for all Trotter modes."""
        mode_first = TrotterMode.first_order()
        mode_second = TrotterMode.second_order()
        mode_rand = TrotterMode.randomized(42)

        # All should return strings
        assert isinstance(str(mode_first), str)
        assert isinstance(str(mode_second), str)
        assert isinstance(str(mode_rand), str)

        # Should contain expected substrings
        assert "first" in str(mode_first).lower()
        assert "second" in str(mode_second).lower()
        assert "random" in str(mode_rand).lower()

    def test_mode_hash_consistency(self):
        """Test that equal modes have consistent behavior."""
        mode1 = TrotterMode.first_order()
        mode2 = TrotterMode.first_order()

        # Equal modes should behave identically
        assert mode1 == mode2
        assert str(mode1) == str(mode2)
        assert repr(mode1) == repr(mode2)


class TestEvolutionStrategy:
    """Test Python bindings for Hamiltonian evolution strategies."""

    def test_randomized_trotter_strategy_inequality_when_steps_differ(self):
        """Regression: randomized strategies must compare steps as well as seed."""
        strategy1 = EvolutionStrategy.trotter(TrotterMode.randomized(42), steps=1)
        strategy2 = EvolutionStrategy.trotter(TrotterMode.randomized(42), steps=2)

        assert strategy1 != strategy2

    def test_randomized_trotter_strategy_inequality_when_seed_differs(self):
        """Randomized strategies with different seeds are not equal."""
        strategy1 = EvolutionStrategy.trotter(TrotterMode.randomized(1), steps=3)
        strategy2 = EvolutionStrategy.trotter(TrotterMode.randomized(2), steps=3)

        assert strategy1 != strategy2


class TestPauliEvolutionAnsatzInfo:
    """Test exposed evolution metadata semantics."""

    def test_explicit_trotter_reports_exact_for_commuting_hamiltonian(self):
        """Explicit Trotter remains mathematically exact for commuting terms."""
        h = Hamiltonian(2)
        h.add_term(PauliString.from_str("ZZ"), 0.5)
        h.add_term(PauliString.from_str("ZI"), 0.3)

        ansatz = PauliEvolutionAnsatz(h).with_strategy(
            EvolutionStrategy.trotter(TrotterMode.second_order(), steps=7)
        )
        info = ansatz.evolution_info()

        assert info.is_exact is True
        assert info.steps == 7
        assert info.trotter_mode == TrotterMode.second_order()
        assert info.all_terms_commute is True
        assert info.num_terms == 2
