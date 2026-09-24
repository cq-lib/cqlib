// This code is part of Cqlib.
//
// (C) Copyright China Telecom Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! C ABI for ancillary-resource management: policies, limits, preview/commit
//! leases, and manager consistency checks.

use crate::circuit::CCircuit;
use crate::error::CqlibError;
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::compile::resource::{
    AncillaRequirement, ResourceLease, ResourceLimits, ResourceManager, ResourcePlan,
    ResourcePolicy, ResourceRequest,
};
use std::collections::BTreeSet;

/// Ancilla-requirement tag.
/// | Value | Requirement                                        |
/// |-------|----------------------------------------------------|
/// | 0     | Clean zero (enter and leave in `\|0>`)             |
/// | 1     | Dirty (unknown input state, exact restore)         |
pub const RESOURCE_REQUIREMENT_CLEAN_ZERO: u8 = 0;
pub const RESOURCE_REQUIREMENT_DIRTY: u8 = 1;

/// C form of [`ResourcePolicy`].
#[repr(C)]
pub struct CResourcePolicy {
    /// Total clean logical ancillas the compiler may create before layout.
    pub max_pre_layout_clean_ancillas: usize,
    /// 1 allows borrowing input qubits under the dirty contract.
    pub allow_dirty_borrowing: u8,
    /// Reserved padding to keep the struct layout stable.
    pub _reserved: [u8; 7],
}

/// C form of [`ResourceLimits`].
#[repr(C)]
pub struct CResourceLimits {
    /// 1 enforces `max_total_qubits`, 0 leaves the limit unset.
    pub has_max_total_qubits: u8,
    /// Reserved padding to keep the struct layout stable.
    pub _pad: [u8; 7],
    /// Hard limit on total logical qubits (used when `has_max_total_qubits != 0`).
    pub max_total_qubits: usize,
    /// Reserved padding to keep the struct layout stable.
    pub _reserved: [u64; 2],
}

/// Opaque handle around a [`ResourceManager`] plus its synchronized working
/// circuit.
///
/// The manager must evolve together with its circuit, so the C handle owns a
/// private clone of the source circuit. Use [`resource_manager_circuit`] to
/// take out the current working circuit.
pub struct CResourceManager {
    /// Working copy of the managed circuit (mutated by `commit`).
    pub circuit: Circuit,
    /// The resource manager bound to `circuit`.
    pub inner: ResourceManager,
}

/// Opaque handle around a [`ResourcePlan`].
pub struct CResourcePlan {
    pub inner: ResourcePlan,
}

/// Opaque handle around a [`ResourceLease`].
pub struct CResourceLease {
    pub inner: ResourceLease,
}

/// Returns the default resource policy by value.
#[unsafe(no_mangle)]
pub extern "C" fn resource_policy_default() -> CResourcePolicy {
    CResourcePolicy {
        max_pre_layout_clean_ancillas: 0,
        allow_dirty_borrowing: 0,
        _reserved: [0; 7],
    }
}

/// Returns the default resource limits by value (no total-qubit bound).
#[unsafe(no_mangle)]
pub extern "C" fn resource_limits_default() -> CResourceLimits {
    CResourceLimits {
        has_max_total_qubits: 0,
        _pad: [0; 7],
        max_total_qubits: 0,
        _reserved: [0; 2],
    }
}

/// Converts a C resource policy into the core form.
fn resource_policy_from_c(raw: &CResourcePolicy) -> ResourcePolicy {
    ResourcePolicy {
        max_pre_layout_clean_ancillas: raw.max_pre_layout_clean_ancillas,
        allow_dirty_borrowing: raw.allow_dirty_borrowing != 0,
    }
}

/// Converts a C resource-limits struct into the core form.
fn resource_limits_from_c(raw: &CResourceLimits) -> ResourceLimits {
    ResourceLimits {
        max_total_qubits: if raw.has_max_total_qubits != 0 {
            Some(raw.max_total_qubits)
        } else {
            None
        },
    }
}

