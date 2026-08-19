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
Tests for Parameter mathematical functions.

Test coverage:
- Trigonometric functions (sin, cos, tan)
- Inverse trigonometric functions (asin, acos, atan)
- Exponential and logarithm (exp, ln, log)
- Other functions (sqrt, abs)
- Chained function calls
"""

import numpy as np
from cqlib.circuit import Parameter


class TestTrigonometricFunctions:
    """Tests for trigonometric functions."""

    def test_sin_symbol(self):
        """Apply sin to a symbol."""
        theta = Parameter("theta")
        result = theta.sin()
        assert str(result) == "sin(theta)"

    def test_cos_symbol(self):
        """Apply cos to a symbol."""
        theta = Parameter("theta")
        result = theta.cos()
        assert str(result) == "cos(theta)"

    def test_tan_symbol(self):
        """Apply tan to a symbol."""
        theta = Parameter("theta")
        result = theta.tan()
        assert str(result) == "tan(theta)"

    def test_sin_of_float(self):
        """sin(0) evaluates to 0."""
        p = Parameter(0.0)
        result = p.sin()
        # sin(0) = 0, needs simplify() to evaluate
        simplified = result.simplify()
        assert str(simplified) == "0"

    def test_cos_of_float(self):
        """cos(0) evaluates to 1."""
        p = Parameter(0.0)
        result = p.cos()
        # cos(0) = 1, needs simplify() to evaluate
        simplified = result.simplify()
        assert str(simplified) == "1"


class TestInverseTrigonometricFunctions:
    """Tests for inverse trigonometric functions."""

    def test_asin_symbol(self):
        """Apply asin to a symbol."""
        x = Parameter("x")
        result = x.asin()
        assert str(result) == "asin(x)"

    def test_acos_symbol(self):
        """Apply acos to a symbol."""
        x = Parameter("x")
        result = x.acos()
        assert str(result) == "acos(x)"

    def test_atan_symbol(self):
        """Apply atan to a symbol."""
        x = Parameter("x")
        result = x.atan()
        assert str(result) == "atan(x)"


class TestExponentialAndLogarithm:
    """Tests for exponential and logarithm functions."""

    def test_exp_symbol(self):
        """Apply exp to a symbol."""
        x = Parameter("x")
        result = x.exp()
        assert str(result) == "exp(x)"

    def test_ln_symbol(self):
        """Apply ln to a symbol."""
        x = Parameter("x")
        result = x.ln()
        assert str(result) == "ln(x)"

    def test_log_with_base(self):
        """Apply log with specified base."""
        x = Parameter("x")
        base = Parameter(10.0)
        result = x.log(base)
        assert str(result) == "log(10, x)"

    def test_log_without_base(self):
        """Apply log without base (defaults to ln)."""
        x = Parameter("x")
        result = x.log(None)
        assert str(result) == "log(e, x)"

    def test_exp_of_zero(self):
        """exp(0) evaluates to 1."""
        p = Parameter(0.0)
        result = p.exp()
        # exp(0) = 1, needs simplify() to evaluate
        simplified = result.simplify()
        assert str(simplified) == "1"

    def test_ln_of_one(self):
        """ln(1) evaluates to 0."""
        p = Parameter(1.0)
        result = p.ln()
        # ln(1) = 0, needs simplify() to evaluate
        simplified = result.simplify()
        assert str(simplified) == "0"


class TestOtherFunctions:
    """Tests for other mathematical functions."""

    def test_sqrt_symbol(self):
        """Apply sqrt to a symbol."""
        x = Parameter("x")
        result = x.sqrt()
        assert str(result) == "sqrt(x)"

    def test_abs_symbol(self):
        """Apply abs to a symbol."""
        x = Parameter("x")
        result = x.abs()
        assert str(result) == "abs(x)"

    def test_sqrt_of_perfect_square(self):
        """sqrt(4) evaluates to 2."""
        p = Parameter(4.0)
        result = p.sqrt()
        # sqrt(4) = 2, needs simplify() to evaluate
        simplified = result.simplify()
        assert str(simplified) == "2"

    def test_abs_of_positive(self):
        """abs(5) evaluates to 5."""
        p = Parameter(5.0)
        result = p.abs()
        # Constant folding for abs() not implemented, verify by evaluation
        assert np.isclose(result.evaluate({}), 5.0)

    def test_abs_of_negative(self):
        """abs(-5) evaluates to 5."""
        p = Parameter(-5.0)
        result = p.abs()
        # Constant folding for abs() not implemented, verify by evaluation
        assert np.isclose(result.evaluate({}), 5.0)


class TestChainedFunctions:
    """Tests for chained function calls."""

    def test_sin_of_cos(self):
        """Chain sin after cos."""
        theta = Parameter("theta")
        result = theta.cos().sin()
        assert str(result) == "sin(cos(theta))"

    def test_exp_of_sin(self):
        """Chain exp after sin."""
        theta = Parameter("theta")
        result = theta.sin().exp()
        assert str(result) == "exp(sin(theta))"

    def test_sqrt_of_sum(self):
        """Apply sqrt to sum expression."""
        theta = Parameter("theta")
        result = (theta + 1).sqrt()
        assert str(result) == "sqrt(1 + theta)"

    def test_complex_chain(self):
        """Complex chain of function calls."""
        theta = Parameter("theta")
        result = theta.sin().exp().ln()
        assert str(result) == "ln(exp(sin(theta)))"


class TestFunctionWithExpressions:
    """Tests for functions applied to expressions."""

    def test_sin_of_expression(self):
        """Apply sin to expression."""
        theta = Parameter("theta")
        result = (theta + 1).sin()
        assert str(result) == "sin(1 + theta)"

    def test_exp_of_scaled(self):
        """Apply exp to scaled parameter."""
        theta = Parameter("theta")
        result = (2 * theta).exp()
        assert str(result) == "exp(2*theta)"

    def test_sqrt_of_product(self):
        """Apply sqrt to product of parameters."""
        theta = Parameter("theta")
        phi = Parameter("phi")
        result = (theta * phi).sqrt()
        assert str(result) == "sqrt(phi*theta)"
