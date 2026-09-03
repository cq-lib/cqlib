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

//! Audited equivalence templates for direction-sensitive two-qubit gates.

use super::DeviceGateState;
use crate::circuit::StandardGate;
use smallvec::{SmallVec, smallvec};

/// A direction equivalence admitted to exact device-lowering planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DirectionTemplate {
    Cx,
    Rzx,
    Symmetric(StandardGate),
}

impl DirectionTemplate {
    pub(super) fn stable_name(self) -> String {
        match self {
            Self::Cx => "direction_reverse_CX".to_string(),
            Self::Rzx => "direction_reverse_RZX".to_string(),
            Self::Symmetric(gate) => format!("direction_reverse_{}", gate.name()),
        }
    }

    pub(super) fn child_states(self, parent: &DeviceGateState) -> Vec<DeviceGateState> {
        debug_assert_eq!(parent.ordered_qargs.len(), 2);
        let q0 = parent.ordered_qargs[0];
        let q1 = parent.ordered_qargs[1];
        match self {
            Self::Cx | Self::Rzx => vec![
                DeviceGateState::standard(StandardGate::H, smallvec![q0]),
                DeviceGateState::standard(StandardGate::H, smallvec![q1]),
                DeviceGateState {
                    instruction: parent.instruction.clone(),
                    ordered_qargs: smallvec![q1, q0],
                },
                DeviceGateState::standard(StandardGate::H, smallvec![q0]),
                DeviceGateState::standard(StandardGate::H, smallvec![q1]),
            ],
            Self::Symmetric(_) => vec![DeviceGateState {
                instruction: parent.instruction.clone(),
                ordered_qargs: smallvec![q1, q0],
            }],
        }
    }
}

pub(super) fn candidates(state: &DeviceGateState) -> SmallVec<[DirectionTemplate; 1]> {
    if state.ordered_qargs.len() != 2 {
        return SmallVec::new();
    }
    let template = match state.instruction {
        crate::compile::knowledge::KnowledgeInstructionKey::Standard(StandardGate::CX) => {
            DirectionTemplate::Cx
        }
        crate::compile::knowledge::KnowledgeInstructionKey::Standard(StandardGate::RZX) => {
            DirectionTemplate::Rzx
        }
        crate::compile::knowledge::KnowledgeInstructionKey::Standard(gate)
            if gate.is_invariant_under_operand_swap() =>
        {
            DirectionTemplate::Symmetric(gate)
        }
        _ => return SmallVec::new(),
    };
    smallvec![template]
}
