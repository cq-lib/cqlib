# Circuit DAG

`cqlib_core::circuit`

```rust
use cqlib_core::circuit::{CircuitDag, DagControlFlow, DagNode, DagSwitchCase, DagWire};
```

`CircuitDag` is the algorithmic view of a circuit at the operation dependency level: nodes represent operations, and edges represent the precedence between two operations that arises from sharing the same resource. The shared resources include qubits, mutable classical variables, immutable classical values, and a global order used to carry operations that have no concrete data resource. This view does not change the storage structure of the circuit itself, and is mainly used for dependency analysis, layered scheduling and subgraph replacement.

This page covers `CircuitDag`, `DagWire`, `DagNode`, `DagControlFlow` and `DagSwitchCase`.

---

## Core concepts

| Term | Description |
| --- | --- |
| `CircuitDag` | The operation-level dependency graph of a circuit; besides operation nodes, each wire also has input and output sentinel nodes. |
| `DagWire` | The resource an edge carries; its value is a qubit, a classical variable, a classical value or the global order. |
| `DagNode` | The node kind, divided into input sentinel, output sentinel and operation. |
| `DagControlFlow` | The recursive DAG payload carried by a structured control flow operation. |
| `DagSwitchCase` | A single exact integer match branch in a `switch`. |

---

## CircuitDag

### Construction

```rust
pub fn from_circuit(circuit: &Circuit) -> Result<Self, CircuitError>
```

`CircuitDag::from_circuit()` constructs a dependency graph from an existing circuit, copying the circuit's qubits, parameter table, symbol names, classical variable and classical value type tables, and the global phase. `validate()` is called automatically after construction finishes.

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let dag = CircuitDag::from_circuit(&circuit)?;

assert_eq!(dag.num_qubits(), 2);
assert_eq!(dag.num_ops(), 2);
assert_eq!(dag.depth()?, 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

```rust
pub fn from_operations(
    qubits: impl IntoIterator<Item = Qubit>,
    operations: &[Operation],
) -> Result<Self, CircuitError>
```

`CircuitDag::from_operations()` constructs a dependency graph from a self-contained slice of operations, and is the narrow entry point of this view for local pass analysis. It accepts no parameter table, classical declarations or global phase: every operation must be valid with respect to exactly the given qubit set, and must not depend on any circuit-level metadata. When the operation flow contains indexed parameters, classical variables, classical values or control flow bodies, `from_circuit()` should be used instead; an analysis pass that intentionally ignores these resources should normalize them first, for example by resolving parameters or by replacing non-quantum operations with dependency barriers.

An error is returned when duplicate qubits appear in `qubits`, when an operation references a qubit outside the set, or when the operation slice is not self-contained.

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let dag = CircuitDag::from_operations(circuit.qubits(), circuit.operations())?;

assert_eq!(dag.num_ops(), 2);
assert_eq!(dag.parameters().len(), 0);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### Exporting back to a circuit

```rust
pub fn to_circuit(&self) -> Result<Circuit, CircuitError>
```

`to_circuit()` reconstructs the circuit in the deterministic topological order of the DAG, and restores the qubit set, the classical type tables and the global phase. An error is returned when the graph contains a cycle, when an invalid parameter index is referenced, or when the reconstructed operation sequence is rejected by `Circuit::from_operations()`.

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, ParameterValue, Qubit};

let mut circuit = Circuit::new(1);
circuit.rx(Qubit::new(0), ParameterValue::from("theta"))?;

let dag = CircuitDag::from_circuit(&circuit)?;
let recovered = dag.to_circuit()?;

assert_eq!(recovered.operations().len(), 1);
assert!(dag.symbols().contains("theta"));

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### Validation

```rust
pub fn validate(&self) -> Result<(), CircuitError>
```

`validate()` checks the consistency of the graph and the metadata, including: whether the graph contains a cycle, whether the input and output sentinels of each wire are valid, whether operation node numbers are unique and consistent with the order index, whether the qubits and parameter indices referenced by operations are valid, whether classical variables and classical values belong to the current circuit, whether control flow payloads match the owning operation, and whether edge resources are valid.

A DAG that was imported externally, generated automatically or modified manually should be validated explicitly before entering later stages:

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;
circuit.measure(Qubit::new(1))?;

