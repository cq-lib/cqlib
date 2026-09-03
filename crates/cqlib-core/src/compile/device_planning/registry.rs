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

use super::session::{DevicePlanningSessionCache, prepare_cache_baseline};
use super::swap_equivalence::prepare_equivalent_swaps;
use super::{CalibrationEstimator, DeviceGateState};
use crate::circuit::{Instruction, StandardGate};
use crate::compile::CompilerError;
use crate::compile::knowledge::{KnowledgeInstructionKey, RuleLibrary};
use crate::device::{Device, PhysicalQubit};
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};
use std::mem::size_of;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

// Bump the rule-set revision for built-in decomposition or direction-template
// changes. Bump the algorithm revision for graph expansion, feasibility,
// physical-cost, or selection-order changes.
const DEVICE_PLANNING_RULESET_REVISION: u32 = 1;
const DEVICE_PLANNING_ALGORITHM_REVISION: u32 = 3;
const DEVICE_PLANNING_REGISTRY_MAX_ENTRIES: usize = 8;
const DEVICE_PLANNING_REGISTRY_MAX_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct CalibrationFingerprint {
    error_rate: Option<u64>,
    duration: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NativeCapabilityFingerprint {
    gate: StandardGate,
    ordered_qargs: SmallVec<[PhysicalQubit; 2]>,
    calibration: Option<CalibrationFingerprint>,
}

/// Semantic namespace for planning knowledge. Deliberately excluded fields
/// include device name, calibration timestamp, readout and coherence data:
/// none is consulted by planning or its calibration estimator.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct DevicePlanningNamespace {
    ruleset_revision: u32,
    algorithm_revision: u32,
    usable_qubits: Vec<PhysicalQubit>,
    directed_edges: Vec<[PhysicalQubit; 2]>,
    native_capabilities: Vec<NativeCapabilityFingerprint>,
}

impl DevicePlanningNamespace {
    pub(super) fn from_device(device: &Device) -> Self {
        let usable_qubits = device.usable_qubits().collect::<Vec<_>>();
        let usable = usable_qubits.iter().copied().collect::<HashSet<_>>();
        let mut directed_edges = Vec::new();
        for &left in &usable_qubits {
            directed_edges.extend(
                device
                    .topology()
                    .successors(left)
                    .filter(|right| usable.contains(right))
                    .map(|right| [left, right]),
            );
        }
        directed_edges.sort();
        directed_edges.dedup();

        let mut native_capabilities = Vec::new();
        for gate in StandardGate::all().iter().copied() {
            let instruction = Instruction::Standard(gate);
            match gate.num_qubits() {
                0 => {
                    let ordered_qargs = SmallVec::new();
                    native_capabilities.push(NativeCapabilityFingerprint {
                        gate,
                        calibration: calibration_fingerprint(device, &instruction, &ordered_qargs),
                        ordered_qargs,
                    });
                }
                1 => {
                    for &physical in &usable_qubits {
                        let ordered_qargs = SmallVec::from_slice(&[physical]);
                        native_capabilities.push(NativeCapabilityFingerprint {
                            gate,
                            calibration: calibration_fingerprint(
                                device,
                                &instruction,
                                &ordered_qargs,
                            ),
                            ordered_qargs,
                        });
                    }
                }
                2 => {
                    for ordered in &directed_edges {
                        let ordered_qargs = SmallVec::from_slice(ordered);
                        native_capabilities.push(NativeCapabilityFingerprint {
                            gate,
                            calibration: calibration_fingerprint(
                                device,
                                &instruction,
                                &ordered_qargs,
                            ),
                            ordered_qargs,
                        });
                    }
                }
                _ => {}
            }
        }
        Self {
            ruleset_revision: DEVICE_PLANNING_RULESET_REVISION,
            algorithm_revision: DEVICE_PLANNING_ALGORITHM_REVISION,
            usable_qubits,
            directed_edges,
            native_capabilities,
        }
    }

    fn estimated_bytes(&self) -> usize {
        size_of::<Self>()
            .saturating_add(
                self.usable_qubits
                    .capacity()
                    .saturating_mul(size_of::<PhysicalQubit>()),
            )
            .saturating_add(
                self.directed_edges
                    .capacity()
                    .saturating_mul(size_of::<[PhysicalQubit; 2]>()),
            )
            .saturating_add(
                self.native_capabilities
                    .capacity()
                    .saturating_mul(size_of::<NativeCapabilityFingerprint>()),
            )
    }
}

