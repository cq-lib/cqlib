// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Pre-layout elimination of logical wire permutations.
//!
//! A logical SWAP does not need to become a physical operation when layout is
//! still free to choose where logical outputs reside. This transform removes
//! eligible SWAP operations, redirects every later operation to the wire that
//! carries the corresponding logical state, and returns an explicit output
//! permutation for composition with routing metadata.
//!
//! The direction of [`VirtualPermutation`] is part of the API contract:
//!
//! ```text
//! original logical output -> rewritten circuit output wire
//! ```
//!
//! If routing later reports a layout from rewritten outputs to physical
//! qubits, [`VirtualPermutation::compose_final_layout`] produces the complete
//! original-output-to-physical mapping.

use crate::circuit::{Circuit, Instruction, Operation, Qubit, StandardGate, ValueOperation};
use crate::compile::CompilerError;
use crate::compile::transform::CircuitAnalysis;
use crate::compile::transform::rebuild::CircuitRebuildContext;
use crate::device::{Layout, LogicalQubit, PhysicalQubit};
use std::collections::BTreeMap;

/// Output-wire permutation accumulated while logical SWAPs are removed.
///
/// Each entry maps an output qubit in the original circuit to the output wire
/// carrying that state in the rewritten circuit. Inputs remain identity-mapped;
/// this permutation affects output interpretation only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VirtualPermutation {
    original_output_to_rewritten_output: BTreeMap<LogicalQubit, LogicalQubit>,
}

impl VirtualPermutation {
    /// Creates the identity output permutation for `qubits`.
    pub fn identity(qubits: impl IntoIterator<Item = Qubit>) -> Self {
        Self {
            original_output_to_rewritten_output: qubits
                .into_iter()
                .map(|qubit| {
                    let logical = LogicalQubit::from_qubit(qubit);
                    (logical, logical)
                })
                .collect(),
        }
    }

    /// Returns the rewritten output wire carrying `original_output`.
    pub fn rewritten_output(&self, original_output: LogicalQubit) -> Option<LogicalQubit> {
        self.original_output_to_rewritten_output
            .get(&original_output)
            .copied()
    }

    /// Returns the complete original-output-to-rewritten-output mapping.
    pub fn original_output_to_rewritten_output(&self) -> &BTreeMap<LogicalQubit, LogicalQubit> {
        &self.original_output_to_rewritten_output
    }

    /// Returns whether this output permutation is the identity.
    pub fn is_identity(&self) -> bool {
        self.original_output_to_rewritten_output
            .iter()
            .all(|(original, rewritten)| original == rewritten)
    }

    /// Composes this output permutation with a routed final layout.
    ///
    /// `rewritten_final_layout` maps rewritten circuit outputs to physical
    /// qubits. The returned layout maps original circuit outputs to those same
    /// physical qubits:
    ///
    /// ```text
    /// final(original) = rewritten_final_layout(permutation(original))
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`CompilerError::InvalidInput`] when the supplied layout does
    /// not map every rewritten output represented by this permutation.
    pub fn compose_final_layout(
        &self,
        rewritten_final_layout: &Layout,
    ) -> Result<Layout, CompilerError> {
        let mut mapping = BTreeMap::new();
        for (original, rewritten) in &self.original_output_to_rewritten_output {
            let physical = rewritten_final_layout
                .get_physical(*rewritten)
                .ok_or_else(|| {
                    CompilerError::InvalidInput(format!(
                        "final layout does not map rewritten output {rewritten} required by virtual permutation"
                    ))
                })?;
            mapping.insert(*original, physical);
        }

        Layout::new(
            self.original_output_to_rewritten_output
                .keys()
                .copied()
                .collect(),
            rewritten_final_layout
                .physical_qubits()
                .collect::<Vec<PhysicalQubit>>(),
            Some(mapping),
        )
        .map_err(|error| {
            CompilerError::InvariantViolation(format!(
                "failed to compose virtual permutation with final layout: {error}"
            ))
        })
    }

    fn swap_outputs(
        &mut self,
        left: LogicalQubit,
        right: LogicalQubit,
    ) -> Result<(), CompilerError> {
        let left_output = self.rewritten_output(left).ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "virtual permutation does not contain logical qubit {left}"
            ))
        })?;
        let right_output = self.rewritten_output(right).ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "virtual permutation does not contain logical qubit {right}"
            ))
        })?;
        self.original_output_to_rewritten_output
            .insert(left, right_output);
        self.original_output_to_rewritten_output
            .insert(right, left_output);
        Ok(())
    }
}