let dag = CircuitDag::from_circuit(&circuit)?;
dag.validate()?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### Type conversion

`TryFrom` conversions between the DAG and the circuit are also provided, with semantics identical to `from_circuit()` and `to_circuit()`.

| Implementation | Description |
| --- | --- |
| `impl TryFrom<&Circuit> for CircuitDag` | Equivalent to `CircuitDag::from_circuit()`. |
| `impl TryFrom<&CircuitDag> for Circuit` | Equivalent to `CircuitDag::to_circuit()`. |

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let dag = CircuitDag::try_from(&circuit)?;
let recovered = Circuit::try_from(&dag)?;
assert_eq!(recovered, circuit);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### Metadata and statistics

| Method | Returns | Description |
| --- | --- | --- |
| `num_qubits()` | `usize` | The number of qubits. |
| `num_ops()` | `usize` | The number of operation nodes. |
| `is_empty()` | `bool` | Whether there are no operation nodes. |
| `qubits()` | `Vec<Qubit>` | Return the qubits in insertion order. |
| `parameters()` | `&IndexSet<Parameter>` | The parameter table. |
| `symbols()` | `&IndexSet<String>` | The set of free symbol names. |
| `classical_vars()` | `&[ClassicalType]` | The mutable classical variable type table. |
| `classical_values()` | `&[ClassicalType]` | The immutable classical value type table. |
| `add_parameter(param)` | `(usize, bool)` | Make a parameter resident in the parameter table, returning its index in the table and whether it was newly inserted. |

The behavior of `add_parameter()` is identical to `Circuit::add_parameter()`: insert the parameter when it is not present, and register all of its symbol names.

