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

use super::model::QubitOrigin;
use super::{
    AncillaRequirement, ResourceError, ResourceLease, ResourceLimits, ResourcePlan, ResourcePolicy,
    ResourceRequest,
};
use crate::circuit::{Circuit, Qubit};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_MANAGER_ID: AtomicU64 = AtomicU64::new(0);

/// Resource-allocation phase relative to logical-to-physical layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResourcePhase {
    /// Logical ancillary qubits may still be created under policy limits.
    PreLayout,
    /// Only already-mapped ancillary resources may be reused.
    PostLayout,
}

impl ResourcePhase {
    const fn diagnostic_name(self) -> &'static str {
        match self {
            Self::PreLayout => "pre-layout",
            Self::PostLayout => "post-layout",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LeaseRecord {
    requirement: AncillaRequirement,
    qubits: Vec<Qubit>,
}

/// Tracks compiler-visible logical qubits and temporary ancillary-resource leases.
///
/// A manager is created from one circuit and must remain synchronized with that
/// circuit. It separates resource selection from mutation: [`Self::preview`]
/// returns a side-effect-free candidate, while [`Self::commit`] reserves the
/// selected qubits and adds any planned pre-layout logical ancillas to the
/// circuit. The caller must satisfy the lease's restoration contract before
/// calling [`Self::release`].
///
/// The manager records contracts and resource ownership only. It does not
/// analyze quantum state or prove that a consuming transform restored leased
/// qubits correctly.
#[derive(Debug)]
pub struct ResourceManager {
    id: u64,
    phase: ResourcePhase,
    resources: BTreeMap<Qubit, QubitOrigin>,
    leased_qubits: BTreeSet<Qubit>,
    active_leases: BTreeMap<u64, LeaseRecord>,
    revision: u64,
    next_lease_id: u64,
    policy: ResourcePolicy,
    limits: ResourceLimits,
}

impl ResourceManager {
    /// Creates a pre-layout resource manager for the circuit's current qubits.
    ///
    /// Existing circuit qubits are registered as input resources. They are not
    /// assumed to be clean at an arbitrary compiler boundary, but may later be
    /// borrowed for a dirty request when permitted by [`ResourcePolicy`].
    ///
    /// The returned manager must be used with this circuit as both evolve.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceError::CapacityExceeded`] when the input circuit
    /// already exceeds `limits`, or [`ResourceError::CounterOverflow`] if an
    /// internal manager identifier cannot be allocated.
    pub fn from_circuit(
        circuit: &Circuit,
        policy: ResourcePolicy,
        limits: ResourceLimits,
    ) -> Result<Self, ResourceError> {
        if let Some(limit) = limits.max_total_qubits
            && circuit.num_qubits() > limit
        {
            return Err(ResourceError::CapacityExceeded {
                limit,
                requested_total: circuit.num_qubits(),
            });
        }

        let resources = circuit
            .qubits()
            .into_iter()
            .map(|qubit| (qubit, QubitOrigin::Input))
            .collect();

        Ok(Self {
            id: next_manager_id()?,
            phase: ResourcePhase::PreLayout,
            resources,
            leased_qubits: BTreeSet::new(),
            active_leases: BTreeMap::new(),
            revision: 0,
            next_lease_id: 0,
            policy,
            limits,
        })
    }

    /// Returns a side-effect-free resource preview for an algorithm candidate.
    ///
    /// Preview and commit are separate so a planner can compare feasible
    /// algorithms without mutating the circuit or reserving resources. A clean
    /// request may plan fresh logical ancillas before layout. A dirty request
    /// only reuses registered resources and never plans fresh qubits.
    ///
    /// Qubits listed in [`ResourceRequest::excluded`] are never selected. The
    /// returned plan is tied to this manager's current revision and becomes
    /// stale after a successful commit, release, or phase transition.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceError::InvalidRequest`] for malformed requests,
    /// [`ResourceError::InsufficientResources`] when the active policy cannot
    /// satisfy the request, [`ResourceError::CapacityExceeded`] when new clean
    /// qubits would exceed the configured total, or
    /// [`ResourceError::QubitIdOverflow`] when no new logical identifier can
    /// be represented.
    pub fn preview(&self, request: &ResourceRequest) -> Result<ResourcePlan, ResourceError> {
        self.validate_request(request)?;

        match request.requirement {
            AncillaRequirement::CleanZero => self.preview_clean(request),
            AncillaRequirement::Dirty => self.preview_dirty(request),
        }
    }

    /// Commits a previously previewed resource selection.
    ///
    /// The plan must come from this manager's current revision. Successful
    /// commits add any planned clean logical qubits to `circuit` atomically,
    /// reserve all selected qubits, and return a lease that must later be passed
    /// to [`Self::release`]. While the lease is active, its qubits cannot be
    /// selected by another plan.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceError::ForeignPlan`] for a plan created by another
    /// manager, [`ResourceError::StalePlan`] for an outdated preview, or a
    /// consistency, phase, capacity, or counter error when the commit cannot
    /// be applied atomically.
    pub fn commit(
        &mut self,
        circuit: &mut Circuit,
        plan: ResourcePlan,
    ) -> Result<ResourceLease, ResourceError> {
        if plan.manager_id() != self.id {
            return Err(ResourceError::ForeignPlan);
        }
        // A revision change means another candidate committed or the resource
        // pool changed after preview, so the old selection must not be reused.
        if plan.revision() != self.revision {
            return Err(ResourceError::StalePlan {
                plan_revision: plan.revision(),
                current_revision: self.revision,
            });
        }

        self.verify_consistency(circuit)?;
        self.validate_plan(&plan)?;

        let next_revision = self.next_revision()?;
        let lease_id = self.next_lease_id;
        let next_lease_id =
            self.next_lease_id
                .checked_add(1)
                .ok_or(ResourceError::CounterOverflow {
                    counter: "lease identifier",
                })?;

        if !plan.new_qubits().is_empty() {
            circuit
                .add_qubits(plan.new_qubits().to_vec())
                .map_err(|error| ResourceError::CircuitResourceMismatch {
                    reason: format!("circuit rejected validated ancillary qubits: {error}"),
                })?;
        }

        for &qubit in plan.new_qubits() {
            let previous = self.resources.insert(qubit, QubitOrigin::CompilerAllocated);
            debug_assert!(previous.is_none());
        }
        for &qubit in plan.qubits() {
            let inserted = self.leased_qubits.insert(qubit);
            debug_assert!(inserted);
        }

        let lease = ResourceLease::new(
            self.id,
            lease_id,
            plan.requirement(),
            plan.qubits().to_vec(),
        );
        let previous = self.active_leases.insert(
            lease_id,
            LeaseRecord {
                requirement: lease.requirement(),
                qubits: lease.qubits().to_vec(),
            },
        );
        debug_assert!(previous.is_none());

        self.revision = next_revision;
        self.next_lease_id = next_lease_id;
        Ok(lease)
    }

    /// Releases a completed ancillary-resource lease.
    ///
    /// The consuming algorithm must satisfy its restoration contract before
    /// calling this method. The manager records that contract but cannot prove
    /// the algorithm's quantum semantics. Released qubits remain registered and
    /// may satisfy later requests; releasing a lease does not remove qubits from
    /// the circuit.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceError::UnknownLease`] when the lease belongs to
    /// another manager, has already been released, or does not match the
    /// active record. Returns a consistency or counter error if internal
    /// indexes cannot be updated safely.
    pub fn release(&mut self, lease: &ResourceLease) -> Result<(), ResourceError> {
        if lease.manager_id() != self.id {
            return Err(ResourceError::UnknownLease {
                lease_id: lease.id(),
            });
        }
        let Some(record) = self.active_leases.get(&lease.id()) else {
            return Err(ResourceError::UnknownLease {
                lease_id: lease.id(),
            });
        };
        if record.requirement != lease.requirement() || record.qubits.as_slice() != lease.qubits() {
            return Err(ResourceError::UnknownLease {
                lease_id: lease.id(),
            });
        }
        if record
            .qubits
            .iter()
            .any(|qubit| !self.leased_qubits.contains(qubit))
        {
            return Err(ResourceError::CircuitResourceMismatch {
                reason: format!("lease {} is missing a leased-qubit record", lease.id()),
            });
        }

        let next_revision = self.next_revision()?;
        let record = self
            .active_leases
            .remove(&lease.id())
            .ok_or(ResourceError::UnknownLease {
                lease_id: lease.id(),
            })?;
        for qubit in record.qubits {
            let removed = self.leased_qubits.remove(&qubit);
            debug_assert!(removed);
        }
        self.revision = next_revision;
        Ok(())
    }

    /// Enters the phase where all reusable logical ancillary qubits must
    /// already be backed by a layout mapping.
    ///
    /// The transition is one-way and requires an idle, consistent manager.
    /// After this call, previews may reuse existing compiler-allocated ancillas
    /// but may not plan fresh logical qubits.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceError::InvalidPhase`] unless the manager is currently
    /// pre-layout, or a consistency or idle error when the circuit is not at a
    /// valid layout boundary.
    pub fn enter_post_layout(&mut self, circuit: &Circuit) -> Result<(), ResourceError> {
        if self.phase != ResourcePhase::PreLayout {
            return Err(ResourceError::InvalidPhase {
                operation: "enter_post_layout",
                expected: ResourcePhase::PreLayout.diagnostic_name(),
                actual: self.phase.diagnostic_name(),
            });
        }
        self.verify_idle(circuit)?;
        let next_revision = self.next_revision()?;
        self.phase = ResourcePhase::PostLayout;
        self.revision = next_revision;
        Ok(())
    }

    /// Checks that the circuit and all resource-manager indexes agree.
    ///
    /// This verifies structural bookkeeping only. It does not inspect quantum
    /// state or establish that active consumers will satisfy their restoration
    /// contracts.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceError::CircuitResourceMismatch`] when a circuit
    /// qubit, resource record, or lease index is inconsistent.
    pub fn verify_consistency(&self, circuit: &Circuit) -> Result<(), ResourceError> {
        let circuit_qubits: BTreeSet<_> = circuit.qubits().into_iter().collect();
        for qubit in &circuit_qubits {
            if !self.resources.contains_key(qubit) {
                return Err(ResourceError::CircuitResourceMismatch {
                    reason: format!("circuit qubit {qubit} has no resource record"),
                });
            }
        }
        for qubit in self.resources.keys() {
            if !circuit_qubits.contains(qubit) {
                return Err(ResourceError::CircuitResourceMismatch {
                    reason: format!("resource qubit {qubit} is absent from the circuit"),
                });
            }
        }
        for qubit in &self.leased_qubits {
            if !self.resources.contains_key(qubit) {
                return Err(ResourceError::CircuitResourceMismatch {
                    reason: format!("leased qubit {qubit} has no resource record"),
                });
            }
        }

        let mut indexed_lease_qubits = BTreeSet::new();
        for (&lease_id, record) in &self.active_leases {
            for &qubit in &record.qubits {
                if !self.resources.contains_key(&qubit) {
                    return Err(ResourceError::CircuitResourceMismatch {
                        reason: format!("lease {lease_id} contains unknown qubit {qubit}"),
                    });
                }
                if !indexed_lease_qubits.insert(qubit) {
                    return Err(ResourceError::CircuitResourceMismatch {
                        reason: format!("qubit {qubit} is present in overlapping leases"),
                    });
                }
            }
        }
        if indexed_lease_qubits != self.leased_qubits {
            return Err(ResourceError::CircuitResourceMismatch {
                reason: "leased-qubit index does not match active leases".to_string(),
            });
        }
        Ok(())
    }

    /// Checks resource indexes and requires every temporary lease to be released.
    ///
    /// Compiler stages should use this at boundaries where no algorithm may
    /// retain temporary ancillary resources, including before entering
    /// post-layout and after resource-consuming transformations complete.
    ///
    /// # Errors
    ///
    /// Returns the consistency errors documented by
    /// [`Self::verify_consistency`], or [`ResourceError::ActiveLeaseRemaining`]
    /// while any lease remains active.
    pub fn verify_idle(&self, circuit: &Circuit) -> Result<(), ResourceError> {
        self.verify_consistency(circuit)?;
        if !self.active_leases.is_empty() {
            return Err(ResourceError::ActiveLeaseRemaining {
                count: self.active_leases.len(),
            });
        }
        Ok(())
    }

    fn preview_clean(&self, request: &ResourceRequest) -> Result<ResourcePlan, ResourceError> {
        let mut qubits = Vec::with_capacity(request.count);
        self.select_origin(QubitOrigin::CompilerAllocated, request, &mut qubits);

        let mut new_qubits = Vec::new();
        if qubits.len() < request.count {
            if self.phase == ResourcePhase::PostLayout {
                // A fresh logical qubit created after layout would not have a
                // physical mapping, so post-layout requests may only reuse.
                return Err(self.insufficient(request, qubits.len()));
            }

            let required_new = request.count - qubits.len();
            let allocated = self.num_compiler_allocated();
            let available_new = self
                .policy
                .max_pre_layout_clean_ancillas
                .saturating_sub(allocated);
            if required_new > available_new {
                return Err(self.insufficient(request, qubits.len() + available_new));
            }
            self.ensure_total_capacity(required_new)?;

            // New logical IDs follow the current maximum. Checked arithmetic
            // makes the Qubit(u32) representation boundary explicit.
            new_qubits = self.allocate_new_qubits(required_new)?;
            qubits.extend_from_slice(&new_qubits);
        }

        Ok(ResourcePlan::new(
            self.id,
            request.requirement,
            qubits,
            new_qubits,
            self.revision,
        ))
    }

    fn preview_dirty(&self, request: &ResourceRequest) -> Result<ResourcePlan, ResourceError> {
        let mut qubits = Vec::with_capacity(request.count);
        self.select_origin(QubitOrigin::CompilerAllocated, request, &mut qubits);
        if self.policy.allow_dirty_borrowing {
            self.select_origin(QubitOrigin::Input, request, &mut qubits);
        }
        if qubits.len() < request.count {
            return Err(self.insufficient(request, qubits.len()));
        }

        Ok(ResourcePlan::new(
            self.id,
            request.requirement,
            qubits,
            Vec::new(),
            self.revision,
        ))
    }

    fn select_origin(
        &self,
        origin: QubitOrigin,
        request: &ResourceRequest,
        selected: &mut Vec<Qubit>,
    ) {
        for (&qubit, &qubit_origin) in &self.resources {
            if selected.len() == request.count {
                break;
            }
            if qubit_origin == origin
                && !self.leased_qubits.contains(&qubit)
                && !request.excluded.contains(&qubit)
            {
                selected.push(qubit);
            }
        }
    }

    fn validate_request(&self, request: &ResourceRequest) -> Result<(), ResourceError> {
        if request.count == 0 {
            return Err(ResourceError::InvalidRequest {
                reason: "ancillary-qubit count must be greater than zero".to_string(),
            });
        }
        for qubit in &request.excluded {
            if !self.resources.contains_key(qubit) {
                return Err(ResourceError::InvalidRequest {
                    reason: format!("excluded qubit {qubit} is not registered"),
                });
            }
        }
        Ok(())
    }

    fn validate_plan(&self, plan: &ResourcePlan) -> Result<(), ResourceError> {
        if !plan.new_qubits().is_empty() && self.phase != ResourcePhase::PreLayout {
            return Err(ResourceError::InvalidPhase {
                operation: "commit new logical ancillary qubits",
                expected: ResourcePhase::PreLayout.diagnostic_name(),
                actual: self.phase.diagnostic_name(),
            });
        }

        let new_qubits: BTreeSet<_> = plan.new_qubits().iter().copied().collect();
        if new_qubits.len() != plan.new_qubits().len() {
            return Err(ResourceError::CircuitResourceMismatch {
                reason: "resource plan contains duplicate new qubits".to_string(),
            });
        }
        let selected_qubits: BTreeSet<_> = plan.qubits().iter().copied().collect();
        if selected_qubits.len() != plan.qubits().len() {
            return Err(ResourceError::CircuitResourceMismatch {
                reason: "resource plan contains duplicate selected qubits".to_string(),
            });
        }
        for qubit in &new_qubits {
            if self.resources.contains_key(qubit) {
                return Err(ResourceError::DuplicateQubit { qubit: *qubit });
            }
        }
        for qubit in &selected_qubits {
            if self.leased_qubits.contains(qubit) {
                return Err(ResourceError::CircuitResourceMismatch {
                    reason: format!("resource plan selected already-leased qubit {qubit}"),
                });
            }
            if !self.resources.contains_key(qubit) && !new_qubits.contains(qubit) {
                return Err(ResourceError::CircuitResourceMismatch {
                    reason: format!("resource plan selected unknown qubit {qubit}"),
                });
            }
        }
        self.ensure_total_capacity(new_qubits.len())
    }

    fn allocate_new_qubits(&self, count: usize) -> Result<Vec<Qubit>, ResourceError> {
        let mut qubits = Vec::with_capacity(count);
        let mut next_id = match self.resources.last_key_value() {
            Some((qubit, _)) => qubit
                .id()
                .checked_add(1)
                .ok_or(ResourceError::QubitIdOverflow)?,
            None => 0,
        };
        for index in 0..count {
            qubits.push(Qubit::new(next_id));
            if index + 1 < count {
                next_id = next_id
                    .checked_add(1)
                    .ok_or(ResourceError::QubitIdOverflow)?;
            }
        }
        Ok(qubits)
    }

    fn ensure_total_capacity(&self, additional: usize) -> Result<(), ResourceError> {
        let requested_total = self
            .resources
            .len()
            .checked_add(additional)
            .ok_or_else(|| ResourceError::InvalidRequest {
                reason: "logical-qubit count overflow".to_string(),
            })?;
        if let Some(limit) = self.limits.max_total_qubits
            && requested_total > limit
        {
            return Err(ResourceError::CapacityExceeded {
                limit,
                requested_total,
            });
        }
        Ok(())
    }

    fn num_compiler_allocated(&self) -> usize {
        self.resources
            .values()
            .filter(|&&origin| origin == QubitOrigin::CompilerAllocated)
            .count()
    }

    fn next_revision(&self) -> Result<u64, ResourceError> {
        self.revision
            .checked_add(1)
            .ok_or(ResourceError::CounterOverflow {
                counter: "revision",
            })
    }

    fn insufficient(&self, request: &ResourceRequest, available: usize) -> ResourceError {
        ResourceError::InsufficientResources {
            requirement: request.requirement.diagnostic_name(),
            requested: request.count,
            available,
        }
    }
}

fn next_manager_id() -> Result<u64, ResourceError> {
    NEXT_MANAGER_ID
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .map_err(|_| ResourceError::CounterOverflow {
            counter: "manager identifier",
        })
}
