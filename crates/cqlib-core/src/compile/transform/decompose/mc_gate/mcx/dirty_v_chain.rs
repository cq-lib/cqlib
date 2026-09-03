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

//! I15 MCX synthesis using a dirty-ancilla V-chain.
//!
//! This module implements the linear-size construction described by Item et
//! al., *Quantum Circuits for Isometries*, Phys. Rev. A 93, 032318 (2016),
//! [arXiv:1501.06911](https://arxiv.org/abs/1501.06911). For an MCX with `k >=
//! 3` controls, the construction consumes `k - 2` borrowed dirty ancillas.
//! These ancillas may enter in unknown states and may be modified while the
//! circuit runs, but every consumed ancilla is restored exactly before the
//! returned operation sequence completes.
//!
//! Dirty inputs require two V-chain rounds. Each round toggles the target
//! endpoint, propagates control information backward through action gadgets,
//! applies one relative-phase Toffoli at the start of the chain, and restores
//! the internal ancillas with inverse reset gadgets. Repeating the ladder
//! removes the dependence on the unknown initial ancilla values.
//!
//! The module provides an exact MCX entry point and a relative-phase variant.
//! The latter has the same computational-basis bit-flip behavior and the same
//! ancilla-restoration guarantee, but it may add basis-state-dependent phases.
//! It is intended only for surrounding constructions, such as B95 recursion,
//! that prove those phases cancel. Ancilla allocation, algorithm selection,
//! open-control normalization, and later `CCX` basis lowering remain the
//! responsibility of higher compiler layers.

use crate::circuit::{Qubit, StandardGate, operation::ValueOperation};
use crate::compile::error::CompilerError;
use crate::compile::transform::decompose::mc_gate::validation::find_duplicate_qubit;

use super::{
    DECOMPOSE_MCX_NAME, relative_phase::emit_relative_phase_toffoli, trivial::decompose_mcx_small,
};

/// Selects the endpoint construction used by the shared I15 V-chain.
///
/// Both modes restore the consumed dirty ancillas. `Exact` emits an ordinary
/// MCX, while `RelativePhase` preserves only its computational-basis bit-flip
/// behavior and may add basis-state-dependent phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirtyVChainMode {
    Exact,
    RelativePhase,
}

/// Decomposes an exact MCX using dirty ancillary qubits.
///
/// For at least three controls, the I15 V-chain consumes
/// `controls.len() - 2` ancillary qubits. Each consumed ancilla may enter in an
/// unknown state and is restored exactly by the returned sequence. Extra
/// ancillary qubits are ignored. Inputs with at most two controls delegate to
/// [`decompose_mcx_small`] and do not consume ancillas.
///
/// The construction follows Item et al., *Quantum Circuits for Isometries*,
/// Phys. Rev. A 93, 032318 (2016),
/// [arXiv:1501.06911](https://arxiv.org/abs/1501.06911).
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when fewer than
/// `controls.len() - 2` dirty ancillary qubits are provided for an MCX with at
/// least three controls, or when any consumed qubit is repeated.
pub fn decompose_mcx_n_dirty(
    controls: &[Qubit],
    target: Qubit,
    dirty_ancillas: &[Qubit],
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_dirty_v_chain(controls, target, dirty_ancillas, DirtyVChainMode::Exact)
}

/// Decomposes MCX up to relative phase using dirty ancillary qubits.
///
/// For at least three controls, the I15 V-chain consumes
/// `controls.len() - 2` dirty ancillas and restores each one exactly. Extra
/// ancillas are ignored. Inputs with at most two controls use the exact trivial
/// decomposition.
///
/// Unlike [`decompose_mcx_n_dirty`], the returned sequence is
/// not an ordinary MCX: it has the same computational-basis bit-flip behavior,
/// but may introduce basis-state-dependent phases. Callers must use it only
/// inside a composition, such as B95, whose surrounding structure proves that
/// those phases cancel.
///
/// # Errors
///
/// Returns [`CompilerError::TransformFailed`] when fewer than
/// `controls.len() - 2` dirty ancillary qubits are provided for an MCX with at
/// least three controls, or when any consumed qubit is repeated.
pub fn decompose_relative_phase_mcx_n_dirty(
    controls: &[Qubit],
    target: Qubit,
    dirty_ancillas: &[Qubit],
) -> Result<Vec<ValueOperation>, CompilerError> {
    decompose_dirty_v_chain(
        controls,
        target,
        dirty_ancillas,
        DirtyVChainMode::RelativePhase,
    )
}