```rust
use cqlib_core::circuit::{Circuit, ClassicalType, CircuitDag, Qubit};

let mut circuit = Circuit::new(1);
let _flag = circuit.var(ClassicalType::Bool);
let _value = circuit.measure(Qubit::new(0))?;

let dag = CircuitDag::from_circuit(&circuit)?;

assert_eq!(dag.classical_vars(), circuit.classical_vars());
assert_eq!(dag.classical_values(), circuit.classical_values());

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### Attributes

- Implements `Debug` and `Clone`.

---

## DagWire

```rust
pub enum DagWire {
    Qubit(Qubit),
    ClassicalVar(ClassicalVar),
    ClassicalValue(ClassicalValue),
    GlobalOrder,
}
```

`DagWire` is the resource an edge carries. It is also the identity of a wire: each wire corresponds to one input sentinel node and one output sentinel node in the graph.

| Variant | Description |
| --- | --- |
| `DagWire::Qubit(Qubit)` | A qubit wire. |
| `DagWire::ClassicalVar(ClassicalVar)` | A mutable classical variable wire, written by `store`. |
| `DagWire::ClassicalValue(ClassicalValue)` | An immutable classical value wire, usually produced by measurement. |
| `DagWire::GlobalOrder` | The global order, used to carry a stable order between operations that have no concrete data resource. |

`DagWire` implements `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` and `Hash`.

### Wire queries

| Method | Returns | Description |
| --- | --- | --- |
| `wires()` | `impl Iterator<Item = DagWire>` | Iterate over all wires materialized in the current DAG. |
| `has_wire(wire)` | `bool` | Determine whether a wire is already materialized in the current DAG. |
| `is_wire_idle(wire)` | `Result<bool, CircuitError>` | Determine whether a wire has no operation on it. |
| `wire_in(wire)` | `Option<NodeIndex>` | Return the input sentinel node of the wire. |
| `wire_out(wire)` | `Option<NodeIndex>` | Return the output sentinel node of the wire. |
| `nodes_on_wire(wire)` | `Result<Vec<NodeIndex>, CircuitError>` | Return the operation nodes on the wire in wire order. |

`is_wire_idle()` and `nodes_on_wire()` return an error when the wire is not a valid resource of the current DAG, or when the timeline on the wire is broken or forked.

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, DagWire, Qubit};

let mut circuit = Circuit::new(1);
circuit.h(Qubit::new(0))?;
circuit.x(Qubit::new(0))?;
circuit.z(Qubit::new(0))?;

let dag = CircuitDag::from_circuit(&circuit)?;
let nodes = dag.nodes_on_wire(DagWire::Qubit(Qubit::new(0)))?;

assert_eq!(nodes.len(), 3);
assert_eq!(dag.num_ops(), 3);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

Qubit wires are all materialized at construction time, whereas classical variable wires and classical value wires are materialized only when referenced by an operation. The unwritten classical variable wire below is neither in `wires()` nor involved in any dependency; `DagWire::GlobalOrder` always exists, and is used to carry operations that do not involve any concrete data resource.

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, ClassicalType, DagWire, Qubit};

let mut circuit = Circuit::new(2);
let unused_var = circuit.var(ClassicalType::Bool);
circuit.h(Qubit::new(1))?;

let dag = CircuitDag::from_circuit(&circuit)?;

assert!(dag.has_wire(DagWire::Qubit(Qubit::new(0))));
assert!(!dag.has_wire(DagWire::ClassicalVar(unused_var)));
assert!(dag.is_wire_idle(DagWire::Qubit(Qubit::new(0)))?);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## DagNode

```rust
pub enum DagNode {
    WireIn(DagWire),
    WireOut(DagWire),
    Operation {
        operation: Operation,
        order: usize,
    },
}
```

`DagNode` is the node payload in the graph. Each operation node stores a storage-layer operation and a source order position, which is used for deterministic traversal.

| Variant | Description |
| --- | --- |
| `DagNode::WireIn(DagWire)` | The input sentinel of a wire. |
| `DagNode::WireOut(DagWire)` | The output sentinel of a wire. |
| `DagNode::Operation { operation, order }` | An operation, where `order` is its source order position. |

The node kind can be queried with `node_kind()`, which returns `"wire_in"`, `"wire_out"` or `"operation"`, and returns `None` when the node does not exist.

---

## Node identifiers

The node identifier type is `NodeIndex`, provided by `rustworkx_core::petgraph::prelude::NodeIndex`, and its `usize` value can be retrieved with `node.index()`. Node identifiers are obtained from the following entry points:

| How obtained | Description |
| --- | --- |
| `op_nodes()` | Iterate over all operation nodes in source order. |
| `topological_op_nodes()` | Return all operation nodes in topological order. |
| `front_layer()`, `layers()`, `node_layers()` | The nodes in the layering results. |
| `wire_in()` / `wire_out()` | The input and output sentinel nodes of a wire. |
| `apply_operation_back()` and other mutating methods | Return the new node identifier. |

| Method | Returns | Description |
| --- | --- | --- |
| `operation(node)` | `Option<&Operation>` | Return the storage-layer operation corresponding to the node; `None` when the node is not an operation node. |
| `is_operation(node)` | `bool` | Determine whether the node is an operation node. |
| `node_kind(node)` | `Option<&'static str>` | Return the node kind. |
| `control_flow(node)` | `Option<&DagControlFlow>` | Return the control flow payload carried by the node; `None` when there is none. |
| `op_nodes()` | `impl Iterator<Item = NodeIndex>` | Iterate over operation nodes in source order. |
| `topological_op_nodes()` | `Result<Vec<NodeIndex>, CircuitError>` | Return operation nodes in deterministic topological order. |

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, DagWire, Instruction, Qubit, StandardGate};

let mut circuit = Circuit::new(1);
circuit.h(Qubit::new(0))?;

let dag = CircuitDag::from_circuit(&circuit)?;
let node = dag.topological_op_nodes()?[0];
let operation = dag.operation(node).expect("operation node");

assert!(matches!(
    operation.instruction,
    Instruction::Standard(StandardGate::H)
));
assert_eq!(operation.qubits[0], Qubit::new(0));

let wire_in = dag.wire_in(DagWire::Qubit(Qubit::new(0))).expect("qubit wire");
assert!(dag.operation(wire_in).is_none());

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Graph traversal

Predecessor and successor queries come in three levels: all neighbors regardless of resource, neighbors restricted to a single wire, and neighbors classified by quantum or classical resource.

