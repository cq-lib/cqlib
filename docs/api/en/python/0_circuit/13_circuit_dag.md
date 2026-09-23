# Circuit DAG

`cqlib.circuit`

A `CircuitDag` is an algorithmic view of a circuit at the level of operation dependencies: a node represents one operation, and an edge represents the ordering between two operations because they share the same resource. The resources that can be shared include qubits, mutable classical variables, immutable classical values, and a global order used to carry operations that have no concrete data resource. This view does not change the storage structure of the circuit itself and is mainly used for dependency analysis, layered scheduling and subgraph replacement.

This page covers `CircuitDag`, `DagWire`, `DagControlFlow` and `DagSwitchCase`.

---

## Import

```python
from cqlib.circuit import CircuitDag, DagControlFlow, DagSwitchCase, DagWire
```

---

## CircuitDag

`CircuitDag` is the operation-level dependency graph of a circuit. Every operation in the graph corresponds to one operation node, and each wire additionally carries an input sentinel node and an output sentinel node.

### Constructing a circuit DAG

```python
CircuitDag.from_circuit(circuit: Circuit) -> CircuitDag
circuit.dag() -> CircuitDag
```

`CircuitDag.from_circuit()` is a static method that constructs a dependency graph from an existing circuit. `Circuit.dag()` is an equivalent entry that constructs the dependency graph directly on the current circuit.

After construction finishes, the DAG invariants are validated automatically. If the circuit itself does not satisfy the constraints, or an operation references a resource outside the circuit, construction raises `CircuitError`.

Example:

```python
from cqlib import Circuit, CircuitDag

circuit = Circuit(2)
circuit.h(0)
circuit.x(1)
circuit.cx(0, 1)

dag = circuit.dag()
same = CircuitDag.from_circuit(circuit)
```

### Exporting back to a circuit

```python
dag.to_circuit() -> Circuit
```

`to_circuit()` rebuilds a circuit in the deterministic topological order of the DAG. The returned circuit carries the qubits, parameter table, classical variables, classical values and global phase recorded in the DAG. If the DAG contains a cycle, references illegal parameters, or the rebuilt operation sequence fails the circuit construction checks, `CircuitError` is raised.

```python
recovered = dag.to_circuit()
print([operation.name for operation in recovered.operations])
```

### Validation

```python
dag.validate() -> None
```

`validate()` checks the consistency of the graph and its metadata, including: whether the graph contains a cycle, whether the input and output sentinels of each wire are legal, whether operation node indices are unique, whether the qubits and parameter indices referenced by operations are valid, whether classical variables and classical values belong to the current circuit, and whether control flow payloads match their owning operation.

After importing a DAG from external IR or constructing one by hand, it is advisable to call it explicitly once:

```python
dag.validate()
```

### Attributes

| Attribute | Type | Description |
| --- | --- | --- |
| `num_qubits` | `int` | The number of qubits. |
| `num_ops` | `int` | The number of operation nodes. |
| `is_empty` | `bool` | Whether there are no operation nodes. |
| `qubits` | `list[Qubit]` | Return the qubits in insertion order. |
| `parameters` | `list[Parameter]` | The parameter expressions in the parameter table. |
| `symbols` | `list[str]` | The free symbol names appearing in the circuit. |
| `classical_vars` | `list[ClassicalType]` | The allocated mutable classical variable types. |
| `classical_values` | `list[ClassicalType]` | The immutable classical value types, usually produced by measurement. |

```python
assert dag.num_qubits == 2
assert dag.num_ops == 3
assert len(dag) == 3
assert dag.is_empty is False
assert dag.qubits == circuit.qubits
```

### Other behavior

- `len(dag)` returns the number of operation nodes, the same as `num_ops`.
- `repr` has the form `CircuitDag(num_qubits=2, num_ops=3)`.
- Supports `copy` and `deepcopy`; the copied DAG has the same topological order as the original DAG.

