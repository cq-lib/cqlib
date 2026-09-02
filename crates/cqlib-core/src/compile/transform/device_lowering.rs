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

//! Exact ordered-qargs lowering to a concrete device instruction set.
//!
//! Routing deliberately uses an undirected connectivity projection. This
//! transform runs after routing and closes the remaining ISA gap: every
//! gate-like operation is either retained as an exact native capability or
//! lowered through a finite, recursively verified plan whose leaves are exact
//! device capabilities. A separate device verifier remains the terminal safety
//! boundary for each finalized candidate. Enhanced workflow selection may run
//! afterward, but it only chooses among candidates that crossed that boundary.
//!
//! Emission fuses buffered one-qubit runs on each qubit before they reach the
//! output: exact peephole merges (`RZ` accumulation, `X2P`/`X2M` pairs,
//! inverse-pair cancellation) run first, and a run is then replaced by a
//! re-synthesized `RZ`/`U` plan only when the fused form has both strictly
//! fewer native leaves and strictly better calibrated physical cost. Fusion
//! therefore never grows the lowered circuit or trades fidelity for gate count.

use crate::circuit::{
    Circuit, ClassicalControlOp, Instruction, Operation, Parameter, ParameterValue, Qubit,
    StandardGate, ValueClassicalControlOp, ValueControlBody, ValueInstruction, ValueOperation,
    ValueSwitchCase,
};
use crate::compile::CompilerError;
use crate::compile::device_planning::{
    DeviceGateState, DevicePhysicalCost, DevicePlanSnapshot, DevicePlanningSession,
    DirectionTemplate, NativePlanAvailability, NativePlanLeaf, PlanChoice, PlanTemplate,
    SelectedNativePlan,
};
use crate::compile::knowledge::{
    ConcreteOperationView, KnowledgeInstructionKey, RuleLibrary, instantiate_target,
    rule_matches_operations,
};
use crate::compile::transform::decompose::unitary::synthesize_numeric_1q_unitary;
use crate::compile::transform::lowering_support::{LoweringTarget, OperationSequenceLowerer};
use crate::compile::transform::rebuild::{CircuitRebuildContext, ClassicalRemap};
use crate::compile::transform::{CircuitAnalysis, TransformOutcome, Transformer};
use crate::device::{Device, PhysicalQubit};
use ndarray::Array2;
use num_complex::Complex64;
use smallvec::{SmallVec, smallvec};
use std::collections::{BTreeMap, HashSet};
use std::f64::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

const PHASE_EPS: f64 = 1e-12;

#[derive(Debug, Clone)]
pub(super) struct LowerableOperation {
    pub(super) instruction: Instruction,
    pub(super) qubits: SmallVec<[Qubit; 3]>,
    pub(super) params: SmallVec<[ParameterValue; 1]>,
    pub(super) label: Option<Box<str>>,
}

/// Lowers a routed physical circuit to one device's exact native instruction
/// capabilities, including ordered qargs and local capability overrides.
#[derive(Debug, Clone, Copy)]
pub struct DeviceLowerer<'a> {
    device: &'a Device,
    planning_session: Option<&'a DevicePlanningSession>,
}

impl<'a> DeviceLowerer<'a> {
    /// Creates a lowerer borrowing one immutable device capability model.
    pub const fn new(device: &'a Device) -> Self {
        Self {
            device,
            planning_session: None,
        }
    }

    /// Reuses a compile-scoped calibration snapshot and prepared root cache.
    pub(crate) const fn with_session(
        device: &'a Device,
        planning_session: &'a DevicePlanningSession,
    ) -> Self {
        Self {
            device,
            planning_session: Some(planning_session),
        }
    }

    /// Returns the device whose exact capabilities define lowering leaves.
    pub const fn device(&self) -> &'a Device {
        self.device
    }
}

impl Transformer for DeviceLowerer<'_> {
    fn name(&self) -> &'static str {
        "device_lowering"
    }

    fn transform(
        &self,
        circuit: &Circuit,
        _analysis: Option<&CircuitAnalysis>,
    ) -> Result<TransformOutcome, CompilerError> {
        let library = RuleLibrary::builtin_rules()
            .map_err(|error| CompilerError::InvariantViolation(error.to_string()))?;
        let scan = collect_root_states(circuit)?;
        let mut roots = scan.states;
        roots.sort();
        roots.dedup();
        // Fused lowering can synthesize buffered one-qubit runs into `RZ` or
        // `U` operations on any circuit qubit that actually carries gates, so
        // the planner needs candidate plans for those states. They are kept
        // separate from circuit roots: they must not count toward the
        // already-native fast path (a missing `U` plan must not block it), and
        // unused qubits are excluded so planning does not grow with device
        // width on sparse circuits.
        let mut planner_roots = roots.clone();
        for qubit in &scan.used_qubits {
            for gate in [StandardGate::RZ, StandardGate::U] {
                if let Some(instruction) =
                    KnowledgeInstructionKey::from_instruction(&Instruction::Standard(gate))
                {
                    planner_roots.push(DeviceGateState {
                        instruction,
                        ordered_qargs: smallvec![PhysicalQubit::from_qubit(*qubit)],
                    });
                }
            }
        }
        planner_roots.sort();
        planner_roots.dedup();
        let owned_session;
        let planning_session = if let Some(session) = self.planning_session {
            session
        } else {
            owned_session = DevicePlanningSession::new(self.device);
            &owned_session
        };
        let plans = planning_session.prepare(planner_roots.iter().cloned())?;
        // Fast path: when no phase folding is pending, the planner selects the
        // native leaf for every circuit root state, and no potentially
        // fusible one-qubit run exists, lowering (and fused emission) is a
        // no-op and the full circuit rebuild can be skipped. This check must
        // go through the planner: a natively supported gate can still lower
        // further when a cheaper direction or template realization exists.
        let all_roots_native = roots.iter().all(|state| {
            matches!(
                plans.selected_plan(state).map(|plan| plan.choice),
                Some(PlanChoice::Native)
            )
        });
        if !scan.has_gphase && all_roots_native && !has_fusible_one_qubit_run(circuit) {
            return Ok(TransformOutcome::Unchanged);
        }
        DeviceCircuitLowerer::run(circuit, self.device, library, &plans)
    }
}

