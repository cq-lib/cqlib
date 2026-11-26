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

"""Runtime classical data operations.

:class:`ClassicalDataOp` models side-effecting classical updates in a
circuit schedule: stores into mutable classical variables and measurements
producing immutable classical values.  Instances are obtained from
:attr:`Instruction.classical_data` or :attr:`ValueInstruction.classical_data`;
they are not constructed directly.
"""

from .classical import ClassicalValue, ClassicalVar
from .classical_expr import ClassicalExpr

class ClassicalDataOp:
    """Runtime classical data operation (store or measurement)."""
    @property
    def kind(self) -> str:
        """One of ``"store"``, ``"measure_bit"``, or ``"measure_bits"``."""
        ...
    @property
    def target(self) -> ClassicalVar | None:
        """The store target variable if this is a store operation."""
        ...
    @property
    def value(self) -> ClassicalExpr | None:
        """The stored expression if this is a store operation."""
        ...
    @property
    def result(self) -> ClassicalValue | None:
        """The immutable value receiving the result if this is a measurement."""
        ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> ClassicalDataOp: ...
    def __deepcopy__(self, memo: dict) -> ClassicalDataOp: ...
    def __repr__(self) -> str: ...
