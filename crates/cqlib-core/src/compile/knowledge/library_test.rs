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
use crate::circuit::{Instruction, MCGate, Parameter, ParameterValue, StandardGate};
use crate::compile::knowledge::rule::{Condition, RuleItem};
use crate::compile::knowledge::rule_dsl::dump::dump_rule_to_string;
use crate::compile::knowledge::rule_dsl::load::load_rules_from_str;
use smallvec::smallvec;
use std::collections::{BTreeSet, HashSet};

#[test]
fn empty_library_can_be_created() {
    let library = RuleLibrary::new();

    assert!(library.is_empty());
    assert_eq!(library.len(), 0);
}

#[test]
fn from_rules_preserves_insertion_order() {
    let library = RuleLibrary::from_rules(
        vec![
            Rule::new(
                "cancel_h",
                vec![
                    RuleItem::standard(StandardGate::H, &[0], vec![]),
                    RuleItem::standard(StandardGate::H, &[0], vec![]),
                ],
                vec![],
            ),
            Rule::new(
                "cx_h",
                vec![
                    RuleItem::standard(StandardGate::CX, &[0, 1], vec![]),
                    RuleItem::standard(StandardGate::H, &[1], vec![]),
                ],
                vec![RuleItem::standard(StandardGate::H, &[1], vec![])],
            ),
        ],
        RuleKind::Simplify,
    )
    .unwrap();

    assert_eq!(library.rules()[0].name, "cancel_h");
    assert_eq!(library.rules()[1].name, "cx_h");
}

#[test]
fn add_rule_returns_stable_ids() {
    let mut library = RuleLibrary::new();

    let first = library
        .add_rule(
            Rule::new(
                "first",
                vec![
                    RuleItem::standard(StandardGate::H, &[0], vec![]),
                    RuleItem::standard(StandardGate::H, &[0], vec![]),
                ],
                vec![],
            ),
            RuleKind::Cancel,
            true,
        )
        .unwrap();
    let second = library
        .add_rule(
            Rule::new(
                "second",
                vec![
                    RuleItem::standard(StandardGate::CX, &[0, 1], vec![]),
                    RuleItem::standard(StandardGate::H, &[1], vec![]),
                ],
                vec![RuleItem::standard(StandardGate::H, &[1], vec![])],
            ),
            RuleKind::Simplify,
            true,
        )
        .unwrap();

    assert_eq!(first.as_usize(), 0);
    assert_eq!(second.as_usize(), 1);
    assert_eq!(library.get(first).unwrap().name, "first");
    assert_eq!(library.get(second).unwrap().name, "second");
}

#[test]
fn name_lookup_works() {
    let library = RuleLibrary::from_rules(
        vec![Rule::new(
            "cancel_h",
            vec![
                RuleItem::standard(StandardGate::H, &[0], vec![]),
                RuleItem::standard(StandardGate::H, &[0], vec![]),
            ],
            vec![],
        )],
        RuleKind::Cancel,
    )
    .unwrap();

    let id = library.id_by_name("cancel_h").unwrap();
    assert_eq!(id.as_usize(), 0);
    assert_eq!(library.get_by_name("cancel_h").unwrap().name, "cancel_h");
    assert!(library.contains("cancel_h"));
    assert!(!library.contains("missing"));
}

#[test]
fn duplicate_rule_names_are_rejected() {
    let err = RuleLibrary::from_rules(
        vec![
            Rule::new(
                "duplicate",
                vec![
                    RuleItem::standard(StandardGate::H, &[0], vec![]),
                    RuleItem::standard(StandardGate::H, &[0], vec![]),
                ],
                vec![],
            ),
            Rule::new(
                "duplicate",
                vec![
                    RuleItem::standard(StandardGate::CX, &[0, 1], vec![]),
                    RuleItem::standard(StandardGate::H, &[1], vec![]),
                ],
                vec![RuleItem::standard(StandardGate::H, &[1], vec![])],
            ),
        ],
        RuleKind::Simplify,
    )
    .unwrap_err();

    assert_eq!(
        err,
        RuleLibraryError::DuplicateRuleName("duplicate".to_string())
    );
}

