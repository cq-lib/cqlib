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

"""Construction-time classical control-flow operations.

These types model structured classical control flow (``if``, ``while``,
``for``, ``switch``) for dynamic circuits.  Build ops with
:class:`ClassicalControlOp` static methods, then add them to a circuit with
:meth:`~circuit.Circuit.append_control`.

Example::

    from cqlib import Circuit, ClassicalType
    from cqlib.circuit import ClassicalControlOp, ValueControlBody

    c = Circuit(2)
    cond = c.var(ClassicalType.bool())
    # ... build condition expression ...
    body = ValueControlBody([...])  # operations for then-branch
    if_op = ClassicalControlOp.if_(cond.expr().to_bool(), body)
    c.append_control(if_op)
"""

from .classical import ClassicalValue, ClassicalVar
from .classical_expr import ClassicalExpr
from .operation import ValueOperation

class ValueControlBody:
    """Ordered construction-time operations owned by one control-flow region.

    A body is a flat sequence of :class:`ValueOperation` objects.  It is
    consumed by :class:`ClassicalControlOp` factory methods.
    """
    def __init__(self, operations: list[ValueOperation]) -> None: ...
    @property
    def operations(self) -> list[ValueOperation]:
        """The operations in this body."""
        ...
    def __len__(self) -> int: ...
    def has_measurement(self) -> bool:
        """Return ``True`` if this body directly or recursively contains measurement."""
        ...
    def reads_value(self, value: ClassicalValue) -> bool:
        """Return ``True`` if this body directly or recursively reads ``value``."""
        ...
    def __copy__(self) -> ValueControlBody: ...
    def __deepcopy__(self, memo: dict) -> ValueControlBody: ...
    def __repr__(self) -> str: ...

class ValueSwitchCase:
    """Exact integer match and body used by a construction-time switch."""
    def __init__(self, value: int, body: ValueControlBody) -> None: ...
    @property
    def value(self) -> int:
        """The integer value this case matches."""
        ...
    @property
    def body(self) -> ValueControlBody:
        """The body executed when this case matches."""
        ...
    def __copy__(self) -> ValueSwitchCase: ...
    def __deepcopy__(self, memo: dict) -> ValueSwitchCase: ...
    def __repr__(self) -> str: ...

class ClassicalControlOp:
    """Construction-time classical control-flow operation.

    Not a quantum gate — has no unitary matrix representation.  Use static
    factory methods to build, then add to a circuit via
    :meth:`~circuit.Circuit.append_control`.
    """
    @staticmethod
    def if_(condition: ClassicalExpr, then_body: ValueControlBody, else_body: ValueControlBody | None = ...) -> ClassicalControlOp:
        """Build an ``if`` / ``if-else`` operation.

        Args:
            condition: Bool-typed :class:`ClassicalExpr`.
            then_body: Operations executed when condition is true.
            else_body: Optional operations executed when condition is false.
        """
        ...
    @staticmethod
    def while_(condition: ClassicalExpr, body: ValueControlBody) -> ClassicalControlOp:
        """Build a ``while`` loop operation.

        Args:
            condition: Bool-typed :class:`ClassicalExpr`.
            body: Loop body operations.
        """
        ...
    @staticmethod
    def for_uint(var: ClassicalVar, start: ClassicalExpr, stop: ClassicalExpr, step: ClassicalExpr, body: ValueControlBody) -> ClassicalControlOp:
        """Build an unsigned runtime range ``for`` loop.

        Args:
            var: UInt-typed loop variable.
            start: Initial value expression (UInt).
            stop: Half-open upper bound expression (UInt).
            step: Increment expression (UInt).
            body: Loop body operations.

        Raises:
            CircuitError: If ``var`` is not ``UInt``, or ``start``/``stop``/
                ``step`` do not match its width.
        """
        ...
    @staticmethod
    def switch(target: ClassicalExpr, cases: list[ValueSwitchCase], default: ValueControlBody | None = ...) -> ClassicalControlOp:
        """Build a ``switch`` operation over a UInt expression.

        Args:
            target: UInt-typed switch expression.
            cases: List of :class:`ValueSwitchCase` values.
            default: Optional fallback body.

        Raises:
            CircuitError: If ``target`` is not ``UInt``, a case value does not
                fit the target width, or case values are duplicated.
        """
        ...
    @staticmethod
    def break_() -> ClassicalControlOp:
        """Build a ``break`` statement (exits the innermost loop)."""
        ...
    @staticmethod
    def continue_() -> ClassicalControlOp:
        """Build a ``continue`` statement (jumps to next iteration)."""
        ...
    @property
    def kind(self) -> str:
        """One of ``"if"``, ``"while"``, ``"for"``, ``"switch"``, ``"break"``, ``"continue"``."""
        ...
    @property
    def condition(self) -> ClassicalExpr | None:
        """The condition expression (``if``, ``while``)."""
        ...
    @property
    def then_body(self) -> ValueControlBody | None:
        """The then-branch body (``if``)."""
        ...
    @property
    def else_body(self) -> ValueControlBody | None:
        """The else-branch body (``if`` with else)."""
        ...
    @property
    def body(self) -> ValueControlBody | None:
        """The loop body (``while``, ``for``)."""
        ...
    @property
    def var(self) -> ClassicalVar | None:
        """The loop variable (``for``)."""
        ...
    @property
    def start(self) -> ClassicalExpr | None:
        """The start expression (``for``)."""
        ...
    @property
    def stop(self) -> ClassicalExpr | None:
        """The stop expression (``for``)."""
        ...
    @property
    def step(self) -> ClassicalExpr | None:
        """The step expression (``for``)."""
        ...
    @property
    def target(self) -> ClassicalExpr | None:
        """The switch target expression (``switch``)."""
        ...
    @property
    def cases(self) -> list[ValueSwitchCase]:
        """The switch cases (``switch``)."""
        ...
    @property
    def default(self) -> ValueControlBody | None:
        """The default fallback body (``switch``)."""
        ...
    def has_measurement(self) -> bool:
        """Return ``True`` if this control operation recursively contains measurement."""
        ...
    def reads_value(self, value: ClassicalValue) -> bool:
        """Return ``True`` if this control operation recursively reads ``value``."""
        ...
    def __copy__(self) -> ClassicalControlOp: ...
    def __deepcopy__(self, memo: dict) -> ClassicalControlOp: ...
    def __repr__(self) -> str: ...
