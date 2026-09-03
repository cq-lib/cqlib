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

//! Shared structural matcher for compiler knowledge rules.
//!
//! This module owns the semantics of matching a knowledge [`RuleItem`] against a
//! concrete gate-like operation: instruction identity, one-to-one qubit binding,
//! symbolic parameter binding, condition evaluation, and rewrite target
//! instantiation. It intentionally does not implement transform policy such as
//! commutation-aware search, rewrite windows, target-basis filtering, local cost
//! comparison, or patch selection.

use crate::circuit::{Instruction, MCGate, Parameter, ParameterValue, Qubit, StandardGate};
use crate::compile::PARAMETER_EQ_TOLERANCE;
use crate::compile::knowledge::rule::{Condition, Rule, RuleItem};
use smallvec::SmallVec;
use std::collections::HashMap;

/// Instruction subset supported by knowledge-rule structural matching.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KnowledgeInstructionKey {
    Standard(StandardGate),
    McGate(MCGate),
}

impl KnowledgeInstructionKey {
    /// Returns a matcher key for gate-like instructions supported by rules.
    pub fn from_instruction(instruction: &Instruction) -> Option<Self> {
        match instruction {
            Instruction::Standard(gate) => Some(Self::Standard(*gate)),
            Instruction::McGate(gate) => Some(Self::McGate(gate.as_ref().clone())),
            _ => None,
        }
    }

    pub(crate) fn is_implicit(&self) -> bool {
        matches!(self, Self::Standard(StandardGate::GPhase))
    }

    pub(crate) fn num_qubits(&self) -> Option<usize> {
        Some(match self {
            Self::Standard(gate) => gate.num_qubits(),
            Self::McGate(gate) => gate.num_qubits(),
        })
    }

    pub(crate) fn num_params(&self) -> Option<usize> {
        Some(match self {
            Self::Standard(gate) => gate.num_params(),
            Self::McGate(gate) => gate.num_params(),
        })
    }
}

/// Borrowed concrete operation input for rule matching.
#[derive(Debug, Clone, Copy)]
pub struct ConcreteOperationView<'a> {
    pub instruction: &'a Instruction,
    pub qubits: &'a [Qubit],
    pub params: &'a [Parameter],
}

impl<'a> ConcreteOperationView<'a> {
    pub const fn new(
        instruction: &'a Instruction,
        qubits: &'a [Qubit],
        params: &'a [Parameter],
    ) -> Self {
        Self {
            instruction,
            qubits,
            params,
        }
    }

    pub fn key(&self) -> Option<KnowledgeInstructionKey> {
        KnowledgeInstructionKey::from_instruction(self.instruction)
    }
}

/// Mutable bindings produced while matching a rule instance.
#[derive(Debug, Clone, Default)]
pub struct MatchBindings {
    qubits: HashMap<u32, Qubit>,
    reverse_qubits: HashMap<Qubit, u32>,
    params: HashMap<String, Parameter>,
    insertion_log: Vec<BindingInsertion>,
}

#[derive(Debug, Clone)]
enum BindingInsertion {
    Qubit { rule: u32, actual: Qubit },
    Parameter(String),
}

impl PartialEq for MatchBindings {
    fn eq(&self, other: &Self) -> bool {
        self.qubits == other.qubits
            && self.reverse_qubits == other.reverse_qubits
            && self.params == other.params
    }
}

impl Eq for MatchBindings {}

