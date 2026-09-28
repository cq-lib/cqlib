# CircuitCFG

`cqlib_core::circuit::CircuitCFG`

```rust
use cqlib_core::circuit::CircuitCFG;
use cqlib_core::circuit::cfg::{
    BasicBlock,
    ControlFlowRegion,
    FlowEdge,
    OperationMetadata,
    SwitchRegionCase,
    Terminator,
};
```

`CircuitCFG` is the analysis view used in the Rust core to represent a quantum circuit control flow graph (Control Flow Graph, CFG). Unlike `Circuit`, which is oriented to users and structured circuit construction, `CircuitCFG` is oriented more toward internal compiler use, for analyzing, rewriting and validating circuits that contain structured control flow.

---

## Core concepts

| Type | Description |
| --- | --- |
| `CircuitCFG` | The control flow graph view of a circuit, containing basic blocks, control flow edges, the entry block, qubits, the classical data tables, the symbol table, the parameter table and the global phase. |
| `BasicBlock` | A basic block that holds a sequence of sequentially executed `Operation`s; in a valid CFG it must carry exactly one terminator. |
| `Terminator` | The control transfer description at the end of a basic block, for example a sequential jump, a branch or a loop exit. |
| `FlowEdge` | The edge type in the control flow graph, used to describe the execution flow relationship between blocks. |
| `ControlFlowRegion` | Structured control flow region metadata, used to record the structured region corresponding to a branch or a loop. |
| `OperationMetadata` | Operation-level metadata, recording the qubits, parameters and label the operation acts on. |
| `SwitchRegionCase` | Case metadata in a `switch` control flow region, recording the case value and the corresponding entry block. |

Among these types, only `CircuitCFG` can be imported from the `cqlib_core::circuit` module root; the remaining types live in the `cqlib_core::circuit::cfg` submodule. The symbol table, the parameter table and the global phase are kept internally by the CFG and have no public access interface, but they are restored together when `to_circuit()` is called.

---

## `BasicBlock`

`BasicBlock` represents a sequence of sequentially executed operations.

The common methods are as follows:

| Method | Description |
| --- | --- |
| `new()` | Create an empty basic block. |
| `with_label(label)` | Consume itself and return a new basic block with a label; requires chaining or rebinding. |
| `push_operation(op)` | Append one operation to the end of the basic block. |
| `extend_operations(ops)` | Append operations in bulk. |
| `set_terminator(terminator)` | Set the terminator of the basic block. |
| `is_empty()` | Determine whether the basic block has neither operations nor a terminator. |
| `has_terminator()` | Determine whether the basic block already has a terminator set. |
| `len()` | Return the number of operations in the basic block. |
| `label()` | Read the basic block label. |
| `terminator()` | Read the basic block terminator. |
| `operations()` | Read the slice of operations in the basic block. |

In addition, `BasicBlock` implements `Default`, whose behavior is equivalent to `BasicBlock::new()`.

---

## `Terminator`

```rust
pub enum Terminator {
    Branch(ClassicalExpr),
    ForLoop {
        var: ClassicalVar,
        start: ClassicalExpr,
        stop: ClassicalExpr,
        step: ClassicalExpr,
    },
    Switch(ClassicalExpr),
    Jump(NodeIndex),
    Break(NodeIndex),
    Continue(NodeIndex),
    Return,
}
```

| Variant | Description |
| --- | --- |
| `Branch(condition)` | Conditional branch; the condition must be a `Bool` expression, and the outgoing edge types are `FlowEdge::TrueBranch` and `FlowEdge::FalseBranch` respectively. |
| `ForLoop { var, start, stop, step }` | Half-open interval loop; the four expressions must be `UInt` of the same bit width. |
| `Switch(target)` | Multi-way selection; the target must be a `UInt` expression, and the number of outgoing edges must equal the number of cases plus one. |
| `Jump(target)` | Unconditional jump to the target basic block; the outgoing edge type is `FlowEdge::Unconditional`. |
| `Break(target)` | Break out of a loop or a `switch`; the outgoing edge type is `FlowEdge::Break`. |
| `Continue(target)` | Enter the next loop iteration; the outgoing edge type is `FlowEdge::Continue`. |
| `Return` | End the current circuit branch; no outgoing edge is allowed. |

