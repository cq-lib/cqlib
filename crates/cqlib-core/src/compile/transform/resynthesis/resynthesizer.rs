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

//! Transformer entry point for numeric two-qubit block resynthesis.
//!
//! The transformer works on a flat operation sequence at each control-flow
//! level. It collects candidate blocks, synthesizes strictly improving patches,
//! emits replacements at the first matched source order, and then rebuilds a
//! fresh circuit. Rebuilding from a flat sequence avoids fragile in-place DAG
//! surgery while preserving source operations that are not part of a patch.

use super::commutation::{CachedCommutation, OperationView};
use super::config::TwoQubitBlockResynthesisConfig;
use super::dag_collector::collect_two_qubit_blocks_dag;
use super::incremental::{NativeResynthesisSession, NativeScopeId, NativeScopeSegment};
use super::maximal_run::collect_maximal_two_qubit_runs;
use super::selector::{BlockPatch, select_patches_with_device};
use super::synthesis_cache::{TwoQubitSynthesisCache, TwoQubitSynthesisCacheStats};
use crate::circuit::{
    Circuit, CircuitParam, ClassicalControlOp, Instruction, Operation, Parameter, ParameterValue,
    StandardGate, ValueClassicalControlOp, ValueControlBody, ValueInstruction, ValueOperation,
    ValueSwitchCase,
};
use crate::compile::CompilerError;
use crate::compile::transform::decompose::unitary::DeviceTwoQubitSynthesisContext;
use crate::compile::transform::rebuild::{CircuitRebuildContext, ClassicalRemap};
use crate::compile::transform::{CircuitAnalysis, RewriteEdits, TransformOutcome, Transformer};
use smallvec::smallvec;
use std::collections::{HashMap, HashSet};

const TRANSFORM_NAME: &str = "resynthesize.two_qubit_blocks";
const PHASE_EPS: f64 = 1e-12;

/// Transformer that numerically resynthesizes fixed standard-gate two-qubit
/// blocks.
///
/// Use [`TwoQubitBlockResynthesisConfig::normal`] for the default supplemental
/// search budget and [`TwoQubitBlockResynthesisConfig::enhanced`] when extra
/// compile-time can be traded for larger commutation-aware local blocks.
#[derive(Debug, Clone)]
pub struct ResynthesizeTwoQubitBlocks {
    config: TwoQubitBlockResynthesisConfig,
    device_context: Option<DeviceTwoQubitSynthesisContext>,
}

impl ResynthesizeTwoQubitBlocks {
    /// Creates a transformer from an explicit resynthesis configuration.
    pub fn new(config: TwoQubitBlockResynthesisConfig) -> Self {
        Self {
            config,
            device_context: None,
        }
    }

    pub(crate) fn new_device_aware(
        config: TwoQubitBlockResynthesisConfig,
        device_context: DeviceTwoQubitSynthesisContext,
    ) -> Self {
        Self {
            config,
            device_context: Some(device_context),
        }
    }

    pub(crate) fn is_applicable(circuit: &Circuit) -> bool {
        has_fixed_numeric_two_qubit_standard(circuit.operations(), circuit)
    }

    /// Runs resynthesis and returns the exact top-level operation provenance
    /// consumed by a later workflow rewrite session.
    pub(crate) fn transform_with_rewrite_edits(
        &self,
        circuit: &Circuit,
    ) -> Result<(TransformOutcome, RewriteEdits), CompilerError> {
        resynthesize_two_qubit_blocks_with_device_and_edits(
            circuit,
            self.config.clone(),
            self.device_context.clone(),
        )
    }
}

impl Default for ResynthesizeTwoQubitBlocks {
    fn default() -> Self {
        Self::new(TwoQubitBlockResynthesisConfig::default())
    }
}

impl Transformer for ResynthesizeTwoQubitBlocks {
    fn name(&self) -> &'static str {
        TRANSFORM_NAME
    }

    fn transform(
        &self,
        circuit: &Circuit,
        _analysis: Option<&CircuitAnalysis>,
    ) -> Result<TransformOutcome, CompilerError> {
        resynthesize_two_qubit_blocks_with_device(
            circuit,
            self.config.clone(),
            self.device_context.clone(),
        )
    }
}

/// Runs numeric two-qubit block resynthesis on `circuit`.
///
/// Returns [`TransformOutcome::Unchanged`] when the input has no fixed numeric
/// two-qubit standard gates or when every candidate fails the strict
/// improvement and commutation checks.
pub fn resynthesize_two_qubit_blocks(
    circuit: &Circuit,
    config: TwoQubitBlockResynthesisConfig,
) -> Result<TransformOutcome, CompilerError> {
    resynthesize_two_qubit_blocks_with_device(circuit, config, None)
}

