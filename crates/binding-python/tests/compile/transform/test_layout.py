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

from cqlib.circuit import Circuit, StandardGate
from cqlib.compile import CompilerConfigError
from cqlib.compile.sabre import SabreConfig
from cqlib.compile.transform.layout import (
    LayoutDiagnostics,
    CircuitLayoutAnalysis,
    DistanceTable,
    Interaction,
    InteractionGraph,
    LayoutObjective,
    LayoutResult,
    LayoutScore,
    PhysicalLayoutGraph,
    PreparedSabreCircuit,
    PreparedSabreDeviceTarget,
    Vf2EdgeRequirement,
    Vf2LayoutConfig,
    analyze_circuit_for_layout,
    greedy_layout,
    greedy_layout_prepared,
    prepare_sabre_circuit,
    prepare_sabre_device_target,
    sabre_layout,
    sabre_layout_prepared,
    trivial_layout,
    trivial_layout_prepared,
    vf2_perfect_layout,
    vf2_perfect_layout_prepared,
)
from cqlib.device import Device, Layout


def test_layout_module_and_public_types_are_registered() -> None:
    assert "cqlib._native.compile.transform.layout" in sys.modules
    for public_type in (
        LayoutObjective,
        LayoutScore,
        LayoutDiagnostics,
        LayoutResult,
        Vf2EdgeRequirement,
        Vf2LayoutConfig,
        Interaction,
        InteractionGraph,
        CircuitLayoutAnalysis,
        DistanceTable,
        PhysicalLayoutGraph,
        PreparedSabreCircuit,
        PreparedSabreDeviceTarget,
    ):
        assert public_type.__module__ == "cqlib.compile.transform.layout"


def test_layout_configuration_is_immutable_and_copyable() -> None:
    objective = LayoutObjective(
        distance_weight=2.0,
        direction_weight=3.0,
        two_qubit_error_weight=4.0,
        readout_error_weight=5.0,
    )
    requirement = Vf2EdgeRequirement.all_interactions()
    config = Vf2LayoutConfig(
        candidate_limit=4,
        call_limit=20,
        edge_requirement=requirement,
    )

    assert objective.distance_weight == 2.0
    assert objective.uses_fidelity is True
    assert copy.copy(objective) == objective
    assert copy.deepcopy(config) == config
    assert config.edge_requirement == requirement
    assert hash(copy.copy(requirement)) == hash(requirement)

    with pytest.raises(AttributeError):
        config.candidate_limit = 5


def test_device_aware_objective_factories() -> None:
    device = Device.line("line-2", 2)

    automatic = LayoutObjective.auto_from_device(device)
    assert automatic == LayoutObjective.topology_only()
    physical = PhysicalLayoutGraph.from_device(device)
    assert LayoutObjective.auto_from_physical(physical) == automatic

    with pytest.raises(CompilerConfigError, match="no usable fidelity data"):
        LayoutObjective.fidelity_required(device)
    with pytest.raises(CompilerConfigError, match="no usable fidelity data"):
        LayoutObjective.fidelity_required_from_physical(physical)

    device.default_readout_error = 0.01
    required = LayoutObjective.fidelity_required(device)
    assert required.readout_error_weight == 1.0
    assert required.two_qubit_error_weight == 0.0
    physical = PhysicalLayoutGraph.from_device(device)
    assert LayoutObjective.fidelity_required_from_physical(physical) == required


@pytest.mark.parametrize(
    "algorithm",
    [trivial_layout, greedy_layout, vf2_perfect_layout],
)
def test_deterministic_layout_algorithms_return_complete_results(algorithm) -> None:
    circuit = Circuit(3)
    circuit.cx(0, 1)
    circuit.cx(1, 2)
    device = Device.line("line-3", 3)

    result = algorithm(circuit, device)

    assert isinstance(result, LayoutResult)
    assert isinstance(result.layout, Layout)
    assert result.layout.num_logical == 3
    assert result.score is not None
    assert result.diagnostics.is_perfect is True
    assert result.diagnostics.candidates_evaluated >= 1


def test_sabre_layout_is_reproducible_with_a_fixed_seed() -> None:
    circuit = Circuit(3)
    circuit.cx(0, 2)
    device = Device.line("line-3", 3)
    config = SabreConfig.deterministic_seeded(7)

    first = sabre_layout(circuit, device, config=config)
    second = sabre_layout(circuit, device, config=config)

    assert first.layout.l2p_map == second.layout.l2p_map
    assert first.score == second.score
    assert first.diagnostics == second.diagnostics
    assert first == second
    assert first.__eq__(object()) is NotImplemented
    notes = first.diagnostics.notes
    notes.append("caller mutation")
    assert "caller mutation" not in first.diagnostics.notes


