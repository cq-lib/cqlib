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

import numpy as np
import pytest

from cqlib.circuit import Circuit, Instruction, StandardGate, UnitaryGate
from cqlib.circuit.gates import MCGate
from cqlib.compile import CompilerConfigError
from cqlib.compile.commutation import CommutationConfig
from cqlib.compile.knowledge import RuleKind
from cqlib.compile.transform import (
    CanonicalizeConfig,
    CanonicalizeResult,
    Canonicalizer,
    CommutativeCancellation,
    KnowledgeRewriteResult,
    KnowledgeRewriteStats,
    KnowledgeRewriter,
    LowerToRoutingBasis,
    NativeOptimizationResult,
    NativeOptimizationSummary,
    NativeQualityPolicy,
    NativeOptimizer,
    OptimizeOneQubitRuns,
    RewriteConfig,
    RewriteMode,
    TargetBasisCostModel,
    TargetBasisLowerer,
    TransformResult,
    canonicalize_circuit,
    elide_virtual_permutations,
    lower_to_routing_basis,
    rewrite_circuit,
)
from cqlib.device import Device
from cqlib.compile.transform.decompose import (
    TwoQubitUnitaryDecomposeBasis,
    UnitaryDecomposeConfig,
    decompose_unitaries,
    expand_definitions,
)
from cqlib.compile.transform.result import (
    TransformResult as ResultModuleTransformResult,
)
from cqlib.compile.transform.resynthesis import (
    ResynthesizeTwoQubitBlocks,
    TwoQubitBlockResynthesisConfig,
    resynthesize_two_qubit_blocks,
    resynthesize_two_qubit_blocks_for_device,
)


def test_transform_module_and_public_types_are_registered() -> None:
    assert "cqlib._native.compile.transform" in sys.modules
    assert CanonicalizeConfig.__module__ == "cqlib.compile.transform"
    assert Canonicalizer.__module__ == "cqlib.compile.transform"
    assert CanonicalizeResult.__module__ == "cqlib.compile.transform"
    assert (
        CommutativeCancellation.__module__
        == "cqlib.compile.transform.commutative_cancellation"
    )
    assert RewriteMode.__module__ == "cqlib.compile.transform"
    assert RewriteConfig.__module__ == "cqlib.compile.transform"
    assert KnowledgeRewriter.__module__ == "cqlib.compile.transform"
    assert KnowledgeRewriteStats.__module__ == "cqlib.compile.transform"
    assert KnowledgeRewriteResult.__module__ == "cqlib.compile.transform"
    assert LowerToRoutingBasis.__module__ == "cqlib.compile.transform"
    assert (
        resynthesize_two_qubit_blocks_for_device.__module__
        == "cqlib.compile.transform.resynthesis"
    )
    assert (
        elide_virtual_permutations.__module__
        == "cqlib.compile.transform.virtual_permutation"
    )
    assert TransformResult is ResultModuleTransformResult


def test_canonicalize_config_exposes_immutable_options() -> None:
    config = CanonicalizeConfig(
        round_limit=3,
        recurse_control_flow=False,
        fold_gphase=False,
        canonicalize_instruction_form=False,
        drop_noops=False,
        canonicalize_barriers=False,
    )

    assert config.round_limit == 3
    assert config.recurse_control_flow is False
    assert config.fold_gphase is False
    assert config.canonicalize_instruction_form is False
    assert config.drop_noops is False
    assert config.canonicalize_barriers is False
    assert copy.copy(config) == config
    assert copy.deepcopy(config) == config
    assert repr(config).startswith("CanonicalizeConfig(round_limit=3,")

    with pytest.raises(AttributeError):
        config.round_limit = 4


def test_production_canonicalization_does_not_mutate_input() -> None:
    circuit = Circuit(1)
    circuit.i(0)

    result = canonicalize_circuit(circuit)

    assert len(circuit.operations) == 1
    assert len(result.circuit.operations) == 0
    assert result.changed is True
    assert result.rounds >= 1


def test_configured_canonicalizer_can_preserve_noops() -> None:
    circuit = Circuit(1)
    circuit.i(0)
    config = CanonicalizeConfig(drop_noops=False)
    canonicalizer = Canonicalizer(config)

    result = canonicalizer.run(circuit)

    assert canonicalizer.config == config
    assert len(result.circuit.operations) == 1
    assert result.changed is False


