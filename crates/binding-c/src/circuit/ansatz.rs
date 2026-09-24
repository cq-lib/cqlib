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

//! C ABI for variational ansatz templates (TwoLocal, BasicEntanglerLayers,
//! QAOAAnsatz, feature maps, StronglyEntanglingLayers and the Pauli evolution
//! ansatz), facade constructors and entanglement topology helpers.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::circuit::CCircuit;
use crate::device::standard_gate_from_name;
use crate::qis::CHamiltonian;
use cqlib_core::circuit::ansatz::{
    Ansatz, BasicEntanglerLayers, EntanglementTopology, EvolutionStrategy, IQPFeatureMap,
    PauliEvolutionAnsatz, PauliFeatureMap, QAOAAnsatz, StronglyEntanglingLayers, TwoLocal,
    ZFeatureMap, ZZFeatureMap,
};
use cqlib_core::qis::PauliString;
use cqlib_core::qis::evolution::TrotterMode;
use std::ffi::CStr;
use std::os::raw::c_char;

// =====  Topology / strategy tags  =====

/// `two_local_entanglement` topology tag: linear nearest-neighbor chain.
pub const ENTANGLEMENT_LINEAR: i32 = 0;
/// Topology tag: linear chain plus wrap-around edge.
pub const ENTANGLEMENT_CIRCULAR: i32 = 1;
/// Topology tag: all-to-all pairs.
pub const ENTANGLEMENT_FULL: i32 = 2;
/// Topology tag: custom explicit pairs (flat control/target id array).
pub const ENTANGLEMENT_CUSTOM: i32 = 3;

/// `qaoa_ansatz_evolution_strategy` tag: exact term-wise evolution.
pub const EVOLUTION_STRATEGY_EXACT: i32 = 0;
/// Strategy tag: automatic exact/Trotter selection.
pub const EVOLUTION_STRATEGY_AUTO: i32 = 1;
/// Strategy tag: explicit Trotter product formula.
pub const EVOLUTION_STRATEGY_TROTTER: i32 = 2;

/// Trotter mode tag: first-order product formula.
pub const TROTTER_FIRST_ORDER: i32 = 0;
/// Trotter mode tag: second-order product formula.
pub const TROTTER_SECOND_ORDER: i32 = 1;
/// Trotter mode tag: randomized first-order formula.
pub const TROTTER_RANDOMIZED: i32 = 2;

/// `pauli_evolution_ansatz_evolution_info` `trotter_mode` tag: no Trotter
/// decomposition was emitted (single-pass exact path).
pub const TROTTER_MODE_NONE: i32 = -1;

// =====  Internal helpers  =====

fn parse_gate_name(name: *const c_char) -> Result<cqlib_core::circuit::StandardGate, i32> {
    if name.is_null() {
        return Err(-1);
    }
    let name = unsafe { CStr::from_ptr(name) }.to_str().map_err(|_| -4)?;
    standard_gate_from_name(name).ok_or(-8)
}

fn parse_prefix(prefix: *const c_char) -> Result<String, i32> {
    if prefix.is_null() {
        return Err(-1);
    }
    unsafe { CStr::from_ptr(prefix) }
        .to_str()
        .map(str::to_string)
        .map_err(|_| -4)
}

/// Parses a topology tag plus an optional flat custom-pair array into an
/// `EntanglementTopology`. Returns -1 on NULL input, -8 on an unknown tag or
/// an odd custom pair count.
fn parse_topology(
    topology: i32,
    pairs: *const u32,
    pairs_len: usize,
) -> Result<EntanglementTopology, i32> {
    match topology {
        ENTANGLEMENT_LINEAR => Ok(EntanglementTopology::Linear),
        ENTANGLEMENT_CIRCULAR => Ok(EntanglementTopology::Circular),
        ENTANGLEMENT_FULL => Ok(EntanglementTopology::Full),
        ENTANGLEMENT_CUSTOM => {
            if !pairs_len.is_multiple_of(2) {
                return Err(-8);
            }
            let mut list = Vec::with_capacity(pairs_len / 2);
            if pairs_len > 0 {
                if pairs.is_null() {
                    return Err(-1);
                }
                let slice = unsafe { std::slice::from_raw_parts(pairs, pairs_len) };
                for chunk in slice.chunks(2) {
                    list.push((chunk[0] as usize, chunk[1] as usize));
                }
            }
            Ok(EntanglementTopology::Custom(list))
        }
        _ => Err(-8),
    }
}

// =====  TwoLocal  =====

/// Opaque handle around a [`TwoLocal`] ansatz template.
pub struct CTwoLocal {
    pub inner: TwoLocal,
}

/// Creates a TwoLocal ansatz with defaults: `reps = 3`, rotation `[RY]`,
/// entangler `CX`, linear topology, final rotation layer kept.
#[unsafe(no_mangle)]
pub extern "C" fn two_local_new(num_qubits: usize) -> *mut CTwoLocal {
    Box::into_raw(Box::new(CTwoLocal {
        inner: TwoLocal::new(num_qubits),
    }))
}

/// Frees a TwoLocal handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn two_local_free(ptr: *mut CTwoLocal) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Sets the number of repetition layers. Returns 0, or -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn two_local_reps(ptr: *mut CTwoLocal, reps: usize) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let inner = unsafe { &mut (*ptr).inner };
    *inner = inner.clone().reps(reps);
    0
}

