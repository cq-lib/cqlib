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
use crate::compile::knowledge::rule::Condition;

#[test]
fn load_rules_from_str_ok() {
    let source = r#"
            rule merge_rz {
                match {
                    RZ(a) 0
                    RZ(b) 0
                }
                rewrite {
                    RZ(a + b) 0
                }
            }
        "#;
    let rules = load_rules_from_str(source).unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "merge_rz");
    assert_eq!(rules[0].operations.len(), 2);
    assert_eq!(rules[0].target.len(), 1);
}

#[test]
fn load_rule_defs_from_str_ok() {
    let source = r#"
            rule cancel_h {
                match { H 0
                H 0 }
                rewrite {}
            }
        "#;
    let defs = load_rule_defs_from_str(source).unwrap();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].name, "cancel_h");
    assert_eq!(defs[0].match_ops.len(), 2);
    assert!(defs[0].rewrite_ops.is_empty());
}

#[test]
fn load_builtin_rule_files() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let normalize = manifest.join("src/compile/knowledge/rules/normalize.rule");
    let merge = manifest.join("src/compile/knowledge/rules/merge.rule");
    let cancel = manifest.join("src/compile/knowledge/rules/cancel.rule");
    let mc_gate = manifest.join("src/compile/knowledge/rules/decompose_mc_gate.rule");

    let normalize_rules = load_rules_from_file(&normalize).unwrap();
    assert!(!normalize_rules.is_empty());
    let normalize_names: std::collections::HashSet<_> =
        normalize_rules.iter().map(|r| r.name.as_str()).collect();
    assert!(normalize_names.contains("normalize_i"));
    assert!(normalize_names.contains("normalize_rxy_zero"));

    let merge_rules = load_rules_from_file(&merge).unwrap();
    assert!(!merge_rules.is_empty());
    let merge_names: std::collections::HashSet<_> =
        merge_rules.iter().map(|r| r.name.as_str()).collect();
    assert!(merge_names.contains("merge_rx"));
    assert!(merge_names.contains("merge_rz"));

    let cancel_rules = load_rules_from_file(&cancel).unwrap();
    assert!(!cancel_rules.is_empty());
    let cancel_names: std::collections::HashSet<_> =
        cancel_rules.iter().map(|r| r.name.as_str()).collect();
    assert!(cancel_names.contains("cancel_rx_inverse"));
    assert!(cancel_names.contains("cancel_h"));

    let mc_gate_rules = load_rules_from_file(&mc_gate).unwrap();
    assert!(!mc_gate_rules.is_empty());
    let mc_gate_names: std::collections::HashSet<_> =
        mc_gate_rules.iter().map(|r| r.name.as_str()).collect();
    assert!(mc_gate_names.contains("decompose_mcy2_to_ccx"));
    assert!(mc_gate_names.contains("decompose_mcx3_to_parity_phase"));
}

#[test]
fn load_multi_controlled_rule_file_contains_mcgate_patterns() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mc_gate = manifest.join("src/compile/knowledge/rules/decompose_mc_gate.rule");
    let rules = load_rules_from_file(&mc_gate).unwrap();
    let mcz = rules
        .iter()
        .find(|rule| rule.name == "decompose_mcz2_to_ccx")
        .expect("MCZ rule should exist");

    assert!(matches!(
        mcz.operations[0].instruction,
        crate::circuit::Instruction::McGate(_)
    ));
    assert_eq!(mcz.operations[0].qubits.as_slice(), &[0, 1, 2]);
}

#[test]
fn builtin_cancel_rule_uses_evaluable_pi_constant() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cancel = manifest.join("src/compile/knowledge/rules/cancel.rule");
    let rules = load_rules_from_file(&cancel).unwrap();
    let rule = rules
        .iter()
        .find(|r| r.name == "cancel_rx_inverse")
        .expect("cancel_rx_inverse rule should exist");
    let conditions = rule.conditions.as_ref().unwrap();

    match &conditions[0] {
        Condition::EqMod(_, _, modulus) => {
            let actual = modulus.evaluate(&None).unwrap();
            let expected = 4.0 * std::f64::consts::PI;
            assert!((actual - expected).abs() < 1e-12);
        }
        _ => panic!("expected EqMod condition"),
    }
}

#[test]
fn reject_duplicate_rule_names() {
    let err = load_rules_from_str(
        r#"
            rule same {
                match { H 0 }
                rewrite {}
            }
            rule same {
                match { X 0 }
                rewrite {}
            }
        "#,
    )
    .unwrap_err();
    assert!(matches!(err, LoadError::DuplicateRuleName(name) if name == "same"));
}

