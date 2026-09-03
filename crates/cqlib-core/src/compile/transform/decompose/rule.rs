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

//! Runtime decomposition rules.
//!
//! This module provides pass-local templates for algorithmic decomposition
//! results. These rules are generated from synthesized value operations and
//! replayed by role, not by concrete qubit id. They deliberately stay separate
//! from the static knowledge-rule library because decomposition templates may
//! depend on resource signatures such as clean or dirty ancillary usage.

use crate::circuit::{
    Instruction, Parameter, ParameterValue, Qubit, StandardGate, ValueInstruction, ValueOperation,
};
use crate::compile::CompilerError;
use crate::compile::transform::decompose::unitary::TwoQubitSynthesisTarget;
use crate::compile::transform::target_basis::TargetBasisSignature;
use ndarray::Array2;
use num_complex::Complex64;
use smallvec::SmallVec;
use std::collections::HashMap;

/// Pass-local cache for runtime decomposition rules.
#[derive(Debug, Clone, Default)]
pub struct DecompositionRuleCache {
    mc_gate_rules: HashMap<McGateRuleKey, DecompositionRule>,
    numeric_unitary_rules: HashMap<NumericUnitaryRuleKey, NumericUnitaryRule>,
    stats: DecompositionRuleStats,
}

/// Basic cache accounting used by tests and diagnostics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DecompositionRuleStats {
    pub hits: usize,
    pub misses: usize,
    pub inserts: usize,
}

impl DecompositionRuleCache {
    /// Instantiates a cached multi-controlled-gate decomposition rule.
    ///
    /// Returns `Ok(None)` when the cache has no matching rule. The supplied
    /// qubit slices provide the concrete role bindings for a cached template.
    pub fn instantiate_mc_gate(
        &mut self,
        request: McGateRuleRequest<'_>,
        controls: &[Qubit],
        targets: &[Qubit],
        ancillas: &[Qubit],
    ) -> Result<Option<Vec<ValueOperation>>, CompilerError> {
        let key = McGateRuleKey::from_request(request);
        if let Some(rule) = self.mc_gate_rules.get(&key) {
            self.stats.hits += 1;
            return Ok(Some(rule.instantiate(controls, targets, ancillas)?));
        }
        self.stats.misses += 1;
        Ok(None)
    }

    /// Records a synthesized multi-controlled-gate decomposition as a runtime rule.
    ///
    /// `operations` must use only the supplied controls, targets, and ancillas.
    /// The generated rule is independent of the concrete qubit ids and can be
    /// instantiated later with any qubits that have the same role counts.
    pub fn insert_mc_gate(
        &mut self,
        request: McGateRuleRequest<'_>,
        controls: &[Qubit],
        targets: &[Qubit],
        ancillas: &[Qubit],
        operations: &[ValueOperation],
    ) -> Result<(), CompilerError> {
        let key = McGateRuleKey::from_request(request);
        let rule = DecompositionRule::from_operations(controls, targets, ancillas, operations)?;
        self.mc_gate_rules.insert(key, rule);
        self.stats.inserts += 1;
        Ok(())
    }

    /// Returns cache hit/miss/insert counters.
    pub const fn stats(&self) -> DecompositionRuleStats {
        self.stats
    }

    /// Instantiates a cached numeric unitary decomposition rule.
    ///
    /// Returns `Ok(None)` when no bit-exact matrix and basis match exists.
    pub fn instantiate_numeric_unitary(
        &mut self,
        request: NumericUnitaryRuleRequest<'_>,
        qubits: &[Qubit],
    ) -> Result<Option<(Vec<ValueOperation>, f64)>, CompilerError> {
        let key = NumericUnitaryRuleKey::from_request(request);
        if let Some(rule) = self.numeric_unitary_rules.get(&key) {
            self.stats.hits += 1;
            return Ok(Some((
                rule.rule.instantiate(&[], qubits, &[])?,
                rule.phase_delta,
            )));
        }
        self.stats.misses += 1;
        Ok(None)
    }

    /// Records a synthesized numeric unitary decomposition as a runtime rule.
    ///
    /// The numeric matrix key is bit-exact: mathematically equivalent matrices
    /// with different floating-point representations are intentionally distinct.
    pub fn insert_numeric_unitary(
        &mut self,
        request: NumericUnitaryRuleRequest<'_>,
        qubits: &[Qubit],
        operations: &[ValueOperation],
        phase_delta: f64,
    ) -> Result<(), CompilerError> {
        let key = NumericUnitaryRuleKey::from_request(request);
        let rule = DecompositionRule::from_operations(&[], qubits, &[], operations)?;
        self.numeric_unitary_rules
            .insert(key, NumericUnitaryRule { rule, phase_delta });
        self.stats.inserts += 1;
        Ok(())
    }
}

