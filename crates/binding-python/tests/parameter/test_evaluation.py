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
Tests for Parameter expression evaluation.

Test coverage:
- Constant evaluation
- Single symbol evaluation
- Expression evaluation
- Trigonometric evaluation
- Exponential and logarithm evaluation
- Square root and absolute value evaluation
- Evaluation with constants (pi, e)
- Partial evaluation
- Evaluation error handling
"""

import pytest
import numpy as np
from cqlib.circuit import Parameter


class TestConstantEvaluation:
    """Tests for constant evaluation."""

    def test_evaluate_float(self):
        """Evaluate float constant."""
        p = Parameter(3.14)
        result = p.evaluate()
        assert np.isclose(result, 3.14)

    def test_evaluate_zero(self):
        """Evaluate zero constant."""
        p = Parameter(0.0)
        result = p.evaluate({})
        assert np.isclose(result, 0.0)

    def test_evaluate_negative(self):
        """Evaluate negative constant."""
        p = Parameter(-5.5)
        result = p.evaluate({})
        assert np.isclose(result, -5.5)

    def test_evaluate_pi(self):
        """Evaluate pi constant."""
        pi = Parameter.pi()
        result = pi.evaluate({})
        assert np.isclose(result, np.pi)

    def test_evaluate_e(self):
        """Evaluate e constant."""
        e = Parameter.e()
        result = e.evaluate({})
        assert np.isclose(result, np.e)


class TestSingleSymbolEvaluation:
    """Tests for single symbol evaluation."""

    def test_evaluate_symbol(self):
        """Evaluate symbol with binding."""
        theta = Parameter("theta")
        result = theta.evaluate({"theta": 0.5})
        assert np.isclose(result, 0.5)

    def test_evaluate_symbol_zero(self):
        """Evaluate symbol to zero."""
        theta = Parameter("theta")
        result = theta.evaluate({"theta": 0.0})
        assert np.isclose(result, 0.0)

    def test_evaluate_symbol_negative(self):
        """Evaluate symbol to negative value."""
        theta = Parameter("theta")
        result = theta.evaluate({"theta": -2.0})
        assert np.isclose(result, -2.0)

    def test_evaluate_symbol_missing(self):
        """Evaluate without binding raises exception."""
        theta = Parameter("theta")
        with pytest.raises(Exception):
            theta.evaluate({})


class TestExpressionEvaluation:
    """Tests for expression evaluation."""

    def test_evaluate_addition(self):
        """Evaluate addition expression."""
        theta = Parameter("theta")
        expr = theta + 2.0
        result = expr.evaluate({"theta": 1.0})
        assert np.isclose(result, 3.0)

    def test_evaluate_subtraction(self):
        """Evaluate subtraction expression."""
        theta = Parameter("theta")
        expr = theta - 1.0
        result = expr.evaluate({"theta": 3.0})
        assert np.isclose(result, 2.0)

    def test_evaluate_multiplication(self):
        """Evaluate multiplication expression."""
        theta = Parameter("theta")
        expr = theta * 3.0
        result = expr.evaluate({"theta": 2.0})
        assert np.isclose(result, 6.0)

    def test_evaluate_division(self):
        """Evaluate division expression."""
        theta = Parameter("theta")
        expr = theta / 2.0
        result = expr.evaluate({"theta": 8.0})
        assert np.isclose(result, 4.0)

    def test_evaluate_complex_expression(self):
        """Evaluate complex expression."""
        theta = Parameter("theta")
        phi = Parameter("phi")
        expr = theta * 2.0 + phi
        result = expr.evaluate({"theta": 1.0, "phi": 3.0})
        assert np.isclose(result, 5.0)


class TestTrigonometricEvaluation:
    """Tests for trigonometric function evaluation."""

    def test_evaluate_sin(self):
        """Evaluate sin function."""
        theta = Parameter("theta")
        expr = theta.sin()
        result = expr.evaluate({"theta": np.pi / 2})
        assert np.isclose(result, 1.0)

    def test_evaluate_cos(self):
        """Evaluate cos function."""
        theta = Parameter("theta")
        expr = theta.cos()
        result = expr.evaluate({"theta": 0.0})
        assert np.isclose(result, 1.0)

    def test_evaluate_tan(self):
        """Evaluate tan function."""
        theta = Parameter("theta")
        expr = theta.tan()
        result = expr.evaluate({"theta": np.pi / 4})
        assert np.isclose(result, 1.0)

    def test_evaluate_sin_plus_cos(self):
        """Evaluate sin + cos expression."""
        theta = Parameter("theta")
        expr = theta.sin() + theta.cos()
        result = expr.evaluate({"theta": 0.0})
        assert np.isclose(result, 1.0)


class TestExponentialAndLogEvaluation:
    """Tests for exponential and logarithm evaluation."""

    def test_evaluate_exp(self):
        """Evaluate exp function."""
        x = Parameter("x")
        expr = x.exp()
        result = expr.evaluate({"x": 0.0})
        assert np.isclose(result, 1.0)

    def test_evaluate_exp_one(self):
        """Evaluate exp(1)."""
        x = Parameter("x")
        expr = x.exp()
        result = expr.evaluate({"x": 1.0})
        assert np.isclose(result, np.e)

    def test_evaluate_ln(self):
        """Evaluate natural logarithm."""
        x = Parameter("x")
        expr = x.ln()
        result = expr.evaluate({"x": 1.0})
        assert np.isclose(result, 0.0)

    def test_evaluate_ln_e(self):
        """Evaluate ln(e)."""
        x = Parameter("x")
        expr = x.ln()
        result = expr.evaluate({"x": np.e})
        assert np.isclose(result, 1.0)


class TestSqrtAndAbsEvaluation:
    """Tests for square root and absolute value evaluation."""

    def test_evaluate_sqrt(self):
        """Evaluate square root."""
        x = Parameter("x")
        expr = x.sqrt()
        result = expr.evaluate({"x": 4.0})
        assert np.isclose(result, 2.0)

    def test_evaluate_sqrt_zero(self):
        """Evaluate square root of zero."""
        x = Parameter("x")
        expr = x.sqrt()
        result = expr.evaluate({"x": 0.0})
        assert np.isclose(result, 0.0)

    def test_evaluate_abs_positive(self):
        """Evaluate absolute value of positive number."""
        x = Parameter("x")
        expr = x.abs()
        result = expr.evaluate({"x": 5.0})
        assert np.isclose(result, 5.0)

    def test_evaluate_abs_negative(self):
        """Evaluate absolute value of negative number."""
        x = Parameter("x")
        expr = x.abs()
        result = expr.evaluate({"x": -5.0})
        assert np.isclose(result, 5.0)


class TestEvaluationWithConstants:
    """Tests for evaluation with built-in constants."""

    def test_evaluate_with_pi(self):
        """Evaluate expression containing pi."""
        theta = Parameter("theta")
        expr = theta + Parameter.pi()
        result = expr.evaluate({"theta": 0.0})
        assert np.isclose(result, np.pi)

    def test_evaluate_with_e(self):
        """Evaluate expression containing e."""
        x = Parameter("x")
        expr = x * Parameter.e()
        result = expr.evaluate({"x": 1.0})
        assert np.isclose(result, np.e)


class TestPartialEvaluation:
    """Tests for partial evaluation behavior."""

    def test_partial_evaluate_two_symbols(self):
        """Partial evaluation with missing symbol raises exception."""
        theta = Parameter("theta")
        phi = Parameter("phi")
        expr = theta + phi

        # Only binding one symbol should raise
        with pytest.raises(Exception):
            expr.evaluate({"theta": 1.0})

    def test_evaluate_with_extra_bindings(self):
        """Extra bindings are ignored during evaluation."""
        theta = Parameter("theta")
        expr = theta + 1.0
        # Extra binding should be ignored
        result = expr.evaluate({"theta": 2.0, "phi": 3.0})
        assert np.isclose(result, 3.0)


class TestEvaluationErrors:
    """Tests for evaluation error handling."""

    def test_divide_by_zero(self):
        """Division by zero raises exception."""
        x = Parameter("x")
        expr = 1.0 / x
        with pytest.raises(Exception):
            expr.evaluate({"x": 0.0})

    def test_ln_of_zero(self):
        """ln(0) raises exception."""
        x = Parameter("x")
        expr = x.ln()
        with pytest.raises(Exception):
            expr.evaluate({"x": 0.0})

    def test_ln_of_negative(self):
        """ln(negative) raises exception."""
        x = Parameter("x")
        expr = x.ln()
        with pytest.raises(Exception):
            expr.evaluate({"x": -1.0})

    def test_sqrt_of_negative(self):
        """sqrt(negative) raises exception."""
        x = Parameter("x")
        expr = x.sqrt()
        with pytest.raises(Exception):
            expr.evaluate({"x": -1.0})
