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

//! Equivalence validation for knowledge-base rewrite rules.
//!
//! Rule definitions live in [`super::rule`].  This module validates a rule by
//! comparing the symbolic unitary matrices of its left- and right-hand sides,
//! with numerical sampling as a fallback for identities the symbolic
//! simplifier cannot prove directly.

use crate::circuit::error::{CircuitError, ParameterError};
use crate::circuit::symbolic_matrix::matrix::simplify_matrix;
use crate::circuit::symbolic_matrix::{
    SymbolicMatrix, circuit_to_symbolic_matrix, evaluate_symbolic_matrix,
    symbolic_matrices_equivalent,
};
use crate::circuit::{Circuit, Instruction, Parameter, ParameterValue, Qubit, StandardGate};
use crate::compile::knowledge::rule::{Condition, Rule, RuleItem};
use crate::compile::{NUMERIC_ZERO_TOLERANCE, UNIT_PHASE_NORM_TOLERANCE};
use ndarray::parallel::prelude::*;
use num_complex::Complex64;
use rand::Rng;
use smallvec::SmallVec;
use std::collections::HashMap;

const DETERMINISTIC_NUMERIC_TOLERANCE: f64 = 1e-12;
const RULE_CONDITION_TOLERANCE: f64 = 1e-8;
const DEFAULT_VERIFY_NUM_BINDINGS: usize = 16;
const DEFAULT_VERIFY_SAMPLING_TOLERANCE: f64 = 1e-8;
const ENV_VERIFY_BINDINGS: &str = "CQLIB_RULE_VERIFY_BINDINGS";
const ENV_VERIFY_TOLERANCE: &str = "CQLIB_RULE_VERIFY_TOLERANCE";

/// Number of parameter bindings for layered / sampling rule verification.
///
/// Override at runtime with `CQLIB_RULE_VERIFY_BINDINGS` (e.g. `128` for nightly deep checks).
pub fn verify_sampling_bindings() -> usize {
    std::env::var(ENV_VERIFY_BINDINGS)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|count| *count > 0)
        .unwrap_or(DEFAULT_VERIFY_NUM_BINDINGS)
}

/// Numerical tolerance for sampling-based rule verification.
///
/// Override with `CQLIB_RULE_VERIFY_TOLERANCE`.
pub fn verify_sampling_tolerance() -> f64 {
    std::env::var(ENV_VERIFY_TOLERANCE)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|tolerance: &f64| *tolerance > 0.0 && tolerance.is_finite())
        .unwrap_or(DEFAULT_VERIFY_SAMPLING_TOLERANCE)
}

/// Result of layered rule equivalence verification via [`Rule::verify`].
///
/// Taxonomy for rule-library health checks:
///
/// | Outcome | Meaning |
/// |---------|---------|
/// | [`VerifyResult::Equivalent`] | Symbolic or fixed-parameter numeric matrix comparison proved equivalence |
/// | [`VerifyResult::SampledEqual`] | Symbolic verification was insufficient, but parameter sampling passed |
/// | [`VerifyResult::NotEquivalent`] | Symbolic and sampling both failed — suspected rule bug |
/// | [`VerifyResult::Inconclusive`] | Could not generate parameter bindings satisfying `require` conditions |
///
/// Bare symbolic comparison alone is not sufficient for parametric rules, rules with
/// `require` / `mod` conditions, or many commutation rules. [`Rule::verify`] falls back
/// to sampling automatically when those features are present.
#[derive(Debug)]
pub enum VerifyResult {
    /// The symbolic verifier proved the rule equivalent up to global phase.
    Equivalent,
    /// Symbolic matrices differ structurally but are numerically equal at all
    /// sampled parameter bindings.
    SampledEqual { num_bindings: usize },
    /// The verifier did not prove equivalence.
    NotEquivalent,
    /// Could not verify (e.g., cannot generate satisfying bindings).
    Inconclusive { reason: String },
}

impl VerifyResult {
    /// Returns whether equivalence verification succeeded.
    pub fn is_verified(&self) -> bool {
        matches!(self, Self::Equivalent | Self::SampledEqual { .. })
    }
}

