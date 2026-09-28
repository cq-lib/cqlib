// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! C ABI for circuit dependency DAGs.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::circuit::advanced::CValueOperation;
use crate::circuit::operations::COperation;
use crate::circuit::{CCircuit, CParameter};
use cqlib_core::circuit::dag::{CircuitDag, DagControlFlow, DagWire};
use cqlib_core::circuit::gate::instruction::Instruction;
use cqlib_core::circuit::{Operation, Qubit};
use rustworkx_core::petgraph::prelude::NodeIndex;
use std::ffi::CString;
use std::os::raw::c_char;

/// Opaque handle around a [`CircuitDag`].
pub struct CCircuitDag {
    pub inner: CircuitDag,
}

/// Opaque snapshot of a [`DagControlFlow`] structure attached to a DAG
/// node. Produced by `circuit_dag_control_flow`; free with
/// `dag_control_flow_free`.
pub struct CDagControlFlow {
    inner: DagControlFlow,
}

/// Control-flow kind tag: `if/else`.
pub const DAG_CONTROL_FLOW_IF: u32 = 0;
/// Control-flow kind tag: `while`.
pub const DAG_CONTROL_FLOW_WHILE: u32 = 1;
/// Control-flow kind tag: `for`.
pub const DAG_CONTROL_FLOW_FOR: u32 = 2;
/// Control-flow kind tag: `switch`.
pub const DAG_CONTROL_FLOW_SWITCH: u32 = 3;
/// Control-flow kind tag: `break`.
pub const DAG_CONTROL_FLOW_BREAK: u32 = 4;
/// Control-flow kind tag: `continue`.
pub const DAG_CONTROL_FLOW_CONTINUE: u32 = 5;

/// Run-collection criterion: contiguous one-qubit gates.
pub const DAG_RUN_CRITERION_1Q_GATE: u32 = 0;
/// Run-collection criterion: contiguous two-qubit gates.
pub const DAG_RUN_CRITERION_2Q_GATE: u32 = 1;

/// DAG wire tag: quantum bit timeline carrying a qubit id.
pub const DAG_WIRE_QUBIT: u32 = 0;
/// DAG wire tag: mutable classical storage carrying a variable id.
pub const DAG_WIRE_CLASSICAL_VAR: u32 = 1;
/// DAG wire tag: immutable classical value carrying a value id.
pub const DAG_WIRE_CLASSICAL_VALUE: u32 = 2;
/// DAG wire tag: global ordering resource with no payload.
pub const DAG_WIRE_GLOBAL_ORDER: u32 = 3;

fn wire_tag(wire: &DagWire) -> (u32, u32) {
    match wire {
        DagWire::Qubit(qubit) => (DAG_WIRE_QUBIT, qubit.id()),
        DagWire::ClassicalVar(var) => (DAG_WIRE_CLASSICAL_VAR, var.id()),
        DagWire::ClassicalValue(value) => (DAG_WIRE_CLASSICAL_VALUE, value.index()),
        DagWire::GlobalOrder => (DAG_WIRE_GLOBAL_ORDER, 0),
    }
}

/// Rebuilds a wire from its C tag and payload id. Classical wires carry
/// circuit-scoped handles that C cannot reconstruct, so their tags are
/// rejected on input and reported as `-8`.
fn dag_wire_from_tag(tag: u32, id: u32) -> Option<DagWire> {
    match tag {
        DAG_WIRE_QUBIT => Some(DagWire::Qubit(Qubit::new(id))),
        DAG_WIRE_GLOBAL_ORDER => Some(DagWire::GlobalOrder),
        _ => None,
    }
}

/// Copies node indices into a C `u32` buffer. Returns 0 on success, -1 for
/// a NULL buffer (when there is data to write), -8 when `len` is too small.
fn copy_node_ids(nodes: &[NodeIndex], buffer: *mut u32, len: usize) -> i32 {
    if len < nodes.len() {
        return -8;
    }
    if nodes.is_empty() {
        return 0;
    }
    if buffer.is_null() {
        return -1;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(buffer, nodes.len()) };
    for (slot, node) in slice.iter_mut().zip(nodes.iter()) {
        *slot = node.index() as u32;
    }
    0
}

/// Copies a list of node lists (layers or runs) in CSR style: flattened
/// node ids plus boundary offsets with `offsets[0] == 0`. Returns 0 on
/// success, -1 for NULL buffers, -8 when capacities are too small.
fn copy_node_groups(
    groups: &[Vec<NodeIndex>],
    nodes: *mut u32,
    nodes_cap: usize,
    offsets: *mut usize,
    offsets_cap: usize,
) -> i32 {
    let total: usize = groups.iter().map(Vec::len).sum();
    if offsets_cap < groups.len() + 1 || nodes_cap < total {
        return -8;
    }
    if offsets.is_null() {
        return -1;
    }
    let offset_slice = unsafe { std::slice::from_raw_parts_mut(offsets, groups.len() + 1) };
    offset_slice[0] = 0;
    let mut running = 0usize;
    for (slot, group) in offset_slice.iter_mut().skip(1).zip(groups.iter()) {
        running += group.len();
        *slot = running;
    }
    if total > 0 {
        if nodes.is_null() {
            return -1;
        }
        let slice = unsafe { std::slice::from_raw_parts_mut(nodes, total) };
        let mut filled = 0usize;
        for group in groups {
            for node in group {
                slice[filled] = node.index() as u32;
                filled += 1;
            }
        }
    }
    0
}

fn is_one_qubit_gate_c(operation: &Operation) -> bool {
    operation.qubits.len() == 1
        && matches!(
            operation.instruction,
            Instruction::Standard(_)
                | Instruction::McGate(_)
                | Instruction::UnitaryGate(_)
                | Instruction::CircuitGate(_)
        )
}

fn is_two_qubit_gate_c(operation: &Operation) -> bool {
    operation.qubits.len() == 2
        && matches!(
            operation.instruction,
            Instruction::Standard(_)
                | Instruction::McGate(_)
                | Instruction::UnitaryGate(_)
                | Instruction::CircuitGate(_)
        )
}

fn control_flow_kind_tag(flow: &DagControlFlow) -> u32 {
    match flow {
        DagControlFlow::If { .. } => DAG_CONTROL_FLOW_IF,
        DagControlFlow::While { .. } => DAG_CONTROL_FLOW_WHILE,
        DagControlFlow::For { .. } => DAG_CONTROL_FLOW_FOR,
        DagControlFlow::Switch { .. } => DAG_CONTROL_FLOW_SWITCH,
        DagControlFlow::Break => DAG_CONTROL_FLOW_BREAK,
        DagControlFlow::Continue => DAG_CONTROL_FLOW_CONTINUE,
    }
}

