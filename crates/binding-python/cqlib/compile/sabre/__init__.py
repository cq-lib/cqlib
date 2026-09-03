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

"""SWAP-based SABRE routing for connectivity-constrained quantum devices.

Use :func:`sabre_route` when an initial logical-to-physical layout is already
known and the circuit must be made executable on a device topology. A fixed
seed makes cqlib's tie-breaking and multi-trial selection reproducible.

Example::

    from cqlib.circuit import Circuit
    from cqlib.compile.sabre import SabreConfig, sabre_route
    from cqlib.device import Device, Layout

    circuit = Circuit(2)
    circuit.cx(0, 1)
    result = sabre_route(
        circuit,
        Device.line("line3", 3),
        Layout.from_pairs([(0, 0), (1, 2)], physical_count=3),
        config=SabreConfig(routing_trials=1, seed=7),
    )
    assert result.swap_count == 1
"""

from .routing import SabreConfig as SabreConfig
from .routing import SabreHeuristicConfig as SabreHeuristicConfig
from .routing import SabreVf2PrepassConfig as SabreVf2PrepassConfig
from .routing import SabreRoutingDiagnostics as SabreRoutingDiagnostics
from .routing import SabreRoutingResult as SabreRoutingResult
from .routing import sabre_route as sabre_route
from .routing import normalize_initial_layout as normalize_initial_layout
from .routing import validate_reachable_interactions as validate_reachable_interactions

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
