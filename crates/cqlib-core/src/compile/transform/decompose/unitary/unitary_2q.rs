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

//! Numeric two-qubit unitary synthesis.
//!
//! This module converts a concrete 4x4 unitary matrix into standard-gate
//! operations over two supplied qubits plus a scalar global phase. The
//! circuit-agnostic `kak_decompose` primitive first factors the matrix into
//! local single-qubit matrices and a Cartan interaction. This layer then emits
//! the factors in the output basis selected by
//! [`TwoQubitUnitaryDecomposeBasis`].
//!
//! Local factors are lowered through the one-qubit synthesizer and emitted as
//! [`StandardGate::U`] operations. Numerically trivial local gates and
//! interaction rotations are omitted using `ANGLE_EPS`, while their scalar
//! phases remain accumulated in the returned phase.
//!
//! The target-aware planner emits only exact candidates. `PauliRotations`
//! emits the Cartan core directly as `RXX`/`RYY`/`RZZ`; `Rzz` emits the same
//! Cartan core with local basis changes for `XX` and `YY`; `Cx`/`Cy`/`Cz`
//! enumerate zero through three entangler templates and keep only candidates
//! whose reconstructed matrix matches the input matrix within exact numerical
//! tolerance.

use super::matrix::{c, dagger, mat2};
use super::two_qubit_kak::{KakDecomposition, kak_decompose};
use super::unitary_1q::{OneQubitUnitaryDecomposition, synthesize_numeric_1q_unitary};
use super::{
    DevicePhysicalCost, DevicePreLayoutEvaluation, DeviceSynthesisPlacement,
    DeviceTwoQubitSynthesisContext,
};
use crate::circuit::gate::gate_matrix::rz_gate;
use crate::circuit::{Instruction, ParameterValue, Qubit, StandardGate, ValueOperation};
use crate::compile::CompilerError;
use crate::compile::transform::target_basis::{
    TargetBasisCost, TargetBasisCostModel, TargetBasisSignature,
};
use ndarray::Array2;
use ndarray::linalg::kron;
use num_complex::Complex64;
use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2, PI};
use std::sync::Arc;

const ANGLE_EPS: f64 = 1e-12;
const TWO_QUBIT_EXACT_TOLERANCE: f64 = 1e-10;

/// Output basis used for two-qubit unitary synthesis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TwoQubitUnitaryDecomposeBasis {
    /// Emit local `U` gates plus `RXX/RYY/RZZ` for the Cartan core.
    PauliRotations,
    /// Emit local `U` gates plus optimized `CX` templates.
    Cx,
    /// Emit local `U` gates plus optimized `CY` templates.
    Cy,
    /// Emit local `U` gates plus optimized `CZ` templates.
    Cz,
    /// Emit local `U` gates plus `RZZ` interactions for the Cartan core.
    Rzz,
}

/// Target capability used by the two-qubit synthesis planner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TwoQubitSynthesisTarget {
    native_2q: Vec<StandardGate>,
    native_1q: Vec<StandardGate>,
    fallback_pauli: bool,
    lowering_cost_model: Option<Arc<TargetBasisCostModel>>,
}

impl TwoQubitSynthesisTarget {
    /// Builds target capability and exact lowering cost model from workflow
    /// target instructions.
    ///
    /// `None` represents no target constraints and enables the neutral exact
    /// Pauli-rotation fallback.
    pub fn from_instructions(target_basis: Option<&[Instruction]>) -> Result<Self, CompilerError> {
        let Some(target_basis) = target_basis else {
            return Ok(Self::unconstrained());
        };

        let mut native_1q = Vec::new();
        let mut native_2q = Vec::new();
        for instruction in target_basis {
            let Instruction::Standard(gate) = instruction else {
                return Err(CompilerError::InvalidInput(format!(
                    "two-qubit synthesis target requires standard instructions, got {instruction:?}"
                )));
            };
            match gate.num_qubits() {
                1 if !native_1q.contains(gate) => native_1q.push(*gate),
                2 if !native_2q.contains(gate) => native_2q.push(*gate),
                _ => {}
            }
        }
        let lowering_cost_model = Arc::new(TargetBasisCostModel::new(target_basis.to_vec())?);
        Ok(Self {
            native_2q,
            native_1q,
            fallback_pauli: true,
            lowering_cost_model: Some(lowering_cost_model),
        })
    }

    /// Builds a target from standard native gates and attaches the active
    /// target-basis lowering model used for exact cost evaluation.
    pub fn from_standard_gates(
        mut native_1q: Vec<StandardGate>,
        mut native_2q: Vec<StandardGate>,
        fallback_pauli: bool,
    ) -> Result<Self, CompilerError> {
        if let Some(gate) = native_1q.iter().find(|gate| gate.num_qubits() != 1) {
            return Err(CompilerError::InvalidInput(format!(
                "one-qubit target capability contains non-1q gate {gate:?}"
            )));
        }
        if let Some(gate) = native_2q.iter().find(|gate| gate.num_qubits() != 2) {
            return Err(CompilerError::InvalidInput(format!(
                "two-qubit target capability contains non-2q gate {gate:?}"
            )));
        }
        native_1q.sort_by_key(|gate| *gate as u8);
        native_1q.dedup();
        native_2q.sort_by_key(|gate| *gate as u8);
        native_2q.dedup();
        let mut gates = native_1q
            .iter()
            .chain(&native_2q)
            .copied()
            .collect::<Vec<_>>();
        gates.sort_by_key(|gate| *gate as u8);
        gates.dedup();
        let instructions = gates
            .into_iter()
            .map(Instruction::Standard)
            .collect::<Vec<_>>();
        let lowering_cost_model = Arc::new(TargetBasisCostModel::new(instructions)?);
        Ok(Self {
            native_2q,
            native_1q,
            fallback_pauli,
            lowering_cost_model: Some(lowering_cost_model),
        })
    }

    pub(crate) fn from_cost_model(lowering_cost_model: Arc<TargetBasisCostModel>) -> Self {
        let mut native_1q = Vec::new();
        let mut native_2q = Vec::new();
        for instruction in lowering_cost_model.target_basis() {
            let Instruction::Standard(gate) = instruction else {
                unreachable!("target-basis cost models contain only standard instructions");
            };
            match gate.num_qubits() {
                1 if !native_1q.contains(gate) => native_1q.push(*gate),
                2 if !native_2q.contains(gate) => native_2q.push(*gate),
                _ => {}
            }
        }
        Self {
            native_2q,
            native_1q,
            fallback_pauli: true,
            lowering_cost_model: Some(lowering_cost_model),
        }
    }

    /// Returns a target with no physical-basis constraints.
    pub const fn unconstrained() -> Self {
        Self {
            native_2q: Vec::new(),
            native_1q: Vec::new(),
            fallback_pauli: true,
            lowering_cost_model: None,
        }
    }

    /// Native two-qubit gates in the configured target basis.
    pub fn native_2q(&self) -> &[StandardGate] {
        &self.native_2q
    }

    /// Native one-qubit gates in the configured target basis.
    pub fn native_1q(&self) -> &[StandardGate] {
        &self.native_1q
    }

    /// Whether Pauli-rotation fallback is permitted.
    pub const fn fallback_pauli(&self) -> bool {
        self.fallback_pauli
    }

    pub(crate) fn lowering_cost_model(&self) -> Option<&TargetBasisCostModel> {
        self.lowering_cost_model.as_deref()
    }

    pub(crate) fn cache_signature(&self) -> Option<TargetBasisSignature> {
        self.lowering_cost_model
            .as_ref()
            .map(|model| model.signature().clone())
    }

    /// Builds a generator-only target for device-aware physical evaluation.
    ///
    /// Device feasibility and cost are evaluated through exact native plans,
    /// so attaching the basis-only lowering model here would incorrectly
    /// reject device-lowerable intermediate gates.
    fn for_device_backends(mut native_2q: Vec<StandardGate>) -> Self {
        native_2q.sort_by_key(|gate| *gate as u8);
        native_2q.dedup();
        Self {
            native_2q,
            native_1q: Vec::new(),
            fallback_pauli: true,
            lowering_cost_model: None,
        }
    }
}

impl Default for TwoQubitSynthesisTarget {
    fn default() -> Self {
        Self::unconstrained()
    }
}

/// Request passed to the target-aware two-qubit synthesis planner.
pub struct TwoQubitSynthesisRequest<'a> {
    pub matrix: &'a Array2<Complex64>,
    pub qubits: [Qubit; 2],
    pub target: TwoQubitSynthesisTarget,
}

/// Target-aware cost used to order exact two-qubit synthesis candidates.
///
/// Costs are ordered lexicographically in the same order as the fields below:
/// minimize final two-qubit count, final depth, final operation count,
/// remaining parameterized operations, and finally a deterministic backend
/// tie-breaker. When a target basis is configured these values are measured
/// after applying the same lowering rules used by final translation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct TargetAwareSynthesisCost {
    /// Two-qubit operations after target-basis lowering.
    pub lowered_two_qubit_ops: usize,
    /// Greedy depth estimate after target-basis lowering.
    pub lowered_depth: usize,
    /// Total operation count after target-basis lowering.
    pub lowered_total_ops: usize,
    /// Operations that still carry numeric parameters.
    pub parameterized_ops: usize,
    /// Stable backend tie-breaker used only when all semantic cost fields tie.
    pub backend_order: usize,
}

/// Exact synthesis candidate emitted by the target-aware planner.
#[derive(Clone, Debug)]
pub struct TwoQubitSynthesisCandidate {
    pub backend: TwoQubitUnitaryDecomposeBasis,
    pub operations: Vec<ValueOperation>,
    pub global_phase: f64,
    pub cost: TargetAwareSynthesisCost,
}

