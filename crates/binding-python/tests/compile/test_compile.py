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

import math
import copy

import numpy as np
import pytest

from cqlib.circuit import Circuit, Instruction, MCGate, Parameter, StandardGate
from cqlib.compile import (
    CompileMode,
    CompileResult,
    CompilerConfigError,
    WorkflowStepReport,
    compile,
)
from cqlib.compile.compiler import compile as compile_from_submodule
from cqlib.device import Device, Layout


ATOL = 1e-10


def instruction(gate: StandardGate) -> Instruction:
    return Instruction.from_standard_gate(gate)


def operation_name(operation) -> str:
    inner = operation.instruction.instruction
    assert inner is not None, f"operation has no storage instruction: {operation!r}"
    return inner.name.upper()


def operation_signature(circuit: Circuit) -> tuple:
    return tuple(
        (
            operation_name(operation),
            tuple(qubit.index for qubit in operation.qubits),
            tuple(str(param) for param in operation.params),
        )
        for operation in circuit.operations
    )


def step_names(result: CompileResult) -> list[str]:
    return [step.name for step in result.steps]


def step(result: CompileResult, name: str) -> WorkflowStepReport:
    matches = [entry for entry in result.steps if entry.name == name]
    assert matches, f"missing workflow step {name!r}; got {step_names(result)!r}"
    return matches[0]


def assert_step_subsequence(result: CompileResult, expected: list[str]) -> None:
    names = step_names(result)
    cursor = 0
    for name in expected:
        try:
            cursor = names.index(name, cursor) + 1
        except ValueError as exc:
            raise AssertionError(
                f"{name!r} not found after {names[:cursor]!r}"
            ) from exc


def assert_unitary_equivalent(
    source: Circuit,
    compiled: Circuit,
    bindings: list[dict[str, float] | None] | None = None,
) -> None:
    cases = bindings if bindings is not None else [None]
    for case in cases:
        left = source.assign_parameters(case) if case is not None else source
        right = compiled.assign_parameters(case) if case is not None else compiled
        np.testing.assert_allclose(right.to_matrix(), left.to_matrix(), atol=ATOL)


def assert_only_standard_basis(circuit: Circuit, allowed: set[str]) -> None:
    assert circuit.operations, "compiled circuit unexpectedly has no operations"
    for operation in circuit.operations:
        inner = operation.instruction.instruction
        assert inner is not None
        assert inner.is_standard, f"{operation!r} is not a standard instruction"
        assert operation_name(operation) in allowed


def assert_no_high_level_instructions(circuit: Circuit) -> None:
    for operation in circuit.operations:
        inner = operation.instruction.instruction
        assert inner is not None
        assert not inner.is_mcgate
        assert not inner.is_unitary
        assert not inner.is_circuit_gate


def assert_all_two_qubit_ops_on_topology(circuit: Circuit, device: Device) -> None:
    topology = device.topology
    for operation in circuit.operations:
        if len(operation.qubits) != 2:
            continue
        q0, q1 = (qubit.index for qubit in operation.qubits)
        assert topology.supports_coupling_either_direction(q0, q1), (
            f"{operation_name(operation)} is on non-edge ({q0}, {q1})"
        )


def qcis_cz_basis() -> list[Instruction]:
    return [
        instruction(StandardGate.RZ),
        instruction(StandardGate.X2P),
        instruction(StandardGate.X2M),
        instruction(StandardGate.Y2P),
        instruction(StandardGate.Y2M),
        instruction(StandardGate.CZ),
        instruction(StandardGate.GPhase),
    ]


def qcis_native_basis() -> list[Instruction]:
    return [
        instruction(StandardGate.I),
        instruction(StandardGate.RZ),
        instruction(StandardGate.X2P),
        instruction(StandardGate.X2M),
        instruction(StandardGate.Y2P),
        instruction(StandardGate.Y2M),
        instruction(StandardGate.XY2P),
        instruction(StandardGate.XY2M),
        instruction(StandardGate.CZ),
        instruction(StandardGate.GPhase),
    ]


def standard_names(basis: list[Instruction]) -> set[str]:
    names = set()
    for item in basis:
        assert item.is_standard
        names.add(item.name.upper())
    return names


