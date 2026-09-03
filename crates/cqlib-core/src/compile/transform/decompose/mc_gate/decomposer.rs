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

//! Circuit-level multi-controlled-gate decomposition.
//!
//! This module connects the pure synthesis primitives in [`super`] to the
//! compiler ancillary-resource manager. It rebuilds a circuit while replacing
//! every [`Instruction::McGate`] with a deterministic, resource-aware
//! decomposition. Control-flow bodies are rebuilt recursively because each
//! selected primitive restores its leased ancillas before the emitted sequence
//! completes.
//!
//! Candidate selection is intentionally separate from physical routing. This
//! pass operates on logical qubits before layout. The device-aware convenience
//! entry point uses device capacity as a hard logical-qubit limit, but does not
//! claim to satisfy coupling-map constraints or score candidates by topology.

use super::{
    decompose_fsim_n_clean, decompose_fsim_no_aux, decompose_hadamard_n_clean,
    decompose_hadamard_no_aux, decompose_pauli_1_clean_b95, decompose_pauli_1_clean_kg24,
    decompose_pauli_1_dirty, decompose_pauli_2_clean, decompose_pauli_2_dirty,
    decompose_pauli_n_clean, decompose_pauli_n_dirty, decompose_pauli_no_aux,
    decompose_pauli_rotation_n_clean, decompose_pauli_rotation_no_aux, decompose_pauli_small,
    decompose_phase_n_clean, decompose_phase_no_aux, decompose_qcis_n_clean, decompose_qcis_no_aux,
    decompose_rotation_n_clean, decompose_rotation_no_aux, decompose_swap_n_clean,
    decompose_swap_no_aux, decompose_unitary_n_clean, decompose_unitary_no_aux,
};
use crate::circuit::operation::ValueOperation;
use crate::circuit::value_instruction::ValueInstruction;
use crate::circuit::{
    Circuit, CircuitParam, ClassicalControlOp, Instruction, MCGate, Operation, ParameterValue,
    Qubit, StandardGate, ValueClassicalControlOp, ValueControlBody, ValueSwitchCase,
};
use crate::compile::CompilerError;
use crate::compile::resource::{
    AncillaRequirement, ResourceError, ResourceLimits, ResourceManager, ResourcePolicy,
    ResourceRequest,
};
use crate::compile::transform::analysis::WorkflowCircuitAnalysis;
use crate::compile::transform::decompose::rule::{
    DecompositionAlgorithm, DecompositionRuleCache, DecompositionRuleStats, McGateRuleRequest,
    ResourceSignature,
};
use crate::compile::transform::rebuild::{CircuitRebuildContext, ClassicalRemap};
use crate::compile::transform::transformer::{PassApplicability, WorkflowPass};
use crate::compile::transform::{CircuitAnalysis, TransformOutcome, Transformer};
use crate::device::Device;
use std::collections::BTreeSet;

const DECOMPOSE_MC_GATES_NAME: &str = "decompose.mc_gates";

/// Configuration for circuit-level multi-controlled-gate decomposition.
///
/// The policy controls whether synthesis may allocate clean logical ancillas
/// before layout or borrow input qubits under the dirty-restoration contract.
/// The limits provide hard logical-qubit bounds, such as target-device
/// capacity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct McGateDecomposeConfig {
    /// Ancillary-resource permissions for this decomposition pass.
    pub resource_policy: ResourcePolicy,
    /// Hard logical-qubit limits for this decomposition pass.
    pub resource_limits: ResourceLimits,
}

/// [`Transformer`] adapter for [`decompose_mc_gates`].
///
/// Configuration is bound at construction time.
#[derive(Debug, Clone)]
pub struct DecomposeMcGates {
    config: McGateDecomposeConfig,
}

impl DecomposeMcGates {
    /// Creates a transformer with an explicit resource policy and limits.
    pub fn new(config: McGateDecomposeConfig) -> Self {
        Self { config }
    }
}

impl Default for DecomposeMcGates {
    fn default() -> Self {
        Self::new(McGateDecomposeConfig::default())
    }
}

impl WorkflowPass for DecomposeMcGates {
    fn applicability(&self, analysis: &WorkflowCircuitAnalysis) -> PassApplicability {
        if analysis.has_mc_gates() {
            PassApplicability::Run
        } else {
            PassApplicability::ProvenNoOp("circuit contains no multi-controlled operations")
        }
    }
}

