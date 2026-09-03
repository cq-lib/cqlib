// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use crate::circuit::Qubit;
use std::collections::BTreeSet;

/// Source from which a compiler-visible logical qubit entered the resource pool.
///
/// Origins determine which usage contracts a qubit may satisfy. They are
/// bookkeeping facts, not conclusions inferred by quantum-state analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QubitOrigin {
    /// A qubit present in the circuit before compiler resource management began.
    Input,
    /// A clean logical ancillary qubit created by the compiler before layout.
    CompilerAllocated,
}

/// State-restoration contract required by an ancillary-resource consumer.
///
/// A consumer must satisfy the selected contract before releasing its lease.
/// The resource manager records this requirement but does not prove it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AncillaRequirement {
    /// The qubit must enter and leave the consumer in `|0>`.
    CleanZero,
    /// The qubit may enter in an unknown state and must be restored exactly.
    Dirty,
}

impl AncillaRequirement {
    pub(super) const fn diagnostic_name(self) -> &'static str {
        match self {
            Self::CleanZero => "clean-zero",
            Self::Dirty => "dirty",
        }
    }
}

/// Request for a temporary ancillary-qubit lease.
///
/// A planner constructs a request for each algorithm candidate and passes it to
/// [`super::ResourceManager::preview`]. The request does not reserve resources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRequest {
    /// State-restoration contract required by the consuming algorithm.
    pub requirement: AncillaRequirement,
    /// Number of ancillary qubits required by the algorithm.
    pub count: usize,
    /// Qubits that the current algorithm must not consume as ancillary resources.
    ///
    /// This normally includes the algorithm's data, control, and target qubits.
    /// Every excluded qubit must already be registered with the manager.
    pub excluded: BTreeSet<Qubit>,
}

/// Side-effect-free preview of resources that can satisfy a request.
///
/// A plan is produced by [`super::ResourceManager::preview`] and may be
/// inspected while comparing algorithm candidates. It does not reserve qubits
/// or mutate the circuit. A caller must pass the selected plan to
/// [`super::ResourceManager::commit`] before using its qubits.
///
/// Plans are manager-specific snapshots. Any successful resource-pool mutation
/// makes older plans stale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourcePlan {
    manager_id: u64,
    requirement: AncillaRequirement,
    qubits: Vec<Qubit>,
    new_qubits: Vec<Qubit>,
    revision: u64,
}

impl ResourcePlan {
    pub(super) const fn new(
        manager_id: u64,
        requirement: AncillaRequirement,
        qubits: Vec<Qubit>,
        new_qubits: Vec<Qubit>,
        revision: u64,
    ) -> Self {
        Self {
            manager_id,
            requirement,
            qubits,
            new_qubits,
            revision,
        }
    }

    /// Returns the logical qubits selected for the prospective lease.
    pub fn qubits(&self) -> &[Qubit] {
        &self.qubits
    }

    /// Returns the ancillary-resource contract captured by this plan.
    pub const fn requirement(&self) -> AncillaRequirement {
        self.requirement
    }

    /// Returns how many logical qubits must be added when this plan is committed.
    pub fn num_new_qubits(&self) -> usize {
        self.new_qubits.len()
    }

    pub(super) const fn revision(&self) -> u64 {
        self.revision
    }

    pub(super) const fn manager_id(&self) -> u64 {
        self.manager_id
    }

    pub(super) fn new_qubits(&self) -> &[Qubit] {
        &self.new_qubits
    }
}

/// Credential representing an active ancillary-resource lease.
///
/// A lease is returned by [`super::ResourceManager::commit`] after its qubits
/// have been reserved. A consuming algorithm may use [`Self::qubits`] until it
/// has restored the contract reported by [`Self::requirement`], then must pass
/// the lease to [`super::ResourceManager::release`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLease {
    manager_id: u64,
    id: u64,
    requirement: AncillaRequirement,
    qubits: Vec<Qubit>,
}

impl ResourceLease {
    pub(super) const fn new(
        manager_id: u64,
        id: u64,
        requirement: AncillaRequirement,
        qubits: Vec<Qubit>,
    ) -> Self {
        Self {
            manager_id,
            id,
            requirement,
            qubits,
        }
    }

    /// Returns the manager-local identifier for this lease.
    pub const fn id(&self) -> u64 {
        self.id
    }

    pub(super) const fn manager_id(&self) -> u64 {
        self.manager_id
    }

    /// Returns the leased logical qubits.
    pub fn qubits(&self) -> &[Qubit] {
        &self.qubits
    }

    /// Returns the ancillary-resource contract captured by this lease.
    pub const fn requirement(&self) -> AncillaRequirement {
        self.requirement
    }
}
