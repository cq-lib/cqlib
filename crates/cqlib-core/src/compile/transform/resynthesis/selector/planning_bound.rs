// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Admissible synthesis bounds used by bounded-block best-first selection.

use super::super::commutation::OperationView;
use super::super::config::TwoQubitBlockResynthesisConfig;
use super::super::synthesis_cache::{CachedPlanView, TwoQubitSynthesisCache};
use super::{PatchPriority, PreparedBlock, block_matrix};
use crate::compile::CompilerError;
use crate::compile::transform::decompose::unitary::device_synthesis::OrderedPairDomain;
use crate::compile::transform::decompose::unitary::unitary_2q::{
    DeviceTwoQubitSynthesisCandidate, TwoQubitSynthesisCandidate, minimum_constructive_entanglers,
    plan_cx_family_fallbacks_for_device_from_kak, plan_cx_family_fallbacks_from_kak,
    plan_numeric_2q_unitary_for_device_from_kak, plan_numeric_2q_unitary_from_kak,
    plan_pauli_fallback_for_device_from_kak, plan_pauli_fallback_from_kak,
};
use crate::compile::transform::decompose::unitary::{
    DeviceContextCostFailure, DevicePhysicalCost, DeviceSynthesisPlacement,
    DeviceTwoQubitSynthesisContext, TwoQubitSynthesisRequest, TwoQubitUnitaryDecomposeBasis,
};
use ndarray::Array2;
use num_complex::Complex64;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug)]
pub(super) enum BlockPriorityBound {
    /// A priority no worse than any valid patch this block can realize.
    /// Comparing a winner against this value is therefore safe for pruning.
    Known(PatchPriority),
    /// Planning proved that this block cannot produce an improving patch.
    Impossible,
    /// Planning could not preserve the bound invariant. Callers must use the
    /// zero-cost fallback and fully validate the block when it remains live.
    Unknown,
}

fn priority_for_candidate(
    block: &PreparedBlock,
    after_cost: super::super::cost::ResynthesisCost,
    device_after_cost: Option<DevicePhysicalCost>,
) -> PatchPriority {
    PatchPriority {
        device_after_cost,
        after_cost,
        reduction: block
            .facts
            .source_cost
            .lowered_two_qubit_ops
            .saturating_sub(after_cost.lowered_two_qubit_ops),
        matched_orders: block.block.matched_orders.len(),
        first_order: block.block.first_order(),
        selection_rank: block.selection_rank,
    }
}

