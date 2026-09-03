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

use super::*;
use crate::circuit::{Directive, StandardGate};
use crate::compile::knowledge::RuleLibrary;
use crate::compile::knowledge::rule_dsl::load::load_rules_from_str;
use ndarray::arr2;
use smallvec::smallvec;
use std::f64::consts::PI;

fn assert_verify_passed(result: VerifyResult) {
    assert!(
        result.is_verified(),
        "expected verification to pass, got {result:?}"
    );
}

#[test]
fn verify_result_classifies_successful_outcomes() {
    assert!(VerifyResult::Equivalent.is_verified());
    assert!(VerifyResult::SampledEqual { num_bindings: 3 }.is_verified());
    assert!(!VerifyResult::NotEquivalent.is_verified());
    assert!(
        !VerifyResult::Inconclusive {
            reason: "no bindings".to_string(),
        }
        .is_verified()
    );
}

#[test]
fn verify_accepts_cancel_h_up_to_global_phase() {
    assert_verify_passed(
        Rule::new(
            "cancel_h",
            vec![
                RuleItem::standard(StandardGate::H, &[0], vec![]),
                RuleItem::standard(StandardGate::H, &[0], vec![]),
            ],
            vec![],
        )
        .verify()
        .unwrap(),
    );
}

#[test]
fn verify_accepts_symbolic_merge_rz() {
    let rule = Rule::new(
        "merge_rz",
        vec![
            RuleItem::standard(StandardGate::RZ, &[0], vec![ParameterValue::from("a")]),
            RuleItem::standard(StandardGate::RZ, &[0], vec![ParameterValue::from("b")]),
        ],
        vec![RuleItem::standard(
            StandardGate::RZ,
            &[0],
            vec![ParameterValue::from(
                Parameter::symbol("a") + Parameter::symbol("b"),
            )],
        )],
    );

    assert_verify_passed(rule.verify().unwrap());
    assert!(matches!(
        rule.verify().unwrap(),
        VerifyResult::SampledEqual { .. }
    ));
}

#[test]
fn verify_rejects_wrong_cancel_h_rewrite() {
    let rule = Rule::new(
        "bad_cancel_h",
        vec![
            RuleItem::standard(StandardGate::H, &[0], vec![]),
            RuleItem::standard(StandardGate::H, &[0], vec![]),
        ],
        vec![RuleItem::standard(StandardGate::H, &[0], vec![])],
    );

    assert!(matches!(
        rule.verify().unwrap(),
        VerifyResult::NotEquivalent
    ));
}

#[test]
fn verify_reports_numeric_failure_for_constant_mismatch() {
    let rule = Rule::new(
        "bad_rz_constant",
        vec![RuleItem::standard(
            StandardGate::RZ,
            &[0],
            vec![ParameterValue::from(0.0)],
        )],
        vec![RuleItem::standard(
            StandardGate::RZ,
            &[0],
            vec![ParameterValue::from(0.5)],
        )],
    );

    assert!(matches!(
        rule.verify().unwrap(),
        VerifyResult::NotEquivalent
    ));
}

#[test]
fn verify_returns_unsupported_pattern_for_non_standard_instruction() {
    let rule = Rule::new(
        "bad_instruction",
        vec![RuleItem {
            instruction: Instruction::Directive(Directive::Barrier),
            qubits: smallvec![0],
            params: None,
        }],
        vec![],
    );

    assert!(matches!(
        rule.verify(),
        Err(VerifyError::UnsupportedPattern(_))
    ));
}

#[test]
fn verify_accepts_rx_pi_to_x_up_to_global_phase() {
    assert_verify_passed(
        Rule::new(
            "rx_pi_to_x",
            vec![RuleItem::standard(
                StandardGate::RX,
                &[0],
                vec![ParameterValue::from(PI)],
            )],
            vec![RuleItem::standard(StandardGate::X, &[0], vec![])],
        )
        .verify()
        .unwrap(),
    );
}

#[test]
fn verify_accepts_conditioned_rx_inverse_eq_mod() {
    let mut rule = Rule::new(
        "cancel_rx_inverse",
        vec![
            RuleItem::standard(StandardGate::RX, &[0], vec![ParameterValue::from("a")]),
            RuleItem::standard(StandardGate::RX, &[0], vec![ParameterValue::from("b")]),
        ],
        vec![],
    );
    rule.conditions = Some(smallvec![Condition::EqMod(
        Parameter::symbol("a") + Parameter::symbol("b"),
        Parameter::from(0.0),
        Parameter::from(4.0 * PI),
    )]);

    assert_verify_passed(rule.verify().unwrap());
}

#[test]
fn verify_accepts_conditioned_eq_rule_via_layered_fallback() {
    let mut rule = Rule::new(
        "conditioned_equal_rz",
        vec![RuleItem::standard(
            StandardGate::RZ,
            &[0],
            vec![ParameterValue::from("a")],
        )],
        vec![RuleItem::standard(
            StandardGate::RZ,
            &[0],
            vec![ParameterValue::from("b")],
        )],
    );
    rule.conditions = Some(smallvec![Condition::Eq(
        Parameter::symbol("a"),
        Parameter::symbol("b"),
    )]);

    assert_verify_passed(rule.verify().unwrap());
}

