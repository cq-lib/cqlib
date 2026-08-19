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

"""Tests for device property types and Device APIs."""

from datetime import datetime, timezone

import pytest

from cqlib import Qubit
from cqlib.circuit import Instruction, StandardGate
from cqlib.device import (
    Device,
    EdgeProp,
    InstructionProp,
    PhysicalQubit,
    QubitProp,
    Topology,
)


class TestPropertyBuilders:
    """Tests builder-style property objects."""

    def test_instruction_qubit_edge_prop_builders(self):
        """InstructionProp/QubitProp/EdgeProp should preserve configured fields."""
        x_inst = Instruction.from_standard_gate(StandardGate.X)
        ip = InstructionProp(x_inst, 0.01)
        ip.length = 80.0
        assert ip.error_rate == pytest.approx(0.01)
        assert ip.length == pytest.approx(80.0)
        assert ip.instruction.name == "X"

        qp = QubitProp(0.02)
        qp.prob_meas0_prep1 = 0.03
        qp.prob_meas1_prep0 = 0.04
        qp.t1 = 120.0
        qp.t2 = 95.0
        qp.frequency = 5.1
        qp.add_native_instruction(ip)
        assert qp.readout_error == pytest.approx(0.02)
        assert qp.t1 == pytest.approx(120.0)
        assert qp.t2 == pytest.approx(95.0)
        assert qp.frequency == pytest.approx(5.1)
        assert len(qp.native_instructions) == 1
        assert qp.native_instructions[0].instruction.name == "X"

        cx_inst = Instruction.from_standard_gate(StandardGate.CX)
        eip = InstructionProp(cx_inst, 0.08)
        eip.length = 300.0
        ep = EdgeProp()
        ep.add_native_instruction(eip)
        assert len(ep.native_instructions) == 1
        assert ep.native_instructions[0].instruction.name == "CX"


class TestDeviceProperties:
    """Tests Device add/query and validation paths."""

    def test_device_add_and_query(self):
        """Device should store defaults and per-qubit/edge overrides."""
        topo = Topology([0, 1, 2], [(0, 1, "G1"), (1, 2, "G2")])
        device = Device("mock_backend", [Qubit(0), Qubit(1), Qubit(2)], topo)
        device.default_t1 = 50.0
        device.default_t2 = 35.0
        device.default_readout_error = 0.05
        device.default_single_qubit_error = 0.001
        device.default_two_qubit_error = 0.01
        device.native_gates = [
            Instruction.from_standard_gate(StandardGate.X),
            Instruction.from_standard_gate(StandardGate.CX),
        ]

        qp0 = QubitProp(0.02)
        qp0.t1 = 80.0
        qp0.t2 = 70.0
        device.add_qubit_properties(0, qp0)

        ep01 = EdgeProp()
        cx_inst = Instruction.from_standard_gate(StandardGate.CX)
        ep01.add_native_instruction(InstructionProp(cx_inst, 0.06))
        device.add_edge_properties(0, 1, ep01)

        assert device.name == "mock_backend"
        assert sorted(device.qubits) == [
            PhysicalQubit(0),
            PhysicalQubit(1),
            PhysicalQubit(2),
        ]
        assert device.invalid_qubits == []
        assert device.default_single_qubit_error == pytest.approx(0.001)
        assert device.default_two_qubit_error == pytest.approx(0.01)
        assert len(device.native_gates) == 2

        assert device.get_t1(0) == pytest.approx(80.0)
        assert device.get_t1(2) == pytest.approx(50.0)
        assert device.get_t2(0) == pytest.approx(70.0)
        assert device.get_t2(2) == pytest.approx(35.0)
        assert device.get_readout_error(0) == pytest.approx(0.02)
        assert device.get_readout_error(2) == pytest.approx(0.05)

        qp_query = device.qubit_properties(0)
        assert qp_query is not None
        assert qp_query.t1 == pytest.approx(80.0)

        ep_query = device.edge_properties(0, 1)
        assert ep_query is not None
        assert ep_query.native_instructions[0].instruction.name == "CX"

    def test_calibration_time_round_trip(self):
        """Calibration timestamps should round-trip, including pre-epoch ones."""
        topo = Topology([0, 1], [(0, 1, "G1")])
        device = Device("mock", [Qubit(0), Qubit(1)], topo)

        assert device.calibration_time is None

        timestamps = [
            datetime(1970, 1, 1, tzinfo=timezone.utc),
            datetime(1960, 1, 1, 0, 0, 0, 500000, tzinfo=timezone.utc),
            datetime(2026, 8, 18, 12, 34, 56, 789012, tzinfo=timezone.utc),
        ]
        for ts in timestamps:
            device.set_calibration_time(ts)
            assert device.calibration_time == ts

    def test_device_rejects_invalid_qubit_or_edge(self):
        """Adding properties outside topology should raise ValueError."""
        topo = Topology([0, 1], [(0, 1, "G1")])
        device = Device("mock", [Qubit(0), Qubit(1)], topo)

        with pytest.raises(ValueError):
            device.add_qubit_properties(9, QubitProp(0.01))

        with pytest.raises(ValueError):
            device.add_edge_properties(0, 9, EdgeProp())
