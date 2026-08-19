# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
#
# Any modifications or derivative works of this code must retain this
# copyright notice, and modified files need to carry a notice indicating
# that they have been altered from the originals.

"""
Tests for QCIS IR export (dump/dumps) functionality.

Test Coverage:
- QCIS string generation (dumps)
- QCIS file generation (dump)
- Standard gates to QCIS format
- Native QCIS gates (X2P, X2M, Y2P, Y2M, XY2P, XY2M, CZ, RZ, I)
- Parametric gates with numeric and symbolic parameters
- Barrier and Measurement directives
- Float formatting (pi, pi/2, integers)
- Round-trip conversion (Circuit -> QCIS -> Circuit)
- Unsupported gates error handling
"""

import tempfile
import os
import math
from cqlib.circuit import Circuit
from cqlib.ir.qcis import dumps, dump, loads


class TestQcisDumpsEmpty:
    """Test dumping empty circuits."""

    def test_dumps_empty_circuit(self):
        """Dump empty circuit and verify output."""
        c = Circuit(0)
        qcis = dumps(c)
        assert isinstance(qcis, str)
        assert qcis == ""

    def test_dumps_circuit_no_ops(self):
        """Dump circuit with qubits but no operations."""
        c = Circuit(2)
        qcis = dumps(c)
        assert isinstance(qcis, str)
        assert qcis == ""


class TestQcisDumpsNativeGates:
    """Test dumping native QCIS gates."""

    def test_dumps_x2p_gate(self):
        """Dump X2P gate and verify output format."""
        c = Circuit(1)
        c.x2p(0)
        qcis = dumps(c)
        assert "X2P Q0" in qcis

    def test_dumps_x2m_gate(self):
        """Dump X2M gate and verify output format."""
        c = Circuit(1)
        c.x2m(0)
        qcis = dumps(c)
        assert "X2M Q0" in qcis

    def test_dumps_y2p_gate(self):
        """Dump Y2P gate and verify output format."""
        c = Circuit(1)
        c.y2p(0)
        qcis = dumps(c)
        assert "Y2P Q0" in qcis

    def test_dumps_y2m_gate(self):
        """Dump Y2M gate and verify output format."""
        c = Circuit(1)
        c.y2m(0)
        qcis = dumps(c)
        assert "Y2M Q0" in qcis

    def test_dumps_xy2p_gate(self):
        """Dump XY2P gate and verify output format."""
        c = Circuit(1)
        c.xy2p(0, 1.0)
        qcis = dumps(c)
        assert "XY2P Q0 1" in qcis

    def test_dumps_xy2m_gate(self):
        """Dump XY2M gate with pi expression."""
        c = Circuit(1)
        c.xy2m(0, math.pi / 2)
        qcis = dumps(c)
        assert "XY2M Q0 pi/2" in qcis


class TestQcisDumpsStandardGates:
    """Test dumping standard quantum gates."""

    def test_dumps_pauli_gates(self):
        """Dump Pauli gates and verify output."""
        c = Circuit(1)
        c.x(0)
        c.y(0)
        c.z(0)
        qcis = dumps(c)

        assert "X Q0" in qcis
        assert "Y Q0" in qcis
        assert "Z Q0" in qcis

    def test_dumps_hadamard(self):
        """Dump Hadamard gate and verify output."""
        c = Circuit(1)
        c.h(0)
        qcis = dumps(c)
        assert "H Q0" in qcis

    def test_dumps_phase_gates(self):
        """Dump phase gates and verify output."""
        c = Circuit(4)
        c.s(0)
        c.sdg(1)
        c.t(2)
        c.tdg(3)
        qcis = dumps(c)

        assert "S Q0" in qcis
        assert "SD Q1" in qcis
        assert "T Q2" in qcis
        assert "TD Q3" in qcis


class TestQcisDumpsTwoQubitGates:
    """Test dumping two-qubit gates."""

    def test_dumps_cz_gate(self):
        """Dump CZ gate and verify output."""
        c = Circuit(2)
        c.cz(0, 1)
        qcis = dumps(c)
        assert "CZ Q0 Q1" in qcis