/// Runtime cache request for one multi-controlled-gate synthesis.
#[derive(Debug, Clone, Copy)]
pub struct McGateRuleRequest<'a> {
    pub gate: StandardGate,
    pub control_count: usize,
    pub target_count: usize,
    pub params: &'a [ParameterValue],
    pub resource: ResourceSignature,
}

/// Runtime cache request for one numeric unitary synthesis.
#[derive(Debug, Clone)]
pub struct NumericUnitaryRuleRequest<'a> {
    pub num_qubits: u16,
    pub matrix: &'a Array2<Complex64>,
    pub synthesis_key: NumericUnitarySynthesisKey,
}

/// Semantic context that determines numeric-unitary synthesis output.
///
/// This key intentionally includes the canonical target-basis identity and
/// exactness mode, so a cached template is never reused under a different
/// lowering policy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NumericUnitarySynthesisKey {
    OneQubit,
    TwoQubit {
        target_basis: Option<TargetBasisSignature>,
        fallback_pauli: bool,
    },
}

impl NumericUnitarySynthesisKey {
    pub const fn one_qubit() -> Self {
        Self::OneQubit
    }

    pub(crate) fn two_qubit(target: &TwoQubitSynthesisTarget) -> Self {
        Self::TwoQubit {
            target_basis: target.cache_signature(),
            fallback_pauli: target.fallback_pauli(),
        }
    }
}

