// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! HP24 MCX synthesis without external ancillary qubits.
//!
//! This module implements the linear-size construction from Huang and
//! Palsberg, *Compiling Conditional Quantum Gates without Using Helper
//! Qubits*, PLDI 2024,
//! [DOI:10.1145/3656436](https://doi.org/10.1145/3656436). The public
//! interface does not accept or allocate ancillary qubits. Intermediate
//! increment and relative-MCX blocks temporarily borrow qubits already
//! occupied by the MCX input, treat their unknown values as dirty workspace,
//! and restore those values before returning.
//!
//! The top-level construction surrounds a controlled phase operation with
//! `H` gates on the MCX target. For sufficiently large even-width inputs it
//! uses the one-dirty-qubit increment from Figure 7. All other non-trivial
//! inputs use the two-dirty-qubit increment from Figure 8. Both paths emit a
//! number of low-level operations linear in the number of MCX qubits.

use crate::circuit::{ParameterValue, Qubit, StandardGate, operation::ValueOperation};
use crate::compile::error::CompilerError;
use crate::compile::transform::decompose::mc_gate::validation::find_duplicate_qubit;
use std::f64::consts::PI;

use super::{
    DECOMPOSE_MCX_NAME,
    dirty_v_chain::{decompose_mcx_n_dirty, decompose_relative_phase_mcx_n_dirty},
    relative_phase::emit_relative_phase_toffoli,
    trivial::decompose_mcx_small,
};

/// Minimum total MCX width for which the Figure 7 one-dirty path is cheaper.
///
/// The threshold comes from comparing the costs of the one-dirty and
/// two-dirty HP24 constructions. The one-dirty construction also requires an
/// even total MCX width.
pub(super) const ONE_DIRTY_INCREMENT_MIN_QUBITS: usize = 23;

/// Largest dirty-increment width synthesized by the compact cascade.
const SMALL_DIRTY_INCREMENT_MAX_QUBITS: usize = 10;

/// Largest relative-MCX control count synthesized by the recursive formula.
const RECURSIVE_RELATIVE_MCX_MAX_CONTROLS: usize = 10;

/// Selects one of the two top-level HP24 constructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Hp24Path {
    /// Figure 7 construction using one internally borrowed dirty qubit.
    OneDirty,
    /// Figure 8 construction using two internally borrowed dirty qubits.
    TwoDirty,
}

/// Selects whether an increment block adds or subtracts one modulo its width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IncrementDirection {
    /// Add one modulo the number of data-qubit basis states.
    AddOne,
    /// Subtract one modulo the number of data-qubit basis states.
    SubtractOne,
}

/// Decomposes an MCX without allocating or accepting ancillary qubits.
///
/// `controls` are the positive controls that must all be in `|1>` for
/// `target` to flip. The inputs must be pairwise distinct. The HP24
/// construction temporarily borrows input data qubits as dirty workspace and
/// restores every borrowed value before the returned sequence completes.
///
/// Inputs with at most two controls delegate to [`decompose_mcx_small`].
/// Larger inputs emit an exact MCX as independent [`ValueOperation`] values.
/// The output contains no multi-controlled placeholders and references only
/// `controls` and `target`.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when an input qubit is repeated
/// or when a checked size calculation fails.
///
/// # References
///
/// Huang and Palsberg, *Compiling Conditional Quantum Gates without Using
/// Helper Qubits*, PLDI 2024,
/// [DOI:10.1145/3656436](https://doi.org/10.1145/3656436).
pub fn decompose_mcx_no_aux(
    controls: &[Qubit],
    target: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    if controls.len() <= 2 {
        return decompose_mcx_small(controls, target);
    }

    let total_qubits =
        controls
            .len()
            .checked_add(1)
            .ok_or_else(|| CompilerError::TransformFailed {
                name: DECOMPOSE_MCX_NAME,
                reason: "HP24 MCX width overflow".to_string(),
            })?;
    let target_group = [target];
    if let Some(qubit) = find_duplicate_qubit(&[controls, &target_group]) {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "MCX controls, target, and ancillas must be distinct; duplicate {qubit}"
            ),
        });
    }

    let mut operations = vec![];
    operations.push(ValueOperation::from_standard(StandardGate::H, [target], []));

    match select_hp24_path(total_qubits) {
        Hp24Path::OneDirty => {
            emit_hp24_one_dirty_path(&mut operations, controls, target)?;
        }
        Hp24Path::TwoDirty => {
            let (increment_data, internal_dirty_controls) = controls.split_at(controls.len() - 1);
            emit_hp24_two_dirty_path(
                &mut operations,
                increment_data,
                internal_dirty_controls[0],
                target,
            )?;
        }
    }

    operations.push(ValueOperation::from_standard(StandardGate::H, [target], []));
    Ok(operations)
}

