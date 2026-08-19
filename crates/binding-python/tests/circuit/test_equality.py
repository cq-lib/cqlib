# This code is part of Cqlib.
#
# (C) Copyright China Telecom Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
#
# Any modifications or derivative works of this code must retain this
# copyright notice, and modified files need to carry a notice indicating
# that they have been altered from the originals.

"""Value-semantics equality tests for circuit, operation, and gate wrappers."""

import copy

import pytest

from cqlib import Circuit, Qubit
from cqlib.circuit import (
    Directive,
    FrozenCircuit,
    Instruction,
    StandardGate,
    ValueInstruction,
    ValueOperation,
)


def make_bell_circuit():
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)
    return circuit


class TestCircuitEquality:
    """Circuits compare by structure, not by process-local identity."""

    def test_same_content_circuits_are_equal(self):
        assert make_bell_circuit() == make_bell_circuit()

    def test_different_content_circuits_are_unequal(self):
        different = make_bell_circuit()
        different.x(0)
        assert make_bell_circuit() != different

    def test_cross_type_comparison_returns_false(self):
        assert (make_bell_circuit() == 42) is False
        assert (make_bell_circuit() == "circuit") is False

    def test_circuits_are_unhashable(self):
        with pytest.raises(TypeError):
            hash(make_bell_circuit())

    def test_copies_compare_equal(self):
        circuit = make_bell_circuit()
        assert copy.copy(circuit) == circuit
        assert copy.deepcopy(circuit) == circuit


class TestValueOperationEquality:
    """Resolved operations compare by instruction, qubits, params, label."""

    def make_rx_operation(self, theta=0.5, label=None):
        circuit = Circuit(1)
        circuit.rx(0, theta)
        operation = circuit[0]
        if label is not None:
            return ValueOperation(
                operation.instruction, operation.qubits, operation.params, label
            )
        return operation

    def test_same_structure_operations_are_equal(self):
        assert self.make_rx_operation() == self.make_rx_operation()

    def test_different_parameter_operations_are_unequal(self):
        assert self.make_rx_operation(0.5) != self.make_rx_operation(1.5)

    def test_different_label_operations_are_unequal(self):
        assert self.make_rx_operation(label="a") != self.make_rx_operation(label="b")

    def test_direct_construction_matches_resolved_operation(self):
        circuit = Circuit(1)
        circuit.h(0)
        direct = ValueOperation.from_standard_gate(StandardGate.H(), [Qubit(0)])
        assert direct == circuit[0]

    def test_cross_type_comparison_returns_false(self):
        assert (self.make_rx_operation() == 42) is False

    def test_operations_are_unhashable(self):
        with pytest.raises(TypeError):
            hash(self.make_rx_operation())

    def test_copies_compare_equal(self):
        operation = self.make_rx_operation()
        assert copy.copy(operation) == operation
        assert copy.deepcopy(operation) == operation


class TestInstructionEquality:
    """Instructions compare by wrapped definition."""

    def test_same_standard_gates_are_equal(self):
        assert Instruction.from_standard_gate(
            StandardGate.H()
        ) == Instruction.from_standard_gate(StandardGate.H())
        assert Instruction.from_standard_gate(
            StandardGate.H()
        ) != Instruction.from_standard_gate(StandardGate.X())

    def test_same_directives_are_equal(self):
        assert Directive.barrier() == Directive.barrier()
        assert Directive.barrier() != Directive.measure()
        assert Directive.measure() != Directive.reset()

    def test_directives_are_hashable_and_deduplicate(self):
        assert len({Directive.barrier(), Directive.barrier(), Directive.measure()}) == 2
        assert hash(Directive.barrier()) == hash(Directive.barrier())

    def test_directive_cross_type_comparison_returns_false(self):
        assert (Directive.barrier() == 42) is False

    def test_instructions_are_unhashable(self):
        with pytest.raises(TypeError):
            hash(Instruction.from_standard_gate(StandardGate.H()))


class TestValueInstructionEquality:
    """Value instructions compare across plain and control-flow variants."""

    def test_same_wrapped_instructions_are_equal(self):
        def make():
            return ValueInstruction.from_instruction(
                Instruction.from_standard_gate(StandardGate.H())
            )

        assert make() == make()
        assert ValueInstruction.from_instruction(
            Instruction.from_standard_gate(StandardGate.H())
        ) != ValueInstruction.from_instruction(
            Instruction.from_standard_gate(StandardGate.X())
        )

    def test_value_instructions_are_unhashable(self):
        with pytest.raises(TypeError):
            hash(
                ValueInstruction.from_instruction(
                    Instruction.from_standard_gate(StandardGate.H())
                )
            )


class TestFrozenCircuitEquality:
    """Frozen circuits compare by defining structure; caches never participate."""

    def make_frozen(self):
        subcircuit = Circuit(1)
        subcircuit.h(0)
        return FrozenCircuit(subcircuit.qubits, subcircuit.operations)

    def test_same_content_frozen_circuits_are_equal(self):
        assert self.make_frozen() == self.make_frozen()

    def test_different_content_frozen_circuits_are_unequal(self):
        subcircuit = Circuit(1)
        subcircuit.x(0)
        different = FrozenCircuit(subcircuit.qubits, subcircuit.operations)
        assert self.make_frozen() != different

    def test_frozen_circuits_are_unhashable(self):
        with pytest.raises(TypeError):
            hash(self.make_frozen())

    def test_copies_compare_equal(self):
        frozen = self.make_frozen()
        assert copy.copy(frozen) == frozen
        assert copy.deepcopy(frozen) == frozen
