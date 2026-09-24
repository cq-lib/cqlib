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

//! C ABI for structured control-flow graphs ([`CircuitCFG`]).
//!
//! The CFG is a graph view of a circuit: every conditional, loop, and switch
//! header owns a control-flow region recording body entries, merge blocks,
//! and outer operation metadata. Typical usage is
//! `circuit_cfg_from_circuit` -> inspect blocks/terminators/edges/regions ->
//! optionally rebuild -> `circuit_cfg_to_circuit`.
//!
//! Blocks are addressed by stable node indices (u32). `UINT32_MAX` is the
//! "missing block / missing entry" sentinel. Classical conditions are
//! exchanged through [`CClassicalExpr`] / [`CClassicalVar`] handles from the
//! `classical` module.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::circuit::CCircuit;
use crate::circuit::classical::classical_type_parts;
use crate::circuit::classical::{CClassicalExpr, CClassicalType, CClassicalVar};
use crate::circuit::operations::COperation;
use cqlib_core::circuit::Qubit;
use cqlib_core::circuit::cfg::{
    BasicBlock, CircuitCFG, ControlFlowRegion, FlowEdge, OperationMetadata, SwitchRegionCase,
    Terminator,
};
use rustworkx_core::petgraph::prelude::NodeIndex;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Opaque handle around a [`CircuitCFG`].
pub struct CCircuitCFG {
    pub inner: CircuitCFG,
}

// =====  Flow-edge tags  =====

/// Outgoing-edge tag: true edge out of `if` / `while` / `for` headers.
pub const CFG_FLOW_TRUE_BRANCH: u32 = 1;
/// Outgoing-edge tag: false edge out of `if` / `while` / `for` headers.
pub const CFG_FLOW_FALSE_BRANCH: u32 = 2;
/// Outgoing-edge tag: fallthrough jump / structured merge.
pub const CFG_FLOW_UNCONDITIONAL: u32 = 3;
/// Outgoing-edge tag: exact-value switch case (payload in case value halves).
pub const CFG_FLOW_CASE: u32 = 4;
/// Outgoing-edge tag: switch default edge.
pub const CFG_FLOW_DEFAULT_CASE: u32 = 5;
/// Outgoing-edge tag: structured `break` edge.
pub const CFG_FLOW_BREAK: u32 = 6;
/// Outgoing-edge tag: structured `continue` edge.
pub const CFG_FLOW_CONTINUE: u32 = 7;

// =====  Terminator tags  =====

/// Terminator tag: boolean branch header (`Branch`).
pub const CFG_TERMINATOR_BRANCH: u32 = 1;
/// Terminator tag: unsigned range loop header (`ForLoop`).
pub const CFG_TERMINATOR_FOR_LOOP: u32 = 2;
/// Terminator tag: exact-value multi-way branch header (`Switch`).
pub const CFG_TERMINATOR_SWITCH: u32 = 3;
/// Terminator tag: unconditional jump.
pub const CFG_TERMINATOR_JUMP: u32 = 4;
/// Terminator tag: structured break.
pub const CFG_TERMINATOR_BREAK: u32 = 5;
/// Terminator tag: structured continue.
pub const CFG_TERMINATOR_CONTINUE: u32 = 6;
/// Terminator tag: end of execution.
pub const CFG_TERMINATOR_RETURN: u32 = 7;

// =====  Region tags  =====

/// Region tag: structured conditional region.
pub const CFG_REGION_IF: u32 = 1;
/// Region tag: structured while-loop region.
pub const CFG_REGION_WHILE: u32 = 2;
/// Region tag: structured range-loop region.
pub const CFG_REGION_FOR: u32 = 3;
/// Region tag: structured exact-value switch region.
pub const CFG_REGION_SWITCH: u32 = 4;

// =====  Internal helpers  =====

/// Validates a C block index and converts it to a stable graph index.
fn node_index(node: u32) -> Option<NodeIndex> {
    if node == u32::MAX {
        return None;
    }
    Some(NodeIndex::new(node as usize))
}

/// Looks up a block by C node index (read-only).
fn find_block(cfg: &CircuitCFG, node: u32) -> Option<&BasicBlock> {
    let index = node_index(node)?;
    cfg.blocks()
        .find_map(|(i, block)| (i == index).then_some(block))
}

/// Converts a flow-edge tag plus packed case value into a core edge weight.
fn flow_from_tag(tag: u32, case_lo: u64, case_hi: u64) -> Option<FlowEdge> {
    match tag {
        CFG_FLOW_TRUE_BRANCH => Some(FlowEdge::TrueBranch),
        CFG_FLOW_FALSE_BRANCH => Some(FlowEdge::FalseBranch),
        CFG_FLOW_UNCONDITIONAL => Some(FlowEdge::Unconditional),
        CFG_FLOW_CASE => Some(FlowEdge::Case(
            u128::from(case_lo) | (u128::from(case_hi) << 64),
        )),
        CFG_FLOW_DEFAULT_CASE => Some(FlowEdge::DefaultCase),
        CFG_FLOW_BREAK => Some(FlowEdge::Break),
        CFG_FLOW_CONTINUE => Some(FlowEdge::Continue),
        _ => None,
    }
}

