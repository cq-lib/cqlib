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
from cqlib.compile import CompilerConfigError, CompilerTransformError
from cqlib.compile import sabre
from cqlib.compile.sabre import (
    SabreConfig,
    SabreHeuristicConfig,
    SabreRoutingDiagnostics,
    SabreRoutingResult,
    SabreVf2PrepassConfig,
    sabre_route,
)
from cqlib.device import Device, Layout, Topology


def operation_signature(circuit: Circuit) -> tuple:
    return tuple(
        (
            operation.instruction.instruction.name,
            tuple(qubit.index for qubit in operation.qubits),
        )
        for operation in circuit.operations
    )


def layout_signature(layout: Layout, logical_qubits: range) -> tuple[int, ...]:
    return tuple(layout.get_physical(index).index for index in logical_qubits)


def test_sabre_module_and_public_exports_are_registered():
    assert sabre.sabre_route is sabre_route
    assert "cqlib._native.compile.sabre" in sys.modules
    assert SabreHeuristicConfig.__module__ == "cqlib.compile.sabre"
    assert SabreVf2PrepassConfig.__module__ == "cqlib.compile.sabre"
    assert SabreConfig.__module__ == "cqlib.compile.sabre"
    assert SabreRoutingDiagnostics.__module__ == "cqlib.compile.sabre"
    assert SabreRoutingResult.__module__ == "cqlib.compile.sabre"


def test_sabre_configuration_defaults_and_copy_protocols():
    heuristic = SabreHeuristicConfig()
    assert heuristic.basic_weight == 1.0
    assert heuristic.lookahead_weights == [0.5, 0.25, 0.125, 0.0625, 0.03125]
    assert heuristic.decay_increment == 0.002
    assert heuristic.decay_reset == 10
    assert heuristic.attempt_limit == 1000
    assert heuristic.best_epsilon == 1e-10
    assert copy.copy(heuristic) == heuristic
    assert copy.deepcopy(heuristic) == heuristic

    config = SabreConfig()
    assert config.layout_trials == 10
    assert config.layout_assignment_budget == 1_000_000
    assert config.vf2_prepass == SabreVf2PrepassConfig()
    assert config.refinement_iterations == 1
    assert config.routing_trials == 1
    assert config.seed is None
    assert config.heuristic == heuristic
    assert config.heuristic is not heuristic
    assert copy.copy(config) == config
    assert copy.deepcopy(config) == config

    deterministic = SabreConfig.deterministic_seeded(7)
    assert deterministic.routing_trials == 1
    assert deterministic.seed == 7


def test_vf2_prepass_configuration_is_copyable():
    config = SabreVf2PrepassConfig(candidate_limit=4, call_limit=20_000)
    assert config.candidate_limit == 4
    assert config.call_limit == 20_000
    assert copy.copy(config) == config
    assert copy.deepcopy(config) == config


def test_sabre_route_inserts_swap_and_returns_diagnostics():
    circuit = Circuit(2)
    circuit.cx(0, 1)
    device = Device.line("line3", 3)
    layout = Layout.from_pairs([(0, 0), (1, 2)], physical_count=3)

    result = sabre_route(
        circuit,
        device,
        layout,
        SabreConfig(routing_trials=1, seed=7),
    )

    assert isinstance(result, SabreRoutingResult)
    assert result.swap_count == 1
    assert len(result.circuit.operations) == 2
    assert result.circuit.operations[0].instruction.instruction.name.upper() == "SWAP"
    assert result.initial_layout.get_physical(0).index == 0
    assert result.initial_layout.get_physical(1).index == 2
    assert isinstance(result.diagnostics, SabreRoutingDiagnostics)
    assert result.diagnostics.trials_evaluated == 1
    assert result.diagnostics.selected_trial_index == 0
    assert result.diagnostics.operation_count == 2
    assert copy.copy(result).swap_count == result.swap_count
    assert copy.deepcopy(result.diagnostics) == result.diagnostics


def test_default_route_keeps_adjacent_gate_without_swap():
    circuit = Circuit(2)
    circuit.cx(0, 1)
    device = Device.line("line2", 2)
    layout = Layout(logical=[0, 1], physical=[0, 1])

    result = sabre_route(circuit, device, layout)

    assert result.swap_count == 0
    assert operation_signature(result.circuit) == operation_signature(circuit)


def test_seeded_route_is_reproducible():
    circuit = Circuit(3)
    circuit.cx(0, 1)
    circuit.cx(1, 2)
    circuit.cx(0, 2)
    device = Device.line("line5", 5)
    layout = Layout.from_pairs([(0, 0), (1, 4), (2, 2)], physical_count=5)
    config = SabreConfig(routing_trials=3, seed=23)

    first = sabre_route(circuit, device, layout, config)
    second = sabre_route(circuit, device, layout, config)

    assert operation_signature(first.circuit) == operation_signature(second.circuit)
    assert layout_signature(first.final_layout, range(3)) == layout_signature(
        second.final_layout, range(3)
    )
    assert first.diagnostics == second.diagnostics


def test_route_rejects_invalid_configuration_and_disconnected_interaction():
    circuit = Circuit(2)
    circuit.cx(0, 1)
    line = Device.line("line2", 2)
    layout = Layout(logical=[0, 1], physical=[0, 1])

    with pytest.raises(CompilerConfigError, match="routing_trials"):
        sabre_route(circuit, line, layout, SabreConfig(routing_trials=0))

    with pytest.raises(CompilerConfigError, match="basic_weight"):
        sabre_route(
            circuit,
            line,
            layout,
            SabreConfig(heuristic=SabreHeuristicConfig(basic_weight=-1.0)),
        )

    disconnected = Device(
        "disconnected",
        [0, 1],
        Topology([0, 1], []),
    )
    with pytest.raises(
        CompilerTransformError, match="no executable native terminal pair"
    ):
        sabre_route(circuit, disconnected, layout)