struct DeviceCircuitLowerer<'a> {
    source: &'a Circuit,
    device: &'a Device,
    library: &'a RuleLibrary,
    plans: &'a DevicePlanSnapshot,
    rebuild: CircuitRebuildContext,
    changed: bool,
    /// Pending native one-qubit leaves buffered per qubit for run fusion.
    pending: BTreeMap<Qubit, Vec<ValueOperation>>,
    /// Set while a synthesized fusion form is being emitted so its leaves
    /// bypass the pending buffer (otherwise emission would fuse again).
    emit_fused: bool,
}

impl<'a> DeviceCircuitLowerer<'a> {
    fn run(
        source: &'a Circuit,
        device: &'a Device,
        library: &'a RuleLibrary,
        plans: &'a DevicePlanSnapshot,
    ) -> Result<TransformOutcome, CompilerError> {
        let rebuild = CircuitRebuildContext::new(source);
        let root_classical = rebuild.root_classical().clone();
        let mut lowerer = Self {
            source,
            device,
            library,
            plans,
            rebuild,
            changed: false,
            pending: BTreeMap::new(),
            emit_fused: false,
        };
        let mut operations = Vec::with_capacity(source.operations().len());
        let mut phase_delta = Parameter::from(0.0);
        lowerer.lower_sequence(
            source.operations(),
            &root_classical,
            LoweringTarget::top_level(&mut operations, &mut phase_delta),
        )?;
        let mut target = LoweringTarget::top_level(&mut operations, &mut phase_delta);
        lowerer.flush_all_pending(&mut target)?;
        let global_phase = &source.global_phase() + &phase_delta;
        let circuit = lowerer
            .rebuild
            .finish(source.qubits(), operations, global_phase)?;
        Ok(if lowerer.changed {
            TransformOutcome::Changed(circuit)
        } else {
            TransformOutcome::Unchanged
        })
    }

    fn lower_operation(
        &mut self,
        operation: &Operation,
        classical_remap: &ClassicalRemap,
        target: &mut LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        match &operation.instruction {
            Instruction::Standard(_) | Instruction::McGate(_) => {
                let params =
                    CircuitRebuildContext::resolve_source_params(self.source, &operation.params)?;
                self.lower_gate_like(
                    LowerableOperation {
                        instruction: operation.instruction.clone(),
                        qubits: operation.qubits.clone(),
                        params,
                        label: operation.label.clone(),
                    },
                    target,
                )
            }
            Instruction::ClassicalControl(control) => {
                // Control-flow bodies are hard fusion boundaries: flush the
                // enclosing scope's pending runs before lowering any body so
                // nothing is delayed across (or into) a structured body.
                self.flush_all_pending(target)?;
                let instruction = self.lower_control_flow(control, classical_remap)?;
                self.push_operation(
                    ValueOperation {
                        qubits: instruction.used_qubits().into_iter().collect(),
                        instruction: ValueInstruction::ClassicalControl(instruction),
                        params: SmallVec::new(),
                        label: operation.label.clone(),
                    },
                    target,
                )?;
                Ok(())
            }
            Instruction::UnitaryGate(_) | Instruction::CircuitGate(_) => {
                Err(CompilerError::InvalidInput(format!(
                    "cannot lower {} to device instructions: definitions and unitaries must be decomposed first",
                    operation.instruction
                )))
            }
            Instruction::ClassicalData(_) | Instruction::Directive(_) | Instruction::Delay => {
                let operation = self.rebuild.remap_preserved_operation(
                    self.source,
                    operation,
                    classical_remap,
                )?;
                self.push_operation(operation, target)?;
                Ok(())
            }
        }
    }

