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
inspecting, and transforming quantum circuits. It supports:

- **Static circuits** — fixed gate sequences with concrete numeric parameters.
- **Parameterized circuits** — symbolic gate angles for variational algorithms
  (VQE, QAOA, quantum machine learning).
- **Dynamic circuits** — runtime classical control flow driven by mid-circuit
  measurements (conditionals, loops, switches).

Quick start
-----------

Create a Bell state and compute its unitary matrix::

    import numpy as np
    from cqlib import Circuit

    c = Circuit(2)
    c.h(0)
    c.cx(0, 1)

    matrix = c.to_matrix()
    print(matrix.shape)  # (4, 4)

Parameterized circuit with symbolic angles::

    from cqlib import Circuit, Parameter

    theta = Parameter("theta")
    c = Circuit(1)
    c.rx(0, theta)

    evaluated = c.assign_parameters({"theta": 3.14159})

Module map
----------

===================== =============================================== ==============================
Submodule             Purpose                                         Key types
===================== =============================================== ==============================
``cqlib.circuit``     Circuit container and builder API               :class:`Circuit`
``.bit``              Qubit identifier                                :class:`Qubit`
``.gates``            Gate definitions and factory constants          :class:`StandardGate`
``.parameter``        Symbolic/numeric parameter expressions          :class:`Parameter`
``.classical``        Runtime classical types and storage handles     :class:`ClassicalType`
``.classical_expr``   Typed classical expression AST                  :class:`ClassicalExpr`
``.control_flow``     Classical control-flow operations               :class:`ClassicalControlOp`
``.operation``        Instruction and operation types                 :class:`ValueOperation`
``.symbolic_matrix``  Dense symbolic unitary matrices                 :class:`SymbolicMatrix`
===================== =============================================== ==============================
"""

from __future__ import annotations

import numpy as np
from numpy.typing import NDArray

from .bit import Qubit as Qubit
from .circuit import Circuit as Circuit
from .circuit import (
    ParamLike as ParamLike,
    QubitInput as QubitInput,
    QubitLike as QubitLike,
    QubitList as QubitList,
)
from .classical import (
    CircuitId as CircuitId,
    ClassicalType as ClassicalType,
    ClassicalValue as ClassicalValue,
    ClassicalVar as ClassicalVar,
    Measurement as Measurement,
)
from .classical_expr import ClassicalExpr as ClassicalExpr
from .control_flow import (
    ClassicalControlOp as ClassicalControlOp,
    ValueControlBody as ValueControlBody,
    ValueSwitchCase as ValueSwitchCase,
)
from .dag import (
    CircuitDag as CircuitDag,
    DagControlFlow as DagControlFlow,
    DagSwitchCase as DagSwitchCase,
    DagWire as DagWire,
)
from .gates import (
    CircuitGate as CircuitGate,
    Directive as Directive,
    FrozenCircuit as FrozenCircuit,
    MCGate as MCGate,
    StandardGate as StandardGate,
    UnitaryGate as UnitaryGate,
)
from .operation import (
    Instruction as Instruction,
    ValueInstruction as ValueInstruction,
    ValueOperation as ValueOperation,
)
from .parameter import Parameter as Parameter
from .symbolic_matrix import (
    SymbolicComplex as SymbolicComplex,
    SymbolicMatrix as SymbolicMatrix,
)

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

class CqlibError(Exception):
    """Base class for all cqlib-specific exceptions."""

    ...

class CircuitError(CqlibError):
    """Raised when a circuit operation fails validation (arity, qubit, type)."""

    ...

class ParameterError(CqlibError):
    """Raised when a parameter expression is invalid or cannot be evaluated."""

    ...

class QubitError(CqlibError):
    """Raised when a qubit identifier is invalid (negative, out of range)."""

    ...

def circuit_to_matrix(
    circuit: Circuit, qubits_order: list[int] | None = None
) -> NDArray[np.complex128]:
    """Compute the dense unitary matrix for a quantum circuit.

    Equivalent to :meth:`Circuit.to_matrix`.

    Args:
        circuit: The quantum circuit to convert.
        qubits_order: Optional custom qubit ordering.

    Returns:
        A 2D NumPy array (dtype=complex128) of shape (2ⁿ, 2ⁿ).

    Raises:
        CircuitError: If the circuit contains non-unitary operations.
    """
    ...
