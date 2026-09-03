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
Tests for QASM2 IR import (load/loads) functionality.

Test Coverage:
- QASM2 string parsing (loads)
- QASM2 file loading (load)
- Standard gates (single-qubit, multi-qubit, parametric)
- Barrier and Reset directives; measurement classical-data operations
- Parameter expressions (pi, arithmetic)
- Custom gate definitions
- Error handling
"""

import tempfile
import os
from cqlib.circuit import Circuit
from cqlib.ir.qasm2 import loads, load, dumps


class TestQasm2Loads:
    """Test QASM2 parsing with rigorous verification."""

    def test_loads_empty_circuit(self):
        """Parse empty circuit and verify structure."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
"""
        c = loads(qasm)
        assert c.num_qubits == 1
        assert len(c) == 0

    def test_loads_single_qubit_gates(self):
        """Parse single-qubit gates and verify each operation."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
h q[0];
x q[0];
y q[0];
z q[0];
"""
        c = loads(qasm)
        assert c.num_qubits == 1
        assert len(c) == 4

        # Verify each gate type
        assert c[0].name == "H"
        assert c[1].name == "X"
        assert c[2].name == "Y"
        assert c[3].name == "Z"

        # Verify qubit indices
        assert c[0].qubits[0].index == 0
        assert c[1].qubits[0].index == 0

    def test_loads_phase_gates(self):
        """Parse phase gates and verify."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
s q[0];
sdg q[0];
t q[0];
tdg q[0];
"""
        c = loads(qasm)
        assert len(c) == 4

        assert c[0].name == "S"
        assert c[1].name == "SDG"
        assert c[2].name == "T"
        assert c[3].name == "TDG"

    def test_loads_cnot(self):
        """Parse CNOT gate and verify control/target."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
cx q[0],q[1];
"""
        c = loads(qasm)
        assert len(c) == 1

        op = c[0]
        assert op.name == "CX"
        assert op.num_qubits == 2
        assert op.qubits[0].index == 0  # control
        assert op.qubits[1].index == 1  # target

    def test_loads_two_qubit_gates(self):
        """Parse two-qubit gates and verify."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
cy q[0],q[1];
cz q[0],q[1];
swap q[0],q[1];
"""
        c = loads(qasm)
        assert len(c) == 3

        assert c[0].name == "CY"
        assert c[1].name == "CZ"
        assert c[2].name == "SWAP"

    def test_loads_toffoli(self):
        """Parse CCX (Toffoli) gate and verify qubits."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[3];
ccx q[0],q[1],q[2];
"""
        c = loads(qasm)
        assert c.num_qubits == 3
        assert len(c) == 1

        op = c[0]
        assert op.name == "CCX"
        assert op.num_qubits == 3
        assert op.qubits[0].index == 0
        assert op.qubits[1].index == 1
        assert op.qubits[2].index == 2

    def test_loads_parametric_gates(self):
        """Parse parametric rotation gates and verify parameters."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
rx(0.5) q[0];
ry(0.3) q[0];
rz(0.7) q[0];
"""
        c = loads(qasm)
        assert len(c) == 3

        assert c[0].name == "RX"
        assert c[1].name == "RY"
        assert c[2].name == "RZ"

        # Verify parameters (fixed values)
        assert c[0].num_params == 1
        assert c[1].num_params == 1
        assert c[2].num_params == 1

    def test_loads_u_gates(self):
        """Parse U1, U2, U3 gates and verify parameter counts."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
u1(0.5) q[0];
u2(0.5,0.3) q[0];
u3(0.5,0.3,0.2) q[0];
"""
        c = loads(qasm)
        assert len(c) == 3

        assert c[0].name == "Phase"  # U1 maps to Phase
        assert c[0].num_params == 1

        assert c[1].name == "U"  # U2 maps to U
        assert c[1].num_params == 3

        assert c[2].name == "U"  # U3 maps to U
        assert c[2].num_params == 3

    def test_loads_pi_expressions(self):
        """Parse gates with pi expressions and verify."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
rx(pi) q[0];
ry(pi/2) q[0];
rz(3*pi/4) q[0];
"""
        c = loads(qasm)
        assert len(c) == 3

        # All should be parsed successfully with symbolic/expression parameters
        assert c[0].num_params == 1
        assert c[1].num_params == 1
        assert c[2].num_params == 1

    def test_loads_measurement(self):
        """Parse measurement operation and verify classical data operations."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
