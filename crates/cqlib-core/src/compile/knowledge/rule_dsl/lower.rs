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

//! Lowering logic from AST ([`RuleDef`], [`GatePattern`]) to runtime rule structures.

use crate::circuit::gate::StandardGate;
use crate::circuit::{Instruction, MCGate, ParameterValue};
use crate::compile::knowledge::rule::{Rule, RuleItem, RuleValidationError};
use crate::compile::knowledge::rule_dsl::ast::{GatePattern, GateSpec, RuleDef};
use smallvec::SmallVec;
use std::collections::HashSet;

/// Errors that can occur when lowering AST nodes to runtime [`Rule`] structures.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LowerError {
    /// A rule must contain at least one match operation.
    #[error("rule match block is empty")]
    EmptyMatch,
    /// The gate name does not correspond to a known [`StandardGate`].
    #[error("unknown gate: {0}")]
    UnknownGate(String),
    /// A parameter expression could not be parsed or evaluated.
    #[error("invalid expression: {0}")]
    InvalidExpr(String),
    /// A multi-qubit pattern references the same qubit more than once.
    #[error("duplicate qubit {qubit} in gate {gate}")]
    DuplicateQubit {
        /// Gate name used for the error message.
        gate: String,
        /// Duplicated qubit index.
        qubit: u32,
    },
    /// A rewrite or require expression references a symbol not bound by match patterns.
    #[error("unbound symbol {symbol} in {context}")]
    UnboundSymbol {
        /// Unbound symbol name.
        symbol: String,
        /// DSL context where the symbol was found.
        context: &'static str,
    },
    /// A rewrite operation references a qubit label not bound by match patterns.
    #[error("unbound qubit {qubit} in {context}")]
    UnboundQubit {
        /// Unbound rule-local qubit label.
        qubit: u32,
        /// DSL context where the qubit label was found.
        context: &'static str,
    },
    /// The number of qubits does not match the gate definition.
    #[error("wrong qubit count for {gate}: expected {expected}, got {got}")]
    WrongQubitCount {
        /// Gate name used for the error message.
        gate: String,
        /// Expected qubit count.
        expected: usize,
        /// Actual qubit count found in the pattern.
        got: usize,
    },
    /// The number of parameters does not match the gate definition.
    #[error("wrong parameter count for {gate}: expected {expected}, got {got}")]
    WrongParamCount {
        /// Gate name used for the error message.
        gate: String,
        /// Expected parameter count.
        expected: usize,
        /// Actual parameter count found in the pattern.
        got: usize,
    },
    /// A lowered rule violates the runtime rule invariants.
    #[error("invalid lowered rule: {0}")]
    InvalidRule(RuleValidationError),
}

impl GatePattern {
    /// Lowers this surface pattern into a runtime match [`RuleItem`].
    ///
    /// The lowering process:
    /// 1. Resolves the pattern gate to a standard gate or an [`MCGate`].
    /// 2. Validates that `qubits.len()` equals the gate's expected qubit count.
    /// 3. Validates that `params.len()` equals the gate's expected parameter count.
    /// 4. Attempts to evaluate each parameter to a constant; if that fails,
    ///    the parameter is kept as a symbolic [`ParameterValue::Param`].
    pub fn into_pattern_op(self) -> Result<RuleItem, LowerError> {
        let lowered = lower_gate_pattern(self)?;

        Ok(RuleItem {
            instruction: lowered.instruction,
            qubits: lowered.qubits,
            params: if lowered.params.is_empty() {
                None
            } else {
                Some(lowered.params)
            },
        })
    }

    /// Lowers this surface pattern into a runtime rewrite [`RuleItem`].
    ///
    /// The lowering process is identical to [`into_pattern_op`](Self::into_pattern_op)
    /// except it is used for rewrite block diagnostics and validation.
    pub fn into_build_op(self) -> Result<RuleItem, LowerError> {
        let lowered = lower_gate_pattern(self)?;

        Ok(RuleItem {
            instruction: lowered.instruction,
            qubits: lowered.qubits,
            params: if lowered.params.is_empty() {
                None
            } else {
                Some(lowered.params)
            },
        })
    }
}

