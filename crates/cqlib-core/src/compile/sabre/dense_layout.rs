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

use crate::compile::CompilerError;
use crate::device::{Layout, LogicalQubit, PhysicalQubit};
use std::collections::{BTreeMap, HashMap};

/// Trial-local dense representation of a logical/physical mapping.
///
/// Public layouts deliberately support sparse user-defined identifiers. SABRE
/// resolves those identifiers once at the trial boundary and performs every
/// hot-path lookup and SWAP on contiguous indices.
#[derive(Debug, Clone)]
pub(crate) struct DenseRoutingLayout {
    logical_qubits: Vec<LogicalQubit>,
    logical_indices: HashMap<LogicalQubit, usize>,
    logical_to_physical: Vec<usize>,
    physical_to_logical: Vec<Option<usize>>,
}

impl DenseRoutingLayout {
    pub(crate) fn from_layout(
        layout: &Layout,
        physical_qubits: &[PhysicalQubit],
    ) -> Result<Self, CompilerError> {
        let logical_qubits = layout.logical_qubits().collect::<Vec<_>>();
        let logical_indices = logical_qubits
            .iter()
            .copied()
            .enumerate()
            .map(|(index, logical)| (logical, index))
            .collect::<HashMap<_, _>>();
        let physical_indices = physical_qubits
            .iter()
            .copied()
            .enumerate()
            .map(|(index, physical)| (physical, index))
            .collect::<HashMap<_, _>>();
        let mut logical_to_physical = vec![usize::MAX; logical_qubits.len()];
        let mut physical_to_logical = vec![None; physical_qubits.len()];
        for (logical_index, logical) in logical_qubits.iter().copied().enumerate() {
            let physical = layout.get_physical(logical).ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "sabre layout does not map logical qubit {logical}"
                ))
            })?;
            let physical_index = *physical_indices.get(&physical).ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "sabre layout maps logical qubit {logical} outside the routing target"
                ))
            })?;
            if physical_to_logical[physical_index]
                .replace(logical_index)
                .is_some()
            {
                return Err(CompilerError::InvariantViolation(format!(
                    "sabre layout maps multiple logical qubits to physical qubit {physical}"
                )));
            }
            logical_to_physical[logical_index] = physical_index;
        }
        Ok(Self {
            logical_qubits,
            logical_indices,
            logical_to_physical,
            physical_to_logical,
        })
    }

    #[inline]
    pub(crate) fn physical_index(&self, logical: LogicalQubit) -> Result<usize, CompilerError> {
        let logical_index = self.logical_indices.get(&logical).copied().ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "sabre dense layout does not contain logical qubit {logical}"
            ))
        })?;
        Ok(self.logical_to_physical[logical_index])
    }

    #[inline]
    pub(crate) fn swap_physical_indices(&mut self, left: usize, right: usize) {
        if left == right {
            return;
        }
        let left_logical = self.physical_to_logical[left];
        let right_logical = self.physical_to_logical[right];
        self.physical_to_logical.swap(left, right);
        if let Some(logical) = left_logical {
            self.logical_to_physical[logical] = right;
        }
        if let Some(logical) = right_logical {
            self.logical_to_physical[logical] = left;
        }
    }

    pub(crate) fn to_layout(
        &self,
        physical_qubits: &[PhysicalQubit],
    ) -> Result<Layout, CompilerError> {
        let mapping = self
            .logical_qubits
            .iter()
            .copied()
            .enumerate()
            .map(|(logical_index, logical)| {
                (
                    logical,
                    physical_qubits[self.logical_to_physical[logical_index]],
                )
            })
            .collect::<BTreeMap<_, _>>();
        Layout::new(
            self.logical_qubits.clone(),
            physical_qubits.to_vec(),
            Some(mapping),
        )
        .map_err(|error| {
            CompilerError::InvariantViolation(format!(
                "failed to rebuild SABRE layout from dense routing state: {error}"
            ))
        })
    }

    pub(crate) fn signature(&self) -> &[Option<usize>] {
        &self.physical_to_logical
    }
}