class TestQcisDumpsParametricGates:
    """Test dumping parametric rotation gates."""

    def test_dumps_rx_gate(self):
        """Dump RX gate and verify output."""
        c = Circuit(1)
        c.rx(0, 1.0)
        qcis = dumps(c)
        assert "RX Q0 1" in qcis

    def test_dumps_ry_gate(self):
        """Dump RY gate with pi expression."""
        c = Circuit(1)
        c.ry(0, math.pi / 2)
        qcis = dumps(c)
        assert "RY Q0 pi/2" in qcis

    def test_dumps_rz_gate(self):
        """Dump RZ gate with pi expression."""
        c = Circuit(1)
        c.rz(0, math.pi)
        qcis = dumps(c)
        assert "RZ Q0 pi" in qcis

    def test_dumps_rxy_gate(self):
        """Dump RXY gate and verify output."""
        c = Circuit(1)
        c.rxy(0, 1.0, 2.0)
        qcis = dumps(c)
        assert "RXY Q0 1 2" in qcis

    def test_dumps_pi_variants(self):
        """Dump gates with various pi expressions."""
        c = Circuit(4)
        c.rz(0, math.pi)
        c.rz(1, -math.pi)
        c.rz(2, math.pi / 2)
        c.rz(3, math.pi / 4)
        qcis = dumps(c)

        assert "RZ Q0 pi" in qcis
        assert "RZ Q1 -pi" in qcis
        assert "RZ Q2 pi/2" in qcis
        assert "RZ Q3 pi/4" in qcis


class TestQcisDumpsDirectives:
    """Test dumping directives (Barrier, Measurement)."""

    def test_dumps_barrier_single_qubit(self):
        """Dump barrier on single qubit."""
        c = Circuit(1)
        c.barrier([0])
        qcis = dumps(c)
        assert "B Q0" in qcis

    def test_dumps_barrier_multiple_qubits(self):
        """Dump barrier on multiple qubits."""
        c = Circuit(3)
        c.barrier([0, 1, 2])
        qcis = dumps(c)
        assert "B Q0 Q1 Q2" in qcis

    def test_dumps_measurement_single_qubit(self):
        """Dump measurement on single qubit."""
        c = Circuit(1)
        c.measure(0)
        qcis = dumps(c)
        assert "M Q0" in qcis

    def test_dumps_measurement_multiple_qubits(self):
        """Dump measurement on multiple qubits."""
        c = Circuit(2)
        c.measure(0)
        c.measure(1)
        qcis = dumps(c)
        assert "M Q0" in qcis
        assert "M Q1" in qcis


class TestQcisDumpsFloatFormatting:
    """Test float value formatting in QCIS output."""

    def test_dumps_integer_one(self):
        """Verify integer 1 is formatted as '1'."""
        c = Circuit(1)
        c.xy2p(0, 1.0)
        qcis = dumps(c)
        assert "1" in qcis
        assert "1.0" not in qcis

    def test_dumps_integer_zero(self):
        """Verify integer 0 is formatted as '0'."""
        c = Circuit(1)
        c.rz(0, 0.0)
        qcis = dumps(c)
        assert "0" in qcis
        assert "0.0" not in qcis

    def test_dumps_float_precision(self):
        """Verify float precision is handled correctly."""
        c = Circuit(1)
        c.rx(0, 3.14159)
        qcis = dumps(c)
        assert "3.14159" in qcis


class TestQcisDumpsSymbolicParameters:
    """Test dumping circuits with symbolic parameters."""

    def test_dumps_symbolic_single_param(self):
        """Dump circuit with symbolic parameter."""
        from cqlib.circuit import Parameter

        c = Circuit(1)
        theta = Parameter("theta")
        c.rx(0, theta)
        qcis = dumps(c)
        assert "theta" in qcis

    def test_dumps_symbolic_expression(self):
        """Dump circuit with symbolic expression."""
        from cqlib.circuit import Parameter

        c = Circuit(1)
        theta = Parameter("theta")
        c.rz(0, theta + 0.5)
        qcis = dumps(c)
        assert "theta" in qcis


class TestQcisDumpsComplexCircuits:
    """Test dumping complex quantum circuits."""

    def test_dumps_bell_state(self):
        """Dump Bell state circuit."""
        c = Circuit(2)
        c.x2p(0)
        c.x2m(1)
        c.cz(0, 1)
        qcis = dumps(c)

        assert "X2P Q0" in qcis
        assert "X2M Q1" in qcis
        assert "CZ Q0 Q1" in qcis

    def test_dumps_ghz_state(self):
        """Dump GHZ state circuit."""
        c = Circuit(3)
        c.h(0)
        c.cz(0, 1)
        c.cz(0, 2)
        qcis = dumps(c)

        assert "H Q0" in qcis
        assert "CZ Q0 Q1" in qcis
        assert "CZ Q0 Q2" in qcis

    def test_dumps_qft_circuit(self):
        """Dump QFT-like circuit using QCIS native gates."""
        # CRZ is not natively supported by QCIS, use RZ instead
        c = Circuit(3)
        c.h(0)
        c.rz(1, math.pi / 2)
        c.rz(2, math.pi / 4)
        c.h(1)
        c.rz(2, math.pi / 2)
        c.h(2)
        qcis = dumps(c)

        assert isinstance(qcis, str)
        assert len(qcis) > 0
        assert "H Q0" in qcis
        assert "H Q1" in qcis
        assert "H Q2" in qcis
        assert "RZ" in qcis