/// Creates a resource manager synchronized with a private clone of `circuit`.
///
/// `policy` and `limits` may be NULL to select the core defaults. Returns a
/// heap-allocated `CResourceManager*`, or NULL on NULL input or when the
/// circuit already exceeds the configured limits.
#[unsafe(no_mangle)]
pub extern "C" fn resource_manager_from_circuit(
    circuit: *const CCircuit,
    policy: *const CResourcePolicy,
    limits: *const CResourceLimits,
) -> *mut CResourceManager {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let policy = if policy.is_null() {
        ResourcePolicy::default()
    } else {
        resource_policy_from_c(unsafe { &*policy })
    };
    let limits = if limits.is_null() {
        ResourceLimits::default()
    } else {
        resource_limits_from_c(unsafe { &*limits })
    };
    let circuit = unsafe { &(*circuit).inner };
    match ResourceManager::from_circuit(circuit, policy, limits) {
        Ok(inner) => Box::into_raw(Box::new(CResourceManager {
            circuit: circuit.clone(),
            inner,
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a resource manager. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn resource_manager_free(ptr: *mut CResourceManager) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the manager's current working circuit as an owned `CCircuit*`
/// (free with `circuit_free`). The manager keeps its own copy, so the
/// returned handle is independent. Returns NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn resource_manager_circuit(ptr: *const CResourceManager) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(CCircuit {
        inner: unsafe { (*ptr).circuit.clone() },
    }))
}

/// Previews an ancilla request against the manager without reserving
/// resources or mutating the working circuit.
///
/// `requirement` is one of `RESOURCE_REQUIREMENT_*`; `excluded` lists qubits
/// that must not be consumed as ancillary resources (may be NULL when
/// `excluded_len` is 0). Returns a heap-allocated `CResourcePlan*` (free with
/// `resource_plan_free`), or NULL on NULL input, an invalid requirement tag,
/// or an unsatisfiable request.
#[unsafe(no_mangle)]
pub extern "C" fn resource_manager_preview(
    ptr: *const CResourceManager,
    requirement: u8,
    count: usize,
    excluded: *const u32,
    excluded_len: usize,
) -> *mut CResourcePlan {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let requirement = match requirement {
        RESOURCE_REQUIREMENT_CLEAN_ZERO => AncillaRequirement::CleanZero,
        RESOURCE_REQUIREMENT_DIRTY => AncillaRequirement::Dirty,
        _ => return std::ptr::null_mut(),
    };
    let mut excluded_set = BTreeSet::new();
    if excluded_len > 0 {
        if excluded.is_null() {
            return std::ptr::null_mut();
        }
        let slice = unsafe { std::slice::from_raw_parts(excluded, excluded_len) };
        for &id in slice {
            excluded_set.insert(Qubit::new(id));
        }
    }
    let manager = unsafe { &(*ptr).inner };
    let request = ResourceRequest {
        requirement,
        count,
        excluded: excluded_set,
    };
    match manager.preview(&request) {
        Ok(inner) => Box::into_raw(Box::new(CResourcePlan { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a resource plan. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn resource_plan_free(ptr: *mut CResourcePlan) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the number of planned qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn resource_plan_qubits_len(ptr: *const CResourcePlan) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.qubits().len() }
}

/// Copies the planned qubits into `out` (two-step pattern; call
/// `resource_plan_qubits_len` first). Returns the total number of planned
/// qubits, or 0 for NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn resource_plan_qubits(
    ptr: *const CResourcePlan,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let plan = unsafe { &(*ptr).inner };
    let qubits: Vec<u32> = plan.qubits().iter().map(|qubit| qubit.id()).collect();
    crate::device::write_u32_list(&qubits, out, len)
}

/// Returns the number of new logical qubits the plan introduces when
/// committed, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn resource_plan_num_new_qubits(ptr: *const CResourcePlan) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_new_qubits() }
}

/// Returns the plan's ancilla requirement (one of `RESOURCE_REQUIREMENT_*`),
/// or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn resource_plan_requirement(ptr: *const CResourcePlan) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    match unsafe { (*ptr).inner.requirement() } {
        AncillaRequirement::CleanZero => RESOURCE_REQUIREMENT_CLEAN_ZERO as i32,
        AncillaRequirement::Dirty => RESOURCE_REQUIREMENT_DIRTY as i32,
    }
}