/// Chooses Figure 7 only when its parity precondition and cost threshold hold.
pub(super) fn select_hp24_path(total_qubits: usize) -> Hp24Path {
    if total_qubits & 1 == 0 && total_qubits >= ONE_DIRTY_INCREMENT_MIN_QUBITS {
        Hp24Path::OneDirty
    } else {
        Hp24Path::TwoDirty
    }
}

/// Emits the Figure 7 controlled-phase construction.
///
/// `increment_data` contains all controls. `internal_dirty_target` is the MCX
/// target, borrowed by the increment blocks and restored before each phase
/// sequence is complete.
fn emit_hp24_one_dirty_path(
    operations: &mut Vec<ValueOperation>,
    increment_data: &[Qubit],
    internal_dirty_target: Qubit,
) -> Result<(), CompilerError> {
    emit_increment_one_dirty(
        operations,
        increment_data,
        internal_dirty_target,
        IncrementDirection::AddOne,
    )?;

    let mut theta = -PI;
    for control in increment_data[1..].iter().rev() {
        theta /= 2.0;
        emit_controlled_phase(operations, *control, internal_dirty_target, theta);
    }

    emit_increment_one_dirty(
        operations,
        increment_data,
        internal_dirty_target,
        IncrementDirection::SubtractOne,
    )?;

    let mut theta = PI;
    for control in increment_data[1..].iter().rev() {
        theta /= 2.0;
        emit_controlled_phase(operations, *control, internal_dirty_target, theta);
    }
    emit_controlled_phase(operations, increment_data[0], internal_dirty_target, theta);
    Ok(())
}

/// Emits the Figure 8 controlled-phase construction.
///
/// `increment_data` contains the controls not borrowed by the increment
/// blocks. `first_internal_dirty` is the remaining control and
/// `second_internal_dirty` is the MCX target. Both are restored before the
/// returned block completes.
fn emit_hp24_two_dirty_path(
    operations: &mut Vec<ValueOperation>,
    increment_data: &[Qubit],
    first_internal_dirty: Qubit,
    second_internal_dirty: Qubit,
) -> Result<(), CompilerError> {
    emit_increment_two_dirty(
        operations,
        increment_data,
        first_internal_dirty,
        second_internal_dirty,
        IncrementDirection::AddOne,
    )?;

    let mut theta = -PI;
    for control in increment_data[1..].iter().rev() {
        theta /= 2.0;
        emit_double_controlled_phase(
            operations,
            *control,
            first_internal_dirty,
            second_internal_dirty,
            theta,
        );
    }

    emit_increment_two_dirty(
        operations,
        increment_data,
        first_internal_dirty,
        second_internal_dirty,
        IncrementDirection::SubtractOne,
    )?;

    let mut theta = PI;
    for control in increment_data[1..].iter().rev() {
        theta /= 2.0;
        emit_double_controlled_phase(
            operations,
            *control,
            first_internal_dirty,
            second_internal_dirty,
            theta,
        );
    }
    emit_double_controlled_phase(
        operations,
        increment_data[0],
        first_internal_dirty,
        second_internal_dirty,
        theta,
    );
    Ok(())
}

/// Emits an increment or decrement over `data_qubits` using equally many dirty
/// workspace qubits.
///
/// The operation is modular in the width of `data_qubits`. Every consumed
/// dirty workspace qubit is restored to its unknown input value.
pub(super) fn emit_increment_n_dirty(
    operations: &mut Vec<ValueOperation>,
    data_qubits: &[Qubit],
    dirty_workspace: &[Qubit],
    direction: IncrementDirection,
) -> Result<(), CompilerError> {
    let used_dirty_workspace = validate_increment_workspace(data_qubits, dirty_workspace)?;
    let mut block = vec![];
    emit_subtract_wrapper_prefix(&mut block, data_qubits, direction);
    if data_qubits.len() <= SMALL_DIRTY_INCREMENT_MAX_QUBITS {
        emit_increment_n_dirty_small(&mut block, data_qubits, used_dirty_workspace)?;
    } else {
        emit_increment_n_dirty_large(&mut block, data_qubits, used_dirty_workspace)?;
    }
    emit_subtract_wrapper_prefix(&mut block, data_qubits, direction);
    append_cloned_operations(operations, &block)
}