def test_canonicalization_is_idempotent() -> None:
    circuit = Circuit(1)
    circuit.i(0)
    circuit.h(0)

    first = canonicalize_circuit(circuit)
    second = canonicalize_circuit(first.circuit)

    assert first.changed is True
    assert second.changed is False
    assert len(second.circuit.operations) == 1


def test_transform_results_have_value_equality() -> None:
    circuit = Circuit(1)
    circuit.i(0)

    first = canonicalize_circuit(circuit)
    second = canonicalize_circuit(circuit)

    assert first == second
    assert first.__eq__(object()) is NotImplemented
    assert first in [second]

    transform_first = expand_definitions(circuit)
    transform_second = expand_definitions(circuit)
    assert transform_first == transform_second
    assert transform_first.__eq__(object()) is NotImplemented


@pytest.mark.parametrize(
    "run",
    [canonicalize_circuit, lambda circuit: Canonicalizer().run(circuit)],
)
def test_canonicalization_releases_gil(run, assert_releases_gil) -> None:
    circuit = Circuit(1)
    for _ in range(20_000):
        circuit.h(0)

    # Fast implementations may finish a call before the worker is scheduled.
    # Retry within a bounded window instead of requiring any minimum call time.
    assert_releases_gil(lambda: run(circuit))


def test_zero_round_limit_is_rejected_when_run() -> None:
    canonicalizer = Canonicalizer(CanonicalizeConfig(round_limit=0))

    with pytest.raises(
        CompilerConfigError, match="round_limit must be greater than zero"
    ):
        canonicalizer.run(Circuit(1))


def test_rewrite_modes_and_config_expose_immutable_options() -> None:
    optimize = RewriteMode.optimize()
    lowering = RewriteMode.lowering()
    kinds = [RuleKind.cancel(), RuleKind.merge()]
    h = Instruction.from_standard_gate(StandardGate.H)
    config = RewriteConfig(
        max_rounds=3,
        max_window_ops=7,
        max_pattern_len=4,
        recurse_control_flow=False,
        skip_labeled_ops=False,
        preserve_two_qubit_connectivity=True,
        enabled_kinds=kinds,
        mode=lowering,
        target_instructions=[h, h],
    )

    assert optimize.name == "optimize"
    assert lowering.name == "lowering"
    assert lowering == RewriteMode.lowering()
    assert hash(lowering) == hash(RewriteMode.lowering())
    assert config.max_rounds == 3
    assert config.max_window_ops == 7
    assert config.max_pattern_len == 4
    assert config.recurse_control_flow is False
    assert config.skip_labeled_ops is False
    assert config.preserve_two_qubit_connectivity is True
    assert config.enabled_kinds == kinds
    assert config.mode == lowering
    assert [instruction.name for instruction in config.target_instructions] == ["H"]
    assert copy.copy(config) == config
    assert copy.deepcopy(config) == config
    assert repr(config).startswith("RewriteConfig(max_rounds=3,")
    assert "preserve_two_qubit_connectivity=True" in repr(config)

    with pytest.raises(AttributeError):
        config.max_rounds = 4


def test_rewrite_connectivity_policy_blocks_bridge_collapse() -> None:
    circuit = Circuit(3)
    circuit.cx(0, 1)
    circuit.cx(1, 2)
    circuit.cx(0, 1)
    circuit.cx(1, 2)

    unrestricted = rewrite_circuit(circuit)
    preserved = rewrite_circuit(
        circuit,
        RewriteConfig(preserve_two_qubit_connectivity=True),
    )

    assert len(unrestricted.circuit.operations) == 1
    assert [qubit.index for qubit in unrestricted.circuit.operations[0].qubits] == [
        0,
        2,
    ]
    assert len(preserved.circuit.operations) == 4
    assert all(
        operation.instruction.instruction.name == "CX"
        for operation in preserved.circuit.operations
    )
    assert [
        [qubit.index for qubit in operation.qubits]
        for operation in preserved.circuit.operations
    ] == [[0, 1], [1, 2], [0, 1], [1, 2]]


