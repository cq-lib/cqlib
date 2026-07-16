# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

import pytest

from cqlib import Circuit, Qubit
from cqlib.ir import qasm3


def sparse_circuit() -> Circuit:
    q1 = Qubit(1)
    q5 = Qubit(5)
    circuit = Circuit([q1, q5])
    circuit.h(q1)
    circuit.cx(q1, q5)
    return circuit


def test_dumps_defaults_to_logical_qubits() -> None:
    qasm = qasm3.dumps(sparse_circuit())

    assert "qubit[2] q;" in qasm
    assert "h q[0];" in qasm
    assert "cx q[0],q[1];" in qasm


def test_dumps_supports_physical_qubits() -> None:
    qasm = qasm3.dumps(sparse_circuit(), qubit_mode="physical")

    assert "qubit[" not in qasm
    assert "h $1;" in qasm
    assert "cx $1,$5;" in qasm


def test_dump_supports_physical_qubits(tmp_path) -> None:
    path = tmp_path / "physical.qasm"

    qasm3.dump(sparse_circuit(), str(path), qubit_mode="physical")

    assert "h $1;" in path.read_text()


def test_dumps_rejects_unknown_qubit_mode() -> None:
    with pytest.raises(ValueError, match="qubit_mode must be 'logical' or 'physical'"):
        qasm3.dumps(sparse_circuit(), qubit_mode="unknown")