/// Lists body DAGs in a stable order: `if` is `[then, else?]`, `while` and
/// `for` carry a single body, `switch` lists case bodies in order, and
/// `break`/`continue` carry none.
fn control_flow_bodies(flow: &DagControlFlow) -> Vec<&CircuitDag> {
    match flow {
        DagControlFlow::If {
            then_body,
            else_body,
        } => {
            let mut bodies = vec![then_body.as_ref()];
            if let Some(else_body) = else_body {
                bodies.push(else_body.as_ref());
            }
            bodies
        }
        DagControlFlow::While { body } | DagControlFlow::For { body } => vec![body.as_ref()],
        DagControlFlow::Switch { cases, .. } => {
            cases.iter().map(|case| case.body.as_ref()).collect()
        }
        DagControlFlow::Break | DagControlFlow::Continue => Vec::new(),
    }
}

/// Builds a dependency DAG from a circuit. Returns a newly allocated
/// `CCircuitDag*` (free with `circuit_dag_free`), or NULL on NULL input or
/// build failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_from_circuit(ptr: *const CCircuit) -> *mut CCircuitDag {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match CircuitDag::from_circuit(unsafe { &(*ptr).inner }) {
        Ok(inner) => Box::into_raw(Box::new(CCircuitDag { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Builds a dependency DAG from explicit qubits and operations. Each
/// element of `qubits` is used directly as a qubit id; every operation
/// must act on registered ids and duplicate ids fail the build. Returns
/// a newly allocated `CCircuitDag*`, or NULL on NULL input or build
/// failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_from_operations(
    qubits: *const u32,
    qubits_len: usize,
    ops: *const *const COperation,
    ops_len: usize,
) -> *mut CCircuitDag {
    if ops_len > 0 && ops.is_null() {
        return std::ptr::null_mut();
    }
    let mut qubit_list = Vec::with_capacity(qubits_len);
    if qubits_len > 0 {
        if qubits.is_null() {
            return std::ptr::null_mut();
        }
        let slice = unsafe { std::slice::from_raw_parts(qubits, qubits_len) };
        for &id in slice {
            qubit_list.push(Qubit::new(id));
        }
    }
    let mut op_list = Vec::with_capacity(ops_len);
    if ops_len > 0 {
        let slice = unsafe { std::slice::from_raw_parts(ops, ops_len) };
        for op in slice {
            if op.is_null() {
                return std::ptr::null_mut();
            }
            op_list.push(unsafe { (*(*op)).inner.clone() });
        }
    }
    match CircuitDag::from_operations(qubit_list, &op_list) {
        Ok(inner) => Box::into_raw(Box::new(CCircuitDag { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a DAG handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_free(ptr: *mut CCircuitDag) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Rebuilds the source-order circuit from the DAG. Returns a newly
/// allocated `CCircuit*` (free with `circuit_free`), or NULL on NULL input
/// or lowering failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_to_circuit(ptr: *const CCircuitDag) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { (*ptr).inner.to_circuit() } {
        Ok(inner) => Box::into_raw(Box::new(CCircuit { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Validates DAG consistency (topological order). Returns 0 on success, -1
/// on NULL input, -3 on a cyclic or invalid graph.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_validate(ptr: *const CCircuitDag) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.validate().map_or(-3, |_| 0) }
}

/// Returns the number of qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_num_qubits(ptr: *const CCircuitDag) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Returns the number of operation nodes, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_num_ops(ptr: *const CCircuitDag) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_ops() }
}

/// Returns whether the DAG has no operation nodes: 1 when empty, 0
/// otherwise, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_is_empty(ptr: *const CCircuitDag) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.is_empty() as i32 }
}

/// Returns the number of qubit ids in the DAG, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_qubits_len(ptr: *const CCircuitDag) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.qubits().len() }
}

/// Copies the qubit ids in registration order into `buffer`. Returns 0 on
/// success, -1 on NULL input, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_qubits(ptr: *const CCircuitDag, buffer: *mut u32, len: usize) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let qubits = unsafe { (*ptr).inner.qubits() };
    if len < qubits.len() {
        return -8;
    }
    if qubits.is_empty() {
        return 0;
    }
    if buffer.is_null() {
        return -1;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(buffer, qubits.len()) };
    for (slot, qubit) in slice.iter_mut().zip(qubits.iter()) {
        *slot = qubit.id();
    }
    0
}

/// Returns the number of registered parameters, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_parameters_len(ptr: *const CCircuitDag) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.parameters().len() }
}

/// Copies the parameter names in registration order into `out`. Each
/// written element is a freshly allocated C string the caller frees with
/// `cqlib_string_free`. Returns 0 on success, -1 on NULL input, -8 when
/// `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_parameters(
    ptr: *const CCircuitDag,
    out: *mut *mut c_char,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let params = unsafe { (*ptr).inner.parameters() };
    if len < params.len() {
        return -8;
    }
    if params.is_empty() {
        return 0;
    }
    if out.is_null() {
        return -1;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(out, params.len()) };
    for (slot, param) in slice.iter_mut().zip(params.iter()) {
        let text = param.as_symbol().unwrap_or_else(|| param.to_string());
        match CString::new(text) {
            Ok(text) => *slot = text.into_raw(),
            Err(_) => return -3,
        }
    }
    0
}

/// Interns a copy of `param` into the DAG's parameter table, mirroring
/// `Circuit::add_parameter`: the parameter is inserted when absent and all
/// of its symbol names are registered. Writes the table index into
/// `out_index` and 1/0 for "newly inserted" into `out_inserted` unless
/// NULL. Returns 0 on success, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_add_parameter(
    ptr: *mut CCircuitDag,
    param: *const CParameter,
    out_index: *mut usize,
    out_inserted: *mut i32,
) -> i32 {
    let dag = if !ptr.is_null() {
        unsafe { &mut (*ptr).inner }
    } else {
        return -1;
    };
    if param.is_null() {
        return -1;
    }
    let param = unsafe { (*param).inner.clone() };
    let (index, inserted) = dag.add_parameter(param);
    if !out_index.is_null() {
        unsafe { *out_index = index };
    }
    if !out_inserted.is_null() {
        unsafe { *out_inserted = inserted as i32 };
    }
    0
}

/// Returns the number of symbols referenced by the DAG, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_symbols_len(ptr: *const CCircuitDag) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.symbols().len() }
}

/// Copies the symbol names into `out`. Each written element is a freshly
/// allocated C string the caller frees with `cqlib_string_free`. Returns
/// 0 on success, -1 on NULL input, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_symbols(
    ptr: *const CCircuitDag,
    out: *mut *mut c_char,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let symbols = unsafe { (*ptr).inner.symbols() };
    if len < symbols.len() {
        return -8;
    }
    if symbols.is_empty() {
        return 0;
    }
    if out.is_null() {
        return -1;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(out, symbols.len()) };
    for (slot, symbol) in slice.iter_mut().zip(symbols.iter()) {
        match CString::new(symbol.as_str()) {
            Ok(text) => *slot = text.into_raw(),
            Err(_) => return -3,
        }
    }
    0
}