impl MatchBindings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Removes all bindings while retaining the allocated matching workspace.
    pub(crate) fn clear(&mut self) {
        self.qubits.clear();
        self.reverse_qubits.clear();
        self.params.clear();
        self.insertion_log.clear();
    }

    /// Returns all rule-local qubit bindings.
    pub fn qubits(&self) -> &HashMap<u32, Qubit> {
        &self.qubits
    }

    /// Returns the concrete qubit bound to a rule-local qubit label.
    pub fn qubit(&self, rule_qubit: u32) -> Option<Qubit> {
        self.qubits.get(&rule_qubit).copied()
    }

    /// Returns all symbolic parameter bindings.
    pub fn params(&self) -> &HashMap<String, Parameter> {
        &self.params
    }

    /// Returns the concrete parameter bound to a rule symbol.
    pub fn param(&self, symbol: &str) -> Option<&Parameter> {
        self.params.get(symbol)
    }

    pub(crate) fn checkpoint(&self) -> usize {
        self.insertion_log.len()
    }

    pub(crate) fn rollback(&mut self, checkpoint: usize) {
        while self.insertion_log.len() > checkpoint {
            match self
                .insertion_log
                .pop()
                .expect("binding insertion log length was checked")
            {
                BindingInsertion::Qubit { rule, actual } => {
                    self.qubits.remove(&rule);
                    self.reverse_qubits.remove(&actual);
                }
                BindingInsertion::Parameter(symbol) => {
                    self.params.remove(&symbol);
                }
            }
        }
    }

    fn bind_qubits(&mut self, item: &RuleItem, concrete: ConcreteOperationView<'_>) -> bool {
        for (&rule_qubit, &actual_qubit) in item.qubits.iter().zip(concrete.qubits) {
            if let Some(bound) = self.qubits.get(&rule_qubit) {
                if *bound != actual_qubit {
                    return false;
                }
            } else if let Some(other_rule_qubit) = self.reverse_qubits.get(&actual_qubit) {
                if *other_rule_qubit != rule_qubit {
                    return false;
                }
            } else {
                self.qubits.insert(rule_qubit, actual_qubit);
                self.reverse_qubits.insert(actual_qubit, rule_qubit);
                self.insertion_log.push(BindingInsertion::Qubit {
                    rule: rule_qubit,
                    actual: actual_qubit,
                });
            }
        }
        true
    }

    fn bind_parameters(&mut self, item: &RuleItem, concrete: ConcreteOperationView<'_>) -> bool {
        let rule_params = item.params.as_deref().unwrap_or(&[]);
        if rule_params.len() != concrete.params.len() {
            return false;
        }

        rule_params
            .iter()
            .zip(concrete.params)
            .all(|(rule_param, actual)| self.match_parameter(rule_param, actual))
    }

    fn match_parameter(&mut self, rule_param: &ParameterValue, actual: &Parameter) -> bool {
        match rule_param {
            ParameterValue::Fixed(value) => {
                Parameter::from(*value).provably_equal(actual, PARAMETER_EQ_TOLERANCE)
            }
            ParameterValue::Param(pattern) => {
                if let Some(symbol) = pattern.as_symbol() {
                    if let Some(bound) = self.params.get(&symbol) {
                        return bound.provably_equal(actual, PARAMETER_EQ_TOLERANCE);
                    }
                    self.params.insert(symbol.clone(), actual.clone());
                    self.insertion_log.push(BindingInsertion::Parameter(symbol));
                    return true;
                }

                let substituted = pattern.substitute_many(&self.params);
                substituted.get_symbols().is_empty()
                    && substituted.provably_equal(actual, PARAMETER_EQ_TOLERANCE)
            }
        }
    }
}

/// One rewrite target item after applying match bindings.
#[derive(Debug, Clone)]
pub struct MatchedReplacement {
    pub instruction: Instruction,
    pub qubits: SmallVec<[Qubit; 3]>,
    pub params: SmallVec<[ParameterValue; 3]>,
    pub key: KnowledgeInstructionKey,
}

/// Errors produced by rule matching and target instantiation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MatchError {
    #[error("unsupported rule instruction: {instruction}")]
    UnsupportedRuleInstruction { instruction: String },
    #[error("rewrite qubit {qubit} is not bound by the match")]
    UnboundRewriteQubit { qubit: u32 },
    #[error("rewrite symbol {symbol} is not bound by the match")]
    UnboundRewriteSymbol { symbol: String },
}

/// Matches one rule item against a concrete operation.
///
/// The match is transactional: when this function returns `Ok(false)`, the
/// supplied bindings are left unchanged.
pub fn match_rule_item(
    item: &RuleItem,
    concrete: ConcreteOperationView<'_>,
    bindings: &mut MatchBindings,
) -> Result<bool, MatchError> {
    let Some(item_key) = KnowledgeInstructionKey::from_instruction(&item.instruction) else {
        return Err(MatchError::UnsupportedRuleInstruction {
            instruction: item.instruction.to_string(),
        });
    };
    let Some(concrete_key) = concrete.key() else {
        return Ok(false);
    };
    match_rule_item_with_keys(item, &item_key, &concrete_key, concrete, bindings)
}

/// Internal matcher entry point for callers that already cache instruction keys.
pub(crate) fn match_rule_item_with_keys(
    item: &RuleItem,
    item_key: &KnowledgeInstructionKey,
    concrete_key: &KnowledgeInstructionKey,
    concrete: ConcreteOperationView<'_>,
    bindings: &mut MatchBindings,
) -> Result<bool, MatchError> {
    if item_key != concrete_key || item.qubits.len() != concrete.qubits.len() {
        return Ok(false);
    }

    let checkpoint = bindings.checkpoint();
    if !bindings.bind_qubits(item, concrete) {
        bindings.rollback(checkpoint);
        return Ok(false);
    }
    if !bindings.bind_parameters(item, concrete) {
        bindings.rollback(checkpoint);
        return Ok(false);
    }

    Ok(true)
}

