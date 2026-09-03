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

import copy
import sys

import pytest

from cqlib.circuit import Circuit
from cqlib.compile import CompilerConfigError, transform
from cqlib.compile.sabre import (
    SabreConfig,
    SabreRoutingDiagnostics,
    SabreVf2PrepassConfig,
    normalize_initial_layout,
    validate_reachable_interactions,
)
from cqlib.compile.transform import (
    LayoutObjective,
    RoutedCircuit,
    SabreRouteResult,
    layout,
    route_sabre,
    route_with_layout,
    routing,
)
from cqlib.device import Device, Layout


def test_routing_module_and_parent_exports_are_registered() -> None:
    assert "cqlib._native.compile.transform.routing" in sys.modules
    assert transform.layout is layout
    assert transform.routing is routing
    assert transform.LayoutObjective is layout.LayoutObjective
    assert transform.trivial_layout is layout.trivial_layout
    assert transform.RoutedCircuit is routing.RoutedCircuit
    assert transform.SabreRouteResult is routing.SabreRouteResult
    assert transform.route_with_layout is routing.route_with_layout
    assert transform.route_sabre is routing.route_sabre
    assert RoutedCircuit.__module__ == "cqlib.compile.transform.routing"
    assert SabreRouteResult.__module__ == "cqlib.compile.transform.routing"


def test_route_sabre_selects_layout_and_routes_without_mutating_input() -> None:
    circuit = Circuit(3)
    circuit.cx(0, 1)
    circuit.cx(1, 2)
    circuit.cx(0, 2)
    operation_count = len(circuit.operations)
    config = SabreConfig.deterministic_seeded(7)

    result = route_sabre(
        circuit,
        Device.line("line-3", 3),
        LayoutObjective.topology_only(),
        config,
    )

    assert isinstance(result, SabreRouteResult)
    assert isinstance(result.routed, RoutedCircuit)
    assert isinstance(result.diagnostics, SabreRoutingDiagnostics)
    assert result.layout_score is not None
    assert result.layout_diagnostics.candidates_evaluated >= 1
    assert result.swap_count > 0
    assert result.changed(circuit) is True
    assert len(circuit.operations) == operation_count
    assert copy.copy(result).swap_count == result.swap_count
    assert copy.deepcopy(result.routed).swap_count == result.swap_count


def test_route_with_layout_uses_supplied_layout() -> None:
    circuit = Circuit(2)
    circuit.cx(0, 1)
    initial_layout = Layout.from_pairs([(0, 0), (1, 2)], physical_count=3)

    result = route_with_layout(
        circuit,
        Device.line("line-3", 3),
        initial_layout,
        SabreConfig.deterministic_seeded(11),
    )

    assert isinstance(result, RoutedCircuit)
    assert result.swap_count == 1
    assert result.initial_layout.l2p_map == initial_layout.l2p_map
    assert result.changed(circuit) is True
    assert len(circuit.operations) == 1


def test_route_sabre_is_reproducible_for_same_seed() -> None:
    circuit = Circuit(3)
    circuit.cx(0, 2)
    circuit.cx(1, 2)
    circuit.cx(0, 1)
    device = Device.line("line-4", 4)
    config = SabreConfig.deterministic_seeded(23)

    first = route_sabre(circuit, device, config=config)
    second = route_sabre(circuit, device, config=config)

    assert first.initial_layout.l2p_map == second.initial_layout.l2p_map
    assert first.final_layout.l2p_map == second.final_layout.l2p_map
    assert first.swap_count == second.swap_count
    assert first == second
    assert first.routed == second.routed
    assert first.__eq__(object()) is NotImplemented
    assert first.diagnostics.operation_count == second.diagnostics.operation_count
    assert first.diagnostics.native_operation_count >= 0
    assert first.diagnostics.requirement_signature_count >= 0


def test_sabre_new_configuration_and_layout_helpers_are_public() -> None:
    prepass = SabreVf2PrepassConfig(candidate_limit=3, call_limit=20)
    config = SabreConfig(vf2_prepass=prepass)
    circuit = Circuit(2)
    circuit.cx(0, 1)
    device = Device.line("line-3", 3)
    layout = Layout.from_pairs([(0, 2), (1, 0)], physical_count=3)

    normalized = normalize_initial_layout([0, 1], device, layout)
    validate_reachable_interactions(circuit, device, normalized)

    assert config.vf2_prepass == prepass
    assert config.layout_assignment_budget == 1_000_000
    assert config.routing_trials == 1
    assert normalized.num_vacant_physical == 1


def test_route_sabre_rejects_invalid_configuration() -> None:
    circuit = Circuit(2)
    circuit.cx(0, 1)

    with pytest.raises(CompilerConfigError, match="routing_trials"):
        SabreConfig(routing_trials=0).validate()

    with pytest.raises(CompilerConfigError, match="routing_trials"):
        route_sabre(
            circuit,
            Device.line("line-2", 2),
            config=SabreConfig(routing_trials=0),
        )


def test_route_sabre_rejects_insufficient_device_capacity() -> None:
    circuit = Circuit(2)
    circuit.cx(0, 1)

    with pytest.raises(
        CompilerConfigError, match="2 logical qubits.*1 usable physical qubits"
    ):
        route_sabre(circuit, Device.line("line-1", 1))


def test_route_sabre_rejects_undecomposed_three_qubit_gate() -> None:
    circuit = Circuit(3)
    circuit.ccx(0, 1, 2)

    with pytest.raises(CompilerConfigError, match="more than two qubits"):
        route_sabre(circuit, Device.line("line-3", 3))