/// Converts a core edge weight into a tag plus packed case value halves.
fn flow_parts(flow: &FlowEdge) -> (u32, u64, u64) {
    match flow {
        FlowEdge::TrueBranch => (CFG_FLOW_TRUE_BRANCH, 0, 0),
        FlowEdge::FalseBranch => (CFG_FLOW_FALSE_BRANCH, 0, 0),
        FlowEdge::Unconditional => (CFG_FLOW_UNCONDITIONAL, 0, 0),
        FlowEdge::Case(value) => (CFG_FLOW_CASE, *value as u64, (*value >> 64) as u64),
        FlowEdge::DefaultCase => (CFG_FLOW_DEFAULT_CASE, 0, 0),
        FlowEdge::Break => (CFG_FLOW_BREAK, 0, 0),
        FlowEdge::Continue => (CFG_FLOW_CONTINUE, 0, 0),
    }
}

/// Maps a terminator to its C tag.
fn terminator_tag(terminator: &Terminator) -> u32 {
    match terminator {
        Terminator::Branch(_) => CFG_TERMINATOR_BRANCH,
        Terminator::ForLoop { .. } => CFG_TERMINATOR_FOR_LOOP,
        Terminator::Switch(_) => CFG_TERMINATOR_SWITCH,
        Terminator::Jump(_) => CFG_TERMINATOR_JUMP,
        Terminator::Break(_) => CFG_TERMINATOR_BREAK,
        Terminator::Continue(_) => CFG_TERMINATOR_CONTINUE,
        Terminator::Return => CFG_TERMINATOR_RETURN,
    }
}

/// Maps a control-flow region to its C tag.
fn region_tag(region: &ControlFlowRegion) -> u32 {
    match region {
        ControlFlowRegion::If { .. } => CFG_REGION_IF,
        ControlFlowRegion::While { .. } => CFG_REGION_WHILE,
        ControlFlowRegion::For { .. } => CFG_REGION_FOR,
        ControlFlowRegion::Switch { .. } => CFG_REGION_SWITCH,
    }
}

/// Extracts outer operation metadata, or empty metadata for NULL.
fn outer_metadata(outer: *const COperation) -> OperationMetadata {
    if outer.is_null() {
        return OperationMetadata {
            qubits: Default::default(),
            params: Default::default(),
            label: None,
        };
    }
    let op = unsafe { &(*outer).inner };
    OperationMetadata {
        qubits: op.qubits.clone(),
        params: op.params.clone(),
        label: op.label.clone(),
    }
}

// =====  Lifecycle and round trip  =====

/// Creates an empty CFG with densely numbered qubits `0..num_qubits`.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_new(num_qubits: usize) -> *mut CCircuitCFG {
    Box::into_raw(Box::new(CCircuitCFG {
        inner: CircuitCFG::new(num_qubits),
    }))
}

/// Creates an empty CFG with the supplied logical qubit ids in insertion
/// order. Returns a newly allocated `CCircuitCFG*` (free with
/// `circuit_cfg_free`), or NULL on NULL `qubits` when `qubits_len > 0`.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_from_qubits(
    qubits: *const u32,
    qubits_len: usize,
) -> *mut CCircuitCFG {
    if qubits_len > 0 && qubits.is_null() {
        return std::ptr::null_mut();
    }
    let list = if qubits_len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(qubits, qubits_len) }
            .iter()
            .map(|&id| Qubit::new(id))
            .collect()
    };
    Box::into_raw(Box::new(CCircuitCFG {
        inner: CircuitCFG::from_qubits(list),
    }))
}

