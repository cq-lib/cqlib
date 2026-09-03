# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

import copy
import sys

import numpy as np
import pytest

from cqlib.circuit import (
    Circuit,
    MCGate,
    ParameterError,
    Qubit,
    StandardGate,
    UnitaryGate,
)
from cqlib.compile import CompilerConfigError, CompilerTransformError
from cqlib.compile.resource import ResourceLimits, ResourcePolicy
from cqlib.compile.transform import TransformResult, decompose
from cqlib.compile.transform.decompose import (
    DecompositionRuleStats,
    McGateDecomposeConfig,
    TwoQubitUnitaryDecomposeBasis,
    UnitaryDecomposeConfig,
    decompose_mc_gates,
    decompose_mc_gates_for_device,
    decompose_mc_gates_with_rule_stats,
    decompose_unitaries,
    decompose_unitaries_with_rule_stats,
    expand_definitions,
)
from cqlib.compile.transform.decompose.unitary import (
    KakDecomposition,
    OneQubitUnitaryDecomposition,
    TwoQubitUnitarySynthesisResult,
    kak_decompose,
    synthesize_numeric_1q_unitary,
    synthesize_numeric_2q_unitary,
)
from cqlib.compile.transform.decompose import mc_gate as mc
from cqlib.device import Device


def operation_names(circuit: Circuit) -> list[str]:
    return [operation.instruction.instruction.name for operation in circuit.operations]


def test_decompose_module_and_types_are_registered() -> None:
    assert "cqlib._native.compile.transform.decompose" in sys.modules
    assert decompose.expand_definitions is expand_definitions
    assert TransformResult.__module__ == "cqlib.compile.transform"
    assert DecompositionRuleStats.__module__ == "cqlib.compile.transform.decompose"
    assert UnitaryDecomposeConfig.__module__ == "cqlib.compile.transform.decompose"
    assert McGateDecomposeConfig.__module__ == "cqlib.compile.transform.decompose"


def test_decompose_configs_are_immutable_copyable_values() -> None:
    pauli = TwoQubitUnitaryDecomposeBasis.pauli_rotations()
    cx = TwoQubitUnitaryDecomposeBasis.cx()
    cy = TwoQubitUnitaryDecomposeBasis.cy()
    cz = TwoQubitUnitaryDecomposeBasis.cz()
    rzz = TwoQubitUnitaryDecomposeBasis.rzz()
    assert pauli != cx
    assert len({pauli, cx, cy, cz, rzz}) == 5
    assert copy.copy(pauli) == pauli
    assert copy.deepcopy(cx) == cx
    assert repr(cy) == "TwoQubitUnitaryDecomposeBasis.cy()"
    assert repr(cz) == "TwoQubitUnitaryDecomposeBasis.cz()"
    assert repr(rzz) == "TwoQubitUnitaryDecomposeBasis.rzz()"

    unitary = UnitaryDecomposeConfig(two_qubit_basis=cx, recurse_control_flow=False)
    assert unitary.two_qubit_basis == cx
    assert unitary.recurse_control_flow is False
    assert copy.copy(unitary) == unitary

    policy = ResourcePolicy(
        max_pre_layout_clean_ancillas=2,
        allow_dirty_borrowing=True,
    )
    limits = ResourceLimits(max_total_qubits=8)
    mc_gate = McGateDecomposeConfig(resource_policy=policy, resource_limits=limits)
    assert mc_gate.resource_policy == policy
    assert mc_gate.resource_limits == limits
    assert copy.deepcopy(mc_gate) == mc_gate

    with pytest.raises(AttributeError):
        unitary.recurse_control_flow = True


def test_expand_definitions_returns_changed_result_without_mutating_input() -> None:
    definition = Circuit(1)
    definition.h(0)
    circuit = Circuit(1)
    circuit.append_circuit_gate(definition.to_gate("custom_h"), [0])

    result = expand_definitions(circuit)

    assert isinstance(result, TransformResult)
    assert result.changed is True
    assert operation_names(circuit) == ["custom_h"]
    assert operation_names(result.circuit) == ["H"]
    assert copy.copy(result).changed is True


def test_expand_definitions_reports_unchanged_standard_circuit() -> None:
    circuit = Circuit(1)
    circuit.h(0)

    result = expand_definitions(circuit)

    assert result.changed is False
    assert operation_names(result.circuit) == ["H"]


