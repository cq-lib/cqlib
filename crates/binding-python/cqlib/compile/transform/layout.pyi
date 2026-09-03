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

"""Initial logical-to-physical qubit layout selection."""

from __future__ import annotations

from cqlib.circuit import Circuit, Qubit, StandardGate
from cqlib.compile.sabre import SabreConfig
from cqlib.device import Device, Layout, LogicalQubit, PhysicalQubit

_PhysicalQubitLike = int | Qubit | PhysicalQubit

class LayoutObjective:
    """Weighted objective used to rank candidate initial layouts.

    Lower scores are better. Weight validation occurs when an algorithm scores
    a layout; every weight must then be finite and non-negative.
    """

    def __init__(
        self,
        *,
        distance_weight: float = 1.0,
        direction_weight: float = 1.0,
        two_qubit_error_weight: float = 0.0,
        readout_error_weight: float = 0.0,
    ) -> None:
        """Create an objective from explicit component weights."""
        ...
    @staticmethod
    def topology_only() -> LayoutObjective:
        """Return the topology-only objective."""
        ...
    @staticmethod
    def fidelity_aware() -> LayoutObjective:
        """Return the default fidelity-aware objective.

        Missing calibration entries contribute zero rather than failing.
        """
        ...
    @staticmethod
    def auto_from_device(device: Device) -> LayoutObjective:
        """Use fidelity scoring when ``device`` has usable calibration data.

        Otherwise this returns :meth:`topology_only`.

        Raises:
            CompilerConfigError: If the device cannot be converted into a
                usable physical layout graph.
        """
        ...
    @staticmethod
    def auto_from_physical(physical: PhysicalLayoutGraph) -> LayoutObjective:
        """Use fidelity scoring when a prepared graph has calibration data."""
        ...
    @staticmethod
    def fidelity_required(device: Device) -> LayoutObjective:
        """Return a fidelity-aware objective and require calibration data.

        Raises:
            CompilerConfigError: If the device is invalid or has no usable
                fidelity data.
        """
        ...
    @staticmethod
    def fidelity_required_from_physical(
        physical: PhysicalLayoutGraph,
    ) -> LayoutObjective:
        """Require usable calibration data in a prepared physical graph.

        Raises:
            CompilerConfigError: If the graph has no usable fidelity data.
        """
        ...
    @property
    def distance_weight(self) -> float:
        """Weight for logical-interaction distance."""
        ...
    @property
    def direction_weight(self) -> float:
        """Weight for directed-coupling mismatch."""
        ...
    @property
    def two_qubit_error_weight(self) -> float:
        """Weight for known two-qubit error rates."""
        ...
    @property
    def readout_error_weight(self) -> float:
        """Weight for known readout error rates."""
        ...
    @property
    def uses_fidelity(self) -> bool:
        """Whether either fidelity component can affect scoring."""
        ...
    def score_layout(
        self,
        analysis: CircuitLayoutAnalysis,
        physical: PhysicalLayoutGraph,
        layout: Layout,
    ) -> LayoutScore:
        """Score a complete mapping against prepared inputs.

        Raises:
            CompilerConfigError: If weights or the mapping are invalid.
        """
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> LayoutObjective: ...
    def __deepcopy__(self, memo: dict[int, object]) -> LayoutObjective: ...

class LayoutScore:
    """Breakdown of a candidate layout score."""

    @property
    def total(self) -> float:
        """Weighted sum of all score components."""
        ...
    @property
    def distance(self) -> float:
        """Raw weighted logical-interaction distance."""
        ...
    @property
    def direction(self) -> float:
        """Raw direction-mismatch component."""
        ...
    @property
    def two_qubit_error(self) -> float:
        """Raw two-qubit error component."""
        ...
    @property
    def readout_error(self) -> float:
        """Raw readout error component."""
        ...
    @property
    def used_fidelity(self) -> bool:
        """Whether the objective was configured to use fidelity terms."""
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> LayoutScore: ...
    def __deepcopy__(self, memo: dict[int, object]) -> LayoutScore: ...