/// Expands a structured circuit into a validated CFG. Returns a newly
/// allocated `CCircuitCFG*` (free with `circuit_cfg_free`), or NULL on NULL
/// input or build failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_from_circuit(ptr: *const CCircuit) -> *mut CCircuitCFG {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match CircuitCFG::from_circuit(unsafe { &(*ptr).inner }) {
        Ok(inner) => Box::into_raw(Box::new(CCircuitCFG { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a CFG handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_free(ptr: *mut CCircuitCFG) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Reconstructs a structured circuit from this CFG. Returns a newly
/// allocated `CCircuit*` (free with `circuit_free`), or NULL on NULL input
/// or lowering failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_to_circuit(ptr: *const CCircuitCFG) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { (*ptr).inner.to_circuit() } {
        Ok(inner) => Box::into_raw(Box::new(CCircuit { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Checks graph, operation, parameter, edge, terminator, and region
/// invariants. Returns 0 on success, -1 on NULL input, -3 on validation
/// failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_validate(ptr: *const CCircuitCFG) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.validate().map_or(-3, |_| 0) }
}

// =====  Global properties  =====

/// Returns the number of logical qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_num_qubits(ptr: *const CCircuitCFG) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Returns the number of blocks currently stored in the graph, or 0 for
/// NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_num_blocks(ptr: *const CCircuitCFG) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_blocks() }
}

/// Copies the logical qubit ids in insertion order into `buffer`. Returns 0
/// on success, -1 on NULL input, -8 when `len` is smaller than
/// `circuit_cfg_num_qubits`.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_qubits(ptr: *const CCircuitCFG, buffer: *mut u32, len: usize) -> i32 {
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

/// Returns the number of mutable classical variables in the static type
/// table, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_classical_vars_len(ptr: *const CCircuitCFG) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.classical_vars().len() }
}

/// Copies the mutable classical variable types as (tag, width) snapshots
/// into `buffer`. Returns 0 on success, -1 on NULL input, -8 when `len` is
/// too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_classical_vars(
    ptr: *const CCircuitCFG,
    buffer: *mut CClassicalType,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let types = unsafe { (*ptr).inner.classical_vars() };
    if len < types.len() {
        return -8;
    }
    if types.is_empty() {
        return 0;
    }
    if buffer.is_null() {
        return -1;
    }
    for (i, ty) in types.iter().enumerate() {
        let (tag, width) = classical_type_parts(*ty);
        unsafe {
            *buffer.add(i) = CClassicalType { tag, width };
        }
    }
    0
}

/// Returns the number of immutable classical values in the static type
/// table, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_classical_values_len(ptr: *const CCircuitCFG) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.classical_values().len() }
}

/// Copies the immutable classical value types as (tag, width) snapshots
/// into `buffer`. Returns 0 on success, -1 on NULL input, -8 when `len` is
/// too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_classical_values(
    ptr: *const CCircuitCFG,
    buffer: *mut CClassicalType,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let types = unsafe { (*ptr).inner.classical_values() };
    if len < types.len() {
        return -8;
    }
    if types.is_empty() {
        return 0;
    }
    if buffer.is_null() {
        return -1;
    }
    for (i, ty) in types.iter().enumerate() {
        let (tag, width) = classical_type_parts(*ty);
        unsafe {
            *buffer.add(i) = CClassicalType { tag, width };
        }
    }
    0
}

/// Returns the designated entry block, or `UINT32_MAX` when none has been
/// set or the handle is NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_entry_block(ptr: *const CCircuitCFG) -> u32 {
    if ptr.is_null() {
        return u32::MAX;
    }
    unsafe {
        (*ptr)
            .inner
            .entry_block()
            .map_or(u32::MAX, |i| i.index() as u32)
    }
}

// =====  Construction  =====

/// Adds an empty block and returns its stable node index, or `UINT32_MAX`
/// on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_add_block(ptr: *mut CCircuitCFG) -> u32 {
    if ptr.is_null() {
        return u32::MAX;
    }
    let wrapper = unsafe { &mut *ptr };
    wrapper.inner.add_block(BasicBlock::new()).index() as u32
}

/// Adds a directed semantic edge between existing blocks. `flow` is one of
/// the `CFG_FLOW_*` tags; for `CFG_FLOW_CASE` the 128-bit case value is
/// passed as two 64-bit halves (lo least-significant). Returns 0 on
/// success, -1 on NULL input, -2 when either endpoint does not exist, -8
/// on an unknown flow tag.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_add_edge(
    ptr: *mut CCircuitCFG,
    source: u32,
    target: u32,
    flow: u32,
    case_lo: u64,
    case_hi: u64,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let Some(flow) = flow_from_tag(flow, case_lo, case_hi) else {
        return -8;
    };
    let Some(source) = node_index(source) else {
        return -2;
    };
    let Some(target) = node_index(target) else {
        return -2;
    };
    let wrapper = unsafe { &mut *ptr };
    match wrapper.inner.add_edge(source, target, flow) {
        Some(_) => 0,
        None => -2,
    }
}

/// Sets the graph entry block. The index is only checked by
/// `circuit_cfg_validate`. Returns 0 on success, -1 on NULL input, -2 on
/// the sentinel index.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_set_entry_block(ptr: *mut CCircuitCFG, node: u32) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let Some(index) = node_index(node) else {
        return -2;
    };
    let wrapper = unsafe { &mut *ptr };
    wrapper.inner.set_entry_block(index);
    0
}

