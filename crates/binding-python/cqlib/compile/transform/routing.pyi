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

"""Device-aware circuit routing transforms."""

from __future__ import annotations

from cqlib.circuit import Circuit
from cqlib.compile.sabre import SabreConfig, SabreRoutingDiagnostics
from cqlib.device import Device, Layout
from .layout import LayoutDiagnostics, LayoutObjective, LayoutScore

class RoutedCircuit:
    """Physical circuit and metadata produced from a supplied layout."""

    @property
    def circuit(self) -> Circuit:
        """Independent copy of the routed physical circuit."""
        ...
    @property
    def initial_layout(self) -> Layout:
        """Independent copy of the initial layout used for routing."""
        ...
    @property
    def final_layout(self) -> Layout:
        """Independent copy of the final layout after routing."""
        ...
    @property
    def swap_count(self) -> int:
        """Number of SWAP operations inserted by routing."""
        ...
    @property
    def diagnostics(self) -> SabreRoutingDiagnostics:
        """Independent copy of the SABRE routing diagnostics."""
        ...
    def changed(self, original: Circuit) -> bool:
        """Return whether routing observably changed ``original``."""
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> RoutedCircuit: ...
    def __deepcopy__(self, memo: dict[int, object]) -> RoutedCircuit: ...

class SabreRouteResult:
    """Result of SABRE layout selection followed by circuit routing."""

    @property
    def routed(self) -> RoutedCircuit:
        """Independent copy of the routed circuit and routing metadata."""
        ...
    @property
    def layout_score(self) -> LayoutScore | None:
        """Observed score of the selected initial layout, when available.

        SABRE selects the winning layout by predicted route quality under the
        prepared routing cost model; this score is diagnostic and is not the
        route-selection key.
        """
        ...
    @property
    def layout_diagnostics(self) -> LayoutDiagnostics:
        """Diagnostics produced while selecting the initial layout."""
        ...
    @property
    def circuit(self) -> Circuit:
        """Independent copy of the routed physical circuit."""
        ...
    @property
    def initial_layout(self) -> Layout:
        """Independent copy of the selected initial layout."""
        ...
    @property
    def final_layout(self) -> Layout:
        """Independent copy of the final layout after routing."""
        ...
    @property
    def swap_count(self) -> int:
        """Number of SWAP operations inserted by routing."""
        ...
    @property
    def diagnostics(self) -> SabreRoutingDiagnostics:
        """Independent copy of the SABRE routing diagnostics."""
        ...
    def changed(self, original: Circuit) -> bool:
        """Return whether routing observably changed ``original``."""
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> SabreRouteResult: ...
    def __deepcopy__(self, memo: dict[int, object]) -> SabreRouteResult: ...

def route_with_layout(
    circuit: Circuit,
    device: Device,
    initial_layout: Layout,
    *,
    config: SabreConfig | None = None,
) -> RoutedCircuit:
    """Route ``circuit`` from a caller-supplied initial layout.

    This function skips automatic layout selection. The input objects are not
    modified, and ``None`` selects the default SABRE configuration.

    Raises:
        CompilerConfigError: If the configuration, circuit, device, or layout
            is invalid for routing.
        CompilerTransformError: If SABRE cannot find a feasible route.
    """
    ...

def route_sabre(
    circuit: Circuit,
    device: Device,
    *,
    objective: LayoutObjective | None = None,
    config: SabreConfig | None = None,
) -> SabreRouteResult:
    """Select a SABRE initial layout and route ``circuit`` for ``device``.

    The input objects are not modified. ``None`` selects topology-only layout
    scoring and the default SABRE configuration. This transform does not lower
    operations to a target basis or legalize directed native gates.

    Raises:
        CompilerConfigError: If the configuration, capacity, topology,
            circuit, or layout scoring input is invalid.
        CompilerTransformError: If SABRE cannot find a feasible layout or
            route.
    """
    ...

__all__ = [
    "RoutedCircuit",
    "SabreRouteResult",
    "route_with_layout",
    "route_sabre",
]