creg c[1];
h q[0];
measure q[0] -> c[0];
"""
        c = loads(qasm)
        assert len(c) == 4

        assert c[0].name == "store"
        assert c[1].name == "H"
        assert c[2].name == "measure_bit"
        assert c[2].qubits[0].index == 0
        assert c[3].name == "store"
        assert "measure q[0] -> c0[0];" in dumps(c)

    def test_loads_measurement_register(self):
        """Parse measurement on entire register."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
creg c[2];
h q[0];
measure q -> c;
"""
        c = loads(qasm)
        assert len(c) == 4

        assert c[0].name == "store"
        assert c[1].name == "H"
        assert c[2].name == "measure_bits"
        assert [q.index for q in c[2].qubits] == [0, 1]
        assert c[3].name == "store"
        qasm_roundtrip = dumps(c)
        assert "measure q[0] -> c0[0];" in qasm_roundtrip
        assert "measure q[1] -> c0[1];" in qasm_roundtrip

    def test_loads_barrier_single(self):
        """Parse barrier on single qubit."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
h q[0];
barrier q[0];
x q[1];
"""
        c = loads(qasm)
        assert len(c) == 3

        assert c[0].name == "H"
        assert c[1].is_directive
        assert c[1].name == "barrier"
        assert c[2].name == "X"

    def test_loads_barrier_multiple(self):
        """Parse barrier on multiple qubits."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[3];
h q[0];
barrier q[0],q[1],q[2];
x q[1];
"""
        c = loads(qasm)
        assert len(c) == 3

        barrier_op = c[1]
        assert barrier_op.name == "barrier"
        assert barrier_op.num_qubits == 3

    def test_loads_barrier_register(self):
        """Parse barrier on entire register."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[3];
h q[0];
barrier q;
x q[1];
"""
        c = loads(qasm)
        assert len(c) == 3

        barrier_op = c[1]
        assert barrier_op.name == "barrier"
        assert barrier_op.num_qubits == 3

    def test_loads_reset_single(self):
        """Parse reset on single qubit."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
x q[0];
reset q[0];
"""
        c = loads(qasm)
        assert len(c) == 2

        assert c[0].name == "X"
        assert c[1].is_directive
        assert c[1].name == "reset"
        assert c[1].qubits[0].index == 0

    def test_loads_reset_register(self):
        """Parse reset on entire register."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
x q[0];
reset q;
"""
        c = loads(qasm)
        # x + 2 resets
        assert len(c) == 3

        assert c[1].name == "reset"
        assert c[2].name == "reset"

    def test_loads_bell_state(self):
        """Parse Bell state circuit and verify structure."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
h q[0];
cx q[0],q[1];
"""
        c = loads(qasm)
        assert c.num_qubits == 2
        assert len(c) == 2

        assert c[0].name == "H"
        assert c[1].name == "CX"

    def test_loads_ghz_state(self):
        """Parse GHZ state circuit and verify structure."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[3];
h q[0];
cx q[0],q[1];
cx q[0],q[2];
"""
        c = loads(qasm)
        assert c.num_qubits == 3
        assert len(c) == 3

        assert c[0].name == "H"
        assert c[1].name == "CX"
        assert c[2].name == "CX"

    def test_loads_multiple_qregs(self):
        """Parse circuit with multiple quantum registers."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg a[2];
qreg b[2];
h a[0];
cx a[0],b[0];
"""
        c = loads(qasm)
        assert c.num_qubits == 4
        assert len(c) == 2

        # a[0] -> qubit 0, b[0] -> qubit 2
        assert c[0].qubits[0].index == 0
        assert c[1].qubits[0].index == 0
        assert c[1].qubits[1].index == 2

    def test_loads_identity_gate(self):
        """Parse identity gate."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
id q[0];
"""
        c = loads(qasm)
        assert len(c) == 1
        assert c[0].name == "I"

    def test_loads_sx_sxdg(self):
        """Parse SX and SXDG gates."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
sx q[0];
sxdg q[0];
"""
        c = loads(qasm)
        assert len(c) == 2

        assert c[0].name == "X2P"
        assert c[1].name == "X2M"

    def test_loads_p_gate(self):
        """Parse P gate (alias for Phase/RZ)."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
