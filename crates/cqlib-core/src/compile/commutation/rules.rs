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

//! Rule-based commutation oracle.
//!
//! This module extracts exact two-operation commutation rules from the compiler
//! knowledge library.  A rule is accepted by this oracle only when its source
//! and target are the same two operations in swapped order, i.e. an explicit
//! `A; B -> B; A` proof.  Matching is structural: instruction identity, a
//! one-to-one qubit binding, symbolic parameter bindings, and rule conditions
//! must all agree.
//!
//! # Examples
//!
//! ```rust
//! use cqlib_core::circuit::{Instruction, Parameter, Qubit, StandardGate};
//! use cqlib_core::compile::commutation::{Commutation, CommutationChecker};
//!
//! let checker = CommutationChecker::builtin();
//! let result = checker.check(
//!     &Instruction::Standard(StandardGate::CX),
//!     &[Qubit::new(0), Qubit::new(1)],
//!     &[],
//!     &Instruction::Standard(StandardGate::RZ),
//!     &[Qubit::new(0)],
//!     &[Parameter::symbol("theta")],
//! );
//!
//! assert_eq!(result, Some(Commutation::Exact));
//! ```

use crate::circuit::{Instruction, Parameter, ParameterValue, Qubit};
use crate::compile::PARAMETER_EQ_TOLERANCE;
use crate::compile::commutation::checker::{Commutation, CommutationResult};
use crate::compile::knowledge::library::{RuleKind, RuleLibrary};
use crate::compile::knowledge::matcher::KnowledgeInstructionKey;
use crate::compile::knowledge::rule::{Condition, Rule, RuleItem};
use smallvec::SmallVec;
use std::collections::HashMap;

/// Compiled commutation rules loaded from the compiler knowledge library.
#[derive(Debug, Clone, Default)]
pub struct RuleCommutationOracle {
    /// Validated two-operation swap rules.
    rules: Vec<Rule>,
    /// Candidate rules indexed by the concrete ordered instruction pair.
    rule_index:
        HashMap<(KnowledgeInstructionKey, KnowledgeInstructionKey), SmallVec<[IndexedRule; 4]>>,
}

/// One rule candidate and the operand order used to match it.
#[derive(Debug, Clone, Copy)]
struct IndexedRule {
    /// Position of the rule in [`RuleCommutationOracle::rules`].
    rule_index: usize,
    /// Whether the concrete operands match the rule's source order or its reverse.
    direction: MatchDirection,
}

/// Operand direction recorded while building the instruction-pair index.
#[derive(Debug, Clone, Copy)]
enum MatchDirection {
    Forward,
    Reverse,
}

/// Bindings accumulated while matching one commutation rule instance.
///
/// Qubits are tracked in both directions so one rule-local qubit cannot bind to
/// multiple concrete qubits and one concrete qubit cannot satisfy two distinct
/// rule-local labels.
#[derive(Debug, Clone, Default)]
struct MatchState {
    /// Mapping from rule-local qubit labels to concrete circuit qubits.
    qubits: HashMap<u32, Qubit>,
    /// Reverse mapping used to enforce one-to-one qubit bindings.
    reverse_qubits: HashMap<Qubit, u32>,
    /// Symbolic parameter bindings captured from the rule pattern.
    params: HashMap<String, Parameter>,
}

impl RuleCommutationOracle {
    /// Builds an oracle from the builtin compiler knowledge library.
    pub fn builtin() -> Self {
        let library =
            RuleLibrary::builtin_rules().expect("builtin compiler knowledge rules must load");
        Self::from_library(library)
    }

    /// Builds an oracle from the `RuleKind::Commute` rules in `library`.
    pub fn from_library(library: &RuleLibrary) -> Self {
        let rules = library
            .rules_by_kind(RuleKind::Commute)
            .iter()
            .filter_map(|id| library.get(*id))
            .cloned()
            .collect::<Vec<_>>();

        Self::from_rules(&rules)
    }

    /// Checks whether the oracle has an exact swap proof for the operation pair.
    ///
    /// Candidate rules are selected by ordered instruction pair and retain both
    /// operand directions, so callers do not need to canonicalize `lhs` and
    /// `rhs` before querying.
    pub fn check(
        &self,
        lhs_inst: &Instruction,
        lhs_qubits: &[Qubit],
        lhs_params: &[Parameter],
        rhs_inst: &Instruction,
        rhs_qubits: &[Qubit],
        rhs_params: &[Parameter],
    ) -> CommutationResult {
        let lhs_key = KnowledgeInstructionKey::from_instruction(lhs_inst)?;
        let rhs_key = KnowledgeInstructionKey::from_instruction(rhs_inst)?;
        let candidates = self.rule_index.get(&(lhs_key, rhs_key))?;

        for candidate in candidates {
            let rule = &self.rules[candidate.rule_index];
            let matched = match candidate.direction {
                MatchDirection::Forward => rule_matches(
                    rule, lhs_inst, lhs_qubits, lhs_params, rhs_inst, rhs_qubits, rhs_params,
                ),
                MatchDirection::Reverse => rule_matches(
                    rule, rhs_inst, rhs_qubits, rhs_params, lhs_inst, lhs_qubits, lhs_params,
                ),
            };
            if matched {
                return Some(Commutation::Exact);
            }
        }
        None
    }