def bell_circuit() -> Circuit:
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)
    return circuit


def qft3_circuit() -> Circuit:
    circuit = Circuit(3)
    circuit.h(0)
    circuit.crz(1, 0, math.pi / 2.0)
    circuit.crz(2, 0, math.pi / 4.0)
    circuit.h(1)
    circuit.crz(2, 1, math.pi / 2.0)
    circuit.h(2)
    return circuit


def controlled_rotation_circuit() -> Circuit:
    circuit = Circuit(3)
    circuit.h(0)
    circuit.crx(0, 1, 0.31)
    circuit.cry(1, 2, -0.27)
    circuit.crz(2, 0, 0.19)
    return circuit


def two_qubit_suite_without_fsim() -> Circuit:
    circuit = Circuit(4)
    circuit.h(0)
    circuit.rx(1, 0.17)
    circuit.ry(2, -0.23)
    circuit.rz(3, 0.29)
    circuit.cx(0, 2)
    circuit.cy(1, 3)
    circuit.cz(2, 0)
    circuit.swap(3, 1)
    circuit.rxx(0, 1, 0.31)
    circuit.ryy(2, 3, -0.37)
    circuit.rzz(1, 2, 0.41)
    circuit.rzx(3, 0, -0.43)
    circuit.crx(0, 3, 0.47)
    circuit.cry(1, 2, -0.53)
    circuit.crz(2, 0, 0.59)
    return circuit


def ising_exchange_circuit() -> Circuit:
    circuit = Circuit(2)
    circuit.h(0)
    circuit.rx(1, 0.17)
    circuit.rxx(0, 1, 0.29)
    circuit.ryy(0, 1, -0.31)
    circuit.rzz(0, 1, 0.37)
    circuit.fsim(0, 1, 0.13, -0.17)
    return circuit


def long_range_device_circuit(num_qubits: int = 4) -> Circuit:
    circuit = Circuit(num_qubits)
    circuit.h(0)
    circuit.cx(0, num_qubits - 1)
    circuit.rzz(1, num_qubits - 1, 0.37)
    circuit.crz(num_qubits - 1, 0, -0.21)
    return circuit


def parameterized_circuit() -> Circuit:
    theta = Parameter("theta")
    phi = Parameter("phi")
    circuit = Circuit(3)
    circuit.rx(0, theta)
    circuit.rz(2, phi)
    circuit.cx(0, 2)
    circuit.rzz(1, 2, 0.37)
    circuit.crz(2, 0, theta)
    return circuit


def mc_gate_circuit(
    num_controls: int = 3, gate: StandardGate = StandardGate.X
) -> Circuit:
    circuit = Circuit(num_controls + 1)
    circuit.append_mc_gate(MCGate(num_controls, gate), list(range(num_controls + 1)))
    return circuit


def test_compile_api_exports_modes_results_and_readonly_reports():
    result = compile_from_submodule(bell_circuit())

    assert isinstance(result, CompileResult)
    assert result.mode == CompileMode.normal()
    assert CompileMode.normal() != CompileMode.enhanced()
    assert hash(CompileMode.normal()) == hash(copy.copy(CompileMode.normal()))
    assert repr(CompileMode.normal()) == "CompileMode.Normal"
    assert isinstance(result.steps[0], WorkflowStepReport)
    assert "CompileResult" in repr(result)
    assert "WorkflowStepReport" in repr(result.steps[0])

    with pytest.raises(AttributeError):
        result.changed = False
    with pytest.raises(AttributeError):
        result.steps[0].name = "changed"


def test_compile_workflow_reports_public_step_order_and_skip_reasons():
    result = compile(bell_circuit())

    assert_step_subsequence(
        result,
        [
            "canonicalize.input",
            "decompose.definitions",
            "optimize.pre_decomposition",
            "decompose.unitary",
            "decompose.mc_gates",
            "canonicalize.after_decomposition",
            "optimize.post_decomposition",
            "decompose.routing_basis",
            "route.sabre",
            "translate.target_basis",
            "canonicalize.output",
        ],
    )
    assert step(result, "decompose.routing_basis").skipped
    assert step(result, "decompose.routing_basis").reason
    assert step(result, "route.sabre").skipped
    assert step(result, "route.sabre").reason
    assert step(result, "translate.target_basis").skipped
    assert step(result, "translate.target_basis").reason


