# Copyright China Telecom Quantum Group 2026
# SPDX-License-Identifier: Apache-2.0

"""Supported old workflows must produce ordinary native circuits and operations."""

import copy
import importlib
import subprocess
import sys
import warnings

import numpy as np
import pytest

from cqlib import Circuit, CqlibDeprecationWarning, Parameter, Qubit, StandardGate
from cqlib.circuit import (
    CircuitError,
    ClassicalExpr,
    ClassicalType,
    Measurement,
    ValueOperation,
)
from cqlib.ir import qcis

pytestmark = pytest.mark.filterwarnings("ignore::cqlib.CqlibDeprecationWarning")


def test_old_imports_preserve_native_type_and_exception_identity():
    import cqlib
    from cqlib import InstructionData, gates
    from cqlib.circuits import Circuit as OldCircuit
    from cqlib.circuits import Instruction, Parameter as OldParameter, Qubit as OldQubit
    from cqlib.exceptions import CqlibError

    assert OldCircuit is Circuit
    assert OldParameter is Parameter
    assert OldQubit is Qubit
    assert Instruction is cqlib.Instruction
    assert CqlibError is cqlib.CqlibError
    for module, name in [
        ("circuit", "Circuit"),
        ("qubit", "Qubit"),
        ("parameter", "Parameter"),
        ("instruction", "Instruction"),
    ]:
        assert getattr(
            importlib.import_module(f"cqlib.circuits.{module}"), name
        ) is getattr(cqlib, name)
    assert type(OldCircuit(2)) is Circuit
    assert type(InstructionData(gates.H(), [0])) is ValueOperation


def test_native_imports_are_warning_free_in_fresh_process():
    subprocess.run(
        [
            sys.executable,
            "-Werror::DeprecationWarning",
            "-c",
            "from cqlib import *; from cqlib.circuit import *; Circuit(1).h(0)",
        ],
        check=True,
        capture_output=True,
        text=True,
    )


GATE_CASES = (
    [
        (name, name, ())
        for name in "H X Y Z S T CX CY CZ CCX SWAP X2P X2M Y2P Y2M".split()
    ]
    + [("SD", "SDG", ()), ("TD", "TDG", ()), ("CNOT", "CX", ()), ("CCNOT", "CCX", ())]
    + [(name, name, (0.37,)) for name in "RX RY RZ CRX CRY CRZ XY XY2P XY2M".split()]
    + [("U", "U", (0.2, 0.3, 0.4)), ("FSIM", "FSIM", (0.2, 0.3))]
)


@pytest.mark.parametrize("old_name,new_name,params", GATE_CASES)
def test_gate_factories_and_append_preserve_gate_semantics(old_name, new_name, params):
    from cqlib.circuits import gates

    gate = getattr(gates, old_name)(*params)
    native = getattr(StandardGate, new_name)(*params)
    assert type(gate) is StandardGate
    np.testing.assert_allclose(gate.matrix(), native.matrix(), atol=1e-12)
    circuit = Circuit(gate.num_qubits)
    expected = Circuit(gate.num_qubits)
    circuit.append(gate, list(range(gate.num_qubits)))
    expected.append_gate(native, list(range(gate.num_qubits)))
    assert circuit == expected


def test_instruction_data_keeps_symbolic_bound_parameters_and_native_interop():
    from cqlib.circuits import InstructionData, gates

    theta, phi = Parameter("theta"), Parameter("phi")
    operation = InstructionData(gates.RX(theta=theta + 2 * phi), Qubit(4))
    assert operation.params == [theta + 2 * phi]
    assert operation.qubits == [Qubit(4)]
    circuit = Circuit([4])
    circuit.append_instruction_data(operation)
    expected = Circuit([4])
    expected.rx(4, theta + 2 * phi)
    assert circuit == expected
    circuit.append(instruction=gates.U(theta=0.1, phi=0.2, lam=0.3), qubits=4)
    expected.u(4, 0.1, 0.2, 0.3)
    assert circuit == expected
    with pytest.raises(TypeError, match="labels"):
        gates.RX(0.1, label="readout")