/// Appends one ordinary (non-control-flow) operation to a block. Returns 0
/// on success, -1 on NULL input or NULL `op`, -2 when the block does not
/// exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_push_operation(
    ptr: *mut CCircuitCFG,
    node: u32,
    op: *const COperation,
) -> i32 {
    if ptr.is_null() || op.is_null() {
        return -1;
    }
    let Some(index) = node_index(node) else {
        return -2;
    };
    let operation = unsafe { (*op).inner.clone() };
    let wrapper = unsafe { &mut *ptr };
    match wrapper.inner.block_mut(index) {
        Some(block) => {
            block.push_operation(operation);
            0
        }
        None => -2,
    }
}

/// Appends a batch of cloned operations to the end of block `node`.
///
/// Every handle in `ops` is validated and cloned before any mutation, so the
/// block is left untouched when any entry is NULL. Returns 0 on success,
/// -1 on NULL input, -2 when `node` does not address a block.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_extend_operations(
    ptr: *mut CCircuitCFG,
    node: u32,
    ops: *const *const COperation,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    if len > 0 && ops.is_null() {
        return -1;
    }
    let slice = if len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(ops, len) }
    };
    let mut batch = Vec::with_capacity(len);
    for &op in slice {
        if op.is_null() {
            return -1;
        }
        batch.push(unsafe { (*op).inner.clone() });
    }
    let Some(index) = node_index(node) else {
        return -2;
    };
    let wrapper = unsafe { &mut *ptr };
    match wrapper.inner.block_mut(index) {
        Some(block) => {
            block.extend_operations(batch);
            0
        }
        None => -2,
    }
}

/// Sets or replaces the label of block `node`.
///
/// Returns 0 on success, -1 on NULL input, -2 when `node` does not address
/// a block, -4 when `label` is not valid UTF-8.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_with_label(
    ptr: *mut CCircuitCFG,
    node: u32,
    label: *const c_char,
) -> i32 {
    if ptr.is_null() || label.is_null() {
        return -1;
    }
    let text = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(text) => text,
        Err(_) => return -4,
    };
    let Some(index) = node_index(node) else {
        return -2;
    };
    let wrapper = unsafe { &mut *ptr };
    match wrapper.inner.block_mut(index) {
        Some(block) => {
            let labeled = block.clone().with_label(text);
            *block = labeled;
            0
        }
        None => -2,
    }
}

// =====  Block enumeration and inspection  =====

/// Copies the stable node indices of all blocks in graph order into
/// `buffer`. Returns 0 on success, -1 on NULL input, -8 when `len` is
/// smaller than `circuit_cfg_num_blocks`.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_blocks(ptr: *const CCircuitCFG, buffer: *mut u32, len: usize) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &*ptr };
    let count = wrapper.inner.num_blocks();
    if len < count {
        return -8;
    }
    if count == 0 {
        return 0;
    }
    if buffer.is_null() {
        return -1;
    }
    for (i, (index, _)) in wrapper.inner.blocks().enumerate() {
        unsafe {
            *buffer.add(i) = index.index() as u32;
        }
    }
    0
}

/// Returns the block's diagnostic label as a newly allocated C string
/// (free with `cqlib_string_free`), NULL when the block is unlabeled, does
/// not exist, or the handle is NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_block_label(ptr: *const CCircuitCFG, node: u32) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &*ptr };
    let Some(block) = find_block(&wrapper.inner, node) else {
        return std::ptr::null_mut();
    };
    match block.label().and_then(|label| CString::new(label).ok()) {
        Some(text) => text.into_raw(),
        None => std::ptr::null_mut(),
    }
}

/// Returns whether the block has neither operations nor a terminator: 1
/// when empty, 0 otherwise, -1 on NULL input, -2 when the block does not
/// exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_block_is_empty(ptr: *const CCircuitCFG, node: u32) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &*ptr };
    match find_block(&wrapper.inner, node) {
        Some(block) => block.is_empty() as i32,
        None => -2,
    }
}

/// Returns the number of ordinary operations (terminator excluded) in the
/// block, or 0 on NULL input or when the block does not exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_block_operations_len(ptr: *const CCircuitCFG, node: u32) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let wrapper = unsafe { &*ptr };
    find_block(&wrapper.inner, node).map_or(0, |block| block.len())
}

/// Clones the block's ordinary operations into `out`: each written element
/// is a freshly allocated `COperation*` freed separately with
/// `operation_free`. Returns 0 on success, -1 on NULL input, -2 when the
/// block does not exist, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_block_operations(
    ptr: *const CCircuitCFG,
    node: u32,
    out: *mut *mut COperation,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &*ptr };
    let Some(block) = find_block(&wrapper.inner, node) else {
        return -2;
    };
    let operations = block.operations();
    if len < operations.len() {
        return -8;
    }
    if operations.is_empty() {
        return 0;
    }
    if out.is_null() {
        return -1;
    }
    for (i, operation) in operations.iter().enumerate() {
        let handle = Box::into_raw(Box::new(COperation {
            inner: operation.clone(),
        }));
        unsafe {
            *out.add(i) = handle;
        }
    }
    0
}

