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

//! C ABI for `NoiseModel`.

use crate::device::{CNoiseModel, standard_gate_from_name};
use crate::error::CqlibError;
use cqlib_core::circuit::Qubit;
use cqlib_core::device::noise::{ReadoutError, SingleQubitNoise, TwoQubitNoise};
use std::ffi::CStr;
use std::os::raw::c_char;

/// Single-qubit noise channel tags for `noise_model_add_single_qubit`.
///
/// | Value | Channel          | Parameter meaning        |
/// |-------|------------------|--------------------------|
/// | 0     | BitFlip          | flip probability p       |
/// | 1     | PhaseFlip        | flip probability p       |
/// | 2     | Depolarizing     | depolarizing parameter p |
/// | 3     | AmplitudeDamping | damping parameter gamma  |
/// | 4     | PhaseDamping     | scattering probability   |
pub const NOISE_BIT_FLIP: u8 = 0;
pub const NOISE_PHASE_FLIP: u8 = 1;
pub const NOISE_DEPOLARIZING: u8 = 2;
pub const NOISE_AMPLITUDE_DAMPING: u8 = 3;
pub const NOISE_PHASE_DAMPING: u8 = 4;

/// Two-qubit noise channel tags for `noise_model_add_two_qubit`.
///
/// | Value | Channel      | Parameter meaning        |
/// |-------|--------------|--------------------------|
/// | 0     | Depolarizing | depolarizing parameter p |
pub const NOISE_TWO_DEPOLARIZING: u8 = 0;

fn single_qubit_noise(tag: u8, p: f64) -> Option<SingleQubitNoise> {
    Some(match tag {
        NOISE_BIT_FLIP => SingleQubitNoise::BitFlip(p),
        NOISE_PHASE_FLIP => SingleQubitNoise::PhaseFlip(p),
        NOISE_DEPOLARIZING => SingleQubitNoise::Depolarizing(p),
        NOISE_AMPLITUDE_DAMPING => SingleQubitNoise::AmplitudeDamping(p),
        NOISE_PHASE_DAMPING => SingleQubitNoise::PhaseDamping(p),
        _ => return None,
    })
}

/// Creates a new empty noise model.
#[unsafe(no_mangle)]
pub extern "C" fn noise_model_new() -> *mut CNoiseModel {
    Box::into_raw(Box::new(CNoiseModel {
        inner: cqlib_core::device::NoiseModel::new(),
    }))
}

/// Frees a noise model. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn noise_model_free(ptr: *mut CNoiseModel) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Adds a single-qubit noise channel to `gate_name` acting on `qubit`.
///
/// `noise_type` is one of the `NOISE_*` single-qubit tags; `p` is the
/// channel parameter. Returns 0 on success, -4 for unknown gate/noise tag,
/// or -8 for invalid probabilities.
#[unsafe(no_mangle)]
pub extern "C" fn noise_model_add_single_qubit(
    ptr: *mut CNoiseModel,
    gate_name: *const c_char,
    qubit: u32,
    noise_type: u8,
    p: f64,
) -> i32 {
    if ptr.is_null() || gate_name.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let gate_name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return CqlibError::ParseError as i32,
    };
    let gate = match standard_gate_from_name(gate_name) {
        Some(g) => g,
        None => return CqlibError::ParseError as i32,
    };
    let noise = match single_qubit_noise(noise_type, p) {
        Some(n) => n,
        None => return CqlibError::InvalidParam as i32,
    };
    let wrapper = unsafe { &mut *ptr };
    match wrapper
        .inner
        .add_single_qubit_error(gate, Qubit::new(qubit), noise)
    {
        Ok(()) => 0,
        Err(_) => CqlibError::InvalidParam as i32,
    }
}

/// Adds a two-qubit depolarizing channel to `gate_name` acting on
/// `(q0, q1)`. Returns 0 on success, -4 for unknown gate, or -8 when the
/// qubits collide or the probability is invalid.
#[unsafe(no_mangle)]
pub extern "C" fn noise_model_add_two_qubit(
    ptr: *mut CNoiseModel,
    gate_name: *const c_char,
    q0: u32,
    q1: u32,
    noise_type: u8,
    p: f64,
) -> i32 {
    if ptr.is_null() || gate_name.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if noise_type != NOISE_TWO_DEPOLARIZING {
        return CqlibError::InvalidParam as i32;
    }
    let gate_name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return CqlibError::ParseError as i32,
    };
    let gate = match standard_gate_from_name(gate_name) {
        Some(g) => g,
        None => return CqlibError::ParseError as i32,
    };
    let wrapper = unsafe { &mut *ptr };
    match wrapper.inner.add_two_qubit_error(
        gate,
        Qubit::new(q0),
        Qubit::new(q1),
        TwoQubitNoise::Depolarizing(p),
    ) {
        Ok(()) => 0,
        Err(_) => CqlibError::InvalidParam as i32,
    }
}

/// Adds an asymmetric readout error for `qubit`.
///
/// `p_0_given_1` is P(measure 0 | true 1); `p_1_given_0` is P(1 | 0).
/// Returns 0 on success or -8 for invalid probabilities.
#[unsafe(no_mangle)]
pub extern "C" fn noise_model_add_readout(
    ptr: *mut CNoiseModel,
    qubit: u32,
    p_0_given_1: f64,
    p_1_given_0: f64,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let error = ReadoutError {
        p_0_given_1,
        p_1_given_0,
    };
    let wrapper = unsafe { &mut *ptr };
    match wrapper.inner.add_readout_error(Qubit::new(qubit), error) {
        Ok(()) => 0,
        Err(_) => CqlibError::InvalidParam as i32,
    }
}
