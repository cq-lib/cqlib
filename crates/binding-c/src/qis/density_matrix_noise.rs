// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! C ABI for the `DensityMatrixNoise` simulator.
//!
//! Mirrors the `Statevector`/`DensityMatrix` binding patterns. A noise model
//! is attached at construction time via a `CNoiseModel*` handle (NULL for an
//! ideal simulation); gate noise is applied automatically after every gate.

use crate::circuit::CCircuit;
use crate::device::{CNoiseModel, standard_gate_from_name};
use crate::error::CqlibError;
use crate::qis::{CDensityMatrixNoise, CHamiltonian, COutcomeList, check_qubit, qis_err_code};
use cqlib_core::qis::{DensityMatrixNoise, Observable};
use ndarray::Array2;
use num_complex::Complex64;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[inline]
fn map_err(err: cqlib_core::qis::QisError) -> i32 {
    qis_err_code(&err)
}

#[inline]
fn num_qubits_of(wrapper: &CDensityMatrixNoise) -> usize {
    wrapper.inner.state.num_qubits
}

/// Clone the optional noise model referenced by a C handle.
fn clone_noise_model(noise_model: *const CNoiseModel) -> Option<cqlib_core::device::NoiseModel> {
    if noise_model.is_null() {
        None
    } else {
        Some(unsafe { (*noise_model).inner.clone() })
    }
}