def test_lowering_mode_selects_lowering_rule_defaults() -> None:
    config = RewriteConfig(mode=RewriteMode.lowering())

    assert config == RewriteConfig.lowering()
    assert RuleKind.decompose() in config.enabled_kinds
    assert RuleKind.hardware_native() in config.enabled_kinds


def test_production_rewrite_does_not_mutate_input_and_reports_stats() -> None:
    circuit = Circuit(1)
    circuit.h(0)
    circuit.h(0)

    result = rewrite_circuit(circuit)

    assert len(circuit.operations) == 2
    assert len(result.circuit.operations) == 0
    assert result.changed is True
    assert result.stats.rules_applied == 1
    assert result.stats.changed_sequences == 1
    assert result.stats.rounds_executed >= 1
    assert result.stats.reached_fixpoint is True
    assert copy.copy(result.stats) == result.stats


def test_rewriter_lowers_to_explicit_target_basis() -> None:
    circuit = Circuit(2)
    circuit.cx(0, 1)
    config = RewriteConfig(
        mode=RewriteMode.lowering(),
        target_instructions=[
            Instruction.from_standard_gate(StandardGate.H),
            Instruction.from_standard_gate(StandardGate.CZ),
        ],
    )
    rewriter = KnowledgeRewriter(config)

    result = rewriter.run(circuit)

    assert rewriter.config == config
    assert [
        operation.instruction.instruction.name
        for operation in result.circuit.operations
    ] == ["H", "CZ", "H"]
    assert result.stats.rules_applied >= 1


def test_rewrite_rejects_invalid_configuration_and_unsatisfied_basis() -> None:
    with pytest.raises(CompilerConfigError, match="must not be empty"):
        RewriteConfig(target_instructions=[])
    with pytest.raises(
        CompilerConfigError, match="unsupported rewrite target instruction"
    ):
        RewriteConfig(target_instructions=[Instruction.delay()])

    zero_round_rewriter = KnowledgeRewriter(RewriteConfig(max_rounds=0))
    with pytest.raises(
        CompilerConfigError, match="max_rounds must be greater than zero"
    ):
        zero_round_rewriter.run(Circuit(1))

    circuit = Circuit(1)
    circuit.h(0)
    config = RewriteConfig(
        mode=RewriteMode.lowering(),
        target_instructions=[Instruction.from_standard_gate(StandardGate.CZ)],
    )
    with pytest.raises(CompilerConfigError, match="lowering incomplete"):
        rewrite_circuit(circuit, config)


def test_commutative_cancellation_cancels_distant_self_inverse_pairs() -> None:
    circuit = Circuit(3)
    circuit.h(0)
    circuit.cz(1, 2)
    circuit.cz(1, 2)
    circuit.h(0)
    cancellation = CommutativeCancellation()

    result = cancellation.run(circuit)

    assert repr(cancellation) == "CommutativeCancellation()"
    assert result.changed is True
    assert len(result.circuit.operations) == 0
    assert len(circuit.operations) == 4
    assert copy.copy(cancellation).__class__ is CommutativeCancellation
    assert copy.deepcopy(cancellation).__class__ is CommutativeCancellation


def test_commutative_cancellation_respects_non_commuting_barriers() -> None:
    circuit = Circuit(1)
    circuit.h(0)
    circuit.x(0)
    circuit.h(0)

    result = CommutativeCancellation().run(circuit)

    assert result.changed is False
    assert len(result.circuit.operations) == 3


def test_lower_to_routing_basis_lowers_toffoli_to_two_qubit_ops() -> None:
    circuit = Circuit(3)
    circuit.ccx(0, 1, 2)

    result = lower_to_routing_basis(circuit)

    assert result.changed is True
    names = [
        operation.instruction.instruction.name
        for operation in result.circuit.operations
    ]
    assert "CCX" not in names
    assert "CX" in names
    assert len(circuit.operations) == 1


def test_lower_to_routing_basis_prefers_cz_only_basis() -> None:
    circuit = Circuit(3)
    circuit.ccx(0, 1, 2)
    basis = [
        Instruction.from_standard_gate(StandardGate.H),
        Instruction.from_standard_gate(StandardGate.T),
        Instruction.from_standard_gate(StandardGate.TDG),
        Instruction.from_standard_gate(StandardGate.CZ),
        Instruction.from_standard_gate(StandardGate.GPhase),
    ]
    transform = LowerToRoutingBasis(preferred_basis=basis)

    result = transform.run(circuit)
    names = [
        operation.instruction.instruction.name
        for operation in result.circuit.operations
    ]

    assert [instruction.name for instruction in transform.preferred_basis] == [
        instruction.name for instruction in basis
    ]
    assert "CCX" not in names
    assert "CX" not in names
    assert "CZ" in names