/// Internal device-scored candidate with deterministic direction metadata.
#[derive(Clone, Debug)]
pub(crate) struct DeviceTwoQubitSynthesisCandidate {
    pub(crate) candidate: TwoQubitSynthesisCandidate,
    pub(crate) physical_cost: DevicePhysicalCost,
    pub(crate) pre_layout: Option<DevicePreLayoutEvaluation>,
    direction_order: usize,
}

/// Numeric synthesis result for a two-qubit unitary matrix.
#[derive(Clone, Debug)]
pub struct TwoQubitUnitarySynthesisResult {
    /// Self-contained standard-gate operations implementing the unitary up to
    /// [`global_phase`](Self::global_phase).
    pub operations: Vec<ValueOperation>,
    /// Scalar phase multiplying the emitted operation sequence.
    pub global_phase: f64,
}

/// Synthesizes a finite 4x4 unitary matrix into the selected interaction basis.
///
/// # Errors
///
/// Returns [`CompilerError`] when `matrix` is not a finite 4x4 unitary, the
/// qubits are not distinct, or the numeric decomposition cannot be validated.
pub fn synthesize_numeric_2q_unitary(
    matrix: &Array2<Complex64>,
    qubits: [Qubit; 2],
    basis: TwoQubitUnitaryDecomposeBasis,
) -> Result<TwoQubitUnitarySynthesisResult, CompilerError> {
    let target = target_for_single_backend(basis)?;
    let mut candidates = plan_numeric_2q_unitary(TwoQubitSynthesisRequest {
        matrix,
        qubits,
        target,
    })?;
    let Some(candidate) = candidates
        .drain(..)
        .find(|candidate| candidate.backend == basis)
    else {
        return Err(CompilerError::TransformFailed {
            name: "synthesize.numeric_2q_unitary",
            reason: format!("no exact candidate for {basis:?} backend"),
        });
    };
    Ok(TwoQubitUnitarySynthesisResult {
        operations: candidate.operations,
        global_phase: candidate.global_phase,
    })
}

/// Plans exact two-qubit synthesis candidates for the requested target.
pub fn plan_numeric_2q_unitary(
    request: TwoQubitSynthesisRequest<'_>,
) -> Result<Vec<TwoQubitSynthesisCandidate>, CompilerError> {
    validate_two_qubit_qargs(request.qubits)?;
    let decomp = kak_decompose(request.matrix)?;
    plan_numeric_2q_unitary_from_kak(request, &decomp)
}

/// Plans candidates from a KAK decomposition already validated for the exact
/// request matrix.
pub(crate) fn plan_numeric_2q_unitary_from_kak(
    request: TwoQubitSynthesisRequest<'_>,
    decomp: &KakDecomposition,
) -> Result<Vec<TwoQubitSynthesisCandidate>, CompilerError> {
    validate_two_qubit_qargs(request.qubits)?;
    let mut candidates = Vec::new();
    let needs_cx_family = request
        .target
        .native_2q()
        .iter()
        .any(|gate| matches!(gate, StandardGate::CX | StandardGate::CY | StandardGate::CZ));
    let cx_basis = if needs_cx_family {
        Some(CxBasisData::new()?)
    } else {
        None
    };

    for backend in [
        TwoQubitUnitaryDecomposeBasis::Cx,
        TwoQubitUnitaryDecomposeBasis::Cy,
        TwoQubitUnitaryDecomposeBasis::Cz,
        TwoQubitUnitaryDecomposeBasis::Rzz,
        TwoQubitUnitaryDecomposeBasis::PauliRotations,
    ] {
        if !should_generate_backend(backend, &request.target, candidates.is_empty()) {
            continue;
        }
        generate_backend_candidates(
            &mut candidates,
            backend,
            CandidateGenerationContext {
                matrix: request.matrix,
                decomp,
                qubits: request.qubits,
                target: &request.target,
                cx_basis: cx_basis.as_ref(),
            },
        )?;
    }

    candidates.sort_by(|lhs, rhs| {
        lhs.cost
            .cmp(&rhs.cost)
            .then_with(|| lhs.operations.len().cmp(&rhs.operations.len()))
    });
    Ok(candidates)
}

/// Plans exact two-qubit candidates against one pass-local device context.
pub(crate) fn plan_numeric_2q_unitary_for_device(
    matrix: &Array2<Complex64>,
    qubits: [Qubit; 2],
    context: &DeviceTwoQubitSynthesisContext,
) -> Result<Vec<DeviceTwoQubitSynthesisCandidate>, CompilerError> {
    validate_two_qubit_qargs(qubits)?;
    let decomp = kak_decompose(matrix)?;
    let swap = StandardGate::SWAP
        .matrix(&[])
        .map_err(CompilerError::Circuit)?
        .into_owned();
    let reversed_matrix = swap.dot(matrix).dot(&swap);
    let reversed_decomp = kak_decompose(&reversed_matrix)?;
    plan_numeric_2q_unitary_for_device_from_kak(
        matrix,
        qubits,
        context,
        &decomp,
        &reversed_matrix,
        &reversed_decomp,
    )
}

fn validate_two_qubit_qargs(qubits: [Qubit; 2]) -> Result<(), CompilerError> {
    if qubits[0] == qubits[1] {
        return Err(CompilerError::InvalidInput(format!(
            "2q unitary synthesis requires distinct qubits, both are {}",
            qubits[0]
        )));
    }
    Ok(())
}

/// Device-aware planning from exact forward and reversed KAK decompositions.
pub(crate) fn plan_numeric_2q_unitary_for_device_from_kak(
    matrix: &Array2<Complex64>,
    qubits: [Qubit; 2],
    context: &DeviceTwoQubitSynthesisContext,
    decomp: &KakDecomposition,
    reversed_matrix: &Array2<Complex64>,
    reversed_decomp: &KakDecomposition,
) -> Result<Vec<DeviceTwoQubitSynthesisCandidate>, CompilerError> {
    let mut native_2q = context
        .native_two_qubit_backends(qubits)
        .into_iter()
        .collect::<Vec<_>>();
    native_2q.sort_by_key(|gate| *gate as u8);
    let target = TwoQubitSynthesisTarget::for_device_backends(native_2q);
    let mut oriented = Vec::new();

    for candidate in plan_numeric_2q_unitary_from_kak(
        TwoQubitSynthesisRequest {
            matrix,
            qubits,
            target: target.clone(),
        },
        decomp,
    )? {
        push_device_candidate(&mut oriented, candidate, qubits, 0, context)?;
    }

    let reversed_qubits = [qubits[1], qubits[0]];
    for candidate in plan_numeric_2q_unitary_from_kak(
        TwoQubitSynthesisRequest {
            matrix: reversed_matrix,
            qubits: reversed_qubits,
            target,
        },
        reversed_decomp,
    )? {
        if candidate_matches_matrix(
            &candidate.operations,
            candidate.global_phase,
            matrix,
            qubits,
        )? {
            push_device_candidate(&mut oriented, candidate, qubits, 1, context)?;
        }
    }

    Ok(oriented)
}

/// Selects the best unitary decomposition without comparing unlike placement domains.
pub(crate) fn select_device_unitary_candidate(
    mut candidates: Vec<DeviceTwoQubitSynthesisCandidate>,
    _qubits: [Qubit; 2],
    context: &DeviceTwoQubitSynthesisContext,
) -> Option<TwoQubitSynthesisCandidate> {
    if candidates.is_empty() {
        return None;
    }

    match context.placement() {
        DeviceSynthesisPlacement::ExactPhysical => {
            candidates.sort_by(compare_device_candidates);
        }
        DeviceSynthesisPlacement::PreLayoutEnvelope => {
            let best_coverage = candidates
                .iter()
                .filter_map(|candidate| candidate.pre_layout.as_ref())
                .map(|evaluation| evaluation.coverage)
                .min()?;
            candidates.retain(|candidate| {
                candidate
                    .pre_layout
                    .as_ref()
                    .is_some_and(|evaluation| evaluation.coverage == best_coverage)
            });

            let mut common_domain = candidates
                .first()
                .and_then(|candidate| candidate.pre_layout.as_ref())?
                .domain
                .clone();
            for candidate in candidates.iter().skip(1) {
                let domain = &candidate.pre_layout.as_ref()?.domain;
                common_domain.retain(|pair| domain.contains(pair));
            }

            if common_domain.is_empty() {
                candidates.sort_by(compare_stable_device_candidates);
            } else {
                for candidate in &mut candidates {
                    candidate.physical_cost = candidate
                        .pre_layout
                        .as_ref()?
                        .worst_cost_on_domain(&common_domain)?;
                }
                candidates.sort_by(compare_device_candidates);
            }
        }
    }
    candidates
        .into_iter()
        .next()
        .map(|candidate| candidate.candidate)
}

fn push_device_candidate(
    output: &mut Vec<DeviceTwoQubitSynthesisCandidate>,
    candidate: TwoQubitSynthesisCandidate,
    qubits: [Qubit; 2],
    direction_order: usize,
    context: &DeviceTwoQubitSynthesisContext,
) -> Result<(), CompilerError> {
    match context.placement() {
        DeviceSynthesisPlacement::PreLayoutEnvelope => {
            if let Some(evaluation) = context.evaluate_pre_layout(&candidate.operations, qubits) {
                output.push(DeviceTwoQubitSynthesisCandidate {
                    physical_cost: evaluation.worst_cost,
                    pre_layout: Some(evaluation),
                    candidate,
                    direction_order,
                });
            }
        }
        DeviceSynthesisPlacement::ExactPhysical => {
            match context.exact_cost_diagnostic(&candidate.operations, qubits) {
                Ok(physical_cost) => output.push(DeviceTwoQubitSynthesisCandidate {
                    candidate,
                    physical_cost,
                    pre_layout: None,
                    direction_order,
                }),
                Err(super::DeviceContextCostFailure::Unsupported(_)) => {}
                Err(super::DeviceContextCostFailure::Unprepared(state)) => {
                    return Err(CompilerError::InvariantViolation(format!(
                        "device synthesis context was not prepared for generated state {state:?}"
                    )));
                }
                Err(super::DeviceContextCostFailure::WrongPlacement) => {
                    return Err(CompilerError::InvariantViolation(
                        "exact device candidate was evaluated with a pre-layout context"
                            .to_string(),
                    ));
                }
                Err(super::DeviceContextCostFailure::InvalidOperation(reason)) => {
                    return Err(CompilerError::InvariantViolation(format!(
                        "invalid generated exact-device candidate: {reason}"
                    )));
                }
            }
        }
    }
    Ok(())
}

