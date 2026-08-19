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
Tests for QCIS IR import (load/loads) functionality.

Test Coverage:
- QCIS string parsing (loads)
- QCIS file loading (load)
- Standard single-qubit gates (X, Y, Z, H, S, T, etc.)
- Native QCIS gates (X2P, X2M, Y2P, Y2M, XY2P, XY2M, CZ, RZ, I)
- Parametric gates (RX, RY, RZ, RXY)
- Barrier and Measurement directives
- Parameter expressions (pi, arithmetic operations)
- Comments and whitespace handling
- Error handling (invalid qubit format, parameter count mismatch)
"""

import pytest
import tempfile
import os
from cqlib.ir.qcis import dumps, loads, load


class TestQcisLoadsBasic:
    """Test basic QCIS string parsing (loads)."""

    def test_loads_empty_string(self):
        """Parse empty string and verify empty circuit."""
        c = loads("")
        assert c.num_qubits == 0
        assert len(c) == 0

    def test_loads_single_qubit_no_ops(self):
        """Parse circuit with qubits but no operations."""
        qcis = """
X2P Q0
X2M Q1
"""
        c = loads(qcis)
        assert c.num_qubits == 2
        assert len(c) == 2


class TestQcisLoadsNativeGates:
    """Test parsing of native QCIS gates."""

    def test_loads_x2p_gate(self):
        """Parse X2P gate and verify."""
        qcis = "X2P Q0"
        c = loads(qcis)
        assert c.num_qubits == 1
        assert len(c) == 1
        assert c[0].name == "X2P"
        assert c[0].qubits[0].index == 0

    def test_loads_x2m_gate(self):
        """Parse X2M gate and verify."""
        qcis = "X2M Q0"
        c = loads(qcis)
        assert len(c) == 1
        assert c[0].name == "X2M"

    def test_loads_y2p_gate(self):
        """Parse Y2P gate and verify."""
        qcis = "Y2P Q0"
        c = loads(qcis)
        assert len(c) == 1
        assert c[0].name == "Y2P"

    def test_loads_y2m_gate(self):
        """Parse Y2M gate and verify."""
        qcis = "Y2M Q0"
        c = loads(qcis)
        assert len(c) == 1
        assert c[0].name == "Y2M"

    def test_loads_xy2p_gate(self):
        """Parse XY2P gate with parameter and verify."""
        qcis = "XY2P Q0 1.0"
        c = loads(qcis)
        assert c.num_qubits == 1
        assert len(c) == 1
        assert c[0].name == "XY2P"
        assert c[0].num_params == 1

    def test_loads_xy2m_gate(self):
        """Parse XY2M gate with pi expression and verify."""
        qcis = "XY2M Q0 pi/2"
        c = loads(qcis)
        assert len(c) == 1
        assert c[0].name == "XY2M"

    def test_loads_cz_gate(self):
        """Parse CZ gate and verify control/target."""
        qcis = """
H Q0
CZ Q0 Q1
"""
        c = loads(qcis)
        assert c.num_qubits == 2
        assert len(c) == 2

        # First is H gate
        assert c[0].name == "H"

        # Second is CZ gate
        assert c[1].name == "CZ"
        assert c[1].num_qubits == 2
        assert c[1].qubits[0].index == 0
        assert c[1].qubits[1].index == 1

    def test_loads_rz_gate(self):
        """Parse RZ gate and verify."""
        qcis = "RZ Q0 pi/4"
        c = loads(qcis)
        assert len(c) == 1
        assert c[0].name == "RZ"
        assert c[0].num_params == 1

    def test_loads_delay_gate(self):
        """Parse I (delay) gate and verify."""
        qcis = "I Q0 1.0"
        c = loads(qcis)
        assert len(c) == 1
        # QCIS "I" gate is parsed as Delay in the internal representation
        assert dumps(c) == "I Q0 1\n"
        assert c[0].num_params == 1


class TestQcisLoadsStandardGates:
    """Test parsing of standard quantum gates."""

    def test_loads_pauli_gates(self):
        """Parse Pauli gates (X, Y, Z) and verify."""
        qcis = """