class LayoutDiagnostics:
    """Search and scoring diagnostics from a layout algorithm."""

    @property
    def is_perfect(self) -> bool:
        """Whether all positive interactions map to adjacent qubits."""
        ...
    @property
    def candidates_evaluated(self) -> int:
        """Number of candidates considered using the algorithm's search unit."""
        ...
    @property
    def used_fidelity(self) -> bool:
        """Whether fidelity data contributed to the selected score."""
        ...
    @property
    def notes(self) -> list[str]:
        """Copy of human-readable diagnostic notes."""
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> LayoutDiagnostics: ...
    def __deepcopy__(self, memo: dict[int, object]) -> LayoutDiagnostics: ...

class LayoutResult:
    """Selected initial layout, observed score, and diagnostics."""

    @property
    def layout(self) -> Layout:
        """Selected logical-to-physical mapping."""
        ...
    @property
    def score(self) -> LayoutScore | None:
        """Observed score of this layout under the requested objective.

        Individual algorithms may use a different selection key. In
        particular, SABRE selects its winner by predicted native route
        quality and reports this score for diagnostics.
        """
        ...
    @property
    def diagnostics(self) -> LayoutDiagnostics:
        """Diagnostics emitted by the layout algorithm."""
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> LayoutResult: ...
    def __deepcopy__(self, memo: dict[int, object]) -> LayoutResult: ...

class Vf2EdgeRequirement:
    """Select which logical interactions are hard VF2 constraints."""

    @staticmethod
    def positive_interactions() -> Vf2EdgeRequirement:
        """Require interactions with positive accumulated weight."""
        ...
    @staticmethod
    def all_interactions() -> Vf2EdgeRequirement:
        """Require every stored interaction."""
        ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __copy__(self) -> Vf2EdgeRequirement: ...
    def __deepcopy__(self, memo: dict[int, object]) -> Vf2EdgeRequirement: ...

class Vf2LayoutConfig:
    """Configuration for VF2 perfect-layout search.

    Validation occurs when :func:`vf2_perfect_layout` runs.
    """

    def __init__(
        self,
        *,
        candidate_limit: int = 10,
        call_limit: int | None = None,
        edge_requirement: Vf2EdgeRequirement | None = None,
    ) -> None:
        """Create a VF2 configuration using Core defaults when omitted."""
        ...
    @property
    def candidate_limit(self) -> int:
        """Maximum number of complete candidates to score."""
        ...
    @property
    def call_limit(self) -> int | None:
        """Maximum partial mapping extensions, or ``None`` for no limit."""
        ...
    @property
    def edge_requirement(self) -> Vf2EdgeRequirement:
        """Hard interaction constraint used by VF2."""
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> Vf2LayoutConfig: ...
    def __deepcopy__(self, memo: dict[int, object]) -> Vf2LayoutConfig: ...