/// Emits the compact dirty-increment cascade for small widths.
///
/// Each suffix carry is represented by an exact MCX. The workspace excludes
/// its first qubit so that the required dirty prefix is always available.
fn emit_increment_n_dirty_small(
    operations: &mut Vec<ValueOperation>,
    data_qubits: &[Qubit],
    dirty_workspace: &[Qubit],
) -> Result<(), CompilerError> {
    for carry_width in (1..data_qubits.len()).rev() {
        let carry = decompose_mcx_n_dirty(
            &data_qubits[..carry_width],
            data_qubits[carry_width],
            &dirty_workspace[1..],
        )?;
        append_cloned_operations(operations, &carry)?;
    }
    operations.push(ValueOperation::from_standard(
        StandardGate::X,
        [data_qubits[0]],
        [],
    ));
    Ok(())
}

/// Emits the linear-size dirty-increment construction for large widths.
///
/// The mirrored `Ux` and `Uz` ladders restore all dirty workspace values. The
/// first workspace qubit carries the temporary parity used by both ladders.
fn emit_increment_n_dirty_large(
    operations: &mut Vec<ValueOperation>,
    data_qubits: &[Qubit],
    dirty_workspace: &[Qubit],
) -> Result<(), CompilerError> {
    let first_dirty = dirty_workspace[0];
    operations.push(ValueOperation::from_standard(
        StandardGate::X,
        [first_dirty],
        [],
    ));
    for data in data_qubits {
        operations.push(ValueOperation::from_standard(
            StandardGate::CX,
            [first_dirty, *data],
            [],
        ));
    }
    operations.push(ValueOperation::from_standard(
        StandardGate::X,
        [first_dirty],
        [],
    ));

    for index in 0..data_qubits.len() - 1 {
        emit_ux(
            operations,
            first_dirty,
            dirty_workspace[index + 1],
            data_qubits[index],
        );
    }
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [first_dirty, data_qubits[data_qubits.len() - 1]],
        [],
    ));
    for index in (0..data_qubits.len() - 1).rev() {
        emit_uz(
            operations,
            first_dirty,
            dirty_workspace[index + 1],
            data_qubits[index],
        );
    }

    for dirty in &dirty_workspace[1..] {
        operations.push(ValueOperation::from_standard(StandardGate::X, [*dirty], []));
    }

    for index in 0..data_qubits.len() - 1 {
        emit_ux(
            operations,
            first_dirty,
            dirty_workspace[index + 1],
            data_qubits[index],
        );
    }
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [first_dirty, data_qubits[data_qubits.len() - 1]],
        [],
    ));
    for index in (0..data_qubits.len() - 1).rev() {
        emit_uz(
            operations,
            first_dirty,
            dirty_workspace[index + 1],
            data_qubits[index],
        );
    }
    for dirty in &dirty_workspace[1..] {
        operations.push(ValueOperation::from_standard(StandardGate::X, [*dirty], []));
    }

    operations.push(ValueOperation::from_standard(
        StandardGate::X,
        [data_qubits[data_qubits.len() - 1]],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::X,
        [first_dirty],
        [],
    ));
    for data in data_qubits {
        operations.push(ValueOperation::from_standard(
            StandardGate::CX,
            [first_dirty, *data],
            [],
        ));
    }
    operations.push(ValueOperation::from_standard(
        StandardGate::X,
        [first_dirty],
        [],
    ));
    Ok(())
}