X Q0
Y Q1
Z Q2
"""
        c = loads(qcis)
        assert c.num_qubits == 3
        assert len(c) == 3

        assert c[0].name == "X"
        assert c[1].name == "Y"
        assert c[2].name == "Z"

    def test_loads_hadamard(self):
        """Parse Hadamard gate and verify."""
        qcis = "H Q0"
        c = loads(qcis)
        assert len(c) == 1
        assert c[0].name == "H"

    def test_loads_phase_gates(self):
        """Parse phase gates (S, SD, T, TD) and verify."""
        qcis = """
S Q0
SD Q1
T Q2
TD Q3
"""
        c = loads(qcis)
        assert c.num_qubits == 4
        assert len(c) == 4

        assert c[0].name == "S"
        assert c[1].name == "SDG"
        assert c[2].name == "T"
        assert c[3].name == "TDG"


class TestQcisLoadsParametricGates:
    """Test parsing of parametric rotation gates."""

    def test_loads_rx_gate(self):
        """Parse RX gate and verify."""
        qcis = "RX Q0 0.5"
        c = loads(qcis)
        assert len(c) == 1
        assert c[0].name == "RX"
        assert c[0].num_params == 1

    def test_loads_ry_gate(self):
        """Parse RY gate and verify."""
        qcis = "RY Q0 pi/2"
        c = loads(qcis)
        assert len(c) == 1
        assert c[0].name == "RY"

    def test_loads_rxy_gate(self):
        """Parse RXY gate with two parameters and verify."""
        qcis = "RXY Q0 1.0 0.5"
        c = loads(qcis)
        assert len(c) == 1
        assert c[0].name == "RXY"
        assert c[0].num_params == 2

    def test_loads_parametric_pi_expressions(self):
        """Parse gates with pi expressions and verify."""
        qcis = """
RX Q0 pi
RY Q1 pi/2
RZ Q2 pi/4+0.5
"""
        c = loads(qcis)
        assert c.num_qubits == 3
        assert len(c) == 3

        assert c[0].name == "RX"
        assert c[1].name == "RY"
        assert c[2].name == "RZ"

    def test_loads_parametric_arithmetic(self):
        """Parse gates with arithmetic expressions."""
        qcis = """
RX Q0 2*pi
RY Q1 pi/4+0.5
RZ Q2 3.14*2
"""
        c = loads(qcis)
        assert c.num_qubits == 3
        assert len(c) == 3


class TestQcisLoadsDirectives:
    """Test parsing of directives (Barrier, Measurement)."""

    def test_loads_barrier_single_qubit(self):
        """Parse barrier on single qubit and verify."""
        qcis = "B Q0"
        c = loads(qcis)
        assert len(c) == 1
        assert c[0].is_directive
        assert c[0].name == "barrier"

    def test_loads_barrier_multiple_qubits(self):
        """Parse barrier on multiple qubits and verify."""
        qcis = "B Q0 Q1 Q2 Q3"
        c = loads(qcis)
        assert len(c) == 1
        assert c[0].name == "barrier"
        assert c[0].num_qubits == 4

    def test_loads_measurement_single_qubit(self):
        """Parse measurement on single qubit and verify."""
        qcis = "M Q0"
        c = loads(qcis)
        assert len(c) == 1
        assert dumps(c) == "M Q0\n"
        assert c[0].qubits[0].index == 0

    def test_loads_measurement_multiple_qubits(self):
        """Parse measurement on multiple qubits and verify."""
        qcis = "M Q0 Q1 Q2"
        c = loads(qcis)
        # Each qubit measurement becomes a separate operation
        assert c.num_qubits == 3
        assert len(c) == 3
        assert dumps(c) == "M Q0\nM Q1\nM Q2\n"
        for i in range(3):
            assert c[i].qubits[0].index == i


class TestQcisLoadsComplexCircuits:
    """Test parsing of complex quantum circuits."""

    def test_loads_bell_state(self):
        """Parse Bell state circuit and verify."""
        qcis = """
