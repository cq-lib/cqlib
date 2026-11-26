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

"""
Tests for Instruction / ValueInstruction payload accessors.

Test coverage:
- Predicate/accessor symmetry for every instruction variant
- MCGate / UnitaryGate / CircuitGate payload contents
- ClassicalDataOp payload accessors (kind, target, value, result)
- Parameter ownership: instruction-level gates carry no bound parameters
- Non-matching variants return None
"""

import pytest

from cqlib.circuit import (
    Circuit,
    ClassicalDataOp,
    ClassicalControlOp,
    Instruction,
    Parameter,
    Qubit,
    ValueInstruction,
    ValueOperation,
)
from cqlib.circuit.gates import Directive, MCGate, RX, StandardGate, UnitaryGate

# instruction_type -> (predicate property, payload accessor property)
VARIANTS = {
    "standard": ("is_standard", "standard_gate"),
    "mcgate": ("is_mcgate", "mc_gate"),
    "unitary": ("is_unitary", "unitary_gate"),
    "circuit": ("is_circuit_gate", "circuit_gate"),
    "directive": ("is_directive", "directive"),
    "classical_data": ("is_classical_data", "classical_data"),
}


def _circuit_gate():
    sub = Circuit(1)
    sub.h(0)
    return sub.to_gate("h_gate")


def _instructions():
    """One Instruction per directly constructible variant."""
    return [
        ("standard", Instruction.from_standard_gate(StandardGate.H)),
        ("mcgate", Instruction.from_mc_gate(MCGate(2, StandardGate.X))),
        ("unitary", Instruction.from_unitary_gate(UnitaryGate("oracle", 1))),
        ("circuit", Instruction.from_circuit_gate(_circuit_gate())),
        ("directive", Instruction.from_directive(Directive.barrier())),
    ]


class TestInstructionAccessorSymmetry:
    """Every predicate must pair with a payload accessor that agrees with it."""

    @pytest.mark.parametrize(("kind", "instruction"), _instructions())
    def test_accessors_agree_with_predicates(self, kind, instruction) -> None:
        assert instruction.instruction_type == kind
        for other_kind, (predicate, accessor) in VARIANTS.items():
            matching = other_kind == kind
            assert getattr(instruction, predicate) is matching
            assert (getattr(instruction, accessor) is not None) is matching

    def test_delay_has_no_payload(self) -> None:
        instruction = Instruction.delay()
        assert instruction.is_delay
        for predicate, accessor in VARIANTS.values():
            assert getattr(instruction, predicate) is False
            assert getattr(instruction, accessor) is None


class TestValueInstructionAccessorSymmetry:
    @pytest.mark.parametrize(("kind", "instruction"), _instructions())
    def test_accessors_agree_with_predicates(self, kind, instruction) -> None:
        value_instruction = ValueInstruction.from_instruction(instruction)
        assert value_instruction.instruction_type == kind
        for other_kind, (predicate, accessor) in VARIANTS.items():
            matching = other_kind == kind
            assert getattr(value_instruction, predicate) is matching
            assert (getattr(value_instruction, accessor) is not None) is matching

    def test_classical_control_has_no_gate_payload(self) -> None:
        value_instruction = ValueInstruction.from_classical_control(
            ClassicalControlOp.break_()
        )
        assert value_instruction.is_classical_control
        assert value_instruction.classical_control is not None
        for predicate, accessor in VARIANTS.values():
            assert getattr(value_instruction, predicate) is False
            assert getattr(value_instruction, accessor) is None


class TestMcGatePayload:
    def test_payload_contents(self) -> None:
        instruction = Instruction.from_mc_gate(MCGate(2, StandardGate.RZ))
        gate = instruction.mc_gate
        assert gate is not None
        assert gate.num_ctrl_qubits == 2
        assert gate.base_gate == StandardGate.RZ
        assert gate.num_qubits == 3

    def test_payload_carries_no_bound_parameters(self) -> None:
        """Parameters belong to the operation, not to the instruction."""
        theta = Parameter("theta")
        operation = ValueOperation.from_mc_gate(
            MCGate(1, RX(theta)), [Qubit(0), Qubit(1)]
        )
        instruction = operation.instruction
        assert instruction.is_mcgate
        gate = instruction.mc_gate
        assert gate is not None
        assert gate.params == []
        assert operation.params == [theta]


class TestUnitaryGatePayload:
    def test_payload_contents(self) -> None:
        instruction = Instruction.from_unitary_gate(UnitaryGate("oracle", 2))
        gate = instruction.unitary_gate
        assert gate is not None
        assert gate.label == "oracle"
        assert gate.num_qubits == 2


class TestCircuitGatePayload:
    def test_payload_contents(self) -> None:
        instruction = Instruction.from_circuit_gate(_circuit_gate())
        gate = instruction.circuit_gate
        assert gate is not None
        assert gate.name == "h_gate"
        assert gate.num_qubits == 1


class TestDirectivePayload:
    def test_payload_contents(self) -> None:
        instruction = Instruction.from_directive(Directive.measure())
        directive = instruction.directive
        assert directive is not None
        assert str(directive) == "measure"


class TestClassicalDataPayload:
    def test_value_instruction_payload(self) -> None:
        circuit = Circuit(1)
        circuit.measure(0)
        instruction = circuit.operations[0].instruction
        assert instruction.is_classical_data
        data = instruction.classical_data
        assert isinstance(data, ClassicalDataOp)
        assert data.kind == "measure_bit"
        assert data.result is not None
        assert data.target is None
        assert data.value is None

    def test_storage_instruction_payload(self) -> None:
        circuit = Circuit(2)
        circuit.measure_bits([0, 1])
        value_instruction = circuit.operations[0].instruction
        instruction = value_instruction.instruction
        assert instruction is not None
        assert instruction.is_classical_data
        data = instruction.classical_data
        assert data is not None
        assert data.kind == "measure_bits"
        assert data.result is not None

    def test_store_payload(self) -> None:
        circuit = Circuit(1)
        measurement = circuit.measure(0)
        target = circuit.var(measurement.ty)
        circuit.store(target, measurement.expr())
        instruction = circuit.operations[1].instruction
        assert instruction.is_classical_data
        data = instruction.classical_data
        assert data is not None
        assert data.kind == "store"
        assert data.target is not None
        assert data.value is not None
        assert data.result is None


class TestClassicalDataSubmodule:
    def test_runtime_submodule_matches_stub(self) -> None:
        import cqlib.circuit.classical_data as submodule

        assert submodule.ClassicalDataOp is ClassicalDataOp
        assert submodule.__all__ == ["ClassicalDataOp"]