/// Returns whether all rule conditions hold under current bindings.
pub fn conditions_hold(conditions: Option<&[Condition]>, bindings: &MatchBindings) -> bool {
    conditions.unwrap_or(&[]).iter().all(|condition| {
        condition_symbols_bound(condition, bindings)
            && match condition {
                Condition::Eq(lhs, rhs) => {
                    let lhs = lhs.substitute_many(&bindings.params);
                    let rhs = rhs.substitute_many(&bindings.params);
                    lhs.provably_equal(&rhs, PARAMETER_EQ_TOLERANCE)
                }
                Condition::EqMod(lhs, rhs, modulus) => {
                    let lhs = lhs.substitute_many(&bindings.params);
                    let rhs = rhs.substitute_many(&bindings.params);
                    let modulus = modulus.substitute_many(&bindings.params);
                    lhs.provably_equal_modulo(&rhs, &modulus, PARAMETER_EQ_TOLERANCE)
                }
            }
    })
}

/// Instantiates a rewrite target using existing match bindings.
pub fn instantiate_target(
    target: &[RuleItem],
    bindings: &MatchBindings,
) -> Result<Vec<MatchedReplacement>, MatchError> {
    let mut replacements = Vec::with_capacity(target.len());

    for item in target {
        let Some(key) = KnowledgeInstructionKey::from_instruction(&item.instruction) else {
            return Err(MatchError::UnsupportedRuleInstruction {
                instruction: item.instruction.to_string(),
            });
        };
        let qubits = item
            .qubits
            .iter()
            .map(|rule_qubit| {
                bindings
                    .qubit(*rule_qubit)
                    .ok_or(MatchError::UnboundRewriteQubit { qubit: *rule_qubit })
            })
            .collect::<Result<SmallVec<[_; 3]>, _>>()?;
        let params = item
            .params
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|value| instantiate_parameter(value, bindings))
            .collect::<Result<SmallVec<[_; 3]>, _>>()?;

        replacements.push(MatchedReplacement {
            instruction: item.instruction.clone(),
            qubits,
            params,
            key,
        });
    }

    Ok(replacements)
}

/// Matches a rule against an adjacent sequence of concrete operations.
///
/// This helper does not search across intervening operations. Callers that need
/// commutation-aware or cost-aware matching should build that policy on top of
/// `match_rule_item`.
pub fn rule_matches_operations(
    rule: &Rule,
    operations: &[ConcreteOperationView<'_>],
) -> Result<Option<MatchBindings>, MatchError> {
    if rule.operations.len() != operations.len() {
        return Ok(None);
    }

    let mut bindings = MatchBindings::new();
    for (item, operation) in rule.operations.iter().zip(operations) {
        if !match_rule_item(item, *operation, &mut bindings)? {
            return Ok(None);
        }
    }

    if !conditions_hold(rule.conditions.as_deref(), &bindings) {
        return Ok(None);
    }

    Ok(Some(bindings))
}

fn condition_symbols_bound(condition: &Condition, bindings: &MatchBindings) -> bool {
    match condition {
        Condition::Eq(lhs, rhs) => symbols_bound(lhs, bindings) && symbols_bound(rhs, bindings),
        Condition::EqMod(lhs, rhs, modulus) => {
            symbols_bound(lhs, bindings)
                && symbols_bound(rhs, bindings)
                && symbols_bound(modulus, bindings)
        }
    }
}

fn symbols_bound(parameter: &Parameter, bindings: &MatchBindings) -> bool {
    parameter
        .get_symbols()
        .iter()
        .all(|symbol| bindings.params.contains_key(symbol))
}

fn instantiate_parameter(
    value: &ParameterValue,
    bindings: &MatchBindings,
) -> Result<ParameterValue, MatchError> {
    let parameter = match value {
        ParameterValue::Fixed(value) => Parameter::from(*value),
        ParameterValue::Param(parameter) => {
            for symbol in parameter.get_symbols() {
                if !bindings.params.contains_key(&symbol) {
                    return Err(MatchError::UnboundRewriteSymbol { symbol });
                }
            }
            parameter.substitute_many(&bindings.params)
        }
    };
    Ok(ParameterValue::from(parameter))
}

#[cfg(test)]
#[path = "./matcher_test.rs"]
mod matcher_test;
