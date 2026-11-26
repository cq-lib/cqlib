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

"""Fermionic Python contracts and independent Fock-space comparisons."""

import copy
import itertools

import numpy as np
import pytest

from cqlib.qis import (
    FermionicLadder as Ladder,
    FermionicOp as Op,
    FermionToQubitMapper,
    Hamiltonian,
    JordanWignerMapper,
    OccupationStateMapper,
    Statevector,
)
from cqlib.circuit.ansatz import BasisEncoding, EvolutionStrategy, PauliEvolutionAnsatz


def fock_ladder(num_modes, mode, create):
    """Independent ladder action on little-endian occupation basis states."""
    matrix = np.zeros((1 << num_modes, 1 << num_modes), dtype=complex)
    for column in range(1 << num_modes):
        occupied = bool(column & (1 << mode))
        if occupied == create:
            continue
        lower = column & ((1 << mode) - 1)
        sign = (-1) ** bin(lower).count("1")
        matrix[column ^ (1 << mode), column] = sign
    return matrix


def fock_matrix(op):
    dim = 1 << op.num_modes
    result = np.zeros((dim, dim), dtype=complex)
    for term, coefficient in op.terms:
        matrix = np.eye(dim, dtype=complex)
        for ladder in term:
            matrix = matrix @ fock_ladder(
                op.num_modes, ladder.mode, ladder.kind == "create"
            )
        result += coefficient * matrix
    return result


def test_exports_and_ladder_contract():
    from cqlib.qis.fermion import FermionicOp

    assert FermionicOp is Op
    assert Op.__module__ == "cqlib.qis.fermion"
    create = Ladder.create(2)
    assert create.mode == 2 and create.kind == "create"
    assert create.adjoint() == Ladder.annihilate(2)
    assert create.adjoint().adjoint() == create
    assert len({create, Ladder.create(2)}) == 1
    assert create != 2
    assert repr(create) == "FermionicLadder.create(2)"
    with pytest.raises(AttributeError):
        create.mode = 1
    with pytest.raises(TypeError):
        Ladder()


def test_construction_snapshots_and_exact_zero():
    create = Ladder.create(1)
    op = Op.from_terms(3, iter([([create], 2), ((create,), -1), ((), 1j)]))
    assert op.num_modes == 3 and op.num_terms == 2
    assert all(isinstance(c, complex) for _, c in op.terms)
    snapshot = op.terms
    assert all(isinstance(term, tuple) for term, _ in snapshot)
    snapshot.clear()
    assert op.num_terms == 2
    assert Op.from_terms(3, []).is_zero()
    assert Op.from_term(3, []).equiv(Op.identity(3))
    tiny = Op.from_term(3, [create], 1e-15)
    assert not tiny.is_zero()
    assert not tiny.equiv(Op(3))
    assert tiny.chop(atol=1e-15) is None
    assert tiny.is_zero()
    with pytest.raises(AttributeError):
        op.num_modes = 1
    with pytest.raises(TypeError):
        hash(op)


def test_arithmetic_adjoint_and_normal_order():
    a = Op.annihilation(2, 0)
    b = Op.annihilation(2, 1)
    identity = Op.identity(2)
    assert (a * a.adjoint() + a.adjoint() * a).equiv(identity)
    assert (a * b + b * a).equiv(Op(2))
    assert (a * b.adjoint() + b.adjoint() * a).equiv(Op(2))
    contraction = a * a.adjoint()
    assert contraction != contraction.normal_order()
    assert contraction.equiv(identity - Op.number(2, 0))
    assert not (a * a).is_zero()  # Raw product is stored until normal ordering.
    assert (a * a).normal_order().is_zero()
    assert (a * b).adjoint() == b.adjoint() * a.adjoint()
    assert (2j * a) == a * 2j == a.scaled(2j)
    assert -a == a * -1
    assert sum([a, b], start=Op(2)) == a + b
    assert Op.total_number(2).is_hermitian()
    alias = a
    a += b
    assert alias == Op.annihilation(2, 0)
    assert a == alias + b


def test_copy_and_mutation_failure_atomicity():
    op = Op.number(2, 0)
    for duplicate in (op.copy(), copy.copy(op), copy.deepcopy(op)):
        assert duplicate == op and duplicate is not op
        duplicate.add_term([], 2)
        assert duplicate != op
    before = op.copy()
    for term, coefficient, error in (
        ([Ladder.create(2)], 0, IndexError),
        (["bad"], 1, TypeError),
        ([], complex(float("nan"), 0), ValueError),
    ):
        with pytest.raises(error):
            op.add_term(term, coefficient)
        assert op == before
    assert "num_modes=2" in repr(op)
    assert "a†_0 a_0" in str(op)
    assert str(Op(0)) == "0" and "I" in str(Op.identity(0))