def test_lower_to_routing_basis_reports_route_sabre_contract() -> None:
    circuit = Circuit(3)
    circuit.append_unitary_gate(UnitaryGate("three_q", 3), [0, 1, 2])

    with pytest.raises(
        CompilerConfigError,
        match="routing-basis lowering did not satisfy route.sabre input contract",
    ):
        lower_to_routing_basis(circuit)


def test_two_qubit_block_resynthesis_config_exposes_all_options() -> None:
    commutation = CommutationConfig(
        enable_rule_oracle=False,
        enable_matrix_fallback=True,
        max_matrix_qubits=2,
    )
    config = TwoQubitBlockResynthesisConfig(
        two_qubit_basis=TwoQubitUnitaryDecomposeBasis.cx(),
        enhanced=True,
        max_block_ops=20,
        max_crossed_ops=5,
        max_scan_span=40,
        skip_labeled_ops=False,
        recurse_control_flow=False,
        commutation=commutation,
    )

    assert config.two_qubit_basis == TwoQubitUnitaryDecomposeBasis.cx()
    assert config.max_block_ops == 20
    assert config.max_crossed_ops == 5
    assert config.max_scan_span == 40
    assert config.skip_labeled_ops is False
    assert config.recurse_control_flow is False
    assert config.commutation == commutation
    assert copy.copy(config) == config
    assert copy.deepcopy(config) == config
    assert repr(config).startswith("TwoQubitBlockResynthesisConfig(")


def test_unitary_decompose_config_preserves_legacy_python_basis() -> None:
    config = UnitaryDecomposeConfig(
        two_qubit_basis=TwoQubitUnitaryDecomposeBasis.rzz(),
        recurse_control_flow=False,
    )

    assert config.two_qubit_basis == TwoQubitUnitaryDecomposeBasis.rzz()
    assert config.recurse_control_flow is False
    assert copy.copy(config) == config
    assert copy.deepcopy(config) == config
    assert repr(config).startswith("UnitaryDecomposeConfig(")


def test_unitary_decompose_config_default_matches_omitted_config() -> None:
    config = UnitaryDecomposeConfig()

    assert config.two_qubit_basis is None
    assert config.recurse_control_flow is True
    assert "two_qubit_basis=None" in repr(config)

    matrix = np.array(
        [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 0, 1], [0, 0, 1, 0]],
        dtype=np.complex128,
    )
    gate = UnitaryGate("cx_matrix", 2).with_matrix(matrix)
    circuit = Circuit(2)
    circuit.append_unitary_gate(gate, [0, 1])

    def operation_names(result: TransformResult) -> list[str]:
        return [
            operation.instruction.instruction.name
            for operation in result.circuit.operations
        ]

    omitted = decompose_unitaries(circuit)
    explicit = decompose_unitaries(circuit, UnitaryDecomposeConfig())

    assert operation_names(explicit) == operation_names(omitted)

    legacy = decompose_unitaries(
        circuit,
        UnitaryDecomposeConfig(
            two_qubit_basis=TwoQubitUnitaryDecomposeBasis.pauli_rotations()
        ),
    )
    assert legacy.changed is True


def test_decompose_configs_reject_conflicting_basis_selection() -> None:
    target_basis = [Instruction.from_standard_gate(StandardGate.CZ)]
    with pytest.raises(
        CompilerConfigError,
        match="two_qubit_basis and target_basis are mutually exclusive",
    ):
        UnitaryDecomposeConfig(
            two_qubit_basis=TwoQubitUnitaryDecomposeBasis.cx(),
            target_basis=target_basis,
        )
    with pytest.raises(
        CompilerConfigError,
        match="two_qubit_basis and target_basis are mutually exclusive",
    ):
        TwoQubitBlockResynthesisConfig(
            two_qubit_basis=TwoQubitUnitaryDecomposeBasis.cx(),
            target_basis=target_basis,
        )


