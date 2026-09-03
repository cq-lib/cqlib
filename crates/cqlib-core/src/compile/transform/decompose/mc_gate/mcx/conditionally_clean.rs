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

//! KG24 MCX synthesis using internally generated conditionally-clean workspace.
//!
//! A conditionally-clean workspace is not an ancillary qubit supplied by the
//! caller. It is a control qubit whose value becomes usable as temporary
//! workspace only under the conditions established by the surrounding ladder.
//! This module consumes one or two ordinary external ancillas and constructs
//! that temporary workspace internally.
//!
//! The one-ancilla variants use a linear-depth ladder. The two-ancilla variants
//! use a logarithmic-depth outer ladder and a linear-depth middle action over
//! the logarithmically reduced control frontier. Clean entry points require
//! every supplied ancilla to enter in `|0>` and restore it to `|0>`. Dirty
//! entry points permit unknown initial ancillary states and restore those
//! states exactly.
//!
//! The constructions follow Khattar and Gidney, *Rise of conditionally clean
//! ancillae for optimizing quantum circuits* (2024),
//! [arXiv:2407.17966](https://arxiv.org/abs/2407.17966).

use crate::circuit::{Qubit, StandardGate, operation::ValueOperation};
use crate::compile::error::CompilerError;
use crate::compile::transform::decompose::mc_gate::validation::find_duplicate_qubit;

use super::{
    DECOMPOSE_MCX_NAME, relative_phase::emit_relative_phase_toffoli, trivial::decompose_mcx_small,
    utils::invert_parameter_free_operations,
};

/// Initial-state contract for an externally supplied ancillary qubit.
///
/// Both modes restore the ancillary qubit before the returned operation
/// sequence completes. `Clean` selects the shorter circuit available when the
/// caller guarantees `|0>`. `Dirty` adds toggle detection so an unknown
/// initial value does not affect the target action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AncillaState {
    /// The ancillary qubit enters in `|0>` and returns to `|0>`.
    Clean,
    /// The ancillary qubit may enter in an unknown state and is restored.
    Dirty,
}

/// Parameter-free linear ladder used by the one-ancilla KG24 construction.
///
/// The ladder receives the logical wire order `[ancilla, controls...]`. Its
/// operations propagate the conditional workspace through those wires while
/// preserving one control for the exact target action.
#[derive(Debug)]
pub(super) struct LinearLadder {
    /// Operations used to propagate the conditional workspace.
    pub(super) operations: Vec<ValueOperation>,
    /// Control paired with the ancillary flag at the exact target action.
    pub(super) final_control: Qubit,
}

/// Parameter-free logarithmic-depth ladder used by the two-ancilla KG24
/// construction.
///
/// The ladder consumes controls in their caller-provided role order. Its
/// remaining frontier is passed to the middle one-ancilla action.
#[derive(Debug)]
pub(super) struct LogDepthLadder {
    /// Operations that establish the reduced control frontier.
    pub(super) operations: Vec<ValueOperation>,
    /// Controls that remain after the parallel ladder reduction.
    pub(super) remaining_controls: Vec<Qubit>,
}

/// Decomposes an exact MCX using one clean ancillary qubit.
///
/// `controls` are the MCX controls, `target` is the X target, and
/// `clean_ancilla` is an ordinary external ancilla that must enter in `|0>`.
/// The returned sequence restores it to `|0>` and implements exact MCX
/// without residual input-dependent relative phases. Inputs with at most two
/// controls delegate to [`decompose_mcx_small`] and do not consume or
/// validate `clean_ancilla`.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when a consumed qubit is
/// repeated or when an internal parameter-free operation cannot be inverted.
///
/// # References
///
/// Khattar and Gidney, *Rise of conditionally clean ancillae for optimizing
/// quantum circuits* (2024), Section 5.1,
/// [arXiv:2407.17966](https://arxiv.org/abs/2407.17966).
pub fn decompose_mcx_1_clean_kg24(
    controls: &[Qubit],
    target: Qubit,
    clean_ancilla: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_mcx_with_one_conditionally_clean_workspace(
        controls,
        target,
        clean_ancilla,
        AncillaState::Clean,
    )
}

