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

from pathlib import Path

import pytest

from cqlib.circuit import Circuit, StandardGate, ValueOperation
from cqlib.ir import qcis


SUPPORTED_STANDARD_GATES = [
    "H Q0",
    "RX Q0 0.1",
    "RXX Q0 Q1 0.1",
    "RXY Q0 0.1 0.2",
    "RY Q0 0.1",
    "RYY Q0 Q1 0.1",
    "RZ Q0 0.1",
    "RZX Q0 Q1 0.1",
    "RZZ Q0 Q1 0.1",
    "S Q0",
    "SD Q0",
    "SWAP Q0 Q1",
    "T Q0",
    "TD Q0",
    "U Q0 0.1 0.2 0.3",
    "X Q0",
    "XY Q0 0.1",
    "X2P Q0",
    "X2M Q0",
    "XY2P Q0 0.1",
    "XY2M Q0 0.1",
    "Y Q0",
    "Y2P Q0",
    "Y2M Q0",
    "Z Q0",
    "PHASE Q0 0.1",
    "CX Q0 Q1",
    "CCX Q0 Q1 Q2",
    "CY Q0 Q1",
    "CZ Q0 Q1",
    "CRX Q0 Q1 0.1",
    "CRY Q0 Q1 0.1",
    "CRZ Q0 Q1 0.1",
    "FSIM Q0 Q1 0.1 0.2",
]


@pytest.mark.parametrize("source", SUPPORTED_STANDARD_GATES)
def test_supported_standard_gate_roundtrip(source: str) -> None:
    assert qcis.dumps(qcis.loads(source)) == f"{source}\n"


@pytest.mark.parametrize(
    ("source", "expected"),
    [("SDG Q0", "SD Q0\n"), ("TDG Q0", "TD Q0\n")],
)
def test_dagger_aliases_are_normalized(source: str, expected: str) -> None:
    assert qcis.dumps(qcis.loads(source)) == expected


def test_delay_remains_distinct_from_standard_identity() -> None:
    assert qcis.dumps(qcis.loads("I Q0 10")) == "I Q0 10\n"

    circuit = Circuit(1)
    circuit.i(0)
    with pytest.raises(ValueError, match="Unsupported gate 'I'"):
        qcis.dumps(circuit)


def test_gphase_is_not_supported(tmp_path: Path) -> None:
    with pytest.raises(ValueError, match="Unknown gate: 'GPHASE'"):
        qcis.loads("GPHASE 0.1")

    circuit = Circuit(0)
    circuit.append(ValueOperation.from_standard_gate(StandardGate.GPhase(0.1), []))
    with pytest.raises(ValueError, match="Unsupported gate 'GPhase'"):
        qcis.dumps(circuit)
    with pytest.raises(ValueError, match="Unsupported gate 'GPhase'"):
        qcis.dump(circuit, str(tmp_path / "gphase.qcis"))


@pytest.mark.parametrize("source", ["CCX Q0 Q1", "FSIM Q0 Q1 0.1"])
def test_invalid_standard_gate_arity_is_a_value_error(source: str) -> None:
    with pytest.raises(ValueError):
        qcis.loads(source)
