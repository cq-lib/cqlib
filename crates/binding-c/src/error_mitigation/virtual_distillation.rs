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

//! C ABI for `VirtualDistillation`.

use crate::circuit::CCircuit;
use crate::error_mitigation::CVirtualDistillation;
use cqlib_core::error_mitigation::VirtualDistillation;

/// Creates a virtual distillation helper over `circuit` with `copies`
/// replicated registers. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn virtual_distillation_new(
    circuit: *const CCircuit,
    copies: usize,
) -> *mut CVirtualDistillation {
    if circuit.is_null() || copies == 0 {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner }.clone();
    match VirtualDistillation::new(circuit, copies) {
        Ok(inner) => Box::into_raw(Box::new(CVirtualDistillation { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a virtual distillation helper. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn virtual_distillation_free(ptr: *mut CVirtualDistillation) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Builds the copy-swap circuit used by the virtual distillation protocol.
///
/// Returns an owned `CCircuit*` (free with `circuit_free`), or NULL
/// on error.
#[unsafe(no_mangle)]
pub extern "C" fn virtual_distillation_build_circuit(
    ptr: *const CVirtualDistillation,
) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let vd = unsafe { &(*ptr).inner };
    match vd.build_copy_swap_circuit() {
        Ok(circuit) => Box::into_raw(Box::new(CCircuit { inner: circuit })),
        Err(_) => std::ptr::null_mut(),
    }
}