@pytest.mark.parametrize("value", [-1, 2**100])
def test_index_errors(value):
    error = IndexError if value < 0 else OverflowError
    with pytest.raises(error):
        Ladder.create(value)
    with pytest.raises(error):
        Op.creation(2, value)
    with pytest.raises(ValueError if value < 0 else OverflowError):
        Op(value)
    with pytest.raises(ValueError if value < 0 else OverflowError):
        JordanWignerMapper().num_qubits(value)


@pytest.mark.parametrize("value", [1.2, "1", None])
def test_type_errors(value):
    with pytest.raises(TypeError):
        Ladder.create(value)
    with pytest.raises(TypeError):
        Op(value)
    if not isinstance(value, float):
        with pytest.raises(TypeError):
            Op.from_term(2, [], value)


@pytest.mark.parametrize("atol", [-1, float("nan"), float("inf")])
def test_invalid_tolerances(atol):
    op = Op.number(2, 0)
    before = op.copy()
    for operation in (
        lambda: op.chop(atol=atol),
        lambda: op.equiv(before, atol=atol),
        lambda: op.is_hermitian(atol=atol),
    ):
        with pytest.raises(ValueError):
            operation()
        assert op == before


def test_arithmetic_errors_and_reflected_dispatch():
    a = Op.number(2, 0)
    b = Op.number(3, 0)
    for operation in (lambda: a + b, lambda: a - b, lambda: a * b, lambda: a.equiv(b)):
        with pytest.raises(ValueError):
            operation()
    for operation in (lambda: a + 1, lambda: a * "x", lambda: "x" * a):
        with pytest.raises(TypeError):
            operation()
    assert a.__add__(object()) is NotImplemented
    assert a.__sub__(object()) is NotImplemented
    assert a.__mul__(object()) is NotImplemented
    assert a.__rmul__(object()) is NotImplemented
    assert a.__eq__(object()) is NotImplemented

    class Reflected:
        def __rmul__(self, other):
            return "reflected"

    assert a * Reflected() == "reflected"
    for value in (float("nan"), float("inf"), complex(0, float("inf"))):
        with pytest.raises(ValueError):
            Op(2) * value
        with pytest.raises(ValueError):
            Op.from_term(2, [], value)
    large = Op.from_term(2, [], 1e308)
    before = large.copy()
    with pytest.raises(ValueError):
        large.add_term([], 1e308)
    assert large == before


@pytest.mark.parametrize(
    "sources,targets", [([], []), ([0], [1, 2]), ([0, 0], [1, 2]), ([0], [0])]
)
def test_excitation_validation(sources, targets):
    with pytest.raises(ValueError):
        Op.excitation_generator(4, sources, targets)


def test_mapping_against_independent_fock_products():
    mapper = JordanWignerMapper()
    ladders = [
        factory(p) for factory in (Ladder.create, Ladder.annihilate) for p in range(2)
    ]
    # Exhaustive ordered products exercise JW phases, contractions and fermionic signs.
    for length in range(4):
        for term in itertools.product(ladders, repeat=length):
            op = Op.from_term(2, term, 0.3 + 0.2j)
            expected = fock_matrix(op)
            np.testing.assert_allclose(
                mapper.map_operator(op).to_matrix(), expected, atol=1e-15
            )
            np.testing.assert_allclose(
                fock_matrix(op.normal_order()), expected, atol=1e-15
            )
    a = Op.annihilation(2, 0)
    np.testing.assert_allclose(
        mapper.map_operator(a.adjoint()).to_matrix(), fock_matrix(a).conj().T
    )
    combo = Op.from_terms(
        2, [((), 1j), ([Ladder.create(1)], 0.7), ([Ladder.create(1)], 0.3)]
    )
    np.testing.assert_allclose(
        mapper.map_operator(combo).to_matrix(), fock_matrix(combo)
    )
    for op in (Op.number(2, 1), Op.total_number(2)):
        np.testing.assert_allclose(mapper.map_operator(op).to_matrix(), fock_matrix(op))


def test_zero_identity_and_occupations():
    mapper = JordanWignerMapper()
    for n in (0, 1, 3):
        zero = mapper.map_operator(Op(n))
        assert (
            isinstance(zero, Hamiltonian)
            and zero.num_qubits == n
            and zero.num_terms == 0
        )
        np.testing.assert_allclose(zero.to_matrix(), np.zeros((1 << n, 1 << n)))
        np.testing.assert_allclose(
            mapper.map_operator(Op.identity(n)).to_matrix(), np.eye(1 << n)
        )
    assert mapper.map_occupation([]) == []
    assert mapper.map_occupation([1, False, True]) == [True, False, True]
    for value in (-1, 2, 256, 2**100):
        with pytest.raises(ValueError):
            mapper.map_occupation([value])
    with pytest.raises(TypeError):
        mapper.map_occupation([1.0])
    circuit = BasisEncoding(mapper.map_occupation([1, 0, 1])).build_circuit("hf")
    state = Statevector.from_circuit(circuit)
    np.testing.assert_allclose(state.data, np.eye(8)[5])


