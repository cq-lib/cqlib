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

"""Public bridge to the native SABRE routing implementation.

SABRE inserts SWAP operations so that every two-qubit interaction is executed
on connected physical qubits. The caller supplies the initial logical-to-
physical mapping; the result reports both the normalized initial mapping and
the final mapping after all inserted SWAPs.

Example::

    from cqlib.circuit import Circuit
    from cqlib.compile.sabre import SabreConfig, sabre_route
    from cqlib.device import Device, Layout

    circuit = Circuit(2)
    circuit.cx(0, 1)
    device = Device.line("line3", 3)
    layout = Layout.from_pairs([(0, 0), (1, 2)], physical_count=3)

    result = sabre_route(
        circuit,
        device,
        layout,
        config=SabreConfig(routing_trials=1, seed=7),
    )
    assert result.swap_count == 1
    print(result.final_layout)
    print(result.diagnostics.two_qubit_depth)
"""

from ..._native import compile as _compile_module

_sabre_module = _compile_module.sabre

SabreHeuristicConfig = _sabre_module.SabreHeuristicConfig
SabreVf2PrepassConfig = _sabre_module.SabreVf2PrepassConfig
SabreConfig = _sabre_module.SabreConfig
SabreRoutingDiagnostics = _sabre_module.SabreRoutingDiagnostics
SabreRoutingResult = _sabre_module.SabreRoutingResult
sabre_route = _sabre_module.sabre_route
normalize_initial_layout = _sabre_module.normalize_initial_layout
validate_reachable_interactions = _sabre_module.validate_reachable_interactions

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
