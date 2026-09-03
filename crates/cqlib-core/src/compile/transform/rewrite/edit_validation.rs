// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of the License in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Validation and coordinate mapping for rewrite edit scripts.

use super::edit::{OperationReplacement, QubitBijection, operations_are_equivalent};
use crate::circuit::{Circuit, CircuitParam, Operation, Parameter};
use std::ops::Range;

pub(super) fn valid_exact_clean_gaps(
    before: &Circuit,
    after: &Circuit,
    replacements: &[OperationReplacement],
) -> bool {
    if !valid_replacements(
        before.operations().len(),
        after.operations().len(),
        replacements,
    ) {
        return false;
    }
    let mut old_cursor = 0usize;
    let mut new_cursor = 0usize;
    for replacement in replacements {
        if !exact_gap_is_unchanged(
            before,
            after,
            old_cursor..replacement.old.start,
            new_cursor..replacement.new.start,
        ) {
            return false;
        }
        old_cursor = replacement.old.end;
        new_cursor = replacement.new.end;
    }
    exact_gap_is_unchanged(
        before,
        after,
        old_cursor..before.operations().len(),
        new_cursor..after.operations().len(),
    )
}

fn exact_gap_is_unchanged(
    before: &Circuit,
    after: &Circuit,
    old: Range<usize>,
    new: Range<usize>,
) -> bool {
    old.len() == new.len()
        && old.zip(new).all(|(old_order, new_order)| {
            operations_are_equivalent(
                before,
                &before.operations()[old_order],
                after,
                &after.operations()[new_order],
            )
        })
}

pub(super) fn valid_replacements(
    old_len: usize,
    new_len: usize,
    replacements: &[OperationReplacement],
) -> bool {
    if replacements.is_empty() {
        return old_len == new_len;
    }
    let mut old_cursor = 0usize;
    let mut new_cursor = 0usize;
    for replacement in replacements {
        if replacement.old.start < old_cursor
            || replacement.old.start > replacement.old.end
            || replacement.new.start < new_cursor
            || replacement.new.start > replacement.new.end
            || replacement.old.end > old_len
            || replacement.new.end > new_len
            || replacement.old.start - old_cursor != replacement.new.start - new_cursor
        {
            return false;
        }
        old_cursor = replacement.old.end;
        new_cursor = replacement.new.end;
    }
    old_len - old_cursor == new_len - new_cursor
}

pub(super) fn map_and_invalidate_ranges(
    dirty: &[Range<usize>],
    replacements: &[OperationReplacement],
    old_len: usize,
    new_len: usize,
    proof_reach: usize,
) -> Vec<Range<usize>> {
    let mut ranges = Vec::with_capacity(dirty.len().saturating_add(replacements.len()));
    let mut old_cursor = 0usize;
    let mut new_cursor = 0usize;
    for replacement in replacements {
        map_dirty_segment(
            dirty,
            old_cursor..replacement.old.start,
            new_cursor,
            &mut ranges,
        );
        let start = replacement.new.start.saturating_sub(proof_reach);
        let end = replacement
            .new
            .end
            .max(replacement.new.start.saturating_add(1))
            .min(new_len);
        if start < end {
            ranges.push(start..end);
        } else if new_len > 0 {
            ranges.push(new_len.saturating_sub(proof_reach.saturating_add(1))..new_len);
        }
        old_cursor = replacement.old.end;
        new_cursor = replacement.new.end;
    }
    map_dirty_segment(dirty, old_cursor..old_len, new_cursor, &mut ranges);
    merge_ranges(&mut ranges);
    ranges
}

fn map_dirty_segment(
    dirty: &[Range<usize>],
    old_segment: Range<usize>,
    new_start: usize,
    output: &mut Vec<Range<usize>>,
) {
    for range in dirty {
        let start = range.start.max(old_segment.start);
        let end = range.end.min(old_segment.end);
        if start < end {
            output.push(
                new_start.saturating_add(start - old_segment.start)
                    ..new_start.saturating_add(end - old_segment.start),
            );
        }
    }
}

fn merge_ranges(ranges: &mut Vec<Range<usize>>) {
    ranges.sort_by_key(|range| (range.start, range.end));
    let mut merged = Vec::<Range<usize>>::with_capacity(ranges.len());
    for range in ranges.drain(..) {
        if let Some(previous) = merged.last_mut()
            && range.start <= previous.end
        {
            previous.end = previous.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    *ranges = merged;
}

pub(super) fn valid_clean_gap_bijections(
    before: &Circuit,
    after: &Circuit,
    replacements: &[OperationReplacement],
    bijections: &[QubitBijection],
) -> bool {
    let old_len = before.operations().len();
    let new_len = after.operations().len();
    if !valid_replacements(old_len, new_len, replacements)
        || bijections.len() != replacements.len().saturating_add(1)
    {
        return false;
    }

    let mut old_cursor = 0usize;
    let mut new_cursor = 0usize;
    for (replacement, bijection) in replacements.iter().zip(bijections) {
        if !valid_clean_gap_bijection(
            before,
            after,
            old_cursor..replacement.old.start,
            new_cursor..replacement.new.start,
            bijection,
        ) {
            return false;
        }
        old_cursor = replacement.old.end;
        new_cursor = replacement.new.end;
    }
    valid_clean_gap_bijection(
        before,
        after,
        old_cursor..old_len,
        new_cursor..new_len,
        &bijections[bijections.len() - 1],
    )
}

fn valid_clean_gap_bijection(
    before: &Circuit,
    after: &Circuit,
    old: Range<usize>,
    new: Range<usize>,
    bijection: &QubitBijection,
) -> bool {
    if old.len() != new.len()
        || bijection
            .pairs
            .windows(2)
            .any(|pair| pair[0].0 >= pair[1].0)
        || bijection
            .pairs
            .iter()
            .enumerate()
            .any(|(index, (_, target))| {
                bijection.pairs[..index]
                    .iter()
                    .any(|(_, previous)| previous == target)
            })
    {
        return false;
    }

    let mut used = vec![false; bijection.pairs.len()];
    for (old_order, new_order) in old.zip(new) {
        let source = &before.operations()[old_order];
        let target = &after.operations()[new_order];
        if source.instruction != target.instruction
            || source.label != target.label
            || source.qubits.len() != target.qubits.len()
            || !resolved_params_are_equal(before, source, after, target)
        {
            return false;
        }
        for (&source_qubit, &target_qubit) in source.qubits.iter().zip(&target.qubits) {
            let Ok(index) = bijection
                .pairs
                .binary_search_by_key(&source_qubit, |(source, _)| *source)
            else {
                return false;
            };
            if bijection.pairs[index].1 != target_qubit {
                return false;
            }
            used[index] = true;
        }
    }
    used.into_iter().all(|entry| entry)
}

fn resolved_params_are_equal(
    before: &Circuit,
    source: &Operation,
    after: &Circuit,
    target: &Operation,
) -> bool {
    source.params.len() == target.params.len()
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
