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
Tests for Parameter arithmetic operations.

Test coverage:
- Addition (+)
- Subtraction (-)
- Multiplication (*)
- Division (/)
- Power (**)
- Negation (-)
- Mixed expressions
"""

from cqlib.circuit import Parameter


class TestAddition:
    """Tests for addition operation."""

    def test_add_two_symbols(self):
        """Add two symbolic parameters."""
        theta = Parameter("theta")
        phi = Parameter("phi")
        result = theta + phi
        # Symbol library orders terms alphabetically
        assert str(result) == "phi + theta"

    def test_add_symbol_and_float(self):
        """Add a symbol and a float."""
        theta = Parameter("theta")
        result = theta + 2.0
        # Symbol library puts numbers before symbols
        assert str(result) == "2 + theta"

    def test_add_float_and_symbol(self):
        """Add a float and a symbol (reverse addition)."""
        theta = Parameter("theta")
        result = 2.0 + theta
        # Commutative: 2 + theta stays as 2 + theta
        assert str(result) == "2 + theta"

    def test_add_zero(self):
        """Add zero to a symbol."""
        theta = Parameter("theta")
        result = theta + 0.0
        # theta + 0 simplifies to theta
        assert str(result.simplify()) == "theta"

    def test_add_two_floats(self):
        """Add two float parameters."""
        p1 = Parameter(2.0)
        p2 = Parameter(3.0)
        result = p1 + p2
        # Simplify to evaluate the addition
        assert str(result.simplify()) == "5"


class TestSubtraction:
    """Tests for subtraction operation."""

    def test_subtract_two_symbols(self):
        """Subtract two symbols."""
        theta = Parameter("theta")
        phi = Parameter("phi")
        result = theta - phi
        # Symbol library converts subtraction to addition: a - b = -b + a
        # Alphabetical order: -phi + theta
        assert str(result) == "-phi + theta"

    def test_subtract_float_from_symbol(self):
        """Subtract a float from a symbol."""
        theta = Parameter("theta")
        result = theta - 1.0
        # Symbol library converts to addition: theta - 1 = -1 + theta
        assert str(result) == "-1 + theta"

    def test_subtract_symbol_from_float(self):
        """Subtract a symbol from a float."""
        theta = Parameter("theta")
        result = 1.0 - theta
        # 1 - theta stays as 1 - theta
        assert str(result) == "1 - theta"

    def test_subtract_zero(self):
        """Subtract zero from a symbol."""
        theta = Parameter("theta")
        result = theta - 0.0
        # theta - 0 simplifies to theta
        assert str(result.simplify()) == "theta"

    def test_subtract_same_symbol(self):
        """Subtract a symbol from itself."""
        theta = Parameter("theta")
        result = theta - theta
        # theta - theta simplifies to 0
        assert str(result.simplify()) == "0"


class TestMultiplication:
    """Tests for multiplication operation."""

    def test_multiply_two_symbols(self):
        """Multiply two symbols."""
        theta = Parameter("theta")
        phi = Parameter("phi")
        result = theta * phi
        # Symbol library orders alphabetically, no spaces around *
        assert str(result) == "phi*theta"

    def test_multiply_symbol_by_float(self):
        """Multiply a symbol by a float."""
        theta = Parameter("theta")
        result = theta * 2.0
        # Symbol library puts numbers before symbols, no spaces
        assert str(result) == "2*theta"

    def test_multiply_float_by_symbol(self):
        """Multiply a float by a symbol."""
        theta = Parameter("theta")
        result = 2.0 * theta
        # Symbol library puts numbers before symbols, no spaces
        assert str(result) == "2*theta"

    def test_multiply_by_one(self):
        """Multiply a symbol by one."""
        theta = Parameter("theta")
        result = theta * 1.0
        # theta * 1 simplifies to theta
        assert str(result.simplify()) == "theta"

    def test_multiply_by_zero(self):
        """Multiply a symbol by zero."""
        theta = Parameter("theta")
        result = theta * 0.0
        # theta * 0 simplifies to 0
        assert str(result.simplify()) == "0"


class TestDivision:
    """Tests for division operation."""

    def test_divide_two_symbols(self):
        """Divide two symbols."""
        theta = Parameter("theta")
        phi = Parameter("phi")
        result = theta / phi
        # Division keeps order, no spaces around /
        assert str(result) == "theta/phi"

    def test_divide_symbol_by_float(self):
        """Divide a symbol by a float."""
        theta = Parameter("theta")
        result = theta / 2.0
        # No spaces around /
        assert str(result) == "theta/2"

    def test_divide_float_by_symbol(self):
        """Divide a float by a symbol."""
        theta = Parameter("theta")
        result = 2.0 / theta
        # No spaces around /
        assert str(result) == "2/theta"

    def test_divide_by_one(self):
        """Divide a symbol by one."""
        theta = Parameter("theta")
        result = theta / 1.0
        # theta / 1 simplifies to theta
        assert str(result.simplify()) == "theta"


class TestPower:
    """Tests for power operation."""

    def test_power_float(self):
        """Raise a symbol to a float power."""
        theta = Parameter("theta")
        result = theta**2
        assert str(result) == "theta^2"

    def test_power_symbol(self):
        """Raise a symbol to another symbol's power."""
        theta = Parameter("theta")
        phi = Parameter("phi")
        result = theta**phi
        assert str(result) == "theta^phi"

    def test_power_zero(self):
        """Raise a symbol to power zero."""
        theta = Parameter("theta")
        result = theta**0
        # theta^0 simplifies to 1
        assert str(result.simplify()) == "1"

    def test_power_one(self):
        """Raise a symbol to power one."""
        theta = Parameter("theta")
        result = theta**1
        # theta^1 simplifies to theta
        assert str(result.simplify()) == "theta"


class TestNegation:
    """Tests for negation operation."""

    def test_negate_symbol(self):
        """Negate a symbol."""
        theta = Parameter("theta")
        result = -theta
        # Symbol library represents negation as -theta
        assert str(result) == "-theta"

    def test_negate_float(self):
        """Negate a float parameter."""
        p = Parameter(5.0)
        result = -p
        # Negation result needs simplify() to evaluate
        assert str(result.simplify()) == "-5"

    def test_double_negation(self):
        """Double negation of a symbol."""
        theta = Parameter("theta")
        result = -(-theta)
        # -(-theta) simplifies to theta
        assert str(result.simplify()) == "theta"


class TestComplexExpressions:
    """Tests for complex expressions."""

    def test_linear_combination(self):
        """Test linear combination of symbols."""
        theta = Parameter("theta")
        phi = Parameter("phi")
        result = 2 * theta + 3 * phi
        # Symbol library orders terms: coefficients first, then alphabetically
        assert str(result) == "3*phi + 2*theta"

    def test_polynomial(self):
        """Test polynomial expression."""
        theta = Parameter("theta")
        result = theta**2 + 2 * theta + 1
        # Symbol library orders: constants first, then terms by coefficient
        assert str(result) == "1 + (2*theta + theta^2)"

    def test_nested_operations(self):
        """Test nested operations."""
        theta = Parameter("theta")
        phi = Parameter("phi")
        result = (theta + phi) * (theta - phi)
        # Symbol library: (-phi + theta)*(phi + theta)
        assert str(result) == "(-phi + theta)*(phi + theta)"

    def test_mixed_expression(self):
        """Test mixed arithmetic expression."""
        theta = Parameter("theta")
        result = theta**2 / 2 + 3 * theta - 1
        # Symbol library: -1 + 3*theta + theta^2/2
        assert str(result) == "-1 + 3*theta + theta^2/2"
