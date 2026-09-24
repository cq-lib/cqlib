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

//! C ABI for device construction, topology, and circuit validation.

use crate::circuit::CCircuit;
use crate::device::{CDevice, CTopology, standard_gate_from_name, write_u32_list, write_u32_pairs};
use crate::error::CqlibError;
use cqlib_core::circuit::gate::Instruction;
use cqlib_core::device::error::TopologyError;
use cqlib_core::device::{Device, PhysicalQubit, Topology};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Creates a device named `name` with `num_qubits` isolated physical qubits
/// and no couplings. Use `device_from_edges` for custom topologies.
/// Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn device_new(name: *const c_char, num_qubits: u32) -> *mut CDevice {
    if name.is_null() {
        return std::ptr::null_mut();
    }
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let qubits: Vec<_> = (0..num_qubits)
        .map(cqlib_core::device::PhysicalQubit::new)
        .collect();
    let topology = match cqlib_core::device::Topology::new(qubits.clone(), Vec::new()) {
        Ok(t) => t,
        Err(_) => return std::ptr::null_mut(),
    };
    match Device::new(name, qubits.into_iter().collect(), topology) {
        Ok(device) => Box::into_raw(Box::new(CDevice { inner: device })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a device. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn device_free(ptr: *mut CDevice) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

fn boxed_device(result: Result<Device, cqlib_core::device::DeviceError>) -> *mut CDevice {
    match result {
        Ok(device) => Box::into_raw(Box::new(CDevice { inner: device })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Creates a device with a directed line topology `0 -> 1 -> ... -> n-1`.
#[unsafe(no_mangle)]
pub extern "C" fn device_line(name: *const c_char, num_qubits: u32) -> *mut CDevice {
    if name.is_null() {
        return std::ptr::null_mut();
    }
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    boxed_device(Device::line(name, num_qubits))
}

/// Creates a device with a bidirectional line topology.
#[unsafe(no_mangle)]
pub extern "C" fn device_bidirectional_line(name: *const c_char, num_qubits: u32) -> *mut CDevice {
    if name.is_null() {
        return std::ptr::null_mut();
    }
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    boxed_device(Device::bidirectional_line(name, num_qubits))
}

/// Creates a device with a bidirectional ring topology.
#[unsafe(no_mangle)]
pub extern "C" fn device_ring(name: *const c_char, num_qubits: u32) -> *mut CDevice {
    if name.is_null() {
        return std::ptr::null_mut();
    }
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    boxed_device(Device::ring(name, num_qubits))
}

/// Creates a device with a bidirectional grid topology (row-major qubit IDs).
#[unsafe(no_mangle)]
pub extern "C" fn device_grid(name: *const c_char, rows: u32, cols: u32) -> *mut CDevice {
    if name.is_null() {
        return std::ptr::null_mut();
    }
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    boxed_device(Device::grid(name, rows, cols))
}

/// Creates a device with a bidirectional star topology around `center`.
#[unsafe(no_mangle)]
pub extern "C" fn device_star(name: *const c_char, num_qubits: u32, center: u32) -> *mut CDevice {
    if name.is_null() {
        return std::ptr::null_mut();
    }
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    boxed_device(Device::star(name, num_qubits, center))
}

/// Creates a device from explicit directed edges.
///
/// `edges` points to `2 * num_edges` u32 values laid out as consecutive
/// `(control, target)` pairs. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn device_from_edges(
    name: *const c_char,
    num_qubits: u32,
    edges: *const u32,
    num_edges: usize,
) -> *mut CDevice {
    if name.is_null() || (num_edges > 0 && edges.is_null()) {
        return std::ptr::null_mut();
    }
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let raw = if num_edges == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(edges, num_edges * 2) }
    };
    let pairs: Vec<(u32, u32)> = raw.chunks_exact(2).map(|c| (c[0], c[1])).collect();
    boxed_device(Device::from_edges(name, num_qubits, &pairs))
}

/// Replaces the device's native gate set with the gates named in
/// `gate_names` (comma separated, e.g. "H,CX,RZ"). Returns 0 on success,
/// -4 if any name is unknown, or a negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn device_with_native_gates(ptr: *mut CDevice, gate_names: *const c_char) -> i32 {
    if ptr.is_null() || gate_names.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let names = match unsafe { CStr::from_ptr(gate_names) }.to_str() {
        Ok(s) => s,
        Err(_) => return CqlibError::ParseError as i32,
    };
    let mut gates = Vec::new();
    for part in names.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match standard_gate_from_name(part) {
            Some(gate) => gates.push(Instruction::Standard(gate)),
            None => return CqlibError::ParseError as i32,
        }
    }
    if gates.is_empty() {
        return CqlibError::InvalidParam as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    // `with_native_gates` consumes self; work on a clone so the original
    // stays intact when the builder rejects the gate set.
    match wrapper.inner.clone().with_native_gates(gates) {
        Ok(new_device) => {
            wrapper.inner = new_device;
            0
        }
        Err(_) => CqlibError::CircuitError as i32,
    }
}

/// Validates that `circuit` is compatible with the device.
/// Returns 0 when valid, -3 when validation fails, or a negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn device_validate_circuit(ptr: *const CDevice, circuit: *const CCircuit) -> i32 {
    if ptr.is_null() || circuit.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let device = unsafe { &(*ptr).inner };
    let circuit = unsafe { &(*circuit).inner };
    match device.validate_circuit(circuit) {
        Ok(()) => 0,
        Err(_) => CqlibError::CircuitError as i32,
    }
}