/// Sets the rotation gates from gate names (must be RX, RY or RZ; validated
/// on build). Returns 0, -1 on NULL input, -4 on invalid UTF-8, -8 on an
/// unknown gate name.
#[unsafe(no_mangle)]
pub extern "C" fn two_local_rotation_gates(
    ptr: *mut CTwoLocal,
    names: *const *const c_char,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    if len > 0 && names.is_null() {
        return -1;
    }
    let mut gates = Vec::with_capacity(len);
    for i in 0..len {
        let name = unsafe { *names.add(i) };
        match parse_gate_name(name) {
            Ok(gate) => gates.push(gate),
            Err(code) => return code,
        }
    }
    let inner = unsafe { &mut (*ptr).inner };
    *inner = inner.clone().rotation_gates(gates);
    0
}

/// Sets the entanglement gate by name (CX, CY or CZ; validated on build).
/// Returns 0, -1 on NULL input, -4 on invalid UTF-8, -8 on an unknown name.
#[unsafe(no_mangle)]
pub extern "C" fn two_local_entanglement_gate(ptr: *mut CTwoLocal, name: *const c_char) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match parse_gate_name(name) {
        Ok(gate) => {
            let inner = unsafe { &mut (*ptr).inner };
            *inner = inner.clone().entanglement_gate(gate);
            0
        }
        Err(code) => code,
    }
}

/// Sets the entanglement topology.
///
/// For `ENTANGLEMENT_CUSTOM`, `pairs` is a flat array of
/// `pairs_len / 2` control/target qubit-id pairs. Returns 0, -1 on NULL
/// input, -8 on an unknown topology or an odd pair count.
#[unsafe(no_mangle)]
pub extern "C" fn two_local_entanglement(
    ptr: *mut CTwoLocal,
    topology: i32,
    pairs: *const u32,
    pairs_len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match parse_topology(topology, pairs, pairs_len) {
        Ok(topo) => {
            let inner = unsafe { &mut (*ptr).inner };
            *inner = inner.clone().entanglement(topo);
            0
        }
        Err(code) => code,
    }
}

/// Sets whether to skip the final rotation layer. Returns 0, or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn two_local_skip_final_rotation_layer(ptr: *mut CTwoLocal, skip: bool) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let inner = unsafe { &mut (*ptr).inner };
    *inner = inner.clone().skip_final_rotation_layer(skip);
    0
}

/// Validates the ansatz configuration. Returns 0 on success, -1 on NULL
/// input, -3 on invalid configuration.
#[unsafe(no_mangle)]
pub extern "C" fn two_local_validate(ptr: *const CTwoLocal) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.validate().map_or(-3, |_| 0) }
}

/// Returns the number of independent parameters, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn two_local_num_parameters(ptr: *const CTwoLocal) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_parameters() }
}

/// Returns the number of qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn two_local_num_qubits(ptr: *const CTwoLocal) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Builds the parameterized circuit. Parameters are named
/// `"{prefix}_{index}"`. Returns a newly allocated `CCircuit*` (free with
/// `circuit_free`), or NULL on NULL input, invalid UTF-8 or build failure.
#[unsafe(no_mangle)]
pub extern "C" fn two_local_build_circuit(
    ptr: *const CTwoLocal,
    prefix: *const c_char,
) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let prefix = match parse_prefix(prefix) {
        Ok(prefix) => prefix,
        Err(_) => return std::ptr::null_mut(),
    };
    match unsafe { (*ptr).inner.build_circuit(&prefix) } {
        Ok(circuit) => Box::into_raw(Box::new(CCircuit { inner: circuit })),
        Err(_) => std::ptr::null_mut(),
    }
}

// =====  ZFeatureMap  =====

/// Opaque handle around a [`ZFeatureMap`] template.
pub struct CZFeatureMap {
    pub inner: ZFeatureMap,
}

/// Creates a ZFeatureMap with default `reps = 2`.
#[unsafe(no_mangle)]
pub extern "C" fn z_feature_map_new(num_qubits: usize) -> *mut CZFeatureMap {
    Box::into_raw(Box::new(CZFeatureMap {
        inner: ZFeatureMap::new(num_qubits),
    }))
}

/// Frees a ZFeatureMap handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn z_feature_map_free(ptr: *mut CZFeatureMap) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Sets the number of repetition layers. Returns 0, or -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn z_feature_map_reps(ptr: *mut CZFeatureMap, reps: usize) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let inner = unsafe { &mut (*ptr).inner };
    *inner = inner.clone().reps(reps);
    0
}

/// Validates the template. Returns 0 on success, -1 on NULL input, -3 on an
/// invalid configuration.
#[unsafe(no_mangle)]
pub extern "C" fn z_feature_map_validate(ptr: *const CZFeatureMap) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.validate().map_or(-3, |_| 0) }
}

/// Returns the number of independent parameters, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn z_feature_map_num_parameters(ptr: *const CZFeatureMap) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_parameters() }
}

/// Returns the number of qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn z_feature_map_num_qubits(ptr: *const CZFeatureMap) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Builds the parameterized circuit (parameters named `"{prefix}_{index}"`).
/// Returns a newly allocated `CCircuit*`, or NULL on NULL input, invalid
/// UTF-8 or build failure.
#[unsafe(no_mangle)]
pub extern "C" fn z_feature_map_build_circuit(
    ptr: *const CZFeatureMap,
    prefix: *const c_char,
) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let prefix = match parse_prefix(prefix) {
        Ok(prefix) => prefix,
        Err(_) => return std::ptr::null_mut(),
    };
    match unsafe { (*ptr).inner.build_circuit(&prefix) } {
        Ok(circuit) => Box::into_raw(Box::new(CCircuit { inner: circuit })),
        Err(_) => std::ptr::null_mut(),
    }
}

// =====  ZZFeatureMap  =====

/// Opaque handle around a [`ZZFeatureMap`] template.
pub struct CZZFeatureMap {
    pub inner: ZZFeatureMap,
}