impl Transformer for DecomposeMcGates {
    fn name(&self) -> &'static str {
        DECOMPOSE_MC_GATES_NAME
    }

    fn transform(
        &self,
        circuit: &Circuit,
        analysis: Option<&CircuitAnalysis>,
    ) -> Result<TransformOutcome, CompilerError> {
        let local_analysis;
        let analysis = match analysis {
            Some(analysis) => analysis,
            None => {
                local_analysis = CircuitAnalysis::analyze(circuit);
                &local_analysis
            }
        };
        if !analysis.has_mc_gates {
            return Ok(TransformOutcome::Unchanged);
        }
        decompose_mc_gates(circuit, self.config)
    }
}

/// Rewrites every supported [`Instruction::McGate`] in `circuit`.
///
/// Multi-controlled gates inside `if`, `else`, and `while` bodies are rebuilt
/// recursively. Expanded operations do not inherit the source `McGate` label,
/// because one source operation may lower to many operations with different
/// roles. Preserved operations retain their labels.
///
/// Candidate selection is deterministic. Multi-controlled Pauli gates try the
/// available exact MCX algorithms in a fixed two-qubit-cost-oriented order and
/// fall back to ancillary-free synthesis. Other supported gate families try
/// their clean-accumulator primitive first when it would consume ancillas, then
/// fall back to ancillary-free synthesis.
///
/// # Errors
///
/// Returns [`CompilerError`] when the input circuit is inconsistent, a
/// multi-controlled gate family is unsupported, all synthesis candidates fail,
/// or ancillary-resource bookkeeping violates its contract.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::circuit::{Circuit, Instruction, MCGate, Qubit, StandardGate};
/// use cqlib_core::compile::resource::ResourcePolicy;
/// use cqlib_core::compile::transform::{TransformOutcome, decompose::mc_gate::{
///     McGateDecomposeConfig, decompose_mc_gates,
/// }};
///
/// let mut circuit = Circuit::new(3);
/// circuit
///     .append(
///         Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
///         [Qubit::new(0), Qubit::new(1), Qubit::new(2)],
///         [],
///         None,
///     )
///     .unwrap();
///
/// let result = decompose_mc_gates(
///     &circuit,
///     McGateDecomposeConfig {
///         resource_policy: ResourcePolicy::default(),
///         ..McGateDecomposeConfig::default()
///     },
/// )
/// .unwrap();
///
/// let TransformOutcome::Changed(decomposed) = result else {
///     panic!("the MC gate should be decomposed");
/// };
/// assert!(matches!(
///     decomposed.operations()[0].instruction,
///     Instruction::Standard(StandardGate::CCX),
/// ));
/// ```
pub fn decompose_mc_gates(
    circuit: &Circuit,
    config: McGateDecomposeConfig,
) -> Result<TransformOutcome, CompilerError> {
    McGateDecomposer::new(circuit, config)?.run()
}

/// Rewrites multi-controlled gates and returns runtime decomposition-rule stats.
///
/// This is the diagnostic form of [`decompose_mc_gates`]. The returned stats
/// describe pass-local runtime rule reuse during this decomposition run.
pub fn decompose_mc_gates_with_rule_stats(
    circuit: &Circuit,
    config: McGateDecomposeConfig,
) -> Result<(TransformOutcome, DecompositionRuleStats), CompilerError> {
    McGateDecomposer::new(circuit, config)?.run_with_rule_stats()
}

/// Rewrites multi-controlled gates while enforcing target-device capacity.
///
/// This is a pre-layout logical transform. It limits the complete logical
/// circuit to [`Device::num_usable_qubits`], but deliberately does not inspect
/// coupling topology. Physical connectivity belongs to later layout and
/// routing stages where logical qubits have physical mappings.
///
/// # Errors
///
/// Returns the errors documented by [`decompose_mc_gates`], including an input
/// error when the source circuit is already wider than the usable device.
pub fn decompose_mc_gates_for_device(
    circuit: &Circuit,
    device: &Device,
    resource_policy: ResourcePolicy,
) -> Result<TransformOutcome, CompilerError> {
    decompose_mc_gates(
        circuit,
        McGateDecomposeConfig {
            resource_policy,
            resource_limits: ResourceLimits {
                max_total_qubits: Some(device.num_usable_qubits()),
            },
        },
    )
}

struct McGateDecomposer<'a> {
    source: &'a Circuit,
    rebuild: CircuitRebuildContext,
    resource_circuit: Circuit,
    resources: ResourceManager,
    rule_cache: DecompositionRuleCache,
    changed: bool,
}

#[derive(Clone, Copy)]
struct McSynthesisContext<'a> {
    gate: StandardGate,
    params: &'a [ParameterValue],
    controls: &'a [Qubit],
    targets: &'a [Qubit],
}

