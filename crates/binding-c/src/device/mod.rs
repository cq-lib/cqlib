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

//! C ABI for the device module (Device, Layout, ExecutionResult, NoiseModel).

#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod layout;
pub mod noise;
pub mod qubit;
pub mod result;
pub mod topology;

use crate::circuit::CValueOperation;
use crate::error::CqlibError;
use cqlib_core::circuit::gate::{Instruction, StandardGate};
use cqlib_core::device::{
    Device, DeviceError, EdgeProp, ExecutionResult, InstructionProp, Layout, NoiseModel,
    PhysicalQubit, QubitProp,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Opaque handle around [`Device`] for C ABI.
pub struct CDevice {
    pub inner: Device,
}

/// Opaque handle around [`cqlib_core::device::Topology`] for C ABI.
pub struct CTopology {
    pub inner: cqlib_core::device::Topology,
}

/// Opaque handle around [`Layout`] for C ABI.
pub struct CLayout {
    pub inner: Layout,
}

/// Opaque handle around [`ExecutionResult`] for C ABI.
pub struct CExecutionResult {
    pub inner: ExecutionResult,
}

/// Opaque handle around [`NoiseModel`] for C ABI.
pub struct CNoiseModel {
    pub inner: NoiseModel,
}

/// Key/value list of measurement outcomes (bitstring -> weight).
///
/// Used by `execution_result_counts` and `execution_result_probabilities`;
/// `values` holds counts for the former and probabilities for the latter.
pub struct CCountsList {
    pub keys: Vec<String>,
    pub values: Vec<f64>,
}

/// Resolves a standard-gate name (e.g. "H", "CX", "RZZ") to a `StandardGate`.
pub(crate) fn standard_gate_from_name(name: &str) -> Option<StandardGate> {
    use StandardGate as G;
    Some(match name {
        "I" => G::I,
        "H" => G::H,
        "RX" => G::RX,
        "RXX" => G::RXX,
        "RXY" => G::RXY,
        "RY" => G::RY,
        "RYY" => G::RYY,
        "RZ" => G::RZ,
        "RZX" => G::RZX,
        "RZZ" => G::RZZ,
        "S" => G::S,
        "SDG" => G::SDG,
        "SWAP" => G::SWAP,
        "T" => G::T,
        "TDG" => G::TDG,
        "U" => G::U,
        "X" => G::X,
        "XY" => G::XY,
        "X2P" => G::X2P,
        "X2M" => G::X2M,
        "XY2P" => G::XY2P,
        "XY2M" => G::XY2M,
        "Y" => G::Y,
        "Y2P" => G::Y2P,
        "Y2M" => G::Y2M,
        "Z" => G::Z,
        "Phase" => G::Phase,
        "GPhase" => G::GPhase,
        "CX" => G::CX,
        "CCX" => G::CCX,
        "CY" => G::CY,
        "CZ" => G::CZ,
        "CRX" => G::CRX,
        "CRY" => G::CRY,
        "CRZ" => G::CRZ,
        "fSim" | "FSIM" => G::FSIM,
        _ => return None,
    })
}

// =====  Section 3.1: device attribute queries and updates  =====

/// Copies a u32 list into `out` (two-step pattern). Returns the total number
/// of available entries; a NULL `out` only queries the length.
pub(crate) fn write_u32_list(items: &[u32], out: *mut u32, len: usize) -> usize {
    let total = items.len();
    if !out.is_null() {
        for (i, item) in items.iter().take(total.min(len)).enumerate() {
            unsafe { *out.add(i) = *item };
        }
    }
    total
}

/// Copies `(a, b)` u32 pairs into `out` (two-step pattern). `out` must have
/// room for `2 * len` entries. Returns the total number of available pairs.
pub(crate) fn write_u32_pairs(pairs: &[(u32, u32)], out: *mut u32, len: usize) -> usize {
    let total = pairs.len();
    if !out.is_null() {
        for (i, (a, b)) in pairs.iter().take(total.min(len)).enumerate() {
            unsafe {
                *out.add(i * 2) = *a;
                *out.add(i * 2 + 1) = *b;
            }
        }
    }
    total
}

/// Maps a `DeviceError` raised while writing qubit/edge properties to a C
/// error code: physical-resource mismatches map to -2, calibration payload
/// problems map to -8.
fn device_error_code(error: &DeviceError) -> i32 {
    match error {
        DeviceError::QubitNotInDevice(_)
        | DeviceError::QubitNotInTopology(_)
        | DeviceError::InvalidOnlineQubit(_)
        | DeviceError::EdgeNotInTopology(_, _)
        | DeviceError::InvalidTopology(_) => CqlibError::QubitOutOfBounds as i32,
        _ => CqlibError::InvalidParam as i32,
    }
}

/// Writes an optional f64 to `*out` (NaN when unset) and returns 0, or -8
/// when the value is not available.
fn optional_f64(value: Option<f64>, out: *mut f64) -> i32 {
    match value {
        Some(v) => {
            if !out.is_null() {
                unsafe { *out = v };
            }
            CqlibError::Ok as i32
        }
        None => {
            if !out.is_null() {
                unsafe { *out = f64::NAN };
            }
            CqlibError::InvalidParam as i32
        }
    }
}

/// Applies `mutate` to a copy of the recorded properties for `qubit` and
/// re-records them on the device.
///
/// Returns 0 on success, -8 when the qubit has no recorded properties or the
/// mutation is rejected, or the mapped device error when re-recording fails.
unsafe fn update_qubit_properties(
    ptr: *mut CDevice,
    qubit: u32,
    mutate: impl FnOnce(&mut QubitProp) -> Result<(), i32>,
) -> i32 {
    let mut props = match unsafe { (*ptr).inner.qubit_properties(PhysicalQubit::new(qubit)) } {
        Some(existing) => existing.clone(),
        None => return CqlibError::InvalidParam as i32,
    };
    if let Err(code) = mutate(&mut props) {
        return code;
    }
    match unsafe { &mut *ptr }
        .inner
        .add_qubit_properties(PhysicalQubit::new(qubit), props)
    {
        Ok(()) => CqlibError::Ok as i32,
        Err(e) => device_error_code(&e),
    }
}

/// Applies `edit` to the native instruction recorded at `index` for `qubit`
/// and revalidates the whole list (the core type has no in-place list
/// mutation, so the list is rebuilt and revalidated entry by entry).
///
/// Returns 0 on success, -8 when the qubit has no recorded properties or
/// `index` is out of bounds, and the mapped device error when the edited
/// entry fails validation.
unsafe fn update_qubit_native_instruction(
    ptr: *mut CDevice,
    qubit: u32,
    index: usize,
    edit: impl FnOnce(&mut InstructionProp),
) -> i32 {
    unsafe {
        update_qubit_properties(ptr, qubit, |props| {
            if index >= props.native_instructions().len() {
                return Err(CqlibError::InvalidParam as i32);
            }
            let mut list: Vec<InstructionProp> = props.native_instructions().to_vec();
            edit(&mut list[index]);
            let mut rebuilt = QubitProp::new(props.readout_error());
            if let Some(v) = props.t1() {
                rebuilt.set_t1(v);
            }
            if let Some(v) = props.t2() {
                rebuilt.set_t2(v);
            }
            if let Some(v) = props.prob_meas0_prep1() {
                rebuilt.set_prob_meas0_prep1(v);
            }
            if let Some(v) = props.prob_meas1_prep0() {
                rebuilt.set_prob_meas1_prep0(v);
            }
            if let Some(v) = props.frequency() {
                rebuilt.set_frequency(v);
            }
            for item in list {
                if let Err(e) = rebuilt.set_native_instruction(item) {
                    return Err(device_error_code(&e));
                }
            }
            *props = rebuilt;
            Ok(())
        })
    }
}

/// C snapshot of one native instruction capability (`InstructionProp`).
///
/// `name` is a heap-allocated gate name the caller frees with
/// `cqlib_string_free`; `length` is NaN when the duration is unset.
#[repr(C)]
pub struct CNativeInstruction {
    /// Heap-allocated standard-gate name; free with `cqlib_string_free`.
    pub name: *mut c_char,
    /// Error rate in [0, 1].
    pub error_rate: f64,
    /// Duration in nanoseconds; NaN when unset.
    pub length: f64,
}

/// C snapshot of the per-qubit properties recorded for one physical qubit.
///
/// Optional coherence fields are NaN when unset. Native instructions are not
/// inlined; query them with `device_qubit_prop_native_instruction` using
/// `num_native_instructions` as the index bound.
#[repr(C)]
pub struct CQubitProp {
    /// Readout error rate in [0, 1].
    pub readout_error: f64,
    /// T1 relaxation time in microseconds; NaN when unset.
    pub t1: f64,
    /// T2 dephasing time in microseconds; NaN when unset.
    pub t2: f64,
    /// P(measure 0 | prepared 1); NaN when unset.
    pub prob_meas0_prep1: f64,
    /// P(measure 1 | prepared 0); NaN when unset.
    pub prob_meas1_prep0: f64,
    /// Frequency in GHz; NaN when unset.
    pub frequency: f64,
    /// Number of native instructions recorded for the qubit.
    pub num_native_instructions: usize,
}

/// C snapshot of the per-edge properties recorded for one directed coupling.
///
/// Native instructions are not inlined; query them with
/// `device_edge_prop_native_instruction` using `num_native_instructions` as
/// the index bound.
#[repr(C)]
pub struct CEdgeProp {
    /// Number of native instructions recorded for the directed edge.
    pub num_native_instructions: usize,
}

/// C input for `device_add_qubit_properties`.
///
/// Optional f64 fields use NaN to leave the value unset. Native instructions
/// are supplied as parallel arrays of length `num_native_instructions`:
/// gate names, error rates, and durations (NaN = unset). `native_lengths`
/// may be NULL to leave all durations unset.
#[repr(C)]
pub struct CQubitPropInput {
    /// Readout error rate in [0, 1].
    pub readout_error: f64,
    /// T1 relaxation time in microseconds; NaN = unset.
    pub t1: f64,
    /// T2 dephasing time in microseconds; NaN = unset.
    pub t2: f64,
    /// P(measure 0 | prepared 1); NaN = unset.
    pub prob_meas0_prep1: f64,
    /// P(measure 1 | prepared 0); NaN = unset.
    pub prob_meas1_prep0: f64,
    /// Frequency in GHz; NaN = unset.
    pub frequency: f64,
    /// Array of standard-gate names, or NULL when none.
    pub native_gate_names: *const *const c_char,
    /// Array of error rates, or NULL when none.
    pub native_error_rates: *const f64,
    /// Array of durations in nanoseconds (NaN = unset), or NULL.
    pub native_lengths: *const f64,
    /// Number of native instructions.
    pub num_native_instructions: usize,
}

/// C input for `device_add_edge_properties`.
///
/// Native instructions are supplied as parallel arrays of length
/// `num_native_instructions` (two-qubit standard gates only). Fields mirror
/// [`CQubitPropInput`].
#[repr(C)]
pub struct CEdgePropInput {
    /// Array of standard-gate names, or NULL when none.
    pub native_gate_names: *const *const c_char,
    /// Array of error rates, or NULL when none.
    pub native_error_rates: *const f64,
    /// Array of durations in nanoseconds (NaN = unset), or NULL.
    pub native_lengths: *const f64,
    /// Number of native instructions.
    pub num_native_instructions: usize,
}

/// Builds the instruction list shared by `CQubitPropInput`/`CEdgePropInput`.
///
/// Returns the gate-name error code (-4) for unknown names or UTF-8 issues,
/// and -1 when required arrays are NULL.
unsafe fn native_instructions_from_c(
    names: *const *const c_char,
    error_rates: *const f64,
    lengths: *const f64,
    count: usize,
) -> Result<Vec<InstructionProp>, i32> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if names.is_null() || error_rates.is_null() {
        return Err(CqlibError::NullPtr as i32);
    }
    let mut props = Vec::with_capacity(count);
    for i in 0..count {
        let raw = unsafe { *names.add(i) };
        if raw.is_null() {
            return Err(CqlibError::NullPtr as i32);
        }
        let name = unsafe { CStr::from_ptr(raw) }
            .to_str()
            .map_err(|_| CqlibError::ParseError as i32)?;
        let gate = standard_gate_from_name(name).ok_or(CqlibError::ParseError as i32)?;
        let error_rate = unsafe { *error_rates.add(i) };
        let mut prop = InstructionProp::new(Instruction::Standard(gate), error_rate);
        if !lengths.is_null() {
            let length = unsafe { *lengths.add(i) };
            if !length.is_nan() {
                prop.set_length(length);
            }
        }
        props.push(prop);
    }
    Ok(props)
}