/// Creates a ZZFeatureMap with defaults: `reps = 2`, full entanglement.
#[unsafe(no_mangle)]
pub extern "C" fn zz_feature_map_new(num_qubits: usize) -> *mut CZZFeatureMap {
    Box::into_raw(Box::new(CZZFeatureMap {
        inner: ZZFeatureMap::new(num_qubits),
    }))
}

/// Frees a ZZFeatureMap handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn zz_feature_map_free(ptr: *mut CZZFeatureMap) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Sets the number of repetition layers. Returns 0, or -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn zz_feature_map_reps(ptr: *mut CZZFeatureMap, reps: usize) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let inner = unsafe { &mut (*ptr).inner };
    *inner = inner.clone().reps(reps);
    0
}

/// Sets the entanglement topology.
///
/// For `ENTANGLEMENT_CUSTOM`, `pairs` is a flat array of
/// `pairs_len / 2` control/target qubit-id pairs. Returns 0, -1 on NULL
/// input, -8 on an unknown topology or an odd pair count.
#[unsafe(no_mangle)]
pub extern "C" fn zz_feature_map_entanglement(
    ptr: *mut CZZFeatureMap,
    topology: i32,
    pairs: *const u32,
    pairs_len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match parse_topology(topology, pairs, pairs_len) {
        Ok(topo) => {
            let inner = unsafe { &mut (*ptr).inner };
            *inner = inner.clone().entanglement(topo);
            0
        }
        Err(code) => code,
    }
}

/// Validates the template. Returns 0 on success, -1 on NULL input, -3 on an
/// invalid configuration.
#[unsafe(no_mangle)]
pub extern "C" fn zz_feature_map_validate(ptr: *const CZZFeatureMap) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.validate().map_or(-3, |_| 0) }
}

/// Returns the number of independent parameters, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn zz_feature_map_num_parameters(ptr: *const CZZFeatureMap) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_parameters() }
}

/// Returns the number of qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn zz_feature_map_num_qubits(ptr: *const CZZFeatureMap) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Builds the parameterized circuit (parameters named `"{prefix}_{index}"`).
/// Returns a newly allocated `CCircuit*`, or NULL on NULL input, invalid
/// UTF-8 or build failure.
#[unsafe(no_mangle)]
pub extern "C" fn zz_feature_map_build_circuit(
    ptr: *const CZZFeatureMap,
    prefix: *const c_char,
) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let prefix = match parse_prefix(prefix) {
        Ok(prefix) => prefix,
        Err(_) => return std::ptr::null_mut(),
    };
    match unsafe { (*ptr).inner.build_circuit(&prefix) } {
        Ok(circuit) => Box::into_raw(Box::new(CCircuit { inner: circuit })),
        Err(_) => std::ptr::null_mut(),
    }
}

// =====  IQPFeatureMap  =====

/// Opaque handle around an [`IQPFeatureMap`] template.
pub struct CIQPFeatureMap {
    pub inner: IQPFeatureMap,
}

/// Creates an IQPFeatureMap with defaults: `reps = 2`, full entanglement.
#[unsafe(no_mangle)]
pub extern "C" fn iqp_feature_map_new(num_qubits: usize) -> *mut CIQPFeatureMap {
    Box::into_raw(Box::new(CIQPFeatureMap {
        inner: IQPFeatureMap::new(num_qubits),
    }))
}

/// Frees an IQPFeatureMap handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn iqp_feature_map_free(ptr: *mut CIQPFeatureMap) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Sets the number of repetition layers. Returns 0, or -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn iqp_feature_map_reps(ptr: *mut CIQPFeatureMap, reps: usize) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let inner = unsafe { &mut (*ptr).inner };
    *inner = inner.clone().reps(reps);
    0
}

/// Sets the entanglement topology for the two-qubit diagonal interactions.
///
/// For `ENTANGLEMENT_CUSTOM`, `pairs` is a flat array of
/// `pairs_len / 2` control/target qubit-id pairs. Returns 0, -1 on NULL
/// input, -8 on an unknown topology or an odd pair count.
#[unsafe(no_mangle)]
pub extern "C" fn iqp_feature_map_entanglement(
    ptr: *mut CIQPFeatureMap,
    topology: i32,
    pairs: *const u32,
    pairs_len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match parse_topology(topology, pairs, pairs_len) {
        Ok(topo) => {
            let inner = unsafe { &mut (*ptr).inner };
            *inner = inner.clone().entanglement(topo);
            0
        }
        Err(code) => code,
    }
}

/// Validates the template. Returns 0 on success, -1 on NULL input, -3 on an
/// invalid configuration.
#[unsafe(no_mangle)]
pub extern "C" fn iqp_feature_map_validate(ptr: *const CIQPFeatureMap) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.validate().map_or(-3, |_| 0) }
}

/// Returns the number of independent parameters, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn iqp_feature_map_num_parameters(ptr: *const CIQPFeatureMap) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_parameters() }
}

/// Returns the number of qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn iqp_feature_map_num_qubits(ptr: *const CIQPFeatureMap) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Builds the parameterized circuit (parameters named `"{prefix}_{index}"`).
/// Returns a newly allocated `CCircuit*`, or NULL on NULL input, invalid
/// UTF-8 or build failure.
#[unsafe(no_mangle)]
pub extern "C" fn iqp_feature_map_build_circuit(
    ptr: *const CIQPFeatureMap,
    prefix: *const c_char,
) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let prefix = match parse_prefix(prefix) {
        Ok(prefix) => prefix,
        Err(_) => return std::ptr::null_mut(),
    };
    match unsafe { (*ptr).inner.build_circuit(&prefix) } {
        Ok(circuit) => Box::into_raw(Box::new(CCircuit { inner: circuit })),
        Err(_) => std::ptr::null_mut(),
    }
}

// =====  PauliFeatureMap  =====

