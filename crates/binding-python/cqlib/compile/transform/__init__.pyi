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

"""Reusable compiler transforms."""

from . import analysis as analysis
from . import commutative_cancellation as commutative_cancellation
from . import decompose as decompose
from . import device_lowering as device_lowering
from . import layout as layout
from . import native_optimization as native_optimization
from . import one_qubit_optimization as one_qubit_optimization
from . import routing as routing
from . import resynthesis as resynthesis
from . import target_basis as target_basis
from . import virtual_permutation as virtual_permutation
from .analysis import CircuitAnalysis as CircuitAnalysis
from .canonicalize import CanonicalizeConfig as CanonicalizeConfig
from .canonicalize import CanonicalizeResult as CanonicalizeResult
from .canonicalize import Canonicalizer as Canonicalizer
from .canonicalize import canonicalize_circuit as canonicalize_circuit
from .commutative_cancellation import (
    CommutativeCancellation as CommutativeCancellation,
)
from .device_lowering import DeviceLowerer as DeviceLowerer
from .layout import LayoutDiagnostics as LayoutDiagnostics
from .layout import CircuitLayoutAnalysis as CircuitLayoutAnalysis
from .layout import DistanceTable as DistanceTable
from .layout import Interaction as Interaction
from .layout import InteractionGraph as InteractionGraph
from .layout import LayoutObjective as LayoutObjective
from .layout import LayoutResult as LayoutResult
from .layout import LayoutScore as LayoutScore
from .layout import PhysicalLayoutGraph as PhysicalLayoutGraph
from .layout import PreparedSabreCircuit as PreparedSabreCircuit
from .layout import PreparedSabreTarget as PreparedSabreTarget
from .layout import Vf2EdgeRequirement as Vf2EdgeRequirement
from .layout import Vf2LayoutConfig as Vf2LayoutConfig
from .layout import analyze_circuit_for_layout as analyze_circuit_for_layout
from .layout import greedy_layout as greedy_layout
from .layout import greedy_layout_prepared as greedy_layout_prepared
from .layout import prepare_sabre_circuit as prepare_sabre_circuit
from .layout import prepare_sabre_device_target as prepare_sabre_device_target
from .layout import prepare_sabre_topology_target as prepare_sabre_topology_target
from .layout import sabre_layout as sabre_layout
from .layout import sabre_layout_prepared as sabre_layout_prepared
from .layout import trivial_layout as trivial_layout
from .layout import trivial_layout_prepared as trivial_layout_prepared
from .layout import vf2_perfect_layout as vf2_perfect_layout
from .layout import vf2_perfect_layout_prepared as vf2_perfect_layout_prepared
from .native_optimization import NativeOptimizationResult as NativeOptimizationResult
from .native_optimization import NativeOptimizationSummary as NativeOptimizationSummary
from .native_optimization import NativeQualityPolicy as NativeQualityPolicy
from .native_optimization import NativeOptimizer as NativeOptimizer
from .one_qubit_optimization import OptimizeOneQubitRuns as OptimizeOneQubitRuns
from .routing import RoutedCircuit as RoutedCircuit
from .routing import SabreRouteResult as SabreRouteResult
from .routing import route_sabre as route_sabre
from .routing import route_with_layout as route_with_layout
from .routing_basis import LowerToRoutingBasis as LowerToRoutingBasis
from .routing_basis import lower_to_routing_basis as lower_to_routing_basis
from .target_basis import TargetBasisCost as TargetBasisCost
from .target_basis import TargetBasisCostModel as TargetBasisCostModel
from .target_basis import TargetBasisLowerer as TargetBasisLowerer
from .target_basis import TargetBasisSignature as TargetBasisSignature
from .virtual_permutation import (
    VirtualPermutationElisionResult as VirtualPermutationElisionResult,
)
from .virtual_permutation import (
    elide_virtual_permutations as elide_virtual_permutations,
)
from .rewrite import KnowledgeRewriteResult as KnowledgeRewriteResult
from .rewrite import KnowledgeRewriteStats as KnowledgeRewriteStats
from .rewrite import KnowledgeRewriter as KnowledgeRewriter
from .rewrite import RewriteConfig as RewriteConfig
from .rewrite import RewriteMode as RewriteMode
from .rewrite import rewrite_circuit as rewrite_circuit
from .result import TransformResult as TransformResult

__all__ = [
    "analysis",
    "commutative_cancellation",
    "decompose",
    "device_lowering",
    "layout",
    "native_optimization",
    "one_qubit_optimization",
    "routing",
    "resynthesis",
    "target_basis",
    "virtual_permutation",
    "TransformResult",
    "CircuitAnalysis",
    "DeviceLowerer",
    "CanonicalizeConfig",
    "Canonicalizer",
    "CanonicalizeResult",
    "canonicalize_circuit",
    "CommutativeCancellation",
    "OptimizeOneQubitRuns",
    "NativeOptimizationSummary",
    "NativeOptimizationResult",
    "NativeQualityPolicy",
    "NativeOptimizer",
    "LayoutObjective",
    "Interaction",
    "InteractionGraph",
    "CircuitLayoutAnalysis",
    "DistanceTable",
    "PhysicalLayoutGraph",
    "PreparedSabreCircuit",
    "PreparedSabreTarget",
    "LayoutScore",
    "LayoutDiagnostics",
    "LayoutResult",
    "Vf2EdgeRequirement",
    "Vf2LayoutConfig",
    "trivial_layout",
    "trivial_layout_prepared",
    "analyze_circuit_for_layout",
    "prepare_sabre_circuit",
    "prepare_sabre_topology_target",
    "prepare_sabre_device_target",
    "greedy_layout",
    "greedy_layout_prepared",
    "vf2_perfect_layout",
    "vf2_perfect_layout_prepared",
    "sabre_layout",
    "sabre_layout_prepared",
    "RoutedCircuit",
    "SabreRouteResult",
    "route_with_layout",
    "route_sabre",
    "LowerToRoutingBasis",
    "lower_to_routing_basis",
    "TargetBasisLowerer",
    "TargetBasisSignature",
    "TargetBasisCost",
    "TargetBasisCostModel",
    "VirtualPermutationElisionResult",
    "elide_virtual_permutations",
    "RewriteMode",
    "RewriteConfig",
    "KnowledgeRewriter",
    "KnowledgeRewriteStats",
    "KnowledgeRewriteResult",
    "rewrite_circuit",
]