class TestQcisDumpsFileIO:
    """Test QCIS file I/O operations."""

    def test_dump_to_file(self):
        """Dump circuit to QCIS file."""
        c = Circuit(2)
        c.h(0)
        c.cz(0, 1)

        with tempfile.NamedTemporaryFile(mode="w", suffix=".qcis", delete=False) as f:
            temp_path = f.name

        try:
            dump(c, temp_path)
            assert os.path.exists(temp_path)

            # Verify file content
            with open(temp_path, "r") as f:
                content = f.read()
            assert "H Q0" in content
            assert "CZ Q0 Q1" in content
        finally:
            os.unlink(temp_path)

    def test_dump_overwrites_existing_file(self):
        """Verify dump overwrites existing file."""
        c1 = Circuit(1)
        c1.h(0)

        c2 = Circuit(1)
        c2.x(0)

        with tempfile.NamedTemporaryFile(mode="w", suffix=".qcis", delete=False) as f:
            temp_path = f.name

        try:
            dump(c1, temp_path)
            dump(c2, temp_path)

            with open(temp_path, "r") as f:
                content = f.read()
            assert "X Q0" in content
            assert "H Q0" not in content
        finally:
            os.unlink(temp_path)


class TestQcisDumpsAdditionalStandardGates:
    """Test dumping additional standard gates supported by QCIS."""

    def test_dumps_cx_gate(self):
        """Verify CX gate is emitted as a QCIS operation."""
        c = Circuit(2)
        c.cx(0, 1)
        assert dumps(c) == "CX Q0 Q1\n"

    def test_dumps_cnot_alias(self):
        """Verify CNOT-style CX usage is emitted as CX."""
        c = Circuit(2)
        c.cx(0, 1)
        assert dumps(c) == "CX Q0 Q1\n"


class TestQcisRoundTrip:
    """Test round-trip conversion (Circuit -> QCIS -> Circuit)."""

    def test_roundtrip_simple_circuit(self):
        """Test simple circuit round-trip."""
        c1 = Circuit(2)
        c1.h(0)
        c1.cz(0, 1)

        qcis = dumps(c1)
        c2 = loads(qcis)

        assert c2.num_qubits == 2
        assert len(c2) == 2

    def test_roundtrip_parametric_circuit(self):
        """Test parametric circuit round-trip."""
        c1 = Circuit(2)
        c1.rx(0, 1.0)
        c1.ry(1, math.pi / 2)
        c1.cz(0, 1)

        qcis = dumps(c1)
        c2 = loads(qcis)

        assert c2.num_qubits == 2
        assert len(c2) == 3

    def test_roundtrip_native_gates(self):
        """Test native QCIS gates round-trip."""
        c1 = Circuit(3)
        c1.x2p(0)
        c1.x2m(1)
        c1.xy2p(2, 1.0)
        c1.cz(0, 1)
        c1.rz(2, math.pi / 4)

        qcis = dumps(c1)
        c2 = loads(qcis)

        assert c2.num_qubits == 3
        assert len(c2) == 5

    def test_roundtrip_preserves_operations(self):
        """Test that round-trip preserves operation order."""
        c1 = Circuit(1)
        c1.h(0)
        c1.x(0)
        c1.y(0)
        c1.z(0)

        qcis = dumps(c1)
        c2 = loads(qcis)

        assert c2[0].name == "H"
        assert c2[1].name == "X"
        assert c2[2].name == "Y"
        assert c2[3].name == "Z"

    def test_roundtrip_with_measurements(self):
        """Test round-trip with measurements."""
        c1 = Circuit(2)
        c1.h(0)
        c1.cz(0, 1)
        c1.measure(0)
        c1.measure(1)

        qcis = dumps(c1)
        c2 = loads(qcis)

        assert c2.num_qubits == 2
        # Measurements are represented as classical-data operations and dump
        # back to QCIS M lines.
        assert dumps(c2).endswith("M Q0\nM Q1\n")