#[derive(Clone, Copy)]
struct OptionalCleanAlgorithms {
    clean: DecompositionAlgorithm,
    no_aux: DecompositionAlgorithm,
}

impl<'a> McGateDecomposer<'a> {
    /// Creates the target circuit shell and initializes ancillary bookkeeping.
    ///
    /// The target shell is built before resource initialization so the manager
    /// sees exactly the logical qubits that are already occupied.
    fn new(source: &'a Circuit, config: McGateDecomposeConfig) -> Result<Self, CompilerError> {
        let resource_circuit = Circuit::from_qubits(source.qubits())?;
        let resources = ResourceManager::from_circuit(
            &resource_circuit,
            config.resource_policy,
            config.resource_limits,
        )
        .map_err(resource_input_failed)?;
        Ok(Self {
            source,
            rebuild: CircuitRebuildContext::new(source),
            resource_circuit,
            resources,
            rule_cache: DecompositionRuleCache::default(),
            changed: false,
        })
    }

    /// Rebuilds the source circuit in order and verifies all leases are idle.
    ///
    /// Ancilla leases are scoped to the synthesis of one source operation and
    /// must all be released when the pass completes.
    fn run(self) -> Result<TransformOutcome, CompilerError> {
        self.run_with_rule_stats().map(|(result, _)| result)
    }

    fn run_with_rule_stats(
        mut self,
    ) -> Result<(TransformOutcome, DecompositionRuleStats), CompilerError> {
        let source = self.source;
        let root_classical = self.rebuild.root_classical().clone();
        let mut operations = Vec::with_capacity(source.operations().len());
        for operation in source.operations() {
            operations.extend(self.rebuild_operation(operation, &root_classical)?);
        }
        self.resources
            .verify_idle(&self.resource_circuit)
            .map_err(resource_invariant_failed)?;
        let qubits = self.resource_circuit.qubits();
        let circuit = self
            .rebuild
            .finish(qubits, operations, source.global_phase())?;
        let stats = self.rule_cache.stats();
        let outcome = if self.changed {
            TransformOutcome::Changed(circuit)
        } else {
            TransformOutcome::Unchanged
        };
        Ok((outcome, stats))
    }

    /// Rebuilds a sequence of operations for a control-flow body.
    ///
    /// Bodies are rebuilt into plain operation vectors first; their enclosing
    /// control instruction is reconstructed by the caller.
    fn rebuild_sequence(
        &mut self,
        source_operations: &[Operation],
        classical_remap: &ClassicalRemap,
    ) -> Result<Vec<ValueOperation>, CompilerError> {
        let mut operations = Vec::with_capacity(source_operations.len());
        for operation in source_operations {
            operations.extend(self.rebuild_operation(operation, classical_remap)?);
        }
        Ok(operations)
    }

    /// Rebuilds one source operation, possibly expanding it to many operations.
    ///
    /// Preserved operations still pass through parameter remapping so the target
    /// circuit owns a consistent parameter table.
    fn rebuild_operation(
        &mut self,
        operation: &Operation,
        classical_remap: &ClassicalRemap,
    ) -> Result<Vec<ValueOperation>, CompilerError> {
        match &operation.instruction {
            Instruction::McGate(gate) => self.decompose_mc_operation(gate, operation),
            Instruction::ClassicalControl(control) => Ok(vec![self.rebuild_control_flow(
                operation,
                control,
                classical_remap,
            )?]),
            _ => Ok(vec![self.rebuild.remap_preserved_operation(
                self.source,
                operation,
                classical_remap,
            )?]),
        }
    }

