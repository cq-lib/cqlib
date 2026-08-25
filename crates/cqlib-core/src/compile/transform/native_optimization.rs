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

//! Exact physical optimization after device instruction lowering.
//!
//! [`NativeOptimizer`] closes a bounded loop over exact-physical two-qubit
//! resynthesis, local phase/frame optimization, device re-legalization, and
//! canonicalization. Every transform recursively visits structured control-flow
//! bodies, while the loop itself advances all scopes synchronously.
//!
//! Candidate circuits are accepted only when their `DevicePhysicalCost`
//! (represented internally by the exact sequence evaluator) is non-worse in
//! every corresponding control-flow scope and strictly better in at least one.
//! This conservative Pareto rule avoids assigning an arbitrary execution count
//! to conditional or loop bodies. The optimizer restores the best whole-circuit
//! point seen; it does not splice independently optimal bodies from different
//! rounds.
//!
//! One immutable exact-physical synthesis context is shared by resynthesis,
//! local optimization, and scope costing for the lifetime of a single run. The
//! catalog is rebuilt transactionally only when scope costing finds an
//! unprepared root; a prepared-but-unsupported root remains a real failure.

use crate::circuit::{
    Circuit, ClassicalControlOp, Directive, Instruction, Operation, Parameter, ParameterValue,
    Qubit, StandardGate, ValueClassicalControlOp, ValueControlBody, ValueInstruction,
    ValueOperation, ValueSwitchCase,
};
use crate::compile::CompilerError;
use crate::compile::device_planning::DevicePlanningSession;
use crate::compile::sabre::MetricAvailability;
use crate::compile::transform::decompose::unitary::{
    DeviceContextCostFailure, DeviceSynthesisPlacement, DeviceTwoQubitSynthesisContext,
    OneQubitUnitaryDecomposition, synthesize_numeric_1q_unitary,
};
use crate::compile::transform::rebuild::{CircuitRebuildContext, ClassicalRemap};
use crate::compile::transform::resynthesis::{
    NativeResynthesisPolicy, NativeResynthesisSession, NativeWorksetStats,
    TwoQubitBlockResynthesisConfig, resynthesize_two_qubit_blocks_incremental,
};
use crate::compile::transform::target_basis::{TargetBasisCost, TargetBasisCostModel};
use crate::compile::transform::{
    Canonicalizer, CircuitAnalysis, DeviceLowerer, TransformOutcome, Transformer,
};
use crate::device::Device;
use ndarray::Array2;
use num_complex::Complex64;
use smallvec::{SmallVec, smallvec};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};
use std::sync::Arc;

const PHASE_EPS: f64 = 1e-12;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NativeOptimizationSummary {
    pub(crate) native_two_qubit_ops: u64,
    pub(crate) native_two_qubit_depth: u64,
    pub(crate) total_native_depth: u64,
    pub(crate) native_total_ops: u64,
    pub(crate) predicted_log_error: Option<f64>,
    pub(crate) unavailable_error_count: u64,
    pub(crate) imputed_error_count: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct NativeOptimizationResult {
    pub(crate) circuit: Circuit,
    pub(crate) changed: bool,
    pub(crate) rounds: u8,
    pub(crate) restored_best: bool,
    pub(crate) before: NativeOptimizationSummary,
    pub(crate) after: NativeOptimizationSummary,
}

/// Bounded native optimization loop with minimum-point restoration.
pub(crate) struct NativeOptimizer<'a> {
    device: &'a Device,
    planning_session: Arc<DevicePlanningSession>,
    resynthesis: TwoQubitBlockResynthesisConfig,
    max_rounds: u8,
    max_stale_rounds: u8,
}

impl<'a> NativeOptimizer<'a> {
    pub(crate) fn with_session(
        device: &'a Device,
        resynthesis: TwoQubitBlockResynthesisConfig,
        max_rounds: u8,
        max_stale_rounds: u8,
        planning_session: Arc<DevicePlanningSession>,
    ) -> Self {
        Self {
            device,
            planning_session,
            resynthesis,
            max_rounds,
            max_stale_rounds,
        }
    }

    pub(crate) fn run(&self, circuit: &Circuit) -> Result<NativeOptimizationResult, CompilerError> {
        self.run_with_policy(circuit, NativeResynthesisPolicy::Incremental)
            .map(|(result, _)| result)
    }