/// Returns the number of wires in the DAG, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_wires_len(ptr: *const CCircuitDag) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.wires().count() }
}

/// Copies wire tags and payload ids into `tags` and `ids`. Returns 0 on
/// success, -1 on NULL input, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_wires(
    ptr: *const CCircuitDag,
    tags: *mut u32,
    ids: *mut u32,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wires: Vec<DagWire> = unsafe { (*ptr).inner.wires().collect() };
    if len < wires.len() {
        return -8;
    }
    if wires.is_empty() {
        return 0;
    }
    if tags.is_null() || ids.is_null() {
        return -1;
    }
    let tag_slice = unsafe { std::slice::from_raw_parts_mut(tags, wires.len()) };
    let id_slice = unsafe { std::slice::from_raw_parts_mut(ids, wires.len()) };
    for (slot, wire) in tag_slice.iter_mut().zip(wires.iter()) {
        *slot = wire_tag(wire).0;
    }
    for (slot, wire) in id_slice.iter_mut().zip(wires.iter()) {
        *slot = wire_tag(wire).1;
    }
    0
}

/// Returns whether the DAG carries the given wire: 1 when present, 0 when
/// not, -1 on NULL input, -8 on an unknown wire tag.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_has_wire(ptr: *const CCircuitDag, tag: u32, id: u32) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match dag_wire_from_tag(tag, id) {
        Some(wire) => unsafe { (*ptr).inner.has_wire(wire) as i32 },
        None => -8,
    }
}

// =====  Node identification  =====

/// Returns whether `node` is an operation node: 1 when it is, 0 when it is
/// a wire sentinel, -1 on NULL input, -2 for an unknown node id.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_is_operation(ptr: *const CCircuitDag, node: u32) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let dag = unsafe { &(*ptr).inner };
    if dag.node_kind(NodeIndex::new(node as usize)).is_none() {
        return -2;
    }
    dag.is_operation(NodeIndex::new(node as usize)) as i32
}

/// Returns the kind of `node` as a freshly allocated C string:
/// `"wire_in"`, `"wire_out"`, or `"operation"` (free with
/// `cqlib_string_free`). Returns NULL on NULL input or an unknown node id.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_node_kind(ptr: *const CCircuitDag, node: u32) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { &(*ptr).inner }.node_kind(NodeIndex::new(node as usize)) {
        Some(kind) => match CString::new(kind) {
            Ok(text) => text.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Returns a copy of the operation stored at `node` as a newly allocated
/// `COperation*` (free with `operation_free`). Returns NULL on NULL input
/// or when `node` is not an operation node.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_operation(ptr: *const CCircuitDag, node: u32) -> *mut COperation {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { &(*ptr).inner }.operation(NodeIndex::new(node as usize)) {
        Some(operation) => Box::into_raw(Box::new(COperation {
            inner: operation.clone(),
        })),
        None => std::ptr::null_mut(),
    }
}

// =====  Graph traversal  =====

/// Returns the number of operation nodes in source order, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_op_nodes_len(ptr: *const CCircuitDag) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &(*ptr).inner }.op_nodes().count()
}

/// Copies the operation nodes in source order into `buffer`. Returns 0 on
/// success, -1 on NULL input, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_op_nodes(
    ptr: *const CCircuitDag,
    buffer: *mut u32,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let nodes: Vec<NodeIndex> = unsafe { &(*ptr).inner }.op_nodes().collect();
    copy_node_ids(&nodes, buffer, len)
}

/// Returns the number of operation nodes in topological order, or 0 for
/// NULL input or a cyclic graph.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_topological_op_nodes_len(ptr: *const CCircuitDag) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &(*ptr).inner }
        .topological_op_nodes()
        .map_or(0, |nodes| nodes.len())
}

/// Copies the operation nodes in deterministic topological order into
/// `buffer`. Returns 0 on success, -1 on NULL input, -8 when `len` is too
/// small, -3 on a cyclic graph.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_topological_op_nodes(
    ptr: *const CCircuitDag,
    buffer: *mut u32,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match unsafe { &(*ptr).inner }.topological_op_nodes() {
        Ok(nodes) => copy_node_ids(&nodes, buffer, len),
        Err(_) => -3,
    }
}

/// Returns the number of direct predecessors of `node`, or 0 for NULL
/// input or an unknown node id.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_predecessors_len(ptr: *const CCircuitDag, node: u32) -> usize {
    let dag = if !ptr.is_null() {
        unsafe { &(*ptr).inner }
    } else {
        return 0;
    };
    if dag.node_kind(NodeIndex::new(node as usize)).is_none() {
        return 0;
    }
    dag.predecessors(NodeIndex::new(node as usize)).count()
}

/// Copies the direct predecessors of `node` into `buffer`. Returns 0 on
/// success, -1 on NULL input or a NULL buffer, -2 for an unknown node id,
/// -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_predecessors(
    ptr: *const CCircuitDag,
    node: u32,
    buffer: *mut u32,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let dag = unsafe { &(*ptr).inner };
    if dag.node_kind(NodeIndex::new(node as usize)).is_none() {
        return -2;
    }
    let nodes: Vec<NodeIndex> = dag.predecessors(NodeIndex::new(node as usize)).collect();
    copy_node_ids(&nodes, buffer, len)
}

/// Returns the number of direct successors of `node`, or 0 for NULL input
/// or an unknown node id.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_successors_len(ptr: *const CCircuitDag, node: u32) -> usize {
    let dag = if !ptr.is_null() {
        unsafe { &(*ptr).inner }
    } else {
        return 0;
    };
    if dag.node_kind(NodeIndex::new(node as usize)).is_none() {
        return 0;
    }
    dag.successors(NodeIndex::new(node as usize)).count()
}

