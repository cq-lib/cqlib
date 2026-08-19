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

"""Symbolic matrix tests for the Python circuit API."""

import numpy as np
import pytest

from cqlib import Parameter
from cqlib.circuit import ParameterError, SymbolicComplex, SymbolicMatrix


def test_symbolic_complex_evaluation():
    theta = Parameter("theta")
    value = SymbolicComplex.exp_i(theta)

    assert np.isclose(value.evaluate({"theta": 0.5}), np.cos(0.5) + 1j * np.sin(0.5))
    assert value.symbols == ["theta"]


def test_symbolic_matrix_evaluation():
    theta = Parameter("theta")
    matrix = SymbolicMatrix(
        [
            [SymbolicComplex.one(), SymbolicComplex.zero()],
            [SymbolicComplex.zero(), SymbolicComplex.exp_i(theta)],
        ]
    )

    evaluated = matrix.evaluate({"theta": 0.25})

    assert matrix.shape == (2, 2)
    assert np.allclose(
        evaluated,
        np.array([[1, 0], [0, np.cos(0.25) + 1j * np.sin(0.25)]], dtype=complex),
    )


def test_symbolic_matrix_substitute_replaces_symbol():
    theta = Parameter("theta")
    phi = Parameter("phi")
    matrix = SymbolicMatrix(
        [
            [SymbolicComplex.from_real(theta), SymbolicComplex.zero()],
            [SymbolicComplex.zero(), SymbolicComplex.from_real(phi)],
        ]
    )

    substituted = matrix.substitute({"theta": phi})

    assert substituted.symbols == ["phi"]


def test_symbolic_matrix_rows_returns_symbolic_entries():
    theta = Parameter("theta")
    matrix = SymbolicMatrix(
        [
            [SymbolicComplex.one(), SymbolicComplex.zero()],
            [SymbolicComplex.zero(), SymbolicComplex.from_real(theta)],
        ]
    )

    rows = matrix.rows()

    assert len(rows) == 2
    assert rows[1][1].symbols == ["theta"]


def test_symbolic_matrix_evaluation_requires_bindings():
    theta = Parameter("theta")
    matrix = SymbolicMatrix([[SymbolicComplex.from_real(theta)]])

    with pytest.raises(ParameterError):
        matrix.evaluate({})