def test_layout_analysis_and_physical_graph_expose_reusable_inputs() -> None:
    circuit = Circuit(3)
    circuit.cx(0, 2)
    circuit.cx(0, 2)
    device = Device.line("line-3", 3)

    analysis = analyze_circuit_for_layout(circuit)
    physical = PhysicalLayoutGraph.from_device(device)

    assert isinstance(analysis, CircuitLayoutAnalysis)
    assert len(analysis.logical_qubits) == 3
    assert isinstance(analysis.interactions, InteractionGraph)
    assert len(analysis.interactions) == 1
    assert analysis.interactions.interactions[0].weight == 2.0
    assert physical.distance(0, 2) == 2
    assert physical.distances.distance(0, 1) == 1
    assert physical.is_adjacent_undirected(0, 1) is True
    assert physical.supports_directed_coupling(0, 1)
    assert not physical.supports_two_qubit_gate_directed(0, 1, StandardGate.CX)
    assert physical.has_fidelity_data is False
    assert InteractionGraph().is_empty() is True


def test_prepared_sabre_layout_matches_direct_entry_point() -> None:
    circuit = Circuit(3)
    circuit.cx(0, 2)
    device = Device.line("line-3", 3)
    config = SabreConfig.deterministic_seeded(17)

    prepared = prepare_sabre_circuit(circuit)
    prepared_target = prepare_sabre_device_target(prepared, device)
    direct = sabre_layout(circuit, device, config=config)
    reused = sabre_layout_prepared(prepared, prepared_target, config=config)

    assert prepared.logical_qubits == prepared.analysis.logical_qubits
    assert [qubit.id for qubit in prepared_target.physical.physical_qubits] == [
        0,
        1,
        2,
    ]
    assert reused == direct


@pytest.mark.parametrize(
    ("direct", "prepared_entry"),
    [
        (trivial_layout, trivial_layout_prepared),
        (greedy_layout, greedy_layout_prepared),
        (vf2_perfect_layout, vf2_perfect_layout_prepared),
    ],
)
def test_prepared_layout_entry_points_match_direct_algorithms(
    direct, prepared_entry
) -> None:
    circuit = Circuit(3)
    circuit.cx(0, 1)
    circuit.cx(1, 2)
    device = Device.line("line-3", 3)
    analysis = analyze_circuit_for_layout(circuit)
    physical = PhysicalLayoutGraph.from_device(device)

    expected = direct(circuit, device)
    actual = prepared_entry(analysis, physical)

    assert actual == expected
    assert expected.score == LayoutObjective.topology_only().score_layout(
        analysis, physical, expected.layout
    )


def test_invalid_objective_and_vf2_config_are_rejected_when_run() -> None:
    circuit = Circuit(2)
    circuit.cx(0, 1)
    device = Device.line("line-2", 2)

    with pytest.raises(
        CompilerConfigError, match="distance_weight must be finite and non-negative"
    ):
        trivial_layout(circuit, device, LayoutObjective(distance_weight=-1.0))

    with pytest.raises(
        CompilerConfigError, match="candidate_limit must be greater than zero"
    ):
        vf2_perfect_layout(circuit, device, config=Vf2LayoutConfig(candidate_limit=0))


def test_layout_rejects_insufficient_physical_capacity() -> None:
    with pytest.raises(
        CompilerConfigError, match="at least as many usable physical qubits"
    ):
        greedy_layout(Circuit(3), Device.line("line-2", 2))


def test_layout_rejects_undecomposed_three_qubit_operation() -> None:
    circuit = Circuit(3)
    circuit.ccx(0, 1, 2)

    with pytest.raises(
        CompilerConfigError, match="more than two qubits to be decomposed"
    ):
        trivial_layout(circuit, Device.line("line-3", 3))


def test_vf2_reports_when_no_perfect_embedding_exists() -> None:
    circuit = Circuit(3)
    circuit.cx(0, 1)
    circuit.cx(1, 2)
    circuit.cx(0, 2)

    with pytest.raises(CompilerConfigError, match="could not find a perfect mapping"):
        vf2_perfect_layout(circuit, Device.line("line-3", 3))
