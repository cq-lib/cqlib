# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

from __future__ import annotations

from cqlib.circuit import Circuit
from .model import ResourceLease, ResourcePlan, ResourceRequest
from .policy import ResourceLimits, ResourcePolicy

class ResourceManager:
    """Tracks compiler-visible logical qubits and temporary leases.

    A manager is initialized from one circuit and must remain synchronized with
    that circuit as both evolve. It does not own the circuit and does not prove
    that a transform restored any leased quantum state.
    """

    @staticmethod
    def from_circuit(
        circuit: Circuit,
        *,
        policy: ResourcePolicy | None = None,
        limits: ResourceLimits | None = None,
    ) -> ResourceManager:
        """Create a pre-layout manager for the circuit's current qubits.

        Existing qubits are registered as input resources, not clean ancillas.

        Raises:
            ResourceUnavailableError: If the circuit already exceeds limits.
            ResourceError: If internal manager identification is exhausted.
        """
        ...
    def preview(self, request: ResourceRequest) -> ResourcePlan:
        """Select resources without reserving them or mutating the circuit.

        Raises:
            ResourceUnavailableError: If policy, phase, or capacity cannot
                satisfy the request.
            ResourceError: If the request is invalid.
        """
        ...
    def commit(self, circuit: Circuit, plan: ResourcePlan) -> ResourceLease:
        """Reserve a plan and add its new logical qubits to ``circuit``.

        The operation mutates both this manager and the supplied circuit.

        Raises:
            ResourceError: If the plan is foreign or stale, the circuit is no
                longer synchronized, or commit violates an invariant.
        """
        ...
    def release(self, lease: ResourceLease) -> None:
        """Release a lease after its restoration contract has been satisfied.

        Released qubits stay in the circuit and can be reused.

        Raises:
            ResourceError: If the lease is foreign, unknown, or already released.
        """
        ...
    def enter_post_layout(self, circuit: Circuit) -> None:
        """Enter the one-way phase that prohibits new logical qubits.

        Raises:
            ResourceError: If the manager is not idle, is inconsistent with
                the circuit, or has already entered post-layout.
        """
        ...
    def verify_consistency(self, circuit: Circuit) -> None:
        """Check structural agreement between circuit and resource indexes.

        This does not inspect quantum state.
        """
        ...
    def verify_idle(self, circuit: Circuit) -> None:
        """Check consistency and require every lease to be released."""
        ...

__all__ = [
    "ResourceManager",
]