fn resynthesize_two_qubit_blocks_with_device(
    circuit: &Circuit,
    config: TwoQubitBlockResynthesisConfig,
    device_context: Option<DeviceTwoQubitSynthesisContext>,
) -> Result<TransformOutcome, CompilerError> {
    resynthesize_two_qubit_blocks_with_device_and_edits(circuit, config, device_context)
        .map(|(outcome, _)| outcome)
}

fn resynthesize_two_qubit_blocks_with_device_and_edits(
    circuit: &Circuit,
    config: TwoQubitBlockResynthesisConfig,
    device_context: Option<DeviceTwoQubitSynthesisContext>,
) -> Result<(TransformOutcome, RewriteEdits), CompilerError> {
    if !has_fixed_numeric_two_qubit_standard(circuit.operations(), circuit) {
        return Ok((
            TransformOutcome::Unchanged,
            RewriteEdits::linear(
                circuit.operations().len(),
                circuit.operations().len(),
                Vec::new(),
            ),
        ));
    }

    let pass = ResynthesisPass {
        source: circuit,
        rebuild: CircuitRebuildContext::new(circuit),
        config,
        device_context,
        synthesis_cache: TwoQubitSynthesisCache::default(),
        incremental: None,
    };
    pass.run_with_edits()
}

pub(crate) fn resynthesize_two_qubit_blocks_incremental(
    circuit: &Circuit,
    config: TwoQubitBlockResynthesisConfig,
    device_context: DeviceTwoQubitSynthesisContext,
    session: &mut NativeResynthesisSession,
) -> Result<TransformOutcome, CompilerError> {
    session.begin_round(&config);
    let result = ResynthesisPass {
        source: circuit,
        rebuild: CircuitRebuildContext::new(circuit),
        config,
        device_context: Some(device_context),
        synthesis_cache: TwoQubitSynthesisCache::default(),
        incremental: Some(session),
    }
    .run();
    if result.is_ok() {
        session.finish_round();
    }
    result
}

struct ResynthesisPass<'a, 'session> {
    source: &'a Circuit,
    rebuild: CircuitRebuildContext,
    config: TwoQubitBlockResynthesisConfig,
    device_context: Option<DeviceTwoQubitSynthesisContext>,
    synthesis_cache: TwoQubitSynthesisCache,
    incremental: Option<&'session mut NativeResynthesisSession>,
}

struct SequenceRewrite {
    operations: Vec<ValueOperation>,
    /// Output-to-input correspondence for this sequence. Replacements are
    /// `None`; preserved source operations retain their original order.
    provenance: Vec<Option<usize>>,
    phase_delta: f64,
    changed: bool,
}

impl<'a, 'session> ResynthesisPass<'a, 'session> {
    fn run(self) -> Result<TransformOutcome, CompilerError> {
        self.run_with_stats().map(|(result, _)| result)
    }

    fn run_with_edits(self) -> Result<(TransformOutcome, RewriteEdits), CompilerError> {
        self.run_with_edits_and_stats()
            .map(|(outcome, edits, _)| (outcome, edits))
    }

    fn run_with_stats(
        self,
    ) -> Result<(TransformOutcome, TwoQubitSynthesisCacheStats), CompilerError> {
        self.run_with_edits_and_stats()
            .map(|(outcome, _, stats)| (outcome, stats))
    }

    fn run_with_edits_and_stats(
        mut self,
    ) -> Result<(TransformOutcome, RewriteEdits, TwoQubitSynthesisCacheStats), CompilerError> {
        let root_classical = self.rebuild.root_classical().clone();
        let rewrite = self.process_sequence(
            self.source.operations(),
            &root_classical,
            &NativeScopeId::default(),
        )?;
        let mut global_phase = self.source.global_phase();
        if rewrite.phase_delta.abs() > PHASE_EPS {
            global_phase = global_phase + Parameter::from(rewrite.phase_delta);
        }
        let circuit =
            self.rebuild
                .finish(self.source.qubits(), rewrite.operations, global_phase)?;
        let stats = self.synthesis_cache.stats();
        let edits = if rewrite.changed {
            RewriteEdits::from_operation_provenance(
                self.source.operations().len(),
                &rewrite.provenance,
            )
        } else {
            RewriteEdits::linear(
                self.source.operations().len(),
                self.source.operations().len(),
                Vec::new(),
            )
        };
        let outcome = if rewrite.changed {
            TransformOutcome::Changed(circuit)
        } else {
            TransformOutcome::Unchanged
        };
        Ok((outcome, edits, stats))
    }