def test_decompose_unitaries_preserves_matrix_and_reports_cache_stats() -> None:
    matrix = np.array([[0, 1], [1, 0]], dtype=np.complex128)
    gate = UnitaryGate("x_matrix", 1).with_matrix(matrix)
    circuit = Circuit(2)
    circuit.append_unitary_gate(gate, [0])
    circuit.append_unitary_gate(gate, [1])

    result, stats = decompose_unitaries_with_rule_stats(circuit)

    assert result.changed is True
    assert operation_names(circuit) == ["x_matrix", "x_matrix"]
    assert all(name == "U" for name in operation_names(result.circuit))
    np.testing.assert_allclose(
        result.circuit.to_matrix(), circuit.to_matrix(), atol=1e-10
    )
    assert (stats.hits, stats.misses, stats.inserts) == (1, 1, 1)
    assert copy.copy(stats) == stats


def test_decompose_unitaries_rejects_missing_matrix() -> None:
    circuit = Circuit(1)
    circuit.append_unitary_gate(UnitaryGate("undefined", 1), [0])

    with pytest.raises(CompilerTransformError, match="no matrix representation"):
        decompose_unitaries(circuit)


def test_decompose_mc_gates_returns_standard_operations_and_stats() -> None:
    circuit = Circuit(4)
    gate = MCGate(3, StandardGate.X)
    circuit.append_mc_gate(gate, [0, 1, 2, 3])
    circuit.append_mc_gate(gate, [0, 1, 2, 3])

    result, stats = decompose_mc_gates_with_rule_stats(circuit)

    assert result.changed is True
    assert len(circuit.operations) == 2
    assert all(
        not operation.instruction.instruction.is_mcgate
        for operation in result.circuit.operations
    )
    np.testing.assert_allclose(
        result.circuit.to_matrix(), circuit.to_matrix(), atol=1e-10
    )
    assert stats.hits >= 1
    assert stats.misses >= 1
    assert stats.inserts >= 1


def test_decompose_mc_gates_default_does_not_add_qubits() -> None:
    circuit = Circuit(4)
    circuit.append_mc_gate(MCGate(3, StandardGate.X), [0, 1, 2, 3])

    result = decompose_mc_gates(circuit)

    assert result.changed is True
    assert result.circuit.num_qubits == circuit.num_qubits


def test_decompose_mc_gates_for_device_enforces_capacity() -> None:
    circuit = Circuit(4)
    circuit.append_mc_gate(MCGate(3, StandardGate.X), [0, 1, 2, 3])

    result = decompose_mc_gates_for_device(circuit, Device.line("line4", 4))
    assert result.changed is True
    assert result.circuit.num_qubits == 4

    with pytest.raises(CompilerConfigError, match="capacity exceeded"):
        decompose_mc_gates_for_device(circuit, Device.line("line3", 3))


def test_numeric_unitary_module_is_registered() -> None:
    assert "cqlib._native.compile.transform.decompose.unitary" in sys.modules
    assert OneQubitUnitaryDecomposition.__module__.endswith("decompose.unitary")
    assert TwoQubitUnitarySynthesisResult.__module__.endswith("decompose.unitary")
    assert KakDecomposition.__module__.endswith("decompose.unitary")


def test_numeric_1q_synthesis_reconstructs_matrix_and_phase() -> None:
    source = (
        np.exp(0.37j)
        * np.array(
            [[1, 1], [1, -1]],
            dtype=np.complex128,
        )
        / np.sqrt(2)
    )

    decomposition = synthesize_numeric_1q_unitary(source)
    circuit = Circuit(1)
    circuit.u(0, decomposition.theta, decomposition.phi, decomposition.lambda_)
    reconstructed = np.exp(1j * decomposition.global_phase) * circuit.to_matrix()

    np.testing.assert_allclose(reconstructed, source, atol=1e-10)
    assert copy.copy(decomposition) == decomposition