/// Apply a single-qubit method that takes only a qubit index.
fn apply_single(
    ptr: *mut CDensityMatrixNoise,
    qubit: u32,
    f: impl FnOnce(&mut DensityMatrixNoise, usize) -> Result<(), cqlib_core::qis::QisError>,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let q = match check_qubit(num_qubits_of(wrapper), qubit) {
        Ok(q) => q,
        Err(code) => return code,
    };
    f(&mut wrapper.inner, q)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

/// Apply a two-qubit method.
fn apply_two(
    ptr: *mut CDensityMatrixNoise,
    q0: u32,
    q1: u32,
    f: impl FnOnce(&mut DensityMatrixNoise, usize, usize) -> Result<(), cqlib_core::qis::QisError>,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let a = match check_qubit(num_qubits_of(wrapper), q0) {
        Ok(q) => q,
        Err(code) => return code,
    };
    let b = match check_qubit(num_qubits_of(wrapper), q1) {
        Ok(q) => q,
        Err(code) => return code,
    };
    f(&mut wrapper.inner, a, b)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

/// Apply a three-qubit method.
fn apply_three(
    ptr: *mut CDensityMatrixNoise,
    q0: u32,
    q1: u32,
    q2: u32,
    f: impl FnOnce(
        &mut DensityMatrixNoise,
        usize,
        usize,
        usize,
    ) -> Result<(), cqlib_core::qis::QisError>,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let a = match check_qubit(num_qubits_of(wrapper), q0) {
        Ok(q) => q,
        Err(code) => return code,
    };
    let b = match check_qubit(num_qubits_of(wrapper), q1) {
        Ok(q) => q,
        Err(code) => return code,
    };
    let c = match check_qubit(num_qubits_of(wrapper), q2) {
        Ok(q) => q,
        Err(code) => return code,
    };
    f(&mut wrapper.inner, a, b, c)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

/// Create a new noisy simulator with `num_qubits` initialised to |0...0>.
/// Pass NULL `noise_model` for an ideal (noise-free) simulation.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_new(
    num_qubits: usize,
    noise_model: *const CNoiseModel,
) -> *mut CDensityMatrixNoise {
    let noise = clone_noise_model(noise_model);
    Box::into_raw(Box::new(CDensityMatrixNoise {
        inner: DensityMatrixNoise::new(num_qubits, noise),
    }))
}

/// Free a noisy simulator. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_free(ptr: *mut CDensityMatrixNoise) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Build a noisy simulator by simulating `circuit`. Returns NULL on error.
/// Pass NULL `noise_model` for an ideal (noise-free) simulation.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_from_circuit(
    circuit: *const CCircuit,
    noise_model: *const CNoiseModel,
) -> *mut CDensityMatrixNoise {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner };
    let noise = clone_noise_model(noise_model);
    match DensityMatrixNoise::from_circuit(circuit, noise) {
        Ok(sim) => Box::into_raw(Box::new(CDensityMatrixNoise { inner: sim })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Return the number of qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_num_qubits(ptr: *const CDensityMatrixNoise) -> usize {
    if ptr.is_null() {
        return 0;
    }
    num_qubits_of(unsafe { &*ptr })
}

/// Apply `circuit` to the simulator in place, injecting gate noise after
/// each operation when a noise model is attached. Returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_circuit(
    ptr: *mut CDensityMatrixNoise,
    circuit: *const CCircuit,
) -> i32 {
    if ptr.is_null() || circuit.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let circuit = unsafe { &(*circuit).inner };
    wrapper
        .inner
        .apply_circuit(circuit)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_h(ptr: *mut CDensityMatrixNoise, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrixNoise::apply_h)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_x(ptr: *mut CDensityMatrixNoise, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrixNoise::apply_x)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_y(ptr: *mut CDensityMatrixNoise, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrixNoise::apply_y)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_z(ptr: *mut CDensityMatrixNoise, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrixNoise::apply_z)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_s(ptr: *mut CDensityMatrixNoise, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrixNoise::apply_s)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_sdg(ptr: *mut CDensityMatrixNoise, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrixNoise::apply_sdg)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_t(ptr: *mut CDensityMatrixNoise, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrixNoise::apply_t)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_tdg(ptr: *mut CDensityMatrixNoise, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrixNoise::apply_tdg)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_x2p(ptr: *mut CDensityMatrixNoise, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrixNoise::apply_x2p)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_x2m(ptr: *mut CDensityMatrixNoise, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrixNoise::apply_x2m)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_y2p(ptr: *mut CDensityMatrixNoise, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrixNoise::apply_y2p)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_y2m(ptr: *mut CDensityMatrixNoise, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrixNoise::apply_y2m)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_rx(
    ptr: *mut CDensityMatrixNoise,
    qubit: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sim, q| sim.apply_rx(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_ry(
    ptr: *mut CDensityMatrixNoise,
    qubit: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sim, q| sim.apply_ry(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_rz(
    ptr: *mut CDensityMatrixNoise,
    qubit: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sim, q| sim.apply_rz(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_phase(
    ptr: *mut CDensityMatrixNoise,
    qubit: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sim, q| sim.apply_phase(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_u(
    ptr: *mut CDensityMatrixNoise,
    qubit: u32,
    theta: f64,
    phi: f64,
    lambda: f64,
) -> i32 {
    if !theta.is_finite() || !phi.is_finite() || !lambda.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sim, q| sim.apply_u(q, theta, phi, lambda))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_xy(
    ptr: *mut CDensityMatrixNoise,
    qubit: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sim, q| sim.apply_xy(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_xy2p(
    ptr: *mut CDensityMatrixNoise,
    qubit: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sim, q| sim.apply_xy2p(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_xy2m(
    ptr: *mut CDensityMatrixNoise,
    qubit: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sim, q| sim.apply_xy2m(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_rxy(
    ptr: *mut CDensityMatrixNoise,
    qubit: u32,
    theta: f64,
    phi: f64,
) -> i32 {
    if !theta.is_finite() || !phi.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sim, q| sim.apply_rxy(q, theta, phi))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_gphase(
    ptr: *mut CDensityMatrixNoise,
    phi: f64,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if !phi.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    wrapper
        .inner
        .apply_gphase(phi)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_cx(
    ptr: *mut CDensityMatrixNoise,
    control: u32,
    target: u32,
) -> i32 {
    apply_two(ptr, control, target, DensityMatrixNoise::apply_cx)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_cy(
    ptr: *mut CDensityMatrixNoise,
    control: u32,
    target: u32,
) -> i32 {
    apply_two(ptr, control, target, DensityMatrixNoise::apply_cy)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_cz(
    ptr: *mut CDensityMatrixNoise,
    q0: u32,
    q1: u32,
) -> i32 {
    apply_two(ptr, q0, q1, DensityMatrixNoise::apply_cz)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_swap(
    ptr: *mut CDensityMatrixNoise,
    q0: u32,
    q1: u32,
) -> i32 {
    apply_two(ptr, q0, q1, DensityMatrixNoise::apply_swap)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_ccx(
    ptr: *mut CDensityMatrixNoise,
    c0: u32,
    c1: u32,
    target: u32,
) -> i32 {
    apply_three(ptr, c0, c1, target, DensityMatrixNoise::apply_ccx)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_rxx(
    ptr: *mut CDensityMatrixNoise,
    q0: u32,
    q1: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, q0, q1, |sim, a, b| sim.apply_rxx(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_ryy(
    ptr: *mut CDensityMatrixNoise,
    q0: u32,
    q1: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, q0, q1, |sim, a, b| sim.apply_ryy(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_rzz(
    ptr: *mut CDensityMatrixNoise,
    q0: u32,
    q1: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, q0, q1, |sim, a, b| sim.apply_rzz(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_rzx(
    ptr: *mut CDensityMatrixNoise,
    q0: u32,
    q1: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, q0, q1, |sim, a, b| sim.apply_rzx(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_crx(
    ptr: *mut CDensityMatrixNoise,
    control: u32,
    target: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, control, target, |sim, a, b| sim.apply_crx(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_cry(
    ptr: *mut CDensityMatrixNoise,
    control: u32,
    target: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, control, target, |sim, a, b| sim.apply_cry(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_crz(
    ptr: *mut CDensityMatrixNoise,
    control: u32,
    target: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, control, target, |sim, a, b| sim.apply_crz(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_fsim(
    ptr: *mut CDensityMatrixNoise,
    q0: u32,
    q1: u32,
    theta: f64,
    phi: f64,
) -> i32 {
    if !theta.is_finite() || !phi.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, q0, q1, |sim, a, b| sim.apply_fsim(a, b, theta, phi))
}

/// Apply a standard gate identified by `gate_name` (e.g. "H", "CX", "RZZ",
/// "GPhase", "FSIM"), followed by any matching gate noise from the attached
/// noise model. Unknown gate names return -8 (InvalidParam).
///
/// `qubits` holds `qubit_len` target qubit indices; `params` holds
/// `param_len` gate parameters. Both counts must match the gate's arity.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_standard_gate_noise(
    ptr: *mut CDensityMatrixNoise,
    gate_name: *const c_char,
    qubits: *const u32,
    qubit_len: usize,
    params: *const f64,
    param_len: usize,
) -> i32 {
    if ptr.is_null() || gate_name.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if qubit_len > 0 && qubits.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if param_len > 0 && params.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return CqlibError::ParseError as i32,
    };
    let gate = match standard_gate_from_name(name) {
        Some(g) => g,
        None => return CqlibError::InvalidParam as i32,
    };
    let wrapper = unsafe { &mut *ptr };
    let qs: Result<Vec<usize>, i32> = if qubit_len == 0 {
        Ok(Vec::new())
    } else {
        unsafe { std::slice::from_raw_parts(qubits, qubit_len) }
            .iter()
            .map(|&q| check_qubit(num_qubits_of(wrapper), q))
            .collect()
    };
    let qs = match qs {
        Ok(qs) => qs,
        Err(code) => return code,
    };
    let ps: Vec<f64> = if param_len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(params, param_len) }.to_vec()
    };
    if ps.iter().any(|p| !p.is_finite()) {
        return CqlibError::InvalidParam as i32;
    }
    wrapper
        .inner
        .apply_standard_gate_noise(gate, &qs, &ps)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

/// Apply an arbitrary unitary gate given as a `matrix_dim x matrix_dim`
/// row-major array of `Complex64` values. `matrix_dim` must equal
/// `2^qubit_len`; no gate noise is applied for generic unitaries.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_apply_unitary_gate(
    ptr: *mut CDensityMatrixNoise,
    qubits: *const u32,
    qubit_len: usize,
    matrix: *const Complex64,
    matrix_dim: usize,
) -> i32 {
    if ptr.is_null() || matrix.is_null() || qubits.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if qubit_len == 0 || matrix_dim == 0 {
        return CqlibError::InvalidParam as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let qs: Result<Vec<usize>, i32> = unsafe { std::slice::from_raw_parts(qubits, qubit_len) }
        .iter()
        .map(|&q| check_qubit(num_qubits_of(wrapper), q))
        .collect();
    let qs = match qs {
        Ok(qs) => qs,
        Err(code) => return code,
    };
    let expected_len = matrix_dim * matrix_dim;
    let flat = unsafe { std::slice::from_raw_parts(matrix, expected_len) };
    let mat = match Array2::from_shape_vec((matrix_dim, matrix_dim), flat.to_vec()) {
        Ok(m) => m,
        Err(_) => return CqlibError::InvalidParam as i32,
    };
    wrapper
        .inner
        .apply_unitary_gate(&qs, &mat)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

/// Return the number of basis-state probabilities (2^N), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_probabilities_len(ptr: *const CDensityMatrixNoise) -> usize {
    if ptr.is_null() {
        return 0;
    }
    1usize << num_qubits_of(unsafe { &*ptr })
}

/// Copy the ideal probability distribution (diagonal of the density matrix,
/// without readout error) into `buffer`. Returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_probabilities(
    ptr: *const CDensityMatrixNoise,
    buffer: *mut f64,
    len: usize,
) -> i32 {
    if ptr.is_null() || buffer.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    let probs = wrapper.inner.probabilities();
    if probs.len() != len {
        return CqlibError::InvalidParam as i32;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(probs.as_ptr(), buffer, len);
    }
    0
}

/// Return the number of basis-state probabilities (2^N), or 0 for NULL.
///
/// The buffer size is independent of the measured-qubit selection passed to
/// `density_matrix_noise_probabilities_with_readout`.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_probabilities_with_readout_len(
    ptr: *const CDensityMatrixNoise,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    1usize << num_qubits_of(unsafe { &*ptr })
}

/// Copy the probability distribution including readout errors configured in
/// the noise model for `qubits` into `buffer`. Returns 0 on success.
///
/// `qubits` may be NULL when `qubit_len` is 0, in which case no readout
/// error is applied and the result matches `density_matrix_noise_probabilities`.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_probabilities_with_readout(
    ptr: *const CDensityMatrixNoise,
    buffer: *mut f64,
    len: usize,
    qubits: *const u32,
    qubit_len: usize,
) -> i32 {
    if ptr.is_null() || buffer.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if qubit_len > 0 && qubits.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    let qs: Result<Vec<usize>, i32> = if qubit_len == 0 {
        Ok(Vec::new())
    } else {
        unsafe { std::slice::from_raw_parts(qubits, qubit_len) }
            .iter()
            .map(|&q| check_qubit(num_qubits_of(wrapper), q))
            .collect()
    };
    let qs = match qs {
        Ok(qs) => qs,
        Err(code) => return code,
    };
    match wrapper.inner.probabilities_with_readout(&qs) {
        Ok(probs) => {
            if probs.len() != len {
                return CqlibError::InvalidParam as i32;
            }
            unsafe {
                std::ptr::copy_nonoverlapping(probs.as_ptr(), buffer, len);
            }
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Compute the expectation value `<H> = Tr(H*rho)` of an observable.
/// Writes the value to `out` and returns 0 on success. Readout noise is
/// not included (it does not affect the quantum state).
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_expectation(
    ptr: *const CDensityMatrixNoise,
    observable: *const CHamiltonian,
    out: *mut f64,
) -> i32 {
    if ptr.is_null() || observable.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    let h = unsafe { &(*observable).inner };
    match wrapper.inner.expectation(h as &dyn Observable) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Measure `qubit` in the Z basis, collapsing the state.
/// Returns 0 for outcome |0>, 1 for outcome |1>, or a negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_measure(ptr: *mut CDensityMatrixNoise, qubit: u32) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let q = match check_qubit(num_qubits_of(wrapper), qubit) {
        Ok(q) => q,
        Err(code) => return code,
    };
    match wrapper.inner.measure(q) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(err) => qis_err_code(&err),
    }
}

/// Measure all qubits and return the outcome as a heap-allocated big-endian
/// bitstring (MSB = qubit N-1, LSB = qubit 0). Caller must free with
/// `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_measure_all(ptr: *mut CDensityMatrixNoise) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &mut *ptr };
    let n = num_qubits_of(wrapper);
    let outcome = wrapper.inner.measure_all();
    let bitstring = outcome.to_bitstring(n);
    match CString::new(bitstring) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Reset `qubit` to |0>. Returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_reset(ptr: *mut CDensityMatrixNoise, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrixNoise::reset)
}

/// Sample `shots` independent measurement outcomes in parallel.
/// Returns a heap-allocated `COutcomeList*` (caller must free with
/// `outcome_list_free`), or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_noise_sample_shots(
    ptr: *const CDensityMatrixNoise,
    shots: usize,
) -> *mut COutcomeList {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &*ptr };
    let n = num_qubits_of(wrapper);
    let outcomes = wrapper.inner.sample_shots(shots);
    let items = outcomes.into_iter().map(|o| o.to_bitstring(n)).collect();
    Box::into_raw(Box::new(COutcomeList { items }))
}