/// Copies the direct successors of `node` into `buffer`. Returns 0 on
/// success, -1 on NULL input or a NULL buffer, -2 for an unknown node id,
/// -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_successors(
    ptr: *const CCircuitDag,
    node: u32,
    buffer: *mut u32,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let dag = unsafe { &(*ptr).inner };
    if dag.node_kind(NodeIndex::new(node as usize)).is_none() {
        return -2;
    }
    let nodes: Vec<NodeIndex> = dag.successors(NodeIndex::new(node as usize)).collect();
    copy_node_ids(&nodes, buffer, len)
}

/// Returns the number of operation predecessors of `node` connected
/// through `wire`, or 0 for NULL input, an unknown node id, an unknown
/// wire tag, or an invalid combination.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_predecessors_on_wire_len(
    ptr: *const CCircuitDag,
    node: u32,
    tag: u32,
    id: u32,
) -> usize {
    match dag_wire_neighbors(ptr, node, tag, id) {
        Ok(nodes) => nodes.len(),
        Err(_) => 0,
    }
}

fn dag_wire_neighbors(
    ptr: *const CCircuitDag,
    node: u32,
    tag: u32,
    id: u32,
) -> Result<Vec<NodeIndex>, i32> {
    if ptr.is_null() {
        return Err(-1);
    }
    let Some(wire) = dag_wire_from_tag(tag, id) else {
        return Err(-8);
    };
    let dag = unsafe { &(*ptr).inner };
    if dag.node_kind(NodeIndex::new(node as usize)).is_none() {
        return Err(-2);
    }
    dag.predecessors_on_wire(NodeIndex::new(node as usize), wire)
        .map_err(|_| -3)
}

/// Copies the operation predecessors of `node` connected through `wire`
/// into `buffer`. Returns 0 on success, -1 on NULL input or a NULL buffer
/// with data available, -2 for an unknown node id, -3 for an invalid
/// wire/node combination, -8 for an unknown wire tag or a too-small
/// buffer.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_predecessors_on_wire(
    ptr: *const CCircuitDag,
    node: u32,
    tag: u32,
    id: u32,
    buffer: *mut u32,
    len: usize,
) -> i32 {
    match dag_wire_neighbors(ptr, node, tag, id) {
        Ok(nodes) => copy_node_ids(&nodes, buffer, len),
        Err(code) => code,
    }
}

/// Returns the number of operation successors of `node` connected through
/// `wire`, or 0 for NULL input, an unknown node id, an unknown wire tag,
/// or an invalid combination.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_successors_on_wire_len(
    ptr: *const CCircuitDag,
    node: u32,
    tag: u32,
    id: u32,
) -> usize {
    match dag_wire_neighbors_out(ptr, node, tag, id) {
        Ok(nodes) => nodes.len(),
        Err(_) => 0,
    }
}

fn dag_wire_neighbors_out(
    ptr: *const CCircuitDag,
    node: u32,
    tag: u32,
    id: u32,
) -> Result<Vec<NodeIndex>, i32> {
    if ptr.is_null() {
        return Err(-1);
    }
    let Some(wire) = dag_wire_from_tag(tag, id) else {
        return Err(-8);
    };
    let dag = unsafe { &(*ptr).inner };
    if dag.node_kind(NodeIndex::new(node as usize)).is_none() {
        return Err(-2);
    }
    dag.successors_on_wire(NodeIndex::new(node as usize), wire)
        .map_err(|_| -3)
}

/// Copies the operation successors of `node` connected through `wire`
/// into `buffer`. Returns 0 on success, -1 on NULL input or a NULL buffer
/// with data available, -2 for an unknown node id, -3 for an invalid
/// wire/node combination, -8 for an unknown wire tag or a too-small
/// buffer.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_successors_on_wire(
    ptr: *const CCircuitDag,
    node: u32,
    tag: u32,
    id: u32,
    buffer: *mut u32,
    len: usize,
) -> i32 {
    match dag_wire_neighbors_out(ptr, node, tag, id) {
        Ok(nodes) => copy_node_ids(&nodes, buffer, len),
        Err(code) => code,
    }
}

enum DagDirection {
    QuantumPredecessors,
    QuantumSuccessors,
    ClassicalPredecessors,
    ClassicalSuccessors,
}

fn node_neighbor_snapshot(
    ptr: *const CCircuitDag,
    node: u32,
    direction: DagDirection,
) -> Result<Vec<NodeIndex>, i32> {
    if ptr.is_null() {
        return Err(-1);
    }
    let dag = unsafe { &(*ptr).inner };
    if dag.node_kind(NodeIndex::new(node as usize)).is_none() {
        return Err(-2);
    }
    let result = match direction {
        DagDirection::QuantumPredecessors => {
            dag.quantum_predecessors(NodeIndex::new(node as usize))
        }
        DagDirection::QuantumSuccessors => dag.quantum_successors(NodeIndex::new(node as usize)),
        DagDirection::ClassicalPredecessors => {
            dag.classical_predecessors(NodeIndex::new(node as usize))
        }
        DagDirection::ClassicalSuccessors => {
            dag.classical_successors(NodeIndex::new(node as usize))
        }
    };
    result.map_err(|_| -3)
}

/// Returns the number of quantum-wire-connected operation predecessors of
/// `node`, or 0 for NULL input, an unknown node id, or failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_quantum_predecessors_len(
    ptr: *const CCircuitDag,
    node: u32,
) -> usize {
    node_neighbor_snapshot(ptr, node, DagDirection::QuantumPredecessors)
        .map_or(0, |nodes| nodes.len())
}

/// Copies the quantum-wire-connected operation predecessors of `node`
/// into `buffer`. Returns 0 on success, -1 on NULL input, -2 for an
/// unknown node id, -3 on failure, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_quantum_predecessors(
    ptr: *const CCircuitDag,
    node: u32,
    buffer: *mut u32,
    len: usize,
) -> i32 {
    match node_neighbor_snapshot(ptr, node, DagDirection::QuantumPredecessors) {
        Ok(nodes) => copy_node_ids(&nodes, buffer, len),
        Err(code) => code,
    }
}

/// Returns the number of quantum-wire-connected operation successors of
/// `node`, or 0 for NULL input, an unknown node id, or failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_quantum_successors_len(ptr: *const CCircuitDag, node: u32) -> usize {
    node_neighbor_snapshot(ptr, node, DagDirection::QuantumSuccessors)
        .map_or(0, |nodes| nodes.len())
}