/// Emits an increment or decrement using one explicitly borrowed dirty qubit.
///
/// `data_qubits` must have odd width. The second half is borrowed as dirty
/// workspace by nested blocks. The mirrored sequence restores both that
/// workspace and `internal_dirty` before returning. This internal block
/// implements the required computational-basis permutation but may introduce
/// relative phases that cancel in the surrounding HP24 construction.
pub(super) fn emit_increment_one_dirty(
    operations: &mut Vec<ValueOperation>,
    data_qubits: &[Qubit],
    internal_dirty: Qubit,
    direction: IncrementDirection,
) -> Result<(), CompilerError> {
    if data_qubits.is_empty() || data_qubits.len() & 1 == 0 {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "HP24 one-dirty increment requires a positive odd data width, got {}",
                data_qubits.len()
            ),
        });
    }
    let internal_dirty_group = [internal_dirty];
    if let Some(qubit) = find_duplicate_qubit(&[data_qubits, &internal_dirty_group]) {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "MCX controls, target, and ancillas must be distinct; duplicate {qubit}"
            ),
        });
    }

    let mut block = vec![];
    emit_subtract_wrapper_prefix(&mut block, data_qubits, direction);
    emit_increment_one_dirty_add_one(&mut block, data_qubits, internal_dirty)?;
    emit_subtract_wrapper_prefix(&mut block, data_qubits, direction);
    append_cloned_operations(operations, &block)
}

/// Emits the additive one-dirty increment core from the recursive split.
fn emit_increment_one_dirty_add_one(
    operations: &mut Vec<ValueOperation>,
    data_qubits: &[Qubit],
    internal_dirty: Qubit,
) -> Result<(), CompilerError> {
    let middle = data_qubits.len().div_ceil(2);
    let internal_dirty_group = [internal_dirty];
    let available_qubits = collect_qubits(&[data_qubits, &internal_dirty_group])?;
    let (nested_increment, template_layout) = build_increment_template(&available_qubits, middle)?;

    let first_nested_data = collect_qubits(&[&internal_dirty_group, &data_qubits[middle..]])?;
    let first_mapping = collect_qubits(&[&first_nested_data, &data_qubits[..middle]])?;
    append_remapped_operations(
        operations,
        &nested_increment,
        &template_layout,
        &first_mapping,
    )?;

    operations.push(ValueOperation::from_standard(
        StandardGate::X,
        [internal_dirty],
        [],
    ));
    for data in &data_qubits[middle..] {
        operations.push(ValueOperation::from_standard(
            StandardGate::CX,
            [internal_dirty, *data],
            [],
        ));
    }

    let mut relative_mcx = vec![];
    emit_relative_mcx_with_internal_dirty_qubits(
        &mut relative_mcx,
        &data_qubits[..middle],
        internal_dirty,
        &data_qubits[middle..],
    )?;
    append_cloned_operations(operations, &relative_mcx)?;
    append_remapped_operations(
        operations,
        &nested_increment,
        &template_layout,
        &first_mapping,
    )?;
    operations.push(ValueOperation::from_standard(
        StandardGate::X,
        [internal_dirty],
        [],
    ));
    append_cloned_operations(operations, &relative_mcx)?;

    for data in &data_qubits[middle..] {
        operations.push(ValueOperation::from_standard(
            StandardGate::CX,
            [internal_dirty, *data],
            [],
        ));
    }

    let last_nested_workspace = collect_qubits(&[&data_qubits[middle..], &internal_dirty_group])?;
    let last_mapping = collect_qubits(&[&data_qubits[..middle], &last_nested_workspace])?;
    append_remapped_operations(
        operations,
        &nested_increment,
        &template_layout,
        &last_mapping,
    )
}

/// Emits an increment or decrement using two explicitly borrowed dirty qubits.
///
/// This form accepts either parity of positive data width. Nested blocks
/// borrow complementary data halves, then restore both data and dirty values.
/// This internal block may introduce relative phases; the top-level HP24
/// composition cancels them.
pub(super) fn emit_increment_two_dirty(
    operations: &mut Vec<ValueOperation>,
    data_qubits: &[Qubit],
    first_internal_dirty: Qubit,
    second_internal_dirty: Qubit,
    direction: IncrementDirection,
) -> Result<(), CompilerError> {
    if data_qubits.is_empty() {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: "HP24 two-dirty increment requires at least one data qubit".to_string(),
        });
    }
    let internal_dirty_group = [first_internal_dirty, second_internal_dirty];
    if let Some(qubit) = find_duplicate_qubit(&[data_qubits, &internal_dirty_group]) {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "MCX controls, target, and ancillas must be distinct; duplicate {qubit}"
            ),
        });
    }

    let mut block = vec![];
    emit_subtract_wrapper_prefix(&mut block, data_qubits, direction);
    emit_increment_two_dirty_add_one(
        &mut block,
        data_qubits,
        first_internal_dirty,
        second_internal_dirty,
    )?;
    emit_subtract_wrapper_prefix(&mut block, data_qubits, direction);
    append_cloned_operations(operations, &block)
}

