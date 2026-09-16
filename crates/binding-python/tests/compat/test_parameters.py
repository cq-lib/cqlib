# Copyright China Telecom Quantum Group 2026
# SPDX-License-Identifier: Apache-2.0

import copy
from decimal import Decimal
import warnings

import numpy as np
import pytest

from cqlib import Circuit, CqlibDeprecationWarning, Parameter, ParameterError
from cqlib.ir import qcis

pytestmark = pytest.mark.filterwarnings("ignore::cqlib.CqlibDeprecationWarning")


def parameterized():
    theta, phi = Parameter("theta"), Parameter("phi")
    # Declaration order deliberately differs from expression/use order.
    circuit = Circuit(2, parameters=[phi, theta])
    circuit.rx(0, theta + 2 * phi)
    circuit.ry(1, theta)
    return circuit, theta, phi


@pytest.mark.parametrize(
    "bind",
    [
        lambda c, t, p: c.assign_parameters({"theta": 0.2, "phi": 0.3}),
        lambda c, t, p: c.assign_parameters({t: 0.2, p: 0.3}),
        lambda c, t, p: c.assign_parameters([0.3, 0.2]),
        lambda c, t, p: c.assign_parameters((0.3, 0.2)),
        lambda c, t, p: c.assign_parameters(np.array([0.3, 0.2])),
        lambda c, t, p: c.assign_parameters(theta=0.2, phi=0.3),
        lambda c, t, p: c.assign_parameters(values={t: 0.2}, phi=0.3),
        lambda c, t, p: c.assign_parameters(
            bindings={"theta": 0.2}, values={"phi": 0.3}
        ),
    ],
)
def test_binding_forms_agree_and_do_not_mutate_source(bind):
    circuit, theta, phi = parameterized()
    before, identity = circuit.copy(), circuit.id
    result = bind(circuit, theta, phi)
    expected = Circuit(2)
    expected.rx(0, 0.8)
    expected.ry(1, 0.2)
    np.testing.assert_allclose(result.to_matrix(), expected.to_matrix(), atol=1e-12)
    assert circuit == before and circuit.id == identity
    assert type(result) is Circuit and result is not circuit
    assert set(result.used_symbols) == set()


def test_partial_binding_and_inplace_return():
    circuit, _, _ = parameterized()
    partial = circuit.assign_parameters(theta=0.2)
    assert partial.used_symbols == ["phi"]
    # Order remains the explicit declaration order even after partial binding.
    expected = circuit.assign_parameters({"theta": 0.2, "phi": 0.3})
    assert partial.assign_parameters([0.3, 99.0]) == expected
    alias = circuit
    result = circuit.assign_parameters(theta=0.2, inplace=True)
    assert result is circuit is alias
    assert circuit == partial


@pytest.mark.parametrize(
    "transform",
    [
        lambda c: c.copy(),
        copy.copy,
        copy.deepcopy,
        lambda c: c.inverse().inverse(),
        lambda c: c.decompose(),
        lambda c: c.assign_parameters({}),
    ],
)
def test_parameter_order_survives_supported_copying_transforms(transform):
    circuit, _, _ = parameterized()
    result = transform(circuit).assign_parameters([0.3, 0.2])
    np.testing.assert_allclose(
        result.to_matrix(),
        circuit.assign_parameters({"phi": 0.3, "theta": 0.2}).to_matrix(),
        atol=1e-12,
    )


@pytest.mark.parametrize(
    "compose",
    [
        lambda a, b: a + b,
        lambda a, b: (a.compose(b), a)[1],
        lambda a, b: a.__iadd__(b),
    ],
)
def test_composition_merges_explicit_order_left_first(compose):
    left, theta, _ = parameterized()
    beta = Parameter("beta")
    right = Circuit(2, parameters=[beta, theta])
    right.rz(0, beta + theta)
    result = compose(left, right)
    assert result.assign_parameters([0.3, 0.2, 0.4]) == result.assign_parameters(
        {"phi": 0.3, "theta": 0.2, "beta": 0.4}
    )


