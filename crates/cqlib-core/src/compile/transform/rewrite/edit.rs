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

//! Rewrite-local edit scripts and routing equivalence metadata.

use crate::circuit::{Circuit, CircuitParam, Operation, Parameter, Qubit};
use std::ops::Range;

/// One exact replacement in a flat operation sequence.
///
/// Ranges are expressed in their respective coordinate spaces: `old` indexes
/// the circuit presented to the transform and `new` indexes its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperationReplacement {
    pub(crate) old: Range<usize>,
    pub(crate) new: Range<usize>,
}

/// One qubit renaming which preserves rewrite observations inside a clean gap.
///
/// Pairs are sorted by the source qubit. Both sides must be unique; consumers
/// validate that every qarg used by the gap is covered by this bijection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QubitBijection {
    pub(crate) pairs: Vec<(Qubit, Qubit)>,
}

/// Exact rewrite edit script for one transform invocation.
///
/// `Linear` promises that operations outside the ordered replacements are
/// exactly unchanged and occur in the same order. It preserves the qubit
/// domain. Circuit-wide phase is deliberately outside this contract because
/// knowledge-match anchors do not observe it.
///
/// `LinearModuloQubitBijection` is deliberately narrower than a general
/// edit script: it is a rewrite-equivalence certificate for routing.
/// Operations in each clean gap may rename qargs through the supplied local
/// bijection, while their instruction, label, resolved parameters, and order
/// remain unchanged. Consumers must both validate the mappings and establish
/// that the proof being advanced is invariant under qubit bijections.
///
/// Transforms which rebuild control flow or cannot prove either
/// correspondence must report `Unknown`. A circuit-wide phase change alone
/// does not invalidate this contract because rewrite anchors do not observe it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RewriteEdits {
    Linear {
        old_len: usize,
        new_len: usize,
        replacements: Vec<OperationReplacement>,
    },
    LinearModuloQubitBijection {
        old_len: usize,
        new_len: usize,
        replacements: Vec<OperationReplacement>,
        /// One mapping for every clean gap, including the prefix and suffix.
        /// Therefore the length is always `replacements.len() + 1`.
        clean_gap_bijections: Vec<QubitBijection>,
    },
    Unknown,
}

impl RewriteEdits {
    /// Derives a conservative exact edit script for two flat operation
    /// streams.  The bounded resynchronization window keeps the common case
    /// linear while still preserving multiple sparse edits.  If a transform
    /// moves a large span, that span is represented by one larger replacement;
    /// this may admit extra rewrite anchors but can never omit an affected one.
    pub(crate) fn between_linear_circuits(before: &Circuit, after: &Circuit) -> Self {
        if before.qubits() != after.qubits() {
            return Self::Unknown;
        }

        const RESYNC_WINDOW: usize = 64;
        const CONFIRMING_RUN: usize = 2;

        let old = before.operations();
        let new = after.operations();
        let mut old_end = old.len();
        let mut new_end = new.len();
        while old_end > 0
            && new_end > 0
            && operations_are_equivalent(before, &old[old_end - 1], after, &new[new_end - 1])
        {
            old_end -= 1;
            new_end -= 1;
        }

        let mut replacements = Vec::new();
        let mut old_cursor = 0usize;
        let mut new_cursor = 0usize;
        while old_cursor < old_end && new_cursor < new_end {
            if operations_are_equivalent(before, &old[old_cursor], after, &new[new_cursor]) {
                old_cursor += 1;
                new_cursor += 1;
                continue;
            }

            let old_limit = old_end.min(old_cursor.saturating_add(RESYNC_WINDOW + 1));
            let new_limit = new_end.min(new_cursor.saturating_add(RESYNC_WINDOW + 1));
            let mut best = None::<(usize, usize, usize)>;
            for candidate_old in old_cursor..old_limit {
                for candidate_new in new_cursor..new_limit {
                    if candidate_old == old_cursor && candidate_new == new_cursor {
                        continue;
                    }
                    if !operations_are_equivalent(
                        before,
                        &old[candidate_old],
                        after,
                        &new[candidate_new],
                    ) {
                        continue;
                    }
                    let available = (old_end - candidate_old).min(new_end - candidate_new);
                    let required = CONFIRMING_RUN.min(available);
                    if (0..required).any(|offset| {
                        !operations_are_equivalent(
                            before,
                            &old[candidate_old + offset],
                            after,
                            &new[candidate_new + offset],
                        )
                    }) {
                        continue;
                    }
                    let score = (candidate_old - old_cursor) + (candidate_new - new_cursor);
                    if best.is_none_or(|(best_score, best_old, best_new)| {
                        (score, candidate_old, candidate_new) < (best_score, best_old, best_new)
                    }) {
                        best = Some((score, candidate_old, candidate_new));
                    }
                }
            }

            let Some((_, next_old, next_new)) = best else {
                replacements.push(OperationReplacement {
                    old: old_cursor..old_end,
                    new: new_cursor..new_end,
                });
                old_cursor = old_end;
                new_cursor = new_end;
                break;
            };
            replacements.push(OperationReplacement {
                old: old_cursor..next_old,
                new: new_cursor..next_new,
            });
            old_cursor = next_old;
            new_cursor = next_new;
        }
        if old_cursor < old_end || new_cursor < new_end {
            replacements.push(OperationReplacement {
                old: old_cursor..old_end,
                new: new_cursor..new_end,
            });
        }

        Self::linear(old.len(), new.len(), replacements)
    }