X2P Q0
X2M Q1
CZ Q0 Q1
M Q0
M Q1
"""
        c = loads(qcis)
        assert c.num_qubits == 2
        assert len(c) == 5

        assert c[0].name == "X2P"
        assert c[1].name == "X2M"
        assert c[2].name == "CZ"
        assert dumps(c).endswith("M Q0\nM Q1\n")

    def test_loads_ghz_state(self):
        """Parse GHZ state circuit and verify."""
        qcis = """
H Q0
CZ Q0 Q1
CZ Q0 Q2
M Q0 Q1 Q2
"""
        c = loads(qcis)
        assert c.num_qubits == 3
        # M Q0 Q1 Q2 is expanded to 3 separate measurements
        assert len(c) == 6

    def test_loads_qft_circuit(self):
        """Parse QFT-like circuit and verify."""
        qcis = """
H Q0
RY Q1 pi/2
RZ Q2 pi/4
"""
        c = loads(qcis)
        assert c.num_qubits == 3
        assert len(c) == 3


class TestQcisLoadsEdgeCases:
    """Test edge cases and special scenarios."""

    def test_loads_with_comments(self):
        """Verify comments are ignored during parsing."""
        qcis = """
// This is a comment
RX Q0 1.0 // inline comment
// Another comment
RY Q1 pi/2
"""
        c = loads(qcis)
        assert c.num_qubits == 2
        assert len(c) == 2

    def test_loads_whitespace_handling(self):
        """Verify extra whitespace is handled correctly."""
        qcis = """


  H Q0

    X2P Q1

"""
        c = loads(qcis)
        assert c.num_qubits == 2
        assert len(c) == 2

    def test_loads_gate_order_preserved(self):
        """Verify gate order is preserved in output."""
        qcis = """
X Q0
Y Q0
Z Q0
H Q0
"""
        c = loads(qcis)
        assert len(c) == 4

        assert c[0].name == "X"
        assert c[1].name == "Y"
        assert c[2].name == "Z"
        assert c[3].name == "H"


class TestQcisLoadsFileIO:
    """Test QCIS file I/O operations."""

    def test_load_from_file(self):
        """Load circuit from QCIS file."""
        qcis_content = """
H Q0
CZ Q0 Q1
M Q0
M Q1
"""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".qcis", delete=False) as f:
            f.write(qcis_content)
            temp_path = f.name

        try:
            c = load(temp_path)
            assert c.num_qubits == 2
            assert len(c) == 4
            assert c[0].name == "H"
            assert c[1].name == "CZ"
        finally:
            os.unlink(temp_path)

    def test_load_nonexistent_file_raises(self):
        """Verify loading nonexistent file raises error."""
        with pytest.raises(Exception):
            load("/nonexistent/path/qcis.qcis")


class TestQcisLoadsErrorHandling:
    """Test error handling in QCIS parsing."""

    def test_loads_invalid_qubit_format_lowercase(self):
        """Verify lowercase qubit raises error."""
        qcis = "RX q0 1.0"
        with pytest.raises(Exception):
            loads(qcis)

    def test_loads_cz_requires_two_qubits(self):
        """Verify CZ gate requires exactly 2 qubits."""
        qcis = "CZ Q0"
        with pytest.raises(Exception):
            loads(qcis)

    def test_loads_rx_requires_one_param(self):
        """Verify RX gate requires exactly 1 parameter."""
        qcis = "RX Q0"
        with pytest.raises(Exception):
            loads(qcis)

    def test_loads_xy2p_requires_one_param(self):
        """Verify XY2P gate requires exactly 1 parameter."""
        qcis = "XY2P Q0"
        with pytest.raises(Exception):
            loads(qcis)

    def test_loads_delay_requires_one_param(self):
        """Verify I (delay) gate requires exactly 1 parameter."""
        qcis = "I Q0"
        with pytest.raises(Exception):
            loads(qcis)

    def test_loads_single_qubit_gate_no_params(self):
        """Verify single-qubit gates require 0 parameters."""
        qcis = "X Q0 1.0"
        with pytest.raises(Exception):
            loads(qcis)