/// Emits the additive two-dirty increment core from the recursive split.
fn emit_increment_two_dirty_add_one(
    operations: &mut Vec<ValueOperation>,
    data_qubits: &[Qubit],
    first_internal_dirty: Qubit,
    second_internal_dirty: Qubit,
) -> Result<(), CompilerError> {
    let middle =
        data_qubits
            .len()
            .checked_add(2)
            .ok_or_else(|| CompilerError::TransformFailed {
                name: DECOMPOSE_MCX_NAME,
                reason: "HP24 two-dirty split overflow".to_string(),
            })?
            / 2;
    let first_nested_width = data_qubits
        .len()
        .checked_sub(middle)
        .and_then(|width| width.checked_add(1))
        .ok_or_else(|| CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: "HP24 nested increment width overflow".to_string(),
        })?;
    let internal_dirty_group = [first_internal_dirty, second_internal_dirty];
    let available_qubits = collect_qubits(&[data_qubits, &internal_dirty_group])?;

    let (first_nested_increment, first_template_layout) =
        build_increment_template(&available_qubits, first_nested_width)?;
    let first_nested_data = collect_qubits(&[&internal_dirty_group[..1], &data_qubits[middle..]])?;
    let first_nested_workspace =
        collect_qubits(&[&data_qubits[..middle], &internal_dirty_group[1..]])?;
    let first_mapping = collect_qubits(&[
        &first_nested_data,
        &first_nested_workspace[..first_nested_width],
    ])?;
    append_remapped_operations(
        operations,
        &first_nested_increment,
        &first_template_layout,
        &first_mapping,
    )?;

    operations.push(ValueOperation::from_standard(
        StandardGate::X,
        [first_internal_dirty],
        [],
    ));
    for data in &data_qubits[middle..] {
        operations.push(ValueOperation::from_standard(
            StandardGate::CX,
            [first_internal_dirty, *data],
            [],
        ));
    }

    let relative_workspace = collect_qubits(&[&data_qubits[middle..], &internal_dirty_group[1..]])?;
    let mut relative_mcx = vec![];
    emit_relative_mcx_with_internal_dirty_qubits(
        &mut relative_mcx,
        &data_qubits[..middle],
        first_internal_dirty,
        &relative_workspace,
    )?;
    append_cloned_operations(operations, &relative_mcx)?;
    append_remapped_operations(
        operations,
        &first_nested_increment,
        &first_template_layout,
        &first_mapping,
    )?;
    operations.push(ValueOperation::from_standard(
        StandardGate::X,
        [first_internal_dirty],
        [],
    ));
    append_cloned_operations(operations, &relative_mcx)?;

    for data in &data_qubits[middle..] {
        operations.push(ValueOperation::from_standard(
            StandardGate::CX,
            [first_internal_dirty, *data],
            [],
        ));
    }

    let (last_nested_increment, last_template_layout) =
        build_increment_template(&available_qubits, middle)?;
    let last_nested_workspace = collect_qubits(&[
        &data_qubits[middle..],
        &internal_dirty_group[..1],
        &internal_dirty_group[1..],
    ])?;
    let last_mapping = collect_qubits(&[&data_qubits[..middle], &last_nested_workspace[..middle]])?;
    append_remapped_operations(
        operations,
        &last_nested_increment,
        &last_template_layout,
        &last_mapping,
    )
}