/// Commits a previewed plan: reserves its qubits, mutates the manager's
/// working circuit, and returns the active lease.
///
/// The plan handle is consumed (freed) by this call. Returns a
/// heap-allocated `CResourceLease*` (free with `resource_lease_free` after
/// `resource_manager_release`), or NULL on NULL input or a stale/invalid
/// plan.
#[unsafe(no_mangle)]
pub extern "C" fn resource_manager_commit(
    ptr: *mut CResourceManager,
    plan: *mut CResourcePlan,
) -> *mut CResourceLease {
    if ptr.is_null() || plan.is_null() {
        return std::ptr::null_mut();
    }
    let manager = unsafe { &mut *ptr };
    let plan = unsafe { Box::from_raw(plan) };
    match manager.inner.commit(&mut manager.circuit, plan.inner) {
        Ok(inner) => Box::into_raw(Box::new(CResourceLease { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a resource lease. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn resource_lease_free(ptr: *mut CResourceLease) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the number of leased qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn resource_lease_qubits_len(ptr: *const CResourceLease) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.qubits().len() }
}

/// Copies the leased qubits into `out` (two-step pattern; call
/// `resource_lease_qubits_len` first). Returns the total number of leased
/// qubits, or 0 for NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn resource_lease_qubits(
    ptr: *const CResourceLease,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let lease = unsafe { &(*ptr).inner };
    let qubits: Vec<u32> = lease.qubits().iter().map(|qubit| qubit.id()).collect();
    crate::device::write_u32_list(&qubits, out, len)
}

/// Returns the lease's ancilla requirement (one of `RESOURCE_REQUIREMENT_*`),
/// or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn resource_lease_requirement(ptr: *const CResourceLease) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    match unsafe { (*ptr).inner.requirement() } {
        AncillaRequirement::CleanZero => RESOURCE_REQUIREMENT_CLEAN_ZERO as i32,
        AncillaRequirement::Dirty => RESOURCE_REQUIREMENT_DIRTY as i32,
    }
}

/// Releases an active lease, returning its qubits to the resource pool.
///
/// The consuming transform must have restored the lease's state contract
/// before this call. The lease handle stays valid for inspection until freed
/// with `resource_lease_free`. Returns 0 on success, -1 on NULL input, -6 on
/// an unknown or already-released lease.
#[unsafe(no_mangle)]
pub extern "C" fn resource_manager_release(
    ptr: *mut CResourceManager,
    lease: *const CResourceLease,
) -> i32 {
    if ptr.is_null() || lease.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let manager = unsafe { &mut *ptr };
    let lease = unsafe { &(*lease).inner };
    match manager.inner.release(lease) {
        Ok(()) => CqlibError::Ok as i32,
        Err(_) => CqlibError::CompilerError as i32,
    }
}

/// Moves the manager into the post-layout phase (no fresh logical ancillas).
///
/// Returns 0 on success, -1 on NULL input, -6 on a consistency failure.
#[unsafe(no_mangle)]
pub extern "C" fn resource_manager_enter_post_layout(ptr: *mut CResourceManager) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let manager = unsafe { &mut *ptr };
    match manager.inner.enter_post_layout(&manager.circuit) {
        Ok(()) => CqlibError::Ok as i32,
        Err(_) => CqlibError::CompilerError as i32,
    }
}

/// Checks that the manager still agrees with its working circuit.
///
/// Returns 0 on success, -1 on NULL input, -6 on a mismatch.
#[unsafe(no_mangle)]
pub extern "C" fn resource_manager_verify_consistency(ptr: *const CResourceManager) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let manager = unsafe { &(*ptr) };
    match manager.inner.verify_consistency(&manager.circuit) {
        Ok(()) => CqlibError::Ok as i32,
        Err(_) => CqlibError::CompilerError as i32,
    }
}

/// Checks that every lease has been released and the pool is idle.
///
/// Returns 0 on success, -1 on NULL input, -6 when leases are still active.
#[unsafe(no_mangle)]
pub extern "C" fn resource_manager_verify_idle(ptr: *const CResourceManager) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let manager = unsafe { &(*ptr) };
    match manager.inner.verify_idle(&manager.circuit) {
        Ok(()) => CqlibError::Ok as i32,
        Err(_) => CqlibError::CompilerError as i32,
    }
}