def test_integer_protocol_validation():
    class ValidIndex:
        def __index__(self):
            return 1

    class InvalidIndex:
        def __index__(self):
            return 1.0

    class InstanceOnlyIndex:
        def __init__(self):
            self.__index__ = lambda: 1

    mapper = JordanWignerMapper()
    assert Ladder.create(ValidIndex()) == Ladder.create(1)
    assert mapper.map_occupation([ValidIndex()]) == [True]
    assert mapper.num_qubits(ValidIndex()) == 1
    assert Op(np.int64(2)).num_modes == 2
    assert mapper.map_occupation(np.array([0, 1], dtype=np.int64)) == [False, True]
    for value in (InvalidIndex(), InstanceOnlyIndex()):
        for operation in (
            lambda: Ladder.create(value),
            lambda: Op(value),
            lambda: mapper.num_qubits(value),
            lambda: mapper.map_occupation([value]),
        ):
            with pytest.raises(TypeError):
                operation()


def test_protocols_and_python_extension():
    mapper = JordanWignerMapper()
    assert isinstance(mapper, FermionToQubitMapper)
    assert isinstance(mapper, OccupationStateMapper)

    class CustomMapper:
        def map_operator(self, op):
            return mapper.map_operator(op)

        def num_qubits(self, num_modes):
            return num_modes

    custom = CustomMapper()
    assert isinstance(custom, FermionToQubitMapper)
    assert not isinstance(custom, OccupationStateMapper)
    np.testing.assert_allclose(
        custom.map_operator(Op.number(2, 0)).to_matrix(), fock_matrix(Op.number(2, 0))
    )


def test_mapping_preserves_representable_subnormal_and_near_unit_coefficients():
    smallest = float.fromhex("0x0.0000000000001p-1022")
    mapper = JordanWignerMapper()
    scalar = Op.from_term(0, [], smallest)
    assert scalar.normal_order().terms[0][1].real == smallest
    assert mapper.map_operator(scalar).terms[0][1].real == smallest
    creation = Op.creation(1, 0).scaled(smallest * 2)
    assert all(
        abs(coefficient) == smallest
        for _, coefficient in mapper.map_operator(creation).terms
    )
    assert mapper.map_operator(creation).to_matrix()[1, 0].real == smallest * 2
    almost_one = float(np.nextafter(1.0, 2.0))
    number = Op.number(1, 0).scaled(almost_one)
    assert number.terms[0][1].real == almost_one
    assert mapper.map_operator(number).to_matrix()[1, 1].real == almost_one


def test_excitation_evolution_with_independent_parameters():
    mapper = JordanWignerMapper()
    generators = [
        Op.excitation_generator(4, [0, 1], [2, 3]),
        Op.excitation_generator(4, [0], [2]),
    ]
    assert generators[0].is_hermitian()
    assert Op.excitation_generator(4, [1, 0], [2, 3]).equiv(-generators[0])
    circuit = BasisEncoding(mapper.map_occupation([1, 1, 0, 0])).build_circuit("hf")
    for name, generator in zip(("theta_double", "theta_single"), generators):
        block = (
            PauliEvolutionAnsatz(mapper.map_operator(generator))
            .with_strategy(EvolutionStrategy.exact())
            .with_time_param_name(name)
            .build_circuit("ignored")
        )
        circuit.compose(block)
    assert set(circuit.symbols) == {"theta_double", "theta_single"}
    angles = {"theta_double": 0.23, "theta_single": -0.37}
    partial = circuit.assign_parameters({"theta_double": angles["theta_double"]})
    assert partial.symbols == ["theta_single"]
    bound = partial.assign_parameters({"theta_single": angles["theta_single"]})
    actual = Statevector.from_circuit(bound)
    expected = np.eye(16, dtype=complex)[3]
    for generator, theta in zip(generators, angles.values()):
        matrix = fock_matrix(generator)
        eigenvalues, eigenvectors = np.linalg.eigh(matrix)
        expected = (
            (eigenvectors * np.exp(-1j * theta * eigenvalues))
            @ eigenvectors.conj().T
            @ expected
        )
    np.testing.assert_allclose(actual.data, expected, atol=1e-14)
    assert mapper.map_operator(Op.total_number(4)).expectation_statevector(
        actual
    ) == pytest.approx(2)