    pub(crate) fn run_with_policy(
        &self,
        circuit: &Circuit,
        policy: NativeResynthesisPolicy,
    ) -> Result<(NativeOptimizationResult, NativeWorksetStats), CompilerError> {
        let initial = Canonicalizer::production()
            .transform(circuit, None)?
            .into_circuit(circuit);
        self.device.validate_circuit(&initial)?;
        // Native rounds can carry very large circuits. Share immutable round
        // states so `current`, `best`, and the exact cycle detector do not each
        // deep-clone every operation and nested control-flow body.
        let initial = Arc::new(initial);
        let mut current = initial.clone();
        let mut best = initial;
        // This exact-physical context is immutable and run-scoped. All consumers
        // share its Arc-backed catalog until a candidate exposes missing coverage.
        let mut context = DeviceTwoQubitSynthesisContext::build_with_session(
            self.device,
            &best,
            DeviceSynthesisPlacement::ExactPhysical,
            Arc::clone(&self.planning_session),
        )?;
        let mut best_costs = scope_costs_with_context(&best, &context).map_err(scope_cost_error)?;
        let before = summarize_scope_costs(&best_costs);
        let mut rounds = 0;
        let mut stale = 0;
        let mut restored_best = false;
        let mut resynthesis_session = NativeResynthesisSession::new(policy);
        let mut seen_states = vec![current.clone()];

        'optimization: while rounds < self.max_rounds && stale < self.max_stale_rounds {
            rounds += 1;
            let resynthesis_outcome = resynthesize_two_qubit_blocks_incremental(
                &current,
                self.resynthesis.clone(),
                context.clone(),
                &mut resynthesis_session,
            )?;
            let resynthesis_changed = resynthesis_outcome.changed();
            let resynthesized = match &resynthesis_outcome {
                TransformOutcome::Unchanged => current.as_ref(),
                TransformOutcome::Changed(circuit) => circuit,
            };
            let local_outcome =
                OptimizeNativeLocalGates::new(context.clone()).transform(resynthesized, None)?;
            let local_changed = local_outcome.changed();
            // A fully stable round makes the remaining passes deterministic
            // no-ops: lowering and canonicalization reproduce `current` (the
            // entry point already validated it as exact-native), so this is
            // the `candidate == current` break below without paying for the
            // full-circuit lowering, canonicalization, validation, and cost
            // evaluation in between.
            if !resynthesis_changed && !local_changed {
                break;
            }
            let legalized = match local_outcome {
                TransformOutcome::Changed(locally_optimized) => {
                    match DeviceLowerer::with_session(self.device, &self.planning_session)
                        .transform(&locally_optimized, None)
                    {
                        Ok(TransformOutcome::Unchanged) => locally_optimized,
                        Ok(TransformOutcome::Changed(legalized)) => legalized,
                        // Frame propagation is speculative: materializing a combined
                        // phase as RZ may be impossible on devices whose discrete
                        // phase gates cannot synthesize arbitrary RZ. Discard only
                        // that local candidate and retain this round's 2Q result.
                        Err(CompilerError::DeviceLoweringFailed(_)) => {
                            let fallback = match &resynthesis_outcome {
                                TransformOutcome::Unchanged => current.as_ref(),
                                TransformOutcome::Changed(circuit) => circuit,
                            };
                            match DeviceLowerer::with_session(self.device, &self.planning_session)
                                .transform(fallback, None)?
                            {
                                TransformOutcome::Changed(legalized) => legalized,
                                TransformOutcome::Unchanged => match resynthesis_outcome {
                                    TransformOutcome::Changed(resynthesized) => resynthesized,
                                    TransformOutcome::Unchanged => break 'optimization,
                                },
                            }
                        }
                        Err(error) => return Err(error),
                    }
                }
                TransformOutcome::Unchanged => {
                    let resynthesized = match resynthesis_outcome {
                        TransformOutcome::Changed(circuit) => circuit,
                        TransformOutcome::Unchanged => unreachable!(
                            "a stable native optimization round exits before legalization"
                        ),
                    };
                    match DeviceLowerer::with_session(self.device, &self.planning_session)
                        .transform(&resynthesized, None)?
                    {
                        TransformOutcome::Unchanged => resynthesized,
                        TransformOutcome::Changed(legalized) => legalized,
                    }
                }
            };
            let candidate = match Canonicalizer::production().transform(&legalized, None)? {
                TransformOutcome::Unchanged => legalized,
                TransformOutcome::Changed(candidate) => candidate,
            };
            // Debug builds validate every round to keep the safety net tight
            // while developing; release builds validate only accepted
            // candidates. The terminal workflow validation remains the final
            // safety boundary either way.
            let validate_every_round = cfg!(debug_assertions);
            if validate_every_round {
                self.device.validate_circuit(&candidate)?;
            }

            if candidate == *current {
                break;
            }
            // Every stage in a native round is deterministic for a fixed
            // context and configuration. Re-entering any exact prior circuit
            // therefore proves a cycle; further stale rounds can only replay
            // states already compared against `best`.
            if let Some(seen) = seen_states.iter().find(|seen| seen.as_ref() == &candidate) {
                current = seen.clone();
                resynthesis_session.record_cycle_early_exit();
                break;
            }
            let candidate_costs = self.candidate_costs_with_reuse(&candidate, &mut context)?;
            let candidate = Arc::new(candidate);
            if scope_costs_dominate(&candidate_costs, &best_costs) {
                if !validate_every_round {
                    self.device.validate_circuit(&candidate)?;
                }
                best = candidate.clone();
                best_costs = candidate_costs;
                stale = 0;
            } else {
                stale = stale.saturating_add(1);
            }
            seen_states.push(candidate.clone());
            current = candidate;
        }

        if current.as_ref() != best.as_ref() {
            restored_best = true;
        }
        let after = summarize_scope_costs(&best_costs);
        // Drop the other shared handles before recovering the owned result.
        // `try_unwrap` is therefore the normal path and preserves the previous
        // API without a final full-circuit clone.
        drop(current);
        drop(seen_states);
        let best = Arc::try_unwrap(best).unwrap_or_else(|shared| shared.as_ref().clone());
        let result = NativeOptimizationResult {
            changed: best != *circuit,
            circuit: best,
            rounds,
            restored_best,
            before,
            after,
        };
        Ok((result, resynthesis_session.stats()))
    }

    /// Costs a candidate with the run-scoped context, rebuilding transactionally
    /// at most once when (and only when) the catalog did not prepare a required root.
    fn candidate_costs_with_reuse(
        &self,
        candidate: &Circuit,
        context: &mut DeviceTwoQubitSynthesisContext,
    ) -> Result<Vec<crate::compile::transform::decompose::unitary::DevicePhysicalCost>, CompilerError>
    {
        match scope_costs_with_context(candidate, context) {
            Ok(costs) => Ok(costs),
            Err(ScopeCostError::Context(DeviceContextCostFailure::Unprepared(_))) => {
                let rebuilt = DeviceTwoQubitSynthesisContext::build_with_session(
                    self.device,
                    candidate,
                    DeviceSynthesisPlacement::ExactPhysical,
                    Arc::clone(&self.planning_session),
                )?;
                let costs = match scope_costs_with_context(candidate, &rebuilt) {
                    Ok(costs) => costs,
                    Err(ScopeCostError::Context(DeviceContextCostFailure::Unprepared(state))) => {
                        return Err(CompilerError::InvariantViolation(format!(
                            "rebuilt native optimizer context was not prepared for {state:?}"
                        )));
                    }
                    Err(error) => return Err(scope_cost_error(error)),
                };
                *context = rebuilt;
                Ok(costs)
            }
            Err(error) => Err(scope_cost_error(error)),
        }
    }
}

#[derive(Debug)]
enum ScopeCostError {
    Compiler(CompilerError),
    Context(DeviceContextCostFailure),
}

