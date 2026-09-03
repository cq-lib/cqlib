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

from ..._native import compile as _compile_module

_layout_module = _compile_module.transform.layout

LayoutObjective = _layout_module.LayoutObjective
LayoutScore = _layout_module.LayoutScore
LayoutDiagnostics = _layout_module.LayoutDiagnostics
LayoutResult = _layout_module.LayoutResult
Vf2EdgeRequirement = _layout_module.Vf2EdgeRequirement
Vf2LayoutConfig = _layout_module.Vf2LayoutConfig
Interaction = _layout_module.Interaction
InteractionGraph = _layout_module.InteractionGraph
CircuitLayoutAnalysis = _layout_module.CircuitLayoutAnalysis
DistanceTable = _layout_module.DistanceTable
PhysicalLayoutGraph = _layout_module.PhysicalLayoutGraph
PreparedSabreCircuit = _layout_module.PreparedSabreCircuit
PreparedSabreTarget = _layout_module.PreparedSabreTarget
analyze_circuit_for_layout = _layout_module.analyze_circuit_for_layout
prepare_sabre_circuit = _layout_module.prepare_sabre_circuit
prepare_sabre_topology_target = _layout_module.prepare_sabre_topology_target
prepare_sabre_device_target = _layout_module.prepare_sabre_device_target
sabre_layout_prepared = _layout_module.sabre_layout_prepared
trivial_layout_prepared = _layout_module.trivial_layout_prepared
greedy_layout_prepared = _layout_module.greedy_layout_prepared
vf2_perfect_layout_prepared = _layout_module.vf2_perfect_layout_prepared
trivial_layout = _layout_module.trivial_layout
greedy_layout = _layout_module.greedy_layout
vf2_perfect_layout = _layout_module.vf2_perfect_layout
sabre_layout = _layout_module.sabre_layout

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