/// Returns whether a terminator has been assigned: 1 when assigned, 0
/// when not, -1 on NULL input, -2 when the block does not exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_block_has_terminator(ptr: *const CCircuitCFG, node: u32) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &*ptr };
    match find_block(&wrapper.inner, node) {
        Some(block) => block.has_terminator() as i32,
        None => -2,
    }
}

/// Returns the block terminator tag: one of the `CFG_TERMINATOR_*`
/// constants, 0 when the block has no terminator, -1 on NULL input, -2
/// when the block does not exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_block_terminator_tag(ptr: *const CCircuitCFG, node: u32) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &*ptr };
    match find_block(&wrapper.inner, node) {
        Some(block) => block
            .terminator()
            .map_or(0, |terminator| terminator_tag(terminator) as i32),
        None => -2,
    }
}

// =====  Terminator detail queries  =====

/// Returns the jump target of a `Jump`, `Break`, or `Continue` terminator,
/// or `UINT32_MAX` for other terminators, missing terminators, NULL
/// input, or unknown blocks.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_terminator_target(ptr: *const CCircuitCFG, node: u32) -> u32 {
    if ptr.is_null() {
        return u32::MAX;
    }
    let wrapper = unsafe { &*ptr };
    let Some(block) = find_block(&wrapper.inner, node) else {
        return u32::MAX;
    };
    match block.terminator() {
        Some(Terminator::Jump(target))
        | Some(Terminator::Break(target))
        | Some(Terminator::Continue(target)) => target.index() as u32,
        _ => u32::MAX,
    }
}

/// Clones the condition of a `Branch` or `Switch` terminator into a newly
/// allocated `CClassicalExpr*` (free with `classical_expr_free`). Returns
/// NULL on NULL input, unknown block, missing terminator, or non-branching
/// terminators.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_terminator_condition(
    ptr: *const CCircuitCFG,
    node: u32,
) -> *mut CClassicalExpr {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &*ptr };
    let Some(block) = find_block(&wrapper.inner, node) else {
        return std::ptr::null_mut();
    };
    match block.terminator() {
        Some(Terminator::Branch(condition)) | Some(Terminator::Switch(condition)) => {
            Box::into_raw(Box::new(CClassicalExpr {
                inner: condition.clone(),
            }))
        }
        _ => std::ptr::null_mut(),
    }
}

/// Writes cloned handles describing a `ForLoop` terminator: the loop
/// variable plus the start, stop, and step expressions. `var` is released
/// with `classical_var_free` and the expressions with
/// `classical_expr_free`. Returns 0 on success, -1 on NULL input, -2 when
/// the block does not exist, -8 when the terminator is not a loop header.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_terminator_for_info(
    ptr: *const CCircuitCFG,
    node: u32,
    var: *mut *mut CClassicalVar,
    start: *mut *mut CClassicalExpr,
    stop: *mut *mut CClassicalExpr,
    step: *mut *mut CClassicalExpr,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &*ptr };
    let Some(block) = find_block(&wrapper.inner, node) else {
        return -2;
    };
    let Some(Terminator::ForLoop {
        var: loop_var,
        start: start_expr,
        stop: stop_expr,
        step: step_expr,
    }) = block.terminator()
    else {
        return -8;
    };
    if var.is_null() || start.is_null() || stop.is_null() || step.is_null() {
        return -1;
    }
    unsafe {
        *var = Box::into_raw(Box::new(CClassicalVar { inner: *loop_var }));
        *start = Box::into_raw(Box::new(CClassicalExpr {
            inner: start_expr.clone(),
        }));
        *stop = Box::into_raw(Box::new(CClassicalExpr {
            inner: stop_expr.clone(),
        }));
        *step = Box::into_raw(Box::new(CClassicalExpr {
            inner: step_expr.clone(),
        }));
    }
    0
}

// =====  Edge queries  =====

/// Returns the number of outgoing edges of a block, or 0 on NULL input or
/// when the block does not exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_outgoing_edges_len(ptr: *const CCircuitCFG, node: u32) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let wrapper = unsafe { &*ptr };
    let Some(index) = node_index(node) else {
        return 0;
    };
    if find_block(&wrapper.inner, node).is_none() {
        return 0;
    }
    wrapper.inner.outgoing_edges(index).count()
}