    fn lower_gate_like(
        &mut self,
        operation: LowerableOperation,
        target: &mut LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        if matches!(
            operation.instruction,
            Instruction::Standard(StandardGate::GPhase)
        ) {
            self.changed = true;
            target.accumulate_phase(gphase_param(&operation)?);
            return Ok(());
        }

        let ordered_qargs = operation
            .qubits
            .iter()
            .copied()
            .map(PhysicalQubit::from_qubit)
            .collect();
        let state = DeviceGateState::from_instruction(&operation.instruction, ordered_qargs)
            .ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "missing device-lowering key for {}",
                    operation.instruction
                ))
            })?;
        let Some(plan) = self.plans.selected_plan(&state) else {
            return match self.plans.availability(&state) {
                Some(NativePlanAvailability::Unsupported(failure)) => Err(
                    CompilerError::DeviceLoweringFailed(failure.as_ref().clone()),
                ),
                _ => Err(CompilerError::InvariantViolation(format!(
                    "prepared device state has no selected plan or failure: {state:?}"
                ))),
            };
        };
        self.lower_gate_like_with_plan(operation, state, plan, target)
    }

    fn lower_gate_like_with_plan(
        &mut self,
        operation: LowerableOperation,
        state: DeviceGateState,
        plan: Arc<SelectedNativePlan>,
        target: &mut LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        if plan.state != state {
            return Err(CompilerError::InvariantViolation(format!(
                "device plan targets {:?}, but lowering requested {state:?}",
                plan.state
            )));
        }
        match plan.choice {
            PlanChoice::Native => {
                if !self
                    .device
                    .supports_native_instruction(&operation.instruction, &state.ordered_qargs)
                {
                    return Err(CompilerError::InvariantViolation(format!(
                        "device planner selected unsupported native leaf {} on {:?}",
                        operation.instruction, state.ordered_qargs
                    )));
                }
                self.push_operation(
                    ValueOperation {
                        instruction: ValueInstruction::from_instruction(operation.instruction),
                        qubits: operation.qubits,
                        params: operation.params,
                        label: operation.label,
                    },
                    target,
                )?;
            }
            PlanChoice::Template(template) => {
                self.changed = true;
                let child_plans = plan.children.to_vec();
                let replacements = match template {
                    PlanTemplate::Rule(rule_id) => {
                        let rule = self.library.get(rule_id).ok_or_else(|| {
                            CompilerError::InvariantViolation(format!(
                                "missing selected device-lowering rule {rule_id:?}"
                            ))
                        })?;
                        instantiate_rule(rule, &operation)?
                    }
                    PlanTemplate::Direction(template) => {
                        instantiate_direction_template(template, &operation)
                    }
                };
                let mut children = child_plans.into_iter();
                for replacement in replacements {
                    if matches!(
                        replacement.instruction,
                        Instruction::Standard(StandardGate::GPhase)
                    ) {
                        self.lower_gate_like(replacement, target)?;
                        continue;
                    }
                    let child = children.next().ok_or_else(|| {
                        CompilerError::InvariantViolation(format!(
                            "device plan for {state:?} emitted more non-phase children than planned"
                        ))
                    })?;
                    let ordered_qargs = replacement
                        .qubits
                        .iter()
                        .copied()
                        .map(PhysicalQubit::from_qubit)
                        .collect();
                    let child_state =
                        DeviceGateState::from_instruction(&replacement.instruction, ordered_qargs)
                            .ok_or_else(|| {
                                CompilerError::InvariantViolation(format!(
                                    "missing device-lowering key for planned child {}",
                                    replacement.instruction
                                ))
                            })?;
                    self.lower_gate_like_with_plan(replacement, child_state, child, target)?;
                }
                if children.next().is_some() {
                    return Err(CompilerError::InvariantViolation(format!(
                        "device plan for {state:?} emitted fewer non-phase children than planned"
                    )));
                }
            }
        }
        Ok(())
    }

    fn lower_control_flow(
        &mut self,
        control: &ClassicalControlOp,
        classical_remap: &ClassicalRemap,
    ) -> Result<ValueClassicalControlOp, CompilerError> {
        Ok(match control {
            ClassicalControlOp::If(op) => ValueClassicalControlOp::If {
                condition: classical_remap.remap_expr(op.condition())?,
                then_body: self.lower_body(op.then_body(), classical_remap)?,
                else_body: op
                    .else_body()
                    .map(|body| self.lower_body(body, classical_remap))
                    .transpose()?,
            },
            ClassicalControlOp::While(op) => ValueClassicalControlOp::While {
                condition: classical_remap.remap_expr(op.condition())?,
                body: self.lower_body(op.body(), classical_remap)?,
            },
            ClassicalControlOp::For(op) => ValueClassicalControlOp::For {
                var: classical_remap.remap_var(op.var())?,
                start: classical_remap.remap_expr(op.start())?,
                stop: classical_remap.remap_expr(op.stop())?,
                step: classical_remap.remap_expr(op.step())?,
                body: self.lower_body(op.body(), classical_remap)?,
            },
            ClassicalControlOp::Switch(op) => ValueClassicalControlOp::Switch {
                target: classical_remap.remap_expr(op.target())?,
                cases: op
                    .cases()
                    .iter()
                    .map(|case| {
                        Ok(ValueSwitchCase::new(
                            case.value(),
                            self.lower_body(case.body(), classical_remap)?,
                        ))
                    })
                    .collect::<Result<_, CompilerError>>()?,
                default: op
                    .default()
                    .map(|body| self.lower_body(body, classical_remap))
                    .transpose()?,
            },
            ClassicalControlOp::Break => ValueClassicalControlOp::Break,
            ClassicalControlOp::Continue => ValueClassicalControlOp::Continue,
        })
    }

    fn lower_body(
        &mut self,
        body: &crate::circuit::ControlBody,
        classical_remap: &ClassicalRemap,
    ) -> Result<ValueControlBody, CompilerError> {
        let mut output = Vec::with_capacity(body.operations().len());
        let mut phase_delta = Parameter::from(0.0);
        self.lower_sequence(
            body.operations(),
            classical_remap,
            LoweringTarget::control_flow_body(&mut output, &mut phase_delta),
        )?;
        self.flush_all_pending(&mut LoweringTarget::control_flow_body(
            &mut output,
            &mut phase_delta,
        ))?;
        self.prepend_body_phase(&mut output, phase_delta);
        Ok(ValueControlBody::new(output))
    }

    fn prepend_body_phase(&mut self, body: &mut Vec<ValueOperation>, phase: Parameter) {
        if phase.is_zero() {
            return;
        }
        body.insert(
            0,
            ValueOperation {
                instruction: ValueInstruction::from_instruction(Instruction::Standard(
                    StandardGate::GPhase,
                )),
                qubits: SmallVec::new(),
                params: smallvec![ParameterValue::from(phase)],
                label: None,
            },
        );
    }

    /// Emits one operation, buffering native one-qubit leaves for run fusion.
    ///
    /// Buffering delays emission past operations on disjoint qubits, which is
    /// a commutation on disjoint supports. Anything touching a buffered qubit
    /// (a multi-qubit leaf, a directive, a delay, or a measurement) flushes
    /// exactly that qubit; global boundaries (control flow, empty-scope
    /// barriers, classical stores) flush every qubit in deterministic order.
    fn push_operation(
        &mut self,
        operation: ValueOperation,
        target: &mut LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        if self.emit_fused {
            self.push_direct(operation, target);
            return Ok(());
        }
        if let Some(qubit) = bufferable_one_qubit_leaf(&operation) {
            self.pending.entry(qubit).or_default().push(operation);
            return Ok(());
        }
        match boundary_qubits(&operation) {
            BoundaryQubits::Touched(qubits) => {
                for qubit in qubits {
                    self.flush_qubit(qubit, target)?;
                }
            }
            BoundaryQubits::All => self.flush_all_pending(target)?,
        }
        self.push_direct(operation, target);
        Ok(())
    }

    fn push_direct(&mut self, operation: ValueOperation, target: &mut LoweringTarget<'_>) {
        target.push(operation);
    }

    fn flush_qubit(
        &mut self,
        qubit: Qubit,
        target: &mut LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        let Some(run) = self.pending.remove(&qubit) else {
            return Ok(());
        };
        self.emit_fused_run(qubit, run, target)
    }

    fn flush_all_pending(&mut self, target: &mut LoweringTarget<'_>) -> Result<(), CompilerError> {
        let qubits = self.pending.keys().copied().collect::<Vec<_>>();
        for qubit in qubits {
            self.flush_qubit(qubit, target)?;
        }
        Ok(())
    }

    /// Emits one buffered one-qubit run, replacing it with a shorter fused
    /// synthesis when the planner offers one on this qubit.
    ///
    /// Fusion is exact: the buffered run's 2x2 unitary is recomposed from the
    /// gate matrices and re-synthesized. A candidate is emitted only when it
    /// is strictly shorter than the buffered run, so fusion never grows the
    /// lowered circuit; otherwise the run passes through unchanged.
    fn emit_fused_run(
        &mut self,
        qubit: Qubit,
        run: Vec<ValueOperation>,
        target: &mut LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        let run = self.peephole_merge_run(qubit, run, target);
        if run.is_empty() {
            return Ok(());
        }
        if run.len() == 1 {
            self.push_direct(run.into_iter().next().expect("one operation"), target);
            return Ok(());
        }
        let candidate = one_qubit_run_matrix(&run)
            .and_then(|matrix| synthesize_numeric_1q_unitary(&matrix).ok())
            .and_then(|decomposition| self.fused_form(qubit, decomposition, &run));
        match candidate {
            Some(FusedForm::Identity { phase }) => {
                self.changed = true;
                target.accumulate_phase(Parameter::from(phase));
            }
            Some(FusedForm::Rz { angle, phase, plan }) => {
                self.changed = true;
                target.accumulate_phase(Parameter::from(phase));
                self.emit_via_plan(
                    StandardGate::RZ,
                    qubit,
                    smallvec![ParameterValue::Fixed(angle)],
                    plan,
                    target,
                )?;
            }
            Some(FusedForm::U {
                decomposition,
                plan,
            }) => {
                self.changed = true;
                target.accumulate_phase(Parameter::from(decomposition.global_phase));
                self.emit_via_plan(
                    StandardGate::U,
                    qubit,
                    smallvec![
                        ParameterValue::Fixed(decomposition.theta),
                        ParameterValue::Fixed(decomposition.phi),
                        ParameterValue::Fixed(decomposition.lambda),
                    ],
                    plan,
                    target,
                )?;
            }
            None => {
                for operation in run {
                    self.push_direct(operation, target);
                }
            }
        }
        Ok(())
    }

    /// Applies exact intra-run peephole merges before whole-run synthesis:
    /// `RZ` angle accumulation, `X2P`/`X2M` pairs to `X` when the planner's
    /// realization of `X` on the qubit is the native gate and strictly better
    /// than the pair (their scalar factors fold into `phase_delta`), and
    /// exact inverse-pair cancellation.
    fn peephole_merge_run(
        &mut self,
        qubit: Qubit,
        run: Vec<ValueOperation>,
        target: &mut LoweringTarget<'_>,
    ) -> Vec<ValueOperation> {
        let mut merged: Vec<ValueOperation> = Vec::with_capacity(run.len());
        for operation in run {
            let Some(last) = merged.len().checked_sub(1) else {
                merged.push(operation);
                continue;
            };
            // RZ(a) · RZ(b) = RZ(a+b) exactly; a zero sum cancels the pair.
            if let (Some(left), Some(right)) =
                (fixed_rz_angle(&merged[last]), fixed_rz_angle(&operation))
            {
                let angle = left + right;
                merged.pop();
                if angle.abs() > PHASE_EPS {
                    merged.push(rz_operation(qubit, angle));
                }
                self.changed = true;
                continue;
            }
            match (x2_variant(&merged[last]), x2_variant(&operation)) {
                (Some(plus), Some(other)) if plus == other => {
                    // X2P² = e^{-iπ/2}·X and X2M² = e^{+iπ/2}·X. Merge only
                    // when the planner realizes X on this qubit as the native
                    // gate and that realization is strictly better than the
                    // pair it replaces (a noisy native X must not win).
                    let pair_cost = self.pair_leaf_cost(&merged[last], &operation);
                    let native_x_better = self
                        .plan_info(StandardGate::X, PhysicalQubit::from_qubit(qubit))
                        .is_some_and(|info| {
                            matches!(info.plan.choice, PlanChoice::Native)
                                && info.leaf_count == 1
                                && pair_cost
                                    .is_some_and(|pair| info.cost.strictly_better_than(pair))
                        });
                    if !native_x_better {
                        merged.push(operation);
                        continue;
                    }
                    merged.pop();
                    let phase = if plus { -FRAC_PI_2 } else { FRAC_PI_2 };
                    target.accumulate_phase(Parameter::from(phase));
                    merged.push(x_operation(qubit));
                    self.changed = true;
                }
                (Some(_), Some(_)) => {
                    // X2P · X2M = X2M · X2P = I exactly.
                    merged.pop();
                    self.changed = true;
                }
                _ => {
                    if is_plain_x(&merged[last]) && is_plain_x(&operation) {
                        // X · X = I exactly.
                        merged.pop();
                        self.changed = true;
                        continue;
                    }
                    merged.push(operation);
                }
            }
        }
        merged
    }

    /// Costs two adjacent buffered operations as native leaves.
    fn pair_leaf_cost(
        &self,
        first: &ValueOperation,
        second: &ValueOperation,
    ) -> Option<DevicePhysicalCost> {
        let leaves = [first, second]
            .into_iter()
            .map(|operation| self.native_leaf(operation))
            .collect::<Option<Vec<_>>>()?;
        Some(self.plans.leaves_physical_cost(&leaves))
    }

    /// Selects the shortest exact fused form for one buffered run, or `None`
    /// when no candidate beats simply emitting the run.
    fn fused_form(
        &self,
        qubit: Qubit,
        decomposition: crate::compile::transform::decompose::unitary::OneQubitUnitaryDecomposition,
        run: &[ValueOperation],
    ) -> Option<FusedForm> {
        let physical = PhysicalQubit::from_qubit(qubit);
        let angle = decomposition.phi + decomposition.lambda;
        // With θ≈0 the run is M = e^{i·gp}·diag(1, e^{iγ}) for γ = φ + λ.
        // When γ ≡ 0 (mod 2π) the diagonal is exactly the identity, so the
        // whole run is the scalar e^{i·gp}: emit no gate and keep the phase.
        let wrapped = angle.rem_euclid(2.0 * PI);
        if decomposition.theta.abs() <= PHASE_EPS
            && (wrapped <= PHASE_EPS || (2.0 * PI - wrapped) <= PHASE_EPS)
        {
            return Some(FusedForm::Identity {
                phase: decomposition.global_phase,
            });
        }
        // Candidates must be both strictly shorter and strictly better under
        // the exact device physical cost used by the planner and native loop.
        // This keeps lowering leaf-monotone while rejecting shorter but noisier
        // (or slower) realizations.
        let run_cost = self.run_physical_cost(run)?;
        let mut best: Option<(DevicePhysicalCost, FusedForm)> = None;
        if decomposition.theta.abs() <= PHASE_EPS
            && let Some(info) = self.plan_info(StandardGate::RZ, physical).filter(|info| {
                fusion_candidate_is_admissible(info.leaf_count, info.cost, run.len(), run_cost)
            })
        {
            // RZ(γ) = diag(e^{-iγ/2}, e^{iγ/2}) while U(0,φ,λ) = diag(1, e^{i(φ+λ)}),
            // so the run is RZ(φ+λ) up to the extra phase (φ+λ)/2.
            best = Some((
                info.cost,
                FusedForm::Rz {
                    angle,
                    phase: decomposition.global_phase + angle / 2.0,
                    plan: info.plan,
                },
            ));
        }
        if let Some(info) = self.plan_info(StandardGate::U, physical).filter(|info| {
            fusion_candidate_is_admissible(info.leaf_count, info.cost, run.len(), run_cost)
        }) && best
            .as_ref()
            .is_none_or(|(best_cost, _)| info.cost.strictly_better_than(*best_cost))
        {
            best = Some((
                info.cost,
                FusedForm::U {
                    decomposition,
                    plan: info.plan,
                },
            ));
        }
        best.map(|(_, form)| form)
    }

    /// Returns the selected plan, exact physical cost, and native leaf count
    /// for a gate on one physical qubit, when the planner prepared that state.
    fn plan_info(&self, gate: StandardGate, physical: PhysicalQubit) -> Option<FusionPlanInfo> {
        let state =
            DeviceGateState::from_instruction(&Instruction::Standard(gate), smallvec![physical])?;
        let plan = self.plans.selected_plan(&state)?;
        let cost = plan.physical_cost;
        let leaf_count = plan.summary.leaves.len();
        Some(FusionPlanInfo {
            plan,
            cost,
            leaf_count,
        })
    }

    /// Costs one buffered run exactly as the planner would cost its leaves.
    fn run_physical_cost(&self, run: &[ValueOperation]) -> Option<DevicePhysicalCost> {
        let leaves = run
            .iter()
            .map(|operation| self.native_leaf(operation))
            .collect::<Option<Vec<_>>>()?;
        Some(self.plans.leaves_physical_cost(&leaves))
    }

    /// Builds the native plan leaf for one buffered operation, including its
    /// calibration data.
    fn native_leaf(&self, operation: &ValueOperation) -> Option<NativePlanLeaf> {
        let ValueInstruction::Instruction(instruction) = &operation.instruction else {
            return None;
        };
        let ordered_qargs: SmallVec<[PhysicalQubit; 2]> = operation
            .qubits
            .iter()
            .copied()
            .map(PhysicalQubit::from_qubit)
            .collect();
        let calibration = self
            .device
            .native_instruction_calibration(instruction, &ordered_qargs)?;
        Some(NativePlanLeaf {
            instruction: instruction.clone(),
            ordered_qargs,
            error_rate: calibration.error_rate,
            duration: calibration.duration,
        })
    }

    /// Emits one fused gate through its selected plan with buffering
    /// suspended, so the plan's own leaves reach the output directly.
    fn emit_via_plan(
        &mut self,
        gate: StandardGate,
        qubit: Qubit,
        params: SmallVec<[ParameterValue; 3]>,
        plan: Arc<SelectedNativePlan>,
        target: &mut LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        let instruction = Instruction::Standard(gate);
        let state = DeviceGateState::from_instruction(
            &instruction,
            smallvec![PhysicalQubit::from_qubit(qubit)],
        )
        .ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "missing device-lowering key for fused {gate:?} emission"
            ))
        })?;
        self.emit_fused = true;
        let result = self.lower_gate_like_with_plan(
            LowerableOperation {
                instruction,
                qubits: smallvec![qubit],
                params: SmallVec::from_vec(params.into_vec()),
                label: None,
            },
            state,
            plan,
            target,
        );
        self.emit_fused = false;
        result
    }
}

