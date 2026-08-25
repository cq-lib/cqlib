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

//! Candidate synthesis and selection for two-qubit resynthesis.
//!
//! Selection has two stages. First, syntactically duplicate blocks are removed
//! and promising blocks are synthesized in a deterministic priority order.
//! Second, accepted patches are greedily filtered so no two selected patches
//! rewrite the same source operation.

use super::collector::TwoQubitNumericBlock;
use super::commutation::{CachedCommutation, OperationView};
use super::config::TwoQubitBlockResynthesisConfig;
use super::cost::{
    ResynthesisCost, cost_of_source_value_operations, value_operations_of_source_ops,
};
use super::synthesis_cache::{CachedBlockFacts, CachedPlanView, TwoQubitSynthesisCache};
use crate::circuit::{
    Instruction, ParameterValue, Qubit, StandardGate, ValueInstruction, ValueOperation,
    value_operations_to_matrix,
};
use crate::compile::transform::decompose::unitary::unitary_2q::{
    TwoQubitMatrixOp, plan_numeric_2q_unitary_for_device,
    plan_numeric_2q_unitary_for_device_from_kak, plan_numeric_2q_unitary_from_kak,
    two_qubit_operation_matrix_product,
};
use crate::compile::transform::decompose::unitary::{
    DeviceContextCostFailure, DevicePhysicalCost, DeviceSynthesisPlacement,
    DeviceTwoQubitSynthesisContext, TwoQubitSynthesisRequest, plan_numeric_2q_unitary,
};
use crate::compile::{CompilerError, compare_some_first_by};
use ndarray::Array2;
use num_complex::Complex64;
use std::cmp::{Ordering, Reverse};
use std::collections::HashSet;

const MAX_PATCH_VALIDATION_QUBITS: usize = 6;
const PATCH_VALIDATION_TOLERANCE: f64 = 1e-8;

#[derive(Debug, Clone)]
pub(crate) struct BlockPatch {
    /// Source order where the synthesized replacement is emitted.
    pub first_order: usize,
    /// Source operations consumed by this patch.
    pub matched_orders: Vec<usize>,
    /// Source operations preserved in place after post-synthesis commutation
    /// verification.
    pub crossed_orders: Vec<usize>,
    pub replacement: Vec<ValueOperation>,
    pub before_cost: ResynthesisCost,
    pub after_cost: ResynthesisCost,
    pub device_after_cost: Option<DevicePhysicalCost>,
    pub synthesis_phase: f64,
}

