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
use cqlib_core::device::OperationKey;
use cqlib_core::device::noise::{ReadoutError, SingleQubitNoise, TwoQubitNoise};
use cqlib_core::qis::pauli::Pauli;
use num_complex::Complex64;
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

// =====  Section 3.4: Pauli channels and noise queries  =====

/// Single-qubit Pauli channel tag for `noise_model_add_single_qubit_pauli`
/// and `CSingleQubitNoise.tag`; the channel parameters are the probabilities
/// `px`/`py`/`pz` (sum <= 1).
pub const NOISE_PAULI: u8 = 5;

/// Two-qubit noise channel tags for `CTwoQubitNoise.tag`.
///
/// | Value | Channel          | Parameter meaning                    |
/// |-------|------------------|--------------------------------------|
/// | 0     | Depolarizing     | depolarizing parameter p             |
/// | 1     | Independent      | per-qubit channels in q0/q1_noise    |
/// | 2     | CorrelatedPauli  | probability p + `NOISE_PAULI_OP_*`   |
pub const NOISE_TWO_INDEPENDENT: u8 = 1;
pub const NOISE_TWO_CORRELATED_PAULI: u8 = 2;

/// Pauli operator tags used by `CTwoQubitNoise.op_q0`/`op_q1` for the
/// correlated-Pauli channel.
pub const NOISE_PAULI_OP_I: u8 = 0;
pub const NOISE_PAULI_OP_X: u8 = 1;
pub const NOISE_PAULI_OP_Y: u8 = 2;
pub const NOISE_PAULI_OP_Z: u8 = 3;

/// C snapshot of a single-qubit noise channel.
///
/// `tag` selects the channel (see the single-qubit `NOISE_*` constants);
/// `p` carries the parameter for tags 0-4 and `px`/`py`/`pz` carry the
/// Pauli probabilities for tag 5. Fields that do not apply to the tagged
/// channel are zero.
#[repr(C)]
pub struct CSingleQubitNoise {
    /// One of the single-qubit `NOISE_*` tags (0-5).
    pub tag: u8,
    /// Channel probability for tags 0-4.
    pub p: f64,
    /// Pauli-X probability for tag 5 (`NOISE_PAULI`).
    pub px: f64,
    /// Pauli-Y probability for tag 5 (`NOISE_PAULI`).
    pub py: f64,
    /// Pauli-Z probability for tag 5 (`NOISE_PAULI`).
    pub pz: f64,
}

/// C snapshot of a two-qubit noise channel.
///
/// `tag` selects the channel (see the `NOISE_TWO_*` constants); fields that
/// do not apply to the tagged channel are zero.
#[repr(C)]
pub struct CTwoQubitNoise {
    /// One of the `NOISE_TWO_*` tags.
    pub tag: u8,
    /// Channel probability for the depolarizing and correlated-Pauli tags.
    pub p: f64,
    /// Channel applied to q0 for the independent tag.
    pub q0_noise: CSingleQubitNoise,
    /// Channel applied to q1 for the independent tag.
    pub q1_noise: CSingleQubitNoise,
    /// Pauli operator applied to q0 (`NOISE_PAULI_OP_*`), correlated tag.
    pub op_q0: u8,
    /// Pauli operator applied to q1 (`NOISE_PAULI_OP_*`), correlated tag.
    pub op_q1: u8,
}

/// Converts a core single-qubit channel into the `CSingleQubitNoise` mirror.
fn single_qubit_noise_to_c(noise: &SingleQubitNoise) -> CSingleQubitNoise {
    let mut out = CSingleQubitNoise {
        tag: 0,
        p: 0.0,
        px: 0.0,
        py: 0.0,
        pz: 0.0,
    };
    match *noise {
        SingleQubitNoise::BitFlip(p) => {
            out.tag = NOISE_BIT_FLIP;
            out.p = p;
        }
        SingleQubitNoise::PhaseFlip(p) => {
            out.tag = NOISE_PHASE_FLIP;
            out.p = p;
        }
        SingleQubitNoise::Depolarizing(p) => {
            out.tag = NOISE_DEPOLARIZING;
            out.p = p;
        }
        SingleQubitNoise::AmplitudeDamping(p) => {
            out.tag = NOISE_AMPLITUDE_DAMPING;
            out.p = p;
        }
        SingleQubitNoise::PhaseDamping(p) => {
            out.tag = NOISE_PHASE_DAMPING;
            out.p = p;
        }
        SingleQubitNoise::Pauli { px, py, pz } => {
            out.tag = NOISE_PAULI;
            out.px = px;
            out.py = py;
            out.pz = pz;
        }
    }
    out
}