/// Errors during verification setup (not verification failure).
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("unsupported instruction pattern: {0}")]
    UnsupportedPattern(String),
    #[error("parameter error: {0}")]
    Parameter(#[from] ParameterError),
    #[error("circuit error: {0}")]
    Circuit(#[from] CircuitError),
}

impl Rule {
    /// Verify this rule with layered equivalence checking:
    ///
    /// 1. Symbolic matrix comparison (and numeric check for fixed-parameter rules)
    /// 2. If that fails and the rule has free parameters or `require` conditions,
    ///    fall back to [`Self::verify_by_sampling`] with default binding count and tolerance
    pub fn verify(&self) -> Result<VerifyResult, VerifyError> {
        let (lhs, rhs) = build_simplified_matrices(self)?;
        layered_verify(
            self,
            &lhs,
            &rhs,
            verify_sampling_bindings(),
            verify_sampling_tolerance(),
        )
    }

    /// Verify this rule symbolically first, then fall back to numerical sampling.
    pub fn verify_by_sampling(
        &self,
        num_bindings: usize,
        tolerance: f64,
    ) -> Result<VerifyResult, VerifyError> {
        let (lhs, rhs) = build_simplified_matrices(self)?;
        layered_verify(self, &lhs, &rhs, num_bindings, tolerance)
    }

    /// Whether layered verification should fall back to parameter sampling after
    /// symbolic comparison fails.
    pub fn needs_sampling_fallback(&self) -> bool {
        !self.collect_free_symbols().is_empty() || self.conditions.is_some()
    }
}

fn layered_verify(
    rule: &Rule,
    lhs: &SymbolicMatrix,
    rhs: &SymbolicMatrix,
    num_bindings: usize,
    tolerance: f64,
) -> Result<VerifyResult, VerifyError> {
    if symbolic_matrices_equivalent(lhs, rhs)? {
        return Ok(VerifyResult::Equivalent);
    }

    if !rule.needs_sampling_fallback() {
        let (lhs_num, rhs_num) = rayon::join(
            || evaluate_symbolic_matrix(lhs, &None),
            || evaluate_symbolic_matrix(rhs, &None),
        );
        let lhs_num = lhs_num?;
        let rhs_num = rhs_num?;
        if max_diff_up_to_global_phase(&lhs_num, &rhs_num) < DETERMINISTIC_NUMERIC_TOLERANCE {
            return Ok(VerifyResult::Equivalent);
        }
        return Ok(VerifyResult::NotEquivalent);
    }

    verify_by_sampling(rule, lhs, rhs, num_bindings, tolerance)
}

fn build_simplified_matrices(rule: &Rule) -> Result<(SymbolicMatrix, SymbolicMatrix), VerifyError> {
    let num_qubits = rule.num_qubits();

    let (lhs, rhs) = rayon::join(
        || rule_items_to_circuit(&rule.operations, num_qubits),
        || rule_items_to_circuit(&rule.target, num_qubits),
    );
    let lhs = lhs?;
    let rhs = rhs?;

    let (lhs, rhs) = rayon::join(
        || circuit_to_symbolic_matrix(&lhs, None),
        || circuit_to_symbolic_matrix(&rhs, None),
    );
    let lhs = lhs?;
    let rhs = rhs?;

    let (lhs, rhs) = rayon::join(|| simplify_matrix(&lhs), || simplify_matrix(&rhs));
    let lhs = lhs?;
    let rhs = rhs?;

    Ok((lhs, rhs))
}

fn verify_by_sampling(
    rule: &Rule,
    lhs: &SymbolicMatrix,
    rhs: &SymbolicMatrix,
    num_bindings: usize,
    tolerance: f64,
) -> Result<VerifyResult, VerifyError> {
    let free_symbols = rule.collect_free_symbols();
    if free_symbols.is_empty() {
        let (lhs_num, rhs_num) = rayon::join(
            || evaluate_symbolic_matrix(lhs, &None),
            || evaluate_symbolic_matrix(rhs, &None),
        );
        let lhs_num = lhs_num?;
        let rhs_num = rhs_num?;
        let diff = max_diff_up_to_global_phase(&lhs_num, &rhs_num);
        if diff < tolerance {
            return Ok(VerifyResult::SampledEqual { num_bindings: 1 });
        }
        return Ok(VerifyResult::NotEquivalent);
    }

    let symbol_names: Vec<&str> = free_symbols.iter().map(|s| s.as_str()).collect();
    let bindings =
        generate_satisfying_bindings(&symbol_names, rule.conditions.as_deref(), num_bindings);

    if bindings.is_empty() {
        return Ok(VerifyResult::Inconclusive {
            reason: "could not generate parameter bindings satisfying conditions".to_string(),
        });
    }

    for binding in &bindings {
        let bindings_ref: Option<HashMap<&str, f64>> =
            Some(binding.iter().map(|(k, &v)| (k.as_str(), v)).collect());
        let (lhs_num, rhs_num) = rayon::join(
            || evaluate_symbolic_matrix(lhs, &bindings_ref),
            || evaluate_symbolic_matrix(rhs, &bindings_ref),
        );
        let lhs_num = lhs_num?;
        let rhs_num = rhs_num?;
        let diff = max_diff_up_to_global_phase(&lhs_num, &rhs_num);
        if diff >= tolerance {
            return Ok(VerifyResult::NotEquivalent);
        }
    }

    Ok(VerifyResult::SampledEqual {
        num_bindings: bindings.len(),
    })
}

fn rule_items_to_circuit(ops: &[RuleItem], num_qubits: usize) -> Result<Circuit, VerifyError> {
    let mut circuit = Circuit::new(num_qubits);

    for op in ops {
        let instruction = match &op.instruction {
            Instruction::Standard(gate) => Instruction::Standard(*gate),
            Instruction::McGate(gate) => Instruction::McGate(gate.clone()),
            other => {
                return Err(VerifyError::UnsupportedPattern(other.to_string()));
            }
        };

        if matches!(&instruction, Instruction::Standard(StandardGate::GPhase)) {
            let actual = op.params.as_ref().map_or(0, SmallVec::len);
            if actual != 1 {
                return Err(CircuitError::ParameterCountMismatch {
                    expected: 1,
                    actual,
                }
                .into());
            }
            let theta = op
                .params
                .as_ref()
                .and_then(|params| params.first())
                .expect("GPhase parameter count was checked");
            let phase = match theta {
                ParameterValue::Param(param) => param.clone(),
                ParameterValue::Fixed(value) => Parameter::from(*value),
            };
            circuit.set_global_phase(circuit.global_phase() + phase);
            continue;
        }

        let qubits: SmallVec<[Qubit; 3]> =
            op.qubits.iter().map(|&qubit| Qubit::new(qubit)).collect();
        let params = op.params.clone().unwrap_or_default();
        circuit.append(instruction, qubits, params, None)?;
    }

    Ok(circuit)
}

/// Compute the maximum element-wise difference between two numerical matrices.
fn max_diff_strict(lhs: &ndarray::Array2<Complex64>, rhs: &ndarray::Array2<Complex64>) -> f64 {
    (0..lhs.len())
        .into_par_iter()
        .map(|i| {
            let l = lhs.as_slice().unwrap()[i];
            let r = rhs.as_slice().unwrap()[i];
            (l - r).norm()
        })
        .fold(|| 0.0_f64, f64::max)
        .reduce(|| 0.0_f64, f64::max)
}

/// Compute the maximum element-wise difference between two numerical matrices.
fn max_diff_up_to_global_phase(
    lhs: &ndarray::Array2<Complex64>,
    rhs: &ndarray::Array2<Complex64>,
) -> f64 {
    let mut phase_ratio = None;

    for (&l, &r) in lhs.iter().zip(rhs.iter()) {
        let l_zero = l.norm() <= NUMERIC_ZERO_TOLERANCE;
        let r_zero = r.norm() <= NUMERIC_ZERO_TOLERANCE;

        if l_zero != r_zero {
            return max_diff_strict(lhs, rhs);
        }

        if !l_zero {
            let ratio = r / l;
            let ratio_norm = ratio.norm();

            if !ratio_norm.is_finite() || (ratio_norm - 1.0).abs() > UNIT_PHASE_NORM_TOLERANCE {
                return max_diff_strict(lhs, rhs);
            }

            phase_ratio = Some(ratio);
            break;
        }
    }

    let Some(phase_ratio) = phase_ratio else {
        return max_diff_strict(lhs, rhs);
    };

    let lhs_slice = lhs.as_slice().expect("lhs matrix must be contiguous");
    let rhs_slice = rhs.as_slice().expect("rhs matrix must be contiguous");

    (0..lhs_slice.len())
        .into_par_iter()
        .map(|i| {
            let l = lhs_slice[i];
            let r = rhs_slice[i];

            // phase_ratio = rhs / lhs, so rhs / phase_ratio aligns with lhs.
            let normalized_r = r / phase_ratio;
            (l - normalized_r).norm()
        })
        .fold(|| 0.0_f64, f64::max)
        .reduce(|| 0.0_f64, f64::max)
}

/// Generate parameter bindings that satisfy the rule's conditions.
fn generate_satisfying_bindings(
    symbols: &[&str],
    conditions: Option<&[Condition]>,
    num_bindings: usize,
) -> Vec<HashMap<String, f64>> {
    let mut result = Vec::new();
    let mut rng = rand::rng();
    let two_pi = 2.0 * std::f64::consts::PI;

    for _ in 0..num_bindings {
        let mut bindings: HashMap<String, f64> = HashMap::new();
        for &sym in symbols {
            let val = rng.random_range(-two_pi..two_pi);
            bindings.insert(sym.to_string(), val);
        }

        if let Some(conds) = conditions {
            for cond in conds {
                adjust_binding_for_condition(&mut bindings, cond, symbols);
            }
        }

        if let Some(conds) = conditions {
            let bindings_ref: Option<HashMap<&str, f64>> =
                Some(bindings.iter().map(|(k, &v)| (k.as_str(), v)).collect());
            let all_satisfied = conds.iter().all(|c| match c {
                Condition::Eq(a, b) => {
                    if let (Ok(va), Ok(vb)) = (a.evaluate(&bindings_ref), b.evaluate(&bindings_ref))
                    {
                        (va - vb).abs() < RULE_CONDITION_TOLERANCE
                    } else {
                        false
                    }
                }
                Condition::EqMod(a, b, m) => {
                    if let (Ok(va), Ok(vb), Ok(vm)) = (
                        a.evaluate(&bindings_ref),
                        b.evaluate(&bindings_ref),
                        m.evaluate(&bindings_ref),
                    ) {
                        if vm.abs() < NUMERIC_ZERO_TOLERANCE {
                            false
                        } else {
                            let remainder = (va - vb) % vm;
                            remainder.abs() < RULE_CONDITION_TOLERANCE
                                || (vm - remainder).abs() < RULE_CONDITION_TOLERANCE
                        }
                    } else {
                        false
                    }
                }
            });
            if !all_satisfied {
                continue;
            }
        }

        result.push(bindings);
    }

    result
}

/// Adjust one binding to satisfy a condition.
fn adjust_binding_for_condition(
    bindings: &mut HashMap<String, f64>,
    condition: &Condition,
    symbols: &[&str],
) {
    match condition {
        Condition::Eq(lhs, rhs) => {
            adjust_for_equality(bindings, lhs, rhs, None, symbols);
        }
        Condition::EqMod(lhs, rhs, modulus) => {
            if let Ok(mod_val) = modulus.evaluate(&Some(
                bindings.iter().map(|(k, &v)| (k.as_str(), v)).collect(),
            )) && mod_val.abs() > NUMERIC_ZERO_TOLERANCE
            {
                adjust_for_equality(bindings, lhs, rhs, Some(mod_val), symbols);
            }
        }
    }
}

/// Adjust one binding so that `lhs == rhs`, or `lhs == rhs mod modulus`.
fn adjust_for_equality(
    bindings: &mut HashMap<String, f64>,
    lhs: &Parameter,
    rhs: &Parameter,
    modulus: Option<f64>,
    symbols: &[&str],
) {
    if try_adjust_symbol(bindings, rhs, lhs, modulus, symbols) {
        return;
    }
    try_adjust_symbol(bindings, lhs, rhs, modulus, symbols);
}

/// Try to adjust one symbol in `expr` so that `expr == target mod modulus`.
fn try_adjust_symbol(
    bindings: &mut HashMap<String, f64>,
    expr: &Parameter,
    target: &Parameter,
    modulus: Option<f64>,
    symbols: &[&str],
) -> bool {
    let expr_syms = expr.get_symbols();
    let target_sym = symbols.iter().find(|&&s| expr_syms.contains(s));

    let Some(&sym) = target_sym else {
        return false;
    };

    let bindings_ref: Option<HashMap<&str, f64>> =
        Some(bindings.iter().map(|(k, &v)| (k.as_str(), v)).collect());

    let Ok(target_val) = target.evaluate(&bindings_ref) else {
        return false;
    };

    let bindings_without: Option<HashMap<&str, f64>> = Some(
        bindings
            .iter()
            .map(|(k, &v)| {
                if k == sym {
                    (k.as_str(), 0.0)
                } else {
                    (k.as_str(), v)
                }
            })
            .collect(),
    );
    let bindings_one: Option<HashMap<&str, f64>> = Some(
        bindings
            .iter()
            .map(|(k, &v)| {
                if k == sym {
                    (k.as_str(), 1.0)
                } else {
                    (k.as_str(), v)
                }
            })
            .collect(),
    );
    let (Ok(expr_zero), Ok(expr_one)) = (
        expr.evaluate(&bindings_without),
        expr.evaluate(&bindings_one),
    ) else {
        return false;
    };

    let coeff = expr_one - expr_zero;
    if coeff.abs() < NUMERIC_ZERO_TOLERANCE {
        return false;
    }

    let current_sym_val = bindings.get(sym).copied().unwrap_or(0.0);

    if let Some(mod_val) = modulus {
        let sym_for_k0 = (target_val - expr_zero) / coeff;
        let k = ((current_sym_val - sym_for_k0) / mod_val).round();
        let adjusted = sym_for_k0 + k * mod_val;
        bindings.insert(sym.to_string(), adjusted);
    } else {
        let new_val = (target_val - expr_zero) / coeff;
        bindings.insert(sym.to_string(), new_val);
    }

    true
}

#[cfg(test)]
#[path = "rule_equivalence_test.rs"]
mod rule_equivalence_test;