/// Returns the number of usable physical qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn device_num_qubits(ptr: *const CDevice) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_usable_qubits() }
}

/// Returns the device name as a heap-allocated C string.
/// Caller must free with `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn device_name(ptr: *const CDevice) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let name = unsafe { (*ptr).inner.name() }.to_string();
    match CString::new(name) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Returns the native gate names as a comma-separated heap-allocated C
/// string (e.g. "H,CX"). Caller must free with `cqlib_string_free`.
/// Returns NULL on error or when the gate set is empty.
#[unsafe(no_mangle)]
pub extern "C" fn device_native_gates(ptr: *const CDevice) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let names: Vec<String> = unsafe { (*ptr).inner.native_gates() }
        .iter()
        .map(|gate| gate.name().to_string())
        .collect();
    if names.is_empty() {
        return std::ptr::null_mut();
    }
    match CString::new(names.join(",")) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Returns a clone of the device topology as an owned `CTopology*`.
/// Caller must free with `topology_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn device_topology(ptr: *const CDevice) -> *mut CTopology {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let topology = unsafe { (*ptr).inner.topology() }.clone();
    Box::into_raw(Box::new(CTopology { inner: topology }))
}

/// Frees a topology returned by `device_topology`. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn topology_free(ptr: *mut CTopology) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the number of qubits in the topology, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn topology_num_qubits(ptr: *const CTopology) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Returns the number of coupling edges in the topology, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn topology_num_couplings(ptr: *const CTopology) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_couplings() }
}

// =====  Section 3.3: topology graph queries and updates  =====

/// Maps a `TopologyError` to a C error code: missing qubits or couplings map
/// to -2, conflicting or duplicated updates map to -8.
fn topology_error_code(error: &TopologyError) -> i32 {
    match error {
        TopologyError::QubitNotFound(_) | TopologyError::CouplingNotFound { .. } => {
            CqlibError::QubitOutOfBounds as i32
        }
        _ => CqlibError::InvalidParam as i32,
    }
}

/// Reads a u32 ID array into physical qubits; `len == 0` yields an empty vec.
/// Returns -1 when the array is NULL despite a positive length.
fn physical_qubits_from_c(qubits: *const u32, len: usize) -> Result<Vec<PhysicalQubit>, i32> {
    if len == 0 {
        return Ok(Vec::new());
    }
    if qubits.is_null() {
        return Err(CqlibError::NullPtr as i32);
    }
    Ok(unsafe { std::slice::from_raw_parts(qubits, len) }
        .iter()
        .map(|&q| PhysicalQubit::new(q))
        .collect())
}