#[test]
fn verify_accepts_multi_controlled_rule_file_by_sampling() {
    let rules = load_rules_from_str(include_str!("rules/decompose_mc_gate.rule")).unwrap();

    for rule in rules {
        assert_verify_passed(rule.verify().unwrap());
    }
}

#[test]
fn verify_returns_inconclusive_when_no_satisfying_bindings_requested() {
    let mut rule = Rule::new(
        "no_bindings",
        vec![RuleItem::standard(
            StandardGate::RZ,
            &[0],
            vec![ParameterValue::from("a")],
        )],
        vec![RuleItem::standard(
            StandardGate::RZ,
            &[0],
            vec![ParameterValue::from("b")],
        )],
    );
    rule.conditions = Some(smallvec![Condition::Eq(
        Parameter::symbol("a"),
        Parameter::symbol("b"),
    )]);

    match rule.verify_by_sampling(0, 1e-8).unwrap() {
        VerifyResult::Inconclusive { reason } => {
            assert!(reason.contains("could not generate parameter bindings"));
        }
        other => panic!("expected inconclusive result, got {other:?}"),
    }
}

#[test]
fn max_diff_up_to_global_phase_falls_back_to_strict_for_invalid_phase_ratio() {
    let lhs = arr2(&[
        [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
        [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
    ]);
    let rhs = arr2(&[
        [Complex64::new(2.0, 0.0), Complex64::new(0.0, 0.0)],
        [Complex64::new(0.0, 0.0), Complex64::new(2.0, 0.0)],
    ]);

    let strict = max_diff_strict(&lhs, &rhs);
    let phase = max_diff_up_to_global_phase(&lhs, &rhs);

    assert!((phase - strict).abs() < 1e-12, "expected strict fallback");
}

#[test]
fn max_diff_up_to_global_phase_falls_back_to_strict_for_zero_structure_mismatch() {
    let lhs = arr2(&[
        [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
        [Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)],
    ]);
    let rhs = arr2(&[
        [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
        [Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)],
    ]);

    let strict = max_diff_strict(&lhs, &rhs);
    let phase = max_diff_up_to_global_phase(&lhs, &rhs);

    assert!(phase.is_finite(), "expected a finite diff");
    assert!((phase - strict).abs() < 1e-12, "expected strict fallback");
}

#[test]
fn max_diff_up_to_global_phase_ignores_unit_phase() {
    let lhs = arr2(&[
        [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
        [Complex64::new(0.0, 0.0), Complex64::new(-1.0, 0.0)],
    ]);
    let rhs = lhs.mapv(|value| Complex64::new(0.0, 1.0) * value);

    assert!(max_diff_up_to_global_phase(&lhs, &rhs) < 1e-12);
}

#[test]
fn max_diff_strict_detects_global_phase_difference() {
    let lhs = arr2(&[
        [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
        [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
    ]);
    let rhs = lhs.mapv(|value| Complex64::new(0.0, 1.0) * value);

    assert!(max_diff_strict(&lhs, &rhs) > 1.0);
}

/// Layered equivalence check for every rule in the builtin [`RuleLibrary`].
fn verify_all_builtin_rules_layered_equivalence() {
    let library = RuleLibrary::builtin_rules().unwrap();
    let mut failures = Vec::new();

    for rule in library.rules() {
        if let Err(err) = rule.validate() {
            failures.push(format!("{}: validate failed: {err}", rule.name));
            continue;
        }

        match rule.verify() {
            Ok(VerifyResult::Equivalent | VerifyResult::SampledEqual { .. }) => {}
            Ok(VerifyResult::NotEquivalent) => {
                failures.push(format!("{}: NotEquivalent", rule.name));
            }
            Ok(VerifyResult::Inconclusive { reason }) => {
                failures.push(format!("{}: Inconclusive: {reason}", rule.name));
            }
            Err(err) => {
                failures.push(format!("{}: verify setup error: {err}", rule.name));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "builtin rule equivalence failures (bindings={}, tolerance={}): {failures:?}",
        verify_sampling_bindings(),
        verify_sampling_tolerance()
    );
}

#[test]
#[ignore = "single-rule timing probe; cargo test -p cqlib-core qcis_fsim_to_cz_verify_timing -- --ignored --nocapture"]
fn qcis_fsim_to_cz_verify_timing() {
    let library = RuleLibrary::builtin_rules().unwrap();
    let rule = library
        .get_by_name("qcis_fsim_to_cz")
        .expect("qcis_fsim_to_cz");

    let validate_start = std::time::Instant::now();
    rule.validate().expect("validate");
    let validate_elapsed = validate_start.elapsed();

    let verify_start = std::time::Instant::now();
    let result = rule.verify();
    let verify_elapsed = verify_start.elapsed();

    println!(
        "qcis_fsim_to_cz: validate={validate_elapsed:?}, verify={verify_elapsed:?}, total={:?}",
        validate_elapsed + verify_elapsed
    );
    println!("qcis_fsim_to_cz verify result: {result:?}");
    println!(
        "bindings={}, tolerance={}",
        verify_sampling_bindings(),
        verify_sampling_tolerance()
    );
}

#[test]
#[ignore = "full builtin rule equivalence sweep; cargo test -p cqlib-core verify_all_builtin_rules_layered_equivalence -- --ignored --nocapture"]
fn verify_all_builtin_rules_layered_equivalence_ignored() {
    verify_all_builtin_rules_layered_equivalence();
}