impl OperationSequenceLowerer for DeviceCircuitLowerer<'_> {
    fn lower_one_operation(
        &mut self,
        operation: &Operation,
        classical_remap: &ClassicalRemap,
        target: &mut LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        self.lower_operation(operation, classical_remap, target)
    }
}

/// One exact fused replacement for a buffered one-qubit run.
enum FusedForm {
    /// The run is a pure global phase; no operation is emitted.
    Identity { phase: f64 },
    /// The run is a single `RZ` up to `phase` (a pure z rotation).
    Rz {
        angle: f64,
        phase: f64,
        plan: Arc<SelectedNativePlan>,
    },
    /// The run is a general `U` up to its decomposition's global phase.
    U {
        decomposition: crate::compile::transform::decompose::unitary::OneQubitUnitaryDecomposition,
        plan: Arc<SelectedNativePlan>,
    },
}

/// Planner data needed to decide whether one fused realization is admissible.
struct FusionPlanInfo {
    plan: Arc<SelectedNativePlan>,
    cost: DevicePhysicalCost,
    leaf_count: usize,
}

/// Returns whether a non-identity fused realization satisfies lowering's
/// production monotonicity contract.
fn fusion_candidate_is_admissible(
    candidate_leaf_count: usize,
    candidate_cost: DevicePhysicalCost,
    run_leaf_count: usize,
    run_cost: DevicePhysicalCost,
) -> bool {
    candidate_leaf_count < run_leaf_count && candidate_cost.strictly_better_than(run_cost)
}