/// Returns the T1 relaxation time (μs) for `qubit`, falling back to the
/// device default. Writes the value to `*out` and returns 0, or -8 when no
/// T1 data is available for the qubit.
#[unsafe(no_mangle)]
pub extern "C" fn device_get_t1(ptr: *const CDevice, qubit: u32, out: *mut f64) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    optional_f64(
        unsafe { (*ptr).inner.get_t1(PhysicalQubit::new(qubit)) },
        out,
    )
}

/// Returns the T2 dephasing time (μs) for `qubit`, falling back to the
/// device default. Returns 0 with the value in `*out`, or -8 when unset.
#[unsafe(no_mangle)]
pub extern "C" fn device_get_t2(ptr: *const CDevice, qubit: u32, out: *mut f64) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    optional_f64(
        unsafe { (*ptr).inner.get_t2(PhysicalQubit::new(qubit)) },
        out,
    )
}

/// Returns the readout error rate for `qubit`, falling back to the device
/// default. Returns 0 with the value in `*out`, or -8 when unset.
#[unsafe(no_mangle)]
pub extern "C" fn device_get_readout_error(ptr: *const CDevice, qubit: u32, out: *mut f64) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    optional_f64(
        unsafe { (*ptr).inner.get_readout_error(PhysicalQubit::new(qubit)) },
        out,
    )
}