    pub(crate) fn linear(
        old_len: usize,
        new_len: usize,
        replacements: Vec<OperationReplacement>,
    ) -> Self {
        Self::Linear {
            old_len,
            new_len,
            replacements,
        }
    }

    pub(crate) fn linear_modulo_qubit_bijection(
        old_len: usize,
        new_len: usize,
        replacements: Vec<OperationReplacement>,
        clean_gap_bijections: Vec<QubitBijection>,
    ) -> Self {
        Self::LinearModuloQubitBijection {
            old_len,
            new_len,
            replacements,
            clean_gap_bijections,
        }
    }

    /// Builds ordered replacement ranges for a routed operation stream.
    /// `Some(i)` identifies output provenance from source operation `i`;
    /// `None` marks an inserted or rebuilt operation. The longest increasing
    /// provenance subsequence is retained, so independent SABRE scheduling
    /// moves become bounded replacements instead of invalidating the whole
    /// route. The SABRE adapter subsequently attaches and validates the qubit
    /// bijections for the retained gaps.
    pub(crate) fn from_route_provenance(old_len: usize, provenance: &[Option<usize>]) -> Self {
        Self::from_operation_provenance(old_len, provenance)
    }

    /// Builds an exact edit script from output-to-input operation provenance.
    /// Every retained source index must occur at most once; `None` denotes a
    /// newly generated operation. The increasing retained subsequence defines
    /// clean gaps, while moved or replaced spans are conservatively grouped as
    /// replacements.
    pub(crate) fn from_operation_provenance(old_len: usize, provenance: &[Option<usize>]) -> Self {
        let mut seen = vec![false; old_len];
        let mut candidates = Vec::<(usize, usize)>::new();
        for (new_order, source_order) in provenance.iter().copied().enumerate() {
            let Some(source_order) = source_order else {
                continue;
            };
            if source_order >= old_len || seen[source_order] {
                return Self::Unknown;
            }
            seen[source_order] = true;
            candidates.push((source_order, new_order));
        }

        let kept = longest_increasing_source_subsequence(&candidates);
        let mut replacements = Vec::new();
        let mut old_cursor = 0usize;
        let mut new_cursor = 0usize;
        for (old_order, new_order) in kept {
            if old_cursor < old_order || new_cursor < new_order {
                replacements.push(OperationReplacement {
                    old: old_cursor..old_order,
                    new: new_cursor..new_order,
                });
            }
            old_cursor = old_order + 1;
            new_cursor = new_order + 1;
        }
        if old_cursor < old_len || new_cursor < provenance.len() {
            replacements.push(OperationReplacement {
                old: old_cursor..old_len,
                new: new_cursor..provenance.len(),
            });
        }
        Self::linear(old_len, provenance.len(), replacements)
    }
}

pub(crate) fn operations_are_equivalent(
    before: &Circuit,
    source: &Operation,
    after: &Circuit,
    target: &Operation,
) -> bool {
    source.instruction == target.instruction
        && source.label == target.label
        && source.qubits == target.qubits
        && source.params.len() == target.params.len()
        && source
            .params
            .iter()
            .zip(&target.params)
            .all(|(source_parameter, target_parameter)| {
                resolve_operation_param(before, source_parameter)
                    == resolve_operation_param(after, target_parameter)
            })
}

fn resolve_operation_param(circuit: &Circuit, parameter: &CircuitParam) -> Option<Parameter> {
    match parameter {
        CircuitParam::Fixed(value) => Some(Parameter::from(*value)),
        CircuitParam::Index(index) => circuit.parameters().get_index(*index as usize).cloned(),
    }
}

fn longest_increasing_source_subsequence(candidates: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut tails = Vec::<usize>::new();
    let mut previous = vec![None; candidates.len()];
    for (candidate_index, &(source_order, _)) in candidates.iter().enumerate() {
        let position = tails.partition_point(|&tail_index| candidates[tail_index].0 < source_order);
        if position > 0 {
            previous[candidate_index] = Some(tails[position - 1]);
        }
        if position == tails.len() {
            tails.push(candidate_index);
        } else {
            tails[position] = candidate_index;
        }
    }
    let Some(mut cursor) = tails.last().copied() else {
        return Vec::new();
    };
    let mut kept = Vec::with_capacity(tails.len());
    loop {
        kept.push(candidates[cursor]);
        let Some(parent) = previous[cursor] else {
            break;
        };
        cursor = parent;
    }
    kept.reverse();
    kept
}

#[cfg(test)]
#[path = "edit_test.rs"]
mod edit_test;
