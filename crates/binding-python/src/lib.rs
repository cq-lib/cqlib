// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

pub mod circuit;
pub mod compile;
pub mod device;
pub mod error_mitigation;
pub mod ir;
pub mod qis;
mod utils;
pub mod visualization;

use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
#[pyo3(name = "_native")]
fn binding_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    circuit::register_circuit_module(m)?;
    compile::register_compile_module(m)?;
    device::register_device_module(m)?;
    error_mitigation::register_error_mitigation_module(m)?;
    ir::register_ir_module(m)?;
    qis::register_qis_module(m)?;
    visualization::register_visualization_module(m)?;

    Ok(())
}
