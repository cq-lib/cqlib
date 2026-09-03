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

"""Circuit dependency DAG analysis.

``CircuitDag`` is an operation-level dependency view of :class:`~.circuit.Circuit`.
It does not replace the circuit's normal operation list.  Instead, it
materializes resource dependencies between operations so analysis and compiler
passes can ask questions such as:

- which operations are ready in the front layer,
- which operations lie on a qubit or classical-value wire,
- which predecessors/successors are connected by quantum or classical resources,
- how operations can be arranged into ASAP layers,
- which one-qubit or two-qubit gate runs are contiguous on a wire.

Edges are resource-order dependencies, not commutation proofs.  If two
operations are connected by an edge, they share a resource timeline and must
preserve that source-program ordering in this DAG.  The DAG does not decide
whether two operations commute algebraically.

Node identifiers
----------------

DAG node identifiers are returned as ``int`` values.  They are stable only for
the owning ``CircuitDag`` object.  Do not store a node ID and use it with a
different DAG, even if both DAGs came from the same circuit.

Quick example
-------------

Build a DAG, inspect layers, and recover operations from node IDs::

    from cqlib import Circuit
    from cqlib.circuit import DagWire

    c = Circuit(2)
    c.h(0)
    c.x(1)
    c.cx(0, 1)

    dag = c.dag()
    assert dag.depth() == 2

    layers = dag.layers()
    first_layer_ops = [dag.operation(node).name for node in layers[0]]
    assert set(first_layer_ops) == {"H", "X"}

    q0_wire = DagWire.qubit(0)
    q0_nodes = dag.nodes_on_wire(q0_wire)
    assert [dag.operation(node).name for node in q0_nodes] == ["H", "CX"]

Classical dependency example
----------------------------

Classical measurement results are also resource timelines.  Allocate the
target variable type explicitly and use the measurement expression in a
classical operation::

    from cqlib import Circuit
    from cqlib.circuit import ClassicalType
    from cqlib.circuit import DagWire

    c = Circuit(1)
    m = c.measure(0)
    flag = c.var(ClassicalType.bool())
    c.store(flag, m.expr().bit_to_bool())

    dag = c.dag()
    value_wire = DagWire.classical_value(m.value)
    nodes = dag.nodes_on_wire(value_wire)
    assert [dag.operation(node).name for node in nodes] == ["measure_bit", "store"]
"""

from __future__ import annotations

from typing import Literal

from .bit import Qubit
from .circuit import Circuit, QubitLike
from .classical import ClassicalType, ClassicalValue, ClassicalVar
from .operation import ValueOperation
from .parameter import Parameter

