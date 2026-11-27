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

from itertools import permutations

import numpy as np
import pytest

from cqlib.circuit import Circuit, Parameter
from cqlib.circuit.gates import CircuitGate


def _embed_operation_matrix(matrix, targets, width):
    """Independent basis-index embedding: first gate argument is the MSB."""
    result = np.zeros((2**width, 2**width), dtype=complex)
    for column in range(2**width):
        local_column = 0
        for target in targets:
            local_column = 2 * local_column + ((column >> target) & 1)
        for local_row in range(2 ** len(targets)):
            row = column
            for position, target in enumerate(targets):
                bit = (local_row >> (len(targets) - 1 - position)) & 1
                row = (row & ~(1 << target)) | (bit << target)
            result[row, column] = matrix[local_row, local_column]
    return result


class TestCircuitGateOperationMatrix:
    @staticmethod
    def assert_operations(circuit, expected):
        # Cover snapshots returned by both collection and indexing APIs.
        for operation in (circuit.operations[0], circuit[0]):
            np.testing.assert_allclose(operation.matrix(), expected, rtol=0, atol=1e-12)

    @pytest.mark.parametrize("order", [[0, 1], [1, 0], [7, 2]])
    def test_positional_matrix(self, order):
        body = Circuit(order)
        body.x(order[0])
        circuit = Circuit(2)
        circuit.append_circuit_gate(body.to_gate("first_x"), [0, 1])
        self.assert_operations(circuit, np.kron([[0, 1], [1, 0]], np.eye(2)))

    def test_bound_parameter_and_repeated_application(self):
        body = Circuit([7])
        body.rx(7, Parameter("theta"))
        gate = body.to_gate("rx")
        for angle in (0.37, -0.61, 0.37):
            circuit = Circuit(1)
            circuit.append_circuit_gate(gate, [0], [angle])
            c, s = np.cos(angle / 2), -1j * np.sin(angle / 2)
            self.assert_operations(circuit, np.array([[c, s], [s, c]]))
        assert gate.used_symbols == ["theta"]

    @pytest.mark.parametrize("order", list(permutations([7, 2, 5])))
    @pytest.mark.parametrize("targets", [[3, 0, 2], [2, 3, 0]])
    def test_three_qubit_permutations_and_spectator(self, order, targets):
        body = Circuit(list(order))
        body.h(order[0])
        body.rz(order[1], 0.29)
        body.cx(order[0], order[2])
        body.set_global_phase(0.17)
        expected = np.zeros((8, 8), dtype=complex)
        for column in range(8):
            first, middle, last = column >> 2, (column >> 1) & 1, column & 1
            for output_first in (0, 1):
                row = (output_first << 2) | (middle << 1) | (last ^ output_first)
                expected[row, column] = (
                    (-1) ** (first * output_first)
                    / np.sqrt(2)
                    * np.exp(1j * (0.17 + (2 * middle - 1) * 0.29 / 2))
                )
        circuit = Circuit(4)
        circuit.append_circuit_gate(body.to_gate("three"), targets)
        self.assert_operations(circuit, expected)
        full = _embed_operation_matrix(expected, targets, 4)
        np.testing.assert_allclose(circuit.to_matrix(), full, rtol=0, atol=1e-12)
        np.testing.assert_allclose(
            circuit.to_symbolic_matrix().evaluate(), full, rtol=0, atol=1e-12
        )
        np.testing.assert_allclose(
            circuit.decompose().to_matrix(), full, rtol=0, atol=1e-12
        )

    @pytest.mark.parametrize("nested", [False, True])
    @pytest.mark.parametrize("inverse", [False, True])
    def test_signature_phase_and_inverse(self, nested, inverse):
        theta, phi = Parameter("theta"), Parameter("phi")
        body = Circuit([7, 2])
        body.rx(7, theta)
        body.rz(2, phi)
        body.cx(7, 2)
        body.set_global_phase(theta + phi / 2)
        gate = CircuitGate.with_signature(
            "declared", body.to_gate("definition").circuit, ["unused", "phi", "theta"]
        )
        arguments = [12.0, -0.21, 0.37]
        if nested:
            middle = Circuit([5, 1])
            middle.append_circuit_gate(gate, [5, 1], [0, phi, theta])
            gate = CircuitGate.with_signature(
                "nested", middle.to_gate("middle").circuit, ["unused", "phi", "theta"]
            )
        circuit = Circuit(2)
        circuit.append_circuit_gate(gate, [1, 0], arguments)
        if inverse:
            circuit = circuit.inverse()
        c, s = np.cos(0.37 / 2), -1j * np.sin(0.37 / 2)
        rx = np.array([[c, s], [s, c]])
        rz = np.diag(np.exp(1j * np.array([0.21, -0.21]) / 2))
        cx = np.eye(4)[[0, 1, 3, 2]]
        expected = np.exp(1j * (0.37 - 0.21 / 2)) * cx @ np.kron(rx, rz)
        if inverse:
            expected = expected.conj().T
        self.assert_operations(circuit, expected)
        np.testing.assert_allclose(circuit.to_matrix(), expected, rtol=0, atol=1e-12)
        np.testing.assert_allclose(
            circuit.to_symbolic_matrix().evaluate(), expected, rtol=0, atol=1e-12
        )

    def test_zero_qubit_phase(self):
        body = Circuit(0)
        body.set_global_phase(Parameter("theta"))
        circuit = Circuit(0)
        circuit.append_circuit_gate(body.to_gate("phase"), [], [0.37])
        expected = np.array([[np.exp(0.37j)]])
        self.assert_operations(circuit, expected)
        np.testing.assert_allclose(circuit.to_matrix(), expected, rtol=0, atol=1e-12)

    def test_unresolved_arguments_keep_existing_error_behavior(self):
        body = Circuit(1)
        body.rx(0, Parameter("theta"))
        circuit = Circuit(1)
        circuit.append_circuit_gate(body.to_gate("unresolved"), [0], [Parameter("x")])
        with pytest.raises(ValueError):
            circuit.operations[0].matrix()
        with pytest.raises(ValueError):
            circuit[0].matrix()