/// Emits a relative MCX without borrowing any additional qubits.
///
/// The result has the MCX computational-basis permutation but may attach
/// basis-state-dependent phases. It is suitable only inside a surrounding
/// construction that cancels those phases.
pub(super) fn emit_relative_mcx_without_external_ancillas(
    operations: &mut Vec<ValueOperation>,
    controls: &[Qubit],
    target: Qubit,
) -> Result<(), CompilerError> {
    if controls.is_empty() {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: "HP24 relative MCX requires at least one control qubit".to_string(),
        });
    }
    let target_group = [target];
    if let Some(qubit) = find_duplicate_qubit(&[controls, &target_group]) {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "MCX controls, target, and ancillas must be distinct; duplicate {qubit}"
            ),
        });
    }

    let mut block = vec![];
    emit_relative_mcx_recursive(&mut block, controls, target)?;
    append_cloned_operations(operations, &block)
}

/// Emits a relative MCX while allowing explicit internal dirty workspace.
///
/// Small blocks use the recursive HP24 formula without consuming
/// `dirty_workspace`. Larger blocks use the existing dirty V-chain and restore
/// every consumed workspace qubit before returning.
pub(super) fn emit_relative_mcx_with_internal_dirty_qubits(
    operations: &mut Vec<ValueOperation>,
    controls: &[Qubit],
    target: Qubit,
    dirty_workspace: &[Qubit],
) -> Result<(), CompilerError> {
    if controls.len() <= RECURSIVE_RELATIVE_MCX_MAX_CONTROLS {
        return emit_relative_mcx_without_external_ancillas(operations, controls, target);
    }

    let relative_mcx = decompose_relative_phase_mcx_n_dirty(controls, target, dirty_workspace)?;
    append_cloned_operations(operations, &relative_mcx)
}

/// Emits the recursive relative-MCX phase formula over three control blocks.
fn emit_relative_mcx_recursive(
    operations: &mut Vec<ValueOperation>,
    controls: &[Qubit],
    target: Qubit,
) -> Result<(), CompilerError> {
    match controls {
        [] => Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: "HP24 relative MCX requires at least one control qubit".to_string(),
        }),
        [control] => {
            operations.push(ValueOperation::from_standard(
                StandardGate::CX,
                [*control, target],
                [],
            ));
            Ok(())
        }
        [first_control, second_control] => {
            emit_relative_phase_toffoli(operations, *first_control, *second_control, target)
        }
        _ => {
            let third_width = controls.len() / 3;
            let second_width = (controls.len() - third_width) / 2;
            let first_width = controls.len() - third_width - second_width;
            let (first_controls, remaining_controls) = controls.split_at(first_width);
            let (second_controls, third_controls) = remaining_controls.split_at(second_width);

            let mut first_block = vec![];
            emit_relative_mcx_recursive(&mut first_block, first_controls, target)?;
            let mut second_block = vec![];
            emit_relative_mcx_recursive(&mut second_block, second_controls, target)?;
            let mut third_block = vec![];
            emit_relative_mcx_recursive(&mut third_block, third_controls, target)?;

            operations.push(ValueOperation::from_standard(StandardGate::H, [target], []));
            operations.push(ValueOperation::from_standard(
                StandardGate::Phase,
                [target],
                [ParameterValue::Fixed(PI / 8.0)],
            ));
            append_cloned_operations(operations, &third_block)?;
            operations.push(ValueOperation::from_standard(
                StandardGate::Phase,
                [target],
                [ParameterValue::Fixed(-PI / 8.0)],
            ));
            append_cloned_operations(operations, &second_block)?;
            operations.push(ValueOperation::from_standard(
                StandardGate::Phase,
                [target],
                [ParameterValue::Fixed(PI / 8.0)],
            ));
            append_cloned_operations(operations, &third_block)?;
            operations.push(ValueOperation::from_standard(
                StandardGate::Phase,
                [target],
                [ParameterValue::Fixed(-PI / 8.0)],
            ));
            append_cloned_operations(operations, &first_block)?;
            operations.push(ValueOperation::from_standard(
                StandardGate::Phase,
                [target],
                [ParameterValue::Fixed(PI / 8.0)],
            ));
            append_cloned_operations(operations, &third_block)?;
            operations.push(ValueOperation::from_standard(
                StandardGate::Phase,
                [target],
                [ParameterValue::Fixed(-PI / 8.0)],
            ));
            append_cloned_operations(operations, &second_block)?;
            operations.push(ValueOperation::from_standard(
                StandardGate::Phase,
                [target],
                [ParameterValue::Fixed(PI / 8.0)],
            ));
            append_cloned_operations(operations, &third_block)?;
            operations.push(ValueOperation::from_standard(
                StandardGate::Phase,
                [target],
                [ParameterValue::Fixed(-PI / 8.0)],
            ));
            append_cloned_operations(operations, &first_block)?;
            operations.push(ValueOperation::from_standard(StandardGate::H, [target], []));
            Ok(())
        }
    }
}