def test_unitary_decompose_config_accepts_explicit_target_basis() -> None:
    target_basis = [
        Instruction.from_standard_gate(StandardGate.RZ),
        Instruction.from_standard_gate(StandardGate.X2P),
        Instruction.from_standard_gate(StandardGate.CZ),
    ]
    config = UnitaryDecomposeConfig(
        target_basis=target_basis,
        recurse_control_flow=False,
    )

    assert config.two_qubit_basis is None
    assert [instruction.name for instruction in config.target_basis] == [
        "RZ",
        "X2P",
        "CZ",
    ]
    assert config.recurse_control_flow is False
    assert copy.copy(config) == config
    assert copy.deepcopy(config) == config
    assert "target_basis=['RZ', 'X2P', 'CZ']" in repr(config)


def test_decompose_configs_reject_invalid_target_basis() -> None:
    with pytest.raises(
        CompilerConfigError,
        match="two-qubit synthesis target requires standard instructions",
    ):
        UnitaryDecomposeConfig(target_basis=[Instruction.delay()])
    with pytest.raises(
        CompilerConfigError,
        match="target-basis lowering requires a non-empty target basis",
    ):
        UnitaryDecomposeConfig(target_basis=[])


def test_decompose_unitaries_targets_qcis_native_basis() -> None:
    matrix = np.array(
        [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 0, 1], [0, 0, 1, 0]],
        dtype=np.complex128,
    )
    gate = UnitaryGate("swap_matrix", 2).with_matrix(matrix)
    circuit = Circuit(2)
    circuit.append_unitary_gate(gate, [0, 1])

    target_basis = [
        Instruction.from_standard_gate(StandardGate.RZ),
        Instruction.from_standard_gate(StandardGate.X2P),
        Instruction.from_standard_gate(StandardGate.CZ),
    ]
    decomposed = decompose_unitaries(
        circuit, UnitaryDecomposeConfig(target_basis=target_basis)
    )
    assert decomposed.changed is True
    lowered = TargetBasisLowerer(target_basis).run(decomposed.circuit)

    names = [
        operation.instruction.instruction.name
        for operation in lowered.circuit.operations
    ]
    assert names
    assert set(names) <= {"RZ", "X2P", "CZ"}


def test_two_qubit_block_resynthesis_accepts_target_basis() -> None:
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cz(0, 1)
    circuit.h(1)

    config = TwoQubitBlockResynthesisConfig(
        target_basis=[Instruction.from_standard_gate(StandardGate.CZ)],
        enhanced=True,
        max_block_ops=20,
    )

    assert config.two_qubit_basis is None
    assert [instruction.name for instruction in config.target_basis] == ["CZ"]
    assert config.max_block_ops == 20
    assert "target_basis=['CZ']" in repr(config)

    result = resynthesize_two_qubit_blocks(circuit, config)

    names = [
        operation.instruction.instruction.name
        for operation in result.circuit.operations
    ]
    assert names
    assert set(names) <= {"H", "CZ", "U"}


def test_two_qubit_block_resynthesis_config_default_is_unconstrained() -> None:
    config = TwoQubitBlockResynthesisConfig()

    assert config.two_qubit_basis is None
    assert config.recurse_control_flow is True
    assert "two_qubit_basis=None" in repr(config)


def test_two_qubit_block_resynthesis_python_api_preserves_input() -> None:
    circuit = Circuit(2)
    circuit.cx(0, 1)
    circuit.cx(0, 1)
    config = TwoQubitBlockResynthesisConfig(
        two_qubit_basis=TwoQubitUnitaryDecomposeBasis.cx()
    )

    result = resynthesize_two_qubit_blocks(circuit, config)
    transformer = ResynthesizeTwoQubitBlocks(config)
    transformer_result = transformer.run(circuit)

    assert len(circuit.operations) == 2
    assert result.changed is True
    assert len(result.circuit.operations) == 0
    assert transformer.config == config
    assert transformer_result == result


