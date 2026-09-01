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

use super::registry::{CalibrationFingerprint, calibration_fingerprint};
use super::session::{
    CanonicalRootResult, DevicePlanningSessionCache, SelectedNativePlan, populate_canonical_cache,
};
use super::{
    CalibrationEstimator, DeviceGateState, DevicePlanner, NativePlanLeaf, NativePlanSummary,
};
use crate::circuit::{Instruction, StandardGate};
use crate::compile::error::{
    DeviceLoweringCandidateFailure, DeviceLoweringDependency, DeviceLoweringFailure,
};
use crate::compile::knowledge::{KnowledgeInstructionKey, RuleLibrary};
use crate::device::{Device, PhysicalQubit};
use smallvec::SmallVec;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SwapPlanningSignature {
    directed_coupling: [bool; 2],
    native_calibrations: Vec<Option<CalibrationFingerprint>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LocalPlanningSignature {
    instruction: KnowledgeInstructionKey,
    environment: Arc<LocalEnvironmentSignature>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LocalEnvironmentSignature {
    directed_coupling: [bool; 2],
    native_calibrations: Vec<Option<CalibrationFingerprint>>,
}

impl LocalPlanningSignature {
    fn for_root(
        device: &Device,
        root: &DeviceGateState,
        environments: &mut HashMap<SmallVec<[PhysicalQubit; 2]>, Arc<LocalEnvironmentSignature>>,
    ) -> Result<Self, String> {
        if !matches!(root.instruction, KnowledgeInstructionKey::Standard(_))
            || !(1..=2).contains(&root.ordered_qargs.len())
            || root
                .ordered_qargs
                .iter()
                .any(|qarg| !device.is_usable_qubit(*qarg))
            || (root.ordered_qargs.len() == 2 && root.ordered_qargs[0] == root.ordered_qargs[1])
        {
            return Err(format!(
                "local equivalence planner received unsupported root {root:?}"
            ));
        }

        let environment = if let Some(environment) = environments.get(&root.ordered_qargs) {
            Arc::clone(environment)
        } else {
            let environment = Arc::new(LocalEnvironmentSignature::for_qargs(
                device,
                &root.ordered_qargs,
            ));
            environments.insert(root.ordered_qargs.clone(), Arc::clone(&environment));
            environment
        };
        Ok(Self {
            instruction: root.instruction.clone(),
            environment,
        })
    }
}

impl LocalEnvironmentSignature {
    fn for_qargs(device: &Device, ordered_qargs: &[PhysicalQubit]) -> Self {
        let directed_coupling = if ordered_qargs.len() == 2 {
            [
                device
                    .topology()
                    .supports_directed_coupling(ordered_qargs[0], ordered_qargs[1]),
                device
                    .topology()
                    .supports_directed_coupling(ordered_qargs[1], ordered_qargs[0]),
            ]
        } else {
            [false; 2]
        };
        let mut native_calibrations = Vec::new();
        for gate in StandardGate::all().iter().copied() {
            let instruction = Instruction::Standard(gate);
            match gate.num_qubits() {
                1 => {
                    for &qarg in ordered_qargs {
                        native_calibrations.push(calibration_fingerprint(
                            device,
                            &instruction,
                            &[qarg],
                        ));
                    }
                }
                2 if ordered_qargs.len() == 2 => {
                    native_calibrations.push(calibration_fingerprint(
                        device,
                        &instruction,
                        ordered_qargs,
                    ));
                    native_calibrations.push(calibration_fingerprint(
                        device,
                        &instruction,
                        &[ordered_qargs[1], ordered_qargs[0]],
                    ));
                }
                _ => {}
            }
        }
        Self {
            directed_coupling,
            native_calibrations,
        }
    }
}

/// Plans one representative for each role-preserving local device class.
/// Any unsupported state shape or remapping failure aborts the optimization so
/// the caller can retry the complete exact-root batch.
pub(super) fn prepare_equivalent_local_roots(
    device: &Device,
    library: &RuleLibrary,
    roots: &[DeviceGateState],
    estimator: Arc<CalibrationEstimator>,
    cache: &mut DevicePlanningSessionCache,
) -> Result<usize, String> {
    let mut groups = BTreeMap::<LocalPlanningSignature, Vec<DeviceGateState>>::new();
    let mut environments = HashMap::new();
    for root in roots {
        groups
            .entry(LocalPlanningSignature::for_root(
                device,
                root,
                &mut environments,
            )?)
            .or_default()
            .push(root.clone());
    }
    let representatives = groups
        .values()
        .filter_map(|members| members.first().cloned())
        .collect::<Vec<_>>();
    let planner = DevicePlanner::build_with_estimator(
        device,
        library,
        representatives.iter().cloned(),
        estimator,
    )
    .map_err(|error| error.to_string())?;
    let mut owned_memo = HashMap::new();

    for members in groups.into_values() {
        let representative = members
            .first()
            .cloned()
            .ok_or_else(|| "empty local planning equivalence class".to_string())?;
        let mut representative_cache = DevicePlanningSessionCache::default();
        populate_canonical_cache(
            &planner,
            [representative.clone()],
            &mut representative_cache,
            &mut owned_memo,
        )
        .map_err(|error| error.to_string())?;
        for member in members {
            let mapped = if member.ordered_qargs == representative.ordered_qargs {
                representative_cache.clone()
            } else {
                remap_canonical_cache(
                    &representative_cache,
                    &representative.ordered_qargs,
                    &member.ordered_qargs,
                )?
            };
            cache.merge(mapped).map_err(|error| error.to_string())?;
        }
    }
    Ok(representatives.len())
}

impl SwapPlanningSignature {
    fn for_pair(device: &Device, pair: [PhysicalQubit; 2]) -> Self {
        let directed_coupling = [
            device
                .topology()
                .supports_directed_coupling(pair[0], pair[1]),
            device
                .topology()
                .supports_directed_coupling(pair[1], pair[0]),
        ];
        let mut native_calibrations = Vec::new();
        for gate in StandardGate::all().iter().copied() {
            let instruction = Instruction::Standard(gate);
            match gate.num_qubits() {
                1 => {
                    native_calibrations.push(calibration_fingerprint(
                        device,
                        &instruction,
                        &[pair[0]],
                    ));
                    native_calibrations.push(calibration_fingerprint(
                        device,
                        &instruction,
                        &[pair[1]],
                    ));
                }
                2 => {
                    native_calibrations.push(calibration_fingerprint(
                        device,
                        &instruction,
                        &[pair[0], pair[1]],
                    ));
                    native_calibrations.push(calibration_fingerprint(
                        device,
                        &instruction,
                        &[pair[1], pair[0]],
                    ));
                }
                _ => {}
            }
        }
        Self {
            directed_coupling,
            native_calibrations,
        }
    }
}

/// Plans one representative per exact device-equivalence class, then binds
/// the immutable selected tree to every member's physical qargs.
pub(super) fn prepare_equivalent_swaps(
    device: &Device,
    library: &RuleLibrary,
    roots: &[DeviceGateState],
    estimator: Arc<CalibrationEstimator>,
    cache: &mut DevicePlanningSessionCache,
) -> Result<usize, String> {
    let mut groups = BTreeMap::<SwapPlanningSignature, Vec<DeviceGateState>>::new();
    for root in roots {
        if root.instruction != KnowledgeInstructionKey::Standard(StandardGate::SWAP)
            || root.ordered_qargs.len() != 2
            || root.ordered_qargs[0] == root.ordered_qargs[1]
        {
            return Err(format!(
                "SWAP equivalence planner received invalid common root {root:?}"
            ));
        }
        let pair = [root.ordered_qargs[0], root.ordered_qargs[1]];
        groups
            .entry(SwapPlanningSignature::for_pair(device, pair))
            .or_default()
            .push(root.clone());
    }
    let equivalence_classes = groups.len();

    for members in groups.into_values() {
        let representative = members
            .first()
            .ok_or_else(|| "empty SWAP planning equivalence class".to_string())?;
        let planner = DevicePlanner::build_swap_with_estimator(
            device,
            library,
            representative,
            Arc::clone(&estimator),
        )
        .map_err(|error| error.to_string())?;
        let representative_pair = [
            representative.ordered_qargs[0],
            representative.ordered_qargs[1],
        ];

        let mut representative_cache = DevicePlanningSessionCache::default();
        let mut owned_memo = HashMap::new();
        populate_canonical_cache(
            &planner,
            [representative.clone()],
            &mut representative_cache,
            &mut owned_memo,
        )
        .map_err(|error| error.to_string())?;
        for member in members {
            let member_pair = [member.ordered_qargs[0], member.ordered_qargs[1]];
            let mapped = if member_pair == representative_pair {
                representative_cache.clone()
            } else {
                remap_canonical_cache(&representative_cache, &representative_pair, &member_pair)?
            };
            cache.merge(mapped).map_err(|error| error.to_string())?;
        }
    }
    Ok(equivalence_classes)
}

fn remap_canonical_cache(
    source: &DevicePlanningSessionCache,
    from: &[PhysicalQubit],
    to: &[PhysicalQubit],
) -> Result<DevicePlanningSessionCache, String> {
    let mut entries = source.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(state, _)| *state);
    let mut remapped = DevicePlanningSessionCache::default();
    let mut plan_memo = HashMap::new();
    for (state, result) in entries {
        let state = remap_state(state, from, to)?;
        let result = match result.as_ref() {
            CanonicalRootResult::Feasible(selected) => Arc::new(CanonicalRootResult::Feasible(
                remap_selected_plan(selected, from, to, &mut plan_memo)?,
            )),
            CanonicalRootResult::Unsupported(failure) => Arc::new(
                CanonicalRootResult::Unsupported(Arc::new(remap_failure(failure, from, to)?)),
            ),
        };
        remapped
            .insert(state, result)
            .map_err(|error| error.to_string())?;
    }
    Ok(remapped)
}

fn remap_selected_plan(
    selected: &Arc<SelectedNativePlan>,
    from: &[PhysicalQubit],
    to: &[PhysicalQubit],
    memo: &mut HashMap<usize, Arc<SelectedNativePlan>>,
) -> Result<Arc<SelectedNativePlan>, String> {
    let source = Arc::as_ptr(selected) as usize;
    if let Some(remapped) = memo.get(&source) {
        return Ok(Arc::clone(remapped));
    }
    let state = remap_state(&selected.state, from, to)?;
    let children = selected
        .children
        .iter()
        .map(|child| remap_selected_plan(child, from, to, memo))
        .collect::<Result<Vec<_>, _>>()?;
    let summary = Arc::new(NativePlanSummary {
        native_two_qubit_ops: selected.summary.native_two_qubit_ops,
        native_total_ops: selected.summary.native_total_ops,
        leaves: selected
            .summary
            .leaves
            .iter()
            .map(|leaf| {
                Ok(NativePlanLeaf {
                    instruction: leaf.instruction.clone(),
                    ordered_qargs: remap_small_qargs(&leaf.ordered_qargs, from, to)?,
                    error_rate: leaf.error_rate,
                    duration: leaf.duration,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
    });
    let remapped = Arc::new(SelectedNativePlan {
        state: state.clone(),
        choice: selected.choice,
        children: children.into(),
        physical_cost: selected.physical_cost,
        summary,
    });
    memo.insert(source, Arc::clone(&remapped));
    Ok(remapped)
}

fn remap_state(
    state: &DeviceGateState,
    from: &[PhysicalQubit],
    to: &[PhysicalQubit],
) -> Result<DeviceGateState, String> {
    Ok(DeviceGateState {
        instruction: state.instruction.clone(),
        ordered_qargs: remap_small_qargs(&state.ordered_qargs, from, to)?,
    })
}

fn remap_small_qargs(
    qargs: &[PhysicalQubit],
    from: &[PhysicalQubit],
    to: &[PhysicalQubit],
) -> Result<SmallVec<[PhysicalQubit; 2]>, String> {
    qargs
        .iter()
        .copied()
        .map(|qubit| remap_qubit(qubit, from, to))
        .collect()
}

fn remap_vec_qargs(
    qargs: &[PhysicalQubit],
    from: &[PhysicalQubit],
    to: &[PhysicalQubit],
) -> Result<Vec<PhysicalQubit>, String> {
    qargs
        .iter()
        .copied()
        .map(|qubit| remap_qubit(qubit, from, to))
        .collect()
}

fn remap_qubit(
    qubit: PhysicalQubit,
    from: &[PhysicalQubit],
    to: &[PhysicalQubit],
) -> Result<PhysicalQubit, String> {
    from.iter()
        .position(|candidate| *candidate == qubit)
        .and_then(|index| to.get(index).copied())
        .ok_or_else(|| {
            format!("selected plan contains qarg {qubit:?} outside representative roles {from:?}")
        })
}

fn remap_failure(
    failure: &DeviceLoweringFailure,
    from: &[PhysicalQubit],
    to: &[PhysicalQubit],
) -> Result<DeviceLoweringFailure, String> {
    Ok(DeviceLoweringFailure {
        instruction: failure.instruction.clone(),
        qargs: remap_vec_qargs(&failure.qargs, from, to)?,
        attempted_candidates: failure
            .attempted_candidates
            .iter()
            .map(|candidate| {
                Ok(DeviceLoweringCandidateFailure {
                    template: candidate.template.clone(),
                    unsatisfied_dependencies: candidate
                        .unsatisfied_dependencies
                        .iter()
                        .map(|dependency| {
                            Ok(DeviceLoweringDependency {
                                instruction: dependency.instruction.clone(),
                                qargs: remap_vec_qargs(&dependency.qargs, from, to)?,
                            })
                        })
                        .collect::<Result<Vec<_>, String>>()?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
    })
}

#[cfg(test)]
mod tests {
    use super::super::{DevicePhysicalCost, PlanChoice};
    use super::*;

    fn selected(
        state: DeviceGateState,
        native_total_ops: u32,
        children: Vec<Arc<SelectedNativePlan>>,
    ) -> Arc<SelectedNativePlan> {
        Arc::new(SelectedNativePlan {
            state,
            choice: PlanChoice::Native,
            children: children.into(),
            physical_cost: DevicePhysicalCost::optimistic_entangler_lower_bound(native_total_ops),
            summary: Arc::new(NativePlanSummary {
                native_two_qubit_ops: native_total_ops,
                native_total_ops,
                leaves: Vec::new(),
            }),
        })
    }

    #[test]
    fn remap_memo_distinguishes_pareto_nodes_for_the_same_state() {
        let source_qargs = [PhysicalQubit::new(0), PhysicalQubit::new(1)];
        let target_qargs = [PhysicalQubit::new(1), PhysicalQubit::new(0)];
        let child_state = DeviceGateState::standard(StandardGate::CX, source_qargs.into());
        let first = selected(child_state.clone(), 1, Vec::new());
        let second = selected(child_state, 2, Vec::new());
        let parent = selected(
            DeviceGateState::standard(StandardGate::CY, source_qargs.into()),
            3,
            vec![first, second],
        );

        let remapped =
            remap_selected_plan(&parent, &source_qargs, &target_qargs, &mut HashMap::new())
                .unwrap();

        assert_eq!(remapped.children[0].summary.native_total_ops, 1);
        assert_eq!(remapped.children[1].summary.native_total_ops, 2);
        assert!(!Arc::ptr_eq(&remapped.children[0], &remapped.children[1]));
    }
}