/// Creates a topology whose physical qubits (given by ID array `qubits`) are
/// connected as a directed line in the supplied order. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn topology_line(qubits: *const u32, num_qubits: usize) -> *mut CTopology {
    let physical = match physical_qubits_from_c(qubits, num_qubits) {
        Ok(q) => q,
        Err(_) => return std::ptr::null_mut(),
    };
    match Topology::line(physical) {
        Ok(topology) => Box::into_raw(Box::new(CTopology { inner: topology })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Adds qubits (given by ID array) to the topology.
///
/// Returns 0 on success, -1 on NULL, or -8 when a qubit already exists in
/// the topology or is listed twice in the request.
#[unsafe(no_mangle)]
pub extern "C" fn topology_add_qubits(ptr: *mut CTopology, qubits: *const u32, len: usize) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let items = match physical_qubits_from_c(qubits, len) {
        Ok(q) => q,
        Err(code) => return code,
    };
    let wrapper = unsafe { &mut *ptr };
    match wrapper.inner.add_qubits(items) {
        Ok(()) => CqlibError::Ok as i32,
        Err(e) => topology_error_code(&e),
    }
}

/// Removes qubits (given by ID array) and every coupling touching them.
///
/// Returns 0 on success, -1 on NULL, -2 when a qubit is not in the topology,
/// or -8 when a qubit is listed twice.
#[unsafe(no_mangle)]
pub extern "C" fn topology_remove_qubits(
    ptr: *mut CTopology,
    qubits: *const u32,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let items = match physical_qubits_from_c(qubits, len) {
        Ok(q) => q,
        Err(code) => return code,
    };
    let wrapper = unsafe { &mut *ptr };
    match wrapper.inner.remove_qubits(items) {
        Ok(()) => CqlibError::Ok as i32,
        Err(e) => topology_error_code(&e),
    }
}

/// Adds directed coupling edges to the topology.
///
/// `edges` points to `2 * num_edges` u32 values laid out as consecutive
/// `(control, target)` pairs. `names` optionally points to `num_edges` C
/// strings (NULL entries or a NULL array leave the coupling unnamed).
/// Returns 0 on success, -1 on NULL, -4 for invalid UTF-8 in a name, -2 when
/// an endpoint qubit is not in the topology, or -8 for duplicate or self
/// couplings.
#[unsafe(no_mangle)]
pub extern "C" fn topology_add_couplings(
    ptr: *mut CTopology,
    edges: *const u32,
    num_edges: usize,
    names: *const *const c_char,
) -> i32 {
    if ptr.is_null() || (num_edges > 0 && edges.is_null()) {
        return CqlibError::NullPtr as i32;
    }
    let raw = if num_edges == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(edges, num_edges * 2) }
    };
    let mut couplings = Vec::with_capacity(num_edges);
    for (i, chunk) in raw.chunks_exact(2).enumerate() {
        let name = if names.is_null() {
            String::new()
        } else {
            let raw_name = unsafe { *names.add(i) };
            if raw_name.is_null() {
                String::new()
            } else {
                match unsafe { CStr::from_ptr(raw_name) }.to_str() {
                    Ok(s) => s.to_string(),
                    Err(_) => return CqlibError::ParseError as i32,
                }
            }
        };
        couplings.push((
            PhysicalQubit::new(chunk[0]),
            PhysicalQubit::new(chunk[1]),
            name,
        ));
    }
    let wrapper = unsafe { &mut *ptr };
    match wrapper.inner.add_couplings(couplings) {
        Ok(()) => CqlibError::Ok as i32,
        Err(e) => topology_error_code(&e),
    }
}

/// Removes directed coupling edges from the topology.
///
/// `edges` points to `2 * num_edges` u32 values laid out as consecutive
/// `(control, target)` pairs. Returns 0 on success, -1 on NULL, -2 when a
/// qubit or coupling is not in the topology, or -8 when a coupling is listed
/// twice.
#[unsafe(no_mangle)]
pub extern "C" fn topology_remove_couplings(
    ptr: *mut CTopology,
    edges: *const u32,
    num_edges: usize,
) -> i32 {
    if ptr.is_null() || (num_edges > 0 && edges.is_null()) {
        return CqlibError::NullPtr as i32;
    }
    let raw = if num_edges == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(edges, num_edges * 2) }
    };
    let pairs: Vec<(PhysicalQubit, PhysicalQubit)> = raw
        .chunks_exact(2)
        .map(|c| (PhysicalQubit::new(c[0]), PhysicalQubit::new(c[1])))
        .collect();
    let wrapper = unsafe { &mut *ptr };
    match wrapper.inner.remove_couplings(pairs) {
        Ok(()) => CqlibError::Ok as i32,
        Err(e) => topology_error_code(&e),
    }
}

/// Returns 1 when `qubit` is part of the topology, 0 otherwise, or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn topology_contains_qubit(ptr: *const CTopology, qubit: u32) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.contains_qubit(&PhysicalQubit::new(qubit)) as i32 }
}

/// Returns the name of the directed coupling `control -> target` as a
/// heap-allocated C string; an existing but unnamed coupling yields an empty
/// string. Caller must free with `cqlib_string_free`. Returns NULL on NULL
/// input or when the coupling does not exist.
#[unsafe(no_mangle)]
pub extern "C" fn topology_get_coupling_name(
    ptr: *const CTopology,
    control: u32,
    target: u32,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe {
        (*ptr)
            .inner
            .get_coupling_name(PhysicalQubit::new(control), PhysicalQubit::new(target))
    } {
        Some(name) => match CString::new(name) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Returns the number of qubit IDs available from `topology_qubits`, or 0
/// for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn topology_qubits_len(ptr: *const CTopology) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.qubits().count() }
}

/// Copies the qubit IDs into `out` (two-step pattern; pair with
/// `topology_qubits_len`). Returns the total count.
#[unsafe(no_mangle)]
pub extern "C" fn topology_qubits(ptr: *const CTopology, out: *mut u32, len: usize) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let items: Vec<u32> = unsafe { (*ptr).inner.qubits() }.map(|q| q.id()).collect();
    write_u32_list(&items, out, len)
}