@pytest.mark.parametrize(
    "basis",
    [
        TwoQubitUnitaryDecomposeBasis.pauli_rotations(),
        TwoQubitUnitaryDecomposeBasis.cx(),
        TwoQubitUnitaryDecomposeBasis.cy(),
        TwoQubitUnitaryDecomposeBasis.cz(),
        TwoQubitUnitaryDecomposeBasis.rzz(),
    ],
)
def test_numeric_2q_synthesis_reconstructs_matrix(basis) -> None:
    source = np.array(
        [
            [1, 0, 0, 0],
            [0, 1, 0, 0],
            [0, 0, 0, 1],
            [0, 0, 1, 0],
        ],
        dtype=np.complex128,
    )

    synthesis = synthesize_numeric_2q_unitary(source, 0, Qubit(1), basis)
    expected = Circuit(2)
    expected.append_unitary_gate(UnitaryGate("source", 2).with_matrix(source), [0, 1])
    circuit = Circuit.from_operations([Qubit(0), Qubit(1)], synthesis.operations)
    circuit.set_global_phase(synthesis.global_phase)

    np.testing.assert_allclose(circuit.to_matrix(), expected.to_matrix(), atol=1e-8)
    assert synthesis.operations
    assert copy.deepcopy(synthesis).global_phase == synthesis.global_phase


def test_kak_decomposition_reconstructs_matrix_and_returns_owned_arrays() -> None:
    source = np.array(
        [
            [1, 0, 0, 0],
            [0, 0, 1, 0],
            [0, 1, 0, 0],
            [0, 0, 0, 1],
        ],
        dtype=np.complex128,
    )

    decomposition = kak_decompose(source)
    identity = np.eye(4, dtype=np.complex128)
    x = np.array([[0, 1], [1, 0]], dtype=np.complex128)
    y = np.array([[0, -1j], [1j, 0]], dtype=np.complex128)
    z = np.array([[1, 0], [0, -1]], dtype=np.complex128)
    xx, yy, zz = np.kron(x, x), np.kron(y, y), np.kron(z, z)
    cartan = (
        np.cos(decomposition.a) * identity + 1j * np.sin(decomposition.a) * xx
    ) @ (np.cos(decomposition.b) * identity + 1j * np.sin(decomposition.b) * yy)
    cartan = cartan @ (
        np.cos(decomposition.c) * identity + 1j * np.sin(decomposition.c) * zz
    )
    reconstructed = (
        np.exp(1j * decomposition.global_phase)
        * np.kron(decomposition.k1l, decomposition.k1r)
        @ cartan
        @ np.kron(decomposition.k2l, decomposition.k2r)
    )

    np.testing.assert_allclose(reconstructed, source, atol=1e-7)
    local = decomposition.k1l
    expected = local.copy()
    local[0, 0] += 1
    np.testing.assert_allclose(decomposition.k1l, expected)


def test_numeric_synthesis_rejects_invalid_inputs() -> None:
    with pytest.raises(CompilerConfigError, match="2x2"):
        synthesize_numeric_1q_unitary(np.eye(3))
    with pytest.raises(CompilerConfigError, match="distinct qubits"):
        synthesize_numeric_2q_unitary(np.eye(4), 0, 0)
    with pytest.raises(TypeError, match="two-dimensional"):
        kak_decompose([1, 0, 0, 1])


def circuit_from_value_operations(num_qubits: int, operations) -> Circuit:
    return Circuit.from_operations(
        [Qubit(index) for index in range(num_qubits)],
        operations,
    )


def mcx_circuit(num_qubits: int, controls: list[int], target: int) -> Circuit:
    circuit = Circuit(num_qubits)
    circuit.append_mc_gate(MCGate(len(controls), StandardGate.X), [*controls, target])
    return circuit


def test_mc_gate_module_exports_every_native_primitive() -> None:
    assert "cqlib._native.compile.transform.decompose.mc_gate" in sys.modules
    assert mc.Su2RotationAxis.__module__.endswith("decompose.mc_gate")
    assert len(mc.__all__) == 39
    assert all(hasattr(mc, name) for name in mc.__all__)
    assert len(set(mc.__all__)) == len(mc.__all__)


