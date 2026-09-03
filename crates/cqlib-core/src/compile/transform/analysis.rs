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

//! Structural circuit analysis shared by compiler transforms.
//!
//! The analysis reports stable facts about the current IR shape so workflow and
//! transforms can skip inapplicable work without rescanning the full operation
//! tree repeatedly.

use crate::circuit::{
    Circuit, ClassicalControlOp, Directive, Instruction, Operation, StandardGate,
};

/// Structural facts about a circuit relevant to compiler transforms.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CircuitAnalysis {
    pub has_classical_data: bool,
    pub has_classical_control: bool,
    pub has_measurement: bool,
    pub has_classical_values: bool,
    pub has_classical_vars: bool,
    pub has_runtime_classical: bool,
    pub needs_classical_handle_preservation: bool,
    pub has_circuit_gate_definitions: bool,
    pub has_unitary_circuit_definitions: bool,
    pub has_unitary_gates: bool,
    pub has_mc_gates: bool,
}

impl CircuitAnalysis {
    /// Computes structural facts for `circuit`.
    pub fn analyze(circuit: &Circuit) -> Self {
        WorkflowCircuitAnalysis::analyze(circuit).public
    }
}

/// Allocation-free workflow facts that are intentionally not part of the
/// public compiler API.
///
/// Circuit-backed definitions remain opaque until the definition pass expands
/// them. A changed circuit invalidates this summary, so operations revealed by
/// expansion are discovered by the next analysis of the current IR revision.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WorkflowCircuitAnalysis {
    public: CircuitAnalysis,
    standard_gates: StandardGateSet,
    instruction_kinds: InstructionKindSet,
    has_gate_like_operation_over_two_qubits: bool,
    /// Exact correction remains target-dependent; this fact only identifies
    /// circuits for which physical operand direction can be relevant.
    has_direction_sensitive_two_qubit_operation: bool,
    has_reset: bool,
    has_control_flow: bool,
}

impl WorkflowCircuitAnalysis {
    pub(crate) fn analyze(circuit: &Circuit) -> Self {
        let mut analysis = Self {
            public: CircuitAnalysis {
                has_classical_values: !circuit.classical_values().is_empty(),
                has_classical_vars: !circuit.classical_vars().is_empty(),
                ..CircuitAnalysis::default()
            },
            ..Self::default()
        };
        analysis.scan_operations(circuit.operations());
        analysis.public.has_runtime_classical = analysis.public.has_classical_data
            || analysis.public.has_classical_control
            || analysis.public.has_classical_values
            || analysis.public.has_classical_vars;
        analysis.public.needs_classical_handle_preservation = analysis.public.has_runtime_classical;
        analysis
    }

    pub(crate) const fn public(&self) -> &CircuitAnalysis {
        &self.public
    }

    pub(crate) const fn has_unexpanded_definitions(&self) -> bool {
        self.public.has_circuit_gate_definitions || self.public.has_unitary_circuit_definitions
    }

    pub(crate) const fn has_unitary_gates(&self) -> bool {
        self.public.has_unitary_gates
    }

    pub(crate) const fn has_mc_gates(&self) -> bool {
        self.public.has_mc_gates
    }

    pub(crate) const fn has_gate_like_operation_over_two_qubits(&self) -> bool {
        self.has_gate_like_operation_over_two_qubits
    }