#[test]
fn invalid_rule_is_rejected_with_rule_name() {
    let bad = Rule::new(
        "bad",
        vec![],
        vec![RuleItem::standard(StandardGate::H, &[0], vec![])],
    );

    let err = RuleLibrary::from_rules(vec![bad], RuleKind::Simplify).unwrap_err();

    assert_eq!(
        err,
        RuleLibraryError::InvalidRule {
            name: "bad".to_string(),
            source: RuleValidationError::EmptyMatch,
        }
    );
}

#[test]
fn metadata_is_precomputed() {
    let mut rule = Rule::new(
        "conditioned_merge",
        vec![
            RuleItem::standard(
                StandardGate::RZ,
                &[0],
                vec![ParameterValue::Param(Parameter::symbol("a"))],
            ),
            RuleItem::standard(
                StandardGate::RZ,
                &[0],
                vec![ParameterValue::Param(Parameter::symbol("b"))],
            ),
        ],
        vec![RuleItem::standard(
            StandardGate::RZ,
            &[0],
            vec![ParameterValue::Param(
                Parameter::symbol("a") + Parameter::symbol("b"),
            )],
        )],
    );
    rule.conditions = Some(smallvec![Condition::Eq(
        Parameter::symbol("a"),
        Parameter::symbol("b"),
    )]);

    let library = RuleLibrary::from_rules(vec![rule], RuleKind::Merge).unwrap();
    let metadata = library.metadata(RuleId(0)).unwrap();

    assert_eq!(metadata.id, RuleId(0));
    assert_eq!(metadata.kind, RuleKind::Merge);
    assert_eq!(metadata.pattern_len, 2);
    assert_eq!(metadata.rewrite_len, 1);
    assert_eq!(metadata.qubit_count, 1);
    assert!(matches!(
        metadata.first_instruction,
        Instruction::Standard(StandardGate::RZ)
    ));
    assert_eq!(metadata.cost_delta, -1);
    assert!(metadata.has_conditions);
}

