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
use crate::circuit::{Instruction, StandardGate};
use crate::compile::knowledge::rule_dsl::load::load_rules_from_str;
use crate::compile::knowledge::rule_dsl::parser::Parser;

#[test]
fn dump_merge_rz_roundtrip() {
    let input = r#"
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
    let mut parser = Parser::new(input).unwrap();
    let defs = parser.parse_rule_file().unwrap();
    let rules: Vec<Rule> = defs
        .into_iter()
        .map(RuleDef::into_rule)
        .collect::<Result<_, _>>()
        .unwrap();

    let dumped = dump_rule_to_string(&rules[0]);
    let reparsed = load_rules_from_str(&dumped).unwrap();
    assert_eq!(reparsed.len(), 1);
    assert_eq!(reparsed[0].name, "merge_rz");
    assert_eq!(reparsed[0].operations.len(), 2);
    assert_eq!(reparsed[0].target.len(), 1);
}

#[test]
fn dump_cancel_rx_roundtrip() {
    let input = r#"
            rule cancel_rx_inverse {
                match {
                    RX(a) 0
                    RX(b) 0
                }
                require {
                    a + b == 0 mod 2*π
                }
                rewrite {
                }
            }
        "#;
    let mut parser = Parser::new(input).unwrap();
    let defs = parser.parse_rule_file().unwrap();
    let rules: Vec<Rule> = defs
        .into_iter()
        .map(RuleDef::into_rule)
        .collect::<Result<_, _>>()
        .unwrap();

    let dumped = dump_rule_to_string(&rules[0]);
    let reparsed = load_rules_from_str(&dumped).unwrap();
    assert_eq!(reparsed.len(), 1);
    assert_eq!(reparsed[0].name, "cancel_rx_inverse");
    assert_eq!(reparsed[0].operations.len(), 2);
    assert!(reparsed[0].conditions.is_some());
    assert!(reparsed[0].target.is_empty());
}

#[test]
fn dump_rules_to_file_roundtrip() {
    let rules = load_rules_from_str(
        r#"
            rule merge_rz {
                match {
                    RZ(a) 0
                    RZ(b) 0
                }
                rewrite {
                    RZ(a + b) 0
                }
            }
            rule cancel_h {
                match {
                    H 0
                    H 0
                }
                rewrite {}
            }
        "#,
    )
    .unwrap();

    let tmp = std::env::temp_dir().join("cqlib_dump_test.rules");
    dump_rules_to_file(&rules, &tmp).unwrap();

    let reparsed = crate::compile::knowledge::rule_dsl::load::load_rules_from_file(&tmp).unwrap();
    assert_eq!(reparsed.len(), 2);
    assert_eq!(reparsed[0].name, "merge_rz");
    assert_eq!(reparsed[1].name, "cancel_h");

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn dump_gphase_roundtrip() {
    let rules = load_rules_from_str(
        r#"
            rule merge_gphase {
                match { GPhase(a), GPhase(b) }
                rewrite { GPhase(a + b) }
            }
            rule cancel_gphase_inverse {
                match { GPhase(a), GPhase(b) }
                require { a + b == 0 mod 2*π }
                rewrite {}
            }
        "#,
    )
    .unwrap();

    let dumped = rules
        .iter()
        .map(dump_rule_to_string)
        .collect::<Vec<_>>()
        .join("\n");
    let reparsed = load_rules_from_str(&dumped).unwrap();
    assert_eq!(reparsed.len(), 2);
    assert_eq!(reparsed[0].name, "merge_gphase");
    assert!(reparsed[0].operations[0].qubits.is_empty());
    assert_eq!(reparsed[1].name, "cancel_gphase_inverse");
    assert_eq!(reparsed[1].conditions.as_ref().unwrap().len(), 1);
}

#[test]
fn dump_multi_controlled_gate_roundtrip() {
    let rules = load_rules_from_str(
        r#"
            rule decompose_m3cx {
                match { MCX[3] 0 1 2 3 }
                rewrite { CCX 0 1 2 }
            }
            rule decompose_m2rz {
                match { MCRZ[2](theta) 0 1 2 }
                rewrite { CRZ(theta) 1 2 }
            }
        "#,
    )
    .unwrap();

    let dumped = rules
        .iter()
        .map(dump_rule_to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(dumped.contains("MCX[3] 0 1 2 3"));
    assert!(dumped.contains("MCRZ[2](theta) 0 1 2"));

    let reparsed = load_rules_from_str(&dumped).unwrap();
    assert_eq!(reparsed.len(), 2);
    let Instruction::McGate(gate) = &reparsed[0].operations[0].instruction else {
        panic!("expected MCGate");
    };
    assert_eq!(*gate.base_gate(), StandardGate::X);
    assert_eq!(gate.num_ctrl_qubits(), 3);
}
