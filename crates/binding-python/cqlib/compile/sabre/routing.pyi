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

from __future__ import annotations

from collections.abc import Sequence

from cqlib.circuit import Circuit, Qubit
from cqlib.device import Device, Layout, LogicalQubit

_LogicalQubitLike = int | Qubit | LogicalQubit

class SabreHeuristicConfig:
    """Local SWAP-selection weights and release-valve limits.

    Structural distance uses active-layer-normalized lookahead and
    multiplicative congestion control. Exact native 2Q cost resolves
    candidates within a narrow structural window.
    """

    def __init__(
        self,
        *,
        basic_weight: float = 1.0,
        lookahead_weights: Sequence[float] | None = None,
        decay_increment: float | None = 0.002,
        decay_reset: int = 10,
        attempt_limit: int = 1000,
        best_epsilon: float = 1e-10,
    ) -> None: ...
    @property
    def basic_weight(self) -> float: ...
    @property
    def lookahead_weights(self) -> list[float]: ...
    @property
    def decay_increment(self) -> float | None: ...
    @property
    def decay_reset(self) -> int: ...
    @property
    def attempt_limit(self) -> int: ...
    @property
    def best_epsilon(self) -> float: ...
    def __copy__(self) -> SabreHeuristicConfig: ...
    def __deepcopy__(self, memo: dict[int, object]) -> SabreHeuristicConfig: ...
    def __eq__(self, other: object) -> bool: ...

class SabreVf2PrepassConfig:
    """Bounded VF2 prepass used to seed topology-perfect candidates."""

    def __init__(
        self, *, candidate_limit: int = 10, call_limit: int = 1_000_000
    ) -> None: ...
    @property
    def candidate_limit(self) -> int: ...
    @property
    def call_limit(self) -> int: ...
    def __copy__(self) -> SabreVf2PrepassConfig: ...
    def __deepcopy__(self, memo: dict[int, object]) -> SabreVf2PrepassConfig: ...
    def __eq__(self, other: object) -> bool: ...

class SabreConfig:
    """Configuration shared by SABRE layout refinement and routing."""

    def __init__(
        self,
        *,
        layout_trials: int = 10,
        layout_assignment_budget: int = 1_000_000,
        vf2_prepass: SabreVf2PrepassConfig | None = ...,
        refinement_iterations: int = 1,
        routing_trials: int = 1,
        seed: int | None = None,
        heuristic: SabreHeuristicConfig | None = None,
    ) -> None: ...
    @staticmethod
    def deterministic_seeded(seed: int) -> SabreConfig: ...
    @property
    def layout_trials(self) -> int: ...
    @property
    def layout_assignment_budget(self) -> int: ...
    @property
    def vf2_prepass(self) -> SabreVf2PrepassConfig | None: ...
    @property
    def refinement_iterations(self) -> int: ...
    @property
    def routing_trials(self) -> int:
        """Complete routing trials run for each fully refined layout.

        The search directly returns the best route and does not route
        intermediate refinement states or run a separate layout-scoring
        phase.
        """
        ...
    @property
    def seed(self) -> int | None: ...
    @property
    def heuristic(self) -> SabreHeuristicConfig: ...
    def validate(self) -> None:
        """Validate fields used by routing.

        Raises:
            CompilerConfigError: If a routing field is invalid.
        """
        ...
    def __copy__(self) -> SabreConfig: ...
    def __deepcopy__(self, memo: dict[int, object]) -> SabreConfig: ...
    def __eq__(self, other: object) -> bool: ...

class SabreRoutingDiagnostics:
    """Read-only topology, native-cost, and cache diagnostics."""

    @property
    def trials_evaluated(self) -> int: ...
    @property
    def selected_trial_index(self) -> int: ...
    @property
    def fallback_count(self) -> int: ...
    @property
    def control_flow_blocks_routed(self) -> int: ...
    @property
    def two_qubit_depth(self) -> int: ...
    @property
    def operation_count(self) -> int: ...
    @property
    def native_two_qubit_count(self) -> int: ...
    @property
    def native_two_qubit_depth(self) -> int: ...
    @property
    def native_total_depth(self) -> int: ...
    @property
    def native_operation_count(self) -> int: ...
    @property
    def unknown_loop_count(self) -> int: ...
    @property
    def requirement_signature_count(self) -> int: ...
    @property
    def eager_pair_state_count(self) -> int: ...
    @property
    def lazy_pair_l1_lookup_count(self) -> int: ...
    @property
    def lazy_pair_l1_hit_count(self) -> int: ...
    @property
    def lazy_pair_l1_cached_count(self) -> int: ...
    def __copy__(self) -> SabreRoutingDiagnostics: ...
    def __deepcopy__(self, memo: dict[int, object]) -> SabreRoutingDiagnostics: ...
    def __eq__(self, other: object) -> bool: ...

class SabreRoutingResult:
    @property
    def circuit(self) -> Circuit: ...
    @property
    def initial_layout(self) -> Layout: ...
    @property
    def final_layout(self) -> Layout: ...
    @property
    def swap_count(self) -> int: ...
    @property
    def diagnostics(self) -> SabreRoutingDiagnostics: ...
    def __copy__(self) -> SabreRoutingResult: ...
    def __deepcopy__(self, memo: dict[int, object]) -> SabreRoutingResult: ...

def sabre_route(
    circuit: Circuit,
    device: Device,
    initial_layout: Layout,
    *,
    config: SabreConfig | None = None,
) -> SabreRoutingResult:
    """Route from a supplied layout using topology-aware SABRE.

    Raises:
        CompilerConfigError: If the configuration or inputs are invalid.
        CompilerTransformError: If no feasible route exists.
    """
    ...

def normalize_initial_layout(
    logical_qubits: Sequence[_LogicalQubitLike],
    device: Device,
    initial_layout: Layout,
) -> Layout:
    """Normalize a complete mapping against all usable physical qubits.

    Raises:
        CompilerConfigError: If the layout is incomplete or incompatible.
    """
    ...

def validate_reachable_interactions(
    circuit: Circuit,
    device: Device,
    initial_layout: Layout,
) -> None:
    """Validate topology movement and terminal reachability without routing.

    Raises:
        CompilerConfigError: If the inputs are invalid.
        CompilerTransformError: If an interaction cannot be reached.
    """
    ...

__all__ = [
    "SabreHeuristicConfig",
    "SabreVf2PrepassConfig",
    "SabreConfig",
    "SabreRoutingDiagnostics",
    "SabreRoutingResult",
    "sabre_route",
    "normalize_initial_layout",
    "validate_reachable_interactions",
]
