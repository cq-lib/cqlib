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

"""Runtime classical types and circuit-local handles.

These types model the classical side of dynamic circuits: typed storage
locations (:class:`ClassicalVar`), immutable measurement results
(:class:`ClassicalValue`), and measurement receipts
(:class:`Measurement`).  Each handle is tied to a :class:`CircuitId` that
prevents accidental mixing of handles from different circuits.
"""

from .bit import Qubit
from .classical_expr import ClassicalExpr
from cqlib.device import Outcome

import builtins

class CircuitId:
    """Process-local identity shared by classical handles of one circuit.

    Allocated automatically by :meth:`Circuit.__init__`.  Not usually
    constructed directly.
    """
    def __init__(self) -> None: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> CircuitId: ...
    def __deepcopy__(self, memo: dict) -> CircuitId: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class ClassicalType:
    """Static type of a runtime classical expression or storage location.

    Factory methods create the four supported types::

        bit      = ClassicalType.bit()       # single bit
        boolean  = ClassicalType.bool()      # logical bool
        counter  = ClassicalType.uint(8)     # 8-bit unsigned int
        bv       = ClassicalType.bit_vec(4)  # 4-bit ordered vector
    """
    @staticmethod
    def bit() -> ClassicalType:
        """The single-bit type (a measured qubit outcome, 0 or 1)."""
        ...
    @staticmethod
    def bool() -> ClassicalType:
        """The logical boolean type (true/false)."""
        ...
    @staticmethod
    def uint(width: int) -> ClassicalType:
        """An unsigned integer type of the given bit ``width``.

        Args:
            width: Bit width, must be a positive integer.
        """
        ...
    @staticmethod
    def bit_vec(width: int) -> ClassicalType:
        """An ordered bit-vector type of the given ``width``.

        Args:
            width: Vector width, must be a positive integer.
        """
        ...
    @property
    def width(self) -> int:
        """Number of bits represented by this type."""
        ...
    def zero_literal(self) -> ClassicalExpr:
        """Zero literal typed to this classical type."""
        ...
    def one_literal(self) -> ClassicalExpr:
        """One literal typed to this classical type."""
        ...
    def __repr__(self) -> str: ...
    def __copy__(self) -> ClassicalType: ...
    def __deepcopy__(self, memo: dict) -> ClassicalType: ...
    def __eq__(self, other: object) -> builtins.bool: ...
    def __hash__(self) -> int: ...

class ClassicalVar:
    """Circuit-local handle to mutable runtime classical storage.

    Created by :meth:`Circuit.var`.  The handle is tied to the circuit's
    identity and cannot be used with a different circuit.
    """
    def __init__(self, circuit_id: CircuitId, index: int, ty: ClassicalType) -> None: ...
    @property
    def id(self) -> int:
        """Stable identifier combining circuit id and index."""
        ...
    @property
    def index(self) -> int:
        """Position of this variable within its owning circuit."""
        ...
    @property
    def circuit_id(self) -> CircuitId:
        """The :class:`CircuitId` that owns this variable."""
        ...
    @property
    def ty(self) -> ClassicalType:
        """The static :class:`ClassicalType` of this variable."""
        ...
    def expr(self) -> ClassicalExpr:
        """An expression that reads this variable's current value."""
        ...
    def __repr__(self) -> str: ...
    def __copy__(self) -> ClassicalVar: ...
    def __deepcopy__(self, memo: dict) -> ClassicalVar: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class ClassicalValue:
    """Circuit-local handle to an immutable runtime classical value.

    Produced by measurement operations.  Once created the value never changes.
    """
    def __init__(self, circuit_id: CircuitId, index: int, ty: ClassicalType) -> None: ...
    @property
    def index(self) -> int:
        """Position of this value within its owning circuit."""
        ...
    @property
    def circuit_id(self) -> CircuitId:
        """The :class:`CircuitId` that owns this value."""
        ...
    @property
    def ty(self) -> ClassicalType:
        """The static :class:`ClassicalType` of this value."""
        ...
    def expr(self) -> ClassicalExpr:
        """An expression that reads this immutable value."""
        ...
    def __repr__(self) -> str: ...
    def __copy__(self) -> ClassicalValue: ...
    def __deepcopy__(self, memo: dict) -> ClassicalValue: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class Measurement:
    """Receipt from a mid-circuit measurement.

    Combines an immutable :class:`ClassicalValue` result with the ordered list
    of measured qubits.  Created by :meth:`Circuit.measure` and variants.
    """
    def __init__(self, value: ClassicalValue, qubits: list[Qubit]) -> None: ...
    @property
    def value(self) -> ClassicalValue:
        """The immutable measurement result."""
        ...
    @property
    def qubits(self) -> list[Qubit]:
        """Measured qubits in result-bit order."""
        ...
    @property
    def ty(self) -> ClassicalType:
        """The static type of the measurement result."""
        ...
    @property
    def width(self) -> int:
        """Number of measured bits."""
        ...
    def expr(self) -> ClassicalExpr:
        """An expression that reads the measurement result."""
        ...
    def check_qubits(self, num_qubits: int) -> None:
        """Check that all measured qubits are valid for a state with ``num_qubits``.

        Raises:
            ValueError: If any measured qubit index is out of range.
        """
        ...
    def project(self, full: Outcome) -> Outcome:
        """Project a full-register outcome onto this measurement's qubit order."""
        ...
    def project_basis(self, basis: int) -> Outcome:
        """Project a computational-basis index onto this measurement's qubit order."""
        ...
    def __repr__(self) -> str: ...
    def __copy__(self) -> Measurement: ...
    def __deepcopy__(self, memo: dict) -> Measurement: ...
    def __eq__(self, other: object) -> bool: ...