/// How an operation bounds the pending one-qubit buffers.
enum BoundaryQubits {
    /// Flush only the listed qubits.
    Touched(Vec<Qubit>),
    /// Flush every buffered qubit.
    All,
}

/// Classifies the flush scope of a non-bufferable operation.
///
/// Gate-like operations flush exactly their own qubits (operations on disjoint
/// qubits commute across them). Barriers flush their scope, with an empty
/// scope meaning a global barrier. Structured control flow and qubit-less
/// operations (including pure classical stores) are treated conservatively as
/// full flushes.
fn boundary_qubits(operation: &ValueOperation) -> BoundaryQubits {
    match &operation.instruction {
        ValueInstruction::ClassicalControl(_) => BoundaryQubits::All,
        ValueInstruction::Instruction(_) if operation.qubits.is_empty() => BoundaryQubits::All,
        ValueInstruction::Instruction(_) => {
            BoundaryQubits::Touched(operation.qubits.iter().copied().collect())
        }
    }
}

/// Returns the qubit of a native one-qubit leaf eligible for run buffering:
/// a standard one-qubit gate, unlabeled, with all parameters fixed numbers.
fn bufferable_one_qubit_leaf(operation: &ValueOperation) -> Option<Qubit> {
    let ValueInstruction::Instruction(Instruction::Standard(gate)) = &operation.instruction else {
        return None;
    };
    let fixed_numeric = operation
        .params
        .iter()
        .all(|param| matches!(param, ParameterValue::Fixed(value) if value.is_finite()));
    bufferable_one_qubit(
        *gate,
        &operation.qubits,
        operation.label.is_some(),
        fixed_numeric,
    )
}