    fn process_sequence(
        &mut self,
        operations: &[Operation],
        classical_remap: &ClassicalRemap,
        scope: &NativeScopeId,
    ) -> Result<SequenceRewrite, CompilerError> {
        let views = self.build_views(operations)?;
        let mut commutation = CachedCommutation::new(self.config.commutation.clone());
        let maximal_blocks = collect_maximal_two_qubit_runs(&views);
        let maximal_keys = maximal_blocks
            .iter()
            .map(|block| block.matched_orders.clone())
            .collect::<HashSet<_>>();
        let mut patches = select_patches_with_device(
            maximal_blocks,
            &views,
            &commutation,
            &self.config,
            self.device_context.as_ref(),
            &mut self.synthesis_cache,
        )?;
        let protected_orders = patches
            .iter()
            .flat_map(|patch| patch.matched_orders.iter().copied())
            .collect::<HashSet<_>>();

        // The maximal path owns every run it improves. The bounded collector
        // remains a fallback for unprofitable maximal runs and for blocks that
        // require legal commutation crossing, but it must not resynthesize an
        // identical maximal block or overlap an accepted maximal patch.
        let needs_bounded_collection = views.iter().enumerate().any(|(order, view)| {
            super::dag_collector::is_two_qubit_anchor(view, &self.config)
                && !protected_orders.contains(&order)
        });
        if needs_bounded_collection {
            let mut bounded_blocks = if let Some(incremental) = self.incremental.as_deref_mut() {
                incremental.collect_blocks(
                    scope,
                    self.source,
                    operations,
                    &views,
                    &mut commutation,
                    &self.config,
                )?
            } else {
                collect_two_qubit_blocks_dag(&views, &mut commutation, &self.config)?
            };
            bounded_blocks.retain(|block| {
                !maximal_keys.contains(&block.matched_orders)
                    && !block
                        .matched_orders
                        .iter()
                        .chain(&block.crossed_orders)
                        .any(|order| protected_orders.contains(order))
            });
            patches.extend(select_patches_with_device(
                bounded_blocks,
                &views,
                &commutation,
                &self.config,
                self.device_context.as_ref(),
                &mut self.synthesis_cache,
            )?);
            patches.sort_by_key(|patch| patch.first_order);
        }

        if patches.is_empty() {
            return self.preserve_sequence(operations, classical_remap, scope);
        }

        let mut phase_delta = 0.0;
        for patch in &patches {
            phase_delta += patch.synthesis_phase;
        }
        let (rebuilt, provenance) =
            self.emit_patched_sequence(operations, classical_remap, patches, scope)?;
        Ok(SequenceRewrite {
            operations: rebuilt,
            provenance,
            phase_delta,
            changed: true,
        })
    }