In a valid CFG, every basic block must have exactly one terminator, and its outgoing edge count and outgoing edge types must agree with the terminator. For a linear circuit constructed by `from_circuit()`, the entry block terminator is `Terminator::Return`.

---

## `FlowEdge`

| Variant | Description |
| --- | --- |
| `TrueBranch` | The edge taken when the condition is true. |
| `FalseBranch` | The edge taken when the condition is false. |
| `Unconditional` | An unconditional jump edge. |
| `Case(value)` | The edge taken when `value` is matched in a `switch`. |
| `DefaultCase` | The edge of the `switch` default branch; it exists even when the source `switch` has no default body. |
| `Break` | The `break` exit edge. |
| `Continue` | The `continue` edge. |

---

## Creating `CircuitCFG`

`CircuitCFG` provides the following creation interfaces:

```rust
pub fn new(num_qubits: usize) -> Self
pub fn from_qubits(qubits: Vec<Qubit>) -> Self
pub fn from_circuit(circuit: &Circuit) -> Result<Self, CircuitError>
```

| Interface | Description |
| --- | --- |
| `CircuitCFG::new(num_qubits)` | Create an empty CFG containing consecutive logical qubits; no entry block is set yet, so it must be edited before it can pass `validate()`. |
| `CircuitCFG::from_qubits(qubits)` | Create an empty CFG from the specified set of logical qubits, suitable for sparse logical numbering; duplicate qubits are removed and insertion order is preserved. |
| `CircuitCFG::from_circuit(circuit)` | Construct a CFG from an existing structured `Circuit`; this is the most common entry point. `validate()` is called internally once before construction finishes, and control flow that cannot be structured returns an error directly. |

`CircuitCFG` also implements `TryFrom<&Circuit>`, whose behavior is identical to `from_circuit()`; the reverse `TryFrom<&CircuitCFG> for Circuit` is identical to `to_circuit()`. The error type of both is `CircuitError`.