fn compare_device_candidates(
    left: &DeviceTwoQubitSynthesisCandidate,
    right: &DeviceTwoQubitSynthesisCandidate,
) -> std::cmp::Ordering {
    left.physical_cost
        .compare(right.physical_cost)
        .then_with(|| compare_stable_device_candidates(left, right))
}

fn compare_stable_device_candidates(
    left: &DeviceTwoQubitSynthesisCandidate,
    right: &DeviceTwoQubitSynthesisCandidate,
) -> std::cmp::Ordering {
    backend_order(left.candidate.backend)
        .cmp(&backend_order(right.candidate.backend))
        .then_with(|| left.direction_order.cmp(&right.direction_order))
        .then_with(|| {
            left.candidate
                .operations
                .len()
                .cmp(&right.candidate.operations.len())
        })
}

#[derive(Default)]
struct OperationBuilder {
    operations: Vec<ValueOperation>,
    global_phase: f64,
}

impl OperationBuilder {
    fn push_local_u(
        &mut self,
        qubit: Qubit,
        matrix: &Array2<Complex64>,
    ) -> Result<(), CompilerError> {
        let OneQubitUnitaryDecomposition {
            theta,
            phi,
            lambda,
            global_phase,
        } = synthesize_numeric_1q_unitary(matrix)?;
        self.global_phase += global_phase;
        if theta.abs() <= ANGLE_EPS && phi.abs() <= ANGLE_EPS && lambda.abs() <= ANGLE_EPS {
            return Ok(());
        }

        self.operations.push(ValueOperation::from_standard(
            StandardGate::U,
            [qubit],
            [
                ParameterValue::Fixed(theta),
                ParameterValue::Fixed(phi),
                ParameterValue::Fixed(lambda),
            ],
        ));
        Ok(())
    }

    fn push_rotation(&mut self, gate: StandardGate, first: Qubit, second: Qubit, theta: f64) {
        if theta.abs() <= ANGLE_EPS {
            return;
        }

        self.operations.push(ValueOperation::from_standard(
            gate,
            [first, second],
            [ParameterValue::Fixed(theta)],
        ));
    }

    fn push_1q_gate(&mut self, gate: StandardGate, qubit: Qubit) {
        self.operations
            .push(ValueOperation::from_standard(gate, [qubit], []));
    }

    fn push_1q_rotation(&mut self, gate: StandardGate, qubit: Qubit, theta: f64) {
        if theta.abs() <= ANGLE_EPS {
            return;
        }

        self.operations.push(ValueOperation::from_standard(
            gate,
            [qubit],
            [ParameterValue::Fixed(theta)],
        ));
    }

    fn push_cx(&mut self, control: Qubit, target: Qubit) {
        self.operations.push(ValueOperation::from_standard(
            StandardGate::CX,
            [control, target],
            [],
        ));
    }

    fn push_cy(&mut self, control: Qubit, target: Qubit) {
        self.operations.push(ValueOperation::from_standard(
            StandardGate::CY,
            [control, target],
            [],
        ));
    }

    fn push_cz(&mut self, first: Qubit, second: Qubit) {
        self.operations.push(ValueOperation::from_standard(
            StandardGate::CZ,
            [first, second],
            [],
        ));
    }
}

fn emit_pauli_rotations(
    builder: &mut OperationBuilder,
    decomp: &KakDecomposition,
    first: Qubit,
    second: Qubit,
) -> Result<(), CompilerError> {
    builder.global_phase += decomp.global_phase;
    builder.push_local_u(first, &decomp.k2l)?;
    builder.push_local_u(second, &decomp.k2r)?;
    builder.push_rotation(StandardGate::RXX, first, second, -2.0 * decomp.a);
    builder.push_rotation(StandardGate::RYY, first, second, -2.0 * decomp.b);
    builder.push_rotation(StandardGate::RZZ, first, second, -2.0 * decomp.c);
    builder.push_local_u(first, &decomp.k1l)?;
    builder.push_local_u(second, &decomp.k1r)?;
    Ok(())
}

fn should_generate_backend(
    backend: TwoQubitUnitaryDecomposeBasis,
    target: &TwoQubitSynthesisTarget,
    no_candidates_yet: bool,
) -> bool {
    match backend {
        TwoQubitUnitaryDecomposeBasis::Cx => target.native_2q().contains(&StandardGate::CX),
        TwoQubitUnitaryDecomposeBasis::Cy => target.native_2q().contains(&StandardGate::CY),
        TwoQubitUnitaryDecomposeBasis::Cz => target.native_2q().contains(&StandardGate::CZ),
        TwoQubitUnitaryDecomposeBasis::Rzz => target.native_2q().contains(&StandardGate::RZZ),
        TwoQubitUnitaryDecomposeBasis::PauliRotations => {
            let has_full_pauli = [StandardGate::RXX, StandardGate::RYY, StandardGate::RZZ]
                .iter()
                .all(|gate| target.native_2q().contains(gate));
            has_full_pauli || (target.fallback_pauli() && no_candidates_yet)
        }
    }
}

fn target_for_single_backend(
    basis: TwoQubitUnitaryDecomposeBasis,
) -> Result<TwoQubitSynthesisTarget, CompilerError> {
    let native_2q = match basis {
        TwoQubitUnitaryDecomposeBasis::PauliRotations => {
            vec![StandardGate::RXX, StandardGate::RYY, StandardGate::RZZ]
        }
        TwoQubitUnitaryDecomposeBasis::Cx => vec![StandardGate::CX],
        TwoQubitUnitaryDecomposeBasis::Cy => vec![StandardGate::CY],
        TwoQubitUnitaryDecomposeBasis::Cz => vec![StandardGate::CZ],
        TwoQubitUnitaryDecomposeBasis::Rzz => vec![StandardGate::RZZ],
    };
    let native_1q = match basis {
        TwoQubitUnitaryDecomposeBasis::Rzz => {
            vec![StandardGate::U, StandardGate::H, StandardGate::RX]
        }
        _ => vec![StandardGate::U],
    };
    TwoQubitSynthesisTarget::from_standard_gates(native_1q, native_2q, false)
}

struct CandidateGenerationContext<'a> {
    matrix: &'a Array2<Complex64>,
    decomp: &'a KakDecomposition,
    qubits: [Qubit; 2],
    target: &'a TwoQubitSynthesisTarget,
    cx_basis: Option<&'a CxBasisData>,
}

fn generate_backend_candidates(
    candidates: &mut Vec<TwoQubitSynthesisCandidate>,
    backend: TwoQubitUnitaryDecomposeBasis,
    context: CandidateGenerationContext<'_>,
) -> Result<(), CompilerError> {
    match backend {
        TwoQubitUnitaryDecomposeBasis::PauliRotations => {
            let mut builder = OperationBuilder::default();
            emit_pauli_rotations(
                &mut builder,
                context.decomp,
                context.qubits[0],
                context.qubits[1],
            )?;
            push_validated_candidate(
                candidates,
                backend,
                builder,
                context.matrix,
                context.qubits,
                context.target,
            )?;
        }
        TwoQubitUnitaryDecomposeBasis::Rzz => {
            let mut builder = OperationBuilder::default();
            emit_rzz_only(
                &mut builder,
                context.decomp,
                context.qubits[0],
                context.qubits[1],
            )?;
            push_validated_candidate(
                candidates,
                backend,
                builder,
                context.matrix,
                context.qubits,
                context.target,
            )?;
        }
        TwoQubitUnitaryDecomposeBasis::Cx
        | TwoQubitUnitaryDecomposeBasis::Cy
        | TwoQubitUnitaryDecomposeBasis::Cz => {
            let basis = context.cx_basis.ok_or_else(|| {
                CompilerError::InvariantViolation(
                    "missing shared CX-family basis data for 2q synthesis planner".to_string(),
                )
            })?;
            for entanglers in 0..=3 {
                let mut builder = OperationBuilder::default();
                match backend {
                    TwoQubitUnitaryDecomposeBasis::Cx => emit_cx_with_count(
                        &mut builder,
                        context.decomp,
                        basis,
                        context.qubits[0],
                        context.qubits[1],
                        entanglers,
                    )?,
                    TwoQubitUnitaryDecomposeBasis::Cy => emit_cy_with_count(
                        &mut builder,
                        context.decomp,
                        basis,
                        context.qubits[0],
                        context.qubits[1],
                        entanglers,
                    )?,
                    TwoQubitUnitaryDecomposeBasis::Cz => emit_cz_with_count(
                        &mut builder,
                        context.decomp,
                        basis,
                        context.qubits[0],
                        context.qubits[1],
                        entanglers,
                    )?,
                    _ => unreachable!(),
                }
                push_validated_candidate(
                    candidates,
                    backend,
                    builder,
                    context.matrix,
                    context.qubits,
                    context.target,
                )?;
            }
        }
    }
    Ok(())
}