class TestCircuitGateStorageOrder:
    @pytest.mark.parametrize("order", [[7, 2], [1, 0], [2, 7], [0, 1]])
    @pytest.mark.parametrize("targets", [[0, 1], [1, 0]])
    def test_matrix_uses_internal_storage_order(self, order, targets):
        inner = Circuit(order)
        inner.x(order[0])
        outer = Circuit(2)
        outer.append_circuit_gate(inner.to_gate("StoredOrderX"), targets)
        expected = Circuit(2)
        expected.x(targets[0])
        for output_order in (None, [1, 0]):
            reference = expected.to_matrix(output_order)
            np.testing.assert_allclose(
                outer.to_matrix(output_order), reference, rtol=0, atol=1e-10
            )
            np.testing.assert_allclose(
                outer.to_symbolic_matrix(output_order).evaluate(),
                reference,
                rtol=0,
                atol=1e-10,
            )
            np.testing.assert_allclose(
                outer.decompose().to_matrix(output_order), reference, rtol=0, atol=1e-10
            )
        assert outer.to_matrix()[1 << targets[0], 0] == 1

    @pytest.mark.parametrize("middle_order", [None, [0, 1], [5, 1]])
    @pytest.mark.parametrize("inverse", [False, True])
    def test_nested_parameterized_matrix_and_inverse(self, middle_order, inverse):
        theta = Parameter("theta")
        body = Circuit([7, 2])
        body.rx(7, theta)
        body.cx(7, 2)
        if middle_order is not None:
            gate = body.to_gate("Inner")
            body = Circuit(middle_order)
            body.append_circuit_gate(gate, middle_order, [theta])
        outer = Circuit(2)
        outer.append_circuit_gate(body.to_gate("Body"), [0, 1], [Parameter("x")])
        if inverse:
            outer = outer.inverse()
        symbolic = outer.to_symbolic_matrix()
        for angle in (0.37, -0.61):
            expected = Circuit(2)
            expected.rx(0, angle)
            expected.cx(0, 1)
            if inverse:
                expected = expected.inverse()
            reference = expected.to_matrix()
            bound = outer.assign_parameters({"x": angle})
            np.testing.assert_allclose(bound.to_matrix(), reference, rtol=0, atol=1e-10)
            np.testing.assert_allclose(
                symbolic.evaluate({"x": angle}), reference, rtol=0, atol=1e-10
            )
            np.testing.assert_allclose(
                bound.decompose().to_matrix(), reference, rtol=0, atol=1e-10
            )


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