---

## Node identifiers

On the Python side a node is represented by an `int`. Node identifiers are obtained from the following entries:

| Way to obtain | Description |
| --- | --- |
| `op_nodes()` | Return all operation nodes in circuit source order. |
| `topological_op_nodes()` | Return all operation nodes in deterministic topological order. |
| `front_layer()`, `layers()`, `node_layers()` | The nodes in the layering results. |
| `wire_in(wire)` / `wire_out(wire)` | The input and output sentinel nodes of a wire. |
| `apply_operation_front()` / `apply_operation_back()` | Return the new node identifier after appending an operation. |

A node is not necessarily an operation node. `is_operation()` and `node_kind()` can be used to determine the node kind:

```python
node = dag.topological_op_nodes()[0]

assert dag.is_operation(node)
assert dag.node_kind(node) == "operation"
assert dag.node_kind(dag.wire_in(DagWire.qubit(0))) == "wire_in"
```

The values of `node_kind()` are `"wire_in"`, `"wire_out"` and `"operation"`; it returns `None` when the node does not exist.

`operation(node)` returns the `ValueOperation` corresponding to that node, containing the instruction, the qubits acted on, the parameters and the label; it returns `None` when the node is not an operation node. When a node identifier does not belong to the current DAG, traversal-type methods raise `CircuitError`.

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

circuit = Circuit(1)
circuit.rx(0, theta)

dag = circuit.dag()
operation = dag.operation(dag.topological_op_nodes()[0])

assert operation.name == "RX"
assert operation.params[0] == theta
assert dag.parameters == [theta]
assert dag.symbols == ["theta"]
```

---

## DagWire

`DagWire` represents the resource carried by a DAG edge, and has four forms:

| Constructor | Description |
| --- | --- |
| `DagWire.qubit(qubit)` | A qubit wire; the argument is a logical qubit index `int` or a `Qubit` object. |
| `DagWire.classical_var(var)` | A mutable classical variable wire. |
| `DagWire.classical_value(value)` | An immutable classical value wire, usually a measurement result. |
| `DagWire.global_order()` | The global order, used to carry a stable order between operations that have no concrete data resource. |

All of the above are static methods.

### Attributes

| Attribute | Type | Description |
| --- | --- | --- |
| `kind` | `str` | The wire kind, whose value is `"qubit"`, `"classical_var"`, `"classical_value"` or `"global_order"`. |
| `qubit_value` | `Qubit \| None` | The qubit carried by a qubit wire; `None` for the other forms. |
| `classical_var_value` | `ClassicalVar \| None` | The variable carried by a classical variable wire; `None` for the other forms. |
| `classical_value_value` | `ClassicalValue \| None` | The value carried by a classical value wire; `None` for the other forms. |

### Other behavior

- Supports `==` and `hash`, and can be used as a dictionary key or a set element.
- Supports `copy` and `deepcopy`.
- `repr` has the form `DagWire.qubit(0)`.

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalType, DagWire

circuit = Circuit(1)
measured = circuit.measure(0)
flag = circuit.var(ClassicalType.bool())
circuit.store(flag, measured.expr().bit_to_bool())

dag = circuit.dag()

qubit_wire = DagWire.qubit(0)
value_wire = DagWire.classical_value(measured.value)
var_wire = DagWire.classical_var(flag)

assert qubit_wire.kind == "qubit"
assert qubit_wire.qubit_value == circuit.qubits[0]
assert value_wire.classical_value_value == measured.value
assert var_wire.classical_var_value == flag
assert DagWire.global_order().kind == "global_order"
```

---

## Wire queries