impl RuleDef {
    /// Lowers this AST rule into the runtime [`Rule`] type.
    ///
    pub fn into_rule(self) -> Result<Rule, LowerError> {
        if self.match_ops.is_empty() {
            return Err(LowerError::EmptyMatch);
        }

        let mut bound_symbols = HashSet::new();
        for op in &self.match_ops {
            collect_bound_symbols(op, &mut bound_symbols);
        }

        let mut bound_qubits = HashSet::new();
        for op in &self.match_ops {
            collect_bound_qubits(op, &mut bound_qubits);
        }

        for op in &self.rewrite_ops {
            validate_gate_symbols(op, &bound_symbols, "rewrite")?;
            validate_gate_qubits(op, &bound_qubits, "rewrite")?;
        }
        for condition in &self.conditions {
            validate_condition_symbols(condition, &bound_symbols)?;
        }

        let name = self.name;
        let operations = self
            .match_ops
            .into_iter()
            .map(GatePattern::into_pattern_op)
            .collect::<Result<SmallVec<[_; 4]>, _>>()?;
        let target = self
            .rewrite_ops
            .into_iter()
            .map(GatePattern::into_build_op)
            .collect::<Result<SmallVec<[_; 4]>, _>>()?;

        let rule = Rule {
            name,
            operations,
            conditions: if self.conditions.is_empty() {
                None
            } else {
                Some(SmallVec::from_vec(self.conditions))
            },
            target,
        };
        rule.validate().map_err(LowerError::InvalidRule)?;
        Ok(rule)
    }
}

struct LoweredGate {
    instruction: Instruction,
    qubits: SmallVec<[u32; 3]>,
    params: SmallVec<[ParameterValue; 1]>,
}

fn lower_gate_pattern(pattern: GatePattern) -> Result<LoweredGate, LowerError> {
    let display_name = pattern.gate.display_name();
    let instruction = lower_gate_spec(pattern.gate)?;
    let (expected_qubits, expected_params) = instruction
        .gate_arity()
        .expect("rule DSL lowering emits only fixed-arity gates");

    if pattern.qubits.len() != expected_qubits {
        return Err(LowerError::WrongQubitCount {
            gate: display_name,
            expected: expected_qubits,
            got: pattern.qubits.len(),
        });
    }
    if pattern.params.len() != expected_params {
        return Err(LowerError::WrongParamCount {
            gate: display_name,
            expected: expected_params,
            got: pattern.params.len(),
        });
    }
    validate_unique_qubits(&display_name, &pattern.qubits)?;

    let params = pattern
        .params
        .into_iter()
        .map(ParameterValue::from)
        .collect();

    Ok(LoweredGate {
        instruction,
        qubits: SmallVec::from_vec(pattern.qubits),
        params,
    })
}

fn lower_gate_spec(gate: GateSpec) -> Result<Instruction, LowerError> {
    match gate {
        GateSpec::Standard { gate_name } => Ok(Instruction::Standard(parse_gate_name(&gate_name)?)),
        GateSpec::MultiControlled {
            base_gate_name,
            added_controls,
        } => {
            let base_gate = parse_gate_name(&base_gate_name)?;
            Ok(Instruction::McGate(Box::new(MCGate::new(
                added_controls,
                base_gate,
            ))))
        }
    }
}

fn validate_unique_qubits(gate: &str, qubits: &[u32]) -> Result<(), LowerError> {
    let mut seen = SmallVec::<[u32; 3]>::new();
    for &qubit in qubits {
        if seen.contains(&qubit) {
            return Err(LowerError::DuplicateQubit {
                gate: gate.to_string(),
                qubit,
            });
        }
        seen.push(qubit);
    }
    Ok(())
}

fn collect_bound_symbols(pattern: &GatePattern, bound_symbols: &mut HashSet<String>) {
    for param in &pattern.params {
        for symbol in param.get_symbols() {
            if !is_builtin_symbol(&symbol) {
                bound_symbols.insert(symbol);
            }
        }
    }
}

fn collect_bound_qubits(pattern: &GatePattern, bound_qubits: &mut HashSet<u32>) {
    bound_qubits.extend(pattern.qubits.iter().copied());
}

fn validate_gate_symbols(
    pattern: &GatePattern,
    bound_symbols: &HashSet<String>,
    context: &'static str,
) -> Result<(), LowerError> {
    for param in &pattern.params {
        validate_param_symbols(param, bound_symbols, context)?;
    }
    Ok(())
}

fn validate_gate_qubits(
    pattern: &GatePattern,
    bound_qubits: &HashSet<u32>,
    context: &'static str,
) -> Result<(), LowerError> {
    for &qubit in &pattern.qubits {
        if !bound_qubits.contains(&qubit) {
            return Err(LowerError::UnboundQubit { qubit, context });
        }
    }
    Ok(())
}

