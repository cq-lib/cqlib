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

//! Trivial initial layout.
//!
//! This algorithm is the deterministic baseline for the layout stack: logical
//! qubits are assigned to usable physical qubits in their existing order. It is
//! intentionally simple, but it still runs the shared objective and diagnostics
//! so callers can compare it with optimized algorithms.

use super::{
    CircuitLayoutAnalysis, LayoutDiagnostics, LayoutObjective, LayoutResult, PhysicalLayoutGraph,
    analyze_circuit_for_layout, is_perfect_layout,
};
use crate::circuit::Circuit;
use crate::compile::CompilerError;
use crate::device::{Device, Layout};

/// Maps logical qubits to usable physical qubits in their existing order.
///
/// This layout method deliberately does not optimize topology, direction, or
/// calibration data. It is useful as a deterministic baseline and for circuits
/// whose logical order already matches the target device.
///
/// The returned diagnostics report whether the baseline is already perfect for
/// the circuit interaction graph. A non-perfect result is still valid input for
/// a later routing pass.
///
/// # Errors
///
/// Returns [`CompilerError::InvalidInput`] if the circuit has more logical
/// qubits than the device has usable physical qubits, or if scoring rejects the
/// resulting layout.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::circuit::{Circuit, Qubit};
/// use cqlib_core::compile::transform::{LayoutObjective, trivial_layout};
/// use cqlib_core::device::{Device, LogicalQubit, PhysicalQubit};
///
/// let mut circuit = Circuit::new(2);
/// circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
/// let device = Device::line("line-2", 2).unwrap();
///
/// let result = trivial_layout(&circuit, &device, &LayoutObjective::topology_only()).unwrap();
/// assert_eq!(
///     result.layout.get_physical(LogicalQubit::new(0)),
///     Some(PhysicalQubit::new(0)),
/// );
/// assert!(result.diagnostics.is_perfect);
/// ```
pub fn trivial_layout(
    circuit: &Circuit,
    device: &Device,
    objective: &LayoutObjective,
) -> Result<LayoutResult, CompilerError> {
    let analysis = analyze_circuit_for_layout(circuit)?;
    let physical = PhysicalLayoutGraph::from_device(device)?;
    trivial_layout_prepared(&analysis, &physical, objective)
}

/// Maps logical qubits to a prepared physical graph in their existing order.
///
/// This lower-level entry point is useful when a workflow has already prepared
/// circuit analysis and physical graph data for one or more layout algorithms.
pub fn trivial_layout_prepared(
    analysis: &CircuitLayoutAnalysis,
    physical: &PhysicalLayoutGraph,
    objective: &LayoutObjective,
) -> Result<LayoutResult, CompilerError> {
    if analysis.logical_qubits.len() > physical.physical_qubits().len() {
        return Err(CompilerError::InvalidInput(format!(
            "trivial layout requires at least as many usable physical qubits as logical qubits; got {} logical qubits and {} usable physical qubits",
            analysis.logical_qubits.len(),
            physical.physical_qubits().len()
        )));
    }

    // Layout::new with no explicit mapping follows the order of logical and
    // physical qubit lists, which is exactly the trivial-layout contract.
    let layout = Layout::new(
        analysis.logical_qubits.clone(),
        physical.physical_qubits().to_vec(),
        None,
    )
    .map_err(|error| {
        CompilerError::InvariantViolation(format!(
            "trivial layout failed to construct a valid layout: {error}"
        ))
    })?;

    let score = objective.score_layout(analysis, physical, &layout)?;

    let diagnostics = LayoutDiagnostics {
        is_perfect: is_perfect_layout(analysis, physical, &layout),
        candidates_evaluated: 1,
        used_fidelity: score.used_fidelity,
        notes: Vec::new(),
    };

    Ok(LayoutResult {
        layout,
        score: Some(score),
        diagnostics,
    })
}
