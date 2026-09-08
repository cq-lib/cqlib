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

import pytest

import cqlib.compile as compile_module
from cqlib.circuit import Circuit, Instruction, StandardGate
from cqlib.compile import (
    CompileConfig,
    CompileMode,
    CompileResult,
    CompileTarget,
    CompilerConfigError,
    CompilerError,
    CompilerInternalError,
    CompilerTransformError,
    CompilerWorkflow,
    DeviceCompilationMetadata,
    WorkflowStepReport,
    compile,
)
from cqlib.compile.resource import ResourcePolicy
from cqlib.device import Device, Layout


def instruction_names(instructions: list[Instruction] | None) -> list[str] | None:
    if instructions is None:
        return None
    return [instruction.name for instruction in instructions]


def test_workflow_types_are_public_compile_types() -> None:
    assert CompileConfig.__module__ == "cqlib.compile"
    assert CompilerWorkflow.__module__ == "cqlib.compile"
    assert "CompileConfig" in compile_module.__all__
    assert "CompilerWorkflow" in compile_module.__all__
    assert repr(CompileMode.normal()) == "CompileMode.Normal"
    assert repr(CompileMode.enhanced()) == "CompileMode.Enhanced"


@pytest.mark.parametrize("mode", [CompileMode.enhanced(), "enhanced", "ENHANCED"])
def test_compile_mode_accepts_objects_and_strings(mode: object) -> None:
    circuit = Circuit(1)
    circuit.h(0)

    assert compile(circuit, mode=mode).mode == CompileMode.enhanced()
    assert CompileConfig(mode=mode).mode == CompileMode.enhanced()


def test_compile_mode_rejects_unknown_strings() -> None:
    with pytest.raises(CompilerConfigError, match="unknown compile mode"):
        compile(Circuit(1), mode="fast")

    with pytest.raises(CompilerConfigError, match="unknown compile mode"):
        CompileConfig(mode="fast")


def test_compiler_errors_are_public_compile_exceptions() -> None:
    assert CompilerError.__module__ == "cqlib.compile"
    assert CompilerConfigError.__module__ == "cqlib.compile"
    assert CompilerTransformError.__module__ == "cqlib.compile"
    assert CompilerInternalError.__module__ == "cqlib.compile"
    assert issubclass(CompilerConfigError, CompilerError)
    assert issubclass(CompilerTransformError, CompilerError)
    assert issubclass(CompilerInternalError, CompilerError)
    assert not issubclass(CompilerConfigError, ValueError)
    assert {
        "CompilerError",
        "CompilerConfigError",
        "CompilerTransformError",
        "CompilerInternalError",
    } <= set(compile_module.__all__)


def test_compile_config_exposes_immutable_defaults_and_copy_protocol() -> None:
    config = CompileConfig()

    assert config.mode == CompileMode.normal()
    assert config.target.kind == "logical"
    assert config.target.basis_instructions is None
    assert config.target.device_target is None
    assert config.resource_policy == ResourcePolicy()
    assert copy.copy(config) is not config
    assert copy.deepcopy(config) is not config
    assert repr(config).startswith("CompileConfig(mode=CompileMode.Normal,")

    with pytest.raises(AttributeError):
        config.target = CompileTarget.logical()


def test_compile_config_takes_basis_and_device_target_snapshots() -> None:
    basis = ["H"]
    device = Device.line("line-2", 2)
    device.native_gates = [Instruction.from_standard_gate(StandardGate.H)]
    layout = Layout.from_pairs([(0, 0)], physical_count=2)
    policy = ResourcePolicy(max_pre_layout_clean_ancillas=2)
    basis_config = CompileConfig(target=CompileTarget.basis(basis))
    device_config = CompileConfig(
        target=CompileTarget.device(device, initial_layout=layout, seed=7),
        resource_policy=policy,
    )

    basis.append("CZ")
    device.native_gates = [Instruction.from_standard_gate(StandardGate.X)]
    layout.bind(1, 1)

    assert instruction_names(basis_config.target.basis_instructions) == ["H"]
    target = device_config.target.device_target
    assert target is not None
    assert instruction_names(target.device.native_gates) == ["H"]
    assert target.initial_layout is not None
    assert target.initial_layout.num_logical == 1
    assert target.seed == 7
    assert device_config.resource_policy == policy

    returned_device = target.device
    returned_device.native_gates = [Instruction.from_standard_gate(StandardGate.Z)]
    target = device_config.target.device_target
    assert target is not None
    assert instruction_names(target.device.native_gates) == ["H"]


def test_compiler_workflow_owns_config_snapshot_and_is_reusable() -> None:
    config = CompileConfig(mode=CompileMode.enhanced())
    workflow = CompilerWorkflow(config)
    circuit = Circuit(1)
    circuit.h(0)
    circuit.h(0)

    first = workflow.run(circuit)
    second = workflow.run(circuit)

    assert isinstance(first, CompileResult)
    assert first.mode == CompileMode.enhanced()
    assert first.changed is True
    assert len(first.circuit.operations) == 0
    assert len(second.circuit.operations) == 0
    assert len(circuit.operations) == 2
    assert workflow.config.target.kind == "logical"
    assert workflow.config is not workflow.config
    assert all(isinstance(step, WorkflowStepReport) for step in first.steps)
    assert any(step.name == "optimize.target_cleanup" for step in first.steps)