/// Implements the shared I15 dirty-ancilla V-chain.
///
/// For controls `[c0, c1, ..., c{k-1}]` and consumed ancillas
/// `[a0, a1, ..., a{k-3}]`, each round emits:
///
/// ```text
/// endpoint(c{k-1}, a{k-3} -> target)
/// for i in reverse(0..k-3):
///     action(c{i+2}, a{i} -> a{i+1})
/// RCCX(c0, c1 -> a0)
/// for i in 0..k-3:
///     reset(c{i+2}, a{i} -> a{i+1})
/// ```
///
/// Two rounds are required because the ancillas may enter in unknown states.
/// In `Exact` mode, both endpoints are `CCX`. In `RelativePhase` mode, the
/// first endpoint is an action gadget and the second is its reset gadget. The
/// latter mode preserves the MCX computational-basis permutation while
/// allowing relative phases.
///
/// Validation completes before output construction begins. Only the required
/// ancilla prefix participates in validation and synthesis.
fn decompose_dirty_v_chain(
    controls: &[Qubit],
    target: Qubit,
    dirty_ancillas: &[Qubit],
    mode: DirtyVChainMode,
) -> Result<Vec<ValueOperation>, CompilerError> {
    if controls.len() <= 2 {
        return decompose_mcx_small(controls, target);
    }

    let required_ancillas = controls.len() - 2;
    if dirty_ancillas.len() < required_ancillas {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "dirty-ancilla MCX decomposition with {} controls requires {} dirty ancillas, got {}",
                controls.len(),
                required_ancillas,
                dirty_ancillas.len()
            ),
        });
    }

    let used_ancillas = &dirty_ancillas[..required_ancillas];
    let target_group = [target];
    if let Some(qubit) = find_duplicate_qubit(&[controls, &target_group, used_ancillas]) {
        return Err(CompilerError::TransformFailed {
            name: DECOMPOSE_MCX_NAME,
            reason: format!(
                "MCX controls, target, and ancillas must be distinct; duplicate {qubit}"
            ),
        });
    }

    let mut operations = vec![];
    for round in 0..2 {
        let last_control = controls[controls.len() - 1];
        let last_ancilla = used_ancillas[required_ancillas - 1];
        match (mode, round) {
            (DirtyVChainMode::Exact, _) => {
                operations.push(ValueOperation::from_standard(
                    StandardGate::CCX,
                    [last_control, last_ancilla, target],
                    [],
                ));
            }
            (DirtyVChainMode::RelativePhase, 0) => {
                emit_action_gadget(&mut operations, last_control, last_ancilla, target);
            }
            (DirtyVChainMode::RelativePhase, _) => {
                emit_reset_gadget(&mut operations, last_control, last_ancilla, target);
            }
        }

        for i in (0..controls.len() - 3).rev() {
            emit_action_gadget(
                &mut operations,
                controls[i + 2],
                used_ancillas[i],
                used_ancillas[i + 1],
            );
        }

        emit_relative_phase_toffoli(&mut operations, controls[0], controls[1], used_ancillas[0])?;

        for i in 0..controls.len() - 3 {
            emit_reset_gadget(
                &mut operations,
                controls[i + 2],
                used_ancillas[i],
                used_ancillas[i + 1],
            );
        }
    }

    Ok(operations)
}

/// Appends the forward half of an I15 relative-phase Toffoli gadget.
fn emit_action_gadget(
    operations: &mut Vec<ValueOperation>,
    first_control: Qubit,
    second_control: Qubit,
    target: Qubit,
) {
    operations.push(ValueOperation::from_standard(StandardGate::H, [target], []));
    operations.push(ValueOperation::from_standard(StandardGate::T, [target], []));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [first_control, target],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::TDG,
        [target],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [second_control, target],
        [],
    ));
}

/// Appends the inverse of [`emit_action_gadget`].
fn emit_reset_gadget(
    operations: &mut Vec<ValueOperation>,
    first_control: Qubit,
    second_control: Qubit,
    target: Qubit,
) {
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [second_control, target],
        [],
    ));
    operations.push(ValueOperation::from_standard(StandardGate::T, [target], []));
    operations.push(ValueOperation::from_standard(
        StandardGate::CX,
        [first_control, target],
        [],
    ));
    operations.push(ValueOperation::from_standard(
        StandardGate::TDG,
        [target],
        [],
    ));
    operations.push(ValueOperation::from_standard(StandardGate::H, [target], []));
}