/// Shared structural eligibility for source scanning and native-leaf buffering.
fn bufferable_one_qubit(
    gate: StandardGate,
    qubits: &[Qubit],
    has_label: bool,
    fixed_numeric: bool,
) -> Option<Qubit> {
    (gate.num_qubits() == 1 && qubits.len() == 1 && !has_label && fixed_numeric)
        .then_some(qubits[0])
}

/// Recomposes the exact 2x2 unitary of one buffered one-qubit run.
fn one_qubit_run_matrix(operations: &[ValueOperation]) -> Option<Array2<Complex64>> {
    let mut matrix = Array2::<Complex64>::eye(2);
    for operation in operations {
        let ValueInstruction::Instruction(Instruction::Standard(gate)) = &operation.instruction
        else {
            return None;
        };
        let params = operation
            .params
            .iter()
            .map(|param| match param {
                ParameterValue::Fixed(value) if value.is_finite() => Some(*value),
                ParameterValue::Fixed(_) | ParameterValue::Param(_) => None,
            })
            .collect::<Option<Vec<_>>>()?;
        matrix = gate.matrix(&params).ok()?.dot(&matrix);
    }
    Some(matrix)
}

/// Returns the fixed angle of a one-parameter `RZ` operation.
fn fixed_rz_angle(operation: &ValueOperation) -> Option<f64> {
    if !matches!(
        &operation.instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::RZ))
    ) {
        return None;
    }
    match operation.params.first() {
        Some(ParameterValue::Fixed(value)) if value.is_finite() => Some(*value),
        _ => None,
    }
}