    /// Rebuilds a classical-control operation while recursively lowering bodies.
    ///
    /// Classical structure is preserved exactly. The operation qubit list is
    /// recomputed from the rebuilt instruction because body expansion may add
    /// ancillas.
    fn rebuild_control_flow(
        &mut self,
        operation: &Operation,
        control: &ClassicalControlOp,
        classical_remap: &ClassicalRemap,
    ) -> Result<ValueOperation, CompilerError> {
        let instruction = match control {
            ClassicalControlOp::If(op) => {
                let then_body =
                    self.rebuild_sequence(op.then_body().operations(), classical_remap)?;
                let else_body = op
                    .else_body()
                    .map(|body| self.rebuild_sequence(body.operations(), classical_remap))
                    .transpose()?;
                ValueClassicalControlOp::If {
                    condition: classical_remap.remap_expr(op.condition())?,
                    then_body: ValueControlBody::new(then_body),
                    else_body: else_body.map(ValueControlBody::new),
                }
            }
            ClassicalControlOp::While(op) => {
                let body = self.rebuild_sequence(op.body().operations(), classical_remap)?;
                ValueClassicalControlOp::While {
                    condition: classical_remap.remap_expr(op.condition())?,
                    body: ValueControlBody::new(body),
                }
            }
            ClassicalControlOp::For(op) => {
                let body = self.rebuild_sequence(op.body().operations(), classical_remap)?;
                ValueClassicalControlOp::For {
                    var: classical_remap.remap_var(op.var())?,
                    start: classical_remap.remap_expr(op.start())?,
                    stop: classical_remap.remap_expr(op.stop())?,
                    step: classical_remap.remap_expr(op.step())?,
                    body: ValueControlBody::new(body),
                }
            }
            ClassicalControlOp::Switch(op) => {
                let cases = op
                    .cases()
                    .iter()
                    .map(|case| {
                        Ok(ValueSwitchCase::new(
                            case.value(),
                            ValueControlBody::new(
                                self.rebuild_sequence(case.body().operations(), classical_remap)?,
                            ),
                        ))
                    })
                    .collect::<Result<Vec<_>, CompilerError>>()?;
                let default = op
                    .default()
                    .map(|body| self.rebuild_sequence(body.operations(), classical_remap))
                    .transpose()?
                    .map(ValueControlBody::new);
                ValueClassicalControlOp::Switch {
                    target: classical_remap.remap_expr(op.target())?,
                    cases,
                    default,
                }
            }
            ClassicalControlOp::Break => ValueClassicalControlOp::Break,
            ClassicalControlOp::Continue => ValueClassicalControlOp::Continue,
        };
        let qubits = instruction.used_qubits().into_iter().collect();
        Ok(ValueOperation {
            instruction: ValueInstruction::ClassicalControl(instruction),
            qubits,
            params: CircuitRebuildContext::resolve_source_params(self.source, &operation.params)?,
            label: operation.label.clone(),
        })
    }

    /// Decomposes one storage-level `McGate` operation.
    ///
    /// Storage-level parameters are resolved against the source circuit before
    /// invoking pure value-level synthesis primitives.
    fn decompose_mc_operation(
        &mut self,
        gate: &MCGate,
        operation: &Operation,
    ) -> Result<Vec<ValueOperation>, CompilerError> {
        self.validate_mc_operation(gate, operation)?;
        let params = self.resolve_source_params(&operation.params)?;
        let num_controls = gate.num_ctrl_qubits();
        let controls = &operation.qubits[..num_controls];
        let targets = &operation.qubits[num_controls..];
        let operations = self.synthesize_mc_gate(*gate.base_gate(), &params, controls, targets)?;
        self.changed = true;
        Ok(operations)
    }

    fn validate_mc_operation(
        &self,
        gate: &MCGate,
        operation: &Operation,
    ) -> Result<(), CompilerError> {
        if operation.qubits.len() != gate.num_qubits() {
            return Err(CompilerError::InvalidInput(format!(
                "multi-controlled gate {gate} expects {} qubits, got {}",
                gate.num_qubits(),
                operation.qubits.len()
            )));
        }
        if operation.params.len() != gate.num_params() {
            return Err(CompilerError::InvalidInput(format!(
                "multi-controlled gate {gate} expects {} parameters, got {}",
                gate.num_params(),
                operation.params.len()
            )));
        }
        Ok(())
    }

