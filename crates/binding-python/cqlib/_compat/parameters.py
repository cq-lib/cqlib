# Copyright China Telecom Quantum Group 2026
# SPDX-License-Identifier: Apache-2.0

"""Normalize only legacy parameter-binding forms; native dicts bypass this."""

from collections.abc import Mapping
from math import isfinite
from numbers import Real

from ..circuit import Parameter, ParameterError


def normalize(bindings, values, kwargs, order, used_symbols):
    result = {}

    def add(key, value):
        if isinstance(key, Parameter):
            key = key.as_symbol()
        if not isinstance(key, str):
            raise ParameterError(
                "binding keys must be symbol names or single-symbol Parameters"
            )
        if key in result:
            raise ParameterError(f"duplicate binding for parameter {key!r}")
        if not isinstance(value, Real):
            raise TypeError(
                "legacy bindings require real numbers; Tensor bindings are unsupported"
            )
        value = float(value)
        if not isfinite(value):
            raise ParameterError(f"parameter binding must be finite, got {value}")
        result[key] = value

    for source in (bindings, values):
        if source is None:
            continue
        if isinstance(source, Mapping):
            for key, value in source.items():
                add(key, value)
            continue
        if isinstance(source, (str, bytes)) or not (
            hasattr(source, "__len__") and hasattr(source, "__getitem__")
        ):
            raise TypeError("bindings must be a mapping or a sequence of real numbers")
        if type(source).__module__.split(".")[0] == "torch":
            raise TypeError("Tensor bindings are unsupported")
        if order is None:
            raise ParameterError(
                "sequence bindings require an explicit parameter order from "
                "Circuit(..., parameters=[...]) or add_parameter(); use a name dictionary"
            )
        if not set(used_symbols).issubset(order):
            raise ParameterError(
                "declared parameter order does not cover this circuit; use a name dictionary"
            )
        if len(source) != len(order):
            raise ParameterError(
                f"expected {len(order)} parameter values, got {len(source)}"
            )
        for key, value in zip(order, source):
            add(key, value)

    for key, value in (kwargs or {}).items():
        add(key, value)
    return result
