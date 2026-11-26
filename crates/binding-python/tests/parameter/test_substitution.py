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

import numpy as np
import pytest

from cqlib import Circuit, Parameter


@pytest.mark.parametrize("reverse", [False, True])
def test_substitute_swaps_symbols_using_the_same_dictionary(reverse):
    x, y = Parameter("x"), Parameter("y")
    original = x + 2 * y
    bindings = dict(reversed([("x", y), ("y", x)])) if reverse else {"x": y, "y": x}

    for _ in range(100):
        result = original.substitute(bindings)
        assert set(result.symbols) == {"x", "y"}
        assert result.evaluate({"x": 1.0, "y": 3.0}) == 5.0

    assert original.evaluate({"x": 1.0, "y": 3.0}) == 7.0
    assert bindings["x"] == y
    assert bindings["y"] == x


@pytest.mark.parametrize(
    "case", ["chain", "self", "cycle", "multiple_keys", "independent"]
)
def test_substitute_does_not_process_inserted_expressions(case):
    x, y, z = (Parameter(name) for name in ("x", "y", "z"))
    original, bindings, expected = {
        "chain": (x, {"x": y + 1, "y": Parameter(2)}, 11.0),
        "self": (x, {"x": x + 1}, 3.0),
        "cycle": (x + 2 * y + 3 * z, {"x": y, "y": z, "z": x}, 56.0),
        "multiple_keys": (x, {"x": y + z, "y": Parameter(1), "z": Parameter(2)}, 30.0),
        "independent": (x + y, {"x": z + 1, "y": Parameter(2)}, 23.0),
    }[case]
    assert (
        original.substitute(bindings).evaluate({"x": 2.0, "y": 10.0, "z": 20.0})
        == expected
    )


@pytest.mark.parametrize("method", ["replace", "substitute"])
@pytest.mark.parametrize("kind", ["polynomial", "function_polynomial", "nested"])
def test_substitution_reaches_polynomials_and_nested_functions(method, kind):
    x, y = Parameter("x"), Parameter("y")
    base = x.sin()
    original = {
        "polynomial": x * x + x + 1,
        "function_polynomial": base * base + 2 * base + 1,
        "nested": ((x * x + x + 1) / (x + 2)).cos() + x ** (y + 1),
    }[kind]
    result = (
        original.replace("x", y)
        if method == "replace"
        else original.substitute({"x": y})
    )
    assert set(result.symbols) == {"y"}
    assert result.evaluate({"y": 0.5}) == pytest.approx(
        original.evaluate({"x": 0.5, "y": 0.5}), abs=1e-12
    )


def test_substitute_preserves_unbound_symbols_constants_and_empty_bindings():
    x, y = Parameter("x"), Parameter("y")
    original = x + y
    for bindings in ({}, {"missing": Parameter(9)}):
        result = original.substitute(bindings)
        assert set(result.symbols) == {"x", "y"}
        assert result.evaluate({"x": 2.0, "y": 3.0}) == 5.0
    partial = original.substitute({"x": Parameter(2)})
    assert partial.symbols == ["y"]
    assert partial.evaluate({"y": 3.0}) == 5.0
    assert Parameter(0.25).substitute({"x": y}).evaluate() == 0.25


@pytest.mark.parametrize(
    "coefficient", [1e-16, np.nextafter(0.0, 1.0), 1.0 + np.finfo(float).eps]
)
def test_substitution_preserves_affine_coefficients(coefficient):
    x, y = Parameter("x"), Parameter("y")
    original = x * float(coefficient)
    for result in (original.replace("x", y), original.substitute({"x": y})):
        assert result.evaluate({"y": 1.0}) == coefficient


def test_numeric_circuit_assignment_remains_equivalent():
    x, y = Parameter("x"), Parameter("y")
    circuit = Circuit(1)
    circuit.rx(0, x + 2 * y)
    expected = Circuit(1)
    expected.rx(0, 7.0)
    for bound in (
        circuit.assign_parameters({"x": 1.0, "y": 3.0}),
        circuit.assign_parameters({"x": 1.0}).assign_parameters({"y": 3.0}),
    ):
        np.testing.assert_allclose(bound.to_matrix(), expected.to_matrix(), atol=1e-12)