def test_native_optimizer_exposes_structured_exact_physical_result() -> None:
    device = Device.bidirectional_line("native-optimizer-python", 1)
    device.native_gates = [Instruction.from_standard_gate(StandardGate.U)]
    circuit = Circuit(1)
    circuit.u(0, 0.2, 0.3, 0.4)
    circuit.u(0, -0.1, 0.5, -0.2)
    optimizer = NativeOptimizer(device, enhanced=True)

    result = optimizer.run(circuit)

    assert optimizer.enhanced is True
    assert optimizer.max_rounds == 8
    assert optimizer.max_stale_rounds == 3
    assert optimizer.quality_policy == NativeQualityPolicy.entangler_first()
    assert isinstance(result, NativeOptimizationResult)
    assert isinstance(result.before, NativeOptimizationSummary)
    assert len(circuit.operations) == 2
    assert result.changed is True
    assert result.rounds >= 1
    assert result.before.native_total_ops == 2
    assert result.after.native_total_ops == 1
    assert result.after.total_native_depth <= result.before.total_native_depth
    device.validate_circuit(result.circuit)
    assert copy.copy(optimizer).run(circuit).circuit == result.circuit
    assert copy.deepcopy(result).circuit == result.circuit
    assert copy.copy(result.before) == result.before
    assert "NativeOptimizer" in repr(optimizer)
    assert "NativeOptimizationResult" in repr(result)


def test_native_optimizer_exposes_balanced_quality_policy() -> None:
    device = Device.bidirectional_line("native-quality-policy-python", 1)
    device.native_gates = [Instruction.from_standard_gate(StandardGate.U)]

    balanced = NativeOptimizer(
        device,
        quality_policy=NativeQualityPolicy.balanced_depth(),
    )
    by_name = NativeOptimizer(device, quality_policy="balanced_depth")

    assert balanced.quality_policy == NativeQualityPolicy.balanced_depth()
    assert by_name.quality_policy == NativeQualityPolicy.balanced_depth()
    assert str(balanced.quality_policy) == "balanced_depth"
    assert "quality_policy=NativeQualityPolicy.BalancedDepth" in repr(balanced)

    with pytest.raises(ValueError, match="unknown native quality policy"):
        NativeOptimizer(device, quality_policy="fidelity_first")


def test_device_aware_resynthesis_entry_is_explicit_and_non_mutating() -> None:
    device = Device.bidirectional_line("device-aware-resynthesis-python", 2)
    device.native_gates = [
        Instruction.from_standard_gate(StandardGate.U),
        Instruction.from_standard_gate(StandardGate.CX),
    ]
    circuit = Circuit(2)
    circuit.cx(0, 1)
    circuit.cx(0, 1)

    result = resynthesize_two_qubit_blocks_for_device(
        circuit,
        device,
        placement="exact_physical",
    )

    assert len(circuit.operations) == 2
    assert result.changed is True
    assert len(result.circuit.operations) == 0

    with pytest.raises(
        CompilerConfigError, match="unknown device resynthesis placement"
    ):
        resynthesize_two_qubit_blocks_for_device(
            circuit,
            device,
            placement="logical",
        )


def test_native_optimizer_rejects_invalid_round_budgets() -> None:
    device = Device.bidirectional_line("native-optimizer-invalid-budget", 1)
    device.native_gates = [Instruction.from_standard_gate(StandardGate.U)]

    with pytest.raises(
        CompilerConfigError, match="max_rounds must be greater than zero"
    ):
        NativeOptimizer(device, max_rounds=0)
    with pytest.raises(
        CompilerConfigError, match="max_stale_rounds must be greater than zero"
    ):
        NativeOptimizer(device, max_stale_rounds=0)

    with pytest.raises(OverflowError):
        NativeOptimizer(device, max_rounds=-1)
    with pytest.raises(OverflowError):
        NativeOptimizer(device, max_rounds=256)


def test_device_aware_resynthesis_uses_pre_layout_envelope_by_default() -> None:
    device = Device.bidirectional_line("pre-layout-resynthesis-python", 3)
    device.native_gates = [
        Instruction.from_standard_gate(StandardGate.U),
        Instruction.from_standard_gate(StandardGate.CX),
    ]
    circuit = Circuit(2)
    circuit.cx(0, 1)
    circuit.cx(0, 1)

    result = resynthesize_two_qubit_blocks_for_device(circuit, device)

    assert len(circuit.operations) == 2
    assert result.changed is True
    assert len(result.circuit.operations) == 0