/// Returns the error rate of the single-qubit standard gate `gate_name` on
/// `qubit`. Returns 0 with the value in `*out`, -4 for an unknown gate name,
/// or -8 when the qubit does not support the gate.
#[unsafe(no_mangle)]
pub extern "C" fn device_single_qubit_error(
    ptr: *const CDevice,
    gate_name: *const c_char,
    qubit: u32,
    out: *mut f64,
) -> i32 {
    if ptr.is_null() || gate_name.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return CqlibError::ParseError as i32,
    };
    let gate = match standard_gate_from_name(name) {
        Some(g) => g,
        None => return CqlibError::ParseError as i32,
    };
    optional_f64(
        unsafe {
            (*ptr)
                .inner
                .single_qubit_error(PhysicalQubit::new(qubit), &Instruction::Standard(gate))
        },
        out,
    )
}

/// Returns the error rate of the two-qubit standard gate `gate_name` on the
/// directed coupling `control -> target`. Returns 0 with the value in
/// `*out`, -4 for an unknown gate name, or -8 when the coupling does not
/// support the gate.
#[unsafe(no_mangle)]
pub extern "C" fn device_two_qubit_error(
    ptr: *const CDevice,
    gate_name: *const c_char,
    control: u32,
    target: u32,
    out: *mut f64,
) -> i32 {
    if ptr.is_null() || gate_name.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return CqlibError::ParseError as i32,
    };
    let gate = match standard_gate_from_name(name) {
        Some(g) => g,
        None => return CqlibError::ParseError as i32,
    };
    optional_f64(
        unsafe {
            (*ptr).inner.two_qubit_error(
                PhysicalQubit::new(control),
                PhysicalQubit::new(target),
                &Instruction::Standard(gate),
            )
        },
        out,
    )
}

/// Returns the direction-specific coupling error for `control -> target`,
/// i.e. the best calibrated native two-qubit error on the edge or the device
/// default. Returns 0 with the value in `*out`, or -8 when the directed
/// coupling does not exist.
#[unsafe(no_mangle)]
pub extern "C" fn device_edge_error(
    ptr: *const CDevice,
    control: u32,
    target: u32,
    out: *mut f64,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    optional_f64(
        unsafe {
            (*ptr)
                .inner
                .edge_error(PhysicalQubit::new(control), PhysicalQubit::new(target))
        },
        out,
    )
}