fn push_validated_candidate(
    candidates: &mut Vec<TwoQubitSynthesisCandidate>,
    backend: TwoQubitUnitaryDecomposeBasis,
    builder: OperationBuilder,
    matrix: &Array2<Complex64>,
    qubits: [Qubit; 2],
    target: &TwoQubitSynthesisTarget,
) -> Result<(), CompilerError> {
    if !candidate_matches_matrix(&builder.operations, builder.global_phase, matrix, qubits)? {
        return Ok(());
    }
    let cost = match target_aware_cost_of_value_operations(&builder.operations, target, backend) {
        Ok(cost) => cost,
        Err(CompilerError::InvalidInput(_)) => {
            // A candidate that cannot be lowered to the configured physical
            // target is not a viable exact synthesis option. Other backends
            // may still be.
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    candidates.push(TwoQubitSynthesisCandidate {
        backend,
        operations: builder.operations,
        global_phase: builder.global_phase,
        cost,
    });
    Ok(())
}

/// Computes target-aware cost for value operations emitted by synthesis.
pub fn target_aware_cost_of_value_operations(
    operations: &[ValueOperation],
    target: &TwoQubitSynthesisTarget,
    backend: TwoQubitUnitaryDecomposeBasis,
) -> Result<TargetAwareSynthesisCost, CompilerError> {
    if let Some(model) = target.lowering_cost_model() {
        let qubits = operation_qubits(operations);
        let TargetBasisCost {
            two_qubit_ops,
            depth,
            total_ops,
            parameterized_ops,
        } = model.cost_of_fixed_operations(qubits, operations.to_vec())?;
        return Ok(TargetAwareSynthesisCost {
            lowered_two_qubit_ops: two_qubit_ops,
            lowered_depth: depth,
            lowered_total_ops: total_ops,
            parameterized_ops,
            backend_order: backend_order(backend),
        });
    }

    let mut cost = TargetAwareSynthesisCost {
        backend_order: backend_order(backend),
        ..TargetAwareSynthesisCost::default()
    };
    let mut depths = std::collections::HashMap::new();
    for operation in operations {
        let gate = match &operation.instruction {
            crate::circuit::ValueInstruction::Instruction(Instruction::Standard(gate)) => {
                Some(*gate)
            }
            _ => None,
        };
        add_target_aware_cost(
            &mut cost,
            &mut depths,
            gate,
            operation.qubits.as_slice(),
            operation.params.len(),
            target,
        );
    }
    Ok(cost)
}

fn add_target_aware_cost(
    cost: &mut TargetAwareSynthesisCost,
    depths: &mut std::collections::HashMap<Qubit, usize>,
    gate: Option<StandardGate>,
    qubits: &[Qubit],
    param_count: usize,
    _target: &TwoQubitSynthesisTarget,
) {
    if gate == Some(StandardGate::GPhase) {
        return;
    }

    cost.lowered_total_ops += 1;
    if qubits.len() == 2 {
        cost.lowered_two_qubit_ops += 1;
    }
    if param_count > 0 {
        cost.parameterized_ops += 1;
    }
    if qubits.is_empty() {
        return;
    }

    let next = qubits
        .iter()
        .filter_map(|qubit| depths.get(qubit))
        .max()
        .copied()
        .unwrap_or(0)
        + 1;
    for &qubit in qubits {
        depths.insert(qubit, next);
    }
    cost.lowered_depth = cost.lowered_depth.max(next);
}

fn operation_qubits(operations: &[ValueOperation]) -> Vec<Qubit> {
    let mut qubits = operations
        .iter()
        .flat_map(|operation| operation.qubits.iter().copied())
        .collect::<Vec<_>>();
    qubits.sort_by_key(|qubit| qubit.index());
    qubits.dedup();
    qubits
}

fn backend_order(backend: TwoQubitUnitaryDecomposeBasis) -> usize {
    // This is only a deterministic tie-breaker after target-aware cost fields
    // tie exactly. It does not override native target capability or 2Q count.
    match backend {
        TwoQubitUnitaryDecomposeBasis::Cx => 0,
        TwoQubitUnitaryDecomposeBasis::Cz => 1,
        TwoQubitUnitaryDecomposeBasis::Cy => 2,
        TwoQubitUnitaryDecomposeBasis::Rzz => 3,
        TwoQubitUnitaryDecomposeBasis::PauliRotations => 4,
    }
}

fn emit_rzz_only(
    builder: &mut OperationBuilder,
    decomp: &KakDecomposition,
    first: Qubit,
    second: Qubit,
) -> Result<(), CompilerError> {
    builder.global_phase += decomp.global_phase;
    builder.push_local_u(first, &decomp.k2l)?;
    builder.push_local_u(second, &decomp.k2r)?;
    emit_rxx_as_rzz(builder, first, second, -2.0 * decomp.a);
    emit_ryy_as_rzz(builder, first, second, -2.0 * decomp.b);
    builder.push_rotation(StandardGate::RZZ, first, second, -2.0 * decomp.c);
    builder.push_local_u(first, &decomp.k1l)?;
    builder.push_local_u(second, &decomp.k1r)?;
    Ok(())
}

fn emit_rxx_as_rzz(builder: &mut OperationBuilder, first: Qubit, second: Qubit, theta: f64) {
    if theta.abs() <= ANGLE_EPS {
        return;
    }

    builder.push_1q_gate(StandardGate::H, first);
    builder.push_1q_gate(StandardGate::H, second);
    builder.push_rotation(StandardGate::RZZ, first, second, theta);
    builder.push_1q_gate(StandardGate::H, second);
    builder.push_1q_gate(StandardGate::H, first);
}

fn emit_ryy_as_rzz(builder: &mut OperationBuilder, first: Qubit, second: Qubit, theta: f64) {
    if theta.abs() <= ANGLE_EPS {
        return;
    }

    builder.push_1q_rotation(StandardGate::RX, first, FRAC_PI_2);
    builder.push_1q_rotation(StandardGate::RX, second, FRAC_PI_2);
    builder.push_rotation(StandardGate::RZZ, first, second, theta);
    builder.push_1q_rotation(StandardGate::RX, second, -FRAC_PI_2);
    builder.push_1q_rotation(StandardGate::RX, first, -FRAC_PI_2);
}

fn emit_cx_with_count(
    builder: &mut OperationBuilder,
    target: &KakDecomposition,
    basis: &CxBasisData,
    first: Qubit,
    second: Qubit,
    num_cx: usize,
) -> Result<(), CompilerError> {
    let locals = basis.local_decomposition(target, num_cx);

    builder.global_phase += target.global_phase - num_cx as f64 * basis.global_phase;
    if num_cx == 2 {
        builder.global_phase += PI;
    }

    for i in 0..num_cx {
        builder.push_local_u(first, &locals[2 * i + 1])?;
        builder.push_local_u(second, &locals[2 * i])?;
        builder.push_cx(first, second);
    }
    builder.push_local_u(first, &locals[2 * num_cx + 1])?;
    builder.push_local_u(second, &locals[2 * num_cx])?;
    Ok(())
}

fn emit_cy_with_count(
    builder: &mut OperationBuilder,
    target: &KakDecomposition,
    basis: &CxBasisData,
    first: Qubit,
    second: Qubit,
    num_cy: usize,
) -> Result<(), CompilerError> {
    let mut locals = basis.local_decomposition(target, num_cy);
    let s = StandardGate::S
        .matrix(&[])
        .map_err(|e| CompilerError::InvalidInput(e.to_string()))?
        .into_owned();
    let sdg = StandardGate::SDG
        .matrix(&[])
        .map_err(|e| CompilerError::InvalidInput(e.to_string()))?
        .into_owned();
    absorb_cx_replacement_locals(&mut locals, num_cy, &s, &sdg);

    builder.global_phase += target.global_phase - num_cy as f64 * basis.global_phase;
    if num_cy == 2 {
        builder.global_phase += PI;
    }

    for i in 0..num_cy {
        builder.push_local_u(first, &locals[2 * i + 1])?;
        builder.push_local_u(second, &locals[2 * i])?;
        builder.push_cy(first, second);
    }
    builder.push_local_u(first, &locals[2 * num_cy + 1])?;
    builder.push_local_u(second, &locals[2 * num_cy])?;
    Ok(())
}

fn emit_cz_with_count(
    builder: &mut OperationBuilder,
    target: &KakDecomposition,
    basis: &CxBasisData,
    first: Qubit,
    second: Qubit,
    num_cz: usize,
) -> Result<(), CompilerError> {
    let mut locals = basis.local_decomposition(target, num_cz);
    let h = StandardGate::H
        .matrix(&[])
        .map_err(|e| CompilerError::InvalidInput(e.to_string()))?
        .into_owned();
    absorb_cx_replacement_locals(&mut locals, num_cz, &h, &h);

    builder.global_phase += target.global_phase - num_cz as f64 * basis.global_phase;
    if num_cz == 2 {
        builder.global_phase += PI;
    }

    for i in 0..num_cz {
        builder.push_local_u(first, &locals[2 * i + 1])?;
        builder.push_local_u(second, &locals[2 * i])?;
        builder.push_cz(first, second);
    }
    builder.push_local_u(first, &locals[2 * num_cz + 1])?;
    builder.push_local_u(second, &locals[2 * num_cz])?;
    Ok(())
}

fn absorb_cx_replacement_locals(
    locals: &mut [Array2<Complex64>],
    entangler_count: usize,
    pre: &Array2<Complex64>,
    post: &Array2<Complex64>,
) {
    if entangler_count == 0 {
        return;
    }

    for local_index in 0..=entangler_count {
        let right_index = 2 * local_index;
        locals[right_index] = match local_index {
            0 => pre.dot(&locals[right_index]),
            index if index == entangler_count => locals[right_index].dot(post),
            _ => pre.dot(&locals[right_index].dot(post)),
        };
    }
}

fn candidate_matches_matrix(
    operations: &[ValueOperation],
    global_phase: f64,
    expected: &Array2<Complex64>,
    qubits: [Qubit; 2],
) -> Result<bool, CompilerError> {
    let actual = value_operations_matrix(operations, global_phase, qubits)?;
    Ok(actual
        .iter()
        .zip(expected.iter())
        .all(|(actual, expected)| (*actual - *expected).norm() <= TWO_QUBIT_EXACT_TOLERANCE))
}

fn value_operations_matrix(
    operations: &[ValueOperation],
    global_phase: f64,
    qubits: [Qubit; 2],
) -> Result<Array2<Complex64>, CompilerError> {
    let mut resolved = Vec::with_capacity(operations.len());
    for operation in operations {
        let crate::circuit::ValueInstruction::Instruction(Instruction::Standard(gate)) =
            &operation.instruction
        else {
            return Err(CompilerError::InvariantViolation(
                "2q synthesis candidate contains non-standard operation".to_string(),
            ));
        };
        let params = operation
            .params
            .iter()
            .map(|param| match param {
                ParameterValue::Fixed(value) => Ok(*value),
                ParameterValue::Param(_) => Err(CompilerError::InvariantViolation(
                    "2q synthesis candidate contains symbolic parameter".to_string(),
                )),
            })
            .collect::<Result<Vec<_>, _>>()?;
        resolved.push(TwoQubitMatrixOp {
            gate: *gate,
            qubits: operation.qubits.iter().copied().collect(),
            params,
        });
    }
    two_qubit_operation_matrix_product(
        &resolved,
        global_phase,
        qubits,
        "2q synthesis candidate references outside qubits",
    )
}

/// Resolved standard-gate operation used to build a two-qubit matrix product.
///
/// The operation may be global, one-qubit, or two-qubit, but all qubits must be
/// contained in the canonical two-qubit frame passed to
/// [`two_qubit_operation_matrix_product`].
#[derive(Clone, Debug)]
pub(crate) struct TwoQubitMatrixOp {
    pub(crate) gate: StandardGate,
    pub(crate) qubits: Vec<Qubit>,
    pub(crate) params: Vec<f64>,
}

/// Builds a 4x4 matrix for resolved operations in a canonical two-qubit frame.
///
/// Source operations are multiplied using the same convention as
/// `circuit_to_matrix`: `gate_n * ... * gate_0`, where `gate_0` is the earliest
/// operation. `qubits[0]` is the first tensor factor; operations applied to the
/// reversed pair are converted by `SWAP * matrix * SWAP`.
pub(crate) fn two_qubit_operation_matrix_product(
    operations: &[TwoQubitMatrixOp],
    global_phase: f64,
    qubits: [Qubit; 2],
    outside_qubits_error: &str,
) -> Result<Array2<Complex64>, CompilerError> {
    let mut result = Array2::<Complex64>::eye(4);
    for operation in operations {
        let matrix = operation
            .gate
            .matrix(&operation.params)
            .map_err(CompilerError::Circuit)?
            .into_owned();
        let identity = Array2::<Complex64>::eye(2);
        let expanded = match operation.qubits.as_slice() {
            [] => matrix,
            [q] if *q == qubits[0] => kron(&matrix.view(), &identity.view()),
            [q] if *q == qubits[1] => kron(&identity.view(), &matrix.view()),
            [a, b] if *a == qubits[0] && *b == qubits[1] => matrix,
            [a, b] if *a == qubits[1] && *b == qubits[0] => {
                let swap = StandardGate::SWAP.matrix(&[]).unwrap().into_owned();
                swap.dot(&matrix).dot(&swap)
            }
            _ => {
                return Err(CompilerError::InvariantViolation(
                    outside_qubits_error.to_string(),
                ));
            }
        };
        result = expanded.dot(&result);
    }
    let phase = Complex64::from_polar(1.0, global_phase);
    Ok(result.mapv(|value| phase * value))
}

struct CxBasisData {
    basis: KakDecomposition,
    u0l: Array2<Complex64>,
    u0r: Array2<Complex64>,
    u1l: Array2<Complex64>,
    u1ra: Array2<Complex64>,
    u1rb: Array2<Complex64>,
    u2la: Array2<Complex64>,
    u2lb: Array2<Complex64>,
    u2ra: Array2<Complex64>,
    u2rb: Array2<Complex64>,
    u3l: Array2<Complex64>,
    u3r: Array2<Complex64>,
    q0l: Array2<Complex64>,
    q0r: Array2<Complex64>,
    q1la: Array2<Complex64>,
    q1lb: Array2<Complex64>,
    q1ra: Array2<Complex64>,
    q1rb: Array2<Complex64>,
    q2l: Array2<Complex64>,
    q2r: Array2<Complex64>,
    global_phase: f64,
}

impl CxBasisData {
    fn new() -> Result<Self, CompilerError> {
        let cx = StandardGate::CX
            .matrix(&[])
            .map_err(|e| CompilerError::InvalidInput(e.to_string()))?;
        let basis = kak_decompose(cx.as_ref())?;
        let b = basis.b;

        // Closed-form local-equivalence templates for realizing a target KAK
        // point with 0, 1, 2, or 3 CX basis gates. The matrices below are the
        // fixed local corrections around the CX basis KAK coordinates; the
        // target-specific angles are injected later in `local_decomposition`.
        let k12r = mat2(
            c(0.0, FRAC_1_SQRT_2),
            c(FRAC_1_SQRT_2, 0.0),
            c(-FRAC_1_SQRT_2, 0.0),
            c(0.0, -FRAC_1_SQRT_2),
        );
        let k12r_dg = dagger(&k12r);
        let k12l = mat2(c(0.5, 0.5), c(0.5, 0.5), c(-0.5, 0.5), c(0.5, -0.5));
        let k12l_dg = dagger(&k12l);
        let k22l = mat2(
            c(FRAC_1_SQRT_2, 0.0),
            c(-FRAC_1_SQRT_2, 0.0),
            c(FRAC_1_SQRT_2, 0.0),
            c(FRAC_1_SQRT_2, 0.0),
        );
        let k22r = mat2(c(0.0, 0.0), c(1.0, 0.0), c(-1.0, 0.0), c(0.0, 0.0));
        let ipz = mat2(c(0.0, 1.0), c(0.0, 0.0), c(0.0, 0.0), c(0.0, -1.0));

        let exp_pos_b = Complex64::from_polar(1.0, b);
        let exp_neg_b = Complex64::from_polar(1.0, -b);
        let exp_pos_2b = Complex64::from_polar(1.0, 2.0 * b);
        let exp_neg_2b = Complex64::from_polar(1.0, -2.0 * b);
        let i = c(0.0, 1.0);
        let minus_i = c(0.0, -1.0);

        let k11l = mat2(
            c(0.5, -0.5) * minus_i * exp_neg_b,
            c(0.5, -0.5) * exp_neg_b,
            c(0.5, -0.5) * minus_i * exp_pos_b,
            c(0.5, -0.5) * -exp_pos_b,
        );
        let k11r = mat2(
            c(FRAC_1_SQRT_2, 0.0) * i * exp_neg_b,
            c(FRAC_1_SQRT_2, 0.0) * -exp_neg_b,
            c(FRAC_1_SQRT_2, 0.0) * exp_pos_b,
            c(FRAC_1_SQRT_2, 0.0) * minus_i * exp_pos_b,
        );
        let k32l_k21l = mat2(
            c(FRAC_1_SQRT_2, 0.0) * c(1.0, (2.0 * b).cos()),
            c(FRAC_1_SQRT_2, 0.0) * i * c((2.0 * b).sin(), 0.0),
            c(FRAC_1_SQRT_2, 0.0) * i * c((2.0 * b).sin(), 0.0),
            c(FRAC_1_SQRT_2, 0.0) * c(1.0, -(2.0 * b).cos()),
        );
        let k21r = mat2(
            c(0.5, 0.5) * minus_i * exp_neg_2b,
            c(0.5, 0.5) * exp_neg_2b,
            c(0.5, 0.5) * i * exp_pos_2b,
            c(0.5, 0.5) * exp_pos_2b,
        );
        let k31l = mat2(
            c(FRAC_1_SQRT_2, 0.0) * exp_neg_b,
            c(FRAC_1_SQRT_2, 0.0) * exp_neg_b,
            c(FRAC_1_SQRT_2, 0.0) * -exp_pos_b,
            c(FRAC_1_SQRT_2, 0.0) * exp_pos_b,
        );
        let k31r = mat2(i * exp_pos_b, c(0.0, 0.0), c(0.0, 0.0), minus_i * exp_neg_b);
        let k32r = mat2(
            c(0.5, 0.5) * exp_pos_b,
            c(0.5, 0.5) * -exp_neg_b,
            c(0.5, 0.5) * minus_i * exp_pos_b,
            c(0.5, 0.5) * minus_i * exp_neg_b,
        );

        let k1ld = dagger(&basis.k1l);
        let k1rd = dagger(&basis.k1r);
        let k2ld = dagger(&basis.k2l);
        let k2rd = dagger(&basis.k2r);

        let u0l = k31l.dot(&k1ld);
        let u0r = k31r.dot(&k1rd);
        let u1l = k2ld.dot(&k32l_k21l.dot(&k1ld));
        let u1ra = k2rd.dot(&k32r);
        let u1rb = k21r.dot(&k1rd);
        let u2la = k2ld.dot(&k22l);
        let u2lb = k11l.dot(&k1ld);
        let u2ra = k2rd.dot(&k22r);
        let u2rb = k11r.dot(&k1rd);
        let u3l = k2ld.dot(&k12l);
        let u3r = k2rd.dot(&k12r);
        let q0l = k12l_dg.dot(&k1ld);
        let q0r = k12r_dg.dot(&ipz.dot(&k1rd));
        let q1la = k2ld.dot(&dagger(&k11l));
        let q1lb = k11l.dot(&k1ld);
        let q1ra = k2rd.dot(&ipz.dot(&dagger(&k11r)));
        let q1rb = k11r.dot(&k1rd);
        let q2l = k2ld.dot(&k12l);
        let q2r = k2rd.dot(&k12r);
        let global_phase = basis.global_phase;

        Ok(Self {
            basis,
            u0l,
            u0r,
            u1l,
            u1ra,
            u1rb,
            u2la,
            u2lb,
            u2ra,
            u2rb,
            u3l,
            u3r,
            q0l,
            q0r,
            q1la,
            q1lb,
            q1ra,
            q1rb,
            q2l,
            q2r,
            global_phase,
        })
    }

    fn local_decomposition(
        &self,
        target: &KakDecomposition,
        num_cx: usize,
    ) -> Vec<Array2<Complex64>> {
        match num_cx {
            0 => vec![target.k1r.dot(&target.k2r), target.k1l.dot(&target.k2l)],
            1 => vec![
                dagger(&self.basis.k2r).dot(&target.k2r),
                dagger(&self.basis.k2l).dot(&target.k2l),
                target.k1r.dot(&dagger(&self.basis.k1r)),
                target.k1l.dot(&dagger(&self.basis.k1l)),
            ],
            2 => vec![
                self.q2r.dot(&target.k2r),
                self.q2l.dot(&target.k2l),
                self.q1ra.dot(&rz_gate(2.0 * target.b).dot(&self.q1rb)),
                self.q1la.dot(&rz_gate(-2.0 * target.a).dot(&self.q1lb)),
                target.k1r.dot(&self.q0r),
                target.k1l.dot(&self.q0l),
            ],
            3 => vec![
                self.u3r.dot(&target.k2r),
                self.u3l.dot(&target.k2l),
                self.u2ra.dot(&rz_gate(2.0 * target.b).dot(&self.u2rb)),
                self.u2la.dot(&rz_gate(-2.0 * target.a).dot(&self.u2lb)),
                self.u1ra.dot(&rz_gate(-2.0 * target.c).dot(&self.u1rb)),
                self.u1l.clone(),
                target.k1r.dot(&self.u0r),
                target.k1l.dot(&self.u0l),
            ],
            _ => unreachable!("CX decomposer supports at most 3 basis gates"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::{Circuit, Instruction, Parameter, UnitaryGate, circuit_to_matrix};
    use crate::compile::transform::{TargetBasisLowerer, TransformerTestExt};
    use approx::assert_abs_diff_eq;
    use ndarray::linalg::kron;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    fn synthesized_output(
        matrix: &Array2<Complex64>,
        basis: TwoQubitUnitaryDecomposeBasis,
    ) -> (Circuit, Array2<Complex64>, Array2<Complex64>) {
        let gate = UnitaryGate::new("source_2q", 2, 0)
            .with_matrix(matrix.clone())
            .unwrap();
        let mut source = Circuit::new(2);
        source
            .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
            .unwrap();
        let expected = circuit_to_matrix(&source, None).unwrap();

        let synthesis =
            synthesize_numeric_2q_unitary(matrix, [Qubit::new(0), Qubit::new(1)], basis).unwrap();
        let mut circuit = Circuit::from_operations(
            vec![Qubit::new(0), Qubit::new(1)],
            synthesis.operations,
            None,
            None,
        )
        .unwrap();
        circuit.set_global_phase(Parameter::from(synthesis.global_phase));
        let matrix = if circuit.operations().is_empty() {
            let phase = Complex64::from_polar(1.0, synthesis.global_phase);
            Array2::eye(4).mapv(|value: Complex64| phase * value)
        } else {
            circuit_to_matrix(&circuit, None).unwrap()
        };
        (circuit, expected, matrix)
    }

    fn count_gate(circuit: &Circuit, gate: StandardGate) -> usize {
        circuit
            .operations()
            .iter()
            .filter(|operation| matches!(operation.instruction, Instruction::Standard(actual) if actual == gate))
            .count()
    }

    fn seeded_random_unitary_4(rng: &mut StdRng) -> Array2<Complex64> {
        let mut columns = Vec::<[Complex64; 4]>::with_capacity(4);
        for _ in 0..4 {
            let mut column = random_complex_column(rng);
            for previous in &columns {
                let projection = column_inner(previous, &column);
                for row in 0..4 {
                    column[row] -= projection * previous[row];
                }
            }
            let norm = column
                .iter()
                .map(|value| value.norm_sqr())
                .sum::<f64>()
                .sqrt();
            assert!(norm > 1e-10, "seeded random basis column is degenerate");
            for value in &mut column {
                *value /= norm;
            }
            columns.push(column);
        }

        Array2::from_shape_fn((4, 4), |(row, col)| columns[col][row])
    }

    fn random_complex_column(rng: &mut StdRng) -> [Complex64; 4] {
        std::array::from_fn(|_| {
            Complex64::new(rng.random_range(-1.0..1.0), rng.random_range(-1.0..1.0))
        })
    }

    fn column_inner(lhs: &[Complex64; 4], rhs: &[Complex64; 4]) -> Complex64 {
        lhs.iter()
            .zip(rhs)
            .map(|(left, right)| left.conj() * right)
            .sum()
    }

    fn target(native_2q: Vec<StandardGate>) -> TwoQubitSynthesisTarget {
        TwoQubitSynthesisTarget::from_standard_gates(
            vec![StandardGate::U, StandardGate::H, StandardGate::RX],
            native_2q,
            true,
        )
        .unwrap()
    }

    #[test]
    fn synthesis_target_reuses_supplied_cost_model() {
        let cost_model = Arc::new(
            TargetBasisCostModel::new(vec![
                Instruction::Standard(StandardGate::U),
                Instruction::Standard(StandardGate::CX),
            ])
            .unwrap(),
        );

        let target = TwoQubitSynthesisTarget::from_cost_model(Arc::clone(&cost_model));

        assert!(Arc::ptr_eq(
            target.lowering_cost_model.as_ref().unwrap(),
            &cost_model
        ));
        assert_eq!(target.native_1q(), &[StandardGate::U]);
        assert_eq!(target.native_2q(), &[StandardGate::CX]);
    }

    #[test]
    fn planner_filters_candidates_to_native_two_qubit_family() {
        let matrix = StandardGate::SWAP.matrix(&[]).unwrap().into_owned();
        let candidates = plan_numeric_2q_unitary(TwoQubitSynthesisRequest {
            matrix: &matrix,
            qubits: [Qubit::new(0), Qubit::new(1)],
            target: target(vec![StandardGate::CX]),
        })
        .unwrap();

        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|candidate| {
            candidate.backend == TwoQubitUnitaryDecomposeBasis::Cx
                && candidate.operations.iter().all(|operation| {
                    matches!(
                        operation.instruction,
                        crate::circuit::ValueInstruction::Instruction(Instruction::Standard(
                            StandardGate::U
                        )) | crate::circuit::ValueInstruction::Instruction(Instruction::Standard(
                            StandardGate::CX
                        ))
                    )
                })
        }));
    }

    #[test]
    fn target_capability_rejects_gates_with_wrong_arity() {
        let one_qubit_error =
            TwoQubitSynthesisTarget::from_standard_gates(vec![StandardGate::CX], vec![], true)
                .unwrap_err();
        assert!(one_qubit_error.to_string().contains("non-1q gate"));

        let two_qubit_error = TwoQubitSynthesisTarget::from_standard_gates(
            vec![StandardGate::U],
            vec![StandardGate::H],
            true,
        )
        .unwrap_err();
        assert!(two_qubit_error.to_string().contains("non-2q gate"));
    }

    #[test]
    fn planner_selection_is_independent_of_target_basis_order() {
        let matrix = StandardGate::SWAP.matrix(&[]).unwrap().into_owned();
        let cx_first = plan_numeric_2q_unitary(TwoQubitSynthesisRequest {
            matrix: &matrix,
            qubits: [Qubit::new(0), Qubit::new(1)],
            target: target(vec![StandardGate::CX, StandardGate::CZ]),
        })
        .unwrap();
        let cz_first = plan_numeric_2q_unitary(TwoQubitSynthesisRequest {
            matrix: &matrix,
            qubits: [Qubit::new(0), Qubit::new(1)],
            target: target(vec![StandardGate::CZ, StandardGate::CX]),
        })
        .unwrap();

        assert_eq!(cx_first[0].backend, cz_first[0].backend);
        assert_eq!(cx_first[0].cost, cz_first[0].cost);
    }

    #[test]
    fn planner_costs_match_final_target_lowering_costs() {
        let q0 = Qubit::new(0);
        let q1 = Qubit::new(1);
        let native_1q = vec![StandardGate::U, StandardGate::H];
        let native_2q = vec![StandardGate::CX, StandardGate::CZ];
        let target = TwoQubitSynthesisTarget::from_standard_gates(
            native_1q.clone(),
            native_2q.clone(),
            true,
        )
        .unwrap();
        let matrix = StandardGate::SWAP.matrix(&[]).unwrap().into_owned();
        let candidates = plan_numeric_2q_unitary(TwoQubitSynthesisRequest {
            matrix: &matrix,
            qubits: [q0, q1],
            target,
        })
        .unwrap();
        let instructions = native_1q
            .into_iter()
            .chain(native_2q)
            .map(Instruction::Standard)
            .collect::<Vec<_>>();
        let cost_model = TargetBasisCostModel::new(instructions).unwrap();

        assert!(!candidates.is_empty());
        for candidate in &candidates {
            let lowered = cost_model
                .cost_of_fixed_operations(vec![q0, q1], candidate.operations.clone())
                .unwrap();
            assert_eq!(candidate.cost.lowered_two_qubit_ops, lowered.two_qubit_ops);
            assert_eq!(candidate.cost.lowered_depth, lowered.depth);
            assert_eq!(candidate.cost.lowered_total_ops, lowered.total_ops);
            assert_eq!(candidate.cost.parameterized_ops, lowered.parameterized_ops);
        }
        assert!(
            candidates
                .windows(2)
                .all(|pair| pair[0].cost <= pair[1].cost)
        );
    }

    #[test]
    fn planner_uses_rzz_backend_for_rzz_only_target() {
        let matrix = StandardGate::SWAP.matrix(&[]).unwrap().into_owned();
        let candidates = plan_numeric_2q_unitary(TwoQubitSynthesisRequest {
            matrix: &matrix,
            qubits: [Qubit::new(0), Qubit::new(1)],
            target: target(vec![StandardGate::RZZ]),
        })
        .unwrap();

        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|candidate| {
            candidate.backend == TwoQubitUnitaryDecomposeBasis::Rzz
                && candidate.operations.iter().all(|operation| {
                    !matches!(
                        operation.instruction,
                        crate::circuit::ValueInstruction::Instruction(Instruction::Standard(
                            StandardGate::CX
                        )) | crate::circuit::ValueInstruction::Instruction(Instruction::Standard(
                            StandardGate::CY
                        )) | crate::circuit::ValueInstruction::Instruction(Instruction::Standard(
                            StandardGate::CZ
                        )) | crate::circuit::ValueInstruction::Instruction(Instruction::Standard(
                            StandardGate::RXX
                        )) | crate::circuit::ValueInstruction::Instruction(Instruction::Standard(
                            StandardGate::RYY
                        ))
                    )
                })
        }));
    }

    #[test]
    fn pauli_fallback_lowers_to_a_target_without_a_direct_planner_backend() {
        let q0 = Qubit::new(0);
        let q1 = Qubit::new(1);
        let native_1q = vec![StandardGate::U, StandardGate::H, StandardGate::RX];
        let native_2q = vec![StandardGate::RXX];
        let target = TwoQubitSynthesisTarget::from_standard_gates(
            native_1q.clone(),
            native_2q.clone(),
            true,
        )
        .unwrap();
        let matrix = StandardGate::SWAP.matrix(&[]).unwrap().into_owned();
        let candidates = plan_numeric_2q_unitary(TwoQubitSynthesisRequest {
            matrix: &matrix,
            qubits: [q0, q1],
            target,
        })
        .unwrap();

        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|candidate| {
            candidate.backend == TwoQubitUnitaryDecomposeBasis::PauliRotations
        }));
        let candidate = &candidates[0];
        let mut synthesized =
            Circuit::from_operations(vec![q0, q1], candidate.operations.clone(), None, None)
                .unwrap();
        synthesized.set_global_phase(Parameter::from(candidate.global_phase));
        let target_basis = native_1q
            .iter()
            .chain(&native_2q)
            .copied()
            .map(Instruction::Standard)
            .collect::<Vec<_>>();
        let lowered = TargetBasisLowerer::new(target_basis)
            .unwrap()
            .transform_resolved(&synthesized, None)
            .unwrap()
            .circuit;

        assert!(lowered.operations().iter().all(|operation| matches!(
            operation.instruction,
            Instruction::Standard(
                StandardGate::U | StandardGate::H | StandardGate::RX | StandardGate::RXX
            )
        )));
        assert_abs_diff_eq!(
            matrix,
            circuit_to_matrix(&lowered, None).unwrap(),
            epsilon = 1e-8
        );
    }

    #[test]
    fn pauli_backend_reconstructs_common_2q_unitaries() {
        let phase = Complex64::from_polar(1.0, 0.37);
        let cases = [
            StandardGate::CX.matrix(&[]).unwrap().into_owned(),
            StandardGate::CZ.matrix(&[]).unwrap().into_owned(),
            StandardGate::SWAP.matrix(&[]).unwrap().into_owned(),
            StandardGate::FSIM
                .matrix(&[0.2, -0.3])
                .unwrap()
                .into_owned(),
            StandardGate::CX
                .matrix(&[])
                .unwrap()
                .into_owned()
                .mapv(|value| phase * value),
        ];

        for matrix in cases {
            let (decomposed, before, after) =
                synthesized_output(&matrix, TwoQubitUnitaryDecomposeBasis::PauliRotations);

            assert!(decomposed.operations().iter().all(|operation| matches!(
                operation.instruction,
                Instruction::Standard(StandardGate::U)
                    | Instruction::Standard(StandardGate::RXX)
                    | Instruction::Standard(StandardGate::RYY)
                    | Instruction::Standard(StandardGate::RZZ)
            )));
            assert_abs_diff_eq!(before, after, epsilon = 1e-8);
        }
    }

    #[test]
    fn cx_backend_uses_expected_exact_cx_counts() {
        let rxx = StandardGate::RXX.matrix(&[0.7]).unwrap().into_owned();
        let ryy = StandardGate::RYY.matrix(&[-0.4]).unwrap().into_owned();
        let two_cx_matrix = rxx.dot(&ryy);
        let cases = [
            (Array2::eye(4), 0usize),
            (StandardGate::CX.matrix(&[]).unwrap().into_owned(), 1usize),
            (two_cx_matrix, 2usize),
            (StandardGate::SWAP.matrix(&[]).unwrap().into_owned(), 3usize),
        ];

        for (matrix, expected_cx) in cases {
            let (decomposed, before, after) =
                synthesized_output(&matrix, TwoQubitUnitaryDecomposeBasis::Cx);

            assert_eq!(count_gate(&decomposed, StandardGate::CX), expected_cx);
            assert!(decomposed.operations().iter().all(|operation| matches!(
                operation.instruction,
                Instruction::Standard(StandardGate::U) | Instruction::Standard(StandardGate::CX)
            )));
            assert_abs_diff_eq!(before, after, epsilon = 1e-8);
        }
    }

    #[test]
    fn cy_backend_uses_expected_exact_cy_counts() {
        let rxx = StandardGate::RXX.matrix(&[0.7]).unwrap().into_owned();
        let ryy = StandardGate::RYY.matrix(&[-0.4]).unwrap().into_owned();
        let two_cy_matrix = rxx.dot(&ryy);
        let cases = [
            (Array2::eye(4), 0usize),
            (StandardGate::CY.matrix(&[]).unwrap().into_owned(), 1usize),
            (two_cy_matrix, 2usize),
            (StandardGate::SWAP.matrix(&[]).unwrap().into_owned(), 3usize),
        ];

        for (matrix, expected_cy) in cases {
            let (decomposed, before, after) =
                synthesized_output(&matrix, TwoQubitUnitaryDecomposeBasis::Cy);

            assert_eq!(count_gate(&decomposed, StandardGate::CY), expected_cy);
            assert_eq!(count_gate(&decomposed, StandardGate::CX), 0);
            assert_eq!(count_gate(&decomposed, StandardGate::CZ), 0);
            assert!(decomposed.operations().iter().all(|operation| matches!(
                operation.instruction,
                Instruction::Standard(StandardGate::U) | Instruction::Standard(StandardGate::CY)
            )));
            assert_abs_diff_eq!(before, after, epsilon = 1e-8);
        }
    }

    #[test]
    fn cz_backend_uses_expected_exact_cz_counts() {
        let rxx = StandardGate::RXX.matrix(&[0.7]).unwrap().into_owned();
        let ryy = StandardGate::RYY.matrix(&[-0.4]).unwrap().into_owned();
        let two_cz_matrix = rxx.dot(&ryy);
        let cases = [
            (Array2::eye(4), 0usize),
            (StandardGate::CZ.matrix(&[]).unwrap().into_owned(), 1usize),
            (two_cz_matrix, 2usize),
            (StandardGate::SWAP.matrix(&[]).unwrap().into_owned(), 3usize),
        ];

        for (matrix, expected_cz) in cases {
            let (decomposed, before, after) =
                synthesized_output(&matrix, TwoQubitUnitaryDecomposeBasis::Cz);

            assert_eq!(count_gate(&decomposed, StandardGate::CZ), expected_cz);
            assert_eq!(count_gate(&decomposed, StandardGate::CX), 0);
            assert!(decomposed.operations().iter().all(|operation| matches!(
                operation.instruction,
                Instruction::Standard(StandardGate::U) | Instruction::Standard(StandardGate::CZ)
            )));
            assert_abs_diff_eq!(before, after, epsilon = 1e-8);
        }
    }

    #[test]
    fn rzz_backend_uses_rzz_only_for_cartan_core() {
        let rxx = StandardGate::RXX.matrix(&[0.7]).unwrap().into_owned();
        let ryy = StandardGate::RYY.matrix(&[-0.4]).unwrap().into_owned();
        let cases = [
            (Array2::eye(4), 0usize),
            (
                StandardGate::RZZ.matrix(&[0.5]).unwrap().into_owned(),
                1usize,
            ),
            (rxx.dot(&ryy), 2usize),
            (StandardGate::SWAP.matrix(&[]).unwrap().into_owned(), 3usize),
        ];

        for (matrix, expected_rzz) in cases {
            let (decomposed, before, after) =
                synthesized_output(&matrix, TwoQubitUnitaryDecomposeBasis::Rzz);

            assert_eq!(count_gate(&decomposed, StandardGate::RZZ), expected_rzz);
            assert_eq!(count_gate(&decomposed, StandardGate::CX), 0);
            assert_eq!(count_gate(&decomposed, StandardGate::CY), 0);
            assert_eq!(count_gate(&decomposed, StandardGate::CZ), 0);
            assert_eq!(count_gate(&decomposed, StandardGate::RXX), 0);
            assert_eq!(count_gate(&decomposed, StandardGate::RYY), 0);
            assert!(decomposed.operations().iter().all(|operation| matches!(
                operation.instruction,
                Instruction::Standard(StandardGate::U)
                    | Instruction::Standard(StandardGate::H)
                    | Instruction::Standard(StandardGate::RX)
                    | Instruction::Standard(StandardGate::RZZ)
            )));
            assert_abs_diff_eq!(before, after, epsilon = 1e-8);
        }
    }

    #[test]
    fn backends_handle_identity_without_entangling_operations() {
        for basis in [
            TwoQubitUnitaryDecomposeBasis::PauliRotations,
            TwoQubitUnitaryDecomposeBasis::Cx,
            TwoQubitUnitaryDecomposeBasis::Cy,
            TwoQubitUnitaryDecomposeBasis::Cz,
            TwoQubitUnitaryDecomposeBasis::Rzz,
        ] {
            let (decomposed, before, after) = synthesized_output(&Array2::eye(4), basis);

            assert_eq!(count_gate(&decomposed, StandardGate::CX), 0);
            assert_eq!(count_gate(&decomposed, StandardGate::CY), 0);
            assert_eq!(count_gate(&decomposed, StandardGate::CZ), 0);
            assert_eq!(count_gate(&decomposed, StandardGate::RXX), 0);
            assert_eq!(count_gate(&decomposed, StandardGate::RYY), 0);
            assert_eq!(count_gate(&decomposed, StandardGate::RZZ), 0);
            assert_abs_diff_eq!(before, after, epsilon = 1e-8);
        }
    }

    #[test]
    fn cx_backend_preserves_near_zero_entangling_rotation() {
        let matrix = StandardGate::RXX.matrix(&[-1.0e-5]).unwrap().into_owned();
        let (decomposed, before, after) =
            synthesized_output(&matrix, TwoQubitUnitaryDecomposeBasis::Cx);

        assert_abs_diff_eq!(before, after, epsilon = 1e-8);
        assert_eq!(count_gate(&decomposed, StandardGate::CX), 2);
    }

    #[test]
    fn backends_preserve_asymmetric_local_product_without_entanglers() {
        let left = StandardGate::U.matrix(&[0.3, -0.4, 0.5]).unwrap();
        let right = StandardGate::U.matrix(&[0.7, 0.2, -0.6]).unwrap();
        let matrix = kron(left.as_ref(), right.as_ref());

        for basis in [
            TwoQubitUnitaryDecomposeBasis::PauliRotations,
            TwoQubitUnitaryDecomposeBasis::Cx,
            TwoQubitUnitaryDecomposeBasis::Cy,
            TwoQubitUnitaryDecomposeBasis::Cz,
            TwoQubitUnitaryDecomposeBasis::Rzz,
        ] {
            let (decomposed, before, after) = synthesized_output(&matrix, basis);

            assert_eq!(count_gate(&decomposed, StandardGate::CX), 0);
            assert_eq!(count_gate(&decomposed, StandardGate::CY), 0);
            assert_eq!(count_gate(&decomposed, StandardGate::CZ), 0);
            assert_eq!(count_gate(&decomposed, StandardGate::RXX), 0);
            assert_eq!(count_gate(&decomposed, StandardGate::RYY), 0);
            assert_eq!(count_gate(&decomposed, StandardGate::RZZ), 0);
            assert_abs_diff_eq!(before, after, epsilon = 1e-8);
        }
    }

    #[test]
    fn backends_preserve_asymmetric_locals_around_cartan_core() {
        let k1l = StandardGate::U.matrix(&[0.2, -0.4, 0.9]).unwrap();
        let k1r = StandardGate::U.matrix(&[1.0, 0.8, -0.7]).unwrap();
        let k2l = StandardGate::U.matrix(&[0.7, -0.5, 0.1]).unwrap();
        let k2r = StandardGate::U.matrix(&[0.3, 0.6, -0.2]).unwrap();
        let rxx = StandardGate::RXX.matrix(&[-0.62]).unwrap();
        let ryy = StandardGate::RYY.matrix(&[-0.34]).unwrap();
        let rzz = StandardGate::RZZ.matrix(&[0.16]).unwrap();
        let matrix = kron(k1l.as_ref(), k1r.as_ref())
            .dot(&rxx.dot(&ryy.dot(&rzz.dot(&kron(k2l.as_ref(), k2r.as_ref())))));

        for basis in [
            TwoQubitUnitaryDecomposeBasis::PauliRotations,
            TwoQubitUnitaryDecomposeBasis::Cx,
            TwoQubitUnitaryDecomposeBasis::Cy,
            TwoQubitUnitaryDecomposeBasis::Cz,
            TwoQubitUnitaryDecomposeBasis::Rzz,
        ] {
            let (_, before, after) = synthesized_output(&matrix, basis);
            assert_abs_diff_eq!(before, after, epsilon = 1e-8);
        }
    }

    #[test]
    fn backends_reconstruct_controlled_and_cartan_rotation_family() {
        let phase = Complex64::from_polar(1.0, -0.28);
        let cases = [
            StandardGate::CRX.matrix(&[0.31]).unwrap().into_owned(),
            StandardGate::CRY.matrix(&[-0.47]).unwrap().into_owned(),
            StandardGate::CRZ.matrix(&[0.83]).unwrap().into_owned(),
            StandardGate::RXX.matrix(&[0.19]).unwrap().into_owned(),
            StandardGate::RYY.matrix(&[-0.23]).unwrap().into_owned(),
            StandardGate::RZZ
                .matrix(&[0.41])
                .unwrap()
                .into_owned()
                .mapv(|value| phase * value),
        ];

        for matrix in cases {
            for basis in [
                TwoQubitUnitaryDecomposeBasis::PauliRotations,
                TwoQubitUnitaryDecomposeBasis::Cx,
                TwoQubitUnitaryDecomposeBasis::Cy,
                TwoQubitUnitaryDecomposeBasis::Cz,
                TwoQubitUnitaryDecomposeBasis::Rzz,
            ] {
                let (_, before, after) = synthesized_output(&matrix, basis);
                assert_abs_diff_eq!(before, after, epsilon = 1e-8);
            }
        }
    }

    #[test]
    fn backends_reconstruct_seeded_random_2q_unitaries() {
        let mut rng = StdRng::seed_from_u64(0xC0FFEE);
        for _ in 0..50 {
            let matrix = seeded_random_unitary_4(&mut rng);
            for basis in [
                TwoQubitUnitaryDecomposeBasis::PauliRotations,
                TwoQubitUnitaryDecomposeBasis::Cx,
                TwoQubitUnitaryDecomposeBasis::Cy,
                TwoQubitUnitaryDecomposeBasis::Cz,
                TwoQubitUnitaryDecomposeBasis::Rzz,
            ] {
                let (decomposed, before, after) = synthesized_output(&matrix, basis);

                assert_abs_diff_eq!(before, after, epsilon = 1e-8);
                assert!(count_gate(&decomposed, StandardGate::CX) <= 3);
                assert!(count_gate(&decomposed, StandardGate::CY) <= 3);
                assert!(count_gate(&decomposed, StandardGate::CZ) <= 3);
                if basis == TwoQubitUnitaryDecomposeBasis::Rzz {
                    assert!(count_gate(&decomposed, StandardGate::RZZ) <= 3);
                }
            }
        }
    }

    #[test]
    fn rejects_invalid_shape_and_non_unitary_matrix() {
        let bad_shape = Array2::<Complex64>::eye(3);
        let err = synthesize_numeric_2q_unitary(
            &bad_shape,
            [Qubit::new(0), Qubit::new(1)],
            TwoQubitUnitaryDecomposeBasis::PauliRotations,
        )
        .unwrap_err();
        assert!(err.to_string().contains("4x4"));

        let non_unitary = ndarray::array![
            [
                Complex64::new(1.0, 0.0),
                Complex64::new(0.0, 0.0),
                Complex64::new(0.0, 0.0),
                Complex64::new(0.0, 0.0)
            ],
            [
                Complex64::new(0.0, 0.0),
                Complex64::new(2.0, 0.0),
                Complex64::new(0.0, 0.0),
                Complex64::new(0.0, 0.0)
            ],
            [
                Complex64::new(0.0, 0.0),
                Complex64::new(0.0, 0.0),
                Complex64::new(1.0, 0.0),
                Complex64::new(0.0, 0.0)
            ],
            [
                Complex64::new(0.0, 0.0),
                Complex64::new(0.0, 0.0),
                Complex64::new(0.0, 0.0),
                Complex64::new(1.0, 0.0)
            ]
        ];
        let err = synthesize_numeric_2q_unitary(
            &non_unitary,
            [Qubit::new(0), Qubit::new(1)],
            TwoQubitUnitaryDecomposeBasis::Cx,
        )
        .unwrap_err();
        assert!(err.to_string().contains("not unitary"));

        let mut non_finite = Array2::<Complex64>::eye(4);
        non_finite[[1, 2]] = Complex64::new(f64::INFINITY, 0.0);
        let err = synthesize_numeric_2q_unitary(
            &non_finite,
            [Qubit::new(0), Qubit::new(1)],
            TwoQubitUnitaryDecomposeBasis::PauliRotations,
        )
        .unwrap_err();
        assert!(err.to_string().contains("non-finite"));

        let err = synthesize_numeric_2q_unitary(
            &Array2::eye(4),
            [Qubit::new(0), Qubit::new(0)],
            TwoQubitUnitaryDecomposeBasis::PauliRotations,
        )
        .unwrap_err();
        assert!(err.to_string().contains("distinct qubits"));
    }
}