/// Builds an additive dirty-increment template on an explicit qubit prefix.
///
/// The returned layout records the template qubits in data-then-workspace
/// order. Callers remap the same mathematical block onto complementary data
/// partitions without creating synthetic workspace qubits.
fn build_increment_template(
    available_qubits: &[Qubit],
    data_width: usize,
) -> Result<(Vec<ValueOperation>, Vec<Qubit>), CompilerError> {
    let template_width =
        data_width
            .checked_mul(2)
            .ok_or_else(|| CompilerError::TransformFailed {
                name: DECOMPOSE_MCX_NAME,
                reason: "HP24 dirty-increment template width overflow".to_string(),
            })?;
    if available_qubits.len() < template_width {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "HP24 dirty-increment template with {data_width} data qubits requires {template_width} explicit qubits, got {}",
                available_qubits.len()
            ),
        });
    }

    let template_layout = available_qubits[..template_width].to_vec();
    let (template_data, template_dirty_workspace) = template_layout.split_at(data_width);
    let mut template = vec![];
    emit_increment_n_dirty(
        &mut template,
        template_data,
        template_dirty_workspace,
        IncrementDirection::AddOne,
    )?;
    Ok((template, template_layout))
}

/// Appends a child block after remapping its explicit source layout.
///
/// Every child-operation qubit must occur in `source_qubits`. Both layouts
/// must be pairwise distinct and have equal length, making the internal
/// workspace permutation explicit and preventing accidental external-qubit
/// creation.
fn append_remapped_operations(
    operations: &mut Vec<ValueOperation>,
    child_operations: &[ValueOperation],
    source_qubits: &[Qubit],
    mapped_qubits: &[Qubit],
) -> Result<(), CompilerError> {
    if source_qubits.len() != mapped_qubits.len() {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "HP24 qubit remapping requires equal source and destination widths, got {} and {}",
                source_qubits.len(),
                mapped_qubits.len()
            ),
        });
    }
    if let Some(qubit) = find_duplicate_qubit(&[source_qubits]) {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "MCX controls, target, and ancillas must be distinct; duplicate {qubit}"
            ),
        });
    }
    if let Some(qubit) = find_duplicate_qubit(&[mapped_qubits]) {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "MCX controls, target, and ancillas must be distinct; duplicate {qubit}"
            ),
        });
    }
    reserve_operations(operations, child_operations.len())?;

    for operation in child_operations {
        let mut remapped_operation = operation.clone();
        for qubit in &mut remapped_operation.qubits {
            let source_index = source_qubits
                .iter()
                .position(|source| source == qubit)
                .ok_or_else(|| CompilerError::TransformFailed {
                    name: DECOMPOSE_MCX_NAME,
                    reason: format!("HP24 child operation references unmapped qubit {qubit}"),
                })?;
            *qubit = mapped_qubits[source_index];
        }
        operations.push(remapped_operation);
    }
    Ok(())
}

/// Validates and returns the dirty prefix consumed by an `n`-dirty increment.
fn validate_increment_workspace<'a>(
    data_qubits: &[Qubit],
    dirty_workspace: &'a [Qubit],
) -> Result<&'a [Qubit], CompilerError> {
    if data_qubits.is_empty() {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: "HP24 dirty increment requires at least one data qubit".to_string(),
        });
    }
    if dirty_workspace.len() < data_qubits.len() {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "HP24 dirty increment with {} data qubits requires {} dirty workspace qubits, got {}",
                data_qubits.len(),
                data_qubits.len(),
                dirty_workspace.len()
            ),
        });
    }

    let used_dirty_workspace = &dirty_workspace[..data_qubits.len()];
    if let Some(qubit) = find_duplicate_qubit(&[data_qubits, used_dirty_workspace]) {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "MCX controls, target, and ancillas must be distinct; duplicate {qubit}"
            ),
        });
    }
    Ok(used_dirty_workspace)
}