fn rz_operation(qubit: Qubit, angle: f64) -> ValueOperation {
    ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::RZ)),
        qubits: smallvec![qubit],
        params: smallvec![ParameterValue::Fixed(angle)],
        label: None,
    }
}

fn x_operation(qubit: Qubit) -> ValueOperation {
    ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::X)),
        qubits: smallvec![qubit],
        params: SmallVec::new(),
        label: None,
    }
}

/// Returns whether the operation is `X2P` (true) or `X2M` (false).
fn x2_variant(operation: &ValueOperation) -> Option<bool> {
    match &operation.instruction {
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::X2P)) => Some(true),
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::X2M)) => Some(false),
        _ => None,
    }
}

fn is_plain_x(operation: &ValueOperation) -> bool {
    matches!(
        &operation.instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::X))
    )
}

/// Root gate states discovered in one circuit plus whether any `GPhase`
/// operation (which lowering always folds) was seen, plus the qubits that
/// carry at least one gate-like operation.
struct RootStateScan {
    states: Vec<DeviceGateState>,
    has_gphase: bool,
    used_qubits: Vec<Qubit>,
}

fn collect_root_states(circuit: &Circuit) -> Result<RootStateScan, CompilerError> {
    fn collect(
        operations: &[Operation],
        scan: &mut (HashSet<DeviceGateState>, bool, HashSet<Qubit>),
    ) {
        for operation in operations {
            match &operation.instruction {
                Instruction::Standard(StandardGate::GPhase) => scan.1 = true,
                Instruction::Standard(_) | Instruction::McGate(_) => {
                    scan.2.extend(operation.qubits.iter().copied());
                    if let Some(instruction) =
                        KnowledgeInstructionKey::from_instruction(&operation.instruction)
                    {
                        scan.0.insert(DeviceGateState {
                            instruction,
                            ordered_qargs: operation
                                .qubits
                                .iter()
                                .copied()
                                .map(PhysicalQubit::from_qubit)
                                .collect(),
                        });
                    }
                }
                Instruction::ClassicalControl(control) => match control {
                    ClassicalControlOp::If(op) => {
                        collect(op.then_body().operations(), scan);
                        if let Some(body) = op.else_body() {
                            collect(body.operations(), scan);
                        }
                    }
                    ClassicalControlOp::While(op) => collect(op.body().operations(), scan),
                    ClassicalControlOp::For(op) => collect(op.body().operations(), scan),
                    ClassicalControlOp::Switch(op) => {
                        for case in op.cases() {
                            collect(case.body().operations(), scan);
                        }
                        if let Some(body) = op.default() {
                            collect(body.operations(), scan);
                        }
                    }
                    ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
                },
                _ => {}
            }
        }
    }

    let mut scan = (HashSet::new(), false, HashSet::new());
    collect(circuit.operations(), &mut scan);
    Ok(RootStateScan {
        states: scan.0.into_iter().collect(),
        has_gphase: scan.1,
        used_qubits: scan.2.into_iter().collect(),
    })
}

