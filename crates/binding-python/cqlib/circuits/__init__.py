# Copyright China Telecom Quantum Group 2026
# SPDX-License-Identifier: Apache-2.0

"""Legacy imports for the supported circuit-construction workflow.

All types are the native types. Gate and operation constructors are factories;
see docs/legacy-compatibility.md for the supported subset.
"""

from ..circuit import Circuit, Instruction, Parameter, Qubit
from .._compat.deprecation import warn as _warn
from .._compat.operations import InstructionData

_warn("cqlib.circuits", "cqlib.circuit")

__all__ = ["Circuit", "Instruction", "Parameter", "Qubit", "InstructionData", "gates"]


def __getattr__(name):
    if name == "gates":
        from importlib import import_module

        return import_module("cqlib.circuits.gates")
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
