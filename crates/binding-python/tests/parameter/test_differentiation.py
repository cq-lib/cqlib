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
Tests for Parameter symbolic differentiation.

Test coverage:
- Single variable differentiation
- Constant differentiation
- Linear function differentiation
- Polynomial differentiation
- Trigonometric differentiation
- Exponential and logarithm differentiation
- Chain rule
- Higher order derivatives
"""

import numpy as np
from cqlib.circuit import Parameter


class TestBasicDifferentiation:
    """Tests for basic differentiation."""

    def test_derivative_of_symbol(self):
        """Derivative of symbol with respect to itself is 1."""
        theta = Parameter("theta")
        deriv = theta.derivative("theta")
        assert np.isclose(deriv.evaluate({}), 1.0)

    def test_derivative_of_different_symbol(self):
        """Derivative of symbol with respect to different symbol is 0."""
        theta = Parameter("theta")
        deriv = theta.derivative("phi")
        assert np.isclose(deriv.evaluate({}), 0.0)

    def test_derivative_of_constant(self):
        """Derivative of constant is 0."""
        c = Parameter(5.0)
        deriv = c.derivative("theta")
        assert np.isclose(deriv.evaluate({}), 0.0)


class TestLinearFunctionDifferentiation:
    """Tests for linear function differentiation."""

    def test_derivative_of_scaled_symbol(self):
        """Derivative of scaled symbol."""
        theta = Parameter("theta")
        expr = 3.0 * theta
        deriv = expr.derivative("theta")
        # d(3*theta)/d(theta) = 3, needs simplify()
        simplified = deriv.simplify()
        assert np.isclose(simplified.evaluate({}), 3.0)

    def test_derivative_of_scaled_symbol_reverse(self):
        """Derivative of symbol multiplied by constant."""
        theta = Parameter("theta")
        expr = theta * 3.0
        deriv = expr.derivative("theta")
        # Needs simplify() to get constant 3
        simplified = deriv.simplify()
        assert np.isclose(simplified.evaluate({}), 3.0)

    def test_derivative_of_sum_with_constant(self):
        """Derivative of sum with constant."""
        theta = Parameter("theta")
        expr = theta + 5.0
        deriv = expr.derivative("theta")
        # d(theta + 5)/d(theta) = 1
        assert np.isclose(deriv.evaluate({}), 1.0)

    def test_derivative_of_difference(self):
        """Derivative of difference with constant."""
        theta = Parameter("theta")
        expr = theta - 2.0
        deriv = expr.derivative("theta")
        assert np.isclose(deriv.evaluate({}), 1.0)


class TestPolynomialDifferentiation:
    """Tests for polynomial differentiation."""

    def test_derivative_of_quadratic(self):
        """Derivative of quadratic function."""
        theta = Parameter("theta")
        expr = theta**2
        deriv = expr.derivative("theta")
        # d(theta^2)/d(theta) = 2*theta
        result = deriv.evaluate({"theta": 3.0})
        assert np.isclose(result, 6.0)

    def test_derivative_of_cubic(self):
        """Derivative of cubic function."""
        theta = Parameter("theta")
        expr = theta**3
        deriv = expr.derivative("theta")
        # d(theta^3)/d(theta) = 3*theta^2
        result = deriv.evaluate({"theta": 2.0})
        assert np.isclose(result, 12.0)

    def test_derivative_of_polynomial(self):
        """Derivative of polynomial expression."""
        theta = Parameter("theta")
        expr = theta**2 + 3 * theta + 2
        deriv = expr.derivative("theta")
        # d(theta^2 + 3*theta + 2)/d(theta) = 2*theta + 3
        result = deriv.evaluate({"theta": 1.0})
        assert np.isclose(result, 5.0)


class TestTrigonometricDifferentiation:
    """Tests for trigonometric function differentiation."""

    def test_derivative_of_sin(self):
        """Derivative of sin is cos."""
        theta = Parameter("theta")
        expr = theta.sin()
        deriv = expr.derivative("theta")
        # d(sin(theta))/d(theta) = cos(theta)
        result = deriv.evaluate({"theta": 0.0})
        assert np.isclose(result, 1.0)  # cos(0) = 1

    def test_derivative_of_cos(self):
        """Derivative of cos is -sin."""
        theta = Parameter("theta")
        expr = theta.cos()
        deriv = expr.derivative("theta")
        # d(cos(theta))/d(theta) = -sin(theta)
        result = deriv.evaluate({"theta": np.pi / 2})
        assert np.isclose(result, -1.0)  # -sin(pi/2) = -1

    def test_derivative_of_tan(self):
        """Derivative of tan is sec^2."""
        theta = Parameter("theta")
        expr = theta.tan()
        deriv = expr.derivative("theta")
        # d(tan(theta))/d(theta) = sec^2(theta) = 1/cos^2(theta)
        result = deriv.evaluate({"theta": 0.0})
        assert np.isclose(result, 1.0)  # sec^2(0) = 1

    def test_derivative_of_scaled_sin(self):
        """Derivative of scaled sin using chain rule."""
        theta = Parameter("theta")
        expr = (2 * theta).sin()
        deriv = expr.derivative("theta")
        # d(sin(2*theta))/d(theta) = 2*cos(2*theta)
        result = deriv.evaluate({"theta": 0.0})
        assert np.isclose(result, 2.0)


class TestExponentialAndLogDifferentiation:
    """Tests for exponential and logarithm differentiation."""

    def test_derivative_of_exp(self):
        """Derivative of exp is exp."""
        theta = Parameter("theta")
        expr = theta.exp()
        deriv = expr.derivative("theta")
        # d(exp(theta))/d(theta) = exp(theta)
        result = deriv.evaluate({"theta": 0.0})
        assert np.isclose(result, 1.0)  # exp(0) = 1

    def test_derivative_of_ln(self):
        """Derivative of ln is 1/x."""
        theta = Parameter("theta")
        expr = theta.ln()
        deriv = expr.derivative("theta")
        # d(ln(theta))/d(theta) = 1/theta
        result = deriv.evaluate({"theta": 2.0})
        assert np.isclose(result, 0.5)

    def test_derivative_of_exp_scaled(self):
        """Derivative of scaled exp using chain rule."""
        theta = Parameter("theta")
        expr = (2 * theta).exp()
        deriv = expr.derivative("theta")
        # d(exp(2*theta))/d(theta) = 2*exp(2*theta)
        result = deriv.evaluate({"theta": 0.0})
        assert np.isclose(result, 2.0)


class TestProductRule:
    """Tests for product rule."""

    def test_derivative_of_product(self):
        """Derivative of product using product rule."""
        theta = Parameter("theta")
        phi = Parameter("phi")
        expr = theta * phi

        # d(theta * phi)/d(theta) = phi, needs simplify()
        deriv_theta = expr.derivative("theta").simplify()
        result = deriv_theta.evaluate({"phi": 5.0})
        assert np.isclose(result, 5.0)

        # d(theta * phi)/d(phi) = theta, needs simplify()
        deriv_phi = expr.derivative("phi").simplify()
        result = deriv_phi.evaluate({"theta": 3.0})
        assert np.isclose(result, 3.0)

    def test_derivative_of_product_with_constant(self):
        """Derivative of product with constant."""
        theta = Parameter("theta")
        expr = theta * 4.0
        deriv = expr.derivative("theta")
        # Needs simplify() to get constant 4
        simplified = deriv.simplify()
        assert np.isclose(simplified.evaluate({}), 4.0)


class TestQuotientRule:
    """Tests for quotient rule."""

    def test_derivative_of_quotient(self):
        """Derivative of quotient."""
        theta = Parameter("theta")
        expr = theta / 2.0
        deriv = expr.derivative("theta")
        # d(theta/2)/d(theta) = 1/2, needs simplify()
        simplified = deriv.simplify()
        assert np.isclose(simplified.evaluate({}), 0.5)

    def test_derivative_of_reciprocal(self):
        """Derivative of reciprocal."""
        theta = Parameter("theta")
        expr = 1.0 / theta
        deriv = expr.derivative("theta")
        # d(1/theta)/d(theta) = -1/theta^2
        result = deriv.evaluate({"theta": 2.0})
        assert np.isclose(result, -0.25)


class TestChainRule:
    """Tests for chain rule."""

    def test_derivative_of_composition(self):
        """Derivative of function composition."""
        theta = Parameter("theta")
        expr = (theta + 1).sin()
        deriv = expr.derivative("theta")
        # d(sin(theta + 1))/d(theta) = cos(theta + 1)
        result = deriv.evaluate({"theta": 0.0})
        assert np.isclose(result, np.cos(1.0))

    def test_derivative_of_nested_functions(self):
        """Derivative of nested functions."""
        theta = Parameter("theta")
        expr = theta.sin().exp()
        deriv = expr.derivative("theta")
        # d(exp(sin(theta)))/d(theta) = cos(theta) * exp(sin(theta))
        result = deriv.evaluate({"theta": 0.0})
        expected = np.cos(0.0) * np.exp(np.sin(0.0))
        assert np.isclose(result, expected)


class TestHigherOrderDerivatives:
    """Tests for higher order derivatives."""

    def test_second_derivative_of_quadratic(self):
        """Second derivative of quadratic is constant."""
        theta = Parameter("theta")
        expr = theta**2
        first = expr.derivative("theta")  # 2*theta
        second = first.derivative("theta")  # 2
        # Needs simplify() to get constant 2
        simplified = second.simplify()
        assert np.isclose(simplified.evaluate({}), 2.0)

    def test_second_derivative_of_cubic(self):
        """Second derivative of cubic."""
        theta = Parameter("theta")
        expr = theta**3
        first = expr.derivative("theta")  # 3*theta^2
        second = first.derivative("theta")  # 6*theta
        result = second.evaluate({"theta": 2.0})
        assert np.isclose(result, 12.0)

    def test_third_derivative(self):
        """Third derivative."""
        theta = Parameter("theta")
        expr = theta**3
        first = expr.derivative("theta")  # 3*theta^2
        second = first.derivative("theta")  # 6*theta
        third = second.derivative("theta")  # 6
        # Needs simplify() to get constant 6
        simplified = third.simplify()
        assert np.isclose(simplified.evaluate({}), 6.0)

    def test_second_derivative_of_sin(self):
        """Second derivative of sin is -sin."""
        theta = Parameter("theta")
        expr = theta.sin()
        first = expr.derivative("theta")  # cos(theta)
        second = first.derivative("theta")  # -sin(theta)
        result = second.evaluate({"theta": np.pi / 2})
        assert np.isclose(result, -1.0)


class TestSimplificationAfterDifferentiation:
    """Tests for simplification after differentiation."""

    def test_derivative_simplification(self):
        """Derivative result can be simplified."""
        theta = Parameter("theta")
        expr = theta + theta  # 2*theta
        deriv = expr.derivative("theta")
        # d(2*theta)/d(theta) = 2
        assert np.isclose(deriv.evaluate({}), 2.0)
