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

//! C ABI for the unified `ErrorMitigation` facade.

use crate::circuit::CCircuit;
use crate::error::CqlibError;
use crate::error_mitigation::{CErrorMitigation, CEstimatorFn};
use crate::qis::CHamiltonian;
use cqlib_core::error_mitigation::{
    ErrorMitigation, MitigationMethod, ProcessArgs, RunArgs, VirtualDistillationConfig, ZneConfig,
};

/// Mitigation method tags for `error_mitigation_new`.
///
/// | Value | Method              | Extra parameter            |
/// |-------|---------------------|----------------------------|
/// | 0     | ZNE                 | `fold_levels` array        |
/// | 1     | VirtualDistillation | `copies`                   |
pub const MITIGATION_ZNE: u8 = 0;
pub const MITIGATION_VIRTUAL_DISTILLATION: u8 = 1;

/// Extrapolation method tags for `error_mitigation_get_mitigated`.
///
/// | Value | Method              |
/// |-------|---------------------|
/// | 0     | Zne + Polynomial    |
/// | 1     | Zne + Exponential   |
/// | 2     | VirtualDistillation |
pub const PROCESS_ZNE_POLYNOMIAL: u8 = 0;
pub const PROCESS_ZNE_EXPONENTIAL: u8 = 1;
pub const PROCESS_VIRTUAL_DISTILLATION: u8 = 2;

/// Creates a mitigation pipeline over `circuit`.
///
/// - `method == MITIGATION_ZNE`: reads `num_levels` fold levels from
///   `fold_levels` (e.g. `[0, 1, 2]`).
/// - `method == MITIGATION_VIRTUAL_DISTILLATION`: uses `copies`.
///
/// Returns NULL on error or when the parameters do not match the method.
#[unsafe(no_mangle)]
pub extern "C" fn error_mitigation_new(
    circuit: *const CCircuit,
    method: u8,
    fold_levels: *const i32,
    num_levels: usize,
    copies: usize,
) -> *mut CErrorMitigation {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner }.clone();
    let mitigation_method = match method {
        MITIGATION_ZNE => {
            if num_levels == 0 || fold_levels.is_null() {
                return std::ptr::null_mut();
            }
            let levels = unsafe { std::slice::from_raw_parts(fold_levels, num_levels) }.to_vec();
            MitigationMethod::Zne(ZneConfig {
                fold_levels: levels,
            })
        }
        MITIGATION_VIRTUAL_DISTILLATION => {
            if copies == 0 {
                return std::ptr::null_mut();
            }
            MitigationMethod::VirtualDistillation(VirtualDistillationConfig { copies })
        }
        _ => return std::ptr::null_mut(),
    };
    match ErrorMitigation::new(circuit, mitigation_method) {
        Ok(inner) => Box::into_raw(Box::new(CErrorMitigation { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a mitigation pipeline. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn error_mitigation_free(ptr: *mut CErrorMitigation) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Executes the mitigation sampling stage.
///
/// `method` must match the method configured in `error_mitigation_new`
/// (`MITIGATION_ZNE` or `MITIGATION_VIRTUAL_DISTILLATION`).
///
/// - ZNE method: uses `shots_a` as the shot count (0 = unspecified);
///   `shots_b` is ignored.
/// - VirtualDistillation method: uses `shots_a` for the numerator circuit
///   and `shots_b` for the denominator circuit (both must be > 0).
///
/// `estimator` must not be NULL; it is invoked once per required circuit
/// execution. Returns 0 on success or a negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn error_mitigation_run(
    ptr: *mut CErrorMitigation,
    method: u8,
    hamiltonian: *const CHamiltonian,
    shots_a: usize,
    shots_b: usize,
    estimator: CEstimatorFn,
) -> i32 {
    if ptr.is_null() || hamiltonian.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let run_args = match method {
        MITIGATION_ZNE => RunArgs::Zne {
            gate_set: None,
            shots: if shots_a > 0 { Some(shots_a) } else { None },
        },
        MITIGATION_VIRTUAL_DISTILLATION => RunArgs::VirtualDistillation {
            shots_numerator: shots_a,
            shots_denominator: shots_b,
        },
        _ => return CqlibError::InvalidParam as i32,
    };
    let wrapper = unsafe { &mut *ptr };
    let hamiltonian = unsafe { &(*hamiltonian).inner };
    let estimator = crate::error_mitigation::wrap_estimator(estimator);
    match wrapper.inner.run(hamiltonian, run_args, &estimator) {
        Ok(()) => 0,
        Err(_) => CqlibError::SimulationError as i32,
    }
}

/// Produces the final mitigated estimate after `error_mitigation_run`.
///
/// `process_method` is one of the `PROCESS_*` tags. `degree` selects the
/// polynomial degree for ZNE (0 = automatic). Writes the mitigated
/// expectation to `out_expectation` and the variance to `out_variance`
/// (NaN when unavailable). Returns 0 on success or a negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn error_mitigation_get_mitigated(
    ptr: *mut CErrorMitigation,
    process_method: u8,
    degree: usize,
    out_expectation: *mut f64,
    out_variance: *mut f64,
) -> i32 {
    if ptr.is_null() || out_expectation.is_null() || out_variance.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let process_args = match process_method {
        PROCESS_ZNE_POLYNOMIAL => ProcessArgs::Zne {
            method: cqlib_core::error_mitigation::ExtrapolateMethod::Polynomial,
            degree: if degree > 0 { Some(degree) } else { None },
        },
        PROCESS_ZNE_EXPONENTIAL => ProcessArgs::Zne {
            method: cqlib_core::error_mitigation::ExtrapolateMethod::Exponential,
            degree: if degree > 0 { Some(degree) } else { None },
        },
        PROCESS_VIRTUAL_DISTILLATION => ProcessArgs::VirtualDistillation,
        _ => return CqlibError::InvalidParam as i32,
    };
    match wrapper.inner.get_mitigated(process_args) {
        Ok(result) => {
            unsafe {
                *out_expectation = result.expectation;
                *out_variance = result.variance.unwrap_or(f64::NAN);
            }
            0
        }
        Err(_) => CqlibError::SimulationError as i32,
    }
}