    /// Dispatches synthesis by base-gate arity and parameter contract.
    ///
    /// Each branch validates the primitive signature before trying
    /// resource-dependent synthesis candidates.
    fn synthesize_mc_gate(
        &mut self,
        gate: StandardGate,
        params: &[ParameterValue],
        controls: &[Qubit],
        targets: &[Qubit],
    ) -> Result<Vec<ValueOperation>, CompilerError> {
        if controls.is_empty() {
            return Ok(vec![ValueOperation::from_standard(
                gate,
                targets.iter().copied(),
                params.iter().cloned(),
            )]);
        }

        let excluded = controls
            .iter()
            .chain(targets)
            .copied()
            .collect::<BTreeSet<_>>();
        match gate {
            StandardGate::X
            | StandardGate::CX
            | StandardGate::CCX
            | StandardGate::Y
            | StandardGate::CY
            | StandardGate::Z
            | StandardGate::CZ => self.synthesize_pauli(
                gate,
                params,
                controls,
                Self::one_target(gate, targets)?,
                &excluded,
            ),
            StandardGate::RX
            | StandardGate::RY
            | StandardGate::RZ
            | StandardGate::CRX
            | StandardGate::CRY
            | StandardGate::CRZ => {
                let theta = Self::one_param(gate, params)?;
                let target = Self::one_target(gate, targets)?;
                self.synthesize_with_optional_clean(
                    McSynthesisContext {
                        gate,
                        params,
                        controls,
                        targets: &[target],
                    },
                    controls.len().saturating_sub(1),
                    &excluded,
                    OptionalCleanAlgorithms {
                        clean: DecompositionAlgorithm::CleanAccumulator,
                        no_aux: DecompositionAlgorithm::NoAux,
                    },
                    |ancillas| decompose_rotation_n_clean(gate, theta, controls, target, ancillas),
                    || decompose_rotation_no_aux(gate, theta, controls, target),
                )
            }
            StandardGate::S
            | StandardGate::SDG
            | StandardGate::T
            | StandardGate::TDG
            | StandardGate::Phase => {
                let theta = Self::phase_param(gate, params)?;
                let target = Self::one_target(gate, targets)?;
                self.synthesize_with_optional_clean(
                    McSynthesisContext {
                        gate,
                        params,
                        controls,
                        targets: &[target],
                    },
                    controls.len().saturating_sub(1),
                    &excluded,
                    OptionalCleanAlgorithms {
                        clean: DecompositionAlgorithm::CleanAccumulator,
                        no_aux: DecompositionAlgorithm::NoAux,
                    },
                    |ancillas| decompose_phase_n_clean(gate, theta, controls, target, ancillas),
                    || decompose_phase_no_aux(gate, theta, controls, target),
                )
            }
            StandardGate::H => {
                let target = Self::one_target(gate, targets)?;
                self.synthesize_with_optional_clean(
                    McSynthesisContext {
                        gate,
                        params,
                        controls,
                        targets: &[target],
                    },
                    controls.len().saturating_sub(1),
                    &excluded,
                    OptionalCleanAlgorithms {
                        clean: DecompositionAlgorithm::CleanAccumulator,
                        no_aux: DecompositionAlgorithm::NoAux,
                    },
                    |ancillas| decompose_hadamard_n_clean(controls, target, ancillas),
                    || decompose_hadamard_no_aux(controls, target),
                )
            }
            StandardGate::U => {
                let [theta, phi, lambda] = Self::three_params(gate, params)?;
                let target = Self::one_target(gate, targets)?;
                self.synthesize_with_optional_clean(
                    McSynthesisContext {
                        gate,
                        params,
                        controls,
                        targets: &[target],
                    },
                    controls.len().saturating_sub(1),
                    &excluded,
                    OptionalCleanAlgorithms {
                        clean: DecompositionAlgorithm::CleanAccumulator,
                        no_aux: DecompositionAlgorithm::NoAux,
                    },
                    |ancillas| {
                        decompose_unitary_n_clean(theta, phi, lambda, controls, target, ancillas)
                    },
                    || decompose_unitary_no_aux(theta, phi, lambda, controls, target),
                )
            }
            StandardGate::RXX | StandardGate::RYY | StandardGate::RZZ | StandardGate::RZX => {
                let theta = Self::one_param(gate, params)?;
                let [first, second] = Self::two_targets(gate, targets)?;
                self.synthesize_with_optional_clean(
                    McSynthesisContext {
                        gate,
                        params,
                        controls,
                        targets,
                    },
                    controls.len().saturating_sub(1),
                    &excluded,
                    OptionalCleanAlgorithms {
                        clean: DecompositionAlgorithm::CleanAccumulator,
                        no_aux: DecompositionAlgorithm::NoAux,
                    },
                    |ancillas| {
                        decompose_pauli_rotation_n_clean(
                            gate, theta, controls, first, second, ancillas,
                        )
                    },
                    || decompose_pauli_rotation_no_aux(gate, theta, controls, first, second),
                )
            }
            StandardGate::SWAP => {
                let [first, second] = Self::two_targets(gate, targets)?;
                self.synthesize_with_optional_clean(
                    McSynthesisContext {
                        gate,
                        params,
                        controls,
                        targets,
                    },
                    controls.len().saturating_sub(1),
                    &excluded,
                    OptionalCleanAlgorithms {
                        clean: DecompositionAlgorithm::CleanAccumulator,
                        no_aux: DecompositionAlgorithm::NoAux,
                    },
                    |ancillas| decompose_swap_n_clean(controls, first, second, ancillas),
                    || decompose_swap_no_aux(controls, first, second),
                )
            }
            StandardGate::X2P
            | StandardGate::X2M
            | StandardGate::Y2P
            | StandardGate::Y2M
            | StandardGate::XY2P
            | StandardGate::XY2M => {
                let target = Self::one_target(gate, targets)?;
                self.synthesize_with_optional_clean(
                    McSynthesisContext {
                        gate,
                        params,
                        controls,
                        targets: &[target],
                    },
                    controls.len().saturating_sub(1),
                    &excluded,
                    OptionalCleanAlgorithms {
                        clean: DecompositionAlgorithm::CleanAccumulator,
                        no_aux: DecompositionAlgorithm::NoAux,
                    },
                    |ancillas| decompose_qcis_n_clean(gate, params, controls, target, ancillas),
                    || decompose_qcis_no_aux(gate, params, controls, target),
                )
            }
            StandardGate::FSIM => {
                let [first, second] = Self::two_targets(gate, targets)?;
                self.synthesize_with_optional_clean(
                    McSynthesisContext {
                        gate,
                        params,
                        controls,
                        targets,
                    },
                    controls.len(),
                    &excluded,
                    OptionalCleanAlgorithms {
                        clean: DecompositionAlgorithm::CleanAccumulator,
                        no_aux: DecompositionAlgorithm::NoAux,
                    },
                    |ancillas| decompose_fsim_n_clean(params, controls, first, second, ancillas),
                    || decompose_fsim_no_aux(params, controls, first, second),
                )
            }
            _ => Err(CompilerError::TransformFailed {
                name: DECOMPOSE_MC_GATES_NAME,
                reason: format!("multi-controlled {gate} decomposition is not supported"),
            }),
        }
    }