/// Copies the quantum-wire-connected operation successors of `node` into
/// `buffer`. Returns 0 on success, -1 on NULL input, -2 for an unknown
/// node id, -3 on failure, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_quantum_successors(
    ptr: *const CCircuitDag,
    node: u32,
    buffer: *mut u32,
    len: usize,
) -> i32 {
    match node_neighbor_snapshot(ptr, node, DagDirection::QuantumSuccessors) {
        Ok(nodes) => copy_node_ids(&nodes, buffer, len),
        Err(code) => code,
    }
}

/// Returns the number of classical-wire-connected operation predecessors
/// of `node`, or 0 for NULL input, an unknown node id, or failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_classical_predecessors_len(
    ptr: *const CCircuitDag,
    node: u32,
) -> usize {
    node_neighbor_snapshot(ptr, node, DagDirection::ClassicalPredecessors)
        .map_or(0, |nodes| nodes.len())
}

/// Copies the classical-wire-connected operation predecessors of `node`
/// into `buffer`. Returns 0 on success, -1 on NULL input, -2 for an
/// unknown node id, -3 on failure, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_classical_predecessors(
    ptr: *const CCircuitDag,
    node: u32,
    buffer: *mut u32,
    len: usize,
) -> i32 {
    match node_neighbor_snapshot(ptr, node, DagDirection::ClassicalPredecessors) {
        Ok(nodes) => copy_node_ids(&nodes, buffer, len),
        Err(code) => code,
    }
}

/// Returns the number of classical-wire-connected operation successors of
/// `node`, or 0 for NULL input, an unknown node id, or failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_classical_successors_len(
    ptr: *const CCircuitDag,
    node: u32,
) -> usize {
    node_neighbor_snapshot(ptr, node, DagDirection::ClassicalSuccessors)
        .map_or(0, |nodes| nodes.len())
}

/// Copies the classical-wire-connected operation successors of `node` into
/// `buffer`. Returns 0 on success, -1 on NULL input, -2 for an unknown
/// node id, -3 on failure, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_classical_successors(
    ptr: *const CCircuitDag,
    node: u32,
    buffer: *mut u32,
    len: usize,
) -> i32 {
    match node_neighbor_snapshot(ptr, node, DagDirection::ClassicalSuccessors) {
        Ok(nodes) => copy_node_ids(&nodes, buffer, len),
        Err(code) => code,
    }
}

// =====  Layering and depth  =====

/// Returns the number of operation nodes without an operation predecessor,
/// or 0 for NULL input or a cyclic graph.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_front_layer_len(ptr: *const CCircuitDag) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &(*ptr).inner }
        .front_layer()
        .map_or(0, |nodes| nodes.len())
}

/// Copies the front-layer operation nodes into `buffer`. Returns 0 on
/// success, -1 on NULL input, -3 on a cyclic graph, -8 when `len` is too
/// small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_front_layer(
    ptr: *const CCircuitDag,
    buffer: *mut u32,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match unsafe { &(*ptr).inner }.front_layer() {
        Ok(nodes) => copy_node_ids(&nodes, buffer, len),
        Err(_) => -3,
    }
}

/// Returns the number of ASAP layers (the ASAP depth), or 0 for NULL input
/// or a cyclic graph.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_layers_len(ptr: *const CCircuitDag) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &(*ptr).inner }
        .layers()
        .map_or(0, |layers| layers.len())
}

/// Copies the ASAP layers in CSR style: layer nodes flattened into `nodes`
/// and layer boundaries into `offsets` (`offsets[0] == 0`, the end is
/// `offsets[layers_len]`). `nodes_cap` must hold all operation nodes and
/// `offsets_cap` at least `layers_len + 1`. Returns 0 on success, -1 on
/// NULL input, -3 on a cyclic graph, -8 on undersized capacities.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_layers(
    ptr: *const CCircuitDag,
    nodes: *mut u32,
    nodes_cap: usize,
    offsets: *mut usize,
    offsets_cap: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match unsafe { &(*ptr).inner }.layers() {
        Ok(layers) => copy_node_groups(&layers, nodes, nodes_cap, offsets, offsets_cap),
        Err(_) => -3,
    }
}

/// Returns the number of per-node layer entries (operation nodes), or 0
/// for NULL input or a cyclic graph.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_node_layers_len(ptr: *const CCircuitDag) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &(*ptr).inner }
        .node_layers()
        .map_or(0, |entries| entries.len())
}

/// Copies the ASAP layer index of every operation node as (node, layer)
/// pairs into `nodes` and `layers`. Returns 0 on success, -1 on NULL input
/// or NULL buffers, -3 on a cyclic graph, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_node_layers(
    ptr: *const CCircuitDag,
    nodes: *mut u32,
    layers: *mut usize,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let entries = match unsafe { &(*ptr).inner }.node_layers() {
        Ok(entries) => entries,
        Err(_) => return -3,
    };
    if len < entries.len() {
        return -8;
    }
    if entries.is_empty() {
        return 0;
    }
    if nodes.is_null() || layers.is_null() {
        return -1;
    }
    let node_slice = unsafe { std::slice::from_raw_parts_mut(nodes, entries.len()) };
    let layer_slice = unsafe { std::slice::from_raw_parts_mut(layers, entries.len()) };
    for ((slot_node, slot_layer), (node, layer)) in node_slice
        .iter_mut()
        .zip(layer_slice.iter_mut())
        .zip(entries.iter())
    {
        *slot_node = node.index() as u32;
        *slot_layer = *layer;
    }
    0
}

/// Returns the ASAP dependency depth: the number of layers. Returns -1 on
/// NULL input, -3 on a cyclic graph.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_depth(ptr: *const CCircuitDag) -> isize {
    if ptr.is_null() {
        return -1;
    }
    unsafe { &(*ptr).inner }
        .depth()
        .map_or(-3, |depth| depth as isize)
}

// =====  Wire timelines  =====

/// Returns whether `wire` carries no operation: 1 when idle, 0 when in use,
/// -1 on NULL input, -3 when the wire is not part of this DAG, -8 on an
/// unknown wire tag.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_is_wire_idle(ptr: *const CCircuitDag, tag: u32, id: u32) -> i32 {
    let wire = match check_wire(ptr, tag, id) {
        Ok(wire) => wire,
        Err(code) => return code,
    };
    match unsafe { &(*ptr).inner }.is_wire_idle(wire) {
        Ok(idle) => idle as i32,
        Err(_) => -3,
    }
}

fn check_wire(ptr: *const CCircuitDag, tag: u32, id: u32) -> Result<DagWire, i32> {
    if ptr.is_null() {
        return Err(-1);
    }
    dag_wire_from_tag(tag, id).ok_or(-8)
}