class Interaction:
    """One weighted logical interaction in deterministic endpoint order."""

    @property
    def left(self) -> LogicalQubit: ...
    @property
    def right(self) -> LogicalQubit: ...
    @property
    def weight(self) -> float: ...
    @property
    def directed_weight_left_to_right(self) -> float: ...
    @property
    def directed_weight_right_to_left(self) -> float: ...
    @property
    def first_seen_order(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> Interaction: ...
    def __deepcopy__(self, memo: dict[int, object]) -> Interaction: ...

class InteractionGraph:
    """Deterministically ordered logical interaction graph."""

    def __init__(self) -> None: ...
    @property
    def interactions(self) -> list[Interaction]: ...
    def __len__(self) -> int: ...
    def is_empty(self) -> bool: ...
    def logical_activity(self) -> list[tuple[LogicalQubit, float]]: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> InteractionGraph: ...
    def __deepcopy__(self, memo: dict[int, object]) -> InteractionGraph: ...

class CircuitLayoutAnalysis:
    """Reusable circuit-side summary for layout selection."""

    @property
    def logical_qubits(self) -> list[LogicalQubit]: ...
    @property
    def interactions(self) -> InteractionGraph: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> CircuitLayoutAnalysis: ...
    def __deepcopy__(self, memo: dict[int, object]) -> CircuitLayoutAnalysis: ...

class DistanceTable:
    """All-pairs undirected distances over usable physical qubits."""

    @property
    def qubits(self) -> list[PhysicalQubit]: ...
    def distance(self, a: _PhysicalQubitLike, b: _PhysicalQubitLike) -> int | None: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> DistanceTable: ...
    def __deepcopy__(self, memo: dict[int, object]) -> DistanceTable: ...

class PhysicalLayoutGraph:
    """Compiler-local usable topology and calibration view."""

    @staticmethod
    def from_device(device: Device) -> PhysicalLayoutGraph: ...
    @property
    def physical_qubits(self) -> list[PhysicalQubit]: ...
    @property
    def distances(self) -> DistanceTable: ...
    def distance(self, a: _PhysicalQubitLike, b: _PhysicalQubitLike) -> int | None: ...
    def is_adjacent_undirected(
        self, a: _PhysicalQubitLike, b: _PhysicalQubitLike
    ) -> bool: ...
    def readout_error(self, qubit: _PhysicalQubitLike) -> float | None: ...
    def supports_two_qubit_gate_directed(
        self,
        control: _PhysicalQubitLike,
        target: _PhysicalQubitLike,
        gate: StandardGate,
    ) -> bool: ...
    def two_qubit_gate_error_directed(
        self,
        control: _PhysicalQubitLike,
        target: _PhysicalQubitLike,
        gate: StandardGate,
    ) -> float | None: ...
    def supports_directed_coupling(
        self, control: _PhysicalQubitLike, target: _PhysicalQubitLike
    ) -> bool: ...
    @property
    def has_fidelity_data(self) -> bool: ...
    @property
    def has_readout_error_data(self) -> bool: ...
    @property
    def has_two_qubit_error_data(self) -> bool: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> PhysicalLayoutGraph: ...
    def __deepcopy__(self, memo: dict[int, object]) -> PhysicalLayoutGraph: ...

class PreparedSabreCircuit:
    """Circuit-side SABRE data prepared for repeated layout searches."""

    @property
    def analysis(self) -> CircuitLayoutAnalysis: ...
    @property
    def logical_qubits(self) -> list[LogicalQubit]: ...
    def __copy__(self) -> PreparedSabreCircuit: ...
    def __deepcopy__(self, memo: dict[int, object]) -> PreparedSabreCircuit: ...

class PreparedSabreTarget:
    """Prepared SABRE target with fixed routing semantics."""

    @property
    def physical(self) -> PhysicalLayoutGraph: ...
    def __copy__(self) -> PreparedSabreTarget: ...
    def __deepcopy__(self, memo: dict[int, object]) -> PreparedSabreTarget: ...

def analyze_circuit_for_layout(circuit: Circuit) -> CircuitLayoutAnalysis:
    """Analyze logical qubits and weighted interactions once.

    Raises:
        CompilerConfigError: If the circuit contains an unsupported operation.
    """
    ...

def prepare_sabre_circuit(circuit: Circuit) -> PreparedSabreCircuit:
    """Prepare reusable circuit-side data for repeated SABRE searches."""
    ...

def prepare_sabre_topology_target(
    prepared: PreparedSabreCircuit,
    device: Device,
) -> PreparedSabreTarget:
    """Prepare topology-only device data for one circuit.

    This path considers physical connectivity and does not construct native
    implementation plans.

    Raises:
        CompilerConfigError: If the device or circuit requirements are invalid.
    """
    ...

def prepare_sabre_device_target(
    prepared: PreparedSabreCircuit,
    device: Device,
) -> PreparedSabreTarget:
    """Prepare exact device-native data for one circuit.

    Raises:
        CompilerConfigError: If the device or circuit requirements are invalid.
        CompilerTransformError: If no exact native lowering plan is available.
    """
    ...

def sabre_layout_prepared(
    prepared: PreparedSabreCircuit,
    prepared_target: PreparedSabreTarget,
    objective: LayoutObjective | None = None,
    config: SabreConfig | None = None,
) -> LayoutResult:
    """Run SABRE layout selection from precomputed circuit and device data.

    Like :func:`sabre_layout`, this performs fused refinement and routing
    search. Every configured refinement iteration completes before the
    resulting layout is routed; complete routes are never created for
    intermediate refinement states, and trials are ranked by predicted route
    quality under the prepared target's routing cost model.
    ``objective`` contributes candidate generation and the diagnostic score in
    the result, but is not the route-selection key.

    Raises:
        CompilerConfigError: If the configuration or prepared inputs are
            incompatible.
        CompilerTransformError: If SABRE cannot find a feasible layout.
    """
    ...

def trivial_layout_prepared(
    analysis: CircuitLayoutAnalysis,
    physical: PhysicalLayoutGraph,
    objective: LayoutObjective | None = None,
) -> LayoutResult:
    """Map prepared logical and physical qubits in their existing order.

    Raises:
        CompilerConfigError: If capacity or scoring inputs are invalid.
        CompilerInternalError: If a valid layout cannot be constructed.
    """
    ...

def greedy_layout_prepared(
    analysis: CircuitLayoutAnalysis,
    physical: PhysicalLayoutGraph,
    objective: LayoutObjective | None = None,
) -> LayoutResult:
    """Build a deterministic greedy layout from prepared inputs.

    Raises:
        CompilerConfigError: If capacity, topology, or scoring inputs are
            invalid.
    """
    ...

def vf2_perfect_layout_prepared(
    analysis: CircuitLayoutAnalysis,
    physical: PhysicalLayoutGraph,
    objective: LayoutObjective | None = None,
    config: Vf2LayoutConfig | None = None,
) -> LayoutResult:
    """Search for a topology-perfect layout from prepared inputs.

    Raises:
        CompilerConfigError: If configuration or capacity is invalid, no
            perfect mapping exists, or scoring fails.
    """
    ...

def trivial_layout(
    circuit: Circuit,
    device: Device,
    objective: LayoutObjective | None = None,
) -> LayoutResult:
    """Map logical and usable physical qubits in their existing order.

    The input objects are not modified. ``None`` selects topology-only scoring.

    Raises:
        CompilerConfigError: If capacity is insufficient or layout scoring
            fails.
    """
    ...

def greedy_layout(
    circuit: Circuit,
    device: Device,
    objective: LayoutObjective | None = None,
) -> LayoutResult:
    """Build a deterministic greedy initial layout.

    The input objects are not modified. ``None`` selects topology-only scoring.

    Raises:
        CompilerConfigError: If capacity, topology, circuit, or scoring is
            invalid.
    """
    ...

def vf2_perfect_layout(
    circuit: Circuit,
    device: Device,
    objective: LayoutObjective | None = None,
    config: Vf2LayoutConfig | None = None,
) -> LayoutResult:
    """Search for a non-induced topology-perfect initial layout.

    ``None`` selects topology-only scoring and the default VF2 configuration.

    Raises:
        CompilerConfigError: If configuration or capacity is invalid, no
            perfect mapping exists, or scoring fails.
    """
    ...

def sabre_layout(
    circuit: Circuit,
    device: Device,
    *,
    objective: LayoutObjective | None = None,
    config: SabreConfig | None = None,
) -> LayoutResult:
    """Select an initial layout with fused SABRE refinement and routing search.

    Every configured refinement iteration completes before the resulting
    layout is routed; complete routes are never created for intermediate
    refinement states, and trials are ranked by predicted native route
    quality. This
    layout-only API returns the winning route's initial layout; it does not
    insert SWAPs or return a routed circuit. ``None`` selects topology-only
    scoring and the default SABRE configuration. ``objective`` contributes
    candidate generation and diagnostics, but is not the route-selection key.

    Raises:
        CompilerConfigError: If configuration, capacity, topology, circuit, or
            scoring is invalid.
    """
    ...

__all__ = [
    "LayoutObjective",
    "LayoutScore",
    "LayoutDiagnostics",
    "LayoutResult",
    "Vf2EdgeRequirement",
    "Vf2LayoutConfig",
    "Interaction",
    "InteractionGraph",
    "CircuitLayoutAnalysis",
    "DistanceTable",
    "PhysicalLayoutGraph",
    "PreparedSabreCircuit",
    "PreparedSabreTarget",
    "analyze_circuit_for_layout",
    "prepare_sabre_circuit",
    "prepare_sabre_topology_target",
    "prepare_sabre_device_target",
    "sabre_layout_prepared",
    "trivial_layout_prepared",
    "greedy_layout_prepared",
    "vf2_perfect_layout_prepared",
    "trivial_layout",
    "greedy_layout",
    "vf2_perfect_layout",
    "sabre_layout",
]
