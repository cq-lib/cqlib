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

"""Tests for topology APIs exposed under cqlib.device."""

from cqlib.device import PhysicalQubit, Topology


class TestDeviceTopology:
    """Tests topology aliasing and graph helper behavior."""

    def test_topology_basic_queries(self):
        """Topology should expose counts, neighbors, degrees, and coupling names."""
        topo = Topology([0, 1, 2], [(0, 1, "G1"), (1, 2, "G2")])
        assert topo.num_qubits == 3
        assert topo.num_couplings == 2
        assert sorted(topo.qubits) == [
            PhysicalQubit(0),
            PhysicalQubit(1),
            PhysicalQubit(2),
        ]

        assert topo.contains_qubit(1) is True
        assert topo.contains_qubit(99) is False
        # Topology uses directed couplings; neighbors/degree are based on outgoing edges.
        assert set(topo.successors(1)) == {PhysicalQubit(2)}
        assert topo.out_degree(1) == 1
        assert topo.get_coupling_name(1, 2) == "G2"

    def test_topology_add_remove_qubits_and_couplings(self):
        """Topology should support qubit/coupling add-remove operations."""
        topo = Topology([0, 1, 2], [(0, 1, "G1")])
        topo.add_qubits([3])
        topo.add_couplings([(2, 3, "G2")])
        assert topo.contains_qubit(3) is True
        assert topo.supports_coupling_either_direction(2, 3)

        topo.remove_couplings([(2, 3)])
        assert not topo.supports_coupling_either_direction(2, 3)
        topo.remove_qubits([3])
        assert topo.contains_qubit(3) is False