/// Computes the first useful bound without constructing synthesis candidates.
///
/// A non-local generated interaction must lower to at least one native
/// two-qubit operation. The remaining cost fields use their best possible
/// values, so the result can underestimate a real patch but can never
/// overestimate it.
pub(super) fn estimate_block_priority_bound(
    block: &PreparedBlock,
    ops: &[OperationView<'_>],
    config: &TwoQubitBlockResynthesisConfig,
    device_context: Option<&DeviceTwoQubitSynthesisContext>,
    synthesis_cache: &mut TwoQubitSynthesisCache,
) -> Result<BlockPriorityBound, CompilerError> {
    if block.facts.matrix.get().is_none() {
        let stats = synthesis_cache.selection_stats_mut();
        stats.matrix_attempts = stats.matrix_attempts.saturating_add(1);
    }
    let Some(matrix) = block
        .facts
        .matrix
        .get_or_init(|| block_matrix(&block.block, ops).ok())
        .as_ref()
    else {
        return Ok(BlockPriorityBound::Impossible);
    };
    let Some(decomp) = synthesis_cache.kak_decomposition(matrix)? else {
        return Ok(BlockPriorityBound::Impossible);
    };

    let minimum_entanglers = if let Some(context) = device_context {
        let mut native_two_qubit_gates = context
            .native_two_qubit_backends(block.block.qubits)
            .into_iter()
            .collect::<Vec<_>>();
        native_two_qubit_gates.sort_by_key(|gate| *gate as u8);
        minimum_constructive_entanglers(&decomp, &native_two_qubit_gates, true)
    } else {
        minimum_constructive_entanglers(
            &decomp,
            config.two_qubit_target.native_2q(),
            config.two_qubit_target.fallback_pauli(),
        )
    };
    let Some(minimum_entanglers) = minimum_entanglers else {
        return Ok(BlockPriorityBound::Impossible);
    };

    let after_cost = super::super::cost::ResynthesisCost {
        lowered_two_qubit_ops: minimum_entanglers,
        lowered_depth: minimum_entanglers,
        lowered_total_ops: minimum_entanglers,
        parameterized_ops: 0,
        backend_order: 0,
    };
    let device_after_cost = device_context.map(|_| {
        DevicePhysicalCost::optimistic_entangler_lower_bound(
            u32::try_from(minimum_entanglers).expect("two-qubit KAK interaction count fits in u32"),
        )
    });
    Ok(BlockPriorityBound::Known(priority_for_candidate(
        block,
        after_cost,
        device_after_cost,
    )))
}

pub(super) fn plan_block_priority_bound(
    block: &PreparedBlock,
    ops: &[OperationView<'_>],
    config: &TwoQubitBlockResynthesisConfig,
    device_context: Option<&DeviceTwoQubitSynthesisContext>,
    synthesis_cache: &mut TwoQubitSynthesisCache,
) -> Result<BlockPriorityBound, CompilerError> {
    if block.facts.matrix.get().is_none() {
        let stats = synthesis_cache.selection_stats_mut();
        stats.matrix_attempts = stats.matrix_attempts.saturating_add(1);
    }
    let Some(matrix) = block
        .facts
        .matrix
        .get_or_init(|| block_matrix(&block.block, ops).ok())
        .as_ref()
    else {
        return Ok(BlockPriorityBound::Impossible);
    };
    if let Some(context) = device_context {
        plan_device_block_priority_bound(block, matrix, context, synthesis_cache)
    } else {
        plan_generic_block_priority_bound(block, matrix, config, synthesis_cache)
    }
}

fn plan_generic_block_priority_bound(
    block: &PreparedBlock,
    matrix: &Array2<Complex64>,
    config: &TwoQubitBlockResynthesisConfig,
    synthesis_cache: &mut TwoQubitSynthesisCache,
) -> Result<BlockPriorityBound, CompilerError> {
    let Some(mut candidates) = synthesis_cache.with_generic_plan(
        matrix,
        block.block.qubits,
        |cache| {
            let decomp = cache.kak_decomposition(matrix)?.ok_or_else(|| {
                CompilerError::InvariantViolation(
                    "cached KAK decomposition failed for resynthesis matrix".to_string(),
                )
            })?;
            plan_numeric_2q_unitary_from_kak(
                TwoQubitSynthesisRequest {
                    matrix,
                    qubits: block.block.qubits,
                    target: config.two_qubit_target.clone(),
                },
                &decomp,
            )
        },
        |plan| match plan {
            CachedPlanView::Candidates(candidates) => Some(candidates.to_vec()),
            CachedPlanView::Failed => None,
        },
    )?
    else {
        return Ok(BlockPriorityBound::Impossible);
    };

    let Some(decomp) = synthesis_cache.kak_decomposition(matrix)? else {
        return Ok(BlockPriorityBound::Unknown);
    };
    let mut fallback_backends = Vec::new();
    let mut seen_backends = HashSet::new();
    for candidate in &candidates {
        if matches!(
            candidate.backend,
            TwoQubitUnitaryDecomposeBasis::Cx
                | TwoQubitUnitaryDecomposeBasis::Cy
                | TwoQubitUnitaryDecomposeBasis::Cz
        ) && seen_backends.insert(candidate.backend)
        {
            fallback_backends.push(candidate.backend);
        }
    }
    for backend in fallback_backends {
        let Ok(fallbacks) = plan_cx_family_fallbacks_from_kak(
            TwoQubitSynthesisRequest {
                matrix,
                qubits: block.block.qubits,
                target: config.two_qubit_target.clone(),
            },
            &decomp,
            backend,
        ) else {
            return Ok(BlockPriorityBound::Unknown);
        };
        candidates.extend(fallbacks);
    }
    if !candidates
        .iter()
        .any(|candidate| candidate.backend == TwoQubitUnitaryDecomposeBasis::PauliRotations)
    {
        let Ok(fallbacks) = plan_pauli_fallback_from_kak(
            TwoQubitSynthesisRequest {
                matrix,
                qubits: block.block.qubits,
                target: config.two_qubit_target.clone(),
            },
            &decomp,
        ) else {
            return Ok(BlockPriorityBound::Unknown);
        };
        candidates.extend(fallbacks);
    }

    Ok(best_generic_bound(block, &candidates))
}

fn best_generic_bound(
    block: &PreparedBlock,
    candidates: &[TwoQubitSynthesisCandidate],
) -> BlockPriorityBound {
    candidates
        .iter()
        .filter(|candidate| candidate.cost < block.facts.source_cost)
        .map(|candidate| priority_for_candidate(block, candidate.cost, None))
        .min()
        .map_or(BlockPriorityBound::Impossible, BlockPriorityBound::Known)
}

fn plan_device_block_priority_bound(
    block: &PreparedBlock,
    matrix: &Array2<Complex64>,
    context: &DeviceTwoQubitSynthesisContext,
    synthesis_cache: &mut TwoQubitSynthesisCache,
) -> Result<BlockPriorityBound, CompilerError> {
    let (device_before_cost, source_domain) = match context.placement() {
        DeviceSynthesisPlacement::PreLayoutEnvelope => {
            let Some(evaluation) =
                context.evaluate_pre_layout(&block.facts.source_operations, block.block.qubits)
            else {
                return Ok(BlockPriorityBound::Impossible);
            };
            (evaluation.worst_cost, Some(evaluation.domain))
        }
        DeviceSynthesisPlacement::ExactPhysical => {
            match context.exact_cost_diagnostic(&block.facts.source_operations, block.block.qubits)
            {
                Ok(cost) => (cost, None),
                Err(DeviceContextCostFailure::Unsupported(_)) => {
                    return Ok(BlockPriorityBound::Impossible);
                }
                Err(DeviceContextCostFailure::Unprepared(state)) => {
                    return Err(CompilerError::InvariantViolation(format!(
                        "device resynthesis context was not prepared for source state {state:?}"
                    )));
                }
                Err(DeviceContextCostFailure::WrongPlacement) => {
                    return Err(CompilerError::InvariantViolation(
                        "exact device resynthesis used a pre-layout context".to_string(),
                    ));
                }
                Err(DeviceContextCostFailure::InvalidOperation(reason)) => {
                    return Err(CompilerError::InvariantViolation(format!(
                        "invalid exact-device resynthesis source: {reason}"
                    )));
                }
            }
        }
    };
    let Some(mut candidates) = synthesis_cache.with_device_plan(
        matrix,
        block.block.qubits,
        context.placement(),
        |cache| {
            let decomp = cache.kak_decomposition(matrix)?.ok_or_else(|| {
                CompilerError::InvariantViolation(
                    "cached KAK decomposition failed for device resynthesis matrix".to_string(),
                )
            })?;
            plan_numeric_2q_unitary_for_device_from_kak(
                matrix,
                block.block.qubits,
                context,
                &decomp,
            )
        },
        |plan| match plan {
            CachedPlanView::Candidates(candidates) => Some(candidates.to_vec()),
            CachedPlanView::Failed => None,
        },
    )?
    else {
        return Ok(BlockPriorityBound::Impossible);
    };

    let Some(decomp) = synthesis_cache.kak_decomposition(matrix)? else {
        return Ok(BlockPriorityBound::Unknown);
    };
    let mut fallback_families = Vec::new();
    let mut seen_families = HashSet::new();
    for candidate in &candidates {
        let family = (candidate.candidate.backend, candidate.direction_order);
        if matches!(
            candidate.candidate.backend,
            TwoQubitUnitaryDecomposeBasis::Cx
                | TwoQubitUnitaryDecomposeBasis::Cy
                | TwoQubitUnitaryDecomposeBasis::Cz
        ) && seen_families.insert(family)
        {
            fallback_families.push(family);
        }
    }
    for (backend, direction_order) in fallback_families {
        let Ok(fallbacks) = plan_cx_family_fallbacks_for_device_from_kak(
            matrix,
            block.block.qubits,
            context,
            &decomp,
            backend,
            direction_order,
        ) else {
            return Ok(BlockPriorityBound::Unknown);
        };
        candidates.extend(fallbacks);
    }
    if !candidates.iter().any(|candidate| {
        candidate.candidate.backend == TwoQubitUnitaryDecomposeBasis::PauliRotations
    }) {
        let Ok(fallbacks) =
            plan_pauli_fallback_for_device_from_kak(matrix, block.block.qubits, context, &decomp)
        else {
            return Ok(BlockPriorityBound::Unknown);
        };
        candidates.extend(fallbacks);
    }

    Ok(best_device_bound(
        block,
        &candidates,
        context.placement(),
        device_before_cost,
        source_domain.as_ref(),
    ))
}

fn best_device_bound(
    block: &PreparedBlock,
    candidates: &[DeviceTwoQubitSynthesisCandidate],
    placement: DeviceSynthesisPlacement,
    device_before_cost: DevicePhysicalCost,
    source_domain: Option<&OrderedPairDomain>,
) -> BlockPriorityBound {
    candidates
        .iter()
        .filter_map(|candidate| {
            let device_after_cost = match placement {
                DeviceSynthesisPlacement::PreLayoutEnvelope => {
                    let source_domain = source_domain?;
                    let evaluation = candidate.pre_layout.as_ref()?;
                    if !source_domain.is_subset(&evaluation.domain) {
                        return None;
                    }
                    evaluation.worst_cost_on_domain(source_domain)?
                }
                DeviceSynthesisPlacement::ExactPhysical => candidate.physical_cost,
            };
            device_after_cost
                .strictly_better_than(device_before_cost)
                .then(|| {
                    priority_for_candidate(block, candidate.candidate.cost, Some(device_after_cost))
                })
        })
        .min()
        .map_or(BlockPriorityBound::Impossible, BlockPriorityBound::Known)
}