    fn synthesize_pauli(
        &mut self,
        pauli: StandardGate,
        params: &[ParameterValue],
        controls: &[Qubit],
        target: Qubit,
        excluded: &BTreeSet<Qubit>,
    ) -> Result<Vec<ValueOperation>, CompilerError> {
        let targets = [target];
        let context = McSynthesisContext {
            gate: pauli,
            params,
            controls,
            targets: &targets,
        };
        if controls.len() <= 2 {
            return self.cached_synthesis(
                context,
                &[],
                ResourceSignature::no_aux(DecompositionAlgorithm::PauliSmall),
                || decompose_pauli_small(pauli, controls, target),
            );
        }

        // Try exact Pauli/MCX candidates in a fixed two-qubit-cost-oriented
        // order: 2 clean KG24, 1 clean KG24, n clean V-chain, n dirty V-chain,
        // 2 dirty KG24, 1 dirty KG24, 1 clean B95, then no-auxiliary fallback.
        // This keeps selection deterministic while preferring low-cost
        // ancillary-assisted constructions when the resource policy allows
        // them.
        if let Some(operations) = self.try_cached_resource_candidate(
            context,
            excluded,
            AncillaRequirement::CleanZero,
            2,
            DecompositionAlgorithm::PauliTwoClean,
            |ancillas| decompose_pauli_2_clean(pauli, controls, target, [ancillas[0], ancillas[1]]),
        )? {
            return Ok(operations);
        }
        if let Some(operations) = self.try_cached_resource_candidate(
            context,
            excluded,
            AncillaRequirement::CleanZero,
            1,
            DecompositionAlgorithm::PauliOneCleanKg24,
            |ancillas| decompose_pauli_1_clean_kg24(pauli, controls, target, ancillas[0]),
        )? {
            return Ok(operations);
        }

        let v_chain_ancillas = controls.len() - 2;
        if let Some(operations) = self.try_cached_resource_candidate(
            context,
            excluded,
            AncillaRequirement::CleanZero,
            v_chain_ancillas,
            DecompositionAlgorithm::PauliManyClean,
            |ancillas| decompose_pauli_n_clean(pauli, controls, target, ancillas),
        )? {
            return Ok(operations);
        }
        if let Some(operations) = self.try_cached_resource_candidate(
            context,
            excluded,
            AncillaRequirement::Dirty,
            v_chain_ancillas,
            DecompositionAlgorithm::PauliManyDirty,
            |ancillas| decompose_pauli_n_dirty(pauli, controls, target, ancillas),
        )? {
            return Ok(operations);
        }
        if let Some(operations) = self.try_cached_resource_candidate(
            context,
            excluded,
            AncillaRequirement::Dirty,
            2,
            DecompositionAlgorithm::PauliTwoDirty,
            |ancillas| decompose_pauli_2_dirty(pauli, controls, target, [ancillas[0], ancillas[1]]),
        )? {
            return Ok(operations);
        }
        if let Some(operations) = self.try_cached_resource_candidate(
            context,
            excluded,
            AncillaRequirement::Dirty,
            1,
            DecompositionAlgorithm::PauliOneDirty,
            |ancillas| decompose_pauli_1_dirty(pauli, controls, target, ancillas[0]),
        )? {
            return Ok(operations);
        }
        if let Some(operations) = self.try_cached_resource_candidate(
            context,
            excluded,
            AncillaRequirement::CleanZero,
            1,
            DecompositionAlgorithm::PauliOneCleanB95,
            |ancillas| decompose_pauli_1_clean_b95(pauli, controls, target, ancillas[0]),
        )? {
            return Ok(operations);
        }
        self.cached_synthesis(
            context,
            &[],
            ResourceSignature::no_aux(DecompositionAlgorithm::PauliNoAux),
            || decompose_pauli_no_aux(pauli, controls, target),
        )
    }