/// Returns the number of operation nodes on `wire` in wire order, or 0 for
/// NULL input, an unknown wire tag, or an invalid wire.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_nodes_on_wire_len(
    ptr: *const CCircuitDag,
    tag: u32,
    id: u32,
) -> usize {
    match nodes_on_wire_checked(ptr, tag, id) {
        Ok(nodes) => nodes.len(),
        Err(_) => 0,
    }
}

fn nodes_on_wire_checked(
    ptr: *const CCircuitDag,
    tag: u32,
    id: u32,
) -> Result<Vec<NodeIndex>, i32> {
    let wire = check_wire(ptr, tag, id)?;
    unsafe { &(*ptr).inner }.nodes_on_wire(wire).map_err(|_| -3)
}

/// Copies the operation nodes on `wire` in wire order into `buffer`.
/// Returns 0 on success, -1 on NULL input, -3 when the wire is not part of
/// this DAG, -8 for an unknown wire tag or a too-small buffer.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_nodes_on_wire(
    ptr: *const CCircuitDag,
    tag: u32,
    id: u32,
    buffer: *mut u32,
    len: usize,
) -> i32 {
    match nodes_on_wire_checked(ptr, tag, id) {
        Ok(nodes) => copy_node_ids(&nodes, buffer, len),
        Err(code) => code,
    }
}

/// Returns the input sentinel node of `wire`, or -1 on NULL input, -3 when
/// the wire is not part of this DAG, -8 on an unknown wire tag.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_wire_in(ptr: *const CCircuitDag, tag: u32, id: u32) -> i32 {
    let wire = match check_wire(ptr, tag, id) {
        Ok(wire) => wire,
        Err(code) => return code,
    };
    match unsafe { &(*ptr).inner }.wire_in(wire) {
        Some(node) => node.index() as i32,
        None => -3,
    }
}

/// Returns the output sentinel node of `wire`, or -1 on NULL input, -3
/// when the wire is not part of this DAG, -8 on an unknown wire tag.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_wire_out(ptr: *const CCircuitDag, tag: u32, id: u32) -> i32 {
    let wire = match check_wire(ptr, tag, id) {
        Ok(wire) => wire,
        Err(code) => return code,
    };
    match unsafe { &(*ptr).inner }.wire_out(wire) {
        Some(node) => node.index() as i32,
        None => -3,
    }
}

// =====  Control-flow and measurement flags  =====

/// Returns whether any top-level operation is structured control flow: 1
/// when true, 0 otherwise, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_has_control_flow(ptr: *const CCircuitDag) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { &(*ptr).inner }.has_control_flow() as i32
}

/// Returns whether a control-flow body contains structured control flow: 1
/// when true, 0 otherwise, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_has_nested_control_flow(ptr: *const CCircuitDag) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { &(*ptr).inner }.has_nested_control_flow() as i32
}

/// Returns whether any top-level operation directly or recursively
/// measures: 1 when true, 0 otherwise, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_has_measurement(ptr: *const CCircuitDag) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { &(*ptr).inner }.has_measurement() as i32
}

// =====  Operation counting  =====

/// Returns the number of distinct instruction names among top-level
/// operations, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_operation_count_by_name_len(ptr: *const CCircuitDag) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &(*ptr).inner }.operation_count_by_name().len()
}

/// Copies the top-level operation counts into `names` and `counts` in
/// insertion order. Each written name is a freshly allocated C string the
/// caller frees with `cqlib_string_free`. Returns 0 on success, -1 on NULL
/// input or NULL buffers, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_operation_count_by_name(
    ptr: *const CCircuitDag,
    names: *mut *mut c_char,
    counts: *mut usize,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let totals = unsafe { &(*ptr).inner }.operation_count_by_name();
    copy_name_counts(
        totals.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        names,
        counts,
        len,
    )
}

/// Returns the number of distinct instruction names among top-level and
/// nested control-flow body operations, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_operation_count_by_name_recursive_len(
    ptr: *const CCircuitDag,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &(*ptr).inner }
        .operation_count_by_name_recursive()
        .len()
}

/// Copies the top-level and nested operation counts into `names` and
/// `counts` in insertion order. Each written name is a freshly allocated C
/// string the caller frees with `cqlib_string_free`. Returns 0 on success,
/// -1 on NULL input or NULL buffers, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_operation_count_by_name_recursive(
    ptr: *const CCircuitDag,
    names: *mut *mut c_char,
    counts: *mut usize,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let totals = unsafe { &(*ptr).inner }.operation_count_by_name_recursive();
    copy_name_counts(
        totals.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        names,
        counts,
        len,
    )
}

fn copy_name_counts(
    totals: Vec<(String, usize)>,
    names: *mut *mut c_char,
    counts: *mut usize,
    len: usize,
) -> i32 {
    let n = totals.len();
    if len < n {
        return -8;
    }
    if n == 0 {
        return 0;
    }
    if names.is_null() || counts.is_null() {
        return -1;
    }
    let name_slice = unsafe { std::slice::from_raw_parts_mut(names, n) };
    let count_slice = unsafe { std::slice::from_raw_parts_mut(counts, n) };
    for ((slot_name, slot_count), (name, count)) in name_slice
        .iter_mut()
        .zip(count_slice.iter_mut())
        .zip(totals.iter())
    {
        match CString::new(name.as_str()) {
            Ok(text) => *slot_name = text.into_raw(),
            Err(_) => return -3,
        }
        *slot_count = *count;
    }
    0
}

// =====  Contiguous run collection  =====

/// Returns the number of contiguous one-qubit gate runs over all qubit
/// wires, or 0 for NULL input or failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_collect_1q_runs_len(ptr: *const CCircuitDag) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &(*ptr).inner }
        .collect_1q_runs()
        .map_or(0, |runs| runs.len())
}

/// Copies the contiguous one-qubit gate runs in CSR style: run nodes
/// flattened into `nodes` and run boundaries into `offsets`
/// (`offsets[0] == 0`). Returns 0 on success, -1 on NULL input, -3 on a
/// broken timeline, -8 on undersized capacities.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_collect_1q_runs(
    ptr: *const CCircuitDag,
    nodes: *mut u32,
    nodes_cap: usize,
    offsets: *mut usize,
    offsets_cap: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match unsafe { &(*ptr).inner }.collect_1q_runs() {
        Ok(runs) => copy_node_groups(&runs, nodes, nodes_cap, offsets, offsets_cap),
        Err(_) => -3,
    }
}

/// Returns the number of contiguous two-qubit gate runs over all qubit
/// wires, or 0 for NULL input or failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_collect_2q_runs_len(ptr: *const CCircuitDag) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &(*ptr).inner }
        .collect_2q_runs()
        .map_or(0, |runs| runs.len())
}