/// Returns the device-wide default T1 time (μs). Returns 0 with the value
/// in `*out`, or -8 when no default is set.
#[unsafe(no_mangle)]
pub extern "C" fn device_default_t1(ptr: *const CDevice, out: *mut f64) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    optional_f64(unsafe { (*ptr).inner.default_t1() }, out)
}

/// Returns the device-wide default T2 time (μs). Returns 0 with the value
/// in `*out`, or -8 when no default is set.
#[unsafe(no_mangle)]
pub extern "C" fn device_default_t2(ptr: *const CDevice, out: *mut f64) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    optional_f64(unsafe { (*ptr).inner.default_t2() }, out)
}

/// Returns the device-wide default readout error rate. Returns 0 with the
/// value in `*out`, or -8 when no default is set.
#[unsafe(no_mangle)]
pub extern "C" fn device_default_readout_error(ptr: *const CDevice, out: *mut f64) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    optional_f64(unsafe { (*ptr).inner.default_readout_error() }, out)
}

/// Returns the device-wide default single-qubit gate error rate. Returns 0
/// with the value in `*out`, or -8 when no default is set.
#[unsafe(no_mangle)]
pub extern "C" fn device_default_single_qubit_error(ptr: *const CDevice, out: *mut f64) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    optional_f64(unsafe { (*ptr).inner.default_single_qubit_error() }, out)
}

/// Returns the device-wide default two-qubit gate error rate. Returns 0 with
/// the value in `*out`, or -8 when no default is set.
#[unsafe(no_mangle)]
pub extern "C" fn device_default_two_qubit_error(ptr: *const CDevice, out: *mut f64) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    optional_f64(unsafe { (*ptr).inner.default_two_qubit_error() }, out)
}

/// Sets the device-wide default T1 time (μs).
#[unsafe(no_mangle)]
pub extern "C" fn device_set_default_t1(ptr: *mut CDevice, t1: f64) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.set_default_t1(t1) };
    CqlibError::Ok as i32
}

/// Sets the device-wide default T2 time (μs).
#[unsafe(no_mangle)]
pub extern "C" fn device_set_default_t2(ptr: *mut CDevice, t2: f64) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.set_default_t2(t2) };
    CqlibError::Ok as i32
}

/// Sets the device-wide default readout error rate.
#[unsafe(no_mangle)]
pub extern "C" fn device_set_default_readout_error(ptr: *mut CDevice, error: f64) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.set_default_readout_error(error) };
    CqlibError::Ok as i32
}

/// Sets the device-wide default single-qubit gate error rate.
#[unsafe(no_mangle)]
pub extern "C" fn device_set_default_single_qubit_error(ptr: *mut CDevice, error: f64) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.set_default_single_qubit_error(error) };
    CqlibError::Ok as i32
}

/// Sets the device-wide default two-qubit gate error rate.
#[unsafe(no_mangle)]
pub extern "C" fn device_set_default_two_qubit_error(ptr: *mut CDevice, error: f64) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.set_default_two_qubit_error(error) };
    CqlibError::Ok as i32
}

// =====  Section 3.1.1: per-qubit stored-value getters and setters  =====

/// Returns the qubit frequency (GHz) recorded for `qubit` (no default
/// fallback). Returns 0 with the value in `*out`, or -8 when the qubit has
/// no recorded properties or no frequency is set.
#[unsafe(no_mangle)]
pub extern "C" fn device_get_frequency(ptr: *const CDevice, qubit: u32, out: *mut f64) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let Some(props) = (unsafe { (*ptr).inner.qubit_properties(PhysicalQubit::new(qubit)) }) else {
        return CqlibError::InvalidParam as i32;
    };
    optional_f64(props.frequency(), out)
}

/// Returns P(measure 0 | prepared 1) recorded for `qubit` (no default
/// fallback). Returns 0 with the value in `*out`, or -8 when the qubit has
/// no recorded properties or the probability is unset.
#[unsafe(no_mangle)]
pub extern "C" fn device_get_prob_meas0_prep1(
    ptr: *const CDevice,
    qubit: u32,
    out: *mut f64,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let Some(props) = (unsafe { (*ptr).inner.qubit_properties(PhysicalQubit::new(qubit)) }) else {
        return CqlibError::InvalidParam as i32;
    };
    optional_f64(props.prob_meas0_prep1(), out)
}

/// Returns P(measure 1 | prepared 0) recorded for `qubit` (no default
/// fallback). Returns 0 with the value in `*out`, or -8 when the qubit has
/// no recorded properties or the probability is unset.
#[unsafe(no_mangle)]
pub extern "C" fn device_get_prob_meas1_prep0(
    ptr: *const CDevice,
    qubit: u32,
    out: *mut f64,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let Some(props) = (unsafe { (*ptr).inner.qubit_properties(PhysicalQubit::new(qubit)) }) else {
        return CqlibError::InvalidParam as i32;
    };
    optional_f64(props.prob_meas1_prep0(), out)
}

/// Returns the error rate of the native instruction at `index` recorded for
/// `qubit`. Returns 0 with the value in `*out`, or -8 when the qubit has no
/// recorded properties or `index` is out of bounds.
#[unsafe(no_mangle)]
pub extern "C" fn device_get_error_rate(
    ptr: *const CDevice,
    qubit: u32,
    index: usize,
    out: *mut f64,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let Some(props) = (unsafe { (*ptr).inner.qubit_properties(PhysicalQubit::new(qubit)) }) else {
        return CqlibError::InvalidParam as i32;
    };
    let Some(prop) = props.native_instructions().get(index) else {
        return CqlibError::InvalidParam as i32;
    };
    unsafe { *out = prop.error_rate() };
    CqlibError::Ok as i32
}