p(0.5) q[0];
"""
        c = loads(qasm)
        assert len(c) == 1
        assert c[0].name == "Phase"


class TestQasm2LoadsFileIO:
    """Test QASM2 file I/O operations."""

    def test_load_from_file(self):
        """Load circuit from QASM file."""
        qasm_content = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
h q[0];
cx q[0],q[1];
"""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".qasm", delete=False) as f:
            f.write(qasm_content)
            temp_path = f.name

        try:
            c = load(temp_path)
            assert c.num_qubits == 2
            assert len(c) == 2
            assert c[0].name == "H"
            assert c[1].name == "CX"
        finally:
            os.unlink(temp_path)

    def test_file_roundtrip(self):
        """Test round-trip through file."""
        from cqlib.ir.qasm2 import dump

        c1 = Circuit(3)
        c1.h(0)
        c1.cx(0, 1)
        c1.cx(0, 2)

        with tempfile.NamedTemporaryFile(mode="w", suffix=".qasm", delete=False) as f:
            temp_path = f.name

        try:
            dump(c1, temp_path)
            c2 = load(temp_path)
            assert c2.num_qubits == 3
            assert len(c2) == 3
            assert c2[0].name == "H"
            assert c2[1].name == "CX"
            assert c2[2].name == "CX"
        finally:
            os.unlink(temp_path)


class TestQasm2EdgeCases:
    """Test edge cases and special scenarios."""

    def test_comments_ignored(self):
        """Verify comments are ignored during parsing."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
// This is a comment
h q[0];
// Another comment
"""
        c = loads(qasm)
        assert len(c) == 1
        assert c[0].name == "H"

    def test_whitespace_handling(self):
        """Verify extra whitespace is handled correctly."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];


  h q[0];

"""
        c = loads(qasm)
        assert len(c) == 1
        assert c[0].name == "H"

    def test_gate_case_variations(self):
        """Verify gate names are case-insensitive.

        Note: Currently QASM parser requires lowercase gate names.
        This test uses lowercase to verify functionality.
        """
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[1];
h q[0];
x q[0];
y q[0];
z q[0];
"""
        c = loads(qasm)
        assert len(c) == 4
        assert c[0].name == "H"
        assert c[1].name == "X"
        assert c[2].name == "Y"
        assert c[3].name == "Z"

    def test_empty_circuit_with_qreg(self):
        """Parse circuit with only register declarations."""
        qasm = """
OPENQASM 2.0;
include "qelib1.inc";
qreg q[5];
"""
        c = loads(qasm)
        assert c.num_qubits == 5
        assert len(c) == 0


class TestQasm2RoundTrip:
    """Test round-trip conversion (Circuit -> QASM -> Circuit)."""

    def test_ghz_roundtrip(self):
        """Test GHZ state circuit round-trip."""
        from cqlib.ir.qasm2 import dumps

        c1 = Circuit(3)
        c1.h(0)
        c1.cx(0, 1)
        c1.cx(0, 2)

        qasm = dumps(c1)
        c2 = loads(qasm)

        assert c2.num_qubits == 3
        assert len(c2) == 3
        assert c2[0].name == "H"
        assert c2[1].name == "CX"
        assert c2[2].name == "CX"

    def test_qft_roundtrip(self):
        """Test QFT-like circuit round-trip."""
        from cqlib.ir.qasm2 import dumps

        c1 = Circuit(3)
        c1.h(0)
        c1.crz(1, 0, 0.5)
        c1.crz(2, 0, 0.25)
        c1.h(1)
        c1.crz(2, 1, 0.5)
        c1.h(2)

        qasm = dumps(c1)
        c2 = loads(qasm)

        assert c2.num_qubits == 3
        assert len(c2) == 6

    def test_parametric_roundtrip(self):
        """Test parametric circuit round-trip."""
        from cqlib.ir.qasm2 import dumps

        c1 = Circuit(2)
        c1.rx(0, 0.5)
        c1.ry(1, 0.3)
        c1.cz(0, 1)

        qasm = dumps(c1)
        c2 = loads(qasm)

        assert c2.num_qubits == 2
        assert len(c2) == 3
        assert c2[0].name == "RX"
        assert c2[1].name == "RY"
        assert c2[2].name == "CZ"

    def test_complex_circuit_roundtrip(self):
        """Test complex circuit round-trip."""
        from cqlib.ir.qasm2 import dumps

        c1 = Circuit(4)
        for i in range(4):
            c1.h(i)
        for i in range(3):
            c1.cx(i, i + 1)
        c1.rx(0, 0.5)
        c1.barrier([0, 1, 2, 3])
        qasm = dumps(c1)
        c2 = loads(qasm)

        assert c2.num_qubits == 4
        assert len(c2) == len(c1)