/// Coarse disposition of a virtual-permutation elimination attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualPermutationElisionStatus {
    /// No eligible logical permutation was present.
    Unchanged,
    /// At least one logical SWAP was removed.
    Changed,
    /// Structured control flow made a single static output permutation unsafe.
    SkippedControlFlow,
}

/// Result of [`elide_virtual_permutations`].
#[derive(Debug, Clone, PartialEq)]
pub struct VirtualPermutationElisionResult {
    changed_circuit: Option<Circuit>,
    virtual_permutation: VirtualPermutation,
    elided_swap_count: usize,
    status: VirtualPermutationElisionStatus,
}

impl VirtualPermutationElisionResult {
    /// Returns the rewritten circuit when the transform changed the input.
    pub fn changed_circuit(&self) -> Option<&Circuit> {
        self.changed_circuit.as_ref()
    }

    /// Consumes the result and returns the rewritten circuit when changed.
    pub fn into_changed_circuit(self) -> Option<Circuit> {
        self.changed_circuit
    }

    /// Returns the original-output-to-rewritten-output permutation.
    pub fn virtual_permutation(&self) -> &VirtualPermutation {
        &self.virtual_permutation
    }

    /// Returns the number of SWAP operations represented by the permutation.
    pub fn elided_swap_count(&self) -> usize {
        self.elided_swap_count
    }

    /// Returns the transform disposition.
    pub fn status(&self) -> VirtualPermutationElisionStatus {
        self.status
    }
}

/// Removes eligible top-level logical SWAPs and returns their output mapping.
///
/// An eligible SWAP is an unlabeled, parameter-free standard SWAP. Labeled
/// SWAPs are preserved because removing them would discard operation metadata.
/// All other operations, including measurements, resets, barriers, symbolic
/// gates, and runtime classical data operations, are preserved with their
/// qubits redirected through the current permutation. Classical handles,
/// measurement bit order, operation labels, and global phase are preserved by
/// the shared rebuild infrastructure.
///
/// Circuits containing structured classical control are conservatively left
/// unchanged. Branches and loops can have path-dependent output permutations,
/// which cannot be represented by this result's single static mapping.
pub fn elide_virtual_permutations(
    circuit: &Circuit,
) -> Result<VirtualPermutationElisionResult, CompilerError> {
    let mut virtual_permutation = VirtualPermutation::identity(circuit.qubits());
    if CircuitAnalysis::analyze(circuit).has_classical_control {
        return Ok(VirtualPermutationElisionResult {
            changed_circuit: None,
            virtual_permutation,
            elided_swap_count: 0,
            status: VirtualPermutationElisionStatus::SkippedControlFlow,
        });
    }

    if !circuit.operations().iter().any(is_elidable_swap) {
        return Ok(VirtualPermutationElisionResult {
            changed_circuit: None,
            virtual_permutation,
            elided_swap_count: 0,
            status: VirtualPermutationElisionStatus::Unchanged,
        });
    }

    let rebuild = CircuitRebuildContext::new(circuit);
    let mut rewritten = Vec::<ValueOperation>::with_capacity(circuit.operations().len());
    let mut elided_swap_count = 0usize;

    for operation in circuit.operations() {
        if is_elidable_swap(operation) {
            let [left, right] = operation.qubits.as_slice() else {
                return Err(CompilerError::InvariantViolation(
                    "standard SWAP must have exactly two qubits".to_string(),
                ));
            };
            virtual_permutation.swap_outputs(
                LogicalQubit::from_qubit(*left),
                LogicalQubit::from_qubit(*right),
            )?;
            elided_swap_count += 1;
            continue;
        }

        let mut preserved =
            rebuild.remap_preserved_operation(circuit, operation, rebuild.root_classical())?;
        for qubit in &mut preserved.qubits {
            *qubit = virtual_permutation
                .rewritten_output(LogicalQubit::from_qubit(*qubit))
                .ok_or_else(|| {
                    CompilerError::InvariantViolation(format!(
                        "operation references qubit {qubit} absent from virtual permutation"
                    ))
                })?
                .qubit();
        }
        rewritten.push(preserved);
    }

    let rewritten = rebuild.finish(circuit.qubits(), rewritten, circuit.global_phase())?;
    Ok(VirtualPermutationElisionResult {
        changed_circuit: Some(rewritten),
        virtual_permutation,
        elided_swap_count,
        status: VirtualPermutationElisionStatus::Changed,
    })
}

fn is_elidable_swap(operation: &Operation) -> bool {
    matches!(
        operation.instruction,
        Instruction::Standard(StandardGate::SWAP)
    ) && operation.qubits.len() == 2
        && operation.params.is_empty()
        && operation.label.is_none()
}

#[cfg(test)]
#[path = "./virtual_permutation_test.rs"]
mod virtual_permutation_test;