def test_old_builders_forward_without_changing_native_forms():
    old, native = Circuit([2, 7]), Circuit([2, 7])
    old.add_qubit(11)
    native.add_qubits([11])
    old.sd(2)
    native.sdg(2)
    old.td(Qubit(7))
    native.tdg(7)
    old.i(2, t=12)
    native.delay(2, 12)
    old.i(7)
    native.i(7)
    old.barrier(2, Qubit(7))
    native.barrier([2, 7])
    old.barrier_all()
    native.barrier([2, 7, 11])
    assert old == native
    with warnings.catch_warnings():
        warnings.simplefilter("error", CqlibDeprecationWarning)
        old.barrier(qubits=[2])
        old.i(qubit=2)
        old.append(
            operation=ValueOperation.from_standard_gate(StandardGate.H, [Qubit(2)])
        )


def test_invalid_bulk_inputs_leave_circuit_and_classical_identity_unchanged():
    from cqlib.circuits import gates

    circuit = Circuit(2)
    handle = circuit.measure(0)
    before, identity = circuit.copy(), circuit.id
    for action in [
        lambda: circuit.measure([1, 99]),
        lambda: circuit.barrier(0, 99),
        lambda: circuit.append(gates.CX(), [0, 99]),
    ]:
        with pytest.raises(CircuitError):
            action()
        assert circuit == before
        assert circuit.id == identity
    target = circuit.var(ClassicalType.bit())
    circuit.store(target, handle.expr())
    circuit.validate()


def test_bulk_measurement_uses_real_ids_and_preserves_native_measurement_return():
    circuit = Circuit([8, 2, 17])
    circuit.h(8)
    handle = circuit.measure(qubit=8)
    assert isinstance(handle, Measurement)
    assert circuit.measure([Qubit(17)]) is None
    assert circuit.measure_all() is None
    assert [op.qubits for op in circuit.operations[1:]] == [
        [Qubit(8)],
        [Qubit(17)],
        [Qubit(2)],
    ]
    before = circuit.copy()
    assert circuit.measure_all() is None
    assert circuit == before
    assert circuit.measure([]) is None
    circuit.validate()


def test_measure_all_recognizes_imported_measurements():
    circuit = qcis.loads("H Q2\nCZ Q2 Q7\nM Q2\n")
    circuit.measure_all()
    assert len(circuit) == 4
    assert circuit[-1].qubits == [Qubit(7)]
    circuit.measure_all()
    assert len(circuit) == 4


def test_old_bulk_measurement_rejects_dynamic_or_nonterminal_workflows():
    circuit = Circuit(2)
    circuit.measure(0)
    circuit.x(0)
    before = circuit.copy()
    with pytest.raises(CircuitError, match="terminal"):
        circuit.measure_all()
    with pytest.raises(CircuitError, match="terminal"):
        circuit.measure([1])
    assert circuit == before
    # Native measurement remains usable in precisely this situation.
    assert isinstance(circuit.measure(0), Measurement)
    circuit = Circuit(2)
    circuit.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))
    with pytest.raises(CircuitError, match="static"):
        circuit.measure_all()


def test_copy_and_addition_keep_native_identity_and_compose_semantics():
    left = Circuit([7])
    left.h(7)
    right = Circuit([2, 7])
    right.cx(7, 2)
    expected = copy.copy(left)
    expected.compose(right)
    result = left + right
    assert type(result) is Circuit
    assert result == expected
    assert left.num_qubits == 1
    alias, identity = left, left.id
    left += right
    assert left is alias and left.id == identity
    assert left == expected
    before = left.copy()
    left += left
    expected = before + before
    assert left is alias and left == expected
    for cloned in [left.copy(), copy.copy(left), copy.deepcopy(left)]:
        assert cloned == left and cloned is not left
        assert cloned.id != left.id
        cloned.x(2)
        assert cloned != left


def test_inplace_addition_preserves_classical_handles():
    left, right = Circuit(2), Circuit(2)
    handle = left.measure(0)
    right.h(1)
    left += right
    variable = left.var(ClassicalType.bit())
    left.store(variable, handle.expr())
    left.validate()
