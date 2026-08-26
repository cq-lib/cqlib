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
    DevicePlanningSessionCache, NativePlanAvailability, SelectedNativePlan, own_selected_plan,
    retain_selected_dependencies,
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
        if let Some(plan) = planner.selected_plan_for(&representative) {
            let representative_selected = own_selected_plan(&planner, plan, &mut owned_memo)
                .map_err(|error| error.to_string())?;
            for member in members {
                let selected = if member.ordered_qargs == representative.ordered_qargs {
                    Arc::clone(&representative_selected)
                } else {
                    let mut remap_memo = HashMap::new();
                    let selected = remap_selected_plan(
                        &representative_selected,
                        &representative.ordered_qargs,
                        &member.ordered_qargs,
                        &mut remap_memo,
                    )?;
                    retain_selected_dependencies(cache, remap_memo.into_values());
                    selected
                };
                cache.availability.insert(
                    member.clone(),
                    NativePlanAvailability::Feasible(Arc::clone(&selected.summary)),
                );
                cache.selected_plans.insert(member, selected);
            }
        } else {
            let failure = planner.failure_for(&representative);
            for member in members {
                cache.availability.insert(
                    member.clone(),
                    NativePlanAvailability::Unsupported(Arc::new(remap_failure(
                        &failure,
                        &representative.ordered_qargs,
                        &member.ordered_qargs,
                    )?)),
                );
            }
        }
    }
    retain_selected_dependencies(cache, owned_memo.into_values());
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

        if let Some(plan) = planner.selected_plan_for(representative) {
            let mut owned_memo = HashMap::new();
            let representative_selected = own_selected_plan(&planner, plan, &mut owned_memo)
                .map_err(|error| error.to_string())?;
            retain_selected_dependencies(cache, owned_memo.into_values());

            for member in members {
                let member_pair = [member.ordered_qargs[0], member.ordered_qargs[1]];
                let selected = if member_pair == representative_pair {
                    Arc::clone(&representative_selected)
                } else {
                    let mut remap_memo = HashMap::new();
                    let selected = remap_selected_plan(
                        &representative_selected,
                        &representative_pair,
                        &member_pair,
                        &mut remap_memo,
                    )?;
                    retain_selected_dependencies(cache, remap_memo.into_values());
                    selected
                };
                cache.availability.insert(
                    member.clone(),
                    NativePlanAvailability::Feasible(Arc::clone(&selected.summary)),
                );
                cache.selected_plans.insert(member, selected);
            }
        } else {
            let failure = planner.failure_for(representative);
            for member in members {
                let member_pair = [member.ordered_qargs[0], member.ordered_qargs[1]];
                cache.availability.insert(
                    member,
                    NativePlanAvailability::Unsupported(Arc::new(remap_failure(
                        &failure,
                        &representative_pair,
                        &member_pair,
                    )?)),
                );
            }
        }
    }
    Ok(equivalence_classes)
}

fn remap_selected_plan(
    selected: &Arc<SelectedNativePlan>,
    from: &[PhysicalQubit],
    to: &[PhysicalQubit],
    memo: &mut HashMap<DeviceGateState, Arc<SelectedNativePlan>>,
) -> Result<Arc<SelectedNativePlan>, String> {
    if let Some(remapped) = memo.get(&selected.state) {
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
    memo.insert(selected.state.clone(), Arc::clone(&remapped));
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