/// Opaque handle around a [`PauliFeatureMap`] template.
pub struct CPauliFeatureMap {
    pub inner: PauliFeatureMap,
}

/// Creates a PauliFeatureMap with defaults: `reps = 2`, paulis `["Z", "ZZ"]`,
/// full entanglement and parameter prefix `"x"`.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_feature_map_new(num_qubits: usize) -> *mut CPauliFeatureMap {
    Box::into_raw(Box::new(CPauliFeatureMap {
        inner: PauliFeatureMap::new(num_qubits),
    }))
}

/// Frees a PauliFeatureMap handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_feature_map_free(ptr: *mut CPauliFeatureMap) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Sets the number of repetition layers. Returns 0, or -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_feature_map_reps(ptr: *mut CPauliFeatureMap, reps: usize) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let inner = unsafe { &mut (*ptr).inner };
    *inner = inner.clone().reps(reps);
    0
}

/// Sets the Pauli evolution templates.
///
/// `texts` and `labels` are parallel arrays of `len` NUL-terminated C
/// strings; each text is parsed as a `PauliString`. Returns 0, -1 on NULL
/// input, -4 on invalid UTF-8, -8 on a Pauli string that fails to parse.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_feature_map_paulis(
    ptr: *mut CPauliFeatureMap,
    texts: *const *const c_char,
    labels: *const *const c_char,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    if len > 0 && (texts.is_null() || labels.is_null()) {
        return -1;
    }
    let mut paulis = Vec::with_capacity(len);
    for i in 0..len {
        let text = unsafe { *texts.add(i) };
        let label = unsafe { *labels.add(i) };
        if text.is_null() || label.is_null() {
            return -1;
        }
        let text_str = match unsafe { CStr::from_ptr(text) }.to_str() {
            Ok(s) => s,
            Err(_) => return -4,
        };
        let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
            Ok(s) => s,
            Err(_) => return -4,
        };
        let pauli = match text_str.parse::<PauliString>() {
            Ok(p) => p,
            Err(_) => return -8,
        };
        paulis.push((pauli, label_str.to_string()));
    }
    let inner = unsafe { &mut (*ptr).inner };
    *inner = inner.clone().paulis(paulis);
    0
}

/// Sets the entanglement topology for the multi-qubit interactions.
///
/// For `ENTANGLEMENT_CUSTOM`, `pairs` is a flat array of
/// `pairs_len / 2` control/target qubit-id pairs. Returns 0, -1 on NULL
/// input, -8 on an unknown topology or an odd pair count.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_feature_map_entanglement(
    ptr: *mut CPauliFeatureMap,
    topology: i32,
    pairs: *const u32,
    pairs_len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match parse_topology(topology, pairs, pairs_len) {
        Ok(topo) => {
            let inner = unsafe { &mut (*ptr).inner };
            *inner = inner.clone().entanglement(topo);
            0
        }
        Err(code) => code,
    }
}

/// Sets the fallback parameter prefix used when `build_circuit` receives an
/// empty prefix. Returns 0, -1 on NULL input, -4 on invalid UTF-8.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_feature_map_parameter_prefix(
    ptr: *mut CPauliFeatureMap,
    prefix: *const c_char,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match parse_prefix(prefix) {
        Ok(prefix) => {
            let inner = unsafe { &mut (*ptr).inner };
            *inner = inner.clone().parameter_prefix(prefix);
            0
        }
        Err(code) => code,
    }
}

/// Validates the template. Returns 0 on success, -1 on NULL input, -3 on an
/// invalid configuration.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_feature_map_validate(ptr: *const CPauliFeatureMap) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.validate().map_or(-3, |_| 0) }
}

/// Returns the number of independent parameters, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_feature_map_num_parameters(ptr: *const CPauliFeatureMap) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_parameters() }
}

/// Returns the number of qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_feature_map_num_qubits(ptr: *const CPauliFeatureMap) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Builds the parameterized circuit (parameters named `"{prefix}_{index}"`;
/// an empty prefix falls back to the configured parameter prefix). Returns a
/// newly allocated `CCircuit*`, or NULL on NULL input, invalid UTF-8 or
/// build failure.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_feature_map_build_circuit(
    ptr: *const CPauliFeatureMap,
    prefix: *const c_char,
) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let prefix = match parse_prefix(prefix) {
        Ok(prefix) => prefix,
        Err(_) => return std::ptr::null_mut(),
    };
    match unsafe { (*ptr).inner.build_circuit(&prefix) } {
        Ok(circuit) => Box::into_raw(Box::new(CCircuit { inner: circuit })),
        Err(_) => std::ptr::null_mut(),
    }
}

// =====  Facade constructors  =====