class DagWire:
    """Resource timeline carried by a :class:`CircuitDag` edge.

    A wire identifies the resource that orders two adjacent DAG nodes.  Qubit
    wires represent quantum timelines; classical wires represent runtime
    classical storage or immutable measurement results; ``global_order`` is a
    fallback ordering resource for operations without concrete data resources.

    Examples:
        >>> DagWire.qubit(0).kind
        'qubit'
        >>> DagWire.global_order().kind
        'global_order'
    """

    @staticmethod
    def qubit(qubit: QubitLike) -> DagWire:
        """Create a quantum wire.

        Args:
            qubit: Integer qubit index or :class:`Qubit`.
        """
        ...

    @staticmethod
    def classical_var(var: ClassicalVar) -> DagWire:
        """Create a mutable classical-variable wire.

        Classical variable wires order writes to and reads from mutable runtime
        classical storage.
        """
        ...

    @staticmethod
    def classical_value(value: ClassicalValue) -> DagWire:
        """Create an immutable classical-value wire.

        Classical value wires are commonly produced by measurements and read by
        later classical expressions.
        """
        ...

    @staticmethod
    def global_order() -> DagWire:
        """Create the stable ordering wire for operations without data resources."""
        ...

    @property
    def kind(self) -> str:
        """Wire kind: ``qubit``, ``classical_var``, ``classical_value``, or ``global_order``."""
        ...

    @property
    def qubit_value(self) -> Qubit | None:
        """Contained qubit, if this is a qubit wire."""
        ...

    @property
    def classical_var_value(self) -> ClassicalVar | None:
        """Contained classical variable, if this is a classical-variable wire."""
        ...

    @property
    def classical_value_value(self) -> ClassicalValue | None:
        """Contained classical value, if this is a classical-value wire."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> DagWire: ...
    def __deepcopy__(self, memo: dict) -> DagWire: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class DagSwitchCase:
    """One switch-case body in a structured control-flow DAG payload."""

    @property
    def value(self) -> int:
        """Integer case value."""
        ...
    @property
    def body(self) -> CircuitDag:
        """Case body as an independent cloned DAG."""
        ...
    def __repr__(self) -> str: ...

class DagControlFlow:
    """Recursive DAG payload attached to a structured control-flow operation.

    Returned child DAGs are independent clones.  Mutating a child DAG returned
    by this object does not mutate the parent DAG that produced it.
    """

    @property
    def kind(self) -> Literal["if", "while", "for", "switch", "break", "continue"]:
        """Structured control-flow kind."""
        ...
    @property
    def then_body(self) -> CircuitDag | None:
        """Then branch for ``if`` control flow, otherwise ``None``."""
        ...
    @property
    def else_body(self) -> CircuitDag | None:
        """Else branch for ``if`` control flow, if present."""
        ...
    @property
    def body(self) -> CircuitDag | None:
        """Loop body for ``while`` and ``for`` control flow, otherwise ``None``."""
        ...
    @property
    def cases(self) -> list[DagSwitchCase]:
        """Switch case bodies, or an empty list for non-switch control flow."""
        ...
    @property
    def default_body(self) -> CircuitDag | None:
        """Switch default body, if present."""
        ...
    def __repr__(self) -> str: ...

class CircuitDag:
    """Operation-level dependency DAG analysis view for a circuit.

    ``CircuitDag`` is intended for analysis and compiler-facing queries.  The
    Python binding exposes construction from a circuit, traversal, dependency
    queries, layers, run collection, conversion back to a circuit, and targeted
    mutation APIs.

    Node IDs are invalidated by any mutation.  Re-query nodes after calling
    ``apply_operation_back``, ``apply_operation_front``, ``remove_op_node``,
    ``substitute_node``, or ``substitute_node_with_dag``.

    Use :meth:`Circuit.dag` for the common construction path::

        c = Circuit(2)
        c.h(0)
        c.cx(0, 1)
        dag = c.dag()

    Or use :meth:`from_circuit` explicitly::

        dag = CircuitDag.from_circuit(c)
    """

    @staticmethod
    def from_circuit(circuit: Circuit) -> CircuitDag:
        """Build a DAG from a circuit.

        Args:
            circuit: Source circuit whose operation dependencies should be
                materialized.

        Returns:
            A new dependency DAG.  Mutating the original circuit later does not
            update this DAG.
        """
        ...

    def to_circuit(self) -> Circuit:
        """Reconstruct a circuit from deterministic topological order.

        The returned circuit has the same dependencies but may use any
        deterministic topological order allowed by the DAG.
        """
        ...

    def validate(self) -> None:
        """Validate DAG graph and metadata invariants.

        Raises:
            CircuitError: If the DAG is cyclic, has broken wire endpoints, or
                references invalid circuit metadata.
        """
        ...

    @property
    def num_qubits(self) -> int:
        """Number of qubits tracked by the DAG."""
        ...
    @property
    def num_ops(self) -> int:
        """Number of operation nodes."""
        ...
    @property
    def is_empty(self) -> bool:
        """Whether the DAG has no operation nodes."""
        ...
    @property
    def qubits(self) -> list[Qubit]:
        """Qubits in circuit insertion order."""
        ...
    @property
    def parameters(self) -> list[Parameter]:
        """Parameter table used to resolve node operations."""
        ...
    @property
    def symbols(self) -> list[str]:
        """Names of symbolic parameters referenced by the DAG."""
        ...
    @property
    def classical_vars(self) -> list[ClassicalType]:
        """Types of mutable classical variables."""
        ...
    @property
    def classical_values(self) -> list[ClassicalType]:
        """Types of immutable classical values."""
        ...

    def wires(self) -> list[DagWire]:
        """Return all materialized DAG wires."""
        ...
    def has_wire(self, wire: DagWire) -> bool:
        """Return whether ``wire`` has materialized endpoints in the DAG."""
        ...
    def is_wire_idle(self, wire: DagWire) -> bool:
        """Return whether ``wire`` has no operation on its timeline."""
        ...
    def nodes_on_wire(self, wire: DagWire) -> list[int]:
        """Return operation node IDs on ``wire`` in wire order."""
        ...
    def wire_in(self, wire: DagWire) -> int | None:
        """Return the input sentinel node for ``wire``, if materialized."""
        ...
    def wire_out(self, wire: DagWire) -> int | None:
        """Return the output sentinel node for ``wire``, if materialized."""
        ...
    def is_operation(self, node: int) -> bool:
        """Return whether ``node`` is an operation node."""
        ...
    def node_kind(
        self, node: int
    ) -> Literal["operation", "wire_in", "wire_out"] | None:
        """Return the DAG node kind, or ``None`` if ``node`` is not in the graph."""
        ...

    def op_nodes(self) -> list[int]:
        """Return operation node IDs in deterministic source order."""
        ...
    def topological_op_nodes(self) -> list[int]:
        """Return operation node IDs in deterministic topological order."""
        ...
    def front_layer(self) -> list[int]:
        """Return operation nodes with no operation predecessor."""
        ...
    def layers(self) -> list[list[int]]:
        """Return ASAP operation layers.

        Each inner list contains operation node IDs that can be scheduled in
        the same dependency layer.
        """
        ...
    def node_layers(self) -> dict[int, int]:
        """Return the ASAP layer index for every operation node.

        Keys are inserted in deterministic topological order.
        """
        ...

    def predecessors(self, node: int) -> list[int]:
        """Return immediate predecessor node IDs, including wire sentinels.

        Use :meth:`is_operation` or :meth:`node_kind` to distinguish operation
        nodes from ``wire_in`` sentinels.
        """
        ...
    def successors(self, node: int) -> list[int]:
        """Return immediate successor node IDs, including wire sentinels.

        Use :meth:`is_operation` or :meth:`node_kind` to distinguish operation
        nodes from ``wire_out`` sentinels.
        """
        ...
    def predecessors_on_wire(self, node: int, wire: DagWire) -> list[int]:
        """Return operation predecessors of ``node`` connected through ``wire``."""
        ...
    def successors_on_wire(self, node: int, wire: DagWire) -> list[int]:
        """Return operation successors of ``node`` connected through ``wire``."""
        ...
    def quantum_predecessors(self, node: int) -> list[int]:
        """Return operation predecessors connected by qubit wires."""
        ...
    def quantum_successors(self, node: int) -> list[int]:
        """Return operation successors connected by qubit wires."""
        ...
    def classical_predecessors(self, node: int) -> list[int]:
        """Return operation predecessors connected by classical wires."""
        ...
    def classical_successors(self, node: int) -> list[int]:
        """Return operation successors connected by classical wires."""
        ...

    def operation(self, node: int) -> ValueOperation | None:
        """Return the operation at an operation node, or ``None`` for non-operation nodes."""
        ...
    def control_flow(self, node: int) -> DagControlFlow | None:
        """Return recursive control-flow DAG payload attached to ``node``, if any."""
        ...

    def depth(self) -> int:
        """Return the ASAP dependency depth of this DAG."""
        ...
    def has_control_flow(self) -> bool:
        """Return whether any top-level operation is structured control flow."""
        ...
    def has_nested_control_flow(self) -> bool:
        """Return whether a control-flow body contains structured control flow."""
        ...
    def has_measurement(self) -> bool:
        """Return whether any operation directly or recursively measures."""
        ...
    def operation_count_by_name(self) -> dict[str, int]:
        """Count top-level operations by instruction name.

        Keys are inserted in first-occurrence source order.
        """
        ...
    def operation_count_by_name_recursive(self) -> dict[str, int]:
        """Count top-level and nested control-flow body operations by name.

        Keys are inserted in first-occurrence traversal order.
        """
        ...
    def collect_1q_runs(self) -> list[list[int]]:
        """Collect contiguous one-qubit gate runs on every qubit wire."""
        ...
    def collect_2q_runs(self) -> list[list[int]]:
        """Collect contiguous two-qubit gate runs on every qubit wire."""
        ...
    def add_parameter(self, parameter: Parameter) -> tuple[int, bool]:
        """Add ``parameter`` to the DAG parameter table.

        Returns:
            ``(index, is_new)`` where ``index`` is the parameter-table index.
        """
        ...
    def apply_operation_back(self, operation: ValueOperation) -> int:
        """Append ``operation`` to the DAG and return its new node ID."""
        ...
    def apply_operation_front(self, operation: ValueOperation) -> int:
        """Prepend ``operation`` to the DAG and return its new node ID."""
        ...
    def remove_op_node(self, node: int) -> ValueOperation:
        """Remove one operation node and return the removed operation."""
        ...
    def substitute_node(self, node: int, operation: ValueOperation) -> None:
        """Replace one operation node with ``operation``."""
        ...
    def substitute_node_with_dag(self, node: int, replacement: CircuitDag) -> None:
        """Replace one operation node with another DAG."""
        ...
    def __len__(self) -> int: ...
    def __copy__(self) -> CircuitDag: ...
    def __deepcopy__(self, memo: dict) -> CircuitDag: ...
    def __repr__(self) -> str: ...
