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

"""Quantum circuit construction and manipulation.

The ``cqlib.circuit`` module provides the foundational types for building,
inspecting, and transforming quantum circuits.  It supports static circuits
(fixed gate sequences), parameterized circuits (symbolic gate angles for
variational algorithms), and dynamic circuits (runtime classical control
flow driven by mid-circuit measurements).

See the type stubs at ``cqlib/circuit/__init__.pyi`` for the complete API
reference and usage examples.
"""

from .._native import circuit as _circuit_module

from .gates import (
    CircuitGate,
    Directive,
    FrozenCircuit,
    MCGate,
    StandardGate,
    UnitaryGate,
)

Parameter = _circuit_module.Parameter
Circuit = _circuit_module.Circuit
CircuitDag = _circuit_module.CircuitDag
DagControlFlow = _circuit_module.DagControlFlow
DagSwitchCase = _circuit_module.DagSwitchCase
DagWire = _circuit_module.DagWire
Qubit = _circuit_module.Qubit
Instruction = _circuit_module.Instruction
ValueInstruction = _circuit_module.ValueInstruction
ValueOperation = _circuit_module.ValueOperation
CircuitId = _circuit_module.CircuitId
ClassicalType = _circuit_module.ClassicalType
ClassicalVar = _circuit_module.ClassicalVar
ClassicalValue = _circuit_module.ClassicalValue
Measurement = _circuit_module.Measurement
ClassicalExpr = _circuit_module.ClassicalExpr
ValueControlBody = _circuit_module.ValueControlBody
ValueSwitchCase = _circuit_module.ValueSwitchCase
ClassicalControlOp = _circuit_module.ClassicalControlOp
SymbolicComplex = _circuit_module.SymbolicComplex
SymbolicMatrix = _circuit_module.SymbolicMatrix
CqlibError = _circuit_module.CqlibError
CircuitError = _circuit_module.CircuitError
ParameterError = _circuit_module.ParameterError
QubitError = _circuit_module.QubitError
circuit_to_matrix = _circuit_module.circuit_to_matrix

__all__ = [
    "Circuit",
    "CircuitDag",
    "CircuitId",
    "CircuitError",
    "ClassicalControlOp",
    "ClassicalExpr",
    "ClassicalType",
    "ClassicalValue",
    "ClassicalVar",
    "CqlibError",
    "DagControlFlow",
    "DagSwitchCase",
    "DagWire",
    "Instruction",
    "Measurement",
    "Parameter",
    "ParameterError",
    "Qubit",
    "QubitError",
    "SymbolicComplex",
    "SymbolicMatrix",
    "ValueControlBody",
    "ValueInstruction",
    "ValueOperation",
    "ValueSwitchCase",
    "CircuitGate",
    "circuit_to_matrix",
    "Directive",
    "FrozenCircuit",
    "MCGate",
    "StandardGate",
    "UnitaryGate",
]