```rust
use cqlib_core::circuit::{Circuit, CircuitCFG, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;

let cfg = CircuitCFG::from_circuit(&circuit)?;

assert_eq!(cfg.num_qubits(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Graph editing interfaces

`CircuitCFG` provides basic graph editing capabilities, used to add basic blocks, add edges, set the entry block and maintain structured control flow region metadata.

| Method | Description |
| --- | --- |
| `add_block(block)` | Add a `BasicBlock` and return the corresponding `NodeIndex`. |
| `add_edge(source, target, flow)` | Add a control flow edge between `source` and `target`, returning the edge index on success; if either endpoint does not exist, the graph is not modified and `None` is returned. |
| `entry_block()` | Read the entry basic block of the current CFG; return `None` when it is not set. |
| `set_entry_block(index)` | Set the entry basic block of the CFG; this interface does not check whether the index is valid, and validity is decided by `validate()`. |
| `set_control_flow_region(branch_block, region)` | Set structured region metadata for a branch or control flow entry block. |
| `control_flow_region(branch_block)` | Read the structured control flow region metadata associated with a basic block; return `None` when there is no record. |
| `is_loop_header(block)` | Determine whether a basic block is a loop header; only the entry block of a `While` or `For` region returns `true`, and a `Switch` region entry does not count as a loop header. |
| `block_mut(index)` | Mutably read the specified basic block; return `None` when the index does not exist. |
| `outgoing_edges(source)` | Iterate over the outgoing edges of the specified basic block, yielding `(NodeIndex, FlowEdge)` pairs. |

---

## Query interfaces

`CircuitCFG` provides the following query interfaces, used to traverse the graph structure and read basic circuit information.

| Method | Description |
| --- | --- |
| `blocks()` | Iterate over the basic blocks in the CFG, yielding `(NodeIndex, &BasicBlock)` pairs. |
| `num_blocks()` | Return the number of basic blocks. |
| `num_qubits()` | Return the number of qubits. |
| `qubits()` | Return the list of logical qubits in insertion order, as an owned `Vec<Qubit>`. |
| `classical_vars()` | Return the classical variable type table. |
| `classical_values()` | Return the classical value type table. |

---

## Validation and reconstruction

`CircuitCFG` provides two key interfaces for checking and restoring the graph structure:

```rust
pub fn validate(&self) -> Result<(), CircuitError>
pub fn to_circuit(&self) -> Result<Circuit, CircuitError>
```

### `validate()`

`validate()` checks whether the CFG structure is self-consistent. Typical checks include:

- whether a valid entry block exists;
- whether the basic blocks referenced by edges exist;
- whether every basic block has a terminator, whether the terminator matches the outgoing edge count and outgoing edge types, and whether `Return` has no outgoing edge;
- whether unexpanded `Instruction::ClassicalControl` remains inside basic blocks;
- whether there are unreachable or unconsumed basic blocks, that is, whether all basic blocks are reachable from the entry block;
- whether the control flow region metadata is consistent with the graph structure;
- whether loop headers, loop back edges and exit edges satisfy the constraints;
- whether the condition of `Branch` is a `Bool` expression;
- whether the target of `Switch` is a `UInt` expression, and whether the number of outgoing edges equals the number of cases plus one;
- whether the scopes and dependencies of classical variables and values can be interpreted correctly;
- whether the `break` / `continue` terminators each have exactly one outgoing edge of the same type pointing to the target block.

Whether `break` and `continue` appear inside a valid loop or `switch` region is decided by `from_circuit()` at construction time; `validate()` only checks the correspondence between terminators and outgoing edges.

### `to_circuit()`

`to_circuit()` restores the CFG back into a structured `Circuit`. It requires that the CFG is not only a valid graph, but also maps back to the structured control flow forms supported by Cqlib.

If the graph structure of the CFG has been damaged, or some control flow regions cannot be mapped back to structured constructs such as `if`, `while`, `for` and `switch`, `to_circuit()` returns an error. The method calls `validate()` once internally first, so the constraints of the previous section also apply here.

The recommended workflow is as follows:

```rust
use cqlib_core::circuit::{Circuit, CircuitCFG, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let cfg = CircuitCFG::from_circuit(&circuit)?;

// 在这里执行 CFG 分析或转换 pass

cfg.validate()?;
let new_circuit = cfg.to_circuit()?;

assert_eq!(new_circuit.num_qubits(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## `ControlFlowRegion` metadata

`ControlFlowRegion` records the boundaries and semantics of structured control flow regions. For constructs such as `if`, `while`, `for` and `switch`, CFG edges alone are not enough to fully recover the original structured semantics; region metadata is also needed to describe which basic blocks belong to the same control flow structure, where the entry and the exit are, and how switch cases correspond.

```rust
pub enum ControlFlowRegion {
    If {
        then_entry: NodeIndex,
        else_entry: NodeIndex,
        merge_block: NodeIndex,
        has_else: bool,
        outer: OperationMetadata,
    },
    While {
        body_entry: NodeIndex,
        exit_block: NodeIndex,
        outer: OperationMetadata,
    },
    For {
        body_entry: NodeIndex,
        exit_block: NodeIndex,
        outer: OperationMetadata,
    },
    Switch {
        cases: Vec<SwitchRegionCase>,
        default_entry: NodeIndex,
        merge_block: NodeIndex,
        has_default: bool,
        outer: OperationMetadata,
    },
}
```

| Variant | Field | Description |
| --- | --- | --- |
| `If` | `then_entry`, `else_entry`, `merge_block`, `has_else`, `outer` | The entry block of the conditional branch region, the `else` branch entry block, the merge block, and whether the source circuit has an `else` branch. |
| `While` | `body_entry`, `exit_block`, `outer` | The `while` loop body entry block and exit block. |
| `For` | `body_entry`, `exit_block`, `outer` | The `for` loop body entry block and exit block. |
| `Switch` | `cases`, `default_entry`, `merge_block`, `has_default`, `outer` | The metadata of each case, the default branch entry block, the merge block, and whether the source `switch` has a default branch. |

Here `outer` is the operation metadata that records the control flow structure itself, used to restore the expanded control flow operation during reconstruction. The related types are as follows:

```rust
pub struct OperationMetadata {
    pub qubits: SmallVec<[Qubit; 3]>,
    pub params: SmallVec<[CircuitParam; 1]>,
    pub label: Option<Box<str>>,
}

pub struct SwitchRegionCase {
    pub value: u128,
    pub entry: NodeIndex,
}
```

| Field | Description |
| --- | --- |
| `OperationMetadata::qubits` | The qubits the control flow operation acts on. |
| `OperationMetadata::params` | The parameters of the control flow operation. |
| `OperationMetadata::label` | The label of the control flow operation. |
| `SwitchRegionCase::value` | The `UInt` value the case matches. |
| `SwitchRegionCase::entry` | The entry block of the branch corresponding to the case. |

These metadata have no separate access interfaces, so a pass must maintain them itself when modifying the graph structure; otherwise parameters or labels may be lost when `to_circuit()` reconstructs the circuit.

Therefore, the following points require special attention when writing a CFG pass:

- if the branch structure is modified, the corresponding `ControlFlowRegion` should be updated as well;
- if basic blocks are deleted or merged, check whether the region metadata still references the old block;
- if loop edges are adjusted, check whether the loop header and the region boundaries are still correct;
- if switch cases are modified, metadata such as `SwitchRegionCase` should be maintained as well;
- if only local optimization is performed on the operations inside a basic block, the region metadata usually does not need to be modified.

Keeping the region metadata consistent with the graph edges is the key to whether `to_circuit()` can successfully reconstruct a structured circuit.

---

## Typical workflow

### 1. Entering the CFG from a structured circuit

```rust
use cqlib_core::circuit::{Circuit, CircuitCFG, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let cfg = CircuitCFG::from_circuit(&circuit)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### 2. Iterating over basic blocks

```rust
use cqlib_core::circuit::{Circuit, CircuitCFG, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;

let cfg = CircuitCFG::from_circuit(&circuit)?;

for (node, block) in cfg.blocks() {
    for _op in block.operations() {
        // 分析每条 Operation
    }

    if block.has_terminator() {
        // 分析控制转移
    }
}

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### 3. Validating and reconstructing after modification

```rust
use cqlib_core::circuit::cfg::{BasicBlock, Terminator};
use cqlib_core::circuit::{Circuit, CircuitCFG};

let mut cfg = CircuitCFG::new(1);

let mut block = BasicBlock::new().with_label("entry");
block.set_terminator(Terminator::Return);
let entry = cfg.add_block(block);
cfg.set_entry_block(entry);

cfg.validate()?;
let circuit = cfg.to_circuit()?;

assert_eq!(circuit.num_qubits(), 1);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Applicable compiler pass types

`CircuitCFG` is suitable for analysis and transformation tasks that require an explicit control flow structure, for example:

| Pass type | Description |
| --- | --- |
| Control flow reachability analysis | Check unreachable basic blocks, branch paths and loop structures. |
| Local gate optimization within a branch | Perform gate merging, gate cancellation or local replacement inside each basic block. |
| Loop body resource estimation | Count the gate count, depth or measurement usage inside a loop body. |
| Classical data scope checking | Analyze whether `ClassicalValue` is used out of scope. |
| Dynamic circuit validity checking | Check whether control flow, measurement and classical data satisfy backend constraints. |
| Backend control flow lowering | Convert structured control flow into the form supported by the target backend. |
| Branch expansion or staticization | Simplify control flow into linear operations when the condition can be determined statically. |