#[cfg(test)]
pub(crate) fn select_patches(
    blocks: Vec<TwoQubitNumericBlock>,
    ops: &[OperationView<'_>],
    commutation: &CachedCommutation,
    config: &TwoQubitBlockResynthesisConfig,
) -> Result<Vec<BlockPatch>, CompilerError> {
    select_patches_with_device(
        blocks,
        ops,
        commutation,
        config,
        None,
        &mut TwoQubitSynthesisCache::default(),
    )
}

pub(crate) fn select_patches_with_device(
    blocks: Vec<TwoQubitNumericBlock>,
    ops: &[OperationView<'_>],
    commutation: &CachedCommutation,
    config: &TwoQubitBlockResynthesisConfig,
    device_context: Option<&DeviceTwoQubitSynthesisContext>,
    synthesis_cache: &mut TwoQubitSynthesisCache,
) -> Result<Vec<BlockPatch>, CompilerError> {
    let mut seen = HashSet::new();
    let mut blocks = blocks
        .into_iter()
        .filter(|block| seen.insert(block.matched_orders.clone()))
        .collect::<Vec<_>>();
    blocks.sort_by(compare_blocks);

    let mut patches = Vec::new();
    for block in blocks {
        if synthesis_cache.has_terminal_no_patch(&block, ops) {
            continue;
        }
        let patch = try_synthesize_block(
            &block,
            ops,
            commutation,
            config,
            device_context,
            synthesis_cache,
        )?;
        if let Some(patch) = patch {
            patches.push(patch);
        } else {
            synthesis_cache.record_terminal_no_patch(&block, ops);
        }
    }
    patches.sort_by(compare_patches);

    let mut covered = HashSet::new();
    let mut selected = Vec::new();
    for patch in patches {
        if patch
            .matched_orders
            .iter()
            .any(|order| covered.contains(order))
        {
            continue;
        }
        for order in &patch.matched_orders {
            covered.insert(*order);
        }
        selected.push(patch);
    }
    selected.sort_by_key(|patch| patch.first_order);
    Ok(selected)
}

fn compare_blocks(lhs: &TwoQubitNumericBlock, rhs: &TwoQubitNumericBlock) -> Ordering {
    // Synthesis is the expensive step. Prefer candidates most likely to improve:
    // more 2q gates, larger matched unitary, denser span, SWAP involvement, then
    // smaller source span and earlier deterministic position.
    let lhs_density = lhs.matched_orders.len() * rhs.span();
    let rhs_density = rhs.matched_orders.len() * lhs.span();
    Reverse(lhs.matched_2q_count)
        .cmp(&Reverse(rhs.matched_2q_count))
        .then_with(|| Reverse(lhs.matched_orders.len()).cmp(&Reverse(rhs.matched_orders.len())))
        .then_with(|| Reverse(lhs_density).cmp(&Reverse(rhs_density)))
        .then_with(|| Reverse(lhs.contains_swap).cmp(&Reverse(rhs.contains_swap)))
        .then_with(|| lhs.span().cmp(&rhs.span()))
        .then_with(|| lhs.first_order().cmp(&rhs.first_order()))
}

fn compare_patches(lhs: &BlockPatch, rhs: &BlockPatch) -> Ordering {
    let lhs_reduction = lhs
        .before_cost
        .lowered_two_qubit_ops
        .saturating_sub(lhs.after_cost.lowered_two_qubit_ops);
    let rhs_reduction = rhs
        .before_cost
        .lowered_two_qubit_ops
        .saturating_sub(rhs.after_cost.lowered_two_qubit_ops);

    let physical = compare_some_first_by(
        lhs.device_after_cost,
        rhs.device_after_cost,
        |left, right| left.compare(right),
    );

    physical
        .then_with(|| lhs.after_cost.cmp(&rhs.after_cost))
        .then_with(|| rhs_reduction.cmp(&lhs_reduction))
        .then_with(|| Reverse(lhs.matched_orders.len()).cmp(&Reverse(rhs.matched_orders.len())))
        .then_with(|| lhs.first_order.cmp(&rhs.first_order))
}

fn try_synthesize_block(
    block: &TwoQubitNumericBlock,
    ops: &[OperationView<'_>],
    commutation: &CachedCommutation,
    config: &TwoQubitBlockResynthesisConfig,
    device_context: Option<&DeviceTwoQubitSynthesisContext>,
    synthesis_cache: &mut TwoQubitSynthesisCache,
) -> Result<Option<BlockPatch>, CompilerError> {
    let matched = block
        .matched_orders
        .iter()
        .map(|&order| &ops[order])
        .collect::<Vec<_>>();
    let Some(facts) = synthesis_cache.block_facts(block, ops, || {
        let source_operations = value_operations_of_source_ops(&matched)?;
        let source_cost =
            cost_of_source_value_operations(&source_operations, &config.two_qubit_target)?;
        Ok(CachedBlockFacts {
            matrix: block_matrix(block, ops)?,
            source_operations,
            source_cost,
        })
    })?
    else {
        return Ok(None);
    };
    let matrix = &facts.matrix;
    let before_cost = facts.source_cost;

    let crossed = block
        .crossed_orders
        .iter()
        .map(|&order| &ops[order])
        .collect::<Vec<_>>();
    if let Some(device_context) = device_context {
        return try_synthesize_device_block(
            block,
            matrix,
            ops,
            &facts.source_operations,
            &crossed,
            commutation,
            device_context,
            before_cost,
            synthesis_cache,
        );
    }
    synthesis_cache.with_generic_plan(
        matrix,
        block.qubits,
        |cache| {
            if !cache.artifact_reuse_enabled() {
                return plan_numeric_2q_unitary(TwoQubitSynthesisRequest {
                    matrix,
                    qubits: block.qubits,
                    target: config.two_qubit_target.clone(),
                });
            }
            let decomp = cache.kak_decomposition(matrix)?.ok_or_else(|| {
                CompilerError::InvariantViolation(
                    "cached KAK decomposition failed for resynthesis matrix".to_string(),
                )
            })?;
            plan_numeric_2q_unitary_from_kak(
                TwoQubitSynthesisRequest {
                    matrix,
                    qubits: block.qubits,
                    target: config.two_qubit_target.clone(),
                },
                &decomp,
            )
        },
        |plan| {
            let CachedPlanView::Candidates(candidates) = plan else {
                return Ok(None);
            };
            let mut span_validation = None;
            for candidate in candidates {
                if candidate.cost >= before_cost {
                    continue;
                }
                if !commutation.replacements_commute_with_crossed(&crossed, &candidate.operations) {
                    continue;
                }
                if !patch_preserves_relevant_span_cached(
                    block,
                    ops,
                    &candidate.operations,
                    candidate.global_phase,
                    &mut span_validation,
                )? {
                    continue;
                }

                return Ok(Some(BlockPatch {
                    first_order: block.first_order(),
                    matched_orders: block.matched_orders.clone(),
                    crossed_orders: block.crossed_orders.clone(),
                    replacement: candidate.operations.clone(),
                    before_cost,
                    after_cost: candidate.cost,
                    device_after_cost: None,
                    synthesis_phase: candidate.global_phase,
                }));
            }
            Ok(None)
        },
    )?
}

#[allow(clippy::too_many_arguments)]
fn try_synthesize_device_block(
    block: &TwoQubitNumericBlock,
    matrix: &Array2<Complex64>,
    ops: &[OperationView<'_>],
    source_operations: &[ValueOperation],
    crossed: &[&OperationView<'_>],
    commutation: &CachedCommutation,
    context: &DeviceTwoQubitSynthesisContext,
    before_cost: ResynthesisCost,
    synthesis_cache: &mut TwoQubitSynthesisCache,
) -> Result<Option<BlockPatch>, CompilerError> {
    let (device_before_cost, source_domain) = match context.placement() {
        DeviceSynthesisPlacement::PreLayoutEnvelope => {
            let Some(evaluation) = context.evaluate_pre_layout(source_operations, block.qubits)
            else {
                return Ok(None);
            };
            (evaluation.worst_cost, Some(evaluation.domain))
        }
        DeviceSynthesisPlacement::ExactPhysical => {
            match context.exact_cost_diagnostic(source_operations, block.qubits) {
                Ok(cost) => (cost, None),
                Err(DeviceContextCostFailure::Unsupported(_)) => return Ok(None),
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
    synthesis_cache.with_device_plan(
        matrix,
        block.qubits,
        |cache| {
            if !cache.artifact_reuse_enabled() {
                return plan_numeric_2q_unitary_for_device(matrix, block.qubits, context);
            }
            let decomp = cache.kak_decomposition(matrix)?.ok_or_else(|| {
                CompilerError::InvariantViolation(
                    "cached KAK decomposition failed for device resynthesis matrix".to_string(),
                )
            })?;
            let swap = StandardGate::SWAP
                .matrix(&[])
                .map_err(CompilerError::Circuit)?
                .into_owned();
            let reversed_matrix = swap.dot(matrix).dot(&swap);
            let reversed_decomp = cache.kak_decomposition(&reversed_matrix)?.ok_or_else(|| {
                CompilerError::InvariantViolation(
                    "cached KAK decomposition failed for reversed device matrix".to_string(),
                )
            })?;
            plan_numeric_2q_unitary_for_device_from_kak(
                matrix,
                block.qubits,
                context,
                &decomp,
                &reversed_matrix,
                &reversed_decomp,
            )
        },
        |plan| {
            let CachedPlanView::Candidates(candidates) = plan else {
                return Ok(None);
            };
            let mut viable = Vec::new();
            for candidate in candidates {
                let device_after_cost = match context.placement() {
                    DeviceSynthesisPlacement::PreLayoutEnvelope => {
                        let Some(source_domain) = source_domain.as_ref() else {
                            continue;
                        };
                        let Some(evaluation) = candidate.pre_layout.as_ref() else {
                            continue;
                        };
                        if !source_domain.is_subset(&evaluation.domain) {
                            continue;
                        }
                        let Some(cost) = evaluation.worst_cost_on_domain(source_domain) else {
                            continue;
                        };
                        cost
                    }
                    DeviceSynthesisPlacement::ExactPhysical => candidate.physical_cost,
                };
                if !device_after_cost.strictly_better_than(device_before_cost) {
                    continue;
                }
                viable.push((candidate, device_after_cost));
            }
            viable.sort_by(|(left, left_cost), (right, right_cost)| {
                left_cost
                    .compare(*right_cost)
                    .then_with(|| left.candidate.cost.cmp(&right.candidate.cost))
            });
            let mut span_validation = None;
            for (candidate, device_after_cost) in viable {
                if !commutation
                    .replacements_commute_with_crossed(crossed, &candidate.candidate.operations)
                {
                    continue;
                }
                if !patch_preserves_relevant_span_cached(
                    block,
                    ops,
                    &candidate.candidate.operations,
                    candidate.candidate.global_phase,
                    &mut span_validation,
                )? {
                    continue;
                }
                return Ok(Some(BlockPatch {
                    first_order: block.first_order(),
                    matched_orders: block.matched_orders.clone(),
                    crossed_orders: block.crossed_orders.clone(),
                    replacement: candidate.candidate.operations.clone(),
                    before_cost,
                    after_cost: candidate.candidate.cost,
                    device_after_cost: Some(device_after_cost),
                    synthesis_phase: candidate.candidate.global_phase,
                }));
            }
            Ok(None)
        },
    )?
}

// Matrix construction uses the same convention as `circuit_to_matrix`: source
// operations are multiplied as `gate_n * ... * gate_0`, where `gate_0` is the
// earliest source operation. `block.qubits[0]` is the first tensor factor.
fn block_matrix(
    block: &TwoQubitNumericBlock,
    ops: &[OperationView<'_>],
) -> Result<Array2<Complex64>, CompilerError> {
    let mut orders = block.matched_orders.clone();
    orders.sort_unstable();
    let mut resolved = Vec::with_capacity(orders.len());
    for order in orders {
        let view = &ops[order];
        let Instruction::Standard(gate) = view.operation.instruction else {
            return Err(CompilerError::InvariantViolation(
                "resynthesis matrix requested for non-standard operation".to_string(),
            ));
        };
        let params = view
            .params
            .iter()
            .map(|param| param.evaluate(&None))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                CompilerError::InvariantViolation(
                    "resynthesis matrix requested for symbolic operation".to_string(),
                )
            })?;
        resolved.push(TwoQubitMatrixOp {
            gate,
            qubits: view.operation.qubits.iter().copied().collect(),
            params,
        });
    }
    two_qubit_operation_matrix_product(
        &resolved,
        0.0,
        block.qubits,
        "resynthesis block contains operation outside canonical qubits",
    )
}

struct RelevantSpanValidation {
    qubits: Vec<Qubit>,
    source_matrix: Array2<Complex64>,
    converted: Vec<Option<ValueOperation>>,
    included_orders: HashSet<usize>,
    matched_orders: HashSet<usize>,
    span_start: usize,
    span_end: usize,
}

#[cfg(test)]
fn patch_preserves_relevant_span(
    block: &TwoQubitNumericBlock,
    ops: &[OperationView<'_>],
    replacement: &[ValueOperation],
    synthesis_phase: f64,
) -> Result<bool, CompilerError> {
    patch_preserves_relevant_span_cached(block, ops, replacement, synthesis_phase, &mut None)
}

fn patch_preserves_relevant_span_cached(
    block: &TwoQubitNumericBlock,
    ops: &[OperationView<'_>],
    replacement: &[ValueOperation],
    synthesis_phase: f64,
    prepared: &mut Option<RelevantSpanValidation>,
) -> Result<bool, CompilerError> {
    if prepared.is_none() {
        *prepared = prepare_relevant_span(block, ops)?;
    }
    let Some(prepared) = prepared else {
        return Ok(false);
    };
    if replacement
        .iter()
        .flat_map(|operation| operation.qubits.iter())
        .any(|qubit| !prepared.qubits.contains(qubit))
    {
        return Ok(false);
    }

    let mut replacement_ops = Vec::new();
    for (order, cached) in prepared
        .converted
        .iter()
        .enumerate()
        .take(prepared.span_end + 1)
        .skip(prepared.span_start)
    {
        if order == block.first_order() {
            replacement_ops.extend(replacement.iter().cloned());
        }
        if prepared.matched_orders.contains(&order) {
            continue;
        }
        if prepared.included_orders.contains(&order) {
            replacement_ops.push(
                cached
                    .as_ref()
                    .expect("included operation was converted")
                    .clone(),
            );
        }
    }
    let Ok(replacement_matrix) =
        value_operations_to_matrix(&prepared.qubits, &replacement_ops, synthesis_phase)
    else {
        return Ok(false);
    };
    Ok(prepared.source_matrix.shape() == replacement_matrix.shape()
        && prepared
            .source_matrix
            .iter()
            .zip(replacement_matrix.iter())
            .all(|(source, replacement)| {
                (*source - *replacement).norm() <= PATCH_VALIDATION_TOLERANCE
            }))
}

fn prepare_relevant_span(
    block: &TwoQubitNumericBlock,
    ops: &[OperationView<'_>],
) -> Result<Option<RelevantSpanValidation>, CompilerError> {
    let mut relevant_qubits = HashSet::new();
    let mut included_orders = block
        .matched_orders
        .iter()
        .chain(&block.crossed_orders)
        .copied()
        .collect::<HashSet<_>>();
    for &order in &included_orders {
        relevant_qubits.extend(ops[order].operation.qubits.iter().copied());
    }
    if relevant_qubits.len() > MAX_PATCH_VALIDATION_QUBITS {
        return Ok(None);
    }

    let Some(span_start) = included_orders.iter().min().copied() else {
        return Ok(None);
    };
    let Some(span_end) = included_orders.iter().max().copied() else {
        return Ok(None);
    };

    let mut changed = true;
    let mut converted = vec![None; ops.len()];
    while changed {
        changed = false;
        for (order, view) in ops.iter().enumerate().take(span_end + 1).skip(span_start) {
            if included_orders.contains(&order) {
                continue;
            }
            if !view.operation.qubits.is_empty()
                && !view
                    .operation
                    .qubits
                    .iter()
                    .any(|qubit| relevant_qubits.contains(qubit))
            {
                continue;
            }
            let Some(operation) = operation_view_to_value(view) else {
                return Ok(None);
            };
            converted[order] = Some(operation);
            included_orders.insert(order);
            relevant_qubits.extend(view.operation.qubits.iter().copied());
            if relevant_qubits.len() > MAX_PATCH_VALIDATION_QUBITS {
                return Ok(None);
            }
            changed = true;
        }
    }

    for &order in &included_orders {
        if converted[order].is_none() {
            let Some(operation) = operation_view_to_value(&ops[order]) else {
                return Ok(None);
            };
            converted[order] = Some(operation);
        }
    }

    let mut source_ops = Vec::new();
    let matched_orders = block.matched_orders.iter().copied().collect::<HashSet<_>>();
    for (order, cached) in converted
        .iter()
        .enumerate()
        .take(span_end + 1)
        .skip(span_start)
    {
        if included_orders.contains(&order) {
            source_ops.push(
                cached
                    .as_ref()
                    .expect("included operation was converted")
                    .clone(),
            );
        }
    }

    let mut qubits = relevant_qubits.into_iter().collect::<Vec<_>>();
    qubits.sort_by_key(|qubit| qubit.index());

    let Ok(source_matrix) = value_operations_to_matrix(&qubits, &source_ops, 0.0) else {
        return Ok(None);
    };
    Ok(Some(RelevantSpanValidation {
        qubits,
        source_matrix,
        converted,
        included_orders,
        matched_orders,
        span_start,
        span_end,
    }))
}

fn operation_view_to_value(view: &OperationView<'_>) -> Option<ValueOperation> {
    let Instruction::Standard(gate) = view.operation.instruction else {
        return None;
    };
    let params = view
        .params
        .iter()
        .map(|param| param.evaluate(&None))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if params.iter().any(|value| !value.is_finite()) {
        return None;
    }
    Some(ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(gate)),
        qubits: view.operation.qubits.clone(),
        params: params.into_iter().map(ParameterValue::Fixed).collect(),
        label: view.operation.label.clone(),
    })
}

#[cfg(test)]
#[path = "selector_test.rs"]
mod selector_test;