| Method | Returns | Description |
| --- | --- | --- |
| `predecessors(node)` | `impl Iterator<Item = NodeIndex>` | Iterate over all predecessor nodes, which may include sentinel nodes. |
| `successors(node)` | `impl Iterator<Item = NodeIndex>` | Iterate over all successor nodes, which may include sentinel nodes. |
| `predecessors_on_wire(node, wire)` | `Result<Vec<NodeIndex>, CircuitError>` | Operation predecessors connected only through the specified wire. |
| `successors_on_wire(node, wire)` | `Result<Vec<NodeIndex>, CircuitError>` | Operation successors connected only through the specified wire. |
| `quantum_predecessors(node)` | `Result<Vec<NodeIndex>, CircuitError>` | Operation predecessors connected through qubit wires. |
| `quantum_successors(node)` | `Result<Vec<NodeIndex>, CircuitError>` | Operation successors connected through qubit wires. |
| `classical_predecessors(node)` | `Result<Vec<NodeIndex>, CircuitError>` | Operation predecessors connected through classical variable wires or classical value wires. |
| `classical_successors(node)` | `Result<Vec<NodeIndex>, CircuitError>` | Operation successors connected through classical variable wires or classical value wires. |

The neighbors returned by `predecessors()` and `successors()` may include wire sentinel nodes, and `operation(node).is_some()` can be used to filter them out when needed. The remaining methods return only operation nodes, with the results arranged in topological order. An error is returned when a node identifier does not exist, or when a wire is not a valid resource of the current DAG.

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, DagWire, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.x(Qubit::new(1))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;
circuit.z(Qubit::new(0))?;
circuit.h(Qubit::new(1))?;

let dag = CircuitDag::from_circuit(&circuit)?;
let nodes = dag.topological_op_nodes()?;
let (h0, x1, cx, z0, h1) = (nodes[0], nodes[1], nodes[2], nodes[3], nodes[4]);

assert_eq!(
    dag.predecessors_on_wire(cx, DagWire::Qubit(Qubit::new(0)))?,
    vec![h0]
);
assert_eq!(
    dag.successors_on_wire(cx, DagWire::Qubit(Qubit::new(1)))?,
    vec![h1]
);
assert_eq!(dag.quantum_predecessors(cx)?, vec![h0, x1]);
assert_eq!(dag.quantum_successors(cx)?, vec![z0, h1]);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

Dependencies on classical resources use the same set of interfaces:

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, ClassicalExpr, ClassicalType, Qubit};

let mut circuit = Circuit::new(1);
let measured = circuit.measure(Qubit::new(0))?;
let dest = circuit.var(ClassicalType::Bool);
circuit.store(dest, ClassicalExpr::bit_to_bool(measured.expr())?)?;

let dag = CircuitDag::from_circuit(&circuit)?;
let nodes = dag.topological_op_nodes()?;
let (measure, store) = (nodes[0], nodes[1]);

assert_eq!(dag.classical_predecessors(store)?, vec![measure]);
assert!(dag.quantum_predecessors(store)?.is_empty());

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Layers and depth

| Method | Returns | Description |
| --- | --- | --- |
| `front_layer()` | `Result<Vec<NodeIndex>, CircuitError>` | The operation nodes with no operation predecessors, that is, the earliest executable layer. |
| `layers()` | `Result<Vec<Vec<NodeIndex>>, CircuitError>` | The operation nodes layered by earliest scheduling. |
| `node_layers()` | `Result<IndexMap<NodeIndex, usize>, CircuitError>` | The mapping from each operation node to its layer number. |
| `depth()` | `Result<usize, CircuitError>` | The dependency depth, that is, the number of layers. |

`layers()` and `node_layers()` use the same set of layer numbers: the layer number of a layer is the maximum layer number among all of its operation predecessors plus one. `depth()` equals the maximum layer number plus one, and is `0` for an empty graph. An error is returned when the graph contains a cycle, or when the layer numbers of operation predecessors are inconsistent with the topological order.

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.x(Qubit::new(1))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let dag = CircuitDag::from_circuit(&circuit)?;
let layers = dag.layers()?;