def test_compile_preserves_unitary_for_varied_logical_inputs():
    for source in [
        bell_circuit(),
        qft3_circuit(),
        controlled_rotation_circuit(),
        two_qubit_suite_without_fsim(),
        ising_exchange_circuit(),
        mc_gate_circuit(2, StandardGate.X),
        mc_gate_circuit(3, StandardGate.X),
    ]:
        result = compile(source)

        assert_no_high_level_instructions(result.circuit)
        assert_unitary_equivalent(source, result.circuit)


def test_compile_simplifies_rotations_without_changing_unitary():
    circuit = Circuit(1)
    circuit.rz(0, 0.25)
    circuit.rz(0, 0.5)
    circuit.rz(0, -0.75)

    result = compile(circuit, mode=CompileMode.enhanced())

    assert result.changed
    assert len(result.circuit.operations) == 0
    assert_unitary_equivalent(circuit, result.circuit)


def test_compile_preserves_parameterized_semantics_for_multiple_bindings():
    source = parameterized_circuit()

    result = compile(source)

    assert "theta" in result.circuit.symbols
    assert "phi" in result.circuit.symbols
    assert_unitary_equivalent(
        source,
        result.circuit,
        [
            {"theta": 0.0, "phi": 0.0},
            {"theta": 0.37, "phi": -0.41},
            {"theta": -math.pi / 3.0, "phi": math.pi / 7.0},
        ],
    )


def test_compile_target_basis_exact_h_cz_lowering_for_bell():
    source = bell_circuit()
    basis = [instruction(StandardGate.H), instruction(StandardGate.CZ)]

    result = compile(source, target_basis=basis)

    assert step(result, "translate.target_basis").changed
    assert operation_signature(result.circuit) == (
        ("H", (0,), ()),
        ("H", (1,), ()),
        ("CZ", (0, 1), ()),
        ("H", (1,), ()),
    )
    assert_only_standard_basis(result.circuit, {"H", "CZ"})
    assert_unitary_equivalent(source, result.circuit)


def test_compile_target_basis_accepts_case_insensitive_gate_names():
    source = bell_circuit()

    result = compile(source, target_basis=["h", "cZ"])

    assert step(result, "translate.target_basis").changed
    assert_only_standard_basis(result.circuit, {"H", "CZ"})
    assert_unitary_equivalent(source, result.circuit)


def test_compile_target_basis_accepts_mixed_names_and_instructions():
    source = bell_circuit()

    result = compile(
        source,
        target_basis=["H", instruction(StandardGate.CZ)],
    )

    assert_only_standard_basis(result.circuit, {"H", "CZ"})
    assert_unitary_equivalent(source, result.circuit)


def test_compile_target_basis_rejects_unknown_names_and_invalid_types():
    with pytest.raises(CompilerConfigError, match="NOT_A_GATE"):
        compile(bell_circuit(), target_basis=["NOT_A_GATE"])

    with pytest.raises(TypeError):
        compile(bell_circuit(), target_basis=[42])


def test_compile_target_basis_lowers_complex_gate_suite_to_qcis_cz_basis():
    source = two_qubit_suite_without_fsim()
    basis = qcis_cz_basis()

    result = compile(source, target_basis=basis)

    assert step(result, "translate.target_basis").changed
    assert_only_standard_basis(result.circuit, standard_names(basis))
    assert_unitary_equivalent(source, result.circuit)


def test_compile_target_basis_lowers_controlled_rotations_to_rzz_native_basis():
    source = controlled_rotation_circuit()
    basis = [
        instruction(StandardGate.H),
        instruction(StandardGate.RX),
        instruction(StandardGate.RZ),
        instruction(StandardGate.RZZ),
        instruction(StandardGate.GPhase),
    ]

    result = compile(source, target_basis=basis)

    assert step(result, "translate.target_basis").changed
    assert_only_standard_basis(result.circuit, standard_names(basis))
    assert "RZZ" in [
        operation_name(operation) for operation in result.circuit.operations
    ]
    assert_unitary_equivalent(source, result.circuit)