    /// Filters raw rules down to exact two-operation swaps accepted by this oracle.
    fn from_rules(rules: &[Rule]) -> Self {
        let rules = rules
            .iter()
            .filter(|rule| {
                if rule.operations.len() != 2 || rule.target.len() != 2 {
                    return false;
                }

                rule.operations[0].equivalent_to(&rule.target[1])
                    && rule.operations[1].equivalent_to(&rule.target[0])
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut rule_index: HashMap<
            (KnowledgeInstructionKey, KnowledgeInstructionKey),
            SmallVec<[IndexedRule; 4]>,
        > = HashMap::new();

        for (index, rule) in rules.iter().enumerate() {
            let Some(lhs_key) =
                KnowledgeInstructionKey::from_instruction(&rule.operations[0].instruction)
            else {
                continue;
            };
            let Some(rhs_key) =
                KnowledgeInstructionKey::from_instruction(&rule.operations[1].instruction)
            else {
                continue;
            };

            rule_index
                .entry((lhs_key.clone(), rhs_key.clone()))
                .or_default()
                .push(IndexedRule {
                    rule_index: index,
                    direction: MatchDirection::Forward,
                });
            rule_index
                .entry((rhs_key, lhs_key))
                .or_default()
                .push(IndexedRule {
                    rule_index: index,
                    direction: MatchDirection::Reverse,
                });
        }

        Self { rules, rule_index }
    }
}

/// Matches one accepted commutation rule against an ordered operation pair.
fn rule_matches(
    rule: &Rule,
    lhs_inst: &Instruction,
    lhs_qubits: &[Qubit],
    lhs_params: &[Parameter],
    rhs_inst: &Instruction,
    rhs_qubits: &[Qubit],
    rhs_params: &[Parameter],
) -> bool {
    let mut state = MatchState::default();
    match_rule_item(
        &rule.operations[0],
        lhs_inst,
        lhs_qubits,
        lhs_params,
        &mut state,
    ) && match_rule_item(
        &rule.operations[1],
        rhs_inst,
        rhs_qubits,
        rhs_params,
        &mut state,
    ) && conditions_hold(rule.conditions.as_deref().unwrap_or(&[]), &state.params)
}

/// Matches one rule item against one concrete operation and updates bindings.
///
/// This matcher is intentionally small and exact because commutation rules are
/// used as proofs.  Any instruction mismatch, arity mismatch, inconsistent
/// qubit binding, or unprovable parameter relation rejects the rule.
fn match_rule_item(
    item: &RuleItem,
    instruction: &Instruction,
    qubits: &[Qubit],
    params: &[Parameter],
    state: &mut MatchState,
) -> bool {
    if !matches!(
        (&item.instruction, instruction),
        (Instruction::Standard(lhs), Instruction::Standard(rhs)) if lhs == rhs
    ) && !matches!(
        (&item.instruction, instruction),
        (Instruction::McGate(lhs), Instruction::McGate(rhs)) if lhs == rhs
    ) {
        return false;
    }
    if item.qubits.len() != qubits.len() {
        return false;
    }

    for (&rule_qubit, &actual_qubit) in item.qubits.iter().zip(qubits) {
        if let Some(bound) = state.qubits.get(&rule_qubit) {
            if *bound != actual_qubit {
                return false;
            }
        } else if let Some(other_rule_qubit) = state.reverse_qubits.get(&actual_qubit) {
            if *other_rule_qubit != rule_qubit {
                return false;
            }
        } else {
            state.qubits.insert(rule_qubit, actual_qubit);
            state.reverse_qubits.insert(actual_qubit, rule_qubit);
        }
    }

    let rule_params = item.params.as_deref().unwrap_or(&[]);
    if rule_params.len() != params.len() {
        return false;
    }

    rule_params
        .iter()
        .zip(params)
        .all(|(rule_param, actual)| match_parameter(rule_param, actual, &mut state.params))
}

/// Matches or binds one parameter pattern against a concrete parameter value.
///
/// A bare symbol creates or checks a binding.  A compound expression is first
/// substituted with known bindings and must then reduce to a provable equality.
fn match_parameter(
    rule_param: &ParameterValue,
    actual: &Parameter,
    bindings: &mut HashMap<String, Parameter>,
) -> bool {
    match rule_param {
        ParameterValue::Fixed(value) => {
            Parameter::from(*value).provably_equal(actual, PARAMETER_EQ_TOLERANCE)
        }
        ParameterValue::Param(pattern) => {
            if let Some(symbol) = pattern.as_symbol() {
                if let Some(bound) = bindings.get(&symbol) {
                    return bound.provably_equal(actual, PARAMETER_EQ_TOLERANCE);
                }
                bindings.insert(symbol, actual.clone());
                return true;
            }

            let substituted = pattern.substitute_many(bindings);
            substituted.get_symbols().is_empty()
                && substituted.provably_equal(actual, PARAMETER_EQ_TOLERANCE)
        }
    }
}

/// Returns whether every rule condition holds under current parameter bindings.
fn conditions_hold(conditions: &[Condition], bindings: &HashMap<String, Parameter>) -> bool {
    conditions.iter().all(|condition| match condition {
        Condition::Eq(lhs, rhs) => {
            let lhs = lhs.substitute_many(bindings);
            let rhs = rhs.substitute_many(bindings);
            lhs.provably_equal(&rhs, PARAMETER_EQ_TOLERANCE)
        }
        Condition::EqMod(lhs, rhs, modulus) => {
            let lhs = lhs.substitute_many(bindings);
            let rhs = rhs.substitute_many(bindings);
            let modulus = modulus.substitute_many(bindings);
            lhs.provably_equal_modulo(&rhs, &modulus, PARAMETER_EQ_TOLERANCE)
        }
    })
}

#[cfg(test)]
#[path = "./rules_test.rs"]
mod rules_test;