/// Returns the number of undirected neighbors of `qubit` (successors and
/// predecessors, deduplicated), or 0 for NULL or an unknown qubit.
#[unsafe(no_mangle)]
pub extern "C" fn topology_neighbors_undirected_len(ptr: *const CTopology, qubit: u32) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe {
        (*ptr)
            .inner
            .neighbors_undirected(PhysicalQubit::new(qubit))
            .count()
    }
}

/// Copies the undirected neighbor IDs of `qubit` (ascending) into `out`
/// (two-step pattern; pair with `topology_neighbors_undirected_len`).
/// Returns the total count.
#[unsafe(no_mangle)]
pub extern "C" fn topology_neighbors_undirected(
    ptr: *const CTopology,
    qubit: u32,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let items: Vec<u32> = unsafe { (*ptr).inner.neighbors_undirected(PhysicalQubit::new(qubit)) }
        .map(|q| q.id())
        .collect();
    write_u32_list(&items, out, len)
}

/// Returns the number of qubits with a directed coupling into `qubit`, or 0
/// for NULL or an unknown qubit.
#[unsafe(no_mangle)]
pub extern "C" fn topology_predecessors_len(ptr: *const CTopology, qubit: u32) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.predecessors(PhysicalQubit::new(qubit)).count() }
}

/// Copies the predecessor IDs of `qubit` (qubits with a directed coupling
/// into it, ascending) into `out` (two-step pattern; pair with
/// `topology_predecessors_len`). Returns the total count.
#[unsafe(no_mangle)]
pub extern "C" fn topology_predecessors(
    ptr: *const CTopology,
    qubit: u32,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let mut items: Vec<u32> = unsafe { (*ptr).inner.predecessors(PhysicalQubit::new(qubit)) }
        .map(|q| q.id())
        .collect();
    items.sort_unstable();
    write_u32_list(&items, out, len)
}

/// Returns the number of qubits reachable through outgoing couplings from
/// `qubit`, or 0 for NULL or an unknown qubit.
#[unsafe(no_mangle)]
pub extern "C" fn topology_successors_len(ptr: *const CTopology, qubit: u32) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.successors(PhysicalQubit::new(qubit)).count() }
}

/// Copies the successor IDs of `qubit` (qubits reachable through outgoing
/// couplings, ascending) into `out` (two-step pattern; pair with
/// `topology_successors_len`). Returns the total count.
#[unsafe(no_mangle)]
pub extern "C" fn topology_successors(
    ptr: *const CTopology,
    qubit: u32,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let mut items: Vec<u32> = unsafe { (*ptr).inner.successors(PhysicalQubit::new(qubit)) }
        .map(|q| q.id())
        .collect();
    items.sort_unstable();
    write_u32_list(&items, out, len)
}

/// Returns the number of incoming couplings to `qubit` (0 for NULL or an
/// unknown qubit).
#[unsafe(no_mangle)]
pub extern "C" fn topology_in_degree(ptr: *const CTopology, qubit: u32) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.in_degree(&PhysicalQubit::new(qubit)) }
}

/// Returns the number of outgoing couplings from `qubit` (0 for NULL or an
/// unknown qubit).
#[unsafe(no_mangle)]
pub extern "C" fn topology_out_degree(ptr: *const CTopology, qubit: u32) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.out_degree(&PhysicalQubit::new(qubit)) }
}

/// Returns 1 when the directed coupling `control -> target` exists, 0
/// otherwise, or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn topology_supports_directed_coupling(
    ptr: *const CTopology,
    control: u32,
    target: u32,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe {
        (*ptr)
            .inner
            .supports_directed_coupling(PhysicalQubit::new(control), PhysicalQubit::new(target))
            as i32
    }
}

/// Returns 1 when a coupling between the two qubits exists in either
/// direction, 0 otherwise, or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn topology_supports_coupling_either_direction(
    ptr: *const CTopology,
    a: u32,
    b: u32,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe {
        (*ptr)
            .inner
            .supports_coupling_either_direction(PhysicalQubit::new(a), PhysicalQubit::new(b))
            as i32
    }
}

/// Returns the number of unique undirected coupling pairs (bidirectional
/// couplings collapse to one pair), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn topology_undirected_edges_len(ptr: *const CTopology) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.undirected_edges().count() }
}

/// Copies the unique undirected coupling pairs into `out` as consecutive
/// `(low, high)` u32 pairs sorted ascending (two-step pattern; pair with
/// `topology_undirected_edges_len`). `out` must have room for `2 * len`
/// entries. Returns the total number of pairs.
#[unsafe(no_mangle)]
pub extern "C" fn topology_undirected_edges(
    ptr: *const CTopology,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let pairs: Vec<(u32, u32)> = unsafe { (*ptr).inner.undirected_edges() }
        .map(|(a, b)| (a.id(), b.id()))
        .collect();
    write_u32_pairs(&pairs, out, len)
}