assert_eq!(layers.len(), 2);
assert_eq!(layers[0].len(), 2);
assert_eq!(layers[1].len(), 1);
assert_eq!(dag.front_layer()?.len(), 2);
assert_eq!(dag.depth()?, 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

The mapping returned by `node_layers()` agrees with `layers()`, and can be used to look up the layer number by node.

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.x(Qubit::new(1))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let dag = CircuitDag::from_circuit(&circuit)?;
let node_layers = dag.node_layers()?;

assert_eq!(node_layers.len(), dag.num_ops());
for (layer_index, layer) in dag.layers()?.iter().enumerate() {
    for node in layer {
        assert_eq!(node_layers.get(node).copied(), Some(layer_index));
    }
}

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Control flow and measurement

When a circuit contains structured control flow, the control flow operation nodes additionally carry a recursive DAG payload. For the construction interfaces of control flow itself and the low-level IR, see [Classical Data / Control Flow](9_classical_control_flow.md).

| Method | Returns | Description |
| --- | --- | --- |
| `has_control_flow()` | `bool` | Whether structured control flow exists among the top-level operations. |
| `has_nested_control_flow()` | `bool` | Whether a control flow body still contains structured control flow inside it. |
| `has_measurement()` | `bool` | Whether the top-level operations contain measurement directly or recursively. |
| `control_flow(node)` | `Option<&DagControlFlow>` | Return the control flow payload carried by the node. |

Both `has_control_flow()` and `has_measurement()` traverse only the top-level operation nodes; the difference is that `has_measurement()` recurses inside each operation, so measurements inside control flow bodies are counted as well. `has_nested_control_flow()` goes deeper into control flow bodies to determine whether there is still control flow inside them.

### DagControlFlow

```rust
pub enum DagControlFlow {
    If {
        then_body: Box<CircuitDag>,
        else_body: Option<Box<CircuitDag>>,
    },
    While { body: Box<CircuitDag> },
    For { body: Box<CircuitDag> },
    Switch {
        cases: Vec<DagSwitchCase>,
        default: Option<Box<CircuitDag>>,
    },
    Break,
    Continue,
}
```

| Variant | Description |
| --- | --- |
| `If { then_body, else_body }` | The `if` construct; `else_body` is `None` when that branch is absent. |
| `While { body }` | A `while` loop. |
| `For { body }` | A half-open interval loop based on a `UInt` variable. |
| `Switch { cases, default }` | A `switch` multi-way selection. |
| `Break` | A break jump, with no subgraph. |
| `Continue` | A continue jump, with no subgraph. |

Each branch body is itself a `CircuitDag`, so interfaces such as `num_ops()`, `topological_op_nodes()` and `control_flow()` can be called on it for recursive analysis.

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, ClassicalExpr, DagControlFlow, Qubit};

let mut circuit = Circuit::new(2);
let measured = circuit.measure(Qubit::new(0))?;
let condition = ClassicalExpr::bit_to_bool(measured.expr())?;
circuit.if_(condition, |body| {
    body.x(Qubit::new(1))?;
    Ok(())
})?;

let dag = CircuitDag::from_circuit(&circuit)?;
let control_node = dag.topological_op_nodes()?[1];

match dag.control_flow(control_node).expect("if payload") {
    DagControlFlow::If {
        then_body,
        else_body,
    } => {
        assert_eq!(then_body.num_ops(), 1);
        assert!(else_body.is_none());
    }
    _ => panic!("expected if control-flow payload"),
}

assert!(dag.has_control_flow());
assert!(!dag.has_nested_control_flow());

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

Loop bodies are obtained in the same way. The `For` variant stores only the loop body, not the loop variable, the bounds or the step; that information remains in the `ClassicalControlOp::For` of the owning operation.

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, ClassicalExpr, ClassicalType, DagControlFlow, Qubit};

let mut circuit = Circuit::new(1);
let counter = circuit.var(ClassicalType::uint(8).expect("valid width"));
circuit.for_uint(
    counter,
    ClassicalExpr::uint_literal(8, 0)?,
    ClassicalExpr::uint_literal(8, 3)?,
    ClassicalExpr::uint_literal(8, 1)?,
    |body, _| {
        body.h(Qubit::new(0))?;
        Ok(())
    },
)?;

let dag = CircuitDag::from_circuit(&circuit)?;
let control_node = dag
    .topological_op_nodes()?
    .into_iter()
    .find(|node| dag.control_flow(*node).is_some())
    .expect("control-flow node");

match dag.control_flow(control_node).expect("for payload") {
    DagControlFlow::For { body } => {
        assert_eq!(body.num_ops(), 1);
    }
    _ => panic!("expected for control-flow payload"),
}

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### DagSwitchCase

