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

//! C ABI for the compile module.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod commutation;
pub mod decompose;
pub mod knowledge;
pub mod layout;
pub mod pipeline;
pub mod resource;
pub mod routing;
pub mod sabre;
pub mod transform;
pub mod workflow;

use cqlib_core::compile::CompileResult;

/// Opaque handle around a [`CompileResult`].
pub struct CCompileResult {
    pub inner: CompileResult,
}

/// Compile mode tag exposed to C.
/// | Value | Mode     |
/// |-------|----------|
/// | 0     | Normal   |
/// | 1     | Enhanced |
pub const COMPILE_MODE_NORMAL: u8 = 0;
pub const COMPILE_MODE_ENHANCED: u8 = 1;

/// Target tag exposed to C.
/// | Value | Target        |
/// |-------|---------------|
/// | 0     | Logical       |
/// | 1     | Basis         |
/// | 2     | Device         |
/// | 3     | TopologyBasis |
pub const COMPILE_TARGET_LOGICAL: u8 = 0;
pub const COMPILE_TARGET_BASIS: u8 = 1;
pub const COMPILE_TARGET_DEVICE: u8 = 2;
pub const COMPILE_TARGET_TOPOLOGY_BASIS: u8 = 3;

/// C-side compile configuration.
///
/// For `Logical` targets, only `mode` is consulted.
/// For `Basis` targets, the Rust side falls back to logical compilation
/// (the binding does not parse `Instruction` from C; users should construct
/// such targets from Rust or build them out-of-band). All fields are kept
/// so the struct can be extended without breaking ABI.
#[repr(C)]
pub struct CompileConfigC {
    /// One of `COMPILE_MODE_*`.
    pub mode: u8,
    /// One of `COMPILE_TARGET_*`.
    pub target: u8,
    /// Reserved for future use (resource policy flags).
    pub allow_dirty_ancilla: u8,
    pub allow_clean_ancilla: u8,
    /// Reserved padding to keep the struct layout stable.
    pub _reserved: [u8; 4],
}

pub use commutation::*;
pub use decompose::*;
pub use knowledge::*;
pub use layout::*;
pub use pipeline::*;
pub use resource::*;
pub use routing::*;
pub use sabre::*;
pub use transform::*;
pub use workflow::*;