/// Decomposes an exact MCX using one borrowed dirty ancillary qubit.
///
/// `controls` are the MCX controls, `target` is the X target, and
/// `dirty_ancilla` is an ordinary external ancilla whose initial state may be
/// unknown. The returned sequence restores that state exactly and implements
/// exact MCX without residual input-dependent relative phases. Inputs with at
/// most two controls delegate to [`decompose_mcx_small`] and do not consume
/// or validate `dirty_ancilla`.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when a consumed qubit is
/// repeated or when an internal parameter-free operation cannot be inverted.
///
/// # References
///
/// Khattar and Gidney, *Rise of conditionally clean ancillae for optimizing
/// quantum circuits* (2024), Section 5.3,
/// [arXiv:2407.17966](https://arxiv.org/abs/2407.17966).
pub fn decompose_mcx_1_dirty(
    controls: &[Qubit],
    target: Qubit,
    dirty_ancilla: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_mcx_with_one_conditionally_clean_workspace(
        controls,
        target,
        dirty_ancilla,
        AncillaState::Dirty,
    )
}

/// Decomposes an exact MCX using two clean ancillary qubits.
///
/// `controls` are the MCX controls, `target` is the X target, and both
/// `clean_ancillas` are ordinary external ancillas that must enter in `|0>`.
/// The logarithmic-depth outer ladder restores both ancillas to `|0>` and
/// implements exact MCX without residual input-dependent relative phases.
/// Inputs with at most two controls delegate to [`decompose_mcx_small`] and
/// do not consume or validate `clean_ancillas`.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when a consumed qubit is
/// repeated or when an internal parameter-free operation cannot be inverted.
///
/// # References
///
/// Khattar and Gidney, *Rise of conditionally clean ancillae for optimizing
/// quantum circuits* (2024), Section 5.2,
/// [arXiv:2407.17966](https://arxiv.org/abs/2407.17966).
pub fn decompose_mcx_2_clean(
    controls: &[Qubit],
    target: Qubit,
    clean_ancillas: [Qubit; 2],
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_mcx_with_two_conditionally_clean_workspaces(
        controls,
        target,
        clean_ancillas,
        AncillaState::Clean,
    )
}

/// Decomposes an exact MCX using two borrowed dirty ancillary qubits.
///
/// `controls` are the MCX controls, `target` is the X target, and both
/// `dirty_ancillas` are ordinary external ancillas whose initial states may be
/// unknown. The returned sequence restores both states exactly and implements
/// exact MCX without residual input-dependent relative phases. Inputs with at
/// most two controls delegate to [`decompose_mcx_small`] and do not consume
/// or validate `dirty_ancillas`.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when a consumed qubit is
/// repeated or when an internal parameter-free operation cannot be inverted.
///
/// # References
///
/// Khattar and Gidney, *Rise of conditionally clean ancillae for optimizing
/// quantum circuits* (2024), Section 5.4,
/// [arXiv:2407.17966](https://arxiv.org/abs/2407.17966).
pub fn decompose_mcx_2_dirty(
    controls: &[Qubit],
    target: Qubit,
    dirty_ancillas: [Qubit; 2],
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_mcx_with_two_conditionally_clean_workspaces(
        controls,
        target,
        dirty_ancillas,
        AncillaState::Dirty,
    )
}