    pub(crate) fn standard_gates(&self) -> impl Iterator<Item = StandardGate> + '_ {
        StandardGate::all()
            .iter()
            .copied()
            .filter(|gate| self.standard_gates.contains(*gate))
    }

    pub(crate) const fn has_extended_gate_like_operations(&self) -> bool {
        self.instruction_kinds.contains(InstructionKind::McGate)
            || self
                .instruction_kinds
                .contains(InstructionKind::UnitaryGate)
            || self
                .instruction_kinds
                .contains(InstructionKind::CircuitGate)
    }

    fn scan_operations(&mut self, operations: &[Operation]) {
        for operation in operations {
            self.scan_operation(operation);
        }
    }

    fn scan_operation(&mut self, operation: &Operation) {
        match &operation.instruction {
            Instruction::Standard(gate) => {
                self.instruction_kinds.insert(InstructionKind::Standard);
                self.standard_gates.insert(*gate);
                self.has_gate_like_operation_over_two_qubits |= operation.qubits.len() > 2;
                self.has_direction_sensitive_two_qubit_operation |=
                    gate.num_qubits() == 2 && !gate.is_invariant_under_operand_swap();
            }
            Instruction::ClassicalData(op) => {
                self.instruction_kinds
                    .insert(InstructionKind::ClassicalData);
                self.public.has_classical_data = true;
                if op.result().is_some() {
                    self.public.has_measurement = true;
                }
            }
            Instruction::ClassicalControl(op) => {
                self.instruction_kinds
                    .insert(InstructionKind::ClassicalControl);
                self.public.has_classical_control = true;
                self.has_control_flow = true;
                self.scan_control_flow(op);
            }
            Instruction::CircuitGate(_) => {
                self.instruction_kinds.insert(InstructionKind::CircuitGate);
                self.public.has_circuit_gate_definitions = true;
                self.has_gate_like_operation_over_two_qubits |= operation.qubits.len() > 2;
            }
            Instruction::UnitaryGate(gate) => {
                self.instruction_kinds.insert(InstructionKind::UnitaryGate);
                self.public.has_unitary_gates = true;
                if gate.circuit().is_some() {
                    self.public.has_unitary_circuit_definitions = true;
                }
                self.has_gate_like_operation_over_two_qubits |= operation.qubits.len() > 2;
            }
            Instruction::McGate(_) => {
                self.instruction_kinds.insert(InstructionKind::McGate);
                self.public.has_mc_gates = true;
                self.has_gate_like_operation_over_two_qubits |= operation.qubits.len() > 2;
            }
            Instruction::Directive(directive) => {
                self.instruction_kinds.insert(InstructionKind::Directive);
                match directive {
                    Directive::Measure => self.public.has_measurement = true,
                    Directive::Reset => self.has_reset = true,
                    Directive::Barrier => {}
                }
            }
            Instruction::Delay => self.instruction_kinds.insert(InstructionKind::Delay),
        }
    }

    fn scan_control_flow(&mut self, op: &ClassicalControlOp) {
        match op {
            ClassicalControlOp::If(op) => {
                self.scan_operations(op.then_body().operations());
                if let Some(body) = op.else_body() {
                    self.scan_operations(body.operations());
                }
            }
            ClassicalControlOp::While(op) => self.scan_operations(op.body().operations()),
            ClassicalControlOp::For(op) => self.scan_operations(op.body().operations()),
            ClassicalControlOp::Switch(op) => {
                for case in op.cases() {
                    self.scan_operations(case.body().operations());
                }
                if let Some(body) = op.default() {
                    self.scan_operations(body.operations());
                }
            }
            ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct StandardGateSet(u64);

impl StandardGateSet {
    fn insert(&mut self, gate: StandardGate) {
        self.0 |= 1_u64 << gate as u8;
    }

    const fn contains(self, gate: StandardGate) -> bool {
        self.0 & (1_u64 << gate as u8) != 0
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstructionKind {
    Standard,
    McGate,
    UnitaryGate,
    CircuitGate,
    Directive,
    ClassicalData,
    ClassicalControl,
    Delay,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct InstructionKindSet(u16);

impl InstructionKindSet {
    fn insert(&mut self, kind: InstructionKind) {
        self.0 |= 1_u16 << kind as u8;
    }

    const fn contains(self, kind: InstructionKind) -> bool {
        self.0 & (1_u16 << kind as u8) != 0
    }
}

#[cfg(test)]
#[path = "./analysis_test.rs"]
mod analysis_test;