/// Copies the contiguous two-qubit gate runs in CSR style: run nodes
/// flattened into `nodes` and run boundaries into `offsets`
/// (`offsets[0] == 0`). Returns 0 on success, -1 on NULL input, -3 on a
/// broken timeline, -8 on undersized capacities.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_collect_2q_runs(
    ptr: *const CCircuitDag,
    nodes: *mut u32,
    nodes_cap: usize,
    offsets: *mut usize,
    offsets_cap: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match unsafe { &(*ptr).inner }.collect_2q_runs() {
        Ok(runs) => copy_node_groups(&runs, nodes, nodes_cap, offsets, offsets_cap),
        Err(_) => -3,
    }
}

/// Returns the number of contiguous gate runs on `wire` matching
/// `criterion` (`DAG_RUN_CRITERION_1Q_GATE` or `DAG_RUN_CRITERION_2Q_GATE`),
/// or 0 for NULL input, an unknown wire tag, an unknown criterion, or
/// failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_collect_runs_on_wire_len(
    ptr: *const CCircuitDag,
    tag: u32,
    id: u32,
    criterion: u32,
) -> usize {
    match wire_runs_checked(ptr, tag, id, criterion) {
        Ok(runs) => runs.len(),
        Err(_) => 0,
    }
}

fn wire_runs_checked(
    ptr: *const CCircuitDag,
    tag: u32,
    id: u32,
    criterion: u32,
) -> Result<Vec<Vec<NodeIndex>>, i32> {
    let wire = check_wire(ptr, tag, id)?;
    let dag = unsafe { &(*ptr).inner };
    match criterion {
        DAG_RUN_CRITERION_1Q_GATE => dag
            .collect_runs_on_wire(wire, is_one_qubit_gate_c)
            .map_err(|_| -3),
        DAG_RUN_CRITERION_2Q_GATE => dag
            .collect_runs_on_wire(wire, is_two_qubit_gate_c)
            .map_err(|_| -3),
        _ => Err(-8),
    }
}

/// Copies the contiguous gate runs on `wire` matching `criterion` in CSR
/// style: run nodes flattened into `nodes` and run boundaries into
/// `offsets` (`offsets[0] == 0`). Returns 0 on success, -1 on NULL input,
/// -3 when the wire is invalid or the timeline is broken, -8 for an
/// unknown wire tag or criterion, or undersized capacities.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_collect_runs_on_wire(
    ptr: *const CCircuitDag,
    tag: u32,
    id: u32,
    criterion: u32,
    nodes: *mut u32,
    nodes_cap: usize,
    offsets: *mut usize,
    offsets_cap: usize,
) -> i32 {
    match wire_runs_checked(ptr, tag, id, criterion) {
        Ok(runs) => copy_node_groups(&runs, nodes, nodes_cap, offsets, offsets_cap),
        Err(code) => code,
    }
}

// =====  Mutating the DAG  =====

/// Appends a copy of `operation` at the back of the DAG and writes the
/// new node into `out_node` unless NULL. Returns 0 on success, -1 on NULL
/// input, -3 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_apply_operation_back(
    ptr: *mut CCircuitDag,
    operation: *const COperation,
    out_node: *mut u32,
) -> i32 {
    let dag = if !ptr.is_null() {
        unsafe { &mut (*ptr).inner }
    } else {
        return -1;
    };
    if operation.is_null() {
        return -1;
    }
    let applied = unsafe { (*operation).inner.clone() };
    match dag.apply_operation_back(applied) {
        Ok(node) => {
            write_node_out(out_node, node);
            0
        }
        Err(_) => -3,
    }
}

/// Lowers and appends a copy of the self-contained value-level operation
/// `operation` at the back of the DAG and writes the new node into
/// `out_node` unless NULL. Returns 0 on success, -1 on NULL input, -3 on
/// failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_apply_value_operation_back(
    ptr: *mut CCircuitDag,
    operation: *const CValueOperation,
    out_node: *mut u32,
) -> i32 {
    let dag = if !ptr.is_null() {
        unsafe { &mut (*ptr).inner }
    } else {
        return -1;
    };
    if operation.is_null() {
        return -1;
    }
    let applied = unsafe { (*operation).inner.clone() };
    match dag.apply_value_operation_back(applied) {
        Ok(node) => {
            write_node_out(out_node, node);
            0
        }
        Err(_) => -3,
    }
}

/// Prepends a copy of `operation` at the front of the DAG and writes the
/// new node into `out_node` unless NULL. Returns 0 on success, -1 on NULL
/// input, -3 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_apply_operation_front(
    ptr: *mut CCircuitDag,
    operation: *const COperation,
    out_node: *mut u32,
) -> i32 {
    let dag = if !ptr.is_null() {
        unsafe { &mut (*ptr).inner }
    } else {
        return -1;
    };
    if operation.is_null() {
        return -1;
    }
    let applied = unsafe { (*operation).inner.clone() };
    match dag.apply_operation_front(applied) {
        Ok(node) => {
            write_node_out(out_node, node);
            0
        }
        Err(_) => -3,
    }
}

/// Lowers and prepends a copy of the self-contained value-level operation
/// `operation` at the front of the DAG and writes the new node into
/// `out_node` unless NULL. Returns 0 on success, -1 on NULL input, -3 on
/// failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_apply_value_operation_front(
    ptr: *mut CCircuitDag,
    operation: *const CValueOperation,
    out_node: *mut u32,
) -> i32 {
    let dag = if !ptr.is_null() {
        unsafe { &mut (*ptr).inner }
    } else {
        return -1;
    };
    if operation.is_null() {
        return -1;
    }
    let applied = unsafe { (*operation).inner.clone() };
    match dag.apply_value_operation_front(applied) {
        Ok(node) => {
            write_node_out(out_node, node);
            0
        }
        Err(_) => -3,
    }
}

fn write_node_out(out_node: *mut u32, node: NodeIndex) {
    if !out_node.is_null() {
        unsafe { *out_node = node.index() as u32 };
    }
}

