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

"""Strongly typed qubit identifiers for device-facing APIs.

Circuit operations use :class:`cqlib.Qubit` as logical wire identifiers.
Device-facing code must distinguish those logical identifiers from physical
hardware positions. :class:`LogicalQubit` and :class:`PhysicalQubit` provide
that distinction without changing the compact representation.

Key Concepts
    **LogicalQubit** identifies a circuit wire crossing into device-facing
    code. The compiler resource manager allocates logical qubits; the layout
    maps them to physical positions.

    **PhysicalQubit** represents a fixed hardware position on a quantum
    device. It is not a circuit wire — layout code converts between the two.

    Both types are newtype wrappers around :class:`cqlib.Qubit`, so ``id``
    and ``qubit`` accessors return the same underlying identifier.

Display
    Logical qubits render as ``L{id}`` (e.g. ``L0``),
    physical qubits render as ``P{id}`` (e.g. ``P100``).

Key Usage
    >>> from cqlib.device import LogicalQubit, PhysicalQubit
    >>>
    >>> lq = LogicalQubit(0)
    >>> pq = PhysicalQubit(100)
    >>>
    >>> print(lq)                    # L0
    >>> print(pq)                    # P100
    >>>
    >>> lq.id  == 0                  # True
    >>> pq.id  == 100                # True
    >>>
    >>> lq.qubit                     # Qubit(0)
    >>> pq.qubit                     # Qubit(100)
"""

from cqlib import Qubit

class LogicalQubit:
    """Logical qubit identifier used when crossing into device-facing code.

    A logical qubit identifies a circuit wire. It is distinct from a
    :class:`PhysicalQubit`, even when both carry the same numeric identifier.

    Key Usage::

        >>> from cqlib.device import LogicalQubit
        >>>
        >>> lq0 = LogicalQubit(0)
        >>> lq1 = LogicalQubit(1)
        >>>
        >>> lq0.id      # 0
        >>> lq0.qubit   # Qubit(0)
        >>> str(lq0)    # 'L0'

        # Usable as dictionary keys and in sets
        >>> layout = {lq0: PhysicalQubit(100), lq1: PhysicalQubit(101)}
    """

    def __init__(self, id: int) -> None:
        """Creates a logical qubit identifier from its numeric ID.

        Args:
            id: The numeric qubit identifier (non-negative integer).

        Example::

            >>> lq = LogicalQubit(0)
        """

    @property
    def id(self) -> int:
        """Returns the numeric qubit identifier."""

    @property
    def index(self) -> int:
        """Returns the qubit identifier as an index (parity with ``Qubit.index``)."""

    @property
    def qubit(self) -> Qubit:
        """Returns the underlying circuit :class:`cqlib.Qubit`.

        The returned qubit has the same numeric identifier.
        """

    def __copy__(self) -> "LogicalQubit": ...
    def __deepcopy__(self, memo: dict) -> "LogicalQubit": ...
    def __eq__(self, value: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __lt__(self, value: object) -> bool: ...
    def __le__(self, value: object) -> bool: ...
    def __gt__(self, value: object) -> bool: ...
    def __ge__(self, value: object) -> bool: ...

class PhysicalQubit:
    """Physical qubit identifier representing a hardware position on a device.

    A physical qubit is not a circuit wire — see :class:`LogicalQubit` for
    circuit-side identifiers. Layout code is responsible for mapping logical
    qubits to physical qubits.

    Key Usage::

        >>> from cqlib.device import PhysicalQubit
        >>>
        >>> pq0 = PhysicalQubit(100)
        >>> pq1 = PhysicalQubit(101)
        >>>
        >>> pq0.id      # 100
        >>> pq0.qubit   # Qubit(100)
        >>> str(pq0)    # 'P100'

        # Physical qubits are hashable — usable as dict keys
        >>> props = {pq0: "good", pq1: "marginal"}
    """

    def __init__(self, id: int) -> None:
        """Creates a physical qubit identifier from its numeric ID.

        Args:
            id: The numeric hardware-qubit identifier (non-negative integer).

        Example::

            >>> pq = PhysicalQubit(100)
        """

    @property
    def id(self) -> int:
        """Returns the numeric hardware-qubit identifier."""

    @property
    def index(self) -> int:
        """Returns the hardware-qubit identifier as an index (parity with ``Qubit.index``)."""

    @property
    def qubit(self) -> Qubit:
        """Returns the underlying circuit :class:`cqlib.Qubit`.

        The returned qubit has the same numeric identifier.
        """

    def __copy__(self) -> "PhysicalQubit": ...
    def __deepcopy__(self, memo: dict) -> "PhysicalQubit": ...
    def __eq__(self, value: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __lt__(self, value: object) -> bool: ...
    def __le__(self, value: object) -> bool: ...
    def __gt__(self, value: object) -> bool: ...
    def __ge__(self, value: object) -> bool: ...