    fn build_views<'ops>(
        &self,
        operations: &'ops [Operation],
    ) -> Result<Vec<OperationView<'ops>>, CompilerError> {
        operations
            .iter()
            .enumerate()
            .map(|(order, operation)| {
                let params = operation
                    .params
                    .iter()
                    .map(|param| match param {
                        CircuitParam::Fixed(value) => Ok(Parameter::from(*value)),
                        CircuitParam::Index(index) => self
                            .source
                            .parameters()
                            .get_index(*index as usize)
                            .cloned()
                            .ok_or_else(|| {
                                CompilerError::InvalidInput(format!(
                                    "missing parameter index {index}"
                                ))
                            }),
                    })
                    .collect::<Result<smallvec::SmallVec<[_; 3]>, _>>()?;
                Ok(OperationView::new(order, operation, params))
            })
            .collect()
    }

    fn preserve_sequence(
        &mut self,
        operations: &[Operation],
        classical_remap: &ClassicalRemap,
        scope: &NativeScopeId,
    ) -> Result<SequenceRewrite, CompilerError> {
        let mut output = Vec::with_capacity(operations.len());
        let mut phase_delta = 0.0;
        let mut changed = false;
        let mut provenance = Vec::with_capacity(operations.len());
        for (order, operation) in operations.iter().enumerate() {
            let (rebuilt, body_phase, body_changed) =
                self.rebuild_operation(operation, classical_remap, scope, order)?;
            output.push(rebuilt);
            provenance.push((!body_changed).then_some(order));
            phase_delta += body_phase;
            changed |= body_changed;
        }
        Ok(SequenceRewrite {
            operations: output,
            provenance,
            phase_delta,
            changed,
        })
    }

    fn emit_patched_sequence(
        &mut self,
        operations: &[Operation],
        classical_remap: &ClassicalRemap,
        patches: Vec<BlockPatch>,
        scope: &NativeScopeId,
    ) -> Result<(Vec<ValueOperation>, Vec<Option<usize>>), CompilerError> {
        let mut patches_by_first = HashMap::new();
        let mut skipped = HashSet::new();
        for patch in patches {
            for &order in &patch.matched_orders {
                if order != patch.first_order {
                    skipped.insert(order);
                }
            }
            for &order in &patch.crossed_orders {
                debug_assert!(
                    !patch.matched_orders.contains(&order),
                    "crossed operation must not also be matched"
                );
            }
            patches_by_first.insert(patch.first_order, patch);
        }

        let mut output = Vec::with_capacity(operations.len());
        let mut provenance = Vec::with_capacity(operations.len());
        for (order, operation) in operations.iter().enumerate() {
            if let Some(patch) = patches_by_first.remove(&order) {
                provenance.extend(std::iter::repeat_n(None, patch.replacement.len()));
                output.extend(patch.replacement);
                continue;
            }
            if skipped.contains(&order) {
                continue;
            }
            let (rebuilt, _, body_changed) =
                self.rebuild_operation(operation, classical_remap, scope, order)?;
            output.push(rebuilt);
            provenance.push((!body_changed).then_some(order));
        }
        debug_assert!(
            patches_by_first.is_empty(),
            "{} unemitted resynthesis patches",
            patches_by_first.len()
        );
        Ok((output, provenance))
    }

    fn rebuild_operation(
        &mut self,
        operation: &Operation,
        classical_remap: &ClassicalRemap,
        scope: &NativeScopeId,
        operation_order: usize,
    ) -> Result<(ValueOperation, f64, bool), CompilerError> {
        if self.config.recurse_control_flow
            && let Instruction::ClassicalControl(control) = &operation.instruction
        {
            let operation_key = self
                .incremental
                .as_deref()
                .and_then(|session| session.current_operation_key(scope, operation_order))
                .unwrap_or(operation_order as u64);
            let (instruction, changed) =
                self.rebuild_control_flow(control, classical_remap, scope, operation_key)?;
            let qubits = instruction.used_qubits().into_iter().collect();
            return Ok((
                ValueOperation {
                    instruction: ValueInstruction::ClassicalControl(instruction),
                    qubits,
                    params: CircuitRebuildContext::resolve_source_params(
                        self.source,
                        &operation.params,
                    )?,
                    label: operation.label.clone(),
                },
                0.0,
                changed,
            ));
        }
        Ok((
            self.rebuild
                .remap_preserved_operation(self.source, operation, classical_remap)?,
            0.0,
            false,
        ))
    }

    fn rebuild_control_flow(
        &mut self,
        control: &ClassicalControlOp,
        classical_remap: &ClassicalRemap,
        scope: &NativeScopeId,
        operation_key: u64,
    ) -> Result<(ValueClassicalControlOp, bool), CompilerError> {
        Ok(match control {
            ClassicalControlOp::If(op) => {
                let (then_ops, then_changed) = self.rebuild_body(
                    op.then_body().operations(),
                    classical_remap,
                    &scope.child(NativeScopeSegment::IfThen(operation_key)),
                )?;
                let else_rewrite = op
                    .else_body()
                    .map(|body| {
                        self.rebuild_body(
                            body.operations(),
                            classical_remap,
                            &scope.child(NativeScopeSegment::IfElse(operation_key)),
                        )
                    })
                    .transpose()?;
                let else_changed = else_rewrite
                    .as_ref()
                    .is_some_and(|(_, body_changed)| *body_changed);
                (
                    ValueClassicalControlOp::If {
                        condition: classical_remap.remap_expr(op.condition())?,
                        then_body: ValueControlBody::new(then_ops),
                        else_body: else_rewrite.map(|(ops, _)| ValueControlBody::new(ops)),
                    },
                    then_changed || else_changed,
                )
            }
            ClassicalControlOp::While(op) => {
                let (body, changed) = self.rebuild_body(
                    op.body().operations(),
                    classical_remap,
                    &scope.child(NativeScopeSegment::WhileBody(operation_key)),
                )?;
                (
                    ValueClassicalControlOp::While {
                        condition: classical_remap.remap_expr(op.condition())?,
                        body: ValueControlBody::new(body),
                    },
                    changed,
                )
            }
            ClassicalControlOp::For(op) => {
                let (body, changed) = self.rebuild_body(
                    op.body().operations(),
                    classical_remap,
                    &scope.child(NativeScopeSegment::ForBody(operation_key)),
                )?;
                (
                    ValueClassicalControlOp::For {
                        var: classical_remap.remap_var(op.var())?,
                        start: classical_remap.remap_expr(op.start())?,
                        stop: classical_remap.remap_expr(op.stop())?,
                        step: classical_remap.remap_expr(op.step())?,
                        body: ValueControlBody::new(body),
                    },
                    changed,
                )
            }
            ClassicalControlOp::Switch(op) => {
                let mut changed = false;
                let cases = op
                    .cases()
                    .iter()
                    .enumerate()
                    .map(|(case_index, case)| {
                        let (body, body_changed) = self.rebuild_body(
                            case.body().operations(),
                            classical_remap,
                            &scope.child(NativeScopeSegment::SwitchCase {
                                operation: operation_key,
                                case: case_index,
                            }),
                        )?;
                        changed |= body_changed;
                        Ok(ValueSwitchCase::new(
                            case.value(),
                            ValueControlBody::new(body),
                        ))
                    })
                    .collect::<Result<Vec<_>, CompilerError>>()?;
                let default_rewrite = op
                    .default()
                    .map(|body| {
                        self.rebuild_body(
                            body.operations(),
                            classical_remap,
                            &scope.child(NativeScopeSegment::SwitchDefault(operation_key)),
                        )
                    })
                    .transpose()?;
                changed |= default_rewrite
                    .as_ref()
                    .is_some_and(|(_, body_changed)| *body_changed);
                (
                    ValueClassicalControlOp::Switch {
                        target: classical_remap.remap_expr(op.target())?,
                        cases,
                        default: default_rewrite.map(|(ops, _)| ValueControlBody::new(ops)),
                    },
                    changed,
                )
            }
            ClassicalControlOp::Break => (ValueClassicalControlOp::Break, false),
            ClassicalControlOp::Continue => (ValueClassicalControlOp::Continue, false),
        })
    }

    fn rebuild_body(
        &mut self,
        operations: &[Operation],
        classical_remap: &ClassicalRemap,
        scope: &NativeScopeId,
    ) -> Result<(Vec<ValueOperation>, bool), CompilerError> {
        let mut rewrite = self.process_sequence(operations, classical_remap, scope)?;
        if rewrite.phase_delta.abs() > PHASE_EPS {
            rewrite.operations.insert(
                0,
                ValueOperation {
                    instruction: ValueInstruction::from_instruction(Instruction::Standard(
                        StandardGate::GPhase,
                    )),
                    qubits: smallvec![],
                    params: smallvec![ParameterValue::Fixed(rewrite.phase_delta)],
                    label: None,
                },
            );
            rewrite.changed = true;
        }
        Ok((rewrite.operations, rewrite.changed))
    }
}