def test_compile_and_explicit_workflow_have_equivalent_results() -> None:
    circuit = Circuit(2)
    circuit.cx(0, 1)
    basis = ["H", "CZ"]

    target = CompileTarget.basis(basis)
    direct = compile(circuit, target=target)
    explicit = CompilerWorkflow(CompileConfig(target=target)).run(circuit)

    direct_names = [
        str(operation.instruction) for operation in direct.circuit.operations
    ]
    explicit_names = [
        str(operation.instruction) for operation in explicit.circuit.operations
    ]
    assert direct_names == explicit_names == ["H", "CZ", "H"]
    assert [step.name for step in direct.steps] == [
        step.name for step in explicit.steps
    ]
    assert direct == explicit
    assert direct.__eq__(object()) is NotImplemented


def test_compile_combines_loose_basis_and_device_arguments() -> None:
    circuit = Circuit(2)
    circuit.cx(0, 1)
    device = Device.bidirectional_line("loose-topology-basis", 2)
    device.native_gates = [Instruction.from_standard_gate(StandardGate.CX)]

    with pytest.warns(UserWarning, match="not guaranteed"):
        result = compile(
            circuit,
            target_basis=["H", "CZ"],
            device=device,
            seed=11,
        )

    assert [operation.instruction.name for operation in result.circuit.operations] == [
        "H",
        "CZ",
        "H",
    ]
    assert result.device_metadata is not None
    assert result.step("lower.device_instructions").skipped
    assert result.step("validate.device").skipped


def test_explicit_topology_basis_target_is_inspectable_and_does_not_warn() -> None:
    circuit = Circuit(2)
    circuit.cx(0, 1)
    device = Device.bidirectional_line("explicit-topology-basis", 2)
    target = CompileTarget.topology_basis(
        device,
        ["H", "CZ"],
    )

    assert target.kind == "topology_basis"
    assert instruction_names(target.basis_instructions) == ["H", "CZ"]
    assert target.device_target is not None
    assert target.device_target.seed is None
    result = compile(circuit, target=target, seed=13)
    assert result.device_metadata is not None
    topology_validation = result.step("validate.topology")
    assert topology_validation is not None
    assert topology_validation.skipped is False


@pytest.mark.parametrize(
    ("kwargs", "message"),
    [
        (
            {"target": CompileTarget.logical(), "target_basis": ["H"]},
            "target cannot be combined",
        ),
        (
            {"initial_layout": Layout.from_pairs([(0, 0)], 1)},
            "initial_layout requires a target device",
        ),
        ({"seed": 7}, "seed requires a target device"),
    ],
)
def test_compile_rejects_conflicting_or_orphaned_target_arguments(
    kwargs: dict[str, object], message: str
) -> None:
    with pytest.raises(CompilerConfigError, match=message):
        compile(Circuit(1), **kwargs)


@pytest.mark.parametrize(
    "run",
    [compile, lambda circuit: CompilerWorkflow().run(circuit)],
)
def test_compiler_entry_points_release_gil(run, assert_releases_gil) -> None:
    circuit = Circuit(1)
    for _ in range(20_000):
        circuit.h(0)

    assert_releases_gil(lambda: run(circuit))


def test_device_compile_returns_layout_metadata() -> None:
    circuit = Circuit(1)
    circuit.h(0)
    device = Device.line("native-h", 1)
    device.native_gates = [Instruction.from_standard_gate(StandardGate.H)]
    target = CompileTarget.device(device)

    result = compile(circuit, target=target, seed=7)

    assert isinstance(result.device_metadata, DeviceCompilationMetadata)
    assert result.device_metadata.initial_layout.num_logical == 1
    assert result.device_metadata.final_layout.num_logical == 1
    assert result.device_metadata.virtual_permutation == {0: 0}
    assert copy.copy(result.device_metadata) == result.device_metadata
    assert result.step("validate.device") is not None
    assert result.step("missing") is None
    assert result.step_changed("route.sabre") is False


def test_device_compile_exposes_elided_output_permutation() -> None:
    circuit = Circuit(2)
    circuit.swap(0, 1)
    device = Device.bidirectional_line("virtual-swap", 2)
    device.native_gates = [
        Instruction.from_standard_gate(StandardGate.H),
        Instruction.from_standard_gate(StandardGate.X),
        Instruction.from_standard_gate(StandardGate.CX),
    ]

    result = compile(circuit, device=device, seed=17)

    assert result.device_metadata is not None
    assert result.device_metadata.virtual_permutation == {0: 1, 1: 0}
    assert result.step_changed("optimize.virtual_permutation") is True
    assert all(
        operation.instruction.name != "SWAP" for operation in result.circuit.operations
    )


def test_compile_config_rejects_unknown_target_gate_name() -> None:
    with pytest.raises(CompilerConfigError, match="unknown standard gate"):
        CompileTarget.basis(["not-a-gate"])


def test_compile_targets_reject_empty_basis_and_native_gate_sets() -> None:
    device = Device.line("no-native-gates", 1)

    with pytest.raises(CompilerConfigError, match="basis must not be empty"):
        CompileTarget.basis([])
    with pytest.raises(CompilerConfigError, match="basis must not be empty"):
        CompileTarget.topology_basis(device, [])
    with pytest.raises(CompilerConfigError, match="device has no native gates"):
        CompileTarget.device(device)


def test_workflow_rejects_non_standard_target_instruction_when_run() -> None:
    config = CompileConfig(target=CompileTarget.basis((Instruction.delay(),)))

    with pytest.raises(
        CompilerConfigError, match="unsupported workflow target instruction"
    ):
        CompilerWorkflow(config).run(Circuit(1))