/// Copies the outgoing edges of a block into parallel arrays: `targets`
/// receives node indices, `flow_tags` receives `CFG_FLOW_*` tags, and
/// `case_lo` / `case_hi` receive the 128-bit case value for
/// `CFG_FLOW_CASE` edges (zeros otherwise). Returns 0 on success, -1 on
/// NULL input, -2 when the block does not exist, -8 when `len` is too
/// small.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_outgoing_edges(
    ptr: *const CCircuitCFG,
    node: u32,
    out_targets: *mut u32,
    flow_tags: *mut u32,
    case_lo: *mut u64,
    case_hi: *mut u64,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &*ptr };
    let Some(index) = node_index(node) else {
        return -2;
    };
    if find_block(&wrapper.inner, node).is_none() {
        return -2;
    }
    let mut edges: Vec<(u32, u32, u64, u64)> = wrapper
        .inner
        .outgoing_edges(index)
        .map(|(target, flow)| {
            let (tag, lo, hi) = flow_parts(&flow);
            (target.index() as u32, tag, lo, hi)
        })
        .collect();
    // petgraph iterates a node's edges in reverse insertion order; reverse
    // back so edges are written in insertion order (TrueBranch before
    // FalseBranch for conditional headers).
    edges.reverse();
    if len < edges.len() {
        return -8;
    }
    if edges.is_empty() {
        return 0;
    }
    if out_targets.is_null() || flow_tags.is_null() || case_lo.is_null() || case_hi.is_null() {
        return -1;
    }
    for (i, (target, tag, lo, hi)) in edges.iter().enumerate() {
        unsafe {
            *out_targets.add(i) = *target;
            *flow_tags.add(i) = *tag;
            *case_lo.add(i) = *lo;
            *case_hi.add(i) = *hi;
        }
    }
    0
}

// =====  Region queries  =====

/// Returns the control-flow region tag owned by a block: one of the
/// `CFG_REGION_*` constants, 0 when the block owns no region, -1 on NULL
/// input, -2 when the block does not exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_region_tag(ptr: *const CCircuitCFG, node: u32) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &*ptr };
    let Some(index) = node_index(node) else {
        return -2;
    };
    if find_block(&wrapper.inner, node).is_none() {
        return -2;
    }
    wrapper
        .inner
        .control_flow_region(index)
        .map_or(0, |region| region_tag(region) as i32)
}

/// Returns whether the block owns a while or for region: 1 when it is a
/// loop header, 0 when not, -1 on NULL input, -2 when the block does not
/// exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_is_loop_header(ptr: *const CCircuitCFG, node: u32) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &*ptr };
    let Some(index) = node_index(node) else {
        return -2;
    };
    if find_block(&wrapper.inner, node).is_none() {
        return -2;
    }
    wrapper.inner.is_loop_header(index) as i32
}

/// Writes the `If` region layout: `then_entry`, `else_entry` (or the merge
/// block when no else body exists), `merge_block`, plus `has_else` (1/0).
/// Returns 0 on success, -1 on NULL input, -2 when the block does not
/// exist, -8 when the block owns no `If` region.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_region_if(
    ptr: *const CCircuitCFG,
    node: u32,
    then_entry: *mut u32,
    else_entry: *mut u32,
    merge_block: *mut u32,
    has_else: *mut i32,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &*ptr };
    let Some(index) = node_index(node) else {
        return -2;
    };
    let Some(ControlFlowRegion::If {
        then_entry: then_index,
        else_entry: else_index,
        merge_block: merge_index,
        has_else: has_else_body,
        ..
    }) = wrapper.inner.control_flow_region(index)
    else {
        return match find_block(&wrapper.inner, node) {
            Some(_) => -8,
            None => -2,
        };
    };
    if then_entry.is_null() || else_entry.is_null() || merge_block.is_null() || has_else.is_null() {
        return -1;
    }
    unsafe {
        *then_entry = then_index.index() as u32;
        *else_entry = else_index.index() as u32;
        *merge_block = merge_index.index() as u32;
        *has_else = *has_else_body as i32;
    }
    0
}

/// Writes the `While` / `For` region layout: `body_entry` and
/// `exit_block`. Returns 0 on success, -1 on NULL input, -2 when the block
/// does not exist, -8 when the block owns no loop region.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_region_loop(
    ptr: *const CCircuitCFG,
    node: u32,
    body_entry: *mut u32,
    exit_block: *mut u32,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &*ptr };
    let Some(index) = node_index(node) else {
        return -2;
    };
    let Some(region) = wrapper.inner.control_flow_region(index) else {
        return match find_block(&wrapper.inner, node) {
            Some(_) => -8,
            None => -2,
        };
    };
    let (body, exit) = match region {
        ControlFlowRegion::While {
            body_entry,
            exit_block,
            ..
        }
        | ControlFlowRegion::For {
            body_entry,
            exit_block,
            ..
        } => (body_entry, exit_block),
        ControlFlowRegion::If { .. } | ControlFlowRegion::Switch { .. } => {
            return -8;
        }
    };
    if body_entry.is_null() || exit_block.is_null() {
        return -1;
    }
    unsafe {
        *body_entry = body.index() as u32;
        *exit_block = exit.index() as u32;
    }
    0
}

