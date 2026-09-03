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

use crate::circuit::{Instruction, StandardGate};
use crate::compile::knowledge::KnowledgeInstructionKey;
use crate::device::PhysicalQubit;
use smallvec::SmallVec;

/// A parameter-independent gate state on exact ordered physical qargs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DeviceGateState {
    pub(crate) instruction: KnowledgeInstructionKey,
    pub(crate) ordered_qargs: SmallVec<[PhysicalQubit; 2]>,
}

impl DeviceGateState {
    pub(crate) fn standard(
        gate: StandardGate,
        ordered_qargs: SmallVec<[PhysicalQubit; 2]>,
    ) -> Self {
        Self {
            instruction: KnowledgeInstructionKey::Standard(gate),
            ordered_qargs,
        }
    }

    pub(crate) fn from_instruction(
        instruction: &Instruction,
        ordered_qargs: SmallVec<[PhysicalQubit; 2]>,
    ) -> Option<Self> {
        Some(Self {
            instruction: KnowledgeInstructionKey::from_instruction(instruction)?,
            ordered_qargs,
        })
    }
}