impl From<CompilerError> for ScopeCostError {
    fn from(error: CompilerError) -> Self {
        Self::Compiler(error)
    }
}

fn scope_cost_error(error: ScopeCostError) -> CompilerError {
    match error {
        ScopeCostError::Compiler(error) => error,
        ScopeCostError::Context(_) => CompilerError::InvariantViolation(
            "native optimizer could not cost a legalized control-flow scope".to_string(),
        ),
    }
}

fn scope_costs_with_context(
    circuit: &Circuit,
    context: &DeviceTwoQubitSynthesisContext,
) -> Result<Vec<crate::compile::transform::decompose::unitary::DevicePhysicalCost>, ScopeCostError>
{
    let mut costs = Vec::new();
    collect_scope_costs(circuit.operations(), context, &mut costs)?;
    Ok(costs)
}

fn collect_scope_costs(
    operations: &[Operation],
    context: &DeviceTwoQubitSynthesisContext,
    output: &mut Vec<crate::compile::transform::decompose::unitary::DevicePhysicalCost>,
) -> Result<(), ScopeCostError> {
    let mut accumulator = context
        .exact_sequence_cost_accumulator()
        .map_err(ScopeCostError::Context)?;
    for operation in operations {
        match &operation.instruction {
            Instruction::Standard(_) | Instruction::McGate(_) => accumulator
                .add_gate(&operation.instruction, &operation.qubits)
                .map_err(ScopeCostError::Context)?,
            Instruction::ClassicalControl(control) => match control {
                ClassicalControlOp::If(op) => {
                    collect_scope_costs(op.then_body().operations(), context, output)?;
                    if let Some(body) = op.else_body() {
                        collect_scope_costs(body.operations(), context, output)?;
                    }
                }
                ClassicalControlOp::While(op) => {
                    collect_scope_costs(op.body().operations(), context, output)?;
                }
                ClassicalControlOp::For(op) => {
                    collect_scope_costs(op.body().operations(), context, output)?;
                }
                ClassicalControlOp::Switch(op) => {
                    for case in op.cases() {
                        collect_scope_costs(case.body().operations(), context, output)?;
                    }
                    if let Some(body) = op.default() {
                        collect_scope_costs(body.operations(), context, output)?;
                    }
                }
                ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
            },
            Instruction::UnitaryGate(_)
            | Instruction::CircuitGate(_)
            | Instruction::ClassicalData(_)
            | Instruction::Directive(_)
            | Instruction::Delay => {}
        }
    }
    output.push(accumulator.finish());
    Ok(())
}

fn scope_costs_dominate(
    candidate: &[crate::compile::transform::decompose::unitary::DevicePhysicalCost],
    current: &[crate::compile::transform::decompose::unitary::DevicePhysicalCost],
) -> bool {
    if candidate.len() != current.len() {
        return false;
    }
    let mut improved = false;
    for (candidate, current) in candidate.iter().zip(current) {
        match candidate.compare(*current) {
            std::cmp::Ordering::Less => improved = true,
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Greater => return false,
        }
    }
    improved
}

fn summarize_scope_costs(
    costs: &[crate::compile::transform::decompose::unitary::DevicePhysicalCost],
) -> NativeOptimizationSummary {
    let mut predicted_log_error = Some(0.0);
    let mut unavailable_error_count = 0;
    let mut imputed_error_count = 0;
    for cost in costs {
        match cost.error {
            MetricAvailability::Available(error) => {
                if let Some(total) = &mut predicted_log_error {
                    *total += error.log_error;
                }
                unavailable_error_count += u64::from(error.unavailable_count);
                imputed_error_count += u64::from(error.imputed_count);
            }
            MetricAvailability::Disabled | MetricAvailability::Inconsistent => {
                predicted_log_error = None;
            }
        }
    }
    NativeOptimizationSummary {
        native_two_qubit_ops: costs
            .iter()
            .map(|cost| u64::from(cost.native_two_qubit_ops))
            .sum(),
        native_two_qubit_depth: costs
            .iter()
            .map(|cost| u64::from(cost.native_two_qubit_depth))
            .sum(),
        total_native_depth: costs
            .iter()
            .map(|cost| u64::from(cost.total_native_depth))
            .sum(),
        native_total_ops: costs
            .iter()
            .map(|cost| u64::from(cost.native_total_ops))
            .sum(),
        predicted_log_error,
        unavailable_error_count,
        imputed_error_count,
    }
}

/// Performs target-costed one-qubit fusion and exact frame propagation.
#[derive(Debug, Clone)]
pub(crate) struct OptimizeNativeLocalGates {
    device_context: DeviceTwoQubitSynthesisContext,
}

impl OptimizeNativeLocalGates {
    pub(crate) fn new(device_context: DeviceTwoQubitSynthesisContext) -> Self {
        Self { device_context }
    }
}

impl Transformer for OptimizeNativeLocalGates {
    fn name(&self) -> &'static str {
        "optimize.native_local_gates"
    }

    fn transform(
        &self,
        circuit: &Circuit,
        _analysis: Option<&CircuitAnalysis>,
    ) -> Result<TransformOutcome, CompilerError> {
        let policy = LocalOptimizationPolicy::Device(self.device_context.clone());
        LocalOneQPass::run(circuit, &policy)
    }
}

/// Cost policy used by the shared one-qubit/frame optimization engine.
#[derive(Debug, Clone)]
pub(crate) enum LocalOptimizationPolicy {
    Logical,
    Basis(Arc<TargetBasisCostModel>),
    Device(DeviceTwoQubitSynthesisContext),
}

/// Runs the shared one-qubit/frame optimizer with an explicit cost policy.
pub(crate) fn optimize_one_qubit_runs_with_policy(
    circuit: &Circuit,
    policy: &LocalOptimizationPolicy,
) -> Result<TransformOutcome, CompilerError> {
    LocalOneQPass::run(circuit, policy)
}

struct LocalOneQPass<'source, 'policy> {
    source: &'source Circuit,
    policy: &'policy LocalOptimizationPolicy,
    rebuild: CircuitRebuildContext,
}