/// Returns the number of explicit cases of a `Switch` region, or 0 on NULL
/// input, unknown block, or non-switch region.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_region_switch_cases_len(ptr: *const CCircuitCFG, node: u32) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let wrapper = unsafe { &*ptr };
    let Some(index) = node_index(node) else {
        return 0;
    };
    match wrapper.inner.control_flow_region(index) {
        Some(ControlFlowRegion::Switch { cases, .. }) => cases.len(),
        _ => 0,
    }
}

/// Writes the `Switch` region layout: `case_lo[i]` / `case_hi[i]` receive
/// the 128-bit match value of case `i`, `case_entries[i]` its body entry
/// block; `default_entry` (or the merge block when no default body
/// exists), `merge_block`, and `has_default` (1/0). Returns 0 on success,
/// -1 on NULL input, -2 when the block does not exist, -8 when the block
/// owns no `Switch` region or `len` is smaller than the case count.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_region_switch(
    ptr: *const CCircuitCFG,
    node: u32,
    case_lo: *mut u64,
    case_hi: *mut u64,
    case_entries: *mut u32,
    len: usize,
    default_entry: *mut u32,
    merge_block: *mut u32,
    has_default: *mut i32,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &*ptr };
    let Some(index) = node_index(node) else {
        return -2;
    };
    let Some(ControlFlowRegion::Switch {
        cases,
        default_entry: default_index,
        merge_block: merge_index,
        has_default: has_default_body,
        ..
    }) = wrapper.inner.control_flow_region(index)
    else {
        return match find_block(&wrapper.inner, node) {
            Some(_) => -8,
            None => -2,
        };
    };
    if len < cases.len() {
        return -8;
    }
    if default_entry.is_null() || merge_block.is_null() || has_default.is_null() {
        return -1;
    }
    if !cases.is_empty() {
        if case_lo.is_null() || case_hi.is_null() || case_entries.is_null() {
            return -1;
        }
        for (i, case) in cases.iter().enumerate() {
            unsafe {
                *case_lo.add(i) = case.value as u64;
                *case_hi.add(i) = (case.value >> 64) as u64;
                *case_entries.add(i) = case.entry.index() as u32;
            }
        }
    }
    unsafe {
        *default_entry = default_index.index() as u32;
        *merge_block = merge_index.index() as u32;
        *has_default = *has_default_body as i32;
    }
    0
}

// =====  Region setters  =====

/// Guards region attachment: resolves the header block and stores the
/// region produced by `build` (as [`CircuitCFG::set_control_flow_region`]
/// does, entry blocks are only cross-checked by `circuit_cfg_validate`).
/// Returns 0 on success, -1 on NULL input, -2 when the header block does
/// not exist.
fn set_control_flow_region(
    ptr: *mut CCircuitCFG,
    node: u32,
    build: impl FnOnce() -> ControlFlowRegion,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let Some(index) = node_index(node) else {
        return -2;
    };
    let wrapper = unsafe { &mut *ptr };
    if wrapper.inner.block_mut(index).is_none() {
        return -2;
    }
    wrapper.inner.set_control_flow_region(index, build());
    0
}

/// Attaches an `If` region to a header block. `outer_op` supplies the
/// outer operation metadata and may be NULL (empty metadata). Entry blocks
/// are only checked by `circuit_cfg_validate`. Returns 0 on success, -1 on
/// NULL input, -2 when the header block does not exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_set_if_region(
    ptr: *mut CCircuitCFG,
    node: u32,
    then_entry: u32,
    else_entry: u32,
    merge_block: u32,
    has_else: i32,
    outer_op: *const COperation,
) -> i32 {
    set_control_flow_region(ptr, node, move || ControlFlowRegion::If {
        then_entry: NodeIndex::new(then_entry as usize),
        else_entry: NodeIndex::new(else_entry as usize),
        merge_block: NodeIndex::new(merge_block as usize),
        has_else: has_else != 0,
        outer: outer_metadata(outer_op),
    })
}

/// Attaches a `While` region to a header block. `outer_op` supplies the
/// outer operation metadata and may be NULL. Entry blocks are only checked
/// by `circuit_cfg_validate`. Returns 0 on success, -1 on NULL input, -2
/// when the header block does not exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_set_while_region(
    ptr: *mut CCircuitCFG,
    node: u32,
    body_entry: u32,
    exit_block: u32,
    outer_op: *const COperation,
) -> i32 {
    set_control_flow_region(ptr, node, move || ControlFlowRegion::While {
        body_entry: NodeIndex::new(body_entry as usize),
        exit_block: NodeIndex::new(exit_block as usize),
        outer: outer_metadata(outer_op),
    })
}