/// Returns the duration (ns) of the native instruction at `index` recorded
/// for `qubit`. Returns 0 with the value in `*out`, or -8 when the qubit has
/// no recorded properties, `index` is out of bounds, or the duration is
/// unset.
#[unsafe(no_mangle)]
pub extern "C" fn device_get_length(
    ptr: *const CDevice,
    qubit: u32,
    index: usize,
    out: *mut f64,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let Some(props) = (unsafe { (*ptr).inner.qubit_properties(PhysicalQubit::new(qubit)) }) else {
        return CqlibError::InvalidParam as i32;
    };
    let Some(prop) = props.native_instructions().get(index) else {
        return CqlibError::InvalidParam as i32;
    };
    optional_f64(prop.length(), out)
}

/// Sets the T1 relaxation time (μs) recorded for `qubit`. Other recorded
/// properties are preserved.
///
/// Returns 0 on success, -1 on NULL, or -8 when the qubit has no recorded
/// properties.
#[unsafe(no_mangle)]
pub extern "C" fn device_set_t1(ptr: *mut CDevice, qubit: u32, t1: f64) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe {
        update_qubit_properties(ptr, qubit, |props| {
            props.set_t1(t1);
            Ok(())
        })
    }
}

/// Sets the T2 dephasing time (μs) recorded for `qubit`. Other recorded
/// properties are preserved.
///
/// Returns 0 on success, -1 on NULL, or -8 when the qubit has no recorded
/// properties.
#[unsafe(no_mangle)]
pub extern "C" fn device_set_t2(ptr: *mut CDevice, qubit: u32, t2: f64) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe {
        update_qubit_properties(ptr, qubit, |props| {
            props.set_t2(t2);
            Ok(())
        })
    }
}

/// Sets the qubit frequency (GHz) recorded for `qubit`. Other recorded
/// properties are preserved.
///
/// Returns 0 on success, -1 on NULL, or -8 when the qubit has no recorded
/// properties.
#[unsafe(no_mangle)]
pub extern "C" fn device_set_frequency(ptr: *mut CDevice, qubit: u32, frequency: f64) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe {
        update_qubit_properties(ptr, qubit, |props| {
            props.set_frequency(frequency);
            Ok(())
        })
    }
}

/// Sets P(measure 0 | prepared 1) recorded for `qubit`. Other recorded
/// properties are preserved.
///
/// Returns 0 on success, -1 on NULL, or -8 when the qubit has no recorded
/// properties.
#[unsafe(no_mangle)]
pub extern "C" fn device_set_prob_meas0_prep1(ptr: *mut CDevice, qubit: u32, prob: f64) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe {
        update_qubit_properties(ptr, qubit, |props| {
            props.set_prob_meas0_prep1(prob);
            Ok(())
        })
    }
}

/// Sets P(measure 1 | prepared 0) recorded for `qubit`. Other recorded
/// properties are preserved.
///
/// Returns 0 on success, -1 on NULL, or -8 when the qubit has no recorded
/// properties.
#[unsafe(no_mangle)]
pub extern "C" fn device_set_prob_meas1_prep0(ptr: *mut CDevice, qubit: u32, prob: f64) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe {
        update_qubit_properties(ptr, qubit, |props| {
            props.set_prob_meas1_prep0(prob);
            Ok(())
        })
    }
}

/// Sets the error rate of the native instruction at `index` recorded for
/// `qubit`. Other recorded properties are preserved.
///
/// Returns 0 on success, -1 on NULL, or -8 when the qubit has no recorded
/// properties or `index` is out of bounds.
#[unsafe(no_mangle)]
pub extern "C" fn device_set_error_rate(
    ptr: *mut CDevice,
    qubit: u32,
    index: usize,
    error_rate: f64,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe {
        update_qubit_native_instruction(ptr, qubit, index, |item| {
            item.set_error_rate(error_rate);
        })
    }
}

/// Sets the duration (ns) of the native instruction at `index` recorded for
/// `qubit`. Other recorded properties are preserved.
///
/// Returns 0 on success, -1 on NULL, or -8 when the qubit has no recorded
/// properties or `index` is out of bounds.
#[unsafe(no_mangle)]
pub extern "C" fn device_set_length(
    ptr: *mut CDevice,
    qubit: u32,
    index: usize,
    length: f64,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe {
        update_qubit_native_instruction(ptr, qubit, index, |item| {
            item.set_length(length);
        })
    }
}

/// Replaces the standard gate carried by the native instruction at `index`
/// recorded for `qubit`. The edited list is revalidated, so changing a
/// single-qubit gate to a two-qubit gate (or vice versa) is rejected.
///
/// Returns 0 on success, -1 on NULL, -4 for an unknown gate name, or -8 when
/// the qubit has no recorded properties, `index` is out of bounds, or the
/// edited list fails validation.
#[unsafe(no_mangle)]
pub extern "C" fn device_set_instruction(
    ptr: *mut CDevice,
    qubit: u32,
    index: usize,
    gate_name: *const c_char,
) -> i32 {
    if ptr.is_null() || gate_name.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return CqlibError::ParseError as i32,
    };
    let Some(gate) = standard_gate_from_name(name) else {
        return CqlibError::ParseError as i32;
    };
    unsafe {
        update_qubit_native_instruction(ptr, qubit, index, |item| {
            item.set_instruction(Instruction::Standard(gate));
        })
    }
}

