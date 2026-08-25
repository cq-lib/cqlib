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
                        representative_pair,
                        member_pair,
                        &mut remap_memo,
                    )?;
                    retain_selected_dependencies(cache, remap_memo.into_values());
                    selected
                };
                cache.availability.insert(
                    member.clone(),
                    NativePlanAvailability::Feasible(selected.summary.clone()),
                );
                cache.selected_plans.insert(member, selected);
            }
        } else {
            let failure = planner.failure_for(representative);
            for member in members {
                let member_pair = [member.ordered_qargs[0], member.ordered_qargs[1]];
                cache.availability.insert(
                    member,
                    NativePlanAvailability::Unsupported(remap_failure(
                        &failure,
                        representative_pair,
                        member_pair,
                    )?),
                );
            }
        }
    }
    Ok(equivalence_classes)
}

fn remap_selected_plan(
    selected: &Arc<SelectedNativePlan>,
    from: [PhysicalQubit; 2],
    to: [PhysicalQubit; 2],
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
    let summary = NativePlanSummary {
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
    };
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
    from: [PhysicalQubit; 2],
    to: [PhysicalQubit; 2],
) -> Result<DeviceGateState, String> {
    Ok(DeviceGateState {
        instruction: state.instruction.clone(),
        ordered_qargs: remap_small_qargs(&state.ordered_qargs, from, to)?,
    })
}

fn remap_small_qargs(
    qargs: &[PhysicalQubit],
    from: [PhysicalQubit; 2],
    to: [PhysicalQubit; 2],
) -> Result<SmallVec<[PhysicalQubit; 2]>, String> {
    qargs
        .iter()
        .copied()
        .map(|qubit| remap_qubit(qubit, from, to))
        .collect()
}

fn remap_vec_qargs(
    qargs: &[PhysicalQubit],
    from: [PhysicalQubit; 2],
    to: [PhysicalQubit; 2],
) -> Result<Vec<PhysicalQubit>, String> {
    qargs
        .iter()
        .copied()
        .map(|qubit| remap_qubit(qubit, from, to))
        .collect()
}

fn remap_qubit(
    qubit: PhysicalQubit,
    from: [PhysicalQubit; 2],
    to: [PhysicalQubit; 2],
) -> Result<PhysicalQubit, String> {
    if qubit == from[0] {
        Ok(to[0])
    } else if qubit == from[1] {
        Ok(to[1])
    } else {
        Err(format!(
            "selected SWAP plan contains qarg {qubit:?} outside representative pair {from:?}"
        ))
    }
}

fn remap_failure(
    failure: &DeviceLoweringFailure,
    from: [PhysicalQubit; 2],
    to: [PhysicalQubit; 2],
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
