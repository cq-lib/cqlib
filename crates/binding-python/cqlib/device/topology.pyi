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
from .qubit import PhysicalQubit

_PhysicalQubitLike = int | Qubit | PhysicalQubit


class Topology:
    """A directed coupling graph representing quantum hardware connectivity.

    Each node represents a physical qubit, and each directed edge
    represents a coupling between qubits (e.g., for two-qubit gates
    like CNOT).

    **Directed**: A coupling ``a → b`` does **not** imply ``b → a``.
    This accurately models hardware where gates only work in specific
    control-target directions.

    Key Usage — construction::

        >>> from cqlib.device import Topology
        >>>
        >>> # Directed couplings: 0 → 1, 1 → 2, 2 → 3
        >>> topo = Topology([0, 1, 2, 3], [(0, 1, "CX"), (1, 2, "CX"), (2, 3, "CX")])
        >>>
        >>> topo.num_qubits     # 4
        >>> topo.num_couplings  # 3

    Key Usage — line factory::

        >>> topo = Topology.line([0, 1, 2, 3])
        >>> # Creates: 0 → 1, 1 → 2, 2 → 3

    Key Usage — query connectivity::

        >>> topo.supports_directed_coupling(0, 1)          # True
        >>> topo.supports_directed_coupling(1, 0)          # False
        >>> topo.supports_coupling_either_direction(0, 1)  # True (1→0 may exist separately)
        >>>
        >>> topo.successors(1)        # [PhysicalQubit(2)]  — outgoing
        >>> topo.predecessors(1)      # [PhysicalQubit(0)]  — incoming
        >>> topo.neighbors_undirected(1)  # [PhysicalQubit(0), PhysicalQubit(2)]
        >>>
        >>> topo.out_degree(0)  # 1
        >>> topo.in_degree(2)   # 1

    Key Usage — mutation::

        >>> topo.add_qubits([4])
        >>> topo.add_couplings([(3, 4, "CX")])
        >>> topo.remove_couplings([(0, 1)])
        >>> topo.remove_qubits([3])
    """

    def __init__(
        self,
        qubits: list[_PhysicalQubitLike],
        couplings: list[tuple[_PhysicalQubitLike, _PhysicalQubitLike, str]],
    ) -> None:
        """Create a topology with given qubits and directed couplings.

        Args:
            qubits: List of qubit identifiers. Duplicates are rejected.
            couplings: List of ``(control, target, name)`` tuples. Each
                tuple defines a directed coupling with an informational
                label (e.g., ``"CX"``, ``"CZ"``).

        Raises:
            ValueError: If duplicate qubits exist, a coupling references
                a non-existent qubit, or a self-coupling is requested.
        """

    @staticmethod
    def line(qubits: list[_PhysicalQubitLike]) -> "Topology":
        """Create a directed line topology.

        Couplings: ``qubits[0] → qubits[1] → ... → qubits[n-1]``.

        Args:
            qubits: List of qubit IDs in line order.

        Empty and one-qubit lines are valid.
        """

    # ---- Size properties ----

    @property
    def num_qubits(self) -> int:
        """Number of physical qubits."""

    @property
    def num_couplings(self) -> int:
        """Number of directed coupling edges."""

    @property
    def qubits(self) -> list[PhysicalQubit]:
        """All physical qubits in the topology."""

    # ---- Mutation methods ----

    def add_qubits(self, qubits: list[_PhysicalQubitLike]) -> None:
        """Add physical qubits to the topology.

        Raises:
            ValueError: If any qubit already exists.
        """

    def add_couplings(
        self,
        couplings: list[tuple[_PhysicalQubitLike, _PhysicalQubitLike, str]],
    ) -> None:
        """Add directed couplings.

        Each coupling is directed from the first qubit to the second.

        Raises:
            ValueError: If an endpoint qubit is missing, the coupling
                already exists, or a self-coupling is requested.
        """

    def remove_qubits(self, qubits: list[_PhysicalQubitLike]) -> None:
        """Remove qubits and all their incident couplings.

        Raises:
            ValueError: If any qubit does not exist.
        """

    def remove_couplings(
        self, couplings: list[tuple[_PhysicalQubitLike, _PhysicalQubitLike]]
    ) -> None:
        """Remove specific directed couplings.

        Only removes the specified direction. The reverse coupling
        (if present) is not affected.

        Raises:
            ValueError: If a coupling or its endpoints are missing.
        """

    # ---- Connectivity queries (directed) ----

    def supports_directed_coupling(
        self, control: _PhysicalQubitLike, target: _PhysicalQubitLike
    ) -> bool:
        """Check for a directed coupling ``control → target``.

        Does **not** check the reverse direction. Use
        :meth:`supports_coupling_either_direction` for bidirectional checks.

        Args:
            control: Source qubit.
            target: Destination qubit.

        Returns:
            ``True`` if the directed coupling exists.
        """

    def supports_coupling_either_direction(
        self, a: _PhysicalQubitLike, b: _PhysicalQubitLike
    ) -> bool:
        """Check for a coupling in either direction.

        Returns ``True`` if ``a → b`` or ``b → a`` (or both) exist.

        Args:
            a: First qubit.
            b: Second qubit.
        """

    # ---- Neighbor queries ----

    def successors(self, qubit: _PhysicalQubitLike) -> list[PhysicalQubit]:
        """All qubits reachable via outgoing couplings from ``qubit``.

        Args:
            qubit: Source qubit.

        Example::

            >>> topo = Topology([0, 1, 2], [(0, 1, ""), (0, 2, "")])
            >>> topo.successors(0)  # [PhysicalQubit(1), PhysicalQubit(2)]
        """

    def predecessors(self, qubit: _PhysicalQubitLike) -> list[PhysicalQubit]:
        """All qubits with incoming couplings to ``qubit``.

        Args:
            qubit: Target qubit.
        """

    def neighbors_undirected(self, qubit: _PhysicalQubitLike) -> list[PhysicalQubit]:
        """All qubits coupled to ``qubit`` in either direction.

        Bidirectional couplings are deduplicated.

        Args:
            qubit: The qubit to query.
        """

    def undirected_edges(self) -> list[tuple[PhysicalQubit, PhysicalQubit]]:
        """All unique coupling pairs ignoring direction.

        Pairs are ordered by qubit ID. Bidirectional couplings collapse
        to a single pair.

        Returns:
            List of unique ``(PhysicalQubit, PhysicalQubit)`` pairs.
        """

    # ---- Metadata queries ----

    def get_coupling_name(
        self, control: _PhysicalQubitLike, target: _PhysicalQubitLike
    ) -> str | None:
        """Get the name of a directed coupling.

        Args:
            control: Source qubit.
            target: Destination qubit.

        Returns:
            The coupling name string, or ``None`` if it does not exist.
        """

    def contains_qubit(self, qubit: _PhysicalQubitLike) -> bool:
        """Check if a qubit exists in the topology."""

    def out_degree(self, qubit: _PhysicalQubitLike) -> int:
        """Number of outgoing couplings from a qubit.

        Returns 0 if the qubit does not exist.
        """

    def in_degree(self, qubit: _PhysicalQubitLike) -> int:
        """Number of incoming couplings to a qubit.

        Returns 0 if the qubit does not exist.
        """

    def __copy__(self) -> "Topology": ...
    def __deepcopy__(self, memo: dict) -> "Topology": ...