struct SequenceRewrite {
    operations: Vec<ValueOperation>,
    phase_delta: f64,
    changed: bool,
}

impl<'source, 'policy> LocalOneQPass<'source, 'policy> {
    fn run(
        source: &'source Circuit,
        policy: &'policy LocalOptimizationPolicy,
    ) -> Result<TransformOutcome, CompilerError> {
        let rebuild = CircuitRebuildContext::new(source);
        let root_classical = rebuild.root_classical().clone();
        let mut pass = Self {
            source,
            policy,
            rebuild,
        };
        let rewrite = pass.process_sequence(source.operations(), &root_classical)?;
        if !rewrite.changed {
            return Ok(TransformOutcome::Unchanged);
        }
        let mut global_phase = source.global_phase();
        if rewrite.phase_delta.abs() > PHASE_EPS {
            global_phase = global_phase + Parameter::from(rewrite.phase_delta);
        }
        let circuit = pass
            .rebuild
            .finish(source.qubits(), rewrite.operations, global_phase)?;
        Ok(TransformOutcome::Changed(circuit))
    }

    fn process_sequence(
        &mut self,
        operations: &[Operation],
        classical_remap: &ClassicalRemap,
    ) -> Result<SequenceRewrite, CompilerError> {
        let mut values = Vec::with_capacity(operations.len());
        let mut nested_changed = false;
        for operation in operations {
            if let Instruction::ClassicalControl(control) = &operation.instruction {
                let (instruction, changed) = self.rebuild_control_flow(control, classical_remap)?;
                values.push(ValueOperation {
                    qubits: instruction.used_qubits().into_iter().collect(),
                    instruction: ValueInstruction::ClassicalControl(instruction),
                    params: CircuitRebuildContext::resolve_source_params(
                        self.source,
                        &operation.params,
                    )?,
                    label: operation.label.clone(),
                });
                nested_changed |= changed;
            } else {
                values.push(self.rebuild.remap_preserved_operation(
                    self.source,
                    operation,
                    classical_remap,
                )?);
            }
        }

        let optimized = match self.policy {
            LocalOptimizationPolicy::Device(_) => {
                // Preserve the existing native behavior: frame movement is
                // speculative within a native round, while the outer minimum
                // point controller decides whether the whole round survives.
                let framed = propagate_frames(values)?;
                let fused = fuse_one_qubit_runs(framed.operations, self.policy)?;
                ValueRewrite {
                    operations: fused.operations,
                    phase_delta: framed.phase_delta + fused.phase_delta,
                    changed: framed.changed || fused.changed,
                }
            }
            LocalOptimizationPolicy::Logical | LocalOptimizationPolicy::Basis(_) => {
                optimize_transactional(values, self.policy)?
            }
        };
        Ok(SequenceRewrite {
            operations: optimized.operations,
            phase_delta: optimized.phase_delta,
            changed: nested_changed || optimized.changed,
        })
    }

    fn rebuild_body(
        &mut self,
        operations: &[Operation],
        classical_remap: &ClassicalRemap,
    ) -> Result<(ValueControlBody, bool), CompilerError> {
        let mut rewrite = self.process_sequence(operations, classical_remap)?;
        if rewrite.phase_delta.abs() > PHASE_EPS {
            rewrite.operations.insert(
                0,
                ValueOperation {
                    instruction: ValueInstruction::from_instruction(Instruction::Standard(
                        StandardGate::GPhase,
                    )),
                    qubits: SmallVec::new(),
                    params: smallvec![ParameterValue::Fixed(rewrite.phase_delta)],
                    label: None,
                },
            );
            rewrite.changed = true;
        }
        Ok((ValueControlBody::new(rewrite.operations), rewrite.changed))
    }

    fn rebuild_control_flow(
        &mut self,
        control: &ClassicalControlOp,
        classical_remap: &ClassicalRemap,
    ) -> Result<(ValueClassicalControlOp, bool), CompilerError> {
        Ok(match control {
            ClassicalControlOp::If(op) => {
                let (then_body, then_changed) =
                    self.rebuild_body(op.then_body().operations(), classical_remap)?;
                let else_rewrite = op
                    .else_body()
                    .map(|body| self.rebuild_body(body.operations(), classical_remap))
                    .transpose()?;
                let else_changed = else_rewrite.as_ref().is_some_and(|(_, changed)| *changed);
                (
                    ValueClassicalControlOp::If {
                        condition: classical_remap.remap_expr(op.condition())?,
                        then_body,
                        else_body: else_rewrite.map(|(body, _)| body),
                    },
                    then_changed || else_changed,
                )
            }
            ClassicalControlOp::While(op) => {
                let (body, changed) = self.rebuild_body(op.body().operations(), classical_remap)?;
                (
                    ValueClassicalControlOp::While {
                        condition: classical_remap.remap_expr(op.condition())?,
                        body,
                    },
                    changed,
                )
            }
            ClassicalControlOp::For(op) => {
                let (body, changed) = self.rebuild_body(op.body().operations(), classical_remap)?;
                (
                    ValueClassicalControlOp::For {
                        var: classical_remap.remap_var(op.var())?,
                        start: classical_remap.remap_expr(op.start())?,
                        stop: classical_remap.remap_expr(op.stop())?,
                        step: classical_remap.remap_expr(op.step())?,
                        body,
                    },
                    changed,
                )
            }
            ClassicalControlOp::Switch(op) => {
                let mut changed = false;
                let cases = op
                    .cases()
                    .iter()
                    .map(|case| {
                        let (body, body_changed) =
                            self.rebuild_body(case.body().operations(), classical_remap)?;
                        changed |= body_changed;
                        Ok(ValueSwitchCase::new(case.value(), body))
                    })
                    .collect::<Result<Vec<_>, CompilerError>>()?;
                let default_rewrite = op
                    .default()
                    .map(|body| self.rebuild_body(body.operations(), classical_remap))
                    .transpose()?;
                changed |= default_rewrite
                    .as_ref()
                    .is_some_and(|(_, body_changed)| *body_changed);
                (
                    ValueClassicalControlOp::Switch {
                        target: classical_remap.remap_expr(op.target())?,
                        cases,
                        default: default_rewrite.map(|(body, _)| body),
                    },
                    changed,
                )
            }
            ClassicalControlOp::Break => (ValueClassicalControlOp::Break, false),
            ClassicalControlOp::Continue => (ValueClassicalControlOp::Continue, false),
        })
    }
}

