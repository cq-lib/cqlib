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
    qasm = qasm3.dumps(sparse_circuit(), physical_qubits=[7, 2])

    assert "qubit[" not in qasm
    assert "h $7;" in qasm
    assert "cx $7,$2;" in qasm


def test_dump_supports_physical_qubits(tmp_path) -> None:
    path = tmp_path / "physical.qasm"

    qasm3.dump(sparse_circuit(), str(path), physical_qubits=[7, 2])

    assert "h $7;" in path.read_text()


def test_dumps_rejects_invalid_physical_mapping() -> None:
    with pytest.raises(ValueError, match="has length 1"):
        qasm3.dumps(sparse_circuit(), physical_qubits=[7])


def test_loads_supports_physical_qubits() -> None:
    circuit = qasm3.loads(
        '''
        OPENQASM 3.0;
        include "stdgates.inc";
        h $5;
        cx $5, $7;
        '''
    )

    assert circuit.qubits == [Qubit(5), Qubit(7)]
