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

//! Python exception types for compiler bindings.

use crate::circuit::error::{CircuitError, CqlibError};
use cqlib_core::compile::CompilerError as CoreCompilerError;
use pyo3::create_exception;
use pyo3::prelude::*;

create_exception!(cqlib.compile, CompilerError, CqlibError);
create_exception!(cqlib.compile, CompilerConfigError, CompilerError);
create_exception!(cqlib.compile, CompilerTransformError, CompilerError);
create_exception!(cqlib.compile, CompilerInternalError, CompilerError);

/// Maps compiler failures to stable, semantically distinct Python exceptions.
pub(crate) fn compiler_error_to_py_err(error: CoreCompilerError) -> PyErr {
    let message = error.to_string();
    match error {
        CoreCompilerError::Circuit(_) => CircuitError::new_err(message),
        CoreCompilerError::InvalidInput(_) => CompilerConfigError::new_err(message),
        CoreCompilerError::TransformFailed { .. }
        | CoreCompilerError::DeviceLoweringFailed(_)
        | CoreCompilerError::SabreRoutingFailed(_)
        | CoreCompilerError::DeviceValidationFailed(_) => CompilerTransformError::new_err(message),
        CoreCompilerError::InvariantViolation(_) => CompilerInternalError::new_err(message),
    }
}

/// Registers the exception hierarchy on `_native.compile`.
pub(super) fn register_errors(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("CompilerError", module.py().get_type::<CompilerError>())?;
    module.add(
        "CompilerConfigError",
        module.py().get_type::<CompilerConfigError>(),
    )?;
    module.add(
        "CompilerTransformError",
        module.py().get_type::<CompilerTransformError>(),
    )?;
    module.add(
        "CompilerInternalError",
        module.py().get_type::<CompilerInternalError>(),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cqlib_core::circuit::{CircuitError as CoreCircuitError, Instruction, StandardGate};
    use cqlib_core::compile::error::{DeviceLoweringFailure, SabreRoutingFailure};
    use cqlib_core::device::{DeviceValidationError, PhysicalQubit};

    #[test]
    fn maps_all_compiler_error_variants() {
        Python::initialize();
        Python::attach(|py| {
            let circuit = compiler_error_to_py_err(CoreCompilerError::Circuit(
                CoreCircuitError::QubitNotFound(7),
            ));
            assert!(circuit.is_instance_of::<CircuitError>(py));

            let config = compiler_error_to_py_err(CoreCompilerError::InvalidInput(
                "invalid configuration".to_owned(),
            ));
            assert!(config.is_instance_of::<CompilerConfigError>(py));

            let transform = compiler_error_to_py_err(CoreCompilerError::TransformFailed {
                name: "kak",
                reason: "did not converge".to_owned(),
            });
            assert!(transform.is_instance_of::<CompilerTransformError>(py));

            let device_lowering = compiler_error_to_py_err(
                CoreCompilerError::DeviceLoweringFailed(DeviceLoweringFailure {
                    instruction: Instruction::Standard(StandardGate::H),
                    qargs: vec![PhysicalQubit::new(0)],
                    attempted_candidates: Vec::new(),
                }),
            );
            assert!(device_lowering.is_instance_of::<CompilerTransformError>(py));

            let sabre = compiler_error_to_py_err(CoreCompilerError::SabreRoutingFailed(
                SabreRoutingFailure::MovementAssignmentInfeasible,
            ));
            assert!(sabre.is_instance_of::<CompilerTransformError>(py));

            let validation = compiler_error_to_py_err(CoreCompilerError::DeviceValidationFailed(
                DeviceValidationError::UnusablePhysicalQubit {
                    device: "test".to_owned(),
                    qubit: PhysicalQubit::new(0),
                },
            ));
            assert!(validation.is_instance_of::<CompilerTransformError>(py));

            let internal = compiler_error_to_py_err(CoreCompilerError::InvariantViolation(
                "broken contract".to_owned(),
            ));
            assert!(internal.is_instance_of::<CompilerInternalError>(py));
        });
    }
}
