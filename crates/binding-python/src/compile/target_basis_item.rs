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

//! Shared Python-facing item type for every API that accepts a target basis.
//!
//! Python callers may pass either case-insensitive standard-gate names for
//! concise call sites (matching the string form printed by `__repr__`, so
//! `eval(repr(x))` round-trips) or fully constructed `Instruction` objects
//! when multi-controlled or custom gates are required.

use crate::circuit::PyInstruction;
use crate::circuit::gate::standard::standard_gate_from_name;
use crate::compile::error::CompilerConfigError;
use cqlib_core::circuit::Instruction;
use pyo3::prelude::*;

/// Python-facing target-basis item accepted by basis-taking constructors.
///
/// String names are resolved against the shared standard-gate table with
/// ASCII case-insensitive matching; unknown names are reported as
/// configuration errors. Multi-controlled gates have no string form and must
/// be passed as `Instruction` objects.
#[derive(FromPyObject)]
pub enum PyTargetBasisItem {
    Name(String),
    Instruction(PyInstruction),
}

impl PyTargetBasisItem {
    /// Converts the item into a core instruction.
    pub(crate) fn into_instruction(self) -> PyResult<Instruction> {
        match self {
            Self::Instruction(instruction) => Ok(instruction.inner),
            Self::Name(name) => standard_gate_from_name(&name)
                .map(Instruction::Standard)
                .ok_or_else(|| {
                    CompilerConfigError::new_err(format!(
                        "unknown standard gate in target basis: {name:?}"
                    ))
                }),
        }
    }
}
