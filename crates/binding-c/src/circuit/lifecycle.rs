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

//! C ABI for circuit lifecycle: construction, structural queries, and
//! checkpoint transactions.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::circuit::CCircuit;
use crate::circuit::advanced::CValueOperation;
use cqlib_core::circuit::Circuit;
use cqlib_core::circuit::{ControlBodyTransaction, Qubit};

/// Create a new quantum circuit with `num_qubits` logical qubits.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_new(num_qubits: usize) -> *mut CCircuit {
    Box::into_raw(Box::new(CCircuit {
        inner: Circuit::new(num_qubits),
    }))
}

/// Free a quantum circuit. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_free(ptr: *mut CCircuit) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Return the number of qubits in the circuit, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_num_qubits(ptr: *const CCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Return the number of operations in the circuit, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_num_operations(ptr: *const CCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.operations().len() }
}

/// Return the number of interned symbolic parameters in the circuit, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_num_parameters(ptr: *const CCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.parameters().len() }
}

/// Validate circuit consistency.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_validate(ptr: *const CCircuit) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.validate().map_or(-3, |_| 0) }
}

// =====  Structural construction and queries  =====

/// Creates a circuit from qubits plus a batch of resolved value-level
/// operations (as produced by `circuit_index`), mirroring the core
/// `Circuit::from_operations`. Operations are appended in order through
/// value-level lowering, so qubit membership is validated and symbolic
/// parameters are interned into the new circuit's parameter table. Returns
/// a newly allocated `CCircuit*` (free with `circuit_free`), or NULL on
/// NULL input (including NULL entries inside `operations`), duplicate
/// qubits, or any operation-level failure.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_from_operations(
    qubits: *const u32,
    qubits_len: usize,
    operations: *const *const CValueOperation,
    operations_len: usize,
) -> *mut CCircuit {
    if qubits_len > 0 && qubits.is_null() {
        return std::ptr::null_mut();
    }
    if operations_len > 0 && operations.is_null() {
        return std::ptr::null_mut();
    }
    let qubit_list: Vec<Qubit> = if qubits_len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(qubits, qubits_len) }
            .iter()
            .map(|&id| Qubit::new(id))
            .collect()
    };
    let mut operation_list = Vec::with_capacity(operations_len);
    for i in 0..operations_len {
        let handle = unsafe { *operations.add(i) };
        if handle.is_null() {
            return std::ptr::null_mut();
        }
        operation_list.push(unsafe { (*handle).inner.clone() });
    }
    match Circuit::from_operations(qubit_list, operation_list, None, None) {
        Ok(inner) => Box::into_raw(Box::new(CCircuit { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Appends one resolved value-level operation (as produced by
/// `circuit_index`) to the circuit. Returns 0 on success, -1 on NULL
/// input, -3 on application failure (e.g. the operation references a qubit
/// outside the circuit).
#[unsafe(no_mangle)]
pub extern "C" fn circuit_append_value_operation(
    ptr: *mut CCircuit,
    op: *const CValueOperation,
) -> i32 {
    if ptr.is_null() || op.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let operation = unsafe { (*op).inner.clone() };
    wrapper
        .inner
        .append_value_operation(operation)
        .map_or(-3, |_| 0)
}

/// Returns whether `qubit` belongs to the circuit: 1 when yes, 0 when no,
/// -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_contains_qubit(ptr: *const CCircuit, qubit: u32) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let qubit = Qubit::new(qubit);
    unsafe { (*ptr).inner.qubits().contains(&qubit) as i32 }
}

/// Returns whether both circuits share the same ordered qubit domain:
/// 1 when yes, 0 when no, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_has_same_qubits(a: *const CCircuit, b: *const CCircuit) -> i32 {
    if a.is_null() || b.is_null() {
        return -1;
    }
    unsafe { ((*a).inner.qubits() == (*b).inner.qubits()) as i32 }
}

/// Returns whether two circuits are structurally equal, mirroring the
/// core `PartialEq for Circuit`: same ordered qubits, parameter tables, and
/// operations (with parameter indices remapped across the two tables, and
/// circuit-local classical handles compared by structure). Returns 1 when
/// yes, 0 when no, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_operations_structurally_equal(
    a: *const CCircuit,
    b: *const CCircuit,
) -> i32 {
    if a.is_null() || b.is_null() {
        return -1;
    }
    unsafe { ((*a).inner == (*b).inner) as i32 }
}

// =====  Checkpoint transactions  =====

/// Opaque rollback token around a core `ControlBodyTransaction`, produced
/// by `circuit_checkpoint` / `circuit_begin` and consumed exactly once by
/// `circuit_commit`, `circuit_rollback_to`, or
/// `circuit_rollback_control_body_transaction`. Free an unused token with
/// `checkpoint_free`.
pub struct CCheckpoint {
    pub inner: ControlBodyTransaction,
}

/// Captures a checkpoint of the circuit state (operation list, parameter
/// and symbol tables, classical tables, control scopes). The returned
/// token is consumed exactly once by `circuit_commit`,
/// `circuit_rollback_to`, or `circuit_rollback_control_body_transaction`.
/// Returns a newly allocated `CCheckpoint*`, or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_checkpoint(ptr: *const CCircuit) -> *mut CCheckpoint {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &*ptr };
    Box::into_raw(Box::new(CCheckpoint {
        inner: wrapper.inner.begin_control_body_transaction(),
    }))
}

/// Starts an externally driven construction transaction; equivalent to
/// [`circuit_checkpoint`]. Returns a newly allocated `CCheckpoint*`, or
/// NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_begin(ptr: *const CCircuit) -> *mut CCheckpoint {
    circuit_checkpoint(ptr)
}

/// Commits the state allocated since the token was captured and consumes
/// the token. Returns 0 on success, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_commit(ptr: *mut CCircuit, checkpoint: *mut CCheckpoint) -> i32 {
    if ptr.is_null() || checkpoint.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let token = unsafe { Box::from_raw(checkpoint) };
    wrapper.inner.commit_control_body_transaction(token.inner);
    0
}

/// Rolls back every state allocated since the token was captured and
/// consumes the token. Exposed through the transaction token because the
/// core keeps the raw checkpoint type private. Returns 0 on success, -1 on
/// NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_rollback_to(ptr: *mut CCircuit, checkpoint: *mut CCheckpoint) -> i32 {
    circuit_rollback_control_body_transaction(ptr, checkpoint)
}

/// Rolls back every state allocated since the token was captured and
/// consumes the token (core `rollback_control_body_transaction`). Returns
/// 0 on success, -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_rollback_control_body_transaction(
    ptr: *mut CCircuit,
    checkpoint: *mut CCheckpoint,
) -> i32 {
    if ptr.is_null() || checkpoint.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let token = unsafe { Box::from_raw(checkpoint) };
    wrapper.inner.rollback_control_body_transaction(token.inner);
    0
}

/// Frees an unconsumed checkpoint token. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn checkpoint_free(ptr: *mut CCheckpoint) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}