fn has_fixed_numeric_two_qubit_standard(operations: &[Operation], circuit: &Circuit) -> bool {
    operations
        .iter()
        .any(|operation| match &operation.instruction {
            Instruction::Standard(gate)
                if operation.qubits.len() == 2
                    && gate.num_qubits() == 2
                    && operation.params.iter().all(|param| {
                        circuit
                            .resolve_parameter(param)
                            .ok()
                            .and_then(|p| p.evaluate(&None).ok())
                            .is_some_and(f64::is_finite)
                    }) =>
            {
                true
            }
            Instruction::ClassicalControl(control) => {
                control_has_fixed_numeric_2q(control, circuit)
            }
            _ => false,
        })
}

fn control_has_fixed_numeric_2q(control: &ClassicalControlOp, circuit: &Circuit) -> bool {
    match control {
        ClassicalControlOp::If(op) => {
            has_fixed_numeric_two_qubit_standard(op.then_body().operations(), circuit)
                || op.else_body().is_some_and(|body| {
                    has_fixed_numeric_two_qubit_standard(body.operations(), circuit)
                })
        }
        ClassicalControlOp::While(op) => {
            has_fixed_numeric_two_qubit_standard(op.body().operations(), circuit)
        }
        ClassicalControlOp::For(op) => {
            has_fixed_numeric_two_qubit_standard(op.body().operations(), circuit)
        }
        ClassicalControlOp::Switch(op) => {
            op.cases()
                .iter()
                .any(|case| has_fixed_numeric_two_qubit_standard(case.body().operations(), circuit))
                || op.default().is_some_and(|body| {
                    has_fixed_numeric_two_qubit_standard(body.operations(), circuit)
                })
        }
        ClassicalControlOp::Break | ClassicalControlOp::Continue => false,
    }
}

#[cfg(test)]
#[path = "resynthesizer_test.rs"]
mod resynthesizer_test;
