# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

from __future__ import annotations

from cqlib.circuit import Qubit

class AncillaRequirement:
    """State-restoration contract required from an ancilla consumer.

    The manager records this contract but cannot prove quantum-state
    restoration. The consuming algorithm remains responsible for correctness.
    """

    @staticmethod
    def clean_zero() -> AncillaRequirement:
        """Require each qubit to enter and leave the consumer in ``|0>``."""
        ...
    @staticmethod
    def dirty() -> AncillaRequirement:
        """Allow unknown input state but require exact complete restoration."""
        ...
    def __copy__(self) -> AncillaRequirement: ...
    def __deepcopy__(self, memo: dict) -> AncillaRequirement: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class ResourceRequest:
    """Description of a temporary ancillary-qubit requirement.

    Args:
        requirement: Restoration contract accepted by the algorithm.
        count: Number of required ancillas. Validation rejects zero.
        excluded: Data, control, and target qubits that must not be borrowed.
            Integers are interpreted as logical qubit IDs. Duplicates are
            removed and returned in ascending ID order.

    Example::

        request = ResourceRequest(
            AncillaRequirement.dirty(),
            1,
            excluded=[0, 1],
        )
    """

    def __init__(
        self,
        requirement: AncillaRequirement,
        count: int,
        *,
        excluded: list[int] | list[Qubit] | None = None,
    ) -> None: ...
    @property
    def requirement(self) -> AncillaRequirement: ...
    @property
    def count(self) -> int: ...
    @property
    def excluded(self) -> list[Qubit]: ...
    def __copy__(self) -> ResourceRequest: ...
    def __deepcopy__(self, memo: dict) -> ResourceRequest: ...
    def __eq__(self, other: object) -> bool: ...

class ResourcePlan:
    """Side-effect-free allocation preview created by a resource manager.

    Plans are manager-specific snapshots. Any successful commit, release, or
    phase transition makes earlier plans stale. Instances cannot be created
    directly; call :meth:`ResourceManager.preview`.
    """

    @property
    def qubits(self) -> list[Qubit]:
        """Logical qubits selected for the prospective lease."""
        ...
    @property
    def requirement(self) -> AncillaRequirement: ...
    @property
    def num_new_qubits(self) -> int:
        """Number of logical qubits that commit must add to the circuit."""
        ...
    def __copy__(self) -> ResourcePlan: ...
    def __deepcopy__(self, memo: dict) -> ResourcePlan: ...
    def __eq__(self, other: object) -> bool: ...

class ResourceLease:
    """Credential for an active ancillary-resource reservation.

    A lease remains active until passed to ``ResourceManager.release``. Release
    asserts only bookkeeping; call it only after the consuming algorithm has
    restored the lease's quantum-state contract.
    """

    @property
    def id(self) -> int:
        """Manager-local lease identifier."""
        ...
    @property
    def qubits(self) -> list[Qubit]: ...
    @property
    def requirement(self) -> AncillaRequirement: ...
    def __copy__(self) -> ResourceLease: ...
    def __deepcopy__(self, memo: dict) -> ResourceLease: ...
    def __eq__(self, other: object) -> bool: ...

__all__ = [
    "AncillaRequirement",
    "ResourceRequest",
    "ResourcePlan",
    "ResourceLease",
]