| Method | Description |
| --- | --- |
| `wires()` | Return all wires already materialized in the current DAG. |
| `has_wire(wire)` | Determine whether a wire is already materialized in the current DAG. |
| `is_wire_idle(wire)` | Determine whether a wire carries no operation at all. |
| `wire_in(wire)` | Return the identifier of the input sentinel node of the wire, or `None` if it does not exist. |
| `wire_out(wire)` | Return the identifier of the output sentinel node of the wire, or `None` if it does not exist. |
| `nodes_on_wire(wire)` | Return the operation nodes on the wire in wire order. |

When a wire is not a legal resource, or the timeline on the wire is broken or branches, `is_wire_idle()` and `nodes_on_wire()` raise `CircuitError`.

```python
nodes = dag.topological_op_nodes()
measure_node, store_node = nodes

assert dag.has_wire(value_wire)
assert dag.nodes_on_wire(value_wire) == [measure_node, store_node]
assert dag.wire_in(qubit_wire) in dag.predecessors(measure_node)
```

---

## Graph traversal

Predecessor and successor queries come in three levels: all neighbors regardless of resource, neighbors restricted to one wire, and neighbors classified by quantum or classical resource.

| Method | Description |
| --- | --- |
| `op_nodes()` | Return all operation nodes in source order. |
| `topological_op_nodes()` | Return all operation nodes in topological order. |
| `predecessors(node)` | Return all predecessor nodes, including sentinel nodes. |
| `successors(node)` | Return all successor nodes, including sentinel nodes. |
| `predecessors_on_wire(node, wire)` | Return the operation predecessors connected only through the given wire. |
| `successors_on_wire(node, wire)` | Return the operation successors connected only through the given wire. |
| `quantum_predecessors(node)` | Return the operation predecessors connected through qubit wires. |
| `quantum_successors(node)` | Return the operation successors connected through qubit wires. |
| `classical_predecessors(node)` | Return the operation predecessors connected through classical variable wires or classical value wires. |
| `classical_successors(node)` | Return the operation successors connected through classical variable wires or classical value wires. |

The neighbors returned by `predecessors()` and `successors()` may include wire sentinel nodes; the other methods return only operation nodes. Methods with a `wire` argument raise `CircuitError` when that wire does not belong to the current DAG; they likewise raise `CircuitError` when the node identifier does not exist.

```python
assert dag.quantum_successors(measure_node) == []
assert dag.classical_predecessors(store_node) == [measure_node]
assert dag.predecessors_on_wire(store_node, value_wire) == [measure_node]
```

---

## Layers and depth

| Method | Description |
| --- | --- |
| `front_layer()` | Return the operation nodes with no operation predecessor, that is, the earliest executable layer. |
| `layers()` | Return the layering result in as-soon-as-possible scheduling, where each layer is a list of operation nodes. |
| `node_layers()` | Return the mapping from each operation node to the layer index it belongs to. |
| `depth()` | Return the dependency depth, that is, the number of layers. |

`layers()` and `node_layers()` use the same set of layer indices, and `depth()` equals the maximum layer index plus one. When the graph contains a cycle, or the layer index of an operation predecessor is inconsistent with the topological order, `CircuitError` is raised.

```python
circuit = Circuit(2)
circuit.h(0)
circuit.x(1)
circuit.cx(0, 1)
dag = circuit.dag()

assert len(dag.front_layer()) == 2
assert [len(layer) for layer in dag.layers()] == [2, 1]
assert dag.depth() == 2
```

The dictionary returned by `node_layers()` is arranged in topological order, with node identifiers as keys and layer indices as values.

```python
assert list(dag.node_layers().keys()) == dag.topological_op_nodes()
```

---

## Control flow

When a circuit contains structured control flow, a control flow operation node additionally carries a recursive DAG payload. The construction interfaces and low-level IR of control flow itself are described in [Classical Data / Control Flow](9_classical_control_flow.md).

| Method | Description |
| --- | --- |
| `control_flow(node)` | Return the `DagControlFlow` carried by that node, or `None` if there is none. |
| `has_control_flow()` | Determine whether structured control flow exists among the top-level operations. |
| `has_nested_control_flow()` | Determine whether structured control flow is further contained inside a control flow body. |
| `has_measurement()` | Determine whether measurement is contained directly or recursively among the top-level operations. |