#[test]
fn load_merge_gphase_rule() {
    let rules = load_rules_from_str(
        r#"
            rule merge_gphase {
                match { GPhase(a), GPhase(b) }
                rewrite { GPhase(a + b) }
            }
        "#,
    )
    .unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "merge_gphase");
    assert_eq!(rules[0].operations.len(), 2);
    assert!(rules[0].operations[0].qubits.is_empty());
    assert_eq!(rules[0].target.len(), 1);
    assert!(rules[0].target[0].qubits.is_empty());
}

#[test]
fn load_cancel_gphase_inverse_rule() {
    let rules = load_rules_from_str(
        r#"
            rule cancel_gphase_inverse {
                match { GPhase(a), GPhase(b) }
                require { a + b == 0 mod 2*π }
                rewrite {}
            }
        "#,
    )
    .unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "cancel_gphase_inverse");
    assert_eq!(rules[0].operations.len(), 2);
    assert!(rules[0].operations[0].qubits.is_empty());
    assert_eq!(rules[0].conditions.as_ref().unwrap().len(), 1);
    assert!(rules[0].target.is_empty());
}

#[test]
fn empty_source_returns_empty_rules() {
    let rules = load_rules_from_str("").unwrap();
    assert!(rules.is_empty());
}

#[test]
fn whitespace_only_source_returns_empty_rules() {
    let rules = load_rules_from_str("   \n  \n  ").unwrap();
    assert!(rules.is_empty());
}

#[test]
fn consecutive_commas_in_match_are_rejected() {
    let result = load_rules_from_str(
        r#"rule bad {
                match { H 0,, H 0 }
                rewrite {}
            }"#,
    );
    assert!(result.is_err());
}

#[test]
fn load_commutation_rule_file() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let commutation = manifest.join("src/compile/knowledge/rules/commutation.rule");
    let rules = load_rules_from_file(&commutation).unwrap();
    assert!(!rules.is_empty());
    let names: std::collections::HashSet<_> = rules.iter().map(|r| r.name.as_str()).collect();
    // new rules from task 1
    assert!(names.contains("comm_s_sdg"));
    assert!(names.contains("comm_phase_phase"));
    assert!(names.contains("comm_x_x2p"));
    assert!(names.contains("comm_y2p_y2m"));
    // task 2
    assert!(names.contains("comm_cx_sdg_ctrl"));
    assert!(names.contains("comm_cx_tdg_ctrl"));
    assert!(names.contains("comm_cz_tdg_0"));
    assert!(names.contains("comm_cz_tdg_1"));
    assert!(names.contains("comm_cy_rz_ctrl"));
    assert!(names.contains("comm_cy_y_target"));
    // task 3
    assert!(names.contains("comm_crx_rz_ctrl"));
    assert!(names.contains("comm_crx_x_target"));
    assert!(names.contains("comm_cry_ry_target"));
    assert!(names.contains("comm_crz_phase_target"));
    assert!(names.contains("comm_ccx_rz_ctrl0"));
    assert!(names.contains("comm_ccx_x_target"));
    // task 4
    assert!(names.contains("comm_rzz_rzz"));
    assert!(names.contains("comm_rxx_rxx"));
    assert!(names.contains("comm_ryy_ryy"));
    assert!(names.contains("comm_rzx_rzx"));
    assert!(names.contains("comm_rzz_phase_1"));
    assert!(names.contains("comm_rzx_tdg_0"));

    // verify matrix equivalence for selected new rules
    for rule in &rules {
        match rule.name.as_str() {
            "comm_s_sdg"
            | "comm_phase_z"
            | "comm_phase_phase"
            | "comm_x_x2p"
            | "comm_y_y2p"
            | "comm_t_tdg"
            | "comm_cx_sdg_ctrl"
            | "comm_cx_tdg_ctrl"
            | "comm_cz_tdg_0"
            | "comm_cz_tdg_1"
            | "comm_cy_rz_ctrl"
            | "comm_cy_y_target"
            | "comm_crx_rz_ctrl"
            | "comm_crx_x_target"
            | "comm_cry_ry_target"
            | "comm_crz_phase_target"
            | "comm_ccx_rz_ctrl0"
            | "comm_ccx_x_target"
            | "comm_rzz_rz_0"
            | "comm_rzz_phase_1"
            | "comm_rzz_rzz"
            | "comm_rxx_x_0"
            | "comm_rxx_rxx"
            | "comm_ryy_y_1"
            | "comm_ryy_ryy"
            | "comm_rzx_tdg_0"
            | "comm_rzx_x2p_1"
            | "comm_rzx_rzx" => {
                let result = rule.verify_by_sampling(10, 1e-8).unwrap();
                assert!(
                    result.is_verified(),
                    "rule {} failed verification: {:?}",
                    rule.name,
                    result
                );
            }
            _ => {}
        }
    }
}
