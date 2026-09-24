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

//! C ABI for circuit visualization (text drawing, SVG figure, matrix export).

#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod circuit_render;
pub mod plot_render;
pub mod result_plot;
pub mod state_plot;
pub mod visual_ir;

pub use circuit_render::*;
pub use plot_render::*;
pub use result_plot::*;
pub use state_plot::*;
pub use visual_ir::*;