struct ValueRewrite {
    operations: Vec<ValueOperation>,
    phase_delta: f64,
    changed: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct LogicalOneQCost {
    one_qubit_ops: usize,
    affected_region_depth: usize,
    total_gate_ops: usize,
}

impl LocalOptimizationPolicy {
    fn strictly_better(
        &self,
        candidate: &[ValueOperation],
        source: &[ValueOperation],
    ) -> Result<bool, CompilerError> {
        Ok(match self {
            Self::Logical => logical_one_qubit_cost(candidate) < logical_one_qubit_cost(source),
            Self::Basis(model) => {
                let Some(before) = basis_cost(model, source)? else {
                    return Ok(false);
                };
                let Some(after) = basis_cost(model, candidate)? else {
                    return Ok(false);
                };
                after.two_qubit_ops <= before.two_qubit_ops
                    && compare_basis_cost(after, before).is_lt()
            }
            Self::Device(context) => {
                let Some(before) = exact_local_sequence_cost(context, source, "source")? else {
                    return Ok(false);
                };
                let Some(after) = exact_local_sequence_cost(context, candidate, "candidate")?
                else {
                    return Ok(false);
                };
                after.strictly_better_than(before)
            }
        })
    }
}

fn exact_local_sequence_cost(
    context: &DeviceTwoQubitSynthesisContext,
    operations: &[ValueOperation],
    role: &str,
) -> Result<Option<crate::compile::transform::decompose::unitary::DevicePhysicalCost>, CompilerError>
{
    match context.exact_sequence_cost_diagnostic(operations) {
        Ok(cost) => Ok(Some(cost)),
        Err(DeviceContextCostFailure::Unsupported(failure)) => {
            let _ = failure;
            Ok(None)
        }
        Err(DeviceContextCostFailure::Unprepared(state)) => {
            Err(CompilerError::InvariantViolation(format!(
                "native local optimization context was not prepared for {role} state {state:?}"
            )))
        }
        Err(DeviceContextCostFailure::WrongPlacement) => Err(CompilerError::InvariantViolation(
            "native local optimization requires an exact-physical context".to_string(),
        )),
        Err(DeviceContextCostFailure::InvalidOperation(reason)) => {
            Err(CompilerError::InvariantViolation(format!(
                "invalid native local optimization {role}: {reason}"
            )))
        }
    }
}

fn optimize_transactional(
    operations: Vec<ValueOperation>,
    policy: &LocalOptimizationPolicy,
) -> Result<ValueRewrite, CompilerError> {
    let framed = propagate_frames(operations.clone())?;
    let fused_after_frames = fuse_one_qubit_runs(framed.operations, policy)?;
    let combined_phase = framed.phase_delta + fused_after_frames.phase_delta;
    let combined_changed = framed.changed || fused_after_frames.changed;
    if combined_changed && policy.strictly_better(&fused_after_frames.operations, &operations)? {
        return Ok(ValueRewrite {
            operations: fused_after_frames.operations,
            phase_delta: combined_phase,
            changed: true,
        });
    }

    // A neutral or harmful frame movement must not hide an independently
    // useful one-qubit fusion on the original sequence.
    fuse_one_qubit_runs(operations, policy)
}

fn logical_one_qubit_cost(operations: &[ValueOperation]) -> LogicalOneQCost {
    let mut depths = HashMap::<Qubit, usize>::new();
    let mut cost = LogicalOneQCost::default();
    for operation in operations {
        let ValueInstruction::Instruction(Instruction::Standard(gate)) = operation.instruction
        else {
            continue;
        };
        if gate == StandardGate::GPhase {
            continue;
        }
        cost.total_gate_ops += 1;
        if gate.num_qubits() == 1 && operation.qubits.len() == 1 {
            cost.one_qubit_ops += 1;
        }
        if operation.qubits.is_empty() {
            continue;
        }
        let next = operation
            .qubits
            .iter()
            .filter_map(|qubit| depths.get(qubit))
            .max()
            .copied()
            .unwrap_or(0)
            + 1;
        for &qubit in &operation.qubits {
            depths.insert(qubit, next);
        }
        cost.affected_region_depth = cost.affected_region_depth.max(next);
    }
    cost
}

fn basis_cost(
    model: &TargetBasisCostModel,
    operations: &[ValueOperation],
) -> Result<Option<TargetBasisCost>, CompilerError> {
    let operations = operations
        .iter()
        .filter(|operation| {
            matches!(
                operation.instruction,
                ValueInstruction::Instruction(Instruction::Standard(_))
            ) && operation.params.iter().all(
                |parameter| matches!(parameter, ParameterValue::Fixed(value) if value.is_finite()),
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    if operations.is_empty() {
        return Ok(Some(TargetBasisCost::default()));
    }
    let mut qubits = operations
        .iter()
        .flat_map(|operation| operation.qubits.iter().copied())
        .collect::<Vec<_>>();
    qubits.sort_by_key(|qubit| qubit.index());
    qubits.dedup();
    match model.cost_of_fixed_operations(qubits, operations) {
        Ok(cost) => Ok(Some(cost)),
        Err(CompilerError::InvalidInput(_)) | Err(CompilerError::TransformFailed { .. }) => {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

fn compare_basis_cost(left: TargetBasisCost, right: TargetBasisCost) -> std::cmp::Ordering {
    left.two_qubit_ops
        .cmp(&right.two_qubit_ops)
        .then_with(|| left.depth.cmp(&right.depth))
        .then_with(|| left.total_ops.cmp(&right.total_ops))
        .then_with(|| left.parameterized_ops.cmp(&right.parameterized_ops))
}

/// Replaces fixed numeric 1Q runs only when the synthesized unitary has a
/// strictly better exact-physical device cost than the original run.
fn fuse_one_qubit_runs(
    operations: Vec<ValueOperation>,
    policy: &LocalOptimizationPolicy,
) -> Result<ValueRewrite, CompilerError> {
    let runs = collect_one_qubit_runs(&operations);
    let mut replacements = HashMap::<usize, (Vec<usize>, Vec<ValueOperation>)>::new();
    let mut phase_delta = 0.0;

    for run in runs.into_iter().filter(|run| run.len() >= 2) {
        let source_ops = run
            .iter()
            .map(|order| operations[*order].clone())
            .collect::<Vec<_>>();
        let Some(matrix) = one_qubit_run_matrix(&source_ops) else {
            continue;
        };
        let Ok(decomposition) = synthesize_numeric_1q_unitary(&matrix) else {
            continue;
        };
        let qubit = source_ops[0].qubits[0];
        let mut candidate = Vec::new();
        if decomposition.theta.abs() > PHASE_EPS
            || decomposition.phi.abs() > PHASE_EPS
            || decomposition.lambda.abs() > PHASE_EPS
        {
            candidate.push(u_operation(qubit, decomposition));
        }
        if !policy.strictly_better(&candidate, &source_ops)? {
            continue;
        }
        phase_delta += decomposition.global_phase;
        replacements.insert(run[0], (run, candidate));
    }

    if replacements.is_empty() {
        return Ok(ValueRewrite {
            operations,
            phase_delta: 0.0,
            changed: false,
        });
    }

    let mut skipped = HashSet::new();
    for (first, (orders, _)) in &replacements {
        skipped.extend(orders.iter().copied().filter(|order| order != first));
    }
    let mut output = Vec::with_capacity(operations.len());
    for (order, operation) in operations.into_iter().enumerate() {
        if let Some((_, replacement)) = replacements.remove(&order) {
            output.extend(replacement);
        } else if !skipped.contains(&order) {
            output.push(operation);
        }
    }
    Ok(ValueRewrite {
        operations: output,
        phase_delta,
        changed: true,
    })
}

fn collect_one_qubit_runs(operations: &[ValueOperation]) -> Vec<Vec<usize>> {
    let mut active = BTreeMap::<Qubit, Vec<usize>>::new();
    let mut runs = Vec::new();
    for (order, operation) in operations.iter().enumerate() {
        if is_fixed_numeric_one_qubit_gate(operation) {
            active.entry(operation.qubits[0]).or_default().push(order);
            continue;
        }

        let global_boundary = operation.qubits.is_empty()
            || matches!(operation.instruction, ValueInstruction::ClassicalControl(_))
            || matches!(
                operation.instruction,
                ValueInstruction::Instruction(Instruction::Directive(Directive::Barrier))
            );
        if global_boundary {
            runs.extend(std::mem::take(&mut active).into_values());
        } else {
            for qubit in &operation.qubits {
                if let Some(run) = active.remove(qubit) {
                    runs.push(run);
                }
            }
        }
    }
    runs.extend(active.into_values());
    runs
}

fn is_fixed_numeric_one_qubit_gate(operation: &ValueOperation) -> bool {
    matches!(
        &operation.instruction,
        ValueInstruction::Instruction(Instruction::Standard(gate))
            if gate.num_qubits() == 1
                && operation.qubits.len() == 1
                && operation.label.is_none()
                && operation.params.iter().all(|param| {
                    matches!(param, ParameterValue::Fixed(value) if value.is_finite())
                })
                && gate.matrix(&fixed_params(operation).unwrap_or_default()).is_ok()
    )
}

fn fixed_params(operation: &ValueOperation) -> Option<Vec<f64>> {
    operation
        .params
        .iter()
        .map(|param| match param {
            ParameterValue::Fixed(value) if value.is_finite() => Some(*value),
            ParameterValue::Fixed(_) | ParameterValue::Param(_) => None,
        })
        .collect()
}

fn one_qubit_run_matrix(operations: &[ValueOperation]) -> Option<Array2<Complex64>> {
    let mut matrix = Array2::<Complex64>::eye(2);
    for operation in operations {
        let ValueInstruction::Instruction(Instruction::Standard(gate)) = &operation.instruction
        else {
            return None;
        };
        let gate_matrix = gate.matrix(&fixed_params(operation)?).ok()?;
        matrix = gate_matrix.dot(&matrix);
    }
    Some(matrix)
}

fn u_operation(qubit: Qubit, decomposition: OneQubitUnitaryDecomposition) -> ValueOperation {
    ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::U)),
        qubits: smallvec![qubit],
        params: smallvec![
            ParameterValue::Fixed(decomposition.theta),
            ParameterValue::Fixed(decomposition.phi),
            ParameterValue::Fixed(decomposition.lambda),
        ],
        label: None,
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct QubitFrame {
    z_angle: f64,
    pauli_x: bool,
    pauli_z: bool,
}

impl QubitFrame {
    fn is_empty(self) -> bool {
        self.z_angle.abs() <= PHASE_EPS && !self.pauli_x && !self.pauli_z
    }

    fn multiply_pauli(&mut self, x: bool, z: bool) -> u8 {
        // The new gate is later in circuit order, so the accumulated matrix is
        // P_new * P_pending. In canonical X^x Z^z order this contributes -1 when
        // the new Z anticommutes with a pending X.
        let phase = if z && self.pauli_x { 2 } else { 0 };
        self.pauli_x ^= x;
        self.pauli_z ^= z;
        phase
    }
}

/// Ejects pending Z/Pauli frames forward through a closed table of proven gate
/// identities and materializes a frame at the first unsupported boundary.
///
/// A pending frame occurs earlier in circuit time than the operation currently
/// being visited. Moving it forward therefore conjugates it by that operation.
/// Frames never cross structured control flow, labels, barriers, or resets.
fn propagate_frames(operations: Vec<ValueOperation>) -> Result<ValueRewrite, CompilerError> {
    let mut frames = BTreeMap::<Qubit, QubitFrame>::new();
    let mut output = Vec::with_capacity(operations.len());
    let mut phase_delta = 0.0;
    let mut changed = false;

    for mut operation in operations {
        if operation.label.is_none()
            && let Some((qubit, z_angle, phase)) = z_carrier(&operation)
        {
            flush_pauli(qubit, &mut frames, &mut output, &mut phase_delta);
            frames.entry(qubit).or_default().z_angle += z_angle;
            phase_delta += phase;
            changed = true;
            continue;
        }
        if operation.label.is_none()
            && let Some((qubit, x, z, phase)) = pauli_carrier(&operation)
        {
            flush_z(qubit, &mut frames, &mut output);
            let frame = frames.entry(qubit).or_default();
            let extra_phase = frame.multiply_pauli(x, z);
            phase_delta += phase + f64::from(extra_phase) * FRAC_PI_2;
            changed = true;
            continue;
        }

        let instruction = match &operation.instruction {
            ValueInstruction::Instruction(instruction) => Some(instruction.clone()),
            ValueInstruction::ClassicalControl(_) => None,
        };

        if operation.label.is_none()
            && matches!(
                instruction.as_ref(),
                Some(Instruction::Standard(StandardGate::SWAP))
            )
            && operation.qubits.len() == 2
        {
            let left = frames.remove(&operation.qubits[0]).unwrap_or_default();
            let right = frames.remove(&operation.qubits[1]).unwrap_or_default();
            frames.insert(operation.qubits[0], right);
            frames.insert(operation.qubits[1], left);
            output.push(operation);
            changed |= !left.is_empty() || !right.is_empty();
            continue;
        }

        if operation.label.is_none()
            && let Some(gate @ (StandardGate::CX | StandardGate::CZ)) =
                instruction.as_ref().and_then(|i| {
                    let Instruction::Standard(gate @ (StandardGate::CX | StandardGate::CZ)) = i
                    else {
                        return None;
                    };
                    Some(*gate)
                })
            && operation.qubits.len() == 2
        {
            let pair = [operation.qubits[0], operation.qubits[1]];
            if gate == StandardGate::CX {
                flush_z(pair[1], &mut frames, &mut output);
            }
            if pair.iter().any(|qubit| {
                frames
                    .get(qubit)
                    .is_some_and(|frame| frame.pauli_x || frame.pauli_z)
            }) {
                for qubit in pair {
                    flush_z(qubit, &mut frames, &mut output);
                }
                phase_delta += propagate_clifford_paulis(gate, pair, &mut frames);
                changed = true;
            }
            output.push(operation);
            continue;
        }

        if operation.label.is_none()
            && let Some(Instruction::Standard(gate)) = instruction.as_ref()
            && is_z_diagonal(*gate)
        {
            let blocked = operation
                .qubits
                .iter()
                .any(|qubit| frames.get(qubit).is_some_and(|frame| frame.pauli_x));
            if !blocked {
                output.push(operation);
                continue;
            }
        }

        if operation.label.is_none() && absorb_z_into_xy_axis(&mut operation, &frames) {
            output.push(operation);
            changed = true;
            continue;
        }

        if instruction
            .as_ref()
            .is_some_and(|instruction| instruction.has_measurement())
        {
            for qubit in &operation.qubits {
                if frames.get(qubit).is_some_and(|frame| frame.pauli_x) {
                    flush_frame(*qubit, &mut frames, &mut output, &mut phase_delta);
                } else {
                    changed |= frames.remove(qubit).is_some_and(|frame| !frame.is_empty());
                }
            }
            output.push(operation);
            continue;
        }

        let global_boundary = operation.qubits.is_empty()
            || operation.label.is_some()
            || matches!(operation.instruction, ValueInstruction::ClassicalControl(_))
            || matches!(
                instruction.as_ref(),
                Some(Instruction::Directive(
                    Directive::Barrier | Directive::Reset
                ))
            );
        if global_boundary {
            flush_all(&mut frames, &mut output, &mut phase_delta);
        } else {
            for qubit in &operation.qubits {
                flush_frame(*qubit, &mut frames, &mut output, &mut phase_delta);
            }
        }
        output.push(operation);
    }
    flush_all(&mut frames, &mut output, &mut phase_delta);

    Ok(ValueRewrite {
        operations: output,
        phase_delta,
        changed,
    })
}

fn z_carrier(operation: &ValueOperation) -> Option<(Qubit, f64, f64)> {
    let ValueInstruction::Instruction(Instruction::Standard(gate)) = &operation.instruction else {
        return None;
    };
    let qubit = *operation.qubits.first()?;
    Some(match gate {
        StandardGate::RZ => (qubit, *fixed_params(operation)?.first()?, 0.0),
        StandardGate::Phase => {
            let angle = *fixed_params(operation)?.first()?;
            (qubit, angle, angle / 2.0)
        }
        StandardGate::S => (qubit, FRAC_PI_2, FRAC_PI_4),
        StandardGate::SDG => (qubit, -FRAC_PI_2, -FRAC_PI_4),
        StandardGate::T => (qubit, FRAC_PI_4, FRAC_PI_4 / 2.0),
        StandardGate::TDG => (qubit, -FRAC_PI_4, -FRAC_PI_4 / 2.0),
        _ => return None,
    })
}

fn pauli_carrier(operation: &ValueOperation) -> Option<(Qubit, bool, bool, f64)> {
    let ValueInstruction::Instruction(Instruction::Standard(gate)) = &operation.instruction else {
        return None;
    };
    let qubit = *operation.qubits.first()?;
    Some(match gate {
        StandardGate::X => (qubit, true, false, 0.0),
        StandardGate::Y => (qubit, true, true, FRAC_PI_2),
        StandardGate::Z => (qubit, false, true, 0.0),
        _ => return None,
    })
}

fn absorb_z_into_xy_axis(
    operation: &mut ValueOperation,
    frames: &BTreeMap<Qubit, QubitFrame>,
) -> bool {
    let Some(&qubit) = operation.qubits.first() else {
        return false;
    };
    if operation.qubits.len() != 1 {
        return false;
    }
    let Some(frame) = frames.get(&qubit) else {
        return false;
    };
    if frame.z_angle.abs() <= PHASE_EPS || frame.pauli_x || frame.pauli_z {
        return false;
    }
    let ValueInstruction::Instruction(Instruction::Standard(gate)) = &operation.instruction else {
        return false;
    };
    let axis_index = match gate {
        StandardGate::RXY => 1,
        StandardGate::XY | StandardGate::XY2P | StandardGate::XY2M => 0,
        _ => return false,
    };
    let Some(ParameterValue::Fixed(axis)) = operation.params.get_mut(axis_index) else {
        return false;
    };
    // G(phi) RZ(a) = RZ(a) G(phi-a), so the pending frame can stay virtual.
    *axis -= frame.z_angle;
    true
}

fn is_z_diagonal(gate: StandardGate) -> bool {
    matches!(
        gate,
        StandardGate::CZ | StandardGate::CRZ | StandardGate::RZZ
    )
}

#[derive(Clone, Copy, Default)]
struct TwoQubitPauli {
    phase: u8,
    x: [bool; 2],
    z: [bool; 2],
}

impl TwoQubitPauli {
    fn multiply(self, right: Self) -> Self {
        let anti = self
            .z
            .iter()
            .zip(right.x)
            .filter(|(z, x)| **z && *x)
            .count() as u8;
        Self {
            phase: (self.phase + right.phase + 2 * anti) % 4,
            x: [self.x[0] ^ right.x[0], self.x[1] ^ right.x[1]],
            z: [self.z[0] ^ right.z[0], self.z[1] ^ right.z[1]],
        }
    }
}

/// Conjugates a two-qubit Pauli frame through CX or CZ using their stabilizer
/// generator images in canonical `X0 X1 Z0 Z1` multiplication order.
fn propagate_clifford_paulis(
    gate: StandardGate,
    qubits: [Qubit; 2],
    frames: &mut BTreeMap<Qubit, QubitFrame>,
) -> f64 {
    let input = TwoQubitPauli {
        phase: 0,
        x: qubits.map(|qubit| frames.get(&qubit).is_some_and(|frame| frame.pauli_x)),
        z: qubits.map(|qubit| frames.get(&qubit).is_some_and(|frame| frame.pauli_z)),
    };
    let generators = match gate {
        StandardGate::CX => [
            TwoQubitPauli {
                x: [true, true],
                ..Default::default()
            },
            TwoQubitPauli {
                x: [false, true],
                ..Default::default()
            },
            TwoQubitPauli {
                z: [true, false],
                ..Default::default()
            },
            TwoQubitPauli {
                z: [true, true],
                ..Default::default()
            },
        ],
        StandardGate::CZ => [
            TwoQubitPauli {
                x: [true, false],
                z: [false, true],
                ..Default::default()
            },
            TwoQubitPauli {
                x: [false, true],
                z: [true, false],
                ..Default::default()
            },
            TwoQubitPauli {
                z: [true, false],
                ..Default::default()
            },
            TwoQubitPauli {
                z: [false, true],
                ..Default::default()
            },
        ],
        _ => unreachable!("only Clifford propagation gates are passed here"),
    };
    let enabled = [input.x[0], input.x[1], input.z[0], input.z[1]];
    let result = generators
        .into_iter()
        .zip(enabled)
        .filter(|(_, enabled)| *enabled)
        .fold(TwoQubitPauli::default(), |acc, (generator, _)| {
            acc.multiply(generator)
        });
    for (index, qubit) in qubits.into_iter().enumerate() {
        let frame = frames.entry(qubit).or_default();
        frame.pauli_x = result.x[index];
        frame.pauli_z = result.z[index];
    }
    f64::from(result.phase) * FRAC_PI_2
}

fn flush_all(
    frames: &mut BTreeMap<Qubit, QubitFrame>,
    output: &mut Vec<ValueOperation>,
    phase_delta: &mut f64,
) {
    let qubits = frames.keys().copied().collect::<Vec<_>>();
    for qubit in qubits {
        flush_frame(qubit, frames, output, phase_delta);
    }
}

fn flush_frame(
    qubit: Qubit,
    frames: &mut BTreeMap<Qubit, QubitFrame>,
    output: &mut Vec<ValueOperation>,
    phase_delta: &mut f64,
) {
    flush_pauli(qubit, frames, output, phase_delta);
    flush_z(qubit, frames, output);
    if frames.get(&qubit).is_some_and(|frame| frame.is_empty()) {
        frames.remove(&qubit);
    }
}

fn flush_pauli(
    qubit: Qubit,
    frames: &mut BTreeMap<Qubit, QubitFrame>,
    output: &mut Vec<ValueOperation>,
    phase_delta: &mut f64,
) {
    let Some(frame) = frames.get_mut(&qubit) else {
        return;
    };
    let gate = match (frame.pauli_x, frame.pauli_z) {
        (false, false) => None,
        (true, false) => Some(StandardGate::X),
        (false, true) => Some(StandardGate::Z),
        (true, true) => {
            *phase_delta -= FRAC_PI_2;
            Some(StandardGate::Y)
        }
    };
    if let Some(gate) = gate {
        output.push(ValueOperation {
            instruction: ValueInstruction::from_instruction(Instruction::Standard(gate)),
            qubits: smallvec![qubit],
            params: SmallVec::new(),
            label: None,
        });
    }
    frame.pauli_x = false;
    frame.pauli_z = false;
}

fn flush_z(
    qubit: Qubit,
    frames: &mut BTreeMap<Qubit, QubitFrame>,
    output: &mut Vec<ValueOperation>,
) {
    let Some(frame) = frames.get_mut(&qubit) else {
        return;
    };
    if frame.z_angle.abs() > PHASE_EPS {
        output.push(ValueOperation {
            instruction: ValueInstruction::from_instruction(Instruction::Standard(
                StandardGate::RZ,
            )),
            qubits: smallvec![qubit],
            params: smallvec![ParameterValue::Fixed(frame.z_angle)],
            label: None,
        });
    }
    frame.z_angle = 0.0;
}

#[cfg(test)]
#[path = "native_optimization_test.rs"]
mod native_optimization_test;
