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

//! C ABI for the device module (Device, Layout, ExecutionResult, NoiseModel).

#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod layout;
pub mod noise;
pub mod result;
pub mod topology;

use cqlib_core::circuit::gate::StandardGate;
use cqlib_core::device::{Device, ExecutionResult, Layout, NoiseModel};

/// Opaque handle around [`Device`] for C ABI.
pub struct CDevice {
    pub inner: Device,
}

/// Opaque handle around [`cqlib_core::device::Topology`] for C ABI.
pub struct CTopology {
    pub inner: cqlib_core::device::Topology,
}

/// Opaque handle around [`Layout`] for C ABI.
pub struct CLayout {
    pub inner: Layout,
}

/// Opaque handle around [`ExecutionResult`] for C ABI.
pub struct CExecutionResult {
    pub inner: ExecutionResult,
}

/// Opaque handle around [`NoiseModel`] for C ABI.
pub struct CNoiseModel {
    pub inner: NoiseModel,
}

/// Key/value list of measurement outcomes (bitstring -> weight).
///
/// Used by `execution_result_counts` and `execution_result_probabilities`;
/// `values` holds counts for the former and probabilities for the latter.
pub struct CCountsList {
    pub keys: Vec<String>,
    pub values: Vec<f64>,
}

/// Resolves a standard-gate name (e.g. "H", "CX", "RZZ") to a `StandardGate`.
pub(crate) fn standard_gate_from_name(name: &str) -> Option<StandardGate> {
    use StandardGate as G;
    Some(match name {
        "I" => G::I,
        "H" => G::H,
        "RX" => G::RX,
        "RXX" => G::RXX,
        "RXY" => G::RXY,
        "RY" => G::RY,
        "RYY" => G::RYY,
        "RZ" => G::RZ,
        "RZX" => G::RZX,
        "RZZ" => G::RZZ,
        "S" => G::S,
        "SDG" => G::SDG,
        "SWAP" => G::SWAP,
        "T" => G::T,
        "TDG" => G::TDG,
        "U" => G::U,
        "X" => G::X,
        "XY" => G::XY,
        "X2P" => G::X2P,
        "X2M" => G::X2M,
        "XY2P" => G::XY2P,
        "XY2M" => G::XY2M,
        "Y" => G::Y,
        "Y2P" => G::Y2P,
        "Y2M" => G::Y2M,
        "Z" => G::Z,
        "Phase" => G::Phase,
        "GPhase" => G::GPhase,
        "CX" => G::CX,
        "CCX" => G::CCX,
        "CY" => G::CY,
        "CZ" => G::CZ,
        "CRX" => G::CRX,
        "CRY" => G::CRY,
        "CRZ" => G::CRZ,
        "fSim" | "FSIM" => G::FSIM,
        _ => return None,
    })
}

pub use layout::*;
pub use noise::*;
pub use result::*;
pub use topology::*;