/// Maps a Pauli operator to its `NOISE_PAULI_OP_*` tag.
fn pauli_op_tag(pauli: &Pauli) -> u8 {
    match pauli {
        Pauli::I => NOISE_PAULI_OP_I,
        Pauli::X => NOISE_PAULI_OP_X,
        Pauli::Y => NOISE_PAULI_OP_Y,
        Pauli::Z => NOISE_PAULI_OP_Z,
    }
}

/// Converts a core two-qubit channel into the `CTwoQubitNoise` mirror.
fn two_qubit_noise_to_c(noise: &TwoQubitNoise) -> CTwoQubitNoise {
    let mut out = CTwoQubitNoise {
        tag: 0,
        p: 0.0,
        q0_noise: CSingleQubitNoise {
            tag: 0,
            p: 0.0,
            px: 0.0,
            py: 0.0,
            pz: 0.0,
        },
        q1_noise: CSingleQubitNoise {
            tag: 0,
            p: 0.0,
            px: 0.0,
            py: 0.0,
            pz: 0.0,
        },
        op_q0: 0,
        op_q1: 0,
    };
    match noise {
        TwoQubitNoise::Depolarizing(p) => {
            out.tag = NOISE_TWO_DEPOLARIZING;
            out.p = *p;
        }
        TwoQubitNoise::Independent { q0_noise, q1_noise } => {
            out.tag = NOISE_TWO_INDEPENDENT;
            out.q0_noise = single_qubit_noise_to_c(q0_noise);
            out.q1_noise = single_qubit_noise_to_c(q1_noise);
        }
        TwoQubitNoise::CorrelatedPauli { op_q0, op_q1, p } => {
            out.tag = NOISE_TWO_CORRELATED_PAULI;
            out.p = *p;
            out.op_q0 = pauli_op_tag(op_q0);
            out.op_q1 = pauli_op_tag(op_q1);
        }
    }
    out
}

/// Adds a general single-qubit Pauli channel to `gate_name` acting on
/// `qubit`, with X/Y/Z probabilities `px`, `py`, `pz` (each >= 0, sum <= 1).
///
/// Returns 0 on success, -1 on NULL, -4 for an unknown gate name, or -8 for
/// invalid probabilities.
#[unsafe(no_mangle)]
pub extern "C" fn noise_model_add_single_qubit_pauli(
    ptr: *mut CNoiseModel,
    gate_name: *const c_char,
    qubit: u32,
    px: f64,
    py: f64,
    pz: f64,
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
    let noise = SingleQubitNoise::Pauli { px, py, pz };
    let wrapper = unsafe { &mut *ptr };
    match wrapper
        .inner
        .add_single_qubit_error(gate, Qubit::new(qubit), noise)
    {
        Ok(()) => 0,
        Err(_) => CqlibError::InvalidParam as i32,
    }
}

/// Returns the number of single-qubit noise channels recorded for the
/// standard gate `gate_name` on `qubit`, or 0 for NULL, an unknown gate
/// name, or absent entries.
#[unsafe(no_mangle)]
pub extern "C" fn noise_model_get_single_qubit_errors_len(
    ptr: *const CNoiseModel,
    gate_name: *const c_char,
    qubit: u32,
) -> usize {
    if ptr.is_null() || gate_name.is_null() {
        return 0;
    }
    let name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let Some(gate) = standard_gate_from_name(name) else {
        return 0;
    };
    let key = OperationKey::new_single(gate, Qubit::new(qubit));
    unsafe {
        (*ptr)
            .inner
            .get_single_qubit_errors(&key)
            .map_or(0, |v| v.len())
    }
}