    /// Tries a clean-ancilla construction before a no-auxiliary fallback.
    ///
    /// Most non-Pauli controlled gates have one preferred clean-ancilla
    /// construction and one no-auxiliary fallback. Resource policy decides
    /// whether the clean construction can be attempted.
    fn synthesize_with_optional_clean(
        &mut self,
        context: McSynthesisContext<'_>,
        required_ancillas: usize,
        excluded: &BTreeSet<Qubit>,
        algorithms: OptionalCleanAlgorithms,
        synthesize_clean: impl FnOnce(&[Qubit]) -> Result<Vec<ValueOperation>, CompilerError>,
        synthesize_no_aux: impl FnOnce() -> Result<Vec<ValueOperation>, CompilerError>,
    ) -> Result<Vec<ValueOperation>, CompilerError> {
        if required_ancillas > 0
            && let Some(operations) = self.try_cached_resource_candidate(
                context,
                excluded,
                AncillaRequirement::CleanZero,
                required_ancillas,
                algorithms.clean,
                synthesize_clean,
            )?
        {
            return Ok(operations);
        }
        self.cached_synthesis(
            context,
            &[],
            ResourceSignature::no_aux(algorithms.no_aux),
            synthesize_no_aux,
        )
    }

    fn try_cached_resource_candidate(
        &mut self,
        context: McSynthesisContext<'_>,
        excluded: &BTreeSet<Qubit>,
        requirement: AncillaRequirement,
        count: usize,
        algorithm: DecompositionAlgorithm,
        synthesize: impl FnOnce(&[Qubit]) -> Result<Vec<ValueOperation>, CompilerError>,
    ) -> Result<Option<Vec<ValueOperation>>, CompilerError> {
        let request = ResourceRequest {
            requirement,
            count,
            excluded: excluded.clone(),
        };
        let plan = match self.resources.preview(&request) {
            Ok(plan) => plan,
            Err(error) if resource_candidate_is_unavailable(&error) => return Ok(None),
            Err(error) => return Err(resource_invariant_failed(error)),
        };

        // Synthesis primitives are pure. Generate from the prospective qubits
        // before committing so a rejected candidate cannot allocate ancillas
        // or invalidate later previews.
        let signature = match requirement {
            AncillaRequirement::CleanZero => ResourceSignature::clean(algorithm, count),
            AncillaRequirement::Dirty => ResourceSignature::dirty(algorithm, count),
        };
        let operations = match self.cached_synthesis(context, plan.qubits(), signature, || {
            synthesize(plan.qubits())
        }) {
            Ok(operations) => operations,
            Err(CompilerError::TransformFailed { .. }) => return Ok(None),
            Err(error) => return Err(error),
        };
        // Some recursive primitives accept a fixed ancillary signature even
        // when a small instance does not consume every provided qubit. Avoid
        // widening the logical circuit for resources that the emitted circuit
        // does not actually use; the next candidate can synthesize the same
        // instance with a tighter lease.
        if !plan
            .qubits()
            .iter()
            .all(|qubit| value_operations_use_qubit(&operations, *qubit))
        {
            return Ok(None);
        }
        let lease = self
            .resources
            .commit(&mut self.resource_circuit, plan)
            .map_err(resource_invariant_failed)?;
        self.resources
            .release(&lease)
            .map_err(resource_invariant_failed)?;
        Ok(Some(operations))
    }

