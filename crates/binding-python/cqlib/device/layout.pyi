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

from cqlib import Qubit
from .qubit import LogicalQubit, PhysicalQubit

_LogicalQubitLike = int | Qubit | LogicalQubit
_PhysicalQubitLike = int | Qubit | PhysicalQubit


class Layout:
    """Maps circuit logical qubits to physical qubits on a quantum device.

    A layout owns the set of physical qubits available to a placement or
    routing step. Every logical qubit present in the layout has exactly one
    physical mapping. A physical qubit may be **vacant** (not carrying a
    logical qubit).

    Layout does **not** allocate auxiliary qubits. Auxiliary qubits are
    logical circuit resources and must be managed by the compiler resource
    manager before they are :meth:`bind` to physical qubits.

    Key Usage — basic mapping::

        >>> from cqlib.device import Layout
        >>>
        >>> # 2 logical qubits on 4 physical qubits
        >>> layout = Layout([0, 1], [100, 101, 102, 103])
        >>>
        >>> layout.num_logical          # 2
        >>> layout.num_vacant_physical  # 2
        >>> layout.num_physical         # 4
        >>>
        >>> layout.get_physical(0)  # PhysicalQubit(100) or similar
        >>> layout.get_logical(100) # LogicalQubit(0) or similar

    Key Usage — custom initial mapping::

        >>> init_map = {0: 101, 1: 100}
        >>> layout = Layout([0, 1], [100, 101, 102], init_map=init_map)
        >>>
        >>> layout.get_physical(0)  # PhysicalQubit(101)

    Key Usage — from pairs::

        >>> layout = Layout.from_pairs([(0, 2), (1, 0)], physical_count=4)
        >>> # Logical 0 → Physical 2, Logical 1 → Physical 0
        >>> # Physical 1, 3 are vacant

    Key Usage — bind/unbind during routing::

        >>> # Bind a new logical qubit to a vacant physical qubit
        >>> layout.bind(2, 102)
        >>>
        >>> # Release a logical qubit
        >>> freed = layout.unbind(2)  # returns the released physical qubit
    """

    def __init__(
        self,
        logical: list[_LogicalQubitLike],
        physical: list[_PhysicalQubitLike],
        init_map: dict[_LogicalQubitLike, _PhysicalQubitLike] | None = None,
    ) -> None:
        """Creates a new layout mapping logical to physical qubits.

        Entries in ``init_map`` are applied first. Remaining logical
        qubits are mapped to remaining physical qubits in the order
        supplied by ``logical`` and ``physical``. Extra physical qubits
        remain vacant.

        Args:
            logical: List of logical qubit identifiers.
            physical: List of physical qubit identifiers.
            init_map: Optional initial mapping from logical to physical.

        Raises:
            ValueError: If logical qubits exceed physical, duplicates
                exist, or init_map is invalid.
        """

    @staticmethod
    def from_pairs(
        pairs: list[tuple[int, int]], physical_count: int
    ) -> "Layout":
        """Create a layout from ``(logical_id, physical_id)`` pairs.

        Logical qubits are the logical IDs in ``pairs``. Physical
        qubits are ``0..physical_count-1``; unreferenced physical qubits
        remain vacant.

        Args:
            pairs: List of ``(logical_id, physical_id)`` pairs.
            physical_count: Total number of physical qubits.

        Raises:
            ValueError: If a logical ID or physical ID appears more
                than once in pairs, or any physical ID ≥ physical_count.
        """

    # ---- Size queries ----

    @property
    def num_logical(self) -> int:
        """Number of mapped logical qubits."""

    @property
    def num_physical(self) -> int:
        """Number of physical qubits available to the layout."""

    @property
    def num_vacant_physical(self) -> int:
        """Number of physical qubits not currently carrying a logical qubit."""

    # ---- Lookup methods ----

    def get_physical(self, logical_id: _LogicalQubitLike) -> PhysicalQubit | None:
        """Get the physical qubit mapped to a logical qubit.

        Returns ``None`` if the logical qubit is not bound. Use the
        ``.qubit`` property on the result for an explicit bridge to a
        circuit :class:`cqlib.Qubit`.

        Args:
            logical_id: The logical qubit to look up.
        """

    def get_logical(self, physical_id: _PhysicalQubitLike) -> LogicalQubit | None:
        """Get the logical qubit carried by a physical qubit.

        Returns ``None`` if the physical qubit is vacant. Use the
        ``.qubit`` property on the result for an explicit bridge to a
        circuit :class:`cqlib.Qubit`.

        Args:
            physical_id: The physical qubit to look up.
        """

    # ---- Iterators ----

    @property
    def logical_qubits(self) -> list[LogicalQubit]:
        """All mapped logical qubits."""

    @property
    def physical_qubits(self) -> list[PhysicalQubit]:
        """All physical qubits available to the layout."""

    @property
    def vacant_physical_qubits(self) -> list[PhysicalQubit]:
        """All vacant physical qubits (not carrying a logical qubit)."""

    def is_physical_vacant(self, physical_id: _PhysicalQubitLike) -> bool:
        """Check whether a physical qubit is in the layout and vacant.

        Args:
            physical_id: The physical qubit to check.
        """

    # ---- Mapping dictionaries ----

    @property
    def l2p_map(self) -> dict[LogicalQubit, PhysicalQubit]:
        """The logical-to-physical qubit mapping.

        Returns a dict mapping each logical qubit to its physical qubit.
        """

    @property
    def p2l_map(self) -> dict[PhysicalQubit, LogicalQubit]:
        """The physical-to-logical qubit mapping.

        Returns a dict mapping each occupied physical qubit to its
        logical qubit. Vacant physical qubits are excluded.
        """

    # ---- Mutation (routing operations) ----

    def bind(self, logical_id: _LogicalQubitLike, physical_id: _PhysicalQubitLike) -> None:
        """Bind an unmapped logical qubit to a vacant physical qubit.

        May introduce a new logical qubit to the layout. The caller must
        ensure the logical qubit is registered with the compiler resource
        manager when required.

        Args:
            logical_id: The logical qubit to bind.
            physical_id: The vacant physical qubit to bind it to.

        Raises:
            ValueError: If the physical qubit is not in the layout, or
                either qubit already participates in a mapping.
        """

    def unbind(self, logical_id: _LogicalQubitLike) -> PhysicalQubit:
        """Remove the mapping for a logical qubit and return the released
        physical qubit.

        Args:
            logical_id: The logical qubit to unbind.

        Returns:
            The physical qubit that was released (now vacant).

        Raises:
            ValueError: If the logical qubit is not bound.
        """

    def swap_physical(
        self, phys_a: _PhysicalQubitLike, phys_b: _PhysicalQubitLike
    ) -> None:
        """Swap the logical qubits carried by two physical qubits.

        Core routing operation. After a SWAP gate on hardware, call this
        to exchange logical qubit positions. Either physical qubit may
        be vacant — swapping occupied→vacant moves the logical qubit.

        Args:
            phys_a: First physical qubit.
            phys_b: Second physical qubit.

        Raises:
            ValueError: If either physical qubit is not in the layout.
        """

    def __copy__(self) -> "Layout": ...
    def __deepcopy__(self, memo: dict) -> "Layout": ...