/// Implements the shared one-ancilla linear-depth KG24 construction.
///
/// In `Clean` mode, `ancilla` must be physically clean. In `Dirty` mode, the
/// repeated target action without the initial conditional-clean creation
/// cancels the contribution from an unknown borrowed-ancilla value.
///
/// The two-ancilla construction also uses the clean-shaped circuit as a middle
/// action. In that nested context, the outer ladder establishes the weaker
/// conditionally-clean workspace contract required by this circuit shape.
fn decompose_mcx_with_one_conditionally_clean_workspace(
    controls: &[Qubit],
    target: Qubit,
    ancilla: Qubit,
    ancilla_state: AncillaState,
) -> Result<Vec<ValueOperation>, CompilerError> {
    if controls.len() <= 2 {
        return decompose_mcx_small(controls, target);
    }

    validate_consumed_qubits(controls, target, &[ancilla])?;

    let ladder = build_linear_depth_ladder(ancilla, controls)?;
    let inverse_ladder = invert_parameter_free_operations(&ladder.operations)?;
    let mut operations = vec![];

    emit_relative_phase_toffoli(&mut operations, controls[0], controls[1], ancilla)?;
    append_linear_target_action(&mut operations, &ladder, &inverse_ladder, ancilla, target);
    emit_relative_phase_toffoli(&mut operations, controls[0], controls[1], ancilla)?;

    if matches!(ancilla_state, AncillaState::Dirty) {
        // Repeating the target action without the initial conditional-clean
        // creation cancels the contribution from an unknown borrowed value.
        append_linear_target_action(&mut operations, &ladder, &inverse_ladder, ancilla, target);
    }

    Ok(operations)
}

/// Appends one linear ladder, its exact target action, and its inverse.
fn append_linear_target_action(
    operations: &mut Vec<ValueOperation>,
    ladder: &LinearLadder,
    inverse_ladder: &[ValueOperation],
    ancilla: Qubit,
    target: Qubit,
) {
    operations.extend(ladder.operations.iter().cloned());
    operations.push(ValueOperation::from_standard(
        StandardGate::CCX,
        [ancilla, ladder.final_control, target],
        [],
    ));
    operations.extend(inverse_ladder.iter().cloned());
}

/// Builds the one-ancilla linear-depth conditional-workspace ladder.
///
/// For `controls = [c0, c1, ...]`, the logical wires are
/// `[ancilla, c0, c1, ...]`. Each propagation step emits an RCCX followed by
/// `X` on the same target. The first RCCX that creates the ancillary flag and
/// the final exact target action are emitted by the caller.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when fewer than three controls
/// are provided or when an emitted RCCX contains repeated qubits.
pub(super) fn build_linear_depth_ladder(
    ancilla: Qubit,
    controls: &[Qubit],
) -> Result<LinearLadder, CompilerError> {
    if controls.len() < 3 {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "KG24 linear ladder requires at least 3 controls, got {}",
                controls.len()
            ),
        });
    }

    let mut wires = Vec::with_capacity(controls.len() + 1);
    wires.push(ancilla);
    wires.extend_from_slice(controls);
    let wire_count = wires.len();
    let mut operations = vec![];

    let mut wire_index = 2;
    while wire_index < wire_count - 2 {
        append_conditional_workspace_step(
            &mut operations,
            wires[wire_index + 1],
            wires[wire_index + 2],
            wires[wire_index],
        )?;
        wire_index += 2;
    }

    let (first_control_index, second_control_index, target_index) = if wire_count % 2 == 0 {
        (
            wire_count as isize - 1,
            wire_count as isize - 4,
            wire_count as isize - 5,
        )
    } else {
        (
            wire_count as isize - 3,
            wire_count as isize - 5,
            wire_count as isize - 6,
        )
    };

    if target_index > 0 {
        append_conditional_workspace_step(
            &mut operations,
            wires[first_control_index as usize],
            wires[second_control_index as usize],
            wires[target_index as usize],
        )?;
    }

    let mut wire_index = target_index;
    while wire_index > 2 {
        append_conditional_workspace_step(
            &mut operations,
            wires[wire_index as usize],
            wires[wire_index as usize - 1],
            wires[wire_index as usize - 2],
        )?;
        wire_index -= 2;
    }

    Ok(LinearLadder {
        operations,
        final_control: controls[5usize.saturating_sub(controls.len())],
    })
}

