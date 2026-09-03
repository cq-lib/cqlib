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

//! Runtime model for knowledge-base rewrite rules.
//!
//! This module describes what a rule is.  Rule equivalence checking lives in
//! `rule_equivalence` because matrix construction, parameter sampling, and
//! numerical comparison are validation strategies rather than rule structure.

use crate::circuit::{Instruction, MCGate, Parameter, ParameterValue, StandardGate};
use crate::compile::PARAMETER_EQ_TOLERANCE;
use smallvec::SmallVec;
use std::collections::{BTreeSet, HashSet};

/// One gate-like item in a rule match or rewrite block.
#[derive(Debug, Clone)]
pub struct RuleItem {
    /// Gate-like instruction matched or emitted by this item.
    pub instruction: Instruction,
    /// Rule-local qubit labels used by this item.
    pub qubits: SmallVec<[u32; 3]>,
    /// Optional symbolic or fixed parameters for this item.
    pub params: Option<SmallVec<[ParameterValue; 1]>>,
}

impl RuleItem {
    /// Builds a standard-gate rule item.
    pub fn standard(gate: StandardGate, qubits: &[u32], params: Vec<ParameterValue>) -> Self {
        Self {
            instruction: Instruction::Standard(gate),
            qubits: SmallVec::from_slice(qubits),
            params: if params.is_empty() {
                None
            } else {
                Some(SmallVec::from_vec(params))
            },
        }
    }

    /// Builds a multi-controlled-gate rule item.
    pub fn mc_gate(gate: MCGate, qubits: &[u32], params: Vec<ParameterValue>) -> Self {
        Self {
            instruction: Instruction::McGate(Box::new(gate)),
            qubits: SmallVec::from_slice(qubits),
            params: if params.is_empty() {
                None
            } else {
                Some(SmallVec::from_vec(params))
            },
        }
    }

    /// Collect free symbols referenced by this item's symbolic parameters.
    pub fn symbols(&self) -> HashSet<String> {
        let mut symbols = HashSet::new();
        if let Some(params) = &self.params {
            for param in params {
                if let ParameterValue::Param(param) = param {
                    symbols.extend(param.get_symbols());
                }
            }
        }

        symbols
    }

    /// Validate invariants that depend only on this item, not on its rule block.
    pub fn validate(&self) -> Result<(), RuleValidationError> {
        let (instruction, expected_qubits, expected_params) = match &self.instruction {
            Instruction::Standard(gate) => (gate.to_string(), gate.num_qubits(), gate.num_params()),
            Instruction::McGate(gate) => (
                format!(
                    "MC{}[{}]",
                    gate.base_gate(),
                    gate.num_qubits() - gate.base_gate().num_qubits()
                ),
                gate.num_qubits(),
                gate.num_params(),
            ),
            other => {
                return Err(RuleValidationError::UnsupportedInstruction {
                    instruction: other.to_string(),
                });
            }
        };

        if self.qubits.len() != expected_qubits {
            return Err(RuleValidationError::WrongQubitCount {
                instruction: instruction.clone(),
                expected: expected_qubits,
                got: self.qubits.len(),
            });
        }

        let got_params = self.params.as_ref().map_or(0, SmallVec::len);
        if got_params != expected_params {
            return Err(RuleValidationError::WrongParamCount {
                instruction: instruction.clone(),
                expected: expected_params,
                got: got_params,
            });
        }

        let mut seen = SmallVec::<[u32; 3]>::new();
        for &qubit in &self.qubits {
            if seen.contains(&qubit) {
                return Err(RuleValidationError::DuplicateQubit {
                    instruction: instruction.clone(),
                    qubit,
                });
            }
            seen.push(qubit);
        }

        Ok(())
    }

    /// Returns whether this item describes the same instruction, qubit labels,
    /// and provably equivalent parameter expressions as another rule item.
    pub fn equivalent_to(&self, other: &Self) -> bool {
        let lhs_params = self.params.as_deref().unwrap_or(&[]);
        let rhs_params = other.params.as_deref().unwrap_or(&[]);
        self.instruction == other.instruction
            && self.qubits == other.qubits
            && lhs_params.len() == rhs_params.len()
            && lhs_params.iter().zip(rhs_params).all(|(lhs, rhs)| {
                let lhs: Parameter = lhs.into();
                lhs.provably_equal(&rhs.into(), PARAMETER_EQ_TOLERANCE)
            })
    }
}

#[derive(Debug, Clone)]
/// Parameter constraints required for a rewrite rule to apply.
pub enum Condition {
    /// Requires two parameter expressions to be equal.
    Eq(Parameter, Parameter),
    /// Requires `lhs == rhs mod modulus`.
    EqMod(Parameter, Parameter, Parameter),
}

impl Condition {
    /// Collect free symbols referenced by this condition.
    pub fn symbols(&self) -> HashSet<String> {
        let mut symbols = HashSet::new();
        match self {
            Condition::Eq(lhs, rhs) => {
                symbols.extend(lhs.get_symbols());
                symbols.extend(rhs.get_symbols());
            }
            Condition::EqMod(lhs, rhs, modulus) => {
                symbols.extend(lhs.get_symbols());
                symbols.extend(rhs.get_symbols());
                symbols.extend(modulus.get_symbols());
            }
        }

        symbols
    }
}