#[test]
fn candidates_use_only_first_match_gate() {
    let library = RuleLibrary::from_rules(
        vec![Rule::new(
            "cx_h",
            vec![
                RuleItem::standard(StandardGate::CX, &[0, 1], vec![]),
                RuleItem::standard(StandardGate::H, &[1], vec![]),
            ],
            vec![RuleItem::standard(StandardGate::H, &[1], vec![])],
        )],
        RuleKind::Simplify,
    )
    .unwrap();

    assert_eq!(
        library
            .candidates_for_first_instruction(&Instruction::Standard(StandardGate::CX))
            .unwrap(),
        &[RuleId(0)]
    );
    assert!(
        library
            .candidates_for_first_instruction(&Instruction::Standard(StandardGate::H))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn candidates_for_first_instruction_preserve_same_key_insertion_order() {
    let library = RuleLibrary::from_rules(
        vec![
            Rule::new(
                "first_cancel_h",
                vec![
                    RuleItem::standard(StandardGate::H, &[0], vec![]),
                    RuleItem::standard(StandardGate::H, &[0], vec![]),
                ],
                vec![],
            ),
            Rule::new(
                "second_cancel_h",
                vec![
                    RuleItem::standard(StandardGate::H, &[0], vec![]),
                    RuleItem::standard(StandardGate::H, &[0], vec![]),
                ],
                vec![],
            ),
        ],
        RuleKind::Cancel,
    )
    .unwrap();

    assert_eq!(
        library
            .candidates_for_first_instruction(&Instruction::Standard(StandardGate::H))
            .unwrap(),
        &[RuleId(0), RuleId(1)]
    );
}

#[test]
fn indexes_distinguish_standard_and_multi_controlled_instructions() {
    let standard_rule = Rule::new(
        "x_to_h",
        vec![RuleItem::standard(StandardGate::X, &[0], vec![])],
        vec![RuleItem::standard(StandardGate::H, &[0], vec![])],
    );
    let mc_rule = Rule::new(
        "mcx3_to_h",
        vec![RuleItem::mc_gate(
            MCGate::new(3, StandardGate::X),
            &[0, 1, 2, 3],
            vec![],
        )],
        vec![RuleItem::standard(StandardGate::H, &[3], vec![])],
    );
    let library = RuleLibrary::from_rules(vec![standard_rule, mc_rule], RuleKind::Simplify)
        .expect("rules should be valid");

    assert_eq!(
        library
            .candidates_for_first_instruction(&Instruction::Standard(StandardGate::X))
            .unwrap(),
        &[RuleId(0)]
    );
    assert_eq!(
        library
            .candidates_for_first_instruction(&Instruction::McGate(Box::new(MCGate::new(
                3,
                StandardGate::X
            ))))
            .unwrap(),
        &[RuleId(1)]
    );

    let standard_ids = library
        .filter_rule_ids_by_instruction_keys(
            &[Instruction::Standard(StandardGate::X)],
            &[Instruction::Standard(StandardGate::H)],
        )
        .unwrap();
    let mc_ids = library
        .filter_rule_ids_by_instruction_keys(
            &[Instruction::McGate(Box::new(MCGate::new(
                3,
                StandardGate::X,
            )))],
            &[Instruction::Standard(StandardGate::H)],
        )
        .unwrap();

    assert_eq!(standard_ids.as_slice(), &[RuleId(0)]);
    assert_eq!(mc_ids.as_slice(), &[RuleId(1)]);
}

#[test]
fn filter_rule_ids_by_gates_distinguishes_standard_and_multi_controlled_rewrites() {
    let standard_rewrite = Rule::new(
        "mcx3_to_x",
        vec![RuleItem::mc_gate(
            MCGate::new(3, StandardGate::X),
            &[0, 1, 2, 3],
            vec![],
        )],
        vec![RuleItem::standard(StandardGate::X, &[3], vec![])],
    );
    let mc_rewrite = Rule::new(
        "mcx3_to_mcx2",
        vec![RuleItem::mc_gate(
            MCGate::new(3, StandardGate::X),
            &[0, 1, 2, 3],
            vec![],
        )],
        vec![RuleItem::mc_gate(
            MCGate::new(2, StandardGate::X),
            &[0, 1, 2],
            vec![],
        )],
    );
    let library =
        RuleLibrary::from_rules(vec![standard_rewrite, mc_rewrite], RuleKind::Decompose).unwrap();

    let standard_ids = library
        .filter_rule_ids_by_instruction_keys(
            &[Instruction::McGate(Box::new(MCGate::new(
                3,
                StandardGate::X,
            )))],
            &[Instruction::Standard(StandardGate::X)],
        )
        .unwrap();
    let mc_ids = library
        .filter_rule_ids_by_instruction_keys(
            &[Instruction::McGate(Box::new(MCGate::new(
                3,
                StandardGate::X,
            )))],
            &[Instruction::McGate(Box::new(MCGate::new(
                2,
                StandardGate::X,
            )))],
        )
        .unwrap();

    assert_eq!(standard_ids.as_slice(), &[RuleId(0)]);
    assert_eq!(mc_ids.as_slice(), &[RuleId(1)]);
}

#[test]
fn instruction_indexes_reject_unsupported_instruction_keys() {
    let library = RuleLibrary::from_rules(
        vec![Rule::new(
            "cancel_h",
            vec![
                RuleItem::standard(StandardGate::H, &[0], vec![]),
                RuleItem::standard(StandardGate::H, &[0], vec![]),
            ],
            vec![],
        )],
        RuleKind::Cancel,
    )
    .unwrap();

    assert!(matches!(
        library.candidates_for_first_instruction(&Instruction::Delay),
        Err(RuleLibraryError::UnsupportedInstruction { .. })
    ));
    assert!(matches!(
        library.filter_rule_ids_by_instruction_keys(&[Instruction::Delay], &[]),
        Err(RuleLibraryError::UnsupportedInstruction { .. })
    ));
    assert!(matches!(
        library.filter_rule_ids_by_instruction_keys(&[], &[Instruction::Delay]),
        Err(RuleLibraryError::UnsupportedInstruction { .. })
    ));
}

#[test]
fn rules_by_kind_returns_matching_rules() {
    let mut library = RuleLibrary::new();
    let cancel = library
        .add_rule(
            Rule::new(
                "cancel_h",
                vec![
                    RuleItem::standard(StandardGate::H, &[0], vec![]),
                    RuleItem::standard(StandardGate::H, &[0], vec![]),
                ],
                vec![],
            ),
            RuleKind::Cancel,
            true,
        )
        .unwrap();
    let simplify = library
        .add_rule(
            Rule::new(
                "cx_h",
                vec![
                    RuleItem::standard(StandardGate::CX, &[0, 1], vec![]),
                    RuleItem::standard(StandardGate::H, &[1], vec![]),
                ],
                vec![RuleItem::standard(StandardGate::H, &[1], vec![])],
            ),
            RuleKind::Simplify,
            true,
        )
        .unwrap();

    assert_eq!(library.rules_by_kind(RuleKind::Cancel), &[cancel]);
    assert_eq!(library.rules_by_kind(RuleKind::Simplify), &[simplify]);
    assert!(library.rules_by_kind(RuleKind::Decompose).is_empty());
}

#[test]
fn filter_rule_ids_by_gates_returns_rules_matching_source_and_target_basis() {
    let library = RuleLibrary::from_rules(
        vec![
            Rule::new(
                "decompose_swap_to_cx",
                vec![RuleItem::standard(StandardGate::SWAP, &[0, 1], vec![])],
                vec![
                    RuleItem::standard(StandardGate::CX, &[0, 1], vec![]),
                    RuleItem::standard(StandardGate::CX, &[1, 0], vec![]),
                    RuleItem::standard(StandardGate::CX, &[0, 1], vec![]),
                ],
            ),
            Rule::new(
                "decompose_swap_to_cz_h",
                vec![RuleItem::standard(StandardGate::SWAP, &[0, 1], vec![])],
                vec![
                    RuleItem::standard(StandardGate::H, &[1], vec![]),
                    RuleItem::standard(StandardGate::CZ, &[0, 1], vec![]),
                    RuleItem::standard(StandardGate::H, &[1], vec![]),
                ],
            ),
        ],
        RuleKind::Decompose,
    )
    .unwrap();

    let ids = library
        .filter_rule_ids_by_instruction_keys(
            &[Instruction::Standard(StandardGate::SWAP)],
            &[Instruction::Standard(StandardGate::CX)],
        )
        .unwrap();

    assert_eq!(ids.as_slice(), &[RuleId(0)]);
}

#[test]
fn filter_rule_ids_by_gates_rejects_rules_with_unlisted_match_gates() {
    let library = RuleLibrary::from_rules(
        vec![Rule::new(
            "cancel_h",
            vec![
                RuleItem::standard(StandardGate::H, &[0], vec![]),
                RuleItem::standard(StandardGate::H, &[0], vec![]),
            ],
            vec![],
        )],
        RuleKind::Cancel,
    )
    .unwrap();

    let ids = library
        .filter_rule_ids_by_instruction_keys(&[Instruction::Standard(StandardGate::X)], &[])
        .unwrap();

    assert!(ids.is_empty());
}

#[test]
fn filter_rule_ids_by_gates_allows_empty_rewrite() {
    let library = RuleLibrary::from_rules(
        vec![Rule::new(
            "cancel_h",
            vec![
                RuleItem::standard(StandardGate::H, &[0], vec![]),
                RuleItem::standard(StandardGate::H, &[0], vec![]),
            ],
            vec![],
        )],
        RuleKind::Cancel,
    )
    .unwrap();

    let ids = library
        .filter_rule_ids_by_instruction_keys(&[Instruction::Standard(StandardGate::H)], &[])
        .unwrap();

    assert_eq!(ids.as_slice(), &[RuleId(0)]);
}

#[test]
fn filter_rule_ids_by_gates_requires_all_rewrite_gates() {
    let library = RuleLibrary::from_rules(
        vec![Rule::new(
            "decompose_swap_to_cz_h",
            vec![RuleItem::standard(StandardGate::SWAP, &[0, 1], vec![])],
            vec![
                RuleItem::standard(StandardGate::H, &[1], vec![]),
                RuleItem::standard(StandardGate::CZ, &[0, 1], vec![]),
                RuleItem::standard(StandardGate::H, &[1], vec![]),
            ],
        )],
        RuleKind::Decompose,
    )
    .unwrap();

    let missing_h = library
        .filter_rule_ids_by_instruction_keys(
            &[Instruction::Standard(StandardGate::SWAP)],
            &[Instruction::Standard(StandardGate::CZ)],
        )
        .unwrap();
    let complete = library
        .filter_rule_ids_by_instruction_keys(
            &[Instruction::Standard(StandardGate::SWAP)],
            &[
                Instruction::Standard(StandardGate::H),
                Instruction::Standard(StandardGate::CZ),
            ],
        )
        .unwrap();

    assert!(missing_h.is_empty());
    assert_eq!(complete.as_slice(), &[RuleId(0)]);
}

#[test]
fn filter_rule_ids_by_gates_treats_gphase_as_required_target_gate() {
    let library = RuleLibrary::from_rules(
        vec![Rule::new(
            "decompose_phase_to_rz_gphase",
            vec![RuleItem::standard(
                StandardGate::Phase,
                &[0],
                vec![ParameterValue::Fixed(0.5)],
            )],
            vec![
                RuleItem::standard(StandardGate::RZ, &[0], vec![ParameterValue::Fixed(0.5)]),
                RuleItem::standard(StandardGate::GPhase, &[], vec![ParameterValue::Fixed(0.25)]),
            ],
        )],
        RuleKind::Decompose,
    )
    .unwrap();

    let missing_gphase = library
        .filter_rule_ids_by_instruction_keys(
            &[Instruction::Standard(StandardGate::Phase)],
            &[Instruction::Standard(StandardGate::RZ)],
        )
        .unwrap();
    let complete = library
        .filter_rule_ids_by_instruction_keys(
            &[Instruction::Standard(StandardGate::Phase)],
            &[
                Instruction::Standard(StandardGate::RZ),
                Instruction::Standard(StandardGate::GPhase),
            ],
        )
        .unwrap();

    assert!(missing_gphase.is_empty());
    assert_eq!(complete.as_slice(), &[RuleId(0)]);
}

#[test]
fn filter_rule_ids_by_gates_preserves_library_rule_ids() {
    let library = RuleLibrary::from_rules(
        vec![
            Rule::new(
                "cancel_h",
                vec![
                    RuleItem::standard(StandardGate::H, &[0], vec![]),
                    RuleItem::standard(StandardGate::H, &[0], vec![]),
                ],
                vec![],
            ),
            Rule::new(
                "decompose_swap_to_cx",
                vec![RuleItem::standard(StandardGate::SWAP, &[0, 1], vec![])],
                vec![
                    RuleItem::standard(StandardGate::CX, &[0, 1], vec![]),
                    RuleItem::standard(StandardGate::CX, &[1, 0], vec![]),
                    RuleItem::standard(StandardGate::CX, &[0, 1], vec![]),
                ],
            ),
            Rule::new(
                "cx_h",
                vec![
                    RuleItem::standard(StandardGate::CX, &[0, 1], vec![]),
                    RuleItem::standard(StandardGate::H, &[1], vec![]),
                ],
                vec![RuleItem::standard(StandardGate::H, &[1], vec![])],
            ),
        ],
        RuleKind::Simplify,
    )
    .unwrap();

    let ids = library
        .filter_rule_ids_by_instruction_keys(
            &[
                Instruction::Standard(StandardGate::SWAP),
                Instruction::Standard(StandardGate::CX),
                Instruction::Standard(StandardGate::H),
            ],
            &[
                Instruction::Standard(StandardGate::CX),
                Instruction::Standard(StandardGate::H),
            ],
        )
        .unwrap();

    assert_eq!(ids.as_slice(), &[RuleId(0), RuleId(1), RuleId(2)]);
    assert_eq!(ids[1].as_usize(), 1);
}

#[test]
fn from_dsl_str_loads_rules_and_builds_indexes() {
    let library = RuleLibrary::from_dsl_str(
        r#"
                rule cancel_h {
                    match { H 0, H 0 }
                    rewrite {}
                }
            "#,
        RuleKind::Cancel,
    )
    .unwrap();

    assert_eq!(library.len(), 1);
    assert_eq!(library.get_by_name("cancel_h").unwrap().name, "cancel_h");
    assert_eq!(
        library
            .candidates_for_first_instruction(&Instruction::Standard(StandardGate::H))
            .unwrap(),
        &[RuleId(0)]
    );
    assert_eq!(library.rules_by_kind(RuleKind::Cancel), &[RuleId(0)]);
}

#[test]
fn from_dsl_str_wraps_load_errors() {
    let err = RuleLibrary::from_dsl_str("not a rule", RuleKind::Other).unwrap_err();

    assert!(matches!(err, RuleLibraryError::Load(_)));
}

#[test]
fn add_rule_with_validation_rejects_invalid_rule() {
    let mut library = RuleLibrary::new();
    let bad = Rule::new(
        "bad",
        vec![RuleItem::standard(StandardGate::CX, &[0], vec![])],
        vec![],
    );

    let err = library
        .add_rule(bad, RuleKind::Other, true)
        .expect_err("validated insert should reject wrong qubit count");

    assert!(matches!(
        err,
        RuleLibraryError::InvalidRule {
            name,
            source: RuleValidationError::WrongQubitCount { .. },
        } if name == "bad"
    ));
}

#[test]
fn add_rule_without_validation_skips_rule_validate() {
    let mut library = RuleLibrary::new();
    let bad = Rule::new(
        "bad",
        vec![RuleItem::standard(StandardGate::CX, &[0], vec![])],
        vec![],
    );

    let id = library
        .add_rule(bad, RuleKind::Other, false)
        .expect("unchecked insert should skip Rule::validate");

    assert_eq!(id, RuleId(0));
    assert_eq!(library.get_by_name("bad").unwrap().name, "bad");
    assert_eq!(
        library
            .candidates_for_first_instruction(&Instruction::Standard(StandardGate::CX))
            .unwrap(),
        &[RuleId(0)]
    );
    assert_eq!(library.rules_by_kind(RuleKind::Other), &[RuleId(0)]);
}

#[test]
fn add_rule_without_validation_still_rejects_unsupported_instruction() {
    let mut library = RuleLibrary::new();
    let bad = Rule::new(
        "bad",
        vec![RuleItem {
            instruction: Instruction::Delay,
            qubits: smallvec![],
            params: None,
        }],
        vec![],
    );

    assert!(matches!(
        library.add_rule(bad, RuleKind::Other, false),
        Err(RuleLibraryError::UnsupportedInstruction { .. })
    ));
}

#[test]
fn extend_rules_is_atomic_on_error() {
    let mut library = RuleLibrary::from_rules(
        vec![Rule::new(
            "existing",
            vec![
                RuleItem::standard(StandardGate::H, &[0], vec![]),
                RuleItem::standard(StandardGate::H, &[0], vec![]),
            ],
            vec![],
        )],
        RuleKind::Cancel,
    )
    .expect("initial library should be valid");

    let err = library
        .extend_rules(
            vec![
                Rule::new(
                    "new",
                    vec![
                        RuleItem::standard(StandardGate::CX, &[0, 1], vec![]),
                        RuleItem::standard(StandardGate::H, &[1], vec![]),
                    ],
                    vec![RuleItem::standard(StandardGate::H, &[1], vec![])],
                ),
                Rule::new(
                    "existing",
                    vec![
                        RuleItem::standard(StandardGate::H, &[0], vec![]),
                        RuleItem::standard(StandardGate::H, &[0], vec![]),
                    ],
                    vec![],
                ),
            ],
            RuleKind::Simplify,
        )
        .unwrap_err();

    assert_eq!(
        err,
        RuleLibraryError::DuplicateRuleName("existing".to_string())
    );
    assert_eq!(library.len(), 1);
    assert!(library.get_by_name("new").is_none());
}

#[test]
fn builtin_rules_loads_expected_rule_groups() {
    let library = RuleLibrary::builtin_rules().unwrap();

    assert!(!library.is_empty());
    assert!(library.get_by_name("cancel_h").is_some());
    assert!(library.get_by_name("merge_rz").is_some());
    assert!(library.get_by_name("normalize_i").is_some());
    assert!(library.get_by_name("identity_hxh_to_z").is_some());
    assert!(library.get_by_name("specialize_rx_pi_to_x").is_some());
    assert!(library.get_by_name("decompose_ccx_to_cx").is_some());
    assert!(library.get_by_name("decompose_ccx_to_u_cx").is_some());
    assert!(library.get_by_name("decompose_ccx_to_u_cz").is_some());
    assert!(library.get_by_name("decompose_ccx_to_rz_x2p_cx").is_some());
    assert!(
        library
            .get_by_name("decompose_ccx_to_rz_x2p_x_cz")
            .is_some()
    );
    assert!(library.get_by_name("decompose_ccx_to_rx_ry_cx").is_some());
    assert!(library.get_by_name("decompose_ccx_to_rx_ry_cz").is_some());
    assert!(library.get_by_name("decompose_ccx_to_rx_ry_rxx").is_some());
    assert!(
        library
            .get_by_name("decompose_ccx_to_rz_x2p_x_rzz")
            .is_some()
    );
    assert!(library.get_by_name("decompose_mcx1_to_cx").is_some());
    assert!(library.get_by_name("comm_s_sdg").is_some());

    assert!(!library.rules_by_kind(RuleKind::Cancel).is_empty());
    assert!(!library.rules_by_kind(RuleKind::Merge).is_empty());
    assert!(!library.rules_by_kind(RuleKind::Canonicalize).is_empty());
    assert!(!library.rules_by_kind(RuleKind::Simplify).is_empty());
    assert!(!library.rules_by_kind(RuleKind::Decompose).is_empty());
    assert!(!library.rules_by_kind(RuleKind::Commute).is_empty());
}

#[test]
fn library_accepts_multi_controlled_rule_file() {
    let library = RuleLibrary::from_dsl_str(
        include_str!("rules/decompose_mc_gate.rule"),
        RuleKind::Decompose,
    )
    .unwrap();

    assert!(library.get_by_name("decompose_mcz2_to_ccx").is_some());
    assert!(
        library
            .get_by_name("decompose_mcx3_to_parity_phase")
            .is_some()
    );
    assert!(!library.rules_by_kind(RuleKind::Decompose).is_empty());
}

#[test]
fn builtin_rules_are_cached() {
    let first = RuleLibrary::builtin_rules().unwrap();
    let second = RuleLibrary::builtin_rules().unwrap();

    assert!(std::ptr::eq(first, second));
}

#[test]
fn builtin_rules_build_candidate_index() {
    let library = RuleLibrary::builtin_rules().unwrap();

    assert!(
        library
            .candidates_for_first_instruction(&Instruction::Standard(StandardGate::H))
            .unwrap()
            .iter()
            .any(|&id| library.get(id).is_some_and(|rule| rule.name == "cancel_h"))
    );
}

#[test]
fn builtin_rule_sources_parse_validate_and_have_unique_names() {
    let mut seen = HashSet::new();
    let mut total = 0usize;

    for (source, _kind) in BUILTIN_RULE_SOURCES {
        let rules = load_rules_from_str(source).unwrap();
        assert!(!rules.is_empty());
        for rule in rules {
            rule.validate().unwrap();
            assert!(
                seen.insert(rule.name.clone()),
                "duplicate builtin rule name {}",
                rule.name
            );
            assert!(!rule.operations.is_empty(), "empty match in {}", rule.name);
            total += 1;
        }
    }

    assert_eq!(total, RuleLibrary::builtin_rules().unwrap().len());
}

#[test]
fn builtin_rule_metadata_matches_rule_contents() {
    let library = RuleLibrary::builtin_rules().unwrap();

    for id in (0..library.len()).map(RuleId) {
        let rule = library.get(id).unwrap();
        let metadata = library.metadata(id).unwrap();
        let qubit_count = rule
            .operations
            .iter()
            .chain(&rule.target)
            .flat_map(|item| item.qubits.iter().copied())
            .collect::<BTreeSet<_>>()
            .len();

        assert_eq!(metadata.id, id);
        assert_eq!(metadata.pattern_len, rule.operations.len(), "{}", rule.name);
        assert_eq!(metadata.rewrite_len, rule.target.len(), "{}", rule.name);
        assert_eq!(metadata.qubit_count, qubit_count, "{}", rule.name);
        assert_eq!(
            format!("{:?}", metadata.first_instruction),
            format!("{:?}", rule.operations[0].instruction),
            "{}",
            rule.name
        );
        assert_eq!(
            metadata.has_conditions,
            rule.conditions
                .as_ref()
                .is_some_and(|conditions| !conditions.is_empty()),
            "{}",
            rule.name
        );
    }
}

#[test]
fn builtin_rules_cover_required_rule_families() {
    let library = RuleLibrary::builtin_rules().unwrap();

    for kind in [
        RuleKind::Canonicalize,
        RuleKind::Cancel,
        RuleKind::Merge,
        RuleKind::Simplify,
        RuleKind::Commute,
        RuleKind::Decompose,
    ] {
        assert!(
            !library.rules_by_kind(kind).is_empty(),
            "missing builtin rules for {kind:?}"
        );
    }

    for name in [
        "decompose_mcz2_to_ccx",
        "decompose_mcx3_to_parity_phase",
        "decompose_u_to_zyz",
        "decompose_u_to_xy_basis",
        "decompose_fsim_to_rzz_phase",
        "decompose_swap_to_cx",
    ] {
        assert!(library.get_by_name(name).is_some(), "missing {name}");
    }
}

#[test]
fn dumped_builtin_rules_load_back_to_equivalent_shape() {
    let library = RuleLibrary::builtin_rules().unwrap();

    for name in [
        "cancel_h",
        "merge_rz",
        "comm_s_sdg",
        "decompose_mcx3_to_parity_phase",
        "decompose_u_to_zyz",
        "decompose_swap_to_cx",
    ] {
        let rule = library.get_by_name(name).unwrap();
        let source = dump_rule_to_string(rule);
        let loaded = load_rules_from_str(&source).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, rule.name);
        assert_eq!(loaded[0].operations.len(), rule.operations.len());
        assert_eq!(loaded[0].target.len(), rule.target.len());
        assert_eq!(
            loaded[0]
                .conditions
                .as_ref()
                .map(|conditions| conditions.len())
                .unwrap_or(0),
            rule.conditions
                .as_ref()
                .map(|conditions| conditions.len())
                .unwrap_or(0)
        );
    }
}
