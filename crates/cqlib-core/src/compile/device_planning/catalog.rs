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

use super::{DeviceGateState, DevicePlanningSession, NativePlanAvailability, NativePlanSummary};
use crate::compile::CompilerError;
use std::collections::HashMap;

/// Immutable exact-device planning results used by routing cost preparation.
///
/// Every requested root is retained. A missing map entry therefore means the
/// caller failed to prepare that state, rather than that planning proved it
/// unsupported.
#[derive(Debug, Clone)]
pub(crate) struct NativePlanCatalog {
    availability: HashMap<DeviceGateState, NativePlanAvailability>,
}

impl NativePlanCatalog {
    pub(crate) fn build_with_session(
        session: &DevicePlanningSession,
        roots: impl IntoIterator<Item = DeviceGateState>,
    ) -> Result<Self, CompilerError> {
        let mut roots = roots.into_iter().collect::<Vec<_>>();
        roots.sort();
        roots.dedup();
        let snapshot = session.prepare(roots.iter().cloned())?;
        let mut availability = HashMap::with_capacity(roots.len());
        for root in roots {
            let planned = snapshot.availability(&root).ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "device planning session did not retain requested root {root:?}"
                ))
            })?;
            availability.insert(root, planned);
        }
        Ok(Self { availability })
    }

    pub(crate) fn availability(&self, state: &DeviceGateState) -> Option<&NativePlanAvailability> {
        self.availability.get(state)
    }

    pub(crate) fn summary(&self, state: &DeviceGateState) -> Option<&NativePlanSummary> {
        match self.availability(state) {
            Some(NativePlanAvailability::Feasible(summary)) => Some(summary),
            Some(NativePlanAvailability::Unsupported(_)) | None => None,
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&DeviceGateState, &NativePlanSummary)> {
        self.availability
            .iter()
            .filter_map(|(state, availability)| {
                let NativePlanAvailability::Feasible(summary) = availability else {
                    return None;
                };
                Some((state, summary))
            })
    }

    pub(crate) fn iter_availability(
        &self,
    ) -> impl Iterator<Item = (&DeviceGateState, &NativePlanAvailability)> {
        self.availability.iter()
    }
}