/// Detects any source one-qubit run that fused emission may improve.
///
/// This mirrors the pending buffer's adjacency semantics: operations on other
/// qubits do not separate a run, operations touching the qubit do, and global
/// boundaries flush every run. Each structured-control body is scanned with
/// fresh state so mutually exclusive branches can never form a false run.
fn has_fusible_one_qubit_run(circuit: &Circuit) -> bool {
    fn bufferable_source_qubit(operation: &Operation) -> Option<Qubit> {
        let Instruction::Standard(gate) = &operation.instruction else {
            return None;
        };
        let fixed_numeric = operation
            .params
            .iter()
            .all(|param| matches!(param, crate::circuit::CircuitParam::Fixed(value) if value.is_finite()));
        bufferable_one_qubit(
            *gate,
            &operation.qubits,
            operation.label.is_some(),
            fixed_numeric,
        )
    }

    fn flush_source_boundary(operation: &Operation, pending: &mut HashSet<Qubit>) {
        if operation.qubits.is_empty() {
            pending.clear();
        } else {
            for qubit in &operation.qubits {
                pending.remove(qubit);
            }
        }
    }

    fn scan(operations: &[Operation]) -> bool {
        let mut pending = HashSet::<Qubit>::new();
        for operation in operations {
            match &operation.instruction {
                Instruction::Standard(_) | Instruction::McGate(_) => {
                    if let Some(qubit) = bufferable_source_qubit(operation) {
                        if !pending.insert(qubit) {
                            return true;
                        }
                    } else {
                        flush_source_boundary(operation, &mut pending);
                    }
                }
                Instruction::ClassicalControl(control) => {
                    pending.clear();
                    let body_has_run = match control {
                        ClassicalControlOp::If(op) => {
                            scan(op.then_body().operations())
                                || op.else_body().is_some_and(|body| scan(body.operations()))
                        }
                        ClassicalControlOp::While(op) => scan(op.body().operations()),
                        ClassicalControlOp::For(op) => scan(op.body().operations()),
                        ClassicalControlOp::Switch(op) => {
                            op.cases().iter().any(|case| scan(case.body().operations()))
                                || op.default().is_some_and(|body| scan(body.operations()))
                        }
                        ClassicalControlOp::Break | ClassicalControlOp::Continue => false,
                    };
                    if body_has_run {
                        return true;
                    }
                }
                _ => flush_source_boundary(operation, &mut pending),
            }
        }
        false
    }

    scan(circuit.operations())
}

fn instantiate_rule(
    rule: &crate::compile::knowledge::rule::Rule,
    operation: &LowerableOperation,
) -> Result<Vec<LowerableOperation>, CompilerError> {
    let params = operation
        .params
        .iter()
        .map(Parameter::from)
        .collect::<Vec<_>>();
    let view = ConcreteOperationView::new(&operation.instruction, &operation.qubits, &params);
    let bindings = rule_matches_operations(rule, &[view])
        .map_err(|error| CompilerError::InvariantViolation(error.to_string()))?
        .ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "selected device-lowering rule {} does not match {}",
                rule.name, operation.instruction
            ))
        })?;
    let replacements = instantiate_target(&rule.target, &bindings)
        .map_err(|error| CompilerError::InvariantViolation(error.to_string()))?;
    Ok(replacements
        .into_iter()
        .map(|replacement| LowerableOperation {
            instruction: replacement.instruction,
            qubits: replacement.qubits,
            params: SmallVec::from_vec(replacement.params.into_vec()),
            label: None,
        })
        .collect())
}

fn instantiate_direction_template(
    template: DirectionTemplate,
    source: &LowerableOperation,
) -> Vec<LowerableOperation> {
    debug_assert_eq!(source.qubits.len(), 2);
    let q0 = source.qubits[0];
    let q1 = source.qubits[1];
    let reversed = || LowerableOperation {
        instruction: source.instruction.clone(),
        qubits: smallvec![q1, q0],
        params: source.params.clone(),
        label: source.label.clone(),
    };
    match template {
        DirectionTemplate::Cx | DirectionTemplate::Rzx => {
            let h = |qubit| LowerableOperation {
                instruction: Instruction::Standard(StandardGate::H),
                qubits: smallvec![qubit],
                params: SmallVec::new(),
                label: None,
            };
            vec![h(q0), h(q1), reversed(), h(q0), h(q1)]
        }
        DirectionTemplate::Symmetric(_) => vec![reversed()],
    }
}

fn gphase_param(operation: &LowerableOperation) -> Result<Parameter, CompilerError> {
    operation
        .params
        .first()
        .map(Parameter::from)
        .ok_or_else(|| {
            CompilerError::InvariantViolation(
                "GPhase operation must contain one parameter".to_string(),
            )
        })
}

#[cfg(test)]
#[path = "device_lowering_test.rs"]
mod device_lowering_test;
