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
use crate::device::{CDevice, CTopology, standard_gate_from_name};
use crate::error::CqlibError;
use cqlib_core::circuit::gate::Instruction;
use cqlib_core::device::Device;
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