/// Attaches a `For` region to a header block. `outer_op` supplies the
/// outer operation metadata and may be NULL. Entry blocks are only checked
/// by `circuit_cfg_validate`. Returns 0 on success, -1 on NULL input, -2
/// when the header block does not exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_set_for_region(
    ptr: *mut CCircuitCFG,
    node: u32,
    body_entry: u32,
    exit_block: u32,
    outer_op: *const COperation,
) -> i32 {
    set_control_flow_region(ptr, node, move || ControlFlowRegion::For {
        body_entry: NodeIndex::new(body_entry as usize),
        exit_block: NodeIndex::new(exit_block as usize),
        outer: outer_metadata(outer_op),
    })
}

/// Attaches a `Switch` region to a header block. Case `i` pairs
/// `case_lo[i]` (least-significant) with `case_hi[i]` as its 128-bit match
/// value and `case_entries[i]` as its body entry block. `outer_op` supplies
/// the outer operation metadata and may be NULL. Entry blocks are only
/// checked by `circuit_cfg_validate`. Returns 0 on success, -1 on NULL
/// input or NULL case arrays when `len > 0`, -2 when the header block does
/// not exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_set_switch_region(
    ptr: *mut CCircuitCFG,
    node: u32,
    case_lo: *const u64,
    case_hi: *const u64,
    case_entries: *const u32,
    len: usize,
    default_entry: u32,
    merge_block: u32,
    has_default: i32,
    outer_op: *const COperation,
) -> i32 {
    let cases = if len == 0 {
        Vec::new()
    } else {
        if case_lo.is_null() || case_hi.is_null() || case_entries.is_null() {
            return -1;
        }
        unsafe {
            let lo = std::slice::from_raw_parts(case_lo, len);
            let hi = std::slice::from_raw_parts(case_hi, len);
            let entries = std::slice::from_raw_parts(case_entries, len);
            lo.iter()
                .zip(hi.iter())
                .zip(entries.iter())
                .map(|((&lo, &hi), &entry)| SwitchRegionCase {
                    value: u128::from(lo) | (u128::from(hi) << 64),
                    entry: NodeIndex::new(entry as usize),
                })
                .collect()
        }
    };
    set_control_flow_region(ptr, node, move || ControlFlowRegion::Switch {
        cases,
        default_entry: NodeIndex::new(default_entry as usize),
        merge_block: NodeIndex::new(merge_block as usize),
        has_default: has_default != 0,
        outer: outer_metadata(outer_op),
    })
}

// =====  Terminator setters  =====

/// Guards terminator assignment: resolves the block and applies the
/// terminator produced by `build`. Returns 0 on success, -1 on NULL input,
/// -2 when the block does not exist.
fn set_terminator(ptr: *mut CCircuitCFG, node: u32, build: impl FnOnce() -> Terminator) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let Some(index) = node_index(node) else {
        return -2;
    };
    let wrapper = unsafe { &mut *ptr };
    match wrapper.inner.block_mut(index) {
        Some(block) => {
            block.set_terminator(build());
            0
        }
        None => -2,
    }
}

/// Replaces the block terminator with an unconditional jump. Returns 0 on
/// success, -1 on NULL input, -2 when the block does not exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_set_terminator_jump(
    ptr: *mut CCircuitCFG,
    node: u32,
    target: u32,
) -> i32 {
    set_terminator(ptr, node, || {
        Terminator::Jump(NodeIndex::new(target as usize))
    })
}

/// Replaces the block terminator with `Return` (end of execution).
/// Returns 0 on success, -1 on NULL input, -2 when the block does not
/// exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_set_terminator_return(ptr: *mut CCircuitCFG, node: u32) -> i32 {
    set_terminator(ptr, node, || Terminator::Return)
}

/// Replaces the block terminator with a structured `Break` referencing the
/// broken loop's header. Returns 0 on success, -1 on NULL input, -2 when
/// the block does not exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_set_terminator_break(
    ptr: *mut CCircuitCFG,
    node: u32,
    target: u32,
) -> i32 {
    set_terminator(ptr, node, || {
        Terminator::Break(NodeIndex::new(target as usize))
    })
}

/// Replaces the block terminator with a structured `Continue` referencing
/// the loop header. Returns 0 on success, -1 on NULL input, -2 when the
/// block does not exist.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_cfg_set_terminator_continue(
    ptr: *mut CCircuitCFG,
    node: u32,
    target: u32,
) -> i32 {
    set_terminator(ptr, node, || {
        Terminator::Continue(NodeIndex::new(target as usize))
    })
}