pub(super) fn calibration_fingerprint(
    device: &Device,
    instruction: &Instruction,
    ordered_qargs: &[PhysicalQubit],
) -> Option<CalibrationFingerprint> {
    device
        .native_instruction_calibration(instruction, ordered_qargs)
        .map(|calibration| CalibrationFingerprint {
            error_rate: calibration.error_rate.map(f64::to_bits),
            duration: calibration.duration.map(f64::to_bits),
        })
}

#[derive(Debug, Default)]
struct SharedPlanningStats {
    common_cache_hits: AtomicU64,
    cold_batches: AtomicU64,
    representative_solves: AtomicU64,
    equivalent_roots_reused: AtomicU64,
    fast_path_fallbacks: AtomicU64,
}

#[derive(Debug)]
pub(super) struct DevicePlanningKnowledge {
    namespace: DevicePlanningNamespace,
    pub(super) device: Arc<Device>,
    pub(super) estimator: Arc<CalibrationEstimator>,
    common_cache: Mutex<DevicePlanningSessionCache>,
    estimated_bytes: AtomicUsize,
    stats: SharedPlanningStats,
}

impl DevicePlanningKnowledge {
    fn new(device: &Device, namespace: DevicePlanningNamespace) -> Self {
        let device = Arc::new(device.clone());
        let estimator = Arc::new(CalibrationEstimator::from_device(
            &device,
            &namespace.usable_qubits,
        ));
        let estimated_bytes = namespace
            .estimated_bytes()
            .saturating_add(size_of::<Self>())
            .saturating_add(size_of::<Device>());
        Self {
            namespace,
            device,
            estimator,
            common_cache: Mutex::new(DevicePlanningSessionCache::default()),
            estimated_bytes: AtomicUsize::new(estimated_bytes),
            stats: SharedPlanningStats::default(),
        }
    }

    pub(super) fn is_common_swap(&self, root: &DeviceGateState) -> bool {
        root.instruction == KnowledgeInstructionKey::Standard(StandardGate::SWAP)
            && root.ordered_qargs.len() == 2
            && root.ordered_qargs[0] != root.ordered_qargs[1]
            && self.device.is_usable_qubit(root.ordered_qargs[0])
            && self.device.is_usable_qubit(root.ordered_qargs[1])
            && self
                .device
                .topology()
                .supports_coupling_either_direction(root.ordered_qargs[0], root.ordered_qargs[1])
    }

    pub(super) fn prepare_common(
        &self,
        mut roots: Vec<DeviceGateState>,
    ) -> Result<Arc<DevicePlanningSessionCache>, CompilerError> {
        roots.sort();
        roots.dedup();
        if roots.is_empty() {
            return Ok(Arc::new(DevicePlanningSessionCache::default()));
        }
        let requested_roots = roots.clone();
        let requested = roots.len();
        let mut cache = self
            .common_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        roots.retain(|root| !cache.contains_key(root));
        self.stats
            .common_cache_hits
            .fetch_add((requested - roots.len()) as u64, Ordering::Relaxed);
        if roots.is_empty() {
            return Ok(Arc::new(cache.subset(&requested_roots)?));
        }

        self.stats.cold_batches.fetch_add(1, Ordering::Relaxed);
        let library = RuleLibrary::builtin_rules()
            .map_err(|error| CompilerError::InvariantViolation(error.to_string()))?;
        let mut staged = DevicePlanningSessionCache::default();
        match prepare_equivalent_swaps(
            &self.device,
            library,
            &roots,
            Arc::clone(&self.estimator),
            &mut staged,
        ) {
            Ok(equivalence_classes) => {
                self.stats
                    .representative_solves
                    .fetch_add(equivalence_classes as u64, Ordering::Relaxed);
                self.stats.equivalent_roots_reused.fetch_add(
                    roots.len().saturating_sub(equivalence_classes) as u64,
                    Ordering::Relaxed,
                );
            }
            Err(_) => {
                // Optimization failure must not alter compiler behavior.
                self.stats
                    .fast_path_fallbacks
                    .fetch_add(1, Ordering::Relaxed);
                staged = DevicePlanningSessionCache::default();
                prepare_cache_baseline(
                    &self.device,
                    library,
                    roots.iter().cloned(),
                    Arc::clone(&self.estimator),
                    &mut staged,
                )?;
            }
        }
        // The process registry is intentionally restricted to common SWAP
        // roots. Their selected trees own every contextual dependency.
        staged.retain(|state| requested_roots.binary_search(state).is_ok());
        cache.merge(staged)?;
        let pinned = Arc::new(cache.subset(&requested_roots)?);
        let cache_bytes = cache.estimated_bytes();
        drop(cache);

        self.estimated_bytes.store(
            self.namespace
                .estimated_bytes()
                .saturating_add(size_of::<Self>())
                .saturating_add(size_of::<Device>())
                .saturating_add(cache_bytes),
            Ordering::Relaxed,
        );
        refresh_registry_entry(self);
        Ok(pinned)
    }
}