def test_compile_target_basis_lowers_fsim_to_ising_exchange_basis():
    source = ising_exchange_circuit()
    basis = [
        instruction(StandardGate.H),
        instruction(StandardGate.RX),
        instruction(StandardGate.RY),
        instruction(StandardGate.RZ),
        instruction(StandardGate.RXX),
        instruction(StandardGate.RYY),
        instruction(StandardGate.RZZ),
        instruction(StandardGate.GPhase),
    ]

    result = compile(source, target_basis=basis)

    assert step(result, "translate.target_basis").changed
    names = [operation_name(operation) for operation in result.circuit.operations]
    assert "FSIM" not in names
    assert "RXX" in names
    assert "RYY" in names
    assert_only_standard_basis(result.circuit, standard_names(basis))
    assert_unitary_equivalent(source, result.circuit)


def test_compile_target_basis_preserves_parameterized_semantics():
    source = parameterized_circuit()
    basis = qcis_cz_basis()

    result = compile(source, target_basis=basis)

    assert step(result, "translate.target_basis").changed
    assert_only_standard_basis(result.circuit, standard_names(basis))
    assert "theta" in result.circuit.symbols
    assert "phi" in result.circuit.symbols
    assert_unitary_equivalent(
        source,
        result.circuit,
        [
            {"theta": 0.0, "phi": 0.0},
            {"theta": 0.21, "phi": -0.19},
            {"theta": -math.pi / 5.0, "phi": math.pi / 9.0},
        ],
    )


def test_compile_rejects_unsupported_and_non_standard_target_basis():
    circuit = Circuit(1)
    circuit.h(0)

    with pytest.raises(CompilerConfigError, match="H"):
        compile(circuit, target_basis=[instruction(StandardGate.CZ)])

    with pytest.raises(
        CompilerConfigError, match="unsupported workflow target instruction"
    ):
        compile(
            bell_circuit(),
            target_basis=[Instruction.from_mc_gate(MCGate(2, StandardGate.X))],
        )

    reduced_ising_basis = [
        instruction(StandardGate.H),
        instruction(StandardGate.RX),
        instruction(StandardGate.RY),
        instruction(StandardGate.RZ),
        instruction(StandardGate.RZZ),
        instruction(StandardGate.GPhase),
    ]
    result = compile(ising_exchange_circuit(), target_basis=reduced_ising_basis)
    names = [operation_name(operation) for operation in result.circuit.operations]
    assert "FSIM" not in names
    assert_only_standard_basis(result.circuit, standard_names(reduced_ising_basis))
    assert_unitary_equivalent(ising_exchange_circuit(), result.circuit)


def test_compile_routes_long_range_circuit_on_line_device():
    source = long_range_device_circuit(4)
    device = Device.line("line-4", 4)
    device.native_gates = qcis_cz_basis()

    result = compile(source, device=device, seed=101)

    assert not step(result, "route.sabre").skipped
    assert step(result, "route.sabre").changed
    assert_all_two_qubit_ops_on_topology(result.circuit, device)
    assert result.circuit.num_qubits <= 4


def test_compile_uses_device_native_gates_as_target_basis_after_routing():
    source = long_range_device_circuit(4)
    basis = qcis_cz_basis()
    device = Device.line("line-qcis", 4)
    device.native_gates = basis

    result = compile(source, device=device, seed=102)

    assert not step(result, "route.sabre").skipped
    assert step(result, "translate.target_basis").skipped
    assert not step(result, "validate.device").skipped
    assert_all_two_qubit_ops_on_topology(result.circuit, device)
    assert_only_standard_basis(result.circuit, standard_names(basis))


def test_compile_explicit_target_basis_takes_precedence_over_device_native_gates():
    source = bell_circuit()
    device = Device.line("line-native-cx", 2)
    device.native_gates = [instruction(StandardGate.CX)]
    explicit_basis = [instruction(StandardGate.H), instruction(StandardGate.CZ)]

    with pytest.warns(UserWarning, match="not guaranteed"):
        result = compile(source, target_basis=explicit_basis, device=device, seed=11)

    assert step(result, "translate.target_basis").changed
    assert_only_standard_basis(result.circuit, {"H", "CZ"})
    assert "CX" not in [
        operation_name(operation) for operation in result.circuit.operations
    ]
    assert_unitary_equivalent(source, result.circuit)