/// Implements the shared two-ancilla logarithmic-depth KG24 construction.
///
/// In `Dirty` mode, the second round skips only the initial conditional-clean
/// creation. Its middle action still uses the clean-shaped one-ancilla
/// circuit. The secondary borrowed ancilla is not globally clean: the outer
/// laddered toggle-detection composition establishes exactly the effective
/// clean-workspace contract required by the nested action and restores the
/// secondary ancilla at the end.
fn decompose_mcx_with_two_conditionally_clean_workspaces(
    controls: &[Qubit],
    target: Qubit,
    ancillas: [Qubit; 2],
    ancilla_state: AncillaState,
) -> Result<Vec<ValueOperation>, CompilerError> {
    if controls.len() <= 2 {
        return decompose_mcx_small(controls, target);
    }

    validate_consumed_qubits(controls, target, &ancillas)?;

    let primary_ancilla = ancillas[0];
    let secondary_ancilla = ancillas[1];
    let ladder = build_log_depth_ladder(primary_ancilla, controls, false)?;
    let inverse_ladder = invert_parameter_free_operations(&ladder.operations)?;
    let middle_action = build_log_depth_middle_action(
        primary_ancilla,
        &ladder.remaining_controls,
        target,
        secondary_ancilla,
    )?;

    let mut operations = Vec::with_capacity(ladder.operations.len() * 2 + middle_action.len());
    append_log_depth_round(
        &mut operations,
        &ladder.operations,
        &middle_action,
        &inverse_ladder,
    );

    if matches!(ancilla_state, AncillaState::Dirty) {
        let toggle_detection_ladder = build_log_depth_ladder(primary_ancilla, controls, true)?;
        if toggle_detection_ladder.remaining_controls != ladder.remaining_controls {
            return Err(CompilerError::TransformFailed {
                name: DECOMPOSE_MCX_NAME,
                reason: "KG24 toggle-detection ladder changed the reduced control frontier"
                    .to_string(),
            });
        }
        let inverse_toggle_detection_ladder =
            invert_parameter_free_operations(&toggle_detection_ladder.operations)?;

        // Omitting only the initial conditional-clean creation isolates and
        // cancels the contribution from unknown borrowed-ancilla values.
        append_log_depth_round(
            &mut operations,
            &toggle_detection_ladder.operations,
            &middle_action,
            &inverse_toggle_detection_ladder,
        );
    }

    Ok(operations)
}

/// Appends one logarithmic ladder, its middle action, and its inverse.
fn append_log_depth_round(
    operations: &mut Vec<ValueOperation>,
    ladder: &[ValueOperation],
    middle_action: &[ValueOperation],
    inverse_ladder: &[ValueOperation],
) {
    operations.extend(ladder.iter().cloned());
    operations.extend(middle_action.iter().cloned());
    operations.extend(inverse_ladder.iter().cloned());
}

/// Builds the exact middle action for a reduced logarithmic-depth frontier.
///
/// A single remaining control needs one exact `CCX`. Larger frontiers use the
/// clean-shaped one-ancilla KG24 circuit with the primary flag prepended as an
/// effective control and the secondary ancillary qubit as workspace.
fn build_log_depth_middle_action(
    primary_ancilla: Qubit,
    remaining_controls: &[Qubit],
    target: Qubit,
    secondary_ancilla: Qubit,
) -> Result<Vec<ValueOperation>, CompilerError> {
    if let [remaining_control] = remaining_controls {
        return Ok(vec![ValueOperation::from_standard(
            StandardGate::CCX,
            [primary_ancilla, *remaining_control, target],
            [],
        )]);
    }

    let mut middle_controls = Vec::with_capacity(remaining_controls.len() + 1);
    middle_controls.push(primary_ancilla);
    middle_controls.extend_from_slice(remaining_controls);
    decompose_mcx_with_one_conditionally_clean_workspace(
        &middle_controls,
        target,
        secondary_ancilla,
        AncillaState::Clean,
    )
}

