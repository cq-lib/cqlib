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
Tests for CircuitGate composite gates.

Test coverage:
- CircuitGate creation via Circuit.to_gate()
- CircuitGate application in circuits
- CircuitGate decomposition in circuits
- Parametric CircuitGate creation and usage
"""

import pytest

from cqlib.circuit import Circuit, Parameter
from cqlib.circuit.gates import CircuitGate


class TestCircuitGateCreation:
    """Test CircuitGate creation from circuits"""

    def test_create_single_qubit_gate(self):
        """Create CircuitGate from single-qubit circuit"""
        c = Circuit(1)
        c.h(0)
        gate = c.to_gate("H")
        assert isinstance(gate, CircuitGate)

    def test_create_two_qubit_gate(self):
        """Create CircuitGate from two-qubit circuit"""
        c = Circuit(2)
        c.h(0)
        c.cx(0, 1)
        gate = c.to_gate("Bell")
        assert isinstance(gate, CircuitGate)

    def test_create_multi_qubit_gate(self):
        """Create CircuitGate from multi-qubit circuit"""
        c = Circuit(3)
        c.h(0)
        c.cx(0, 1)
        c.cx(1, 2)
        gate = c.to_gate("GHZ")
        assert isinstance(gate, CircuitGate)

    def test_create_from_empty_circuit(self):
        """Create CircuitGate from empty circuit"""
        c = Circuit(2)
        gate = c.to_gate("Empty")
        assert isinstance(gate, CircuitGate)


class TestCircuitGateInCircuit:
    """Test using CircuitGate within other circuits"""

    def test_apply_circuit_gate(self):
        """Apply CircuitGate to another circuit"""
        sub = Circuit(2)
        sub.h(0)
        sub.cx(0, 1)
        bell_gate = sub.to_gate("Bell")

        main = Circuit(4)
        main.append_circuit_gate(bell_gate, [0, 1])

        assert len(main) == 1
        assert main[0].name == "Bell"

    def test_apply_multiple_instances(self):
        """Apply same CircuitGate multiple times"""
        sub = Circuit(2)
        sub.h(0)
        sub.cx(0, 1)
        bell_gate = sub.to_gate("Bell")

        main = Circuit(4)
        main.append_circuit_gate(bell_gate, [0, 1])
        main.append_circuit_gate(bell_gate, [2, 3])

        assert len(main) == 2
        # Verify both operations use the same gate name
        assert main[0].name == "Bell"
        assert main[1].name == "Bell"

    def test_apply_to_different_qubits(self):
        """Apply CircuitGate to different qubit subsets"""
        sub = Circuit(2)
        sub.h(0)
        sub.cx(0, 1)
        bell_gate = sub.to_gate("Bell")

        main = Circuit(4)
        main.append_circuit_gate(bell_gate, [1, 2])

        op = main[0]
        assert op.qubits[0].index == 1
        assert op.qubits[1].index == 2


class TestCircuitGateDecomposition:
    """Test CircuitGate decomposition behavior"""

    def test_decompose_expands_gates(self):
        """Decomposition expands CircuitGate into primitives"""
        sub = Circuit(2)
        sub.h(0)
        sub.cx(0, 1)
        bell_gate = sub.to_gate("Bell")

        main = Circuit(2)
        main.append_circuit_gate(bell_gate, [0, 1])

        decomposed = main.decompose()
        # Should expand to H and CX
        assert len(decomposed) == 2
        assert decomposed[0].name == "H"
        assert decomposed[1].name == "CX"

    def test_decompose_preserves_operation_order(self):
        """Decomposition preserves original operation order"""
        sub = Circuit(2)
        sub.x(0)
        sub.h(0)
        sub.cx(0, 1)
        gate = sub.to_gate("Ordered")

        main = Circuit(2)
        main.append_circuit_gate(gate, [0, 1])

        decomposed = main.decompose()
        names = [op.name for op in decomposed]
        assert names == ["X", "H", "CX"]

    def test_decompose_multiple_circuit_gates(self):
        """Decompose circuit with multiple CircuitGates"""
        sub = Circuit(1)
        sub.h(0)
        h_gate = sub.to_gate("H")

        main = Circuit(2)
        main.append_circuit_gate(h_gate, [0])
        main.append_circuit_gate(h_gate, [1])

        decomposed = main.decompose()
        assert len(decomposed) == 2
        assert decomposed[0].name == "H"
        assert decomposed[1].name == "H"


class TestCircuitGateParametric:
    """Test CircuitGate with parametric circuits"""

    def test_parametric_circuit_gate_creation(self):
        """Create CircuitGate from parametric circuit"""
        theta = Parameter("theta")
        c = Circuit(1)
        c.rx(0, theta)
        gate = c.to_gate("Rx")
        assert isinstance(gate, CircuitGate)

    def test_apply_parametric_circuit_gate(self):
        """Apply parametric CircuitGate to circuit"""
        theta = Parameter("theta")
        c = Circuit(1)
        c.rx(0, theta)
        rx_gate = c.to_gate("Rx")

        main = Circuit(2)
        main.append_circuit_gate(rx_gate, [0], [theta])
        main.append_circuit_gate(rx_gate, [1], [theta])

        assert len(main) == 2

    def test_multiple_parametric_gates(self):
        """CircuitGate with multiple parametric operations"""
        theta = Parameter("theta")
        phi = Parameter("phi")
        c = Circuit(1)
        c.rx(0, theta)
        c.rz(0, phi)
        gate = c.to_gate("RxRz")
        assert isinstance(gate, CircuitGate)


class TestCircuitGateNested:
    """Test nested CircuitGate usage"""

    def test_nested_circuit_gates(self):
        """Nested CircuitGate creation and usage"""
        inner = Circuit(1)
        inner.h(0)
        inner_gate = inner.to_gate("H")

        middle = Circuit(1)
        middle.append_circuit_gate(inner_gate, [0])
        middle_gate = middle.to_gate("Middle")

        outer = Circuit(1)
        outer.append_circuit_gate(middle_gate, [0])

        assert len(outer) == 1

    def test_decompose_nested_gates(self):
        """Decompose nested CircuitGates recursively"""
        inner = Circuit(1)
        inner.x(0)
        inner_gate = inner.to_gate("X")

        middle = Circuit(1)
        middle.append_circuit_gate(inner_gate, [0])
        middle_gate = middle.to_gate("Middle")

        outer = Circuit(1)
        outer.append_circuit_gate(middle_gate, [0])

        # Decompose once expands middle to inner
        decomposed_once = outer.decompose()
        assert len(decomposed_once) == 1

        # Decompose again should eventually reach primitives
        decomposed_twice = decomposed_once.decompose()
        assert len(decomposed_twice) == 1


class TestCircuitGateType:
    """Test CircuitGate type checking"""

    def test_circuit_gate_type(self):
        """CircuitGate is correct type"""
        c = Circuit(1)
        c.h(0)
        gate = c.to_gate("H")
        assert type(gate).__name__ == "CircuitGate"

    def test_circuit_gate_operation_name(self):
        """CircuitGate operation has correct name"""
        sub = Circuit(1)
        sub.h(0)
        gate = sub.to_gate("CustomH")

        main = Circuit(1)
        main.append_circuit_gate(gate, [0])

        op = main[0]
        assert op.name == "CustomH"


class TestCircuitGateEquality:
    """Test structural equality and unhashability of CircuitGate."""

    @staticmethod
    def _bell(name="Bell"):
        c = Circuit(2)
        c.h(0)
        c.cx(0, 1)
        return c.to_gate(name)

    def test_structurally_equal(self):
        """Two independently built identical gates compare equal."""
        assert self._bell() == self._bell()

    def test_self_equal(self):
        """A gate compares equal to itself."""
        g = self._bell()
        assert g == g

    def test_different_name_not_equal(self):
        """A differing name breaks equality."""
        assert self._bell("Bell") != self._bell("Other")

    def test_different_structure_not_equal(self):
        """A differing circuit definition breaks equality."""
        a = Circuit(2)
        a.h(0)
        a.cx(0, 1)

        b = Circuit(2)
        b.x(0)

        assert a.to_gate("G") != b.to_gate("G")

    def test_unhashable(self):
        """CircuitGate has no core Hash impl, so it must be unhashable."""
        g = self._bell()
        assert CircuitGate.__hash__ is None
        with pytest.raises(TypeError):
            hash(g)
        with pytest.raises(TypeError):
            {g: 1}