def test_ambiguous_orders_are_rejected_instead_of_guessed():
    circuit = Circuit(1)
    circuit.rx(0, Parameter("theta"))
    with pytest.raises(ParameterError, match="explicit parameter order"):
        circuit.assign_parameters([0.2])
    declared, _, _ = parameterized()
    combined = declared + circuit
    with pytest.raises(ParameterError, match="explicit parameter order"):
        combined.assign_parameters([0.3, 0.2])
    declared.rz(0, Parameter("extra"))
    with pytest.raises(ParameterError, match="does not cover"):
        declared.assign_parameters([0.3, 0.2])
    imported = qcis.loads(qcis.dumps(circuit))
    with pytest.raises(ParameterError, match="explicit parameter order"):
        imported.assign_parameters([0.2])
    assert imported.assign_parameters({"theta": 0.2}) == circuit.assign_parameters(
        {"theta": 0.2}
    )


def test_add_parameter_declares_order_without_changing_native_parameter_views():
    theta, phi = Parameter("theta"), Parameter("phi")
    circuit = Circuit(1)
    circuit.add_parameter(phi)
    circuit.add_parameter(theta)
    circuit.rx(0, theta + phi)
    assert circuit.parameters == [phi, theta, theta + phi]
    assert circuit.assign_parameters([0.3, 0.2]) == circuit.assign_parameters(
        {"phi": 0.3, "theta": 0.2}
    )
    before, params = circuit.copy(), circuit.parameters
    for parameter in [theta, theta + phi, Parameter(0.2)]:
        with pytest.raises(ParameterError):
            circuit.add_parameter(parameter)
        assert circuit == before and circuit.parameters == params
    with pytest.raises(ParameterError, match="duplicate"):
        Circuit(1, parameters=[theta, theta])


@pytest.mark.parametrize(
    "bad_binding",
    [
        {"values": {"theta": 0.2}, "theta": 0.3},
        {"bindings": {Parameter("theta"): 0.2, "theta": 0.3}},
        {"bindings": {Parameter("theta") + Parameter("phi"): 0.2}},
        {"bindings": [0.3]},
        {"bindings": [float("nan"), 0.2]},
        {"bindings": [0.3, float("inf")]},
        {"theta": 1j},
        {"cache_params": True},
    ],
)
def test_invalid_inplace_binding_does_not_mutate(bad_binding):
    circuit, _, _ = parameterized()
    before, identity, params = circuit.copy(), circuit.id, circuit.parameters
    with pytest.raises((ParameterError, TypeError)):
        circuit.assign_parameters(inplace=True, **bad_binding)
    assert circuit == before and circuit.id == identity
    assert circuit.parameters == params


def test_warning_as_error_prevents_legacy_mutation():
    circuit, _, _ = parameterized()
    before, identity = circuit.copy(), circuit.id
    with warnings.catch_warnings():
        warnings.simplefilter("error", CqlibDeprecationWarning)
        with pytest.raises(CqlibDeprecationWarning):
            circuit.assign_parameters(theta=0.2, inplace=True)
    assert circuit == before and circuit.id == identity


def test_native_binding_forms_stay_warning_free_with_original_numeric_conversion():
    with warnings.catch_warnings():
        warnings.simplefilter("error", CqlibDeprecationWarning)
        circuit = Circuit(1)
        circuit.rx(0, Parameter("theta"))
        assert circuit.assign_parameters() == circuit
        assert circuit.assign_parameters(None) == circuit
        assert (
            circuit.assign_parameters(bindings={"theta": Decimal("0.2")}).used_symbols
            == []
        )
        with pytest.raises(ParameterError):
            circuit.assign_parameters({"theta": float("nan")})


def test_reentrant_binding_reports_a_python_borrow_error_without_panicking():
    circuit = Circuit(1)
    errors = []

    class Angle:
        def __float__(self):
            try:
                circuit.assign_parameters()
            except Exception as error:
                errors.append(error)
            return 0.2

    circuit.rx(0, Angle())
    assert len(errors) == 1
    assert isinstance(errors[0], RuntimeError)
    assert "borrow" in str(errors[0])
    assert len(circuit) == 1