/// Copies the single-qubit noise channels recorded for the standard gate
/// `gate_name` on `qubit` into `out` (two-step pattern; pair with
/// `noise_model_get_single_qubit_errors_len`).
///
/// Each entry is a `CSingleQubitNoise` snapshot whose valid fields depend on
/// its `tag`. Returns the total number of recorded channels (0 for NULL, an
/// unknown gate name, or absent entries).
#[unsafe(no_mangle)]
pub extern "C" fn noise_model_get_single_qubit_errors(
    ptr: *const CNoiseModel,
    gate_name: *const c_char,
    qubit: u32,
    out: *mut CSingleQubitNoise,
    len: usize,
) -> usize {
    if ptr.is_null() || gate_name.is_null() {
        return 0;
    }
    let name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let Some(gate) = standard_gate_from_name(name) else {
        return 0;
    };
    let key = OperationKey::new_single(gate, Qubit::new(qubit));
    let Some(errors) = (unsafe { (*ptr).inner.get_single_qubit_errors(&key) }) else {
        return 0;
    };
    let total = errors.len();
    if !out.is_null() {
        for (i, noise) in errors.iter().take(total.min(len)).enumerate() {
            unsafe { *out.add(i) = single_qubit_noise_to_c(noise) };
        }
    }
    total
}

/// Returns the number of two-qubit noise channels recorded for the standard
/// gate `gate_name` on the ordered pair `(q0, q1)`, or 0 for NULL, an
/// unknown gate name, colliding qubits, or absent entries.
#[unsafe(no_mangle)]
pub extern "C" fn noise_model_get_two_qubit_errors_len(
    ptr: *const CNoiseModel,
    gate_name: *const c_char,
    q0: u32,
    q1: u32,
) -> usize {
    if ptr.is_null() || gate_name.is_null() {
        return 0;
    }
    let name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let Some(gate) = standard_gate_from_name(name) else {
        return 0;
    };
    let Ok(key) = OperationKey::new_double(gate, Qubit::new(q0), Qubit::new(q1)) else {
        return 0;
    };
    unsafe {
        (*ptr)
            .inner
            .get_two_qubit_errors(&key)
            .map_or(0, |v| v.len())
    }
}

/// Copies the two-qubit noise channels recorded for the standard gate
/// `gate_name` on the ordered pair `(q0, q1)` into `out` (two-step pattern;
/// pair with `noise_model_get_two_qubit_errors_len`).
///
/// Each entry is a `CTwoQubitNoise` snapshot whose valid fields depend on
/// its `tag`. Returns the total number of recorded channels (0 for NULL, an
/// unknown gate name, colliding qubits, or absent entries).
#[unsafe(no_mangle)]
pub extern "C" fn noise_model_get_two_qubit_errors(
    ptr: *const CNoiseModel,
    gate_name: *const c_char,
    q0: u32,
    q1: u32,
    out: *mut CTwoQubitNoise,
    len: usize,
) -> usize {
    if ptr.is_null() || gate_name.is_null() {
        return 0;
    }
    let name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let Some(gate) = standard_gate_from_name(name) else {
        return 0;
    };
    let Ok(key) = OperationKey::new_double(gate, Qubit::new(q0), Qubit::new(q1)) else {
        return 0;
    };
    let Some(errors) = (unsafe { (*ptr).inner.get_two_qubit_errors(&key) }) else {
        return 0;
    };
    let total = errors.len();
    if !out.is_null() {
        for (i, noise) in errors.iter().take(total.min(len)).enumerate() {
            unsafe { *out.add(i) = two_qubit_noise_to_c(noise) };
        }
    }
    total
}

/// Writes the readout error recorded for `qubit` to the out parameters.
///
/// `*out_p_0_given_1` receives P(measure 0 | true 1) and `*out_p_1_given_0`
/// receives P(measure 1 | true 0). Returns 0 on success, -1 on NULL, or -8
/// when no readout error is recorded for the qubit.
#[unsafe(no_mangle)]
pub extern "C" fn noise_model_get_readout_error(
    ptr: *const CNoiseModel,
    qubit: u32,
    out_p_0_given_1: *mut f64,
    out_p_1_given_0: *mut f64,
) -> i32 {
    if ptr.is_null() || out_p_0_given_1.is_null() || out_p_1_given_0.is_null() {
        return CqlibError::NullPtr as i32;
    }
    match unsafe { (*ptr).inner.get_readout_error(&Qubit::new(qubit)) } {
        Some(error) => {
            unsafe {
                *out_p_0_given_1 = error.p_0_given_1;
                *out_p_1_given_0 = error.p_1_given_0;
            }
            CqlibError::Ok as i32
        }
        None => CqlibError::InvalidParam as i32,
    }
}