#[derive(Debug, Clone)]
struct NumericUnitaryRule {
    rule: DecompositionRule,
    phase_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NumericUnitaryRuleKey {
    num_qubits: u16,
    rows: usize,
    cols: usize,
    matrix: Vec<(u64, u64)>,
    synthesis_key: NumericUnitarySynthesisKey,
}

impl NumericUnitaryRuleKey {
    fn from_request(request: NumericUnitaryRuleRequest<'_>) -> Self {
        Self {
            num_qubits: request.num_qubits,
            rows: request.matrix.nrows(),
            cols: request.matrix.ncols(),
            matrix: request
                .matrix
                .iter()
                .map(|value| (value.re.to_bits(), value.im.to_bits()))
                .collect(),
            synthesis_key: request.synthesis_key,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct McGateRuleKey {
    gate: StandardGate,
    control_count: usize,
    target_count: usize,
    params: Vec<ParameterValueKey>,
    resource: ResourceSignature,
}

impl McGateRuleKey {
    fn from_request(request: McGateRuleRequest<'_>) -> Self {
        Self {
            gate: request.gate,
            control_count: request.control_count,
            target_count: request.target_count,
            params: request.params.iter().map(ParameterValueKey::from).collect(),
            resource: request.resource,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ParameterValueKey {
    Fixed(u64),
    Param(Parameter),
}

impl From<&ParameterValue> for ParameterValueKey {
    fn from(value: &ParameterValue) -> Self {
        match value {
            ParameterValue::Fixed(value) => Self::Fixed(value.to_bits()),
            ParameterValue::Param(parameter) => Self::Param(parameter.clone()),
        }
    }
}

/// Resource-dependent synthesis signature for a runtime rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceSignature {
    pub algorithm: DecompositionAlgorithm,
    pub ancilla_kind: RuntimeAncillaKind,
    pub ancilla_count: usize,
}

impl ResourceSignature {
    /// Creates a signature for an ancillary-free decomposition.
    pub const fn no_aux(algorithm: DecompositionAlgorithm) -> Self {
        Self {
            algorithm,
            ancilla_kind: RuntimeAncillaKind::None,
            ancilla_count: 0,
        }
    }

    /// Creates a signature for a clean-ancilla decomposition.
    pub const fn clean(algorithm: DecompositionAlgorithm, count: usize) -> Self {
        Self {
            algorithm,
            ancilla_kind: RuntimeAncillaKind::CleanZero,
            ancilla_count: count,
        }
    }

    /// Creates a signature for a dirty-ancilla decomposition.
    pub const fn dirty(algorithm: DecompositionAlgorithm, count: usize) -> Self {
        Self {
            algorithm,
            ancilla_kind: RuntimeAncillaKind::Dirty,
            ancilla_count: count,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeAncillaKind {
    None,
    CleanZero,
    Dirty,
}

/// Algorithm identity used to prevent incompatible template reuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecompositionAlgorithm {
    PauliSmall,
    PauliTwoClean,
    PauliOneCleanKg24,
    PauliManyClean,
    PauliManyDirty,
    PauliTwoDirty,
    PauliOneDirty,
    PauliOneCleanB95,
    PauliNoAux,
    CleanAccumulator,
    NoAux,
}

#[derive(Debug, Clone)]
pub struct DecompositionRule {
    operations: Vec<TemplateOperation>,
}

impl DecompositionRule {
    /// Builds a runtime rule from concrete value operations.
    ///
    /// The resulting rule abstracts all qubits into control, target, and
    /// ancilla roles. The operation sequence may later be instantiated with
    /// different concrete qubits that preserve the same role counts.
    pub fn from_operations(
        controls: &[Qubit],
        targets: &[Qubit],
        ancillas: &[Qubit],
        operations: &[ValueOperation],
    ) -> Result<Self, CompilerError> {
        let mut template_operations = Vec::with_capacity(operations.len());
        for operation in operations {
            let ValueInstruction::Instruction(instruction) = &operation.instruction else {
                return Err(CompilerError::InvariantViolation(
                    "runtime decomposition rules support only gate-like value instructions"
                        .to_string(),
                ));
            };
            let qubits = operation
                .qubits
                .iter()
                .map(|qubit| template_qubit(*qubit, controls, targets, ancillas))
                .collect::<Result<SmallVec<[_; 3]>, _>>()?;
            template_operations.push(TemplateOperation {
                instruction: instruction.clone(),
                qubits,
                params: operation.params.clone(),
                label: operation.label.clone(),
            });
        }
        Ok(Self {
            operations: template_operations,
        })
    }

    /// Instantiates this rule with concrete control, target, and ancilla qubits.
    pub fn instantiate(
        &self,
        controls: &[Qubit],
        targets: &[Qubit],
        ancillas: &[Qubit],
    ) -> Result<Vec<ValueOperation>, CompilerError> {
        self.operations
            .iter()
            .map(|operation| operation.instantiate(controls, targets, ancillas))
            .collect()
    }
}

#[derive(Debug, Clone)]
struct TemplateOperation {
    instruction: Instruction,
    qubits: SmallVec<[TemplateQubit; 3]>,
    params: SmallVec<[ParameterValue; 1]>,
    label: Option<Box<str>>,
}

impl TemplateOperation {
    fn instantiate(
        &self,
        controls: &[Qubit],
        targets: &[Qubit],
        ancillas: &[Qubit],
    ) -> Result<ValueOperation, CompilerError> {
        let qubits = self
            .qubits
            .iter()
            .map(|qubit| qubit.instantiate(controls, targets, ancillas))
            .collect::<Result<SmallVec<[_; 3]>, _>>()?;
        Ok(ValueOperation {
            instruction: ValueInstruction::from_instruction(self.instruction.clone()),
            qubits,
            params: self.params.clone(),
            label: self.label.clone(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TemplateQubit {
    Control(usize),
    Target(usize),
    Ancilla(usize),
}

impl TemplateQubit {
    fn instantiate(
        self,
        controls: &[Qubit],
        targets: &[Qubit],
        ancillas: &[Qubit],
    ) -> Result<Qubit, CompilerError> {
        match self {
            Self::Control(index) => controls.get(index).copied(),
            Self::Target(index) => targets.get(index).copied(),
            Self::Ancilla(index) => ancillas.get(index).copied(),
        }
        .ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "runtime decomposition rule references missing template qubit {self:?}"
            ))
        })
    }
}

fn template_qubit(
    qubit: Qubit,
    controls: &[Qubit],
    targets: &[Qubit],
    ancillas: &[Qubit],
) -> Result<TemplateQubit, CompilerError> {
    if let Some(index) = controls.iter().position(|control| *control == qubit) {
        return Ok(TemplateQubit::Control(index));
    }
    if let Some(index) = targets.iter().position(|target| *target == qubit) {
        return Ok(TemplateQubit::Target(index));
    }
    if let Some(index) = ancillas.iter().position(|ancilla| *ancilla == qubit) {
        return Ok(TemplateQubit::Ancilla(index));
    }
    Err(CompilerError::InvariantViolation(format!(
        "runtime decomposition rule output uses qubit {qubit} outside source operands and ancillas"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::{Instruction, Parameter, Qubit, StandardGate, ValueOperation};
    use crate::compile::transform::decompose::unitary::TwoQubitSynthesisTarget;
    use ndarray::Array2;
    use num_complex::Complex64;
    use smallvec::smallvec;

    fn request<'a>(
        params: &'a [ParameterValue],
        resource: ResourceSignature,
    ) -> McGateRuleRequest<'a> {
        McGateRuleRequest {
            gate: StandardGate::RX,
            control_count: 2,
            target_count: 1,
            params,
            resource,
        }
    }

    #[test]
    fn cached_rule_instantiates_on_different_qubits() {
        let params = [ParameterValue::Param(Parameter::symbol("theta"))];
        let resource = ResourceSignature::clean(DecompositionAlgorithm::CleanAccumulator, 1);
        let mut cache = DecompositionRuleCache::default();
        let operations = vec![
            ValueOperation {
                instruction: ValueInstruction::from_instruction(Instruction::Standard(
                    StandardGate::CX,
                )),
                qubits: smallvec![Qubit::new(0), Qubit::new(3)],
                params: smallvec![],
                label: None,
            },
            ValueOperation {
                instruction: ValueInstruction::from_instruction(Instruction::Standard(
                    StandardGate::RX,
                )),
                qubits: smallvec![Qubit::new(2)],
                params: smallvec![params[0].clone()],
                label: None,
            },
        ];

        assert!(
            cache
                .instantiate_mc_gate(
                    request(&params, resource),
                    &[Qubit::new(0), Qubit::new(1)],
                    &[Qubit::new(2)],
                    &[Qubit::new(3)]
                )
                .unwrap()
                .is_none()
        );
        cache
            .insert_mc_gate(
                request(&params, resource),
                &[Qubit::new(0), Qubit::new(1)],
                &[Qubit::new(2)],
                &[Qubit::new(3)],
                &operations,
            )
            .unwrap();

        let instantiated = cache
            .instantiate_mc_gate(
                request(&params, resource),
                &[Qubit::new(5), Qubit::new(6)],
                &[Qubit::new(7)],
                &[Qubit::new(8)],
            )
            .unwrap()
            .unwrap();

        assert_eq!(
            instantiated[0].qubits.as_slice(),
            &[Qubit::new(5), Qubit::new(8)]
        );
        assert_eq!(instantiated[1].qubits.as_slice(), &[Qubit::new(7)]);
        assert_eq!(
            cache.stats(),
            DecompositionRuleStats {
                hits: 1,
                misses: 1,
                inserts: 1,
            }
        );
    }

    #[test]
    fn resource_signature_separates_otherwise_equal_rules() {
        let params = [ParameterValue::Fixed(0.25)];
        let mut cache = DecompositionRuleCache::default();
        let operations = vec![ValueOperation::from_standard(
            StandardGate::RX,
            [Qubit::new(2)],
            [params[0].clone()],
        )];
        cache
            .insert_mc_gate(
                request(
                    &params,
                    ResourceSignature::no_aux(DecompositionAlgorithm::NoAux),
                ),
                &[Qubit::new(0), Qubit::new(1)],
                &[Qubit::new(2)],
                &[],
                &operations,
            )
            .unwrap();

        assert!(
            cache
                .instantiate_mc_gate(
                    request(
                        &params,
                        ResourceSignature::clean(DecompositionAlgorithm::CleanAccumulator, 1),
                    ),
                    &[Qubit::new(0), Qubit::new(1)],
                    &[Qubit::new(2)],
                    &[Qubit::new(3)],
                )
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn numeric_two_qubit_cache_separates_target_signatures() {
        let matrix = Array2::<Complex64>::eye(4);
        let cx_target = TwoQubitSynthesisTarget::from_standard_gates(
            vec![StandardGate::U, StandardGate::H],
            vec![StandardGate::CX],
            true,
        )
        .unwrap();
        let cz_target = TwoQubitSynthesisTarget::from_standard_gates(
            vec![StandardGate::U, StandardGate::H],
            vec![StandardGate::CZ],
            true,
        )
        .unwrap();
        let request = NumericUnitaryRuleRequest {
            num_qubits: 2,
            matrix: &matrix,
            synthesis_key: NumericUnitarySynthesisKey::two_qubit(&cx_target),
        };
        let distinct_target_request = NumericUnitaryRuleRequest {
            num_qubits: 2,
            matrix: &matrix,
            synthesis_key: NumericUnitarySynthesisKey::two_qubit(&cz_target),
        };
        let qubits = [Qubit::new(0), Qubit::new(1)];
        let operations = vec![ValueOperation::from_standard(StandardGate::CX, qubits, [])];
        let mut cache = DecompositionRuleCache::default();

        assert!(
            cache
                .instantiate_numeric_unitary(request.clone(), &qubits)
                .unwrap()
                .is_none()
        );
        cache
            .insert_numeric_unitary(request.clone(), &qubits, &operations, 0.0)
            .unwrap();
        assert!(
            cache
                .instantiate_numeric_unitary(request, &[Qubit::new(2), Qubit::new(3)])
                .unwrap()
                .is_some()
        );
        assert!(
            cache
                .instantiate_numeric_unitary(distinct_target_request, &qubits)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            cache.stats(),
            DecompositionRuleStats {
                hits: 1,
                misses: 2,
                inserts: 1,
            }
        );
    }
}