```rust
pub struct DagSwitchCase {
    pub value: u128,
    pub body: Box<CircuitDag>,
}
```

| Field | Type | Description |
| --- | --- | --- |
| `value` | `u128` | The integer value this branch matches. |
| `body` | `Box<CircuitDag>` | The control flow body executed when this branch is hit. |

```rust
use cqlib_core::circuit::{Circuit, ClassicalType, CircuitDag, DagControlFlow, Qubit};

let mut circuit = Circuit::new(1);
let state = circuit.var(ClassicalType::uint(2).expect("valid width"));
circuit.switch(state.expr(), |cases| {
    cases.value(0, |body| {
        body.x(Qubit::new(0))?;
        Ok(())
    })?;
    cases.value(1, |body| {
        body.z(Qubit::new(0))?;
        Ok(())
    })?;
    cases.default(|body| {
        body.h(Qubit::new(0))?;
        Ok(())
    })?;
    Ok(())
})?;

let dag = CircuitDag::from_circuit(&circuit)?;
let control_node = dag
    .topological_op_nodes()?
    .into_iter()
    .find(|node| dag.control_flow(*node).is_some())
    .expect("switch node");

match dag.control_flow(control_node).expect("switch payload") {
    DagControlFlow::Switch { cases, default } => {
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].value, 0);
        assert_eq!(cases[0].body.num_ops(), 1);
        assert!(default.is_some());
    }
    _ => panic!("expected switch control-flow payload"),
}

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Operation counts and runs

| Method | Returns | Description |
| --- | --- | --- |
| `operation_count_by_name()` | `IndexMap<String, usize>` | Count top-level operations by instruction name. |
| `operation_count_by_name_recursive()` | `IndexMap<String, usize>` | Count operations by instruction name at the top level and inside each control flow body. |
| `collect_runs_on_wire(wire, predicate)` | `Result<Vec<Vec<NodeIndex>>, CircuitError>` | Collect the runs of consecutive operations accepted by `predicate` on the specified wire. |
| `collect_1q_runs()` | `Result<Vec<Vec<NodeIndex>>, CircuitError>` | Collect runs of consecutive single-qubit gates on each qubit wire. |
| `collect_2q_runs()` | `Result<Vec<Vec<NodeIndex>>, CircuitError>` | Collect runs of consecutive two-qubit gates on each qubit wire. |

The counting results preserve insertion order, with instruction names as keys. Runs are collected on a wire in source order: an operation for which `predicate` returns `false` breaks the current run.

`collect_runs_on_wire()` targets concrete qubit wires and classical wires; `DagWire::GlobalOrder` is also accepted, but it only orders operations that have no concrete data resource and is usually not suitable as the basis for dividing optimization runs.

The internal criterion of `collect_1q_runs()` and `collect_2q_runs()` is: the operation acts on one or two qubits, and the instruction is a standard gate, a multi-controlled gate, a custom unitary gate or a composite gate. Non-gate operations, measurement, reset, barrier and multi-qubit operations interrupt a run. The same two-qubit operation may appear in the runs of both of its qubit wires.

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, DagWire, Instruction, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.x(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;
circuit.h(Qubit::new(0))?;
circuit.reset(Qubit::new(0))?;
circuit.z(Qubit::new(0))?;
circuit.x(Qubit::new(1))?;

let dag = CircuitDag::from_circuit(&circuit)?;
let q0_runs = dag.collect_runs_on_wire(DagWire::Qubit(Qubit::new(0)), |operation| {
    matches!(operation.instruction, Instruction::Standard(_)) && operation.qubits.len() == 1
})?;

assert_eq!(
    q0_runs.iter().map(Vec::len).collect::<Vec<_>>(),
    vec![2, 1, 1]
);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Modifying the graph

| Method | Returns | Description |
| --- | --- | --- |
| `apply_operation_back(operation)` | `Result<NodeIndex, CircuitError>` | Append a storage-layer operation to the back of the DAG. |
| `apply_value_operation_back(operation)` | `Result<NodeIndex, CircuitError>` | Lower first, then append a construction-layer operation to the back of the DAG. |
| `apply_operation_front(operation)` | `Result<NodeIndex, CircuitError>` | Append a storage-layer operation to the front of the DAG. |
| `apply_value_operation_front(operation)` | `Result<NodeIndex, CircuitError>` | Lower first, then append a construction-layer operation to the front of the DAG. |
| `remove_op_node(node)` | `Result<Operation, CircuitError>` | Remove an operation node and return the removed operation. |
| `substitute_node(node, operation)` | `Result<(), CircuitError>` | Replace the specified node with a storage-layer operation. |
| `substitute_value_node(node, operation)` | `Result<(), CircuitError>` | Lower first, then replace the specified node with a construction-layer operation. |
| `substitute_node_with_dag(node, replacement)` | `Result<(), CircuitError>` | Replace the specified node with another `CircuitDag`. |

`apply_value_operation_*` and `substitute_value_node()` make the symbolic parameters in a `ValueOperation` resident in the DAG parameter table, so `symbols()` and `parameters()` are updated accordingly after the call. An error is returned if a non-finite numeric parameter appears during lowering or a parameter index is invalid.

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Parameter, ParameterValue, Qubit, StandardGate, ValueOperation};

let mut circuit = Circuit::new(1);
circuit.h(Qubit::new(0))?;
let mut dag = CircuitDag::from_circuit(&circuit)?;
let theta = Parameter::symbol("theta");

let rx = ValueOperation::from_standard(
    StandardGate::RX,
    [Qubit::new(0)],
    [ParameterValue::Param(theta.clone())],
);

let front = dag.apply_value_operation_front(rx)?;

assert!(dag.symbols().contains("theta"));
assert_eq!(dag.num_ops(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`substitute_node_with_dag()` first validates the replacement DAG, then checks whether the resources read or written after the replacement fall within the resource scope of the replaced node. When a control flow node is replaced, the replacement must contain exactly one control flow operation of the same kind. An error is returned if any check fails.

```rust
use cqlib_core::circuit::{Circuit, CircuitDag, Qubit};