// =====  Section 3.5: channel validity and Kraus output  =====

/// Resolves the single-qubit channel recorded for the standard gate
/// `gate_name` on `qubit` at `index`.
///
/// Returns the channel, or the error code -1 (NULL), -4 (invalid UTF-8 or
/// unknown gate name), or -8 (no channels recorded for the key or `index`
/// out of bounds).
fn resolve_single_channel(
    ptr: *const CNoiseModel,
    gate_name: *const c_char,
    qubit: u32,
    index: usize,
) -> Result<SingleQubitNoise, i32> {
    if ptr.is_null() || gate_name.is_null() {
        return Err(CqlibError::NullPtr as i32);
    }
    let name = unsafe { CStr::from_ptr(gate_name) }
        .to_str()
        .map_err(|_| CqlibError::ParseError as i32)?;
    let gate = standard_gate_from_name(name).ok_or(CqlibError::ParseError as i32)?;
    let key = OperationKey::new_single(gate, Qubit::new(qubit));
    let errors = unsafe { (*ptr).inner.get_single_qubit_errors(&key) };
    errors
        .ok_or(CqlibError::InvalidParam as i32)?
        .get(index)
        .copied()
        .ok_or(CqlibError::InvalidParam as i32)
}

/// Reports whether the single-qubit channel recorded for the standard gate
/// `gate_name` on `qubit` at `index` carries valid noise parameters.
///
/// Returns 1 when valid, 0 when not, -1 on NULL, -4 for an unknown gate
/// name, or -8 when no channel exists at that key/index.
#[unsafe(no_mangle)]
pub extern "C" fn noise_model_is_valid(
    ptr: *const CNoiseModel,
    gate_name: *const c_char,
    qubit: u32,
    index: usize,
) -> i32 {
    match resolve_single_channel(ptr, gate_name, qubit, index) {
        Ok(noise) => i32::from(noise.is_valid()),
        Err(code) => code,
    }
}

/// Returns the total number of `Complex64` elements across all Kraus
/// operators of the single-qubit channel recorded for the standard gate
/// `gate_name` on `qubit` at `index` (each operator contributes `dim * dim`
/// elements), or 0 for NULL, an unknown gate name, or an absent key/index.
#[unsafe(no_mangle)]
pub extern "C" fn noise_model_to_kraus_len(
    ptr: *const CNoiseModel,
    gate_name: *const c_char,
    qubit: u32,
    index: usize,
) -> usize {
    match resolve_single_channel(ptr, gate_name, qubit, index) {
        Ok(noise) => noise.to_kraus().iter().map(|op| op.len()).sum(),
        Err(_) => 0,
    }
}

/// Copies all Kraus operators of the single-qubit channel recorded for the
/// standard gate `gate_name` on `qubit` at `index` into `out` (two-step
/// pattern; pair with `noise_model_to_kraus_len`).
///
/// Operators are written one after another, each in row-major order as
/// `dim * dim` `Complex64` re/im pairs. Returns the total element count, or
/// 0 for NULL, an unknown gate name, or an absent key/index.
#[unsafe(no_mangle)]
pub extern "C" fn noise_model_to_kraus(
    ptr: *const CNoiseModel,
    gate_name: *const c_char,
    qubit: u32,
    index: usize,
    out: *mut Complex64,
    len: usize,
) -> usize {
    let noise = match resolve_single_channel(ptr, gate_name, qubit, index) {
        Ok(noise) => noise,
        Err(_) => return 0,
    };
    let ops = noise.to_kraus();
    let total: usize = ops.iter().map(|op| op.len()).sum();
    if !out.is_null() {
        let mut written = 0usize;
        for op in &ops {
            let (rows, cols) = op.dim();
            for row in 0..rows {
                for col in 0..cols {
                    if written >= len {
                        return total;
                    }
                    unsafe { *out.add(written) = op[[row, col]] };
                    written += 1;
                }
            }
        }
    }
    total
}