/// Appends one native single-qubit instruction (standard gate `gate_name`
/// with `error_rate` and duration `length` in ns; NaN length = unset) to the
/// recorded properties of `qubit`.
///
/// Returns 0 on success, -1 on NULL, -4 for an unknown gate name, or -8 when
/// the qubit has no recorded properties or the instruction is rejected
/// (wrong arity or invalid calibration values).
#[unsafe(no_mangle)]
pub extern "C" fn device_set_native_instruction(
    ptr: *mut CDevice,
    qubit: u32,
    gate_name: *const c_char,
    error_rate: f64,
    length: f64,
) -> i32 {
    if ptr.is_null() || gate_name.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return CqlibError::ParseError as i32,
    };
    let Some(gate) = standard_gate_from_name(name) else {
        return CqlibError::ParseError as i32;
    };
    let mut prop = InstructionProp::new(Instruction::Standard(gate), error_rate);
    if !length.is_nan() {
        prop.set_length(length);
    }
    unsafe {
        update_qubit_properties(ptr, qubit, |props| {
            props
                .set_native_instruction(prop)
                .map_err(|e| device_error_code(&e))
        })
    }
}

/// Returns the system calibration timestamp as milliseconds since the Unix
/// epoch. Returns 0 with the value in `*out`, or -8 when unset.
///
/// Note: an explicit `set_calibration_time` binding is not yet available
/// because constructing the core timestamp type requires the `time` crate
/// as a direct dependency of `binding-c`.
#[unsafe(no_mangle)]
pub extern "C" fn device_calibration_time(ptr: *const CDevice, out_unix_ms: *mut i64) -> i32 {
    if ptr.is_null() || out_unix_ms.is_null() {
        return CqlibError::NullPtr as i32;
    }
    match unsafe { (*ptr).inner.calibration_time() } {
        Some(t) => {
            let nanos = t.unix_timestamp_nanos();
            unsafe { *out_unix_ms = nanos.div_euclid(1_000_000) as i64 };
            CqlibError::Ok as i32
        }
        None => CqlibError::InvalidParam as i32,
    }
}

/// Returns the number of invalid (offline/faulty) qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn device_invalid_qubits_len(ptr: *const CDevice) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.invalid_qubits().count() }
}

/// Copies the invalid (offline/faulty) qubit IDs into `out` (two-step
/// pattern; pair with `device_invalid_qubits_len`). Returns the total count.
#[unsafe(no_mangle)]
pub extern "C" fn device_invalid_qubits(ptr: *const CDevice, out: *mut u32, len: usize) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let items: Vec<u32> = unsafe { (*ptr).inner.invalid_qubits() }
        .map(|q| q.id())
        .collect();
    write_u32_list(&items, out, len)
}

/// Replaces the set of invalid (offline/faulty) qubits.
///
/// Returns 0 on success, -1 on NULL, or -2 when any qubit is not registered
/// with the device (the existing set is then preserved).
#[unsafe(no_mangle)]
pub extern "C" fn device_set_invalid_qubits(
    ptr: *mut CDevice,
    qubits: *const u32,
    len: usize,
) -> i32 {
    if ptr.is_null() || (len > 0 && qubits.is_null()) {
        return CqlibError::NullPtr as i32;
    }
    let raw = if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(qubits, len) }
    };
    let set: std::collections::HashSet<PhysicalQubit> =
        raw.iter().map(|&q| PhysicalQubit::new(q)).collect();
    let wrapper = unsafe { &mut *ptr };
    match wrapper.inner.set_invalid_qubits(set) {
        Ok(()) => CqlibError::Ok as i32,
        Err(_) => CqlibError::QubitOutOfBounds as i32,
    }
}

/// Returns 1 when `qubit` is registered with the device and not marked
/// invalid, 0 otherwise, or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn device_is_usable_qubit(ptr: *const CDevice, qubit: u32) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    unsafe { (*ptr).inner.is_usable_qubit(PhysicalQubit::new(qubit)) as i32 }
}

/// Returns the number of usable (registered and not invalid) qubits, or 0
/// for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn device_num_usable_qubits(ptr: *const CDevice) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_usable_qubits() }
}

/// Returns the number of usable qubit IDs available from
/// `device_usable_qubits`, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn device_usable_qubits_len(ptr: *const CDevice) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.usable_qubits().count() }
}

/// Copies the usable qubit IDs into `out` (two-step pattern; pair with
/// `device_usable_qubits_len`). Returns the total count.
#[unsafe(no_mangle)]
pub extern "C" fn device_usable_qubits(ptr: *const CDevice, out: *mut u32, len: usize) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let items: Vec<u32> = unsafe { (*ptr).inner.usable_qubits() }
        .map(|q| q.id())
        .collect();
    write_u32_list(&items, out, len)
}

/// Returns the number of physical qubits registered with the device
/// (including invalid ones), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn device_qubits_len(ptr: *const CDevice) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.qubits().count() }
}

/// Copies all registered physical qubit IDs (including invalid ones) into
/// `out` (two-step pattern; pair with `device_qubits_len`). Returns the
/// total count.
#[unsafe(no_mangle)]
pub extern "C" fn device_qubits(ptr: *const CDevice, out: *mut u32, len: usize) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let items: Vec<u32> = unsafe { (*ptr).inner.qubits() }.map(|q| q.id()).collect();
    write_u32_list(&items, out, len)
}