/// Removes the operation node `node` and returns the removed operation as
/// a newly allocated `COperation*` (free with `operation_free`). Returns
/// NULL on NULL input or when the node is missing or not an operation
/// node.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_remove_op_node(ptr: *mut CCircuitDag, node: u32) -> *mut COperation {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { &mut (*ptr).inner }.remove_op_node(NodeIndex::new(node as usize)) {
        Ok(removed) => Box::into_raw(Box::new(COperation { inner: removed })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Replaces the operation node `node` with a copy of `operation`.
/// Returns 0 on success, -1 on NULL input, -2 when `node` is not an
/// operation node, -3 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_substitute_node(
    ptr: *mut CCircuitDag,
    node: u32,
    operation: *const COperation,
) -> i32 {
    let dag = if !ptr.is_null() {
        unsafe { &mut (*ptr).inner }
    } else {
        return -1;
    };
    if operation.is_null() {
        return -1;
    }
    if !dag.is_operation(NodeIndex::new(node as usize)) {
        return -2;
    }
    match dag.substitute_node(NodeIndex::new(node as usize), unsafe {
        (*operation).inner.clone()
    }) {
        Ok(()) => 0,
        Err(_) => -3,
    }
}

/// Lowers the self-contained value-level operation `operation` and
/// replaces the operation node `node` with it. Returns 0 on success, -1 on
/// NULL input, -2 when `node` is not an operation node, -3 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_substitute_value_node(
    ptr: *mut CCircuitDag,
    node: u32,
    operation: *const CValueOperation,
) -> i32 {
    let dag = if !ptr.is_null() {
        unsafe { &mut (*ptr).inner }
    } else {
        return -1;
    };
    if operation.is_null() {
        return -1;
    }
    if !dag.is_operation(NodeIndex::new(node as usize)) {
        return -2;
    }
    match dag.substitute_value_node(NodeIndex::new(node as usize), unsafe {
        (*operation).inner.clone()
    }) {
        Ok(()) => 0,
        Err(_) => -3,
    }
}

/// Replaces the operation node `node` with a copy of `replacement`.
/// Returns 0 on success, -1 on NULL input, -2 when `node` is not an
/// operation node, -3 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_substitute_node_with_dag(
    ptr: *mut CCircuitDag,
    node: u32,
    replacement: *const CCircuitDag,
) -> i32 {
    let dag = if !ptr.is_null() {
        unsafe { &mut (*ptr).inner }
    } else {
        return -1;
    };
    if replacement.is_null() {
        return -1;
    }
    if !dag.is_operation(NodeIndex::new(node as usize)) {
        return -2;
    }
    let replacement = unsafe { (*replacement).inner.clone() };
    match dag.substitute_node_with_dag(NodeIndex::new(node as usize), replacement) {
        Ok(()) => 0,
        Err(_) => -3,
    }
}

// =====  Control-flow inspection  =====

/// Snapshots the recursive control-flow structure attached to `node`. On
/// success writes a freshly allocated `CDagControlFlow*` into `out` (free
/// with `dag_control_flow_free`) and returns 1. Returns 0 when `node` is
/// an ordinary operation, -1 on NULL input, -2 for an unknown or
/// non-operation node.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_dag_control_flow(
    ptr: *const CCircuitDag,
    node: u32,
    out: *mut *mut CDagControlFlow,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let dag = unsafe { &(*ptr).inner };
    unsafe { *out = std::ptr::null_mut() };
    if !dag.is_operation(NodeIndex::new(node as usize)) {
        return -2;
    }
    match dag.control_flow(NodeIndex::new(node as usize)) {
        Some(flow) => {
            unsafe {
                *out = Box::into_raw(Box::new(CDagControlFlow {
                    inner: flow.clone(),
                }));
            }
            1
        }
        None => 0,
    }
}

/// Returns the control-flow kind tag (`DAG_CONTROL_FLOW_*`), or -1 on NULL
/// input.
#[unsafe(no_mangle)]
pub extern "C" fn dag_control_flow_kind(ptr: *const CDagControlFlow) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    control_flow_kind_tag(unsafe { &(*ptr).inner }) as i32
}

/// Returns the number of body DAGs: `if` carries its `then` and optional
/// `else` bodies, `while` and `for` one body each, `switch` one per case,
/// and `break`/`continue` none. Returns 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn dag_control_flow_len(ptr: *const CDagControlFlow) -> usize {
    if ptr.is_null() {
        return 0;
    }
    control_flow_bodies(unsafe { &(*ptr).inner }).len()
}

/// Returns a copy of body `index` as a newly allocated `CCircuitDag*`
/// (free with `circuit_dag_free`). Returns NULL on NULL input or
/// out-of-range index.
#[unsafe(no_mangle)]
pub extern "C" fn dag_control_flow_body(
    ptr: *const CDagControlFlow,
    index: usize,
) -> *mut CCircuitDag {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let bodies = control_flow_bodies(unsafe { &(*ptr).inner });
    match bodies.get(index) {
        Some(body) => Box::into_raw(Box::new(CCircuitDag {
            inner: (*body).clone(),
        })),
        None => std::ptr::null_mut(),
    }
}

/// Returns whether the `switch` carries a default body: 1 when present, 0
/// otherwise, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn dag_control_flow_has_default(ptr: *const CDagControlFlow) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match unsafe { &(*ptr).inner } {
        DagControlFlow::Switch { default, .. } => default.is_some() as i32,
        _ => 0,
    }
}

/// Returns a copy of a `switch` default body as a newly allocated
/// `CCircuitDag*` (free with `circuit_dag_free`). Returns NULL on NULL
/// input, when the structure is not a `switch`, or when it has no default.
#[unsafe(no_mangle)]
pub extern "C" fn dag_control_flow_default_body(ptr: *const CDagControlFlow) -> *mut CCircuitDag {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { &(*ptr).inner } {
        DagControlFlow::Switch {
            default: Some(default),
            ..
        } => Box::into_raw(Box::new(CCircuitDag {
            inner: default.as_ref().clone(),
        })),
        _ => std::ptr::null_mut(),
    }
}

/// Writes the 128-bit case value of switch body `index` into `lo` and
/// `hi`. Returns 0 on success, -1 on NULL input, -8 for an out-of-range
/// index or when the structure is not a `switch`.
#[unsafe(no_mangle)]
pub extern "C" fn dag_control_flow_body_value(
    ptr: *const CDagControlFlow,
    index: usize,
    lo: *mut u64,
    hi: *mut u64,
) -> i32 {
    if ptr.is_null() || lo.is_null() || hi.is_null() {
        return -1;
    }
    match unsafe { &(*ptr).inner } {
        DagControlFlow::Switch { cases, .. } => match cases.get(index) {
            Some(case) => {
                let value = case.value;
                unsafe {
                    *lo = value as u64;
                    *hi = (value >> 64) as u64;
                }
                0
            }
            None => -8,
        },
        _ => -8,
    }
}

/// Frees a control-flow snapshot handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn dag_control_flow_free(ptr: *mut CDagControlFlow) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}