def test_virtual_permutation_elision_is_structured_and_non_mutating() -> None:
    circuit = Circuit(2)
    circuit.swap(0, 1)
    circuit.x(0)

    result = elide_virtual_permutations(circuit)

    assert len(circuit.operations) == 2
    assert result.changed is True
    assert result.status == "changed"
    assert result.elided_swap_count == 1
    assert result.virtual_permutation == {0: 1, 1: 0}
    assert len(result.circuit.operations) == 1
    assert result.circuit.operations[0].instruction.instruction.name == "X"
    assert [qubit.index for qubit in result.circuit.operations[0].qubits] == [1]
    assert copy.deepcopy(result).virtual_permutation == {0: 1, 1: 0}
    assert "VirtualPermutationElisionResult" in repr(result)


def test_target_basis_lowerer_accepts_gate_name_strings() -> None:
    by_names = TargetBasisLowerer(["h", "CZ"])
    by_instructions = TargetBasisLowerer(
        [
            Instruction.from_standard_gate(StandardGate.H),
            Instruction.from_standard_gate(StandardGate.CZ),
        ]
    )

    names = [instruction.name for instruction in by_names.target_basis]
    assert names == ["H", "CZ"]
    assert names == [instruction.name for instruction in by_instructions.target_basis]

    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)
    lowered = by_names.run(circuit)
    assert {op.instruction.instruction.name for op in lowered.circuit.operations} <= {
        "H",
        "CZ",
    }


def test_target_basis_construction_accepts_mixed_entries() -> None:
    optimizer = OptimizeOneQubitRuns.basis(
        ["H", Instruction.from_standard_gate(StandardGate.CZ)]
    )

    assert optimizer.policy == "basis"
    assert [instruction.name for instruction in optimizer.target_basis] == ["H", "CZ"]


def test_target_basis_reprs_round_trip_through_eval() -> None:
    namespace = {
        "CommutationConfig": CommutationConfig,
        "OptimizeOneQubitRuns": OptimizeOneQubitRuns,
        "TargetBasisCostModel": TargetBasisCostModel,
        "TargetBasisLowerer": TargetBasisLowerer,
        "TwoQubitBlockResynthesisConfig": TwoQubitBlockResynthesisConfig,
        "UnitaryDecomposeConfig": UnitaryDecomposeConfig,
    }
    values = [
        OptimizeOneQubitRuns.logical(),
        OptimizeOneQubitRuns.basis(["RZ", "X2P", "CZ"]),
        TargetBasisCostModel(["RZ", "X2P", "CZ"]),
        TargetBasisLowerer(["RZ", "X2P", "CZ"]),
        UnitaryDecomposeConfig(target_basis=["RZ", "X2P", "CZ"]),
        TwoQubitBlockResynthesisConfig(target_basis=["CZ"]),
    ]

    for value in values:
        rebuilt = eval(repr(value), dict(namespace))
        assert repr(rebuilt) == repr(value)


def test_target_basis_cost_model_repr_exposes_basis() -> None:
    model = TargetBasisCostModel(["RZ", "X2P", "CZ"])

    assert repr(model) == "TargetBasisCostModel(target_basis=['RZ', 'X2P', 'CZ'])"
    assert [instruction.name for instruction in model.target_basis] == [
        "RZ",
        "X2P",
        "CZ",
    ]


def test_target_basis_entries_reject_unknown_gate_names() -> None:
    factories = (
        lambda names: OptimizeOneQubitRuns.basis(names),
        lambda names: TargetBasisCostModel(names),
        lambda names: TargetBasisLowerer(names),
        lambda names: UnitaryDecomposeConfig(target_basis=names),
        lambda names: TwoQubitBlockResynthesisConfig(target_basis=names),
    )

    for factory in factories:
        with pytest.raises(CompilerConfigError, match="unknown standard gate"):
            factory(["H", "not-a-gate"])


def test_target_basis_supports_mcgate_instructions_with_name_repr() -> None:
    basis = [
        Instruction.from_name("H"),
        Instruction.from_mc_gate(MCGate(2, StandardGate.X)),
    ]
    lowerer = TargetBasisLowerer(basis)

    # MCGate entries print their stable name; the string form is not
    # eval-reconstructable and must be passed as an Instruction object.
    assert repr(lowerer) == "TargetBasisLowerer(target_basis=['H', 'C2-X'])"