    fn cached_synthesis(
        &mut self,
        context: McSynthesisContext<'_>,
        ancillas: &[Qubit],
        resource: ResourceSignature,
        synthesize: impl FnOnce() -> Result<Vec<ValueOperation>, CompilerError>,
    ) -> Result<Vec<ValueOperation>, CompilerError> {
        let request = McGateRuleRequest {
            gate: context.gate,
            control_count: context.controls.len(),
            target_count: context.targets.len(),
            params: context.params,
            resource,
        };
        if let Some(operations) = self.rule_cache.instantiate_mc_gate(
            request,
            context.controls,
            context.targets,
            ancillas,
        )? {
            return Ok(operations);
        }

        let operations = synthesize()?;
        self.rule_cache.insert_mc_gate(
            request,
            context.controls,
            context.targets,
            ancillas,
            &operations,
        )?;
        Ok(operations)
    }

    /// Resolves source-table parameters into value-level synthesis parameters.
    ///
    /// This keeps primitive synthesis independent from the source circuit's
    /// storage indices.
    fn resolve_source_params(
        &self,
        params: &[CircuitParam],
    ) -> Result<Vec<ParameterValue>, CompilerError> {
        params
            .iter()
            .map(|param| self.resolve_source_param(param))
            .collect()
    }

    fn resolve_source_param(&self, param: &CircuitParam) -> Result<ParameterValue, CompilerError> {
        match param {
            CircuitParam::Fixed(value) => {
                if !value.is_finite() {
                    return Err(CompilerError::InvalidInput(format!(
                        "non-finite fixed parameter {value}"
                    )));
                }
                Ok(ParameterValue::Fixed(*value))
            }
            CircuitParam::Index(index) => self
                .source
                .parameters()
                .get_index(*index as usize)
                .cloned()
                .map(ParameterValue::Param)
                .ok_or_else(|| {
                    CompilerError::InvalidInput(format!("missing parameter index {index}"))
                }),
        }
    }

    fn one_param(
        gate: StandardGate,
        params: &[ParameterValue],
    ) -> Result<&ParameterValue, CompilerError> {
        let [param] = params else {
            return Err(Self::invalid_primitive_signature(
                gate,
                "1 parameter",
                params.len(),
            ));
        };
        Ok(param)
    }

    fn three_params(
        gate: StandardGate,
        params: &[ParameterValue],
    ) -> Result<[&ParameterValue; 3], CompilerError> {
        let [theta, phi, lambda] = params else {
            return Err(Self::invalid_primitive_signature(
                gate,
                "3 parameters",
                params.len(),
            ));
        };
        Ok([theta, phi, lambda])
    }

    fn phase_param(
        gate: StandardGate,
        params: &[ParameterValue],
    ) -> Result<Option<&ParameterValue>, CompilerError> {
        match gate {
            StandardGate::Phase => Ok(Some(Self::one_param(gate, params)?)),
            StandardGate::S | StandardGate::SDG | StandardGate::T | StandardGate::TDG
                if params.is_empty() =>
            {
                Ok(None)
            }
            _ => Err(Self::invalid_primitive_signature(
                gate,
                "0 parameters",
                params.len(),
            )),
        }
    }

    fn one_target(gate: StandardGate, targets: &[Qubit]) -> Result<Qubit, CompilerError> {
        let [target] = targets else {
            return Err(Self::invalid_primitive_signature(
                gate,
                "1 target",
                targets.len(),
            ));
        };
        Ok(*target)
    }

    fn two_targets(gate: StandardGate, targets: &[Qubit]) -> Result<[Qubit; 2], CompilerError> {
        let [first, second] = targets else {
            return Err(Self::invalid_primitive_signature(
                gate,
                "2 targets",
                targets.len(),
            ));
        };
        Ok([*first, *second])
    }

    fn invalid_primitive_signature(
        gate: StandardGate,
        expected: &str,
        actual: usize,
    ) -> CompilerError {
        CompilerError::InvariantViolation(format!(
            "validated multi-controlled {gate} operation requires {expected}, got {actual}"
        ))
    }
}

fn value_operations_use_qubit(operations: &[ValueOperation], qubit: Qubit) -> bool {
    operations
        .iter()
        .any(|operation| operation.qubits.contains(&qubit))
}

fn resource_candidate_is_unavailable(error: &ResourceError) -> bool {
    matches!(
        error,
        ResourceError::InsufficientResources { .. }
            | ResourceError::CapacityExceeded { .. }
            | ResourceError::QubitIdOverflow
    )
}

fn resource_input_failed(error: ResourceError) -> CompilerError {
    CompilerError::InvalidInput(format!("cannot initialize ancillary resources: {error}"))
}

fn resource_invariant_failed(error: ResourceError) -> CompilerError {
    CompilerError::InvariantViolation(format!("ancillary-resource bookkeeping failed: {error}"))
}