/// Writes the per-qubit property snapshot for `qubit` to `*out`.
///
/// Returns 0 on success, or -8 when no properties have been recorded for
/// the qubit (optional fields in the snapshot are NaN when unset; native
/// instructions must be queried separately).
#[unsafe(no_mangle)]
pub extern "C" fn device_qubit_properties(
    ptr: *const CDevice,
    qubit: u32,
    out: *mut CQubitProp,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let Some(props) = (unsafe { (*ptr).inner.qubit_properties(PhysicalQubit::new(qubit)) }) else {
        return CqlibError::InvalidParam as i32;
    };
    unsafe {
        *out = CQubitProp {
            readout_error: props.readout_error(),
            t1: props.t1().unwrap_or(f64::NAN),
            t2: props.t2().unwrap_or(f64::NAN),
            prob_meas0_prep1: props.prob_meas0_prep1().unwrap_or(f64::NAN),
            prob_meas1_prep0: props.prob_meas1_prep0().unwrap_or(f64::NAN),
            frequency: props.frequency().unwrap_or(f64::NAN),
            num_native_instructions: props.native_instructions().len(),
        };
    }
    CqlibError::Ok as i32
}

/// Writes the native instruction at `index` for `qubit` to `*out`.
///
/// `name` is heap-allocated and must be freed with `cqlib_string_free`;
/// `length` is NaN when unset. Returns 0 on success, or -8 when the qubit
/// has no recorded properties or `index` is out of bounds.
#[unsafe(no_mangle)]
pub extern "C" fn device_qubit_prop_native_instruction(
    ptr: *const CDevice,
    qubit: u32,
    index: usize,
    out: *mut CNativeInstruction,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let Some(props) = (unsafe { (*ptr).inner.qubit_properties(PhysicalQubit::new(qubit)) }) else {
        return CqlibError::InvalidParam as i32;
    };
    let Some(prop) = props.native_instructions().get(index) else {
        return CqlibError::InvalidParam as i32;
    };
    let name = match CString::new(prop.instruction().name()) {
        Ok(cs) => cs.into_raw(),
        Err(_) => return CqlibError::InvalidParam as i32,
    };
    unsafe {
        *out = CNativeInstruction {
            name,
            error_rate: prop.error_rate(),
            length: prop.length().unwrap_or(f64::NAN),
        };
    }
    CqlibError::Ok as i32
}

/// Writes the per-edge property snapshot for the directed coupling
/// `control -> target` to `*out`.
///
/// Returns 0 on success, or -8 when no properties have been recorded for
/// the edge.
#[unsafe(no_mangle)]
pub extern "C" fn device_edge_properties(
    ptr: *const CDevice,
    control: u32,
    target: u32,
    out: *mut CEdgeProp,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let Some(props) = (unsafe {
        (*ptr)
            .inner
            .edge_properties(PhysicalQubit::new(control), PhysicalQubit::new(target))
    }) else {
        return CqlibError::InvalidParam as i32;
    };
    unsafe {
        *out = CEdgeProp {
            num_native_instructions: props.native_instructions().len(),
        };
    }
    CqlibError::Ok as i32
}

/// Writes the native instruction at `index` for the directed coupling
/// `control -> target` to `*out` (two-qubit standard gates).
///
/// `name` is heap-allocated and must be freed with `cqlib_string_free`;
/// `length` is NaN when unset. Returns 0 on success, or -8 when the edge
/// has no recorded properties or `index` is out of bounds.
#[unsafe(no_mangle)]
pub extern "C" fn device_edge_prop_native_instruction(
    ptr: *const CDevice,
    control: u32,
    target: u32,
    index: usize,
    out: *mut CNativeInstruction,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let Some(props) = (unsafe {
        (*ptr)
            .inner
            .edge_properties(PhysicalQubit::new(control), PhysicalQubit::new(target))
    }) else {
        return CqlibError::InvalidParam as i32;
    };
    let Some(prop) = props.native_instructions().get(index) else {
        return CqlibError::InvalidParam as i32;
    };
    let name = match CString::new(prop.instruction().name()) {
        Ok(cs) => cs.into_raw(),
        Err(_) => return CqlibError::InvalidParam as i32,
    };
    unsafe {
        *out = CNativeInstruction {
            name,
            error_rate: prop.error_rate(),
            length: prop.length().unwrap_or(f64::NAN),
        };
    }
    CqlibError::Ok as i32
}

/// Records (or replaces) the properties of `qubit`.
///
/// Returns 0 on success, -1 on NULL, -2 when the qubit is not registered
/// with the device or absent from its topology, -4 for an unknown native
/// gate name, or -8 for invalid native-instruction calibration values.
#[unsafe(no_mangle)]
pub extern "C" fn device_add_qubit_properties(
    ptr: *mut CDevice,
    qubit: u32,
    input: *const CQubitPropInput,
) -> i32 {
    if ptr.is_null() || input.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let raw = unsafe { &*input };
    let instructions = match unsafe {
        native_instructions_from_c(
            raw.native_gate_names,
            raw.native_error_rates,
            raw.native_lengths,
            raw.num_native_instructions,
        )
    } {
        Ok(props) => props,
        Err(code) => return code,
    };
    let mut props = QubitProp::new(raw.readout_error);
    if !raw.t1.is_nan() {
        props.set_t1(raw.t1);
    }
    if !raw.t2.is_nan() {
        props.set_t2(raw.t2);
    }
    if !raw.prob_meas0_prep1.is_nan() {
        props.set_prob_meas0_prep1(raw.prob_meas0_prep1);
    }
    if !raw.prob_meas1_prep0.is_nan() {
        props.set_prob_meas1_prep0(raw.prob_meas1_prep0);
    }
    if !raw.frequency.is_nan() {
        props.set_frequency(raw.frequency);
    }
    for instruction in instructions {
        if let Err(e) = props.set_native_instruction(instruction) {
            return device_error_code(&e);
        }
    }
    let wrapper = unsafe { &mut *ptr };
    match wrapper
        .inner
        .add_qubit_properties(PhysicalQubit::new(qubit), props)
    {
        Ok(()) => CqlibError::Ok as i32,
        Err(e) => device_error_code(&e),
    }
}