let mut circuit = Circuit::new(1);
circuit.x(Qubit::new(0))?;
let mut dag = CircuitDag::from_circuit(&circuit)?;

let mut replacement = Circuit::new(1);
replacement.h(Qubit::new(0))?;
let replacement_dag = CircuitDag::from_circuit(&replacement)?;

let node = dag.topological_op_nodes()?[0];
dag.substitute_node_with_dag(node, replacement_dag)?;

assert_eq!(dag.num_ops(), 1);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Typical usage

Compiler passes and routing algorithms usually perform dependency analysis on the DAG: `front_layer()` and `node_layers()` decide which operations are already ready, `predecessors()` and `successors()` determine whether a precedence constraint exists between two operations, run-collection interfaces such as `collect_1q_runs()` locate gate sequences that can be processed as a whole, and the `substitute_node()` family of interfaces writes transformation results back into the graph.

`from_operations()` is suitable for a local view inside a pass: when a segment of operations has already had its parameters resolved and involves only quantum resources, it can be used to construct a lightweight dependency graph that carries no circuit-level metadata.

---

## Error handling

Most interfaces of `CircuitDag` return `Result<_, CircuitError>`. The common error sources are as follows:

| Error | When it occurs |
| --- | --- |
| `CircuitError::InvalidDag` | The graph contains a cycle, operation node numbers are duplicated or inconsistent with the order index, a wire sentinel is invalid, the timeline on a wire is broken or forked, a node identifier does not exist, a control flow payload does not match the owning operation, or the resource scope or control flow shape of the replacement DAG is invalid. |
| `CircuitError::QubitNotFound` | An operation references a qubit that is not in the DAG qubit set. |
| `CircuitError::DuplicateQubits` | The `qubits` of `from_operations()` contains duplicate qubits. |
| `CircuitError::ForeignClassicalHandle` | An operation or wire references a classical variable or classical value belonging to another circuit. |
| `CircuitError::InvalidParameterIndex` | An operation references a parameter index outside the range of the parameter table, or the global phase parameter cannot be resolved. |
| `CircuitError::InvalidParameterValue` | A non-finite numeric parameter appears while lowering a construction-layer operation. |
| `CircuitError::InvalidOperation` | An invalid control flow representation is nested inside a construction-layer instruction. |
| `CircuitError::DuplicateClassicalValueDefinition` | The same immutable classical value has more than one producer. |