def test_compile_enhanced_device_workflow_runs_post_routing_and_skips_target_cleanup():
    source = Circuit(3)
    source.h(0)
    source.cx(0, 2)
    device = Device.line("enhanced-line", 3)
    device.native_gates = [instruction(StandardGate.H), instruction(StandardGate.CZ)]

    result = compile(source, mode=CompileMode.enhanced(), device=device, seed=42)

    assert result.mode == CompileMode.enhanced()
    assert not step(result, "route.sabre").skipped
    assert not step(result, "optimize.post_routing").skipped
    target_cleanup = step(result, "optimize.target_cleanup")
    assert target_cleanup.skipped
    assert not target_cleanup.changed
    assert target_cleanup.reason == "no explicit target basis configured"
    assert_only_standard_basis(result.circuit, {"H", "CZ"})


def test_compile_with_same_seed_is_deterministic_for_routing_and_reports():
    source = long_range_device_circuit(4)
    device = Device.ring("deterministic-ring", 4)
    device.native_gates = qcis_cz_basis()

    first = compile(source, mode=CompileMode.enhanced(), device=device, seed=2026)
    second = compile(source, mode=CompileMode.enhanced(), device=device, seed=2026)

    assert operation_signature(first.circuit) == operation_signature(second.circuit)
    assert first.changed == second.changed
    assert [
        (entry.stage, entry.name, entry.changed, entry.skipped, entry.reason)
        for entry in first.steps
    ] == [
        (entry.stage, entry.name, entry.changed, entry.skipped, entry.reason)
        for entry in second.steps
    ]


def test_compile_routes_from_supplied_initial_layout_and_reports_reason():
    source = Circuit(1)
    source.h(0)
    device = Device.line("layout-line", 3)
    device.native_gates = [instruction(StandardGate.H)]
    layout = Layout.from_pairs([(0, 2)], 3)

    result = compile(source, device=device, initial_layout=layout, seed=17)

    routing = step(result, "route.sabre")
    assert not routing.skipped
    assert routing.changed
    assert routing.reason is not None
    assert "supplied initial layout" in routing.reason
    assert operation_signature(result.circuit)[0][1] == (2,)


def test_compile_rejects_invalid_device_and_layout_configurations():
    too_wide = Circuit(4)
    too_wide.h(0)

    with pytest.raises(CompilerConfigError, match="4 logical qubits"):
        device = Device.line("line-2", 2)
        device.native_gates = [instruction(StandardGate.H)]
        compile(too_wide, device=device)

    layout = Layout.from_pairs([(0, 0)], 1)
    with pytest.raises(
        CompilerConfigError, match="initial_layout requires a target device"
    ):
        compile(Circuit(1), initial_layout=layout)

    with pytest.raises(CompilerConfigError, match="device has no native gates"):
        compile(Circuit(1), device=Device.line("no-native-gates", 1))


def test_compile_decomposes_multi_controlled_gates_without_high_level_residue():
    ccx = mc_gate_circuit(2, StandardGate.X)
    ccx_result = compile(ccx)
    assert_no_high_level_instructions(ccx_result.circuit)
    assert_unitary_equivalent(ccx, ccx_result.circuit)

    for source in [
        mc_gate_circuit(3, StandardGate.X),
        mc_gate_circuit(3, StandardGate.X2P),
    ]:
        result = compile(source)

        assert step(result, "decompose.mc_gates").changed
        assert_no_high_level_instructions(result.circuit)
        assert_unitary_equivalent(source, result.circuit)


def test_compile_preserves_parameterized_multi_controlled_gate():
    theta = Parameter("theta")
    source = Circuit(3)
    source.append_mc_gate(MCGate(2, StandardGate.RZ(theta)), [0, 1, 2])

    result = compile(source)

    assert step(result, "decompose.mc_gates").changed
    assert "theta" in result.circuit.symbols
    assert_no_high_level_instructions(result.circuit)
    assert_unitary_equivalent(
        source,
        result.circuit,
        [{"theta": 0.0}, {"theta": 0.31}, {"theta": -math.pi / 3.0}],
    )