#[derive(Debug, Clone)]
/// Runtime rewrite rule: match operations plus optional conditions and target.
pub struct Rule {
    /// Stable human-readable rule name.
    pub name: String,
    /// Operations to match in the source circuit.
    pub operations: SmallVec<[RuleItem; 4]>,
    /// Parameter constraints that must hold before applying the rewrite.
    pub conditions: Option<SmallVec<[Condition; 2]>>,
    /// Operations that replace the matched block.
    pub target: SmallVec<[RuleItem; 4]>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuleValidationError {
    /// A rule must match at least one source operation.
    #[error("rule match block is empty")]
    EmptyMatch,
    /// The rule contains an instruction that the rule runtime cannot match.
    #[error("unsupported instruction: {instruction}")]
    UnsupportedInstruction { instruction: String },
    /// A rule item uses the wrong number of qubits for its instruction.
    #[error("wrong qubit count for {instruction}: expected {expected}, got {got}")]
    WrongQubitCount {
        instruction: String,
        expected: usize,
        got: usize,
    },
    /// A rule item uses the wrong number of parameters for its instruction.
    #[error("wrong parameter count for {instruction}: expected {expected}, got {got}")]
    WrongParamCount {
        instruction: String,
        expected: usize,
        got: usize,
    },
    /// A single rule item repeats one rule-local qubit label.
    #[error("duplicate qubit {qubit} in {instruction}")]
    DuplicateQubit { instruction: String, qubit: u32 },
    /// A rewrite target references a qubit label absent from the match block.
    #[error("rewrite qubit {qubit} is not bound by the match block")]
    UnboundRewriteQubit { qubit: u32 },
    /// A rewrite parameter references a symbol absent from the match block.
    #[error("rewrite symbol {symbol} is not bound by the match block")]
    UnboundRewriteSymbol { symbol: String },
    /// A condition references a symbol absent from the match block.
    #[error("condition symbol {symbol} is not bound by the match block")]
    UnboundConditionSymbol { symbol: String },
    /// Rule-local qubit labels must form a dense range starting at zero.
    #[error("qubit labels must be dense from 0, found {labels:?}")]
    NonDenseQubitLabels { labels: Vec<u32> },
}

impl Rule {
    /// Builds a rule with no conditions.
    pub fn new(name: &str, ops: Vec<RuleItem>, target: Vec<RuleItem>) -> Rule {
        Rule {
            name: name.to_string(),
            operations: SmallVec::from_vec(ops),
            conditions: None,
            target: SmallVec::from_vec(target),
        }
    }

    /// Validate structural invariants required by the rule runtime.
    ///
    /// This checks whether a rule is well-formed enough to be dumped, verified,
    /// and eventually matched. It does not prove that the rewrite preserves
    /// quantum semantics; matrix equivalence checking lives in
    /// `rule_equivalence`.
    pub fn validate(&self) -> Result<(), RuleValidationError> {
        if self.operations.is_empty() {
            return Err(RuleValidationError::EmptyMatch);
        }

        // Item-level checks come first so later block checks can assume valid gate arity.
        for item in &self.operations {
            item.validate()?;
        }
        for item in &self.target {
            item.validate()?;
        }

        let match_qubits = self.operation_qubits();
        // Rule-local qubit labels are normalized to a dense 0..n range.
        validate_dense_qubit_labels(&match_qubits)?;

        // Rewrites may use a subset of matched qubits, but cannot introduce new ones.
        for item in &self.target {
            for &qubit in &item.qubits {
                if !match_qubits.contains(&qubit) {
                    return Err(RuleValidationError::UnboundRewriteQubit { qubit });
                }
            }
        }

        let bound_symbols = self.operation_symbols();
        // Rewrite parameters must be expressions over symbols bound by the match block.
        for symbol in self.target_symbols() {
            if !bound_symbols.contains(&symbol) {
                return Err(RuleValidationError::UnboundRewriteSymbol { symbol });
            }
        }

        // Conditions constrain matched symbols; they cannot introduce new parameters.
        for symbol in self.condition_symbols() {
            if !bound_symbols.contains(&symbol) {
                return Err(RuleValidationError::UnboundConditionSymbol { symbol });
            }
        }

        Ok(())
    }

    /// Determine the number of qubits from the rule's operations and target.
    pub fn num_qubits(&self) -> usize {
        let mut op_qubits = self.operation_qubits();
        op_qubits.extend(self.target_qubits());
        // Minimum 1 for GPhase-only rules.
        op_qubits.len().max(1)
    }

    /// Collect all free symbol names from parameters and conditions.
    pub fn collect_free_symbols(&self) -> HashSet<String> {
        let mut symbols = self.operation_symbols();
        symbols.extend(self.target_symbols());
        symbols.extend(self.condition_symbols());
        symbols
    }

    /// Collect qubit labels referenced by the rewrite target.
    pub fn target_qubits(&self) -> BTreeSet<u32> {
        self.target
            .iter()
            .flat_map(|qs| qs.qubits.iter().cloned())
            .collect()
    }

    /// Collect qubit labels referenced by the match block.
    pub fn operation_qubits(&self) -> BTreeSet<u32> {
        self.operations
            .iter()
            .flat_map(|qs| qs.qubits.iter().cloned())
            .collect()
    }

    /// Collect symbols bound by the match block.
    pub fn operation_symbols(&self) -> HashSet<String> {
        self.operations.iter().flat_map(RuleItem::symbols).collect()
    }

    /// Collect symbols referenced by the rewrite target.
    pub fn target_symbols(&self) -> HashSet<String> {
        self.target.iter().flat_map(RuleItem::symbols).collect()
    }

    /// Collect symbols referenced by all rule conditions.
    pub fn condition_symbols(&self) -> HashSet<String> {
        self.conditions
            .iter()
            .flatten()
            .flat_map(Condition::symbols)
            .collect()
    }
}

fn validate_dense_qubit_labels(labels: &BTreeSet<u32>) -> Result<(), RuleValidationError> {
    for (expected, actual) in labels.iter().copied().enumerate() {
        if expected as u32 != actual {
            return Err(RuleValidationError::NonDenseQubitLabels {
                labels: labels.iter().copied().collect(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "./rule_test.rs"]
mod rule_test;