fn validate_condition_symbols(
    condition: &crate::compile::knowledge::rule::Condition,
    bound_symbols: &HashSet<String>,
) -> Result<(), LowerError> {
    match condition {
        crate::compile::knowledge::rule::Condition::Eq(lhs, rhs) => {
            validate_param_symbols(lhs, bound_symbols, "require")?;
            validate_param_symbols(rhs, bound_symbols, "require")
        }
        crate::compile::knowledge::rule::Condition::EqMod(lhs, rhs, modulus) => {
            validate_param_symbols(lhs, bound_symbols, "require")?;
            validate_param_symbols(rhs, bound_symbols, "require")?;
            validate_param_symbols(modulus, bound_symbols, "require")
        }
    }
}

fn validate_param_symbols(
    param: &crate::circuit::Parameter,
    bound_symbols: &HashSet<String>,
    context: &'static str,
) -> Result<(), LowerError> {
    for symbol in param.get_symbols() {
        if !is_builtin_symbol(&symbol) && !bound_symbols.contains(&symbol) {
            return Err(LowerError::UnboundSymbol { symbol, context });
        }
    }
    Ok(())
}

fn is_builtin_symbol(symbol: &str) -> bool {
    matches!(symbol, "π" | "pi" | "e")
}

/// Maps a gate name string to its [`StandardGate`] variant.
fn parse_gate_name(name: &str) -> Result<StandardGate, LowerError> {
    match name {
        "I" => Ok(StandardGate::I),
        "H" => Ok(StandardGate::H),
        "RX" => Ok(StandardGate::RX),
        "RXX" => Ok(StandardGate::RXX),
        "RXY" => Ok(StandardGate::RXY),
        "RY" => Ok(StandardGate::RY),
        "RYY" => Ok(StandardGate::RYY),
        "RZ" => Ok(StandardGate::RZ),
        "RZX" => Ok(StandardGate::RZX),
        "RZZ" => Ok(StandardGate::RZZ),
        "S" => Ok(StandardGate::S),
        "SDG" => Ok(StandardGate::SDG),
        "SWAP" => Ok(StandardGate::SWAP),
        "T" => Ok(StandardGate::T),
        "TDG" => Ok(StandardGate::TDG),
        "U" => Ok(StandardGate::U),
        "X" => Ok(StandardGate::X),
        "XY" => Ok(StandardGate::XY),
        "X2P" => Ok(StandardGate::X2P),
        "X2M" => Ok(StandardGate::X2M),
        "XY2P" => Ok(StandardGate::XY2P),
        "XY2M" => Ok(StandardGate::XY2M),
        "Y" => Ok(StandardGate::Y),
        "Y2P" => Ok(StandardGate::Y2P),
        "Y2M" => Ok(StandardGate::Y2M),
        "Z" => Ok(StandardGate::Z),
        "Phase" => Ok(StandardGate::Phase),
        "GPhase" => Ok(StandardGate::GPhase),
        "CX" => Ok(StandardGate::CX),
        "CCX" => Ok(StandardGate::CCX),
        "CY" => Ok(StandardGate::CY),
        "CZ" => Ok(StandardGate::CZ),
        "CRX" => Ok(StandardGate::CRX),
        "CRY" => Ok(StandardGate::CRY),
        "CRZ" => Ok(StandardGate::CRZ),
        "FSIM" => Ok(StandardGate::FSIM),
        _ => Err(LowerError::UnknownGate(name.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::knowledge::rule_dsl::parser::Parser;

    fn lower_single_rule(source: &str) -> Result<Rule, LowerError> {
        let mut parser = Parser::new(source).unwrap();
        let mut rules = parser.parse_rule_file().unwrap();
        assert_eq!(rules.len(), 1);
        rules.remove(0).into_rule()
    }

    #[test]
    fn reject_empty_match_block() {
        let err = lower_single_rule(
            r#"rule bad {
                match {}
                rewrite { H 0 }
            }"#,
        )
        .unwrap_err();
        assert!(matches!(err, LowerError::EmptyMatch));
    }

    #[test]
    fn reject_duplicate_qubits() {
        let err = lower_single_rule(
            r#"rule bad {
                match { CX 0 0 }
                rewrite {}
            }"#,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            LowerError::DuplicateQubit { gate, qubit } if gate == "CX" && qubit == 0
        ));
    }

    #[test]
    fn reject_unbound_rewrite_symbol() {
        let err = lower_single_rule(
            r#"rule bad {
                match { RZ(a) 0 }
                rewrite { RZ(c) 0 }
            }"#,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            LowerError::UnboundSymbol { symbol, context } if symbol == "c" && context == "rewrite"
        ));
    }

    #[test]
    fn reject_unbound_rewrite_qubit() {
        let err = lower_single_rule(
            r#"rule bad {
                match { H 0 }
                rewrite { H 1 }
            }"#,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            LowerError::UnboundQubit { qubit, context } if qubit == 1 && context == "rewrite"
        ));
    }

    #[test]
    fn allow_rewrite_to_reuse_match_qubit() {
        let rule = lower_single_rule(
            r#"rule ok {
                match { H 0 }
                rewrite { X 0 }
            }"#,
        )
        .unwrap();
        assert_eq!(rule.target[0].qubits.as_slice(), &[0]);
    }

    #[test]
    fn allow_rewrite_to_reorder_bound_match_qubits() {
        let rule = lower_single_rule(
            r#"rule ok {
                match { CX 0 1 }
                rewrite { CX 1 0 }
            }"#,
        )
        .unwrap();
        assert_eq!(rule.target[0].qubits.as_slice(), &[1, 0]);
    }

    #[test]
    fn lower_gphase_rule_ok() {
        let rule = lower_single_rule(
            r#"rule merge_gphase {
                match { GPhase(a), GPhase(b) }
                rewrite { GPhase(a + b) }
            }"#,
        )
        .unwrap();
        assert_eq!(rule.name, "merge_gphase");
        assert_eq!(rule.operations.len(), 2);
        assert!(rule.operations[0].qubits.is_empty());
        assert_eq!(rule.target.len(), 1);
        assert!(rule.target[0].qubits.is_empty());
    }

    #[test]
    fn lower_multi_controlled_gate_rule_ok() {
        let rule = lower_single_rule(
            r#"rule decompose_m3cx {
                match { MCX[3] 0 1 2 3 }
                rewrite { CCX 0 1 2 }
            }"#,
        )
        .unwrap();

        let Instruction::McGate(gate) = &rule.operations[0].instruction else {
            panic!("expected MCGate");
        };
        assert_eq!(gate.num_ctrl_qubits(), 3);
        assert_eq!(*gate.base_gate(), StandardGate::X);
        assert_eq!(rule.operations[0].qubits.as_slice(), &[0, 1, 2, 3]);
    }

    #[test]
    fn lower_parameterized_multi_controlled_gate_rule_ok() {
        let rule = lower_single_rule(
            r#"rule decompose_m2rz {
                match { MCRZ[2](theta) 0 1 2 }
                rewrite { CRZ(theta) 1 2 }
            }"#,
        )
        .unwrap();

        let Instruction::McGate(gate) = &rule.operations[0].instruction else {
            panic!("expected MCGate");
        };
        assert_eq!(gate.num_ctrl_qubits(), 2);
        assert_eq!(*gate.base_gate(), StandardGate::RZ);
        assert_eq!(rule.operations[0].params.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn reject_multi_controlled_gate_wrong_qubit_count() {
        let err = lower_single_rule(
            r#"rule bad {
                match { MCX[3] 0 1 2 }
                rewrite {}
            }"#,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            LowerError::WrongQubitCount { gate, expected: 4, got: 3 } if gate == "MCX[3]"
        ));
    }

    #[test]
    fn reject_multi_controlled_gate_wrong_param_count() {
        let err = lower_single_rule(
            r#"rule bad {
                match { MCRZ[2] 0 1 2 }
                rewrite {}
            }"#,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            LowerError::WrongParamCount { gate, expected: 1, got: 0 } if gate == "MCRZ[2]"
        ));
    }

    #[test]
    fn reject_multi_controlled_gate_unknown_base_gate() {
        let err = lower_single_rule(
            r#"rule bad {
                match { MCNOPE[1] 0 1 }
                rewrite {}
            }"#,
        )
        .unwrap_err();
        assert!(matches!(err, LowerError::UnknownGate(gate) if gate == "NOPE"));
    }

    #[test]
    fn reject_gphase_with_qubit() {
        let err = lower_single_rule(
            r#"rule bad {
                match { GPhase(a) 0 }
                rewrite {}
            }"#,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            LowerError::WrongQubitCount { gate, expected: 0, got: 1 } if gate == "GPhase"
        ));
    }

    #[test]
    fn reject_unbound_condition_symbol() {
        let err = lower_single_rule(
            r#"rule bad {
                match { RZ(a) 0 }
                require { c == 0 }
                rewrite {}
            }"#,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            LowerError::UnboundSymbol { symbol, context } if symbol == "c" && context == "require"
        ));
    }
}