@pytest.mark.parametrize(
    "synthesize,num_qubits,clean_ancillas",
    [
        (lambda: mc.decompose_mcx_no_aux([0, 1, 2], 3), 4, 0),
        (lambda: mc.decompose_mcx_n_clean([0, 1, 2], 3, [4]), 5, 1),
        (lambda: mc.decompose_mcx_n_dirty([0, 1, 2], 3, [4]), 5, 0),
        (lambda: mc.decompose_mcx_1_clean_b95([0, 1, 2], 3, 4), 5, 1),
        (lambda: mc.decompose_mcx_1_clean_kg24([0, 1, 2], 3, 4), 5, 1),
        (lambda: mc.decompose_mcx_1_dirty([0, 1, 2], 3, 4), 5, 0),
        (lambda: mc.decompose_mcx_2_clean([0, 1, 2], 3, [4, 5]), 6, 2),
        (lambda: mc.decompose_mcx_2_dirty([0, 1, 2], 3, [4, 5]), 6, 0),
    ],
)
def test_exact_mcx_primitives_match_mcgate_matrix(
    synthesize, num_qubits, clean_ancillas
) -> None:
    result = circuit_from_value_operations(num_qubits, synthesize())
    expected = mcx_circuit(num_qubits, [0, 1, 2], 3)
    actual_matrix = result.to_matrix()
    expected_matrix = expected.to_matrix()
    if clean_ancillas:
        logical_dimension = 2 ** (num_qubits - clean_ancillas)
        np.testing.assert_allclose(
            actual_matrix[:, :logical_dimension],
            expected_matrix[:, :logical_dimension],
            atol=1e-10,
        )
        np.testing.assert_allclose(
            actual_matrix[logical_dimension:, :logical_dimension],
            0.0,
            atol=1e-10,
        )
    else:
        np.testing.assert_allclose(actual_matrix, expected_matrix, atol=1e-10)


def test_mcx_small_emits_standard_gate_and_validates_inputs() -> None:
    operations = mc.decompose_mcx_small([0, 1], 2)
    assert [operation.instruction.instruction.name for operation in operations] == [
        "CCX"
    ]

    with pytest.raises(CompilerTransformError, match="duplicate"):
        mc.decompose_mcx_no_aux([0, 0, 1], 2)
    with pytest.raises(ValueError, match="exactly 2"):
        mc.decompose_mcx_2_clean([0, 1, 2], 3, [4])


def test_mc_su2_and_rotation_primitives_preserve_semantics() -> None:
    theta = 0.37
    operations = mc.decompose_mc_su2_no_aux(mc.Su2RotationAxis.y(), theta, [0, 1], 2)
    result = circuit_from_value_operations(3, operations)
    expected = Circuit(3)
    expected.append_mc_gate(MCGate(2, StandardGate.RY(theta)), [0, 1, 2])
    np.testing.assert_allclose(result.to_matrix(), expected.to_matrix(), atol=1e-10)

    rotation = mc.decompose_rotation_no_aux(StandardGate.RY, theta, [0, 1], 2)
    rotation_result = circuit_from_value_operations(3, rotation)
    np.testing.assert_allclose(
        result.to_matrix(), rotation_result.to_matrix(), atol=1e-10
    )

    assert mc.Su2RotationAxis.x() != mc.Su2RotationAxis.z()
    assert copy.copy(mc.Su2RotationAxis.y()) == mc.Su2RotationAxis.y()


def test_representative_composite_primitives_return_equivalent_circuits() -> None:
    cases = [
        (mc.decompose_pauli_no_aux(StandardGate.Z, [0, 1], 2), 3),
        (mc.decompose_mc_rzz_no_aux(0.23, [0], 1, 2), 3),
        (mc.decompose_pauli_rotation_no_aux(StandardGate.RXX, 0.19, [0], 1, 2), 3),
        (mc.decompose_phase_no_aux(StandardGate.T, None, [0, 1], 2), 3),
        (mc.decompose_qcis_no_aux(StandardGate.X2P, [], [0, 1], 2), 3),
        (mc.decompose_hadamard_no_aux([0, 1], 2), 3),
        (mc.decompose_swap_no_aux([0], 1, 2), 3),
        (mc.decompose_fsim_no_aux([0.17, -0.11], [0], 1, 2), 3),
        (mc.decompose_unitary_no_aux(0.2, -0.3, 0.4, [0, 1], 2), 3),
    ]

    for operations, width in cases:
        circuit = circuit_from_value_operations(width, operations)
        assert circuit.operations
        assert all(
            not operation.instruction.instruction.is_mcgate
            for operation in circuit.operations
        )


def test_clean_composite_primitives_validate_ancilla_contracts() -> None:
    with pytest.raises(CompilerTransformError, match="requires"):
        mc.decompose_swap_n_clean([0, 1, 2], 3, 4, [])
    with pytest.raises(CompilerTransformError, match="distinct|duplicate"):
        mc.decompose_mc_rzz_n_clean(0.2, [0, 1], 2, 3, [0])
    with pytest.raises(ParameterError, match="finite"):
        mc.decompose_unitary_no_aux(float("inf"), 0.0, 0.0, [0], 1)