### DagControlFlow

The `kind` attribute gives the control flow form, whose value is `"if"`, `"while"`, `"for"`, `"switch"`, `"break"` or `"continue"`. The attributes exposed by each form are as follows; an attribute that does not apply to that form returns `None` or an empty list.

| Attribute | Applicable form | Type | Description |
| --- | --- | --- | --- |
| `kind` | All | `str` | The control flow form. |
| `then_body` | `if` | `CircuitDag \| None` | The branch body executed when the condition holds. |
| `else_body` | `if` | `CircuitDag \| None` | The `else` branch body; `None` if there is no such branch. |
| `body` | `while`, `for` | `CircuitDag \| None` | The loop body. |
| `cases` | `switch` | `list[DagSwitchCase]` | The exact integer match branches. |
| `default_body` | `switch` | `CircuitDag \| None` | The `default` branch body; `None` if there is no such branch. |

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalType

circuit = Circuit(1)
flag = circuit.var(ClassicalType.bool())
circuit.if_else(
    flag.expr(),
    lambda body: body.x(0),
    lambda body: body.z(0),
)

dag = circuit.dag()
control = dag.control_flow(dag.topological_op_nodes()[0])

assert control.kind == "if"
assert [op.name for op in control.then_body.to_circuit().operations] == ["X"]
assert [op.name for op in control.else_body.to_circuit().operations] == ["Z"]
```

### DagSwitchCase

`DagSwitchCase` represents an exact integer match branch in a `switch`.

| Attribute | Type | Description |
| --- | --- | --- |
| `value` | `int` | The integer value this branch matches. |
| `body` | `CircuitDag` | The control flow body executed when that branch is taken. |

```python
circuit = Circuit(1)
target = circuit.var(ClassicalType.uint(8))

circuit.switch(
    target.expr(),
    lambda case: (
        case.value(1, lambda body: body.x(0)),
        case.value(2, lambda body: body.y(0)),
        case.default(lambda body: body.z(0)),
    ),
)

dag = circuit.dag()
control = dag.control_flow(dag.topological_op_nodes()[0])

assert control.kind == "switch"
assert [case.value for case in control.cases] == [1, 2]
assert [op.name for op in control.cases[0].body.to_circuit().operations] == ["X"]
assert [op.name for op in control.default_body.to_circuit().operations] == ["Z"]
```

A control flow body is itself a `CircuitDag`, so interfaces such as `to_circuit()`, `layers()` and `control_flow()` can be called on it further for recursive analysis of nested structures.

---

## Operation counts and runs

| Method | Description |
| --- | --- |
| `operation_count_by_name()` | Count the top-level operations by instruction name. |
| `operation_count_by_name_recursive()` | Count the operations among the top level and inside each control flow body by instruction name. |
| `collect_1q_runs()` | Collect contiguous single-qubit gate runs on each qubit wire. |
| `collect_2q_runs()` | Collect contiguous two-qubit gate runs on each qubit wire. |

The counting result is returned as a dictionary whose keys are instruction names and whose values are counts. Runs are returned as a list of lists, in which each element is a sequence of contiguous gate node identifiers, in the same order as on the wire. Non-gate operations, measurement, reset, barrier and multi-qubit operations interrupt a single-qubit gate run; the same two-qubit operation may appear in the runs of both of its qubit wires at the same time.

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr, ClassicalType

circuit = Circuit(2)
circuit.h(0)
circuit.x(0)
circuit.cx(0, 1)
circuit.cz(0, 1)
counter = circuit.var(ClassicalType.uint(8))
circuit.for_uint(
    counter,
    ClassicalExpr.uint_literal(8, 0),
    ClassicalExpr.uint_literal(8, 2),
    ClassicalExpr.uint_literal(8, 1),
    lambda body, _: body.z(1),
)

dag = circuit.dag()
top_counts = dag.operation_count_by_name()
recursive_counts = dag.operation_count_by_name_recursive()
oneq_runs = dag.collect_1q_runs()
twoq_runs = dag.collect_2q_runs()

assert dag.has_control_flow()
assert dag.has_nested_control_flow() is False
assert top_counts["H"] == 1
assert top_counts["for"] == 1
assert "Z" not in top_counts
assert recursive_counts["Z"] == 1
assert any(len(run) == 2 for run in oneq_runs)
assert any(len(run) == 2 for run in twoq_runs)
```