class TestCircuitGateControl:
    """Tests for CircuitGate.control(n) producing a controlled UnitaryGate."""

    def test_single_and_double_controlled_bell_columns_and_superposition(self):
        from cqlib.circuit import CircuitError, UnitaryGate
        from cqlib.qis import DensityMatrix, Statevector

        bell = Circuit([7, 2])
        bell.h(7)
        bell.cx(7, 2)
        bell.set_global_phase(0.31)
        bell_gate = bell.to_gate("Bell")

        phase = np.exp(1j * 0.31)
        s2 = 1.0 / np.sqrt(2.0)
        bell_local = (
            phase
            * s2
            * np.array(
                [
                    [1, 0, 1, 0],
                    [0, 1, 0, 1],
                    [0, 1, 0, -1],
                    [1, 0, -1, 0],
                ],
                dtype=complex,
            )
        )

        c1_bell = bell_gate.control(1)
        c2_bell = bell_gate.control(2)
        assert isinstance(c1_bell, UnitaryGate)
        assert isinstance(c2_bell, UnitaryGate)
        assert c1_bell.label == "ctl_1_Bell"
        assert c2_bell.label == "ctl_2_Bell"
        assert c1_bell.num_qubits == 3 and c1_bell.num_params == 0
        assert c2_bell.num_qubits == 4 and c2_bell.num_params == 0
        assert c1_bell.matrix().shape == (8, 8)
        assert c2_bell.matrix().shape == (16, 16)

        expected_c1 = np.eye(8, dtype=complex)
        expected_c1[4:, 4:] = bell_local
        np.testing.assert_allclose(c1_bell.matrix(), expected_c1, rtol=0, atol=1e-12)

        expected_c2 = np.eye(16, dtype=complex)
        expected_c2[12:, 12:] = bell_local
        np.testing.assert_allclose(c2_bell.matrix(), expected_c2, rtol=0, atol=1e-12)

        # Out-of-order, non-contiguous placement [3, 1, 4, 0] in a 5-qubit circuit.
        targets = [3, 1, 4, 0]
        circuit = Circuit(5)
        circuit.append_unitary_gate(c2_bell, targets)
        expected_full = _embed_operation_matrix(expected_c2, targets, 5)
        actual_full = circuit.to_matrix()
        for col in range(32):
            np.testing.assert_allclose(
                actual_full[:, col], expected_full[:, col], rtol=0, atol=1e-12
            )

        # Superposition across inactive (|01>) and active (|11>) control branches
        # on controls=(q2, q0), targets=(q3, q1), with initial target |t0=1, t1=0>.
        sup = Circuit(4)
        sup.x(0)
        sup.h(2)
        sup.x(3)
        sup.append_unitary_gate(c2_bell, [2, 0, 3, 1])

        expected_amps = np.zeros(16, dtype=complex)
        expected_amps[0b1001] = s2
        expected_amps[0b0101] = 0.5 * phase
        expected_amps[0b1111] = -0.5 * phase

        sv = Statevector.from_circuit(sup)
        np.testing.assert_allclose(sv.data, expected_amps, rtol=0, atol=1e-12)

        dm = DensityMatrix.from_circuit(sup)
        expected_rho = np.outer(expected_amps, expected_amps.conj())
        np.testing.assert_allclose(
            np.asarray(dm.data).reshape(16, 16), expected_rho, rtol=0, atol=1e-12
        )

        # Matrix-only UnitaryGate currently rejects Circuit.inverse().
        with pytest.raises(CircuitError):
            sup.inverse()

    def test_parameterized_signature_swaps_internal_prefix_and_compilation(self):
        from cqlib.compile import compile as compile_circuit
        from cqlib.qis import Statevector

        alpha = Parameter("alpha")
        beta = Parameter("__cqlib_internal_sub_beta")
        sub = Circuit([9, 3])
        sub.rx(9, alpha + beta)
        sub.cx(9, 3)
        sub.set_global_phase(alpha * 0.5 - beta)

        gate = CircuitGate.with_signature(
            "ParamBell",
            sub.to_gate("raw").circuit,
            ["unused", "__cqlib_internal_sub_beta", "alpha"],
        )
        ctrl = gate.control(1)
        assert ctrl.num_qubits == 3
        assert ctrl.num_params == 3
        assert ctrl.matrix_params == ["unused", "__cqlib_internal_sub_beta", "alpha"]

        # Outer call swaps and combines formal/actual symbols, including internal-prefix names.
        outer = Circuit(3)
        outer.h(2)
        outer.append_unitary_gate(
            ctrl,
            [2, 0, 1],
            [alpha + 3.0, alpha, beta * 2.0],
        )

        sym_mat = outer.to_symbolic_matrix()
        bindings = {"alpha": 0.41, "__cqlib_internal_sub_beta": -0.27}
        bound = outer.assign_parameters(bindings)
        expected_mat = bound.to_matrix()

        np.testing.assert_allclose(
            sym_mat.evaluate(bindings), expected_mat, rtol=0, atol=1e-12
        )
        sv = Statevector.from_circuit(bound)
        np.testing.assert_allclose(sv.data, expected_mat[:, 0], rtol=0, atol=1e-12)

        # Compilation synthesizes the 3-qubit bound UnitaryGate (including unused slot bound).
        compiled = compile_circuit(bound).circuit
        np.testing.assert_allclose(
            compiled.to_matrix(), expected_mat, rtol=0, atol=1e-9
        )

    def test_control_errors_and_ir_boundaries(self):
        from cqlib.circuit import CircuitError
        from cqlib.ir import qasm2, qasm3, qcis

        one_q = Circuit(1)
        one_q.h(0)
        gate = one_q.to_gate("OneH")

        with pytest.raises(CircuitError):
            gate.control(0)
        with pytest.raises(CircuitError):
            gate.control(12)
        with pytest.raises(CircuitError):
            gate.control(30)
        with pytest.raises(OverflowError):
            gate.control(-1)

        non_unitary = Circuit(1)
        non_unitary.measure(0)
        with pytest.raises(CircuitError):
            non_unitary.to_gate("Meas").control(1)

        bell = Circuit(2)
        bell.h(0)
        bell.cx(0, 1)
        cc_bell = bell.to_gate("Bell").control(2)
        c = Circuit(4)
        c.append_unitary_gate(cc_bell, [0, 1, 2, 3])

        qasm2_text = qasm2.dumps(c)
        assert "opaque ctl_2_Bell q0,q1,q2,q3;" in qasm2_text
        with pytest.raises(ValueError):
            qasm3.dumps(c)
        with pytest.raises(ValueError):
            qcis.dumps(c)

        # Symbolic controlled gates enforce the tighter 8-qubit symbolic cap.
        sym_one = Circuit(1)
        sym_one.rx(0, Parameter("theta"))
        with pytest.raises(CircuitError):
            sym_one.to_gate("SymOne").control(8)

        # Classical if_else reports consistent control-flow error on non-parametric gates.
        from cqlib.circuit import ClassicalExpr

        cond = Circuit(1)
        cond.if_else(
            ClassicalExpr.bool_literal(False),
            lambda body: body.x(0),
            lambda body: body.z(0),
        )
        with pytest.raises(CircuitError, match="control-flow"):
            cond.to_gate("CondNum").control(1)

    @pytest.mark.parametrize("param_name", ["e", "pi", "π"])
    def test_control_rejects_constant_named_formal_parameters(self, param_name):
        from cqlib.circuit import CircuitError

        sub = Circuit(1)
        sub.rx(0, Parameter(param_name))
        gate = CircuitGate.with_signature(
            "ConstNamed", sub.to_gate("raw").circuit, [param_name]
        )
        with pytest.raises(CircuitError, match="built-in mathematical constant"):
            gate.control(1)