/// Creates an EfficientSU2 ansatz: `[RY, RZ]` rotation layers with `CX`
/// entanglement. Returns a newly allocated `CTwoLocal*`, or NULL on an
/// invalid topology.
#[unsafe(no_mangle)]
pub extern "C" fn efficient_su2(
    num_qubits: usize,
    reps: usize,
    topology: i32,
    pairs: *const u32,
    pairs_len: usize,
) -> *mut CTwoLocal {
    match parse_topology(topology, pairs, pairs_len) {
        Ok(topo) => Box::into_raw(Box::new(CTwoLocal {
            inner: cqlib_core::circuit::ansatz::efficient_su2(num_qubits, reps, topo),
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Creates a RealAmplitudes ansatz: `[RY]` rotation layers with `CX`
/// entanglement. Returns a newly allocated `CTwoLocal*`, or NULL on an
/// invalid topology.
#[unsafe(no_mangle)]
pub extern "C" fn real_amplitudes(
    num_qubits: usize,
    reps: usize,
    topology: i32,
    pairs: *const u32,
    pairs_len: usize,
) -> *mut CTwoLocal {
    match parse_topology(topology, pairs, pairs_len) {
        Ok(topo) => Box::into_raw(Box::new(CTwoLocal {
            inner: cqlib_core::circuit::ansatz::real_amplitudes(num_qubits, reps, topo),
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Creates a ZZFeatureMap with the given topology. Named
/// `zz_feature_map_build` to avoid clashing with the `zz_feature_map_new`
/// type constructor. Returns a newly allocated `CZZFeatureMap*`, or NULL on
/// an invalid topology.
#[unsafe(no_mangle)]
pub extern "C" fn zz_feature_map_build(
    num_qubits: usize,
    reps: usize,
    topology: i32,
    pairs: *const u32,
    pairs_len: usize,
) -> *mut CZZFeatureMap {
    match parse_topology(topology, pairs, pairs_len) {
        Ok(topo) => Box::into_raw(Box::new(CZZFeatureMap {
            inner: cqlib_core::circuit::ansatz::zz_feature_map(num_qubits, reps, topo),
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Creates a PauliFeatureMap with explicit Pauli templates. `texts` and
/// `labels` are parallel arrays of `paulis_len` NUL-terminated C strings.
/// Named `pauli_feature_map_build` to avoid clashing with the
/// `pauli_feature_map_new` type constructor. Returns a newly allocated
/// `CPauliFeatureMap*`, or NULL on an invalid topology or Pauli string.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_feature_map_build(
    num_qubits: usize,
    reps: usize,
    texts: *const *const c_char,
    labels: *const *const c_char,
    paulis_len: usize,
    topology: i32,
    pairs: *const u32,
    pairs_len: usize,
) -> *mut CPauliFeatureMap {
    let paulis = parse_pauli_templates(texts, labels, paulis_len);
    let paulis = match paulis {
        Ok(paulis) => paulis,
        Err(_) => return std::ptr::null_mut(),
    };
    match parse_topology(topology, pairs, pairs_len) {
        Ok(topo) => Box::into_raw(Box::new(CPauliFeatureMap {
            inner: cqlib_core::circuit::ansatz::pauli_feature_map(num_qubits, reps, paulis, topo),
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Parses parallel Pauli-text/label arrays into core templates.
fn parse_pauli_templates(
    texts: *const *const c_char,
    labels: *const *const c_char,
    len: usize,
) -> Result<Vec<(PauliString, String)>, i32> {
    if len > 0 && (texts.is_null() || labels.is_null()) {
        return Err(-1);
    }
    let mut paulis = Vec::with_capacity(len);
    for i in 0..len {
        let text = unsafe { *texts.add(i) };
        let label = unsafe { *labels.add(i) };
        if text.is_null() || label.is_null() {
            return Err(-1);
        }
        let text_str = unsafe { CStr::from_ptr(text) }.to_str().map_err(|_| -4)?;
        let label_str = unsafe { CStr::from_ptr(label) }.to_str().map_err(|_| -4)?;
        let pauli = text_str.parse::<PauliString>().map_err(|_| -8)?;
        paulis.push((pauli, label_str.to_string()));
    }
    Ok(paulis)
}

// =====  Entanglement topology helpers  =====

/// Returns the flattened length (two entries per pair) generated by
/// `generate_pairs` for the topology, or 0 on an invalid topology. For
/// `ENTANGLEMENT_CUSTOM`, `pairs` is a flat array of `pairs_len / 2`
/// control/target qubit-id pairs echoed verbatim.
#[unsafe(no_mangle)]
pub extern "C" fn entanglement_generate_pairs_len(
    topology: i32,
    pairs: *const u32,
    pairs_len: usize,
    num_qubits: usize,
) -> usize {
    match parse_topology(topology, pairs, pairs_len) {
        Ok(topo) => topo.generate_pairs(num_qubits).len() * 2,
        Err(_) => 0,
    }
}

/// Copies the flattened pairs `[c0, t0, c1, t1, ...]` into `out` (two-step
/// pattern; call `entanglement_generate_pairs_len` first). Returns 0 on
/// success, -1 on NULL input, -8 when `len` does not match the required
/// length or an argument is invalid.
#[unsafe(no_mangle)]
pub extern "C" fn entanglement_generate_pairs(
    topology: i32,
    pairs: *const u32,
    pairs_len: usize,
    num_qubits: usize,
    out: *mut u32,
    len: usize,
) -> i32 {
    let topo = match parse_topology(topology, pairs, pairs_len) {
        Ok(topo) => topo,
        Err(code) => return code,
    };
    let flat = topo.generate_pairs(num_qubits);
    if flat.len() * 2 != len {
        return -8;
    }
    if len > 0 && out.is_null() {
        return -1;
    }
    for (i, (control, target)) in flat.iter().enumerate() {
        unsafe {
            *out.add(i * 2) = *control as u32;
            *out.add(i * 2 + 1) = *target as u32;
        }
    }
    0
}

/// Returns the flattened length (`k` entries per tuple) generated by
/// `generate_k_tuples` for the topology, or 0 on an invalid argument
/// combination.
#[unsafe(no_mangle)]
pub extern "C" fn entanglement_generate_k_tuples_len(
    topology: i32,
    pairs: *const u32,
    pairs_len: usize,
    k: usize,
    num_qubits: usize,
) -> usize {
    match parse_topology(topology, pairs, pairs_len) {
        Ok(topo) => topo.generate_k_tuples(k, num_qubits).len() * k,
        Err(_) => 0,
    }
}

/// Copies the flattened k-tuples `[t0_0, t0_1, ..., t1_0, ...]` into `out`
/// (two-step pattern; call `entanglement_generate_k_tuples_len` first).
/// Returns 0 on success, -1 on NULL input, -8 when `len` does not match the
/// required length or an argument is invalid.
#[unsafe(no_mangle)]
pub extern "C" fn entanglement_generate_k_tuples(
    topology: i32,
    pairs: *const u32,
    pairs_len: usize,
    k: usize,
    num_qubits: usize,
    out: *mut u32,
    len: usize,
) -> i32 {
    let topo = match parse_topology(topology, pairs, pairs_len) {
        Ok(topo) => topo,
        Err(code) => return code,
    };
    let tuples = topo.generate_k_tuples(k, num_qubits);
    if tuples.len() * k != len {
        return -8;
    }
    if len > 0 && out.is_null() {
        return -1;
    }
    let mut idx = 0;
    for tuple in &tuples {
        for &qubit in tuple {
            unsafe {
                *out.add(idx) = qubit as u32;
            }
            idx += 1;
        }
    }
    0
}

// =====  StronglyEntanglingLayers  =====

/// Opaque handle around a [`StronglyEntanglingLayers`] template.
pub struct CStronglyEntanglingLayers {
    pub inner: StronglyEntanglingLayers,
}

/// Creates a StronglyEntanglingLayers template with defaults: `reps = 1`,
/// entangler `CX`, ranges cycling through `1..num_qubits`.
#[unsafe(no_mangle)]
pub extern "C" fn strongly_entangling_layers_new(
    num_qubits: usize,
) -> *mut CStronglyEntanglingLayers {
    Box::into_raw(Box::new(CStronglyEntanglingLayers {
        inner: StronglyEntanglingLayers::new(num_qubits),
    }))
}

/// Frees a StronglyEntanglingLayers handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn strongly_entangling_layers_free(ptr: *mut CStronglyEntanglingLayers) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Sets the number of layers. Returns 0, or -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn strongly_entangling_layers_reps(
    ptr: *mut CStronglyEntanglingLayers,
    reps: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let inner = unsafe { &mut (*ptr).inner };
    *inner = inner.clone().reps(reps);
    0
}

/// Sets the two-qubit entangling gate by name (CX, CY or CZ). Returns 0, -1
/// on NULL input, -4 on invalid UTF-8, -8 on an unknown gate name.
#[unsafe(no_mangle)]
pub extern "C" fn strongly_entangling_layers_entanglement_gate(
    ptr: *mut CStronglyEntanglingLayers,
    name: *const c_char,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match parse_gate_name(name) {
        Ok(gate) => {
            let inner = unsafe { &mut (*ptr).inner };
            *inner = inner.clone().entanglement_gate(gate);
            0
        }
        Err(code) => code,
    }
}

/// Sets explicit entanglement ranges reused cyclically across layers. An
/// empty list (`len == 0`) stores the empty list, which fails validation
/// until replaced. Returns 0, or -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn strongly_entangling_layers_ranges(
    ptr: *mut CStronglyEntanglingLayers,
    ranges: *const usize,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    if len > 0 && ranges.is_null() {
        return -1;
    }
    let list = if len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(ranges, len) }.to_vec()
    };
    let inner = unsafe { &mut (*ptr).inner };
    *inner = inner.clone().ranges(list);
    0
}

/// Validates the template. Returns 0 on success, -1 on NULL input, -3 on an
/// invalid configuration.
#[unsafe(no_mangle)]
pub extern "C" fn strongly_entangling_layers_validate(
    ptr: *const CStronglyEntanglingLayers,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.validate().map_or(-3, |_| 0) }
}

/// Returns the number of independent parameters, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn strongly_entangling_layers_num_parameters(
    ptr: *const CStronglyEntanglingLayers,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_parameters() }
}

/// Returns the number of qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn strongly_entangling_layers_num_qubits(
    ptr: *const CStronglyEntanglingLayers,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Builds the parameterized circuit (parameters named `"{prefix}_{index}"`).
/// Returns a newly allocated `CCircuit*`, or NULL on NULL input, invalid
/// UTF-8 or build failure.
#[unsafe(no_mangle)]
pub extern "C" fn strongly_entangling_layers_build_circuit(
    ptr: *const CStronglyEntanglingLayers,
    prefix: *const c_char,
) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let prefix = match parse_prefix(prefix) {
        Ok(prefix) => prefix,
        Err(_) => return std::ptr::null_mut(),
    };
    match unsafe { (*ptr).inner.build_circuit(&prefix) } {
        Ok(circuit) => Box::into_raw(Box::new(CCircuit { inner: circuit })),
        Err(_) => std::ptr::null_mut(),
    }
}

// =====  PauliEvolutionAnsatz  =====

/// Opaque handle around a [`PauliEvolutionAnsatz`] template.
pub struct CPauliEvolutionAnsatz {
    pub inner: PauliEvolutionAnsatz,
}

/// Creates a Pauli evolution ansatz from a Hamiltonian (cloned and
/// simplified in the process). Returns NULL on NULL input or an empty
/// operator.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_evolution_ansatz_new(
    hamiltonian: *const CHamiltonian,
) -> *mut CPauliEvolutionAnsatz {
    if hamiltonian.is_null() {
        return std::ptr::null_mut();
    }
    match PauliEvolutionAnsatz::new(unsafe { (*hamiltonian).inner.clone() }) {
        Ok(inner) => Box::into_raw(Box::new(CPauliEvolutionAnsatz { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a Pauli evolution ansatz handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_evolution_ansatz_free(ptr: *mut CPauliEvolutionAnsatz) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Overrides the time-parameter symbol used by the evolution lowering; an
/// empty name resets it to the default. Returns 0, -1 on NULL input, -4 on
/// invalid UTF-8.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_evolution_ansatz_with_time_param_name(
    ptr: *mut CPauliEvolutionAnsatz,
    name: *const c_char,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match parse_prefix(name) {
        Ok(name) => {
            let inner = unsafe { &mut (*ptr).inner };
            *inner = inner.clone().with_time_param_name(name);
            0
        }
        Err(code) => code,
    }
}

/// C snapshot of the core `EvolutionInfo` returned by
/// [`PauliEvolutionAnsatz::evolution_info`].
#[repr(C)]
pub struct CPauliEvolutionInfo {
    /// 1 when the decomposition is mathematically exact, 0 otherwise.
    pub is_exact: u8,
    /// 1 when all Hamiltonian terms mutually commute, 0 otherwise.
    pub all_terms_commute: u8,
    /// Reserved padding to keep the struct layout stable.
    pub _pad: [u8; 6],
    /// Number of decomposition repetitions emitted into the circuit.
    pub steps: usize,
    /// Number of Pauli terms in the (simplified) Hamiltonian.
    pub num_terms: usize,
    /// One of `TROTTER_FIRST_ORDER`, `TROTTER_SECOND_ORDER` or
    /// `TROTTER_RANDOMIZED`, or `TROTTER_MODE_NONE` when the single-pass
    /// exact path was used.
    pub trotter_mode: i32,
    /// Seed for `TROTTER_RANDOMIZED`; 0 otherwise.
    pub trotter_seed: u64,
}

/// Reads the evolution strategy info (the C mirror of
/// [`PauliEvolutionAnsatz::evolution_info`]) into `out`.
///
/// `trotter_mode` is `TROTTER_MODE_NONE` when the single-pass exact path was
/// used; otherwise it is one of the `TROTTER_*` tags, and `trotter_seed` is
/// only meaningful for `TROTTER_RANDOMIZED`. Returns 0 on success, or -1 on
/// NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn pauli_evolution_ansatz_evolution_info(
    ptr: *const CPauliEvolutionAnsatz,
    out: *mut CPauliEvolutionInfo,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -1;
    }
    let info = unsafe { &(*ptr).inner }.evolution_info();
    let trotter_mode = match info.trotter_mode {
        None => TROTTER_MODE_NONE,
        Some(TrotterMode::FirstOrder) => TROTTER_FIRST_ORDER,
        Some(TrotterMode::SecondOrder) => TROTTER_SECOND_ORDER,
        Some(TrotterMode::Randomized(seed)) => {
            unsafe {
                (*out).trotter_seed = seed;
            }
            TROTTER_RANDOMIZED
        }
    };
    unsafe {
        (*out).is_exact = info.is_exact as u8;
        (*out).all_terms_commute = info.all_terms_commute as u8;
        (*out).steps = info.steps;
        (*out).num_terms = info.num_terms;
        (*out).trotter_mode = trotter_mode;
    }
    0
}

/// Opaque handle around a [`BasicEntanglerLayers`] template.
pub struct CBasicEntanglerLayers {
    pub inner: BasicEntanglerLayers,
}

/// Creates a BasicEntanglerLayers template with defaults: `reps = 1`,
/// rotation `RX`, entangler `CX`.
#[unsafe(no_mangle)]
pub extern "C" fn basic_entangler_layers_new(num_qubits: usize) -> *mut CBasicEntanglerLayers {
    Box::into_raw(Box::new(CBasicEntanglerLayers {
        inner: BasicEntanglerLayers::new(num_qubits),
    }))
}

/// Frees a BasicEntanglerLayers handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn basic_entangler_layers_free(ptr: *mut CBasicEntanglerLayers) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Sets the number of layers. Returns 0, or -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn basic_entangler_layers_reps(ptr: *mut CBasicEntanglerLayers, reps: usize) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let inner = unsafe { &mut (*ptr).inner };
    *inner = inner.clone().reps(reps);
    0
}

/// Sets the rotation gate by name (RX, RY, RZ or Phase). Returns 0, -1 on
/// NULL input, -4 on invalid UTF-8, -8 on an unknown gate name.
#[unsafe(no_mangle)]
pub extern "C" fn basic_entangler_layers_rotation_gate(
    ptr: *mut CBasicEntanglerLayers,
    name: *const c_char,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match parse_gate_name(name) {
        Ok(gate) => {
            let inner = unsafe { &mut (*ptr).inner };
            *inner = inner.clone().rotation_gate(gate);
            0
        }
        Err(code) => code,
    }
}

/// Sets the entanglement gate by name (CX, CY or CZ). Returns 0, -1 on NULL
/// input, -4 on invalid UTF-8, -8 on an unknown gate name.
#[unsafe(no_mangle)]
pub extern "C" fn basic_entangler_layers_entanglement_gate(
    ptr: *mut CBasicEntanglerLayers,
    name: *const c_char,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match parse_gate_name(name) {
        Ok(gate) => {
            let inner = unsafe { &mut (*ptr).inner };
            *inner = inner.clone().entanglement_gate(gate);
            0
        }
        Err(code) => code,
    }
}

/// Returns the number of independent parameters, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn basic_entangler_layers_num_parameters(
    ptr: *const CBasicEntanglerLayers,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_parameters() }
}

/// Returns the number of qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn basic_entangler_layers_num_qubits(ptr: *const CBasicEntanglerLayers) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Builds the parameterized circuit (parameters named `"{prefix}_{index}"`).
/// Returns a newly allocated `CCircuit*`, or NULL on NULL input, invalid
/// UTF-8 or build failure.
#[unsafe(no_mangle)]
pub extern "C" fn basic_entangler_layers_build_circuit(
    ptr: *const CBasicEntanglerLayers,
    prefix: *const c_char,
) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let prefix = match parse_prefix(prefix) {
        Ok(prefix) => prefix,
        Err(_) => return std::ptr::null_mut(),
    };
    match unsafe { (*ptr).inner.build_circuit(&prefix) } {
        Ok(circuit) => Box::into_raw(Box::new(CCircuit { inner: circuit })),
        Err(_) => std::ptr::null_mut(),
    }
}

// =====  QAOAAnsatz  =====

/// Opaque handle around a [`QAOAAnsatz`] template.
pub struct CQAOAAnsatz {
    pub inner: QAOAAnsatz,
}

/// Creates a QAOA ansatz from a cost Hamiltonian (cloned). The number of
/// qubits is inferred from the operator and the mixer defaults to the
/// standard X-mixer with `reps = 1`. Returns NULL on NULL input or an
/// invalid cost operator.
#[unsafe(no_mangle)]
pub extern "C" fn qaoa_ansatz_new(cost: *const CHamiltonian) -> *mut CQAOAAnsatz {
    if cost.is_null() {
        return std::ptr::null_mut();
    }
    match QAOAAnsatz::new(unsafe { (*cost).inner.clone() }) {
        Ok(inner) => Box::into_raw(Box::new(CQAOAAnsatz { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a QAOA ansatz handle. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn qaoa_ansatz_free(ptr: *mut CQAOAAnsatz) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Sets the number of alternating layers (depth p). Returns 0, or -1 on
/// NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn qaoa_ansatz_reps(ptr: *mut CQAOAAnsatz, reps: usize) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let inner = unsafe { &mut (*ptr).inner };
    *inner = inner.clone().reps(reps);
    0
}

/// Overrides the mixer Hamiltonian (cloned; must match the cost operator's
/// qubit count). Returns 0, -1 on NULL input, -3 on a qubit-count mismatch
/// or invalid mixer.
#[unsafe(no_mangle)]
pub extern "C" fn qaoa_ansatz_mixer(ptr: *mut CQAOAAnsatz, mixer: *const CHamiltonian) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    if mixer.is_null() {
        return -1;
    }
    let mixer = unsafe { (*mixer).inner.clone() };
    let inner = unsafe { &mut (*ptr).inner };
    match inner.clone().mixer(mixer) {
        Ok(ansatz) => {
            *inner = ansatz;
            0
        }
        Err(_) => -3,
    }
}

/// Prepends an initial-state circuit (cloned; must match the qubit count).
/// Returns 0, -1 on NULL input, -3 on a qubit-count mismatch.
#[unsafe(no_mangle)]
pub extern "C" fn qaoa_ansatz_initial_state(
    ptr: *mut CQAOAAnsatz,
    circuit: *const CCircuit,
) -> i32 {
    if ptr.is_null() || circuit.is_null() {
        return -1;
    }
    let circuit = unsafe { (*circuit).inner.clone() };
    let inner = unsafe { &mut (*ptr).inner };
    match inner.clone().initial_state(circuit) {
        Ok(ansatz) => {
            *inner = ansatz;
            0
        }
        Err(_) => -3,
    }
}

/// Sets the Hamiltonian evolution strategy.
///
/// `EVOLUTION_STRATEGY_EXACT` ignores the remaining arguments;
/// `EVOLUTION_STRATEGY_AUTO` uses `steps`; `EVOLUTION_STRATEGY_TROTTER`
/// uses `mode` (`TROTTER_FIRST_ORDER`, `TROTTER_SECOND_ORDER` or
/// `TROTTER_RANDOMIZED`), `steps` and `seed`. Returns 0, -1 on NULL input,
/// -8 on an unknown tag.
#[unsafe(no_mangle)]
pub extern "C" fn qaoa_ansatz_evolution_strategy(
    ptr: *mut CQAOAAnsatz,
    strategy: i32,
    mode: i32,
    steps: usize,
    seed: u64,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let evolution = match strategy {
        EVOLUTION_STRATEGY_EXACT => EvolutionStrategy::Exact,
        EVOLUTION_STRATEGY_AUTO => EvolutionStrategy::Auto { steps },
        EVOLUTION_STRATEGY_TROTTER => {
            let mode = match mode {
                TROTTER_FIRST_ORDER => TrotterMode::FirstOrder,
                TROTTER_SECOND_ORDER => TrotterMode::SecondOrder,
                TROTTER_RANDOMIZED => TrotterMode::Randomized(seed),
                _ => return -8,
            };
            EvolutionStrategy::Trotter { mode, steps }
        }
        _ => return -8,
    };
    let inner = unsafe { &mut (*ptr).inner };
    *inner = inner.clone().evolution_strategy(evolution);
    0
}

/// Validates the ansatz configuration. Returns 0 on success, -1 on NULL
/// input, -3 on invalid configuration.
#[unsafe(no_mangle)]
pub extern "C" fn qaoa_ansatz_validate(ptr: *const CQAOAAnsatz) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.validate().map_or(-3, |_| 0) }
}

/// Returns the number of parameters (2 per layer), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn qaoa_ansatz_num_parameters(ptr: *const CQAOAAnsatz) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_parameters() }
}

/// Returns the number of qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn qaoa_ansatz_num_qubits(ptr: *const CQAOAAnsatz) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Builds the parameterized circuit (parameters named
/// `"{prefix}_gamma_{layer}"` / `"{prefix}_beta_{layer}"`). Returns a newly
/// allocated `CCircuit*`, or NULL on NULL input, invalid UTF-8 or failure.
#[unsafe(no_mangle)]
pub extern "C" fn qaoa_ansatz_build_circuit(
    ptr: *const CQAOAAnsatz,
    prefix: *const c_char,
) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let prefix = match parse_prefix(prefix) {
        Ok(prefix) => prefix,
        Err(_) => return std::ptr::null_mut(),
    };
    match unsafe { (*ptr).inner.build_circuit(&prefix) } {
        Ok(circuit) => Box::into_raw(Box::new(CCircuit { inner: circuit })),
        Err(_) => std::ptr::null_mut(),
    }
}