---

## Modifying the graph

| Method | Description |
| --- | --- |
| `add_parameter(parameter)` | Insert a parameter into the DAG parameter table and return `(index, is_new)`. |
| `apply_operation_front(operation)` | Append an operation at the front of the DAG and return the new node identifier. |
| `apply_operation_back(operation)` | Append an operation at the back of the DAG and return the new node identifier. |
| `remove_op_node(node)` | Delete an operation node and return the deleted `ValueOperation`. |
| `substitute_node(node, operation)` | Replace the given operation node with one `ValueOperation`. |
| `substitute_node_with_dag(node, replacement)` | Replace the given operation node with another `CircuitDag`. |

What is passed in when appending an operation is a self-contained `ValueOperation`. When replacing a control flow node, the replacement content must contain only one control flow operation of the same type; the resources read or written by a replacement DAG must fall within the resource range of the replaced node, otherwise `CircuitError` is raised.

```python
from cqlib import Circuit
from cqlib.circuit import StandardGate, ValueOperation

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)
dag = circuit.dag()
q0, q1 = dag.qubits

front = ValueOperation.from_standard_gate(StandardGate.X, [q1])
back = ValueOperation.from_standard_gate(StandardGate.Z, [q0])
front_node = dag.apply_operation_front(front)
back_node = dag.apply_operation_back(back)

assert dag.operation(front_node).name == "X"
assert dag.operation(back_node).name == "Z"
assert [op.name for op in dag.to_circuit().operations] == ["X", "H", "CX", "Z"]

nodes = dag.topological_op_nodes()
removed = dag.remove_op_node(nodes[1])
assert removed.name == "H"
assert [op.name for op in dag.to_circuit().operations] == ["X", "CX", "Z"]

nodes = dag.topological_op_nodes()
replacement = ValueOperation.from_standard_gate(StandardGate.Y, [q1])
dag.substitute_node(nodes[0], replacement)
assert [op.name for op in dag.to_circuit().operations] == ["Y", "CX", "Z"]
```

Replacing one node with a whole subgraph:

```python
circuit = Circuit(1)
circuit.h(0)
dag = circuit.dag()

replacement_circuit = Circuit(1)
replacement_circuit.x(0)
replacement_circuit.z(0)
replacement = replacement_circuit.dag()

dag.substitute_node_with_dag(dag.topological_op_nodes()[0], replacement)

assert [op.name for op in dag.to_circuit().operations] == ["X", "Z"]
```

---

## Typical use

Compiler passes and routing algorithms usually perform dependency analysis on the DAG: `front_layer()` and `node_layers()` determine which operations are already ready, `predecessors()` and `successors()` determine whether an ordering constraint exists between two operations, run collection such as `collect_1q_runs()` locates gate sequences that can be processed as a whole, and the `substitute_node()` series of interfaces writes the transformation result back into the graph.

---

## Exceptions

- `CircuitError`: a node identifier does not belong to the current DAG, a wire does not belong to the current DAG, the graph contains a cycle, the timeline of a wire is broken or branches, a control flow payload does not match its owning operation, the resource range or control flow form of a replacement DAG is illegal, a parameter index is out of range, a classical variable or classical value belongs to another circuit, and so on.