/// Emits the bitwise-complement prefix used to turn increment into decrement.
fn emit_subtract_wrapper_prefix(
    operations: &mut Vec<ValueOperation>,
    data_qubits: &[Qubit],
    direction: IncrementDirection,
) {
    if direction == IncrementDirection::SubtractOne {
        for data in data_qubits {
            operations.push(ValueOperation::from_standard(StandardGate::X, [*data], []));
        }
    }
}

/// Emits the three-qubit `Ux` increment ladder component.
fn emit_ux(operations: &mut Vec<ValueOperation>, first: Qubit, second: Qubit, third: Qubit) {
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [first, third],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [first, second],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::CCX,
        [second, third, first],
        [],
    ));
}

/// Emits the three-qubit `Uz` increment ladder component.
fn emit_uz(operations: &mut Vec<ValueOperation>, first: Qubit, second: Qubit, third: Qubit) {
    operations.push(ValueOperation::from_standard(
        StandardGate::CCX,
        [second, third, first],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [first, second],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [second, third],
        [],
    ));
}

/// Emits the exact low-level decomposition of `CP(theta)`.
fn emit_controlled_phase(
    operations: &mut Vec<ValueOperation>,
    control: Qubit,
    target: Qubit,
    theta: f64,
) {
    operations.push(ValueOperation::from_standard(
        StandardGate::Phase,
        [control],
        [ParameterValue::Fixed(theta / 2.0)],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::Phase,
        [target],
        [ParameterValue::Fixed(theta / 2.0)],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [control, target],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::Phase,
        [target],
        [ParameterValue::Fixed(-theta / 2.0)],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [control, target],
        [],
    ));
}

/// Emits the exact low-level decomposition of `CCP(theta)`.
fn emit_double_controlled_phase(
    operations: &mut Vec<ValueOperation>,
    first_control: Qubit,
    second_control: Qubit,
    target: Qubit,
    theta: f64,
) {
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [first_control, target],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::Phase,
        [target],
        [ParameterValue::Fixed(-theta / 4.0)],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [second_control, target],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::Phase,
        [target],
        [ParameterValue::Fixed(theta / 4.0)],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [first_control, target],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::Phase,
        [target],
        [ParameterValue::Fixed(-theta / 4.0)],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [second_control, target],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::Phase,
        [target],
        [ParameterValue::Fixed(theta / 4.0)],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::Phase,
        [first_control],
        [ParameterValue::Fixed(theta / 4.0)],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::Phase,
        [second_control],
        [ParameterValue::Fixed(theta / 4.0)],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [first_control, second_control],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::Phase,
        [second_control],
        [ParameterValue::Fixed(-theta / 4.0)],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [first_control, second_control],
        [],
    ));
}

/// Collects explicit qubit groups using checked capacity arithmetic.
fn collect_qubits(qubit_groups: &[&[Qubit]]) -> Result<Vec<Qubit>, CompilerError> {
    let capacity = qubit_groups.iter().try_fold(0usize, |total, qubits| {
        total
            .checked_add(qubits.len())
            .ok_or_else(|| CompilerError::TransformFailed {
                name: DECOMPOSE_MCX_NAME,
                reason: "HP24 qubit layout width overflow".to_string(),
            })
    })?;
    let mut qubits = Vec::with_capacity(capacity);
    for group in qubit_groups {
        qubits.extend_from_slice(group);
    }
    Ok(qubits)
}

/// Appends cloned operations after reserving checked output capacity.
fn append_cloned_operations(
    operations: &mut Vec<ValueOperation>,
    child_operations: &[ValueOperation],
) -> Result<(), CompilerError> {
    reserve_operations(operations, child_operations.len())?;
    operations.extend(child_operations.iter().cloned());
    Ok(())
}

/// Reserves output capacity after checking the resulting operation count.
fn reserve_operations(
    operations: &mut Vec<ValueOperation>,
    additional: usize,
) -> Result<(), CompilerError> {
    operations
        .len()
        .checked_add(additional)
        .ok_or_else(|| CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: "HP24 emitted operation count overflow".to_string(),
        })?;
    operations
        .try_reserve(additional)
        .map_err(|error| CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!("unable to reserve HP24 emitted operation capacity: {error}"),
        })
}
