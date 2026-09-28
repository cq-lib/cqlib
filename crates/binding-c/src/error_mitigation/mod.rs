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

//! C ABI for the error mitigation module.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod unified;
pub mod virtual_distillation;
pub mod zne;

use crate::circuit::CCircuit;
use crate::error::CqlibError;
use crate::qis::CHamiltonian;
use cqlib_core::circuit::Circuit;
use cqlib_core::error_mitigation::{ErrorMitigation, ErrorMitigationError};
use cqlib_core::qis::Hamiltonian;

/// Opaque handle around [`ErrorMitigation`] for C ABI.
pub struct CErrorMitigation {
    pub inner: ErrorMitigation,
}

/// Opaque handle around [`cqlib_core::error_mitigation::ZNEMitigation`] for C ABI
/// (defined in `zne`).
pub use zne::CZneMitigation;

/// Opaque handle around [`cqlib_core::error_mitigation::VirtualDistillation`] for
/// C ABI.
pub struct CVirtualDistillation {
    pub inner: cqlib_core::error_mitigation::VirtualDistillation,
}

/// Owned list of folded circuits returned by `zne_mitigation_fold_circuit`.
pub struct CCircuitList {
    pub items: Vec<Circuit>,
}

/// C estimator callback invoked by `error_mitigation_run`.
///
/// The callback receives the folded circuit, the observable, and the shot
/// count (0 means "not specified"). It must write the estimated expectation
/// value (and optionally its variance, NaN if unknown) through the out
/// pointers. All pointers are valid only for the duration of the call;
/// the Hamiltonian handle must not be freed by the callback.
pub type CEstimatorFn = extern "C" fn(
    circuit: *const CCircuit,
    hamiltonian: *const CHamiltonian,
    shots: usize,
    expectation: *mut f64,
    variance: *mut f64,
);

/// Wraps a C callback into the Rust `Estimator` closure.
///
/// The C side receives borrowed handles that are only valid during the
/// callback; the Hamiltonian pointer may be null.
pub(crate) fn wrap_estimator(
    callback: CEstimatorFn,
) -> impl Fn(&Circuit, Option<&Hamiltonian>, Option<usize>) -> (f64, f64) {
    move |circuit: &Circuit, hamiltonian: Option<&Hamiltonian>, shots: Option<usize>| {
        let circuit_wrapper = CCircuit {
            inner: circuit.clone(),
        };
        let hamiltonian_wrapper = hamiltonian.map(|h| CHamiltonian { inner: h.clone() });
        let mut expectation = f64::NAN;
        let mut variance = f64::NAN;
        callback(
            &circuit_wrapper,
            hamiltonian_wrapper
                .as_ref()
                .map(|w| w as *const CHamiltonian)
                .unwrap_or(std::ptr::null()),
            shots.unwrap_or(0),
            &mut expectation,
            &mut variance,
        );
        (expectation, variance)
    }
}

/// Maps an `ErrorMitigationError` to a C error code.
///
/// Invalid parameter shapes (copies, fold level, polynomial degree, result
/// length, sign) map to `InvalidParam` (-8); circuit-level failures map to
/// `CircuitError` (-3); everything else follows the unified-facade precedent
/// and maps to `SimulationError` (-7).
pub(crate) fn em_error_code(error: &ErrorMitigationError) -> i32 {
    match error {
        ErrorMitigationError::Circuit(_) => CqlibError::CircuitError as i32,
        ErrorMitigationError::InvalidCopies(_)
        | ErrorMitigationError::InvalidFoldLevel(_)
        | ErrorMitigationError::InvalidPolynomialDegree { .. }
        | ErrorMitigationError::HamiltonianQubitCountMismatch { .. }
        | ErrorMitigationError::EmptyNoisyResults
        | ErrorMitigationError::NoisyResultsLengthMismatch { .. }
        | ErrorMitigationError::NonPositiveNoisyResults => CqlibError::InvalidParam as i32,
        _ => CqlibError::SimulationError as i32,
    }
}

pub use unified::*;
pub use virtual_distillation::*;
pub use zne::*;