/// Builds the logarithmic-depth conditional-workspace ladder.
///
/// The builder tracks logical positions rather than sorting physical qubit
/// identifiers. `skip_initial_conditionally_clean_step` omits only the first
/// RCCX that creates the primary flag; it leaves every later propagation step
/// unchanged for dirty-ancilla toggle detection.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when fewer than three controls
/// are provided or when an emitted RCCX contains repeated qubits.
pub(super) fn build_log_depth_ladder(
    primary_ancilla: Qubit,
    controls: &[Qubit],
    skip_initial_conditionally_clean_step: bool,
) -> Result<LogDepthLadder, CompilerError> {
    if controls.len() < 3 {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "KG24 logarithmic ladder requires at least 3 controls, got {}",
                controls.len()
            ),
        });
    }

    let primary_index = controls.len();
    let mut ladder_qubits = controls.to_vec();
    ladder_qubits.push(primary_ancilla);
    let mut available_workspace = vec![primary_index];
    let mut pending_controls: Vec<_> = (0..controls.len()).collect();
    let mut remaining_controls = vec![];
    let mut operations = vec![];

    while pending_controls.len() > 1 {
        let next_batch_len = (available_workspace.len() + 1).min(pending_controls.len());
        let mut next_batch: Vec<_> = pending_controls.drain(..next_batch_len).collect();
        let mut new_workspace = vec![];

        while next_batch.len() > 1 {
            let pair_count = next_batch.len() / 2;
            let offset = next_batch.len() % 2;
            let first_controls = &next_batch[offset..offset + pair_count];
            let second_controls = &next_batch[offset + pair_count..];
            let workspace_start = available_workspace.len() - pair_count;
            let targets = &available_workspace[workspace_start..];

            if targets == [primary_index] {
                if !skip_initial_conditionally_clean_step {
                    emit_relative_phase_toffoli(
                        &mut operations,
                        ladder_qubits[first_controls[0]],
                        ladder_qubits[second_controls[0]],
                        primary_ancilla,
                    )?;
                }
            } else {
                for target_index in targets {
                    operations.push(ValueOperation::from_standard(
                        StandardGate::X,
                        [ladder_qubits[*target_index]],
                        [],
                    ));
                }
                for ((first_control, second_control), target_index) in
                    first_controls.iter().zip(second_controls).zip(targets)
                {
                    emit_relative_phase_toffoli(
                        &mut operations,
                        ladder_qubits[*first_control],
                        ladder_qubits[*second_control],
                        ladder_qubits[*target_index],
                    )?;
                }
            }

            new_workspace.extend_from_slice(&next_batch[offset..]);
            let mut reduced_batch = targets.to_vec();
            reduced_batch.extend_from_slice(&next_batch[..offset]);
            next_batch = reduced_batch;
            available_workspace.truncate(workspace_start);
        }

        available_workspace.extend(new_workspace);
        available_workspace.sort_unstable();
        remaining_controls.extend(next_batch);
    }

    remaining_controls.extend(pending_controls);
    remaining_controls.sort_unstable();
    remaining_controls.retain(|index| *index != primary_index);

    Ok(LogDepthLadder {
        operations,
        remaining_controls: remaining_controls
            .into_iter()
            .map(|index| ladder_qubits[index])
            .collect(),
    })
}

/// Appends one conditional-workspace propagation step.
///
/// The `X` after RCCX converts the temporary target into the conditionally
/// clean polarity required by the next ladder stage.
fn append_conditional_workspace_step(
    operations: &mut Vec<ValueOperation>,
    first_control: Qubit,
    second_control: Qubit,
    target: Qubit,
) -> Result<(), CompilerError> {
    emit_relative_phase_toffoli(operations, first_control, second_control, target)?;
    operations.push(ValueOperation::from_standard(StandardGate::X, [target], []));
    Ok(())
}

/// Validates all qubits consumed by a non-trivial KG24 construction.
///
/// Trivial entry points bypass this helper because they do not consume
/// ancillary qubits.
fn validate_consumed_qubits(
    controls: &[Qubit],
    target: Qubit,
    ancillas: &[Qubit],
) -> Result<(), CompilerError> {
    let target_group = [target];
    if let Some(qubit) = find_duplicate_qubit(&[controls, &target_group, ancillas]) {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "MCX controls, target, and ancillas must be distinct; duplicate {qubit}"
            ),
        });
    }

    Ok(())
}