#[derive(Debug)]
struct RegistryEntry {
    knowledge: Arc<DevicePlanningKnowledge>,
    last_access: u64,
    estimated_bytes: usize,
}

#[derive(Debug)]
struct DevicePlanningRegistry {
    entries: HashMap<DevicePlanningNamespace, RegistryEntry>,
    clock: u64,
}

impl DevicePlanningRegistry {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            clock: 0,
        }
    }

    fn next_access(&mut self) -> u64 {
        self.clock = self.clock.wrapping_add(1);
        self.clock
    }

    fn refresh_weights(&mut self) {
        for entry in self.entries.values_mut() {
            entry.estimated_bytes = entry.knowledge.estimated_bytes.load(Ordering::Relaxed);
        }
    }

    fn evict_to_limits(&mut self, protected: &DevicePlanningNamespace) {
        self.refresh_weights();
        loop {
            let total_bytes = self.entries.values().fold(0_usize, |total, entry| {
                total.saturating_add(entry.estimated_bytes)
            });
            if self.entries.len() <= DEVICE_PLANNING_REGISTRY_MAX_ENTRIES
                && total_bytes <= DEVICE_PLANNING_REGISTRY_MAX_BYTES
            {
                break;
            }
            let candidate = self
                .entries
                .iter()
                .filter(|(namespace, _)| *namespace != protected || self.entries.len() == 1)
                .min_by_key(|(_, entry)| entry.last_access)
                .map(|(namespace, _)| namespace.clone());
            let Some(candidate) = candidate else {
                break;
            };
            if candidate == *protected && self.entries.len() == 1 {
                // Keep one active oversized entry; otherwise reuse would be
                // disabled for any device whose catalog exceeds the soft cap.
                break;
            }
            self.entries.remove(&candidate);
        }
    }
}

static DEVICE_PLANNING_REGISTRY: OnceLock<Mutex<DevicePlanningRegistry>> = OnceLock::new();
static DEVICE_PLANNING_REGISTRY_HITS: AtomicU64 = AtomicU64::new(0);
static DEVICE_PLANNING_REGISTRY_MISSES: AtomicU64 = AtomicU64::new(0);

fn planning_registry() -> &'static Mutex<DevicePlanningRegistry> {
    DEVICE_PLANNING_REGISTRY.get_or_init(|| Mutex::new(DevicePlanningRegistry::new()))
}

pub(super) fn shared_knowledge(device: &Device) -> Arc<DevicePlanningKnowledge> {
    let namespace = DevicePlanningNamespace::from_device(device);
    let mut registry = planning_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let access = registry.next_access();
    if let Some(entry) = registry.entries.get_mut(&namespace) {
        entry.last_access = access;
        let knowledge = Arc::clone(&entry.knowledge);
        DEVICE_PLANNING_REGISTRY_HITS.fetch_add(1, Ordering::Relaxed);
        registry.evict_to_limits(&namespace);
        return knowledge;
    }

    DEVICE_PLANNING_REGISTRY_MISSES.fetch_add(1, Ordering::Relaxed);
    let knowledge = Arc::new(DevicePlanningKnowledge::new(device, namespace.clone()));
    registry.entries.insert(
        namespace.clone(),
        RegistryEntry {
            estimated_bytes: knowledge.estimated_bytes.load(Ordering::Relaxed),
            knowledge: Arc::clone(&knowledge),
            last_access: access,
        },
    );
    registry.evict_to_limits(&namespace);
    knowledge
}

fn refresh_registry_entry(knowledge: &DevicePlanningKnowledge) {
    let mut registry = planning_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let access = registry.next_access();
    if let Some(entry) = registry.entries.get_mut(&knowledge.namespace)
        && std::ptr::eq(Arc::as_ptr(&entry.knowledge), knowledge)
    {
        entry.last_access = access;
        entry.estimated_bytes = knowledge.estimated_bytes.load(Ordering::Relaxed);
    }
    registry.evict_to_limits(&knowledge.namespace);
}