/// Records (or replaces) the properties of the directed coupling
/// `control -> target`.
///
/// Returns 0 on success, -1 on NULL, -2 when the directed coupling is not
/// in the device topology, -4 for an unknown native gate name, or -8 when a
/// native instruction is not a two-qubit standard gate or has invalid
/// calibration values.
#[unsafe(no_mangle)]
pub extern "C" fn device_add_edge_properties(
    ptr: *mut CDevice,
    control: u32,
    target: u32,
    input: *const CEdgePropInput,
) -> i32 {
    if ptr.is_null() || input.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let raw = unsafe { &*input };
    let instructions = match unsafe {
        native_instructions_from_c(
            raw.native_gate_names,
            raw.native_error_rates,
            raw.native_lengths,
            raw.num_native_instructions,
        )
    } {
        Ok(props) => props,
        Err(code) => return code,
    };
    let mut props = EdgeProp::new();
    for instruction in instructions {
        if let Err(e) = props.set_native_instruction(instruction) {
            return device_error_code(&e);
        }
    }
    let wrapper = unsafe { &mut *ptr };
    match wrapper.inner.add_edge_properties(
        PhysicalQubit::new(control),
        PhysicalQubit::new(target),
        props,
    ) {
        Ok(()) => CqlibError::Ok as i32,
        Err(e) => device_error_code(&e),
    }
}

/// Returns whether the standard gate `gate_name` can execute natively on
/// the exact ordered physical `qargs` (`num_qargs` u32 IDs; an empty qargs
/// list is valid for `GPhase`).
///
/// Returns 1 when supported, 0 when not, -1 on NULL, or -4 for an unknown
/// gate name. Calibration values do not affect this capability query.
#[unsafe(no_mangle)]
pub extern "C" fn device_supports_native_instruction(
    ptr: *const CDevice,
    gate_name: *const c_char,
    qargs: *const u32,
    num_qargs: usize,
) -> i32 {
    if ptr.is_null() || gate_name.is_null() || (num_qargs > 0 && qargs.is_null()) {
        return CqlibError::NullPtr as i32;
    }
    let name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return CqlibError::ParseError as i32,
    };
    let Some(gate) = standard_gate_from_name(name) else {
        return CqlibError::ParseError as i32;
    };
    let qargs: Vec<PhysicalQubit> = if num_qargs == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(qargs, num_qargs) }
            .iter()
            .map(|&q| PhysicalQubit::new(q))
            .collect()
    };
    unsafe {
        (*ptr)
            .inner
            .supports_native_instruction(&Instruction::Standard(gate), &qargs) as i32
    }
}

/// Validates one resolved operation snapshot (from `circuit_index`) against
/// the device in the physical-qubit ID space.
///
/// The core type `Operation` has no dedicated C representation, so this
/// entry point (like the Python binding) validates the value-level
/// `CValueOperation` form via `Device::validate_value_operation`.
///
/// Returns 0 when valid, -3 when the device rejects the operation, or -1
/// on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn device_validate_operation(
    ptr: *const CDevice,
    operation: *const CValueOperation,
) -> i32 {
    device_validate_value_operation(ptr, operation)
}

/// Validates one resolved value-level operation snapshot (from
/// `circuit_index`) against the device in the physical-qubit ID space.
/// Control-flow bodies are checked recursively.
///
/// Returns 0 when valid, -3 when the device rejects the operation, or -1
/// on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn device_validate_value_operation(
    ptr: *const CDevice,
    operation: *const CValueOperation,
) -> i32 {
    if ptr.is_null() || operation.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let device = unsafe { &(*ptr).inner };
    let operation = unsafe { &(*operation).inner };
    match device.validate_value_operation(operation) {
        Ok(()) => CqlibError::Ok as i32,
        Err(_) => CqlibError::CircuitError as i32,
    }
}

/// Creates a device whose physical qubits (given by ID array `qubits`) are
/// connected as a directed line in the supplied order. Returns NULL on
/// error.
#[unsafe(no_mangle)]
pub extern "C" fn device_line_from_qubits(
    name: *const c_char,
    qubits: *const u32,
    num_qubits: usize,
) -> *mut CDevice {
    if name.is_null() || (num_qubits > 0 && qubits.is_null()) {
        return std::ptr::null_mut();
    }
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let physical: Vec<PhysicalQubit> = if num_qubits == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(qubits, num_qubits) }
            .iter()
            .map(|&q| PhysicalQubit::new(q))
            .collect()
    };
    match Device::line_from_qubits(name, physical) {
        Ok(device) => Box::into_raw(Box::new(CDevice { inner: device })),
        Err(_) => std::ptr::null_mut(),
    }
}

pub use layout::*;
pub use noise::*;
pub use qubit::*;
pub use result::*;
pub use topology::*;
