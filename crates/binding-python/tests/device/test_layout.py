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

"""Tests for Layout APIs."""

import pytest

from cqlib import Qubit
from cqlib.device import Layout, LogicalQubit, PhysicalQubit


class TestLayout:
    """Tests layout mapping and swap behaviors."""

    def test_layout_mapping_and_swap(self):
        """Layout should expose maps and update mapping after swap."""
        layout = Layout(
            logical=[0, 1], physical=[10, 11, 12], init_map={Qubit(0): Qubit(11)}
        )
        assert layout.num_logical == 2
        assert layout.num_physical == 3
        assert layout.num_vacant_physical == 1

        assert set(layout.logical_qubits) == {LogicalQubit(0), LogicalQubit(1)}
        assert set(layout.physical_qubits) == {
            PhysicalQubit(10),
            PhysicalQubit(11),
            PhysicalQubit(12),
        }
        assert set(layout.l2p_map.keys()).issuperset({LogicalQubit(0), LogicalQubit(1)})
        assert set(layout.p2l_map.keys()).issubset(
            {PhysicalQubit(10), PhysicalQubit(11), PhysicalQubit(12)}
        )

        assert layout.get_physical(0) == PhysicalQubit(11)
        v_on_11 = layout.get_logical(11)
        v_on_12 = layout.get_logical(12)

        layout.swap_physical(11, 12)
        assert layout.get_logical(11) == v_on_12
        assert layout.get_logical(12) == v_on_11

    def test_layout_strong_typed_returns(self):
        """Device qubit outputs should keep their logical/physical identity."""
        layout = Layout(logical=[0], physical=[10, 11], init_map={0: 11})

        physical = layout.get_physical(0)
        # Strong identity: equal to the matching physical qubit only.
        assert physical == PhysicalQubit(11)
        assert physical != Qubit(11)
        assert physical != LogicalQubit(11)
        # Explicit bridge to the circuit qubit and numeric id/index.
        assert physical.qubit == Qubit(11)
        assert physical.id == 11
        assert physical.index == 11

        logical = layout.get_logical(11)
        assert logical == LogicalQubit(0)
        assert logical != Qubit(0)
        assert logical.qubit == Qubit(0)
        assert str(logical) == "L0"
        assert str(physical) == "P11"
        assert repr(logical) == "LogicalQubit(0)"
        assert repr(physical) == "PhysicalQubit(11)"

        # Typed mapping keys/values.
        assert layout.l2p_map == {LogicalQubit(0): PhysicalQubit(11)}
        assert layout.p2l_map == {PhysicalQubit(11): LogicalQubit(0)}

        # Results round-trip back into layout queries.
        assert layout.get_logical(layout.get_physical(0)) == LogicalQubit(0)
        assert layout.get_physical(layout.get_logical(11)) == PhysicalQubit(11)

        # unbind returns the released physical qubit.
        assert layout.unbind(0) == PhysicalQubit(11)
        assert layout.get_physical(0) is None

    def test_device_qubits_are_orderable(self):
        """Strongly typed device qubits should sort by id."""
        assert sorted([PhysicalQubit(2), PhysicalQubit(0)]) == [
            PhysicalQubit(0),
            PhysicalQubit(2),
        ]
        assert PhysicalQubit(1) < PhysicalQubit(2)
        assert LogicalQubit(1) < LogicalQubit(2)
        assert LogicalQubit(2) >= LogicalQubit(2)

    def test_layout_swap_rejects_unknown_physical(self):
        """swap_physical should reject physical qubits outside layout."""
        layout = Layout(logical=[0], physical=[10], init_map=None)
        with pytest.raises(ValueError):
            layout.swap_physical(10, 99)
