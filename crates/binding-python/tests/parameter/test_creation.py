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
Tests for Parameter creation and basic properties.

Test coverage:
- Symbolic parameter creation
- Float parameter creation
- Constant parameters (pi, e)
- String representation
- Parameter equality
"""

import numpy as np
from cqlib.circuit import Parameter


class TestSymbolicParameterCreation:
    """Tests for symbolic parameter creation."""

    def test_create_simple_symbol(self):
        """Create a simple symbol."""
        theta = Parameter("theta")
        assert str(theta) == "theta"

    def test_create_multi_char_symbol(self):
        """Create a multi-character symbol."""
        param = Parameter("parameter_name")
        assert str(param) == "parameter_name"

    def test_create_symbol_with_number(self):
        """Create a symbol with number in name."""
        theta1 = Parameter("theta1")
        assert str(theta1) == "theta1"

    def test_symbol_get_symbols(self):
        """Get symbols from a parameter."""
        theta = Parameter("theta")
        symbols = theta.symbols
        assert "theta" in symbols


class TestFloatParameterCreation:
    """Tests for float parameter creation."""

    def test_create_from_float(self):
        """Create parameter from float."""
        p = Parameter(3.14)
        assert str(p) == "3.14"

    def test_create_from_zero(self):
        """Create parameter from zero."""
        p = Parameter(0.0)
        assert str(p) == "0"

    def test_create_from_negative(self):
        """Create parameter from negative float."""
        p = Parameter(-5.5)
        assert str(p) == "-5.5"

    def test_create_from_integer(self):
        """Create parameter from integer value (as float)."""
        p = Parameter(42.0)
        assert str(p) == "42"


class TestConstantParameters:
    """Tests for constant parameters."""

    def test_pi_constant(self):
        """Test pi constant value."""
        pi = Parameter.pi()
        result = pi.evaluate({})
        assert np.isclose(result, np.pi)

    def test_e_constant(self):
        """Test e constant value."""
        e = Parameter.e()
        result = e.evaluate({})
        assert np.isclose(result, np.e)

    def test_pi_symbol_representation(self):
        """Test pi symbol representation."""
        pi = Parameter.pi()
        # Pi may display as "pi" or "π"
        pi_str = str(pi).lower()
        assert "pi" in pi_str or "π" in pi_str

    def test_e_symbol_representation(self):
        """Test e symbol representation."""
        e = Parameter.e()
        assert "e" in str(e).lower()


class TestParameterRepresentation:
    """Tests for parameter string representation."""

    def test_str_symbol(self):
        """String representation of symbolic parameter."""
        theta = Parameter("theta")
        assert str(theta) == "theta"

    def test_str_float(self):
        """String representation of float parameter."""
        p = Parameter(2.5)
        assert str(p) == "2.5"

    def test_repr_contains_info(self):
        """Repr contains parameter information."""
        theta = Parameter("theta")
        repr_str = repr(theta)
        assert "Parameter" in repr_str
        assert "theta" in repr_str


class TestParameterEquality:
    """Tests for parameter equality comparison."""

    def test_same_symbol_equal(self):
        """Same symbol names are equal."""
        theta1 = Parameter("theta")
        theta2 = Parameter("theta")
        assert theta1 == theta2

    def test_different_symbols_not_equal(self):
        """Different symbol names are not equal."""
        theta = Parameter("theta")
        phi = Parameter("phi")
        assert theta != phi

    def test_same_float_equal(self):
        """Same float values are equal."""
        p1 = Parameter(3.14)
        p2 = Parameter(3.14)
        assert p1 == p2

    def test_different_floats_not_equal(self):
        """Different float values are not equal."""
        p1 = Parameter(1.0)
        p2 = Parameter(2.0)
        assert p1 != p2

    def test_symbol_and_float_not_equal(self):
        """Symbol and float are not equal."""
        theta = Parameter("theta")
        p = Parameter(0.0)
        assert theta != p

    def test_symbol_with_itself(self):
        """Symbol is equal to itself."""
        theta = Parameter("theta")
        assert theta == theta


class TestParameterSymbolsProperty:
    """Tests for parameter symbols property."""

    def test_single_symbol(self):
        """Single symbol parameter."""
        theta = Parameter("theta")
        symbols = theta.symbols
        assert len(symbols) == 1
        assert "theta" in symbols

    def test_constant_no_symbols(self):
        """Float constant has no symbols."""
        p = Parameter(5.0)
        symbols = p.symbols
        assert len(symbols) == 0

    def test_pi_constant_has_no_free_symbols(self):
        """Pi constant is resolved automatically and has no free symbols."""
        pi = Parameter.pi()
        symbols = pi.symbols
        assert len(symbols) == 0
