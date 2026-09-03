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

//! Contract tests for builtin `.rule` libraries (knowledge module only).
//!
//! Covers parse/validate health, layered [`Rule::verify`], and gate-formula
//! equivalence for selected decompositions. Does not exercise `transform` passes.

use crate::circuit::circuit_to_matrix::circuit_to_matrix;
use crate::circuit::test_utils::{
    assert_circuits_equivalent_up_to_global_phase, assert_matrix_approx_eq,
};
use crate::circuit::{Circuit, Parameter, Qubit};
use crate::compile::knowledge::rule::Rule;
use crate::compile::knowledge::rule_dsl::load::load_rules_from_str;
use crate::compile::knowledge::rule_equivalence::VerifyResult;
use crate::compile::knowledge::{RuleKind, RuleLibrary};
use std::f64::consts::PI;

const RULE_FILES: &[(&str, &str, RuleKind)] = &[
    (
        "cancel.rule",
        include_str!("rules/cancel.rule"),
        RuleKind::Cancel,
    ),
    (
        "merge.rule",
        include_str!("rules/merge.rule"),
        RuleKind::Merge,
    ),
    (
        "commutation.rule",
        include_str!("rules/commutation.rule"),
        RuleKind::Commute,
    ),
    (
        "normalize.rule",
        include_str!("rules/normalize.rule"),
        RuleKind::Canonicalize,
    ),
    (
        "identity.rule",
        include_str!("rules/identity.rule"),
        RuleKind::Simplify,
    ),
    (
        "specialize.rule",
        include_str!("rules/specialize.rule"),
        RuleKind::Simplify,
    ),
    (
        "decompose_ccx.rule",
        include_str!("rules/decompose_ccx.rule"),
        RuleKind::Decompose,
    ),
    (
        "decompose_controlled_pauli.rule",
        include_str!("rules/decompose_controlled_pauli.rule"),
        RuleKind::Decompose,
    ),
    (
        "decompose_controlled_rotation.rule",
        include_str!("rules/decompose_controlled_rotation.rule"),
        RuleKind::Decompose,
    ),
    (
        "decompose_mc_gate.rule",
        include_str!("rules/decompose_mc_gate.rule"),
        RuleKind::Decompose,
    ),
    (
        "decompose_fsim.rule",
        include_str!("rules/decompose_fsim.rule"),
        RuleKind::Decompose,
    ),
    (
        "decompose_ising.rule",
        include_str!("rules/decompose_ising.rule"),
        RuleKind::Decompose,
    ),
    (
        "decompose_phase.rule",
        include_str!("rules/decompose_phase.rule"),
        RuleKind::Decompose,
    ),
    (
        "decompose_qcis.rule",
        include_str!("rules/decompose_qcis.rule"),
        RuleKind::Decompose,
    ),
    (
        "decompose_single_clifford.rule",
        include_str!("rules/decompose_single_clifford.rule"),
        RuleKind::Decompose,
    ),
    (
        "decompose_single_rotation.rule",
        include_str!("rules/decompose_single_rotation.rule"),
        RuleKind::Decompose,
    ),
    (
        "decompose_swap.rule",
        include_str!("rules/decompose_swap.rule"),
        RuleKind::Decompose,
    ),
];

fn classify_rule_verification(rule: &Rule) -> VerifyResult {
    rule.verify().expect("verify setup should succeed")
}

fn assert_rule_verification_passes(rule: &Rule) {
    match classify_rule_verification(rule) {
        VerifyResult::Equivalent | VerifyResult::SampledEqual { .. } => {}
        VerifyResult::NotEquivalent => {
            panic!(
                "rule `{}` failed layered equivalence verification",
                rule.name
            );
        }
        VerifyResult::Inconclusive { reason } => {
            panic!("rule `{}` inconclusive: {reason}", rule.name);
        }
    }
}

fn circuits_equivalent_up_to_global_phase(
    actual: &Circuit,
    expected: &Circuit,
    epsilon: f64,
) -> bool {
    crate::circuit::test_utils::circuits_equal_up_to_global_phase(actual, expected, epsilon)
}

#[test]
fn all_rule_files_parse_and_validate() {
    for (file_name, source, _kind) in RULE_FILES {
        let rules = load_rules_from_str(source)
            .unwrap_or_else(|err| panic!("failed to parse {file_name}: {err}"));
        assert!(
            !rules.is_empty(),
            "{file_name} should contain at least one rule"
        );
        for rule in rules {
            rule.validate()
                .unwrap_or_else(|err| panic!("invalid rule `{}` in {file_name}: {err}", rule.name));
        }
    }
}

#[test]
fn rule_file_rule_counts_are_stable() {
    let counts: Vec<(&str, usize)> = RULE_FILES
        .iter()
        .map(|(file_name, source, _)| {
            let count = load_rules_from_str(source).unwrap().len();
            (*file_name, count)
        })
        .collect();
    assert!(counts.iter().all(|(_, count)| *count > 0));
    let total = counts.iter().map(|(_, count)| count).sum::<usize>();
    assert!(
        total > 200,
        "expected a large builtin rule set, got {total} rules: {counts:?}"
    );
}

#[test]
fn selected_semantic_rules_verify_via_matrix() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in [
        "cancel_h",
        "cancel_cx",
        "identity_hxh_to_z",
        "decompose_cx_to_cz",
        "decompose_swap_to_ising",
        "decompose_cz_to_rzz",
        "decompose_cx_to_rzz",
    ] {
        let rule = library.get_by_name(name).expect("selected semantic rule");
        assert_rule_verification_passes(rule);
    }
}

#[test]
fn cancel_xy2_inverse_pair_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in ["cancel_xy2p_xy2m", "cancel_xy2m_xy2p"] {
        let rule = library.get_by_name(name).expect("XY2 cancel rule");
        assert_rule_verification_passes(rule);
    }
}

#[test]
fn parametric_builtin_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in [
        "decompose_crz_to_rzz",
        "decompose_crx_to_rzz",
        "decompose_cry_to_rzz",
        "merge_rx",
        "merge_rzz",
        "decompose_rzz_to_cx",
    ] {
        let rule = library.get_by_name(name).expect("parametric builtin rule");
        let result = classify_rule_verification(rule);
        assert!(
            matches!(
                result,
                VerifyResult::SampledEqual { .. } | VerifyResult::Equivalent
            ),
            "rule `{name}` should pass layered verify, got {result:?}"
        );
    }
}

#[test]
fn decompose_crz_to_rzz_formula_matches_gate_definition() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let theta = 0.42;

    let mut crz = Circuit::new(2);
    crz.crz(q0, q1, theta).unwrap();

    let mut target_on_q1 = Circuit::new(2);
    target_on_q1.rz(q1, theta / 2.0).unwrap();
    target_on_q1.rzz(q0, q1, -theta / 2.0).unwrap();
    assert_circuits_equivalent_up_to_global_phase(&crz, &target_on_q1, 1e-9);

    let mut wrong_control_rz = Circuit::new(2);
    wrong_control_rz.rz(q0, theta / 2.0).unwrap();
    wrong_control_rz.rzz(q0, q1, -theta / 2.0).unwrap();
    assert!(
        !circuits_equivalent_up_to_global_phase(&crz, &wrong_control_rz, 1e-9),
        "RZ on control qubit should not match CRZ decomposition"
    );
}

#[test]
fn decompose_crx_and_cry_to_rzz_formulas_match_gate_definition() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let theta = 0.37;

    let mut crx = Circuit::new(2);
    crx.crx(q0, q1, theta).unwrap();
    let mut crx_decomposed = Circuit::new(2);
    crx_decomposed.h(q1).unwrap();
    crx_decomposed.rz(q1, theta / 2.0).unwrap();
    crx_decomposed.rzz(q0, q1, -theta / 2.0).unwrap();
    crx_decomposed.h(q1).unwrap();
    assert_circuits_equivalent_up_to_global_phase(&crx, &crx_decomposed, 1e-9);

    let mut cry = Circuit::new(2);
    cry.cry(q0, q1, theta).unwrap();
    let mut cry_decomposed = Circuit::new(2);
    cry_decomposed.rx(q1, PI / 2.0).unwrap();
    cry_decomposed.rz(q1, theta / 2.0).unwrap();
    cry_decomposed.rzz(q0, q1, -theta / 2.0).unwrap();
    cry_decomposed.rx(q1, -PI / 2.0).unwrap();
    assert_circuits_equivalent_up_to_global_phase(&cry, &cry_decomposed, 1e-9);
}

#[test]
fn decompose_cz_to_rzz_formula_matches_gate_definition() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    let mut cz = Circuit::new(2);
    cz.cz(q0, q1).unwrap();

    let mut decomposed = Circuit::new(2);
    decomposed.rz(q0, PI / 2.0).unwrap();
    decomposed.rz(q1, PI / 2.0).unwrap();
    decomposed.rzz(q0, q1, -PI / 2.0).unwrap();
    assert_circuits_equivalent_up_to_global_phase(&cz, &decomposed, 1e-9);
}

#[test]
fn decompose_cx_to_rzz_formula_matches_gate_definition() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    let mut cx = Circuit::new(2);
    cx.cx(q0, q1).unwrap();

    let mut decomposed = Circuit::new(2);
    decomposed.h(q1).unwrap();
    decomposed.rz(q0, PI / 2.0).unwrap();
    decomposed.rz(q1, PI / 2.0).unwrap();
    decomposed.rzz(q0, q1, -PI / 2.0).unwrap();
    decomposed.h(q1).unwrap();
    assert_circuits_equivalent_up_to_global_phase(&cx, &decomposed, 1e-9);
}

#[test]
fn decompose_cy_to_rzz_formula_matches_gate_definition() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    let mut cy = Circuit::new(2);
    cy.cy(q0, q1).unwrap();

    let mut decomposed = Circuit::new(2);
    decomposed.sdg(q1).unwrap();
    decomposed.h(q1).unwrap();
    decomposed.rz(q0, PI / 2.0).unwrap();
    decomposed.rz(q1, PI / 2.0).unwrap();
    decomposed.rzz(q0, q1, -PI / 2.0).unwrap();
    decomposed.h(q1).unwrap();
    decomposed.s(q1).unwrap();
    assert_circuits_equivalent_up_to_global_phase(&cy, &decomposed, 1e-9);
}

#[test]
fn new_rzz_native_lowering_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    let name = "decompose_cy_to_rzz";
    let rule = library.get_by_name(name).expect(name);
    assert_rule_verification_passes(rule);
}

#[test]
fn ion_trap_direct_ising_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in [
        "decompose_cz_to_rx_ry_rzz",
        "decompose_cx_to_rx_ry_rzz",
        "decompose_cy_to_rx_ry_rzz",
        "decompose_rxx_to_rx_ry_rzz",
        "decompose_ryy_to_rx_ry_rzz",
        "decompose_rzx_to_rx_ry_rzz",
        "decompose_crz_to_rx_ry_rzz",
        "decompose_crx_to_rx_ry_rzz",
        "decompose_cry_to_rx_ry_rzz",
        "decompose_fsim_to_rx_ry_rzz",
        "decompose_swap_to_rx_ry_rzz",
    ] {
        let rule = library.get_by_name(name).expect(name);
        assert_rule_verification_passes(rule);
    }
}

#[test]
fn ion_trap_direct_ccx_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in [
        "decompose_ccx_to_u_cx",
        "decompose_ccx_to_u_cz",
        "decompose_ccx_to_rz_x2p_cx",
        "decompose_ccx_to_rz_x2p_x_cz",
        "decompose_ccx_to_rx_ry_cx",
        "decompose_ccx_to_rx_ry_cz",
        "decompose_ccx_to_rx_ry_rxx",
        "decompose_ccx_to_rz_x2p_x_rzz",
        "decompose_h_ccx_h_to_rz_x2p_cz",
        "decompose_h_ccx_h_to_rz_cx",
        "decompose_h_ccx_h_to_rx_ry_rzz",
        "decompose_ccx_to_rx_ry_rzz",
    ] {
        let rule = library.get_by_name(name).expect(name);
        assert_rule_verification_passes(rule);
    }
}

#[test]
fn benchpress_frequent_direct_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in [
        "compose_cx_rz_cx_to_rzz",
        "compose_cx_phase_cx_to_rzz",
        "decompose_mcphase1_to_rz_cx",
        "decompose_mcphase1_to_rz_x2p_cz",
        "decompose_mcphase1_to_rx_ry_rzz",
        "decompose_mcswap1_to_rz_x2p_cz",
        "decompose_mcswap1_to_rz_x2p_cx",
        "decompose_mcswap1_to_rx_ry_rzz",
    ] {
        let rule = library.get_by_name(name).expect(name);
        assert_rule_verification_passes(rule);
    }
}

#[test]
fn coverage_gap_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in [
        "compose_cz_rx_cz_to_rzx",
        "compose_cz_rx_cz_to_rzx_swapped",
        "compose_cx_cz_to_cy",
        "compose_cx_cz_to_cy_cz_swapped",
        "compose_cz_cx_to_cy",
        "compose_cz_cx_to_cy_cz_swapped",
        "normalize_crx_zero",
        "normalize_cry_zero",
        "normalize_crz_zero",
        "normalize_mcphase1_zero",
        "normalize_mcphase2_zero",
        "specialize_fsim_zero_theta",
        "specialize_u_zero_theta",
        "cancel_mcx1",
        "cancel_mcx2",
        "cancel_mcx2_controls_swapped",
        "cancel_mcx3",
        "cancel_mcy1",
        "cancel_mcy2",
        "cancel_mcy2_controls_swapped",
        "cancel_mcz1",
        "cancel_mcz1_swapped",
        "cancel_mcz2",
        "cancel_mcz2_controls_swapped",
        "cancel_mch1",
        "cancel_mcswap1",
        "cancel_mcswap1_targets_swapped",
    ] {
        let rule = library.get_by_name(name).expect(name);
        assert_rule_verification_passes(rule);
    }
}

#[test]
fn ion_trap_direct_mc_gate_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in [
        "decompose_mch0_to_rx_ry",
        "decompose_mcrxx0_to_rx_ry_rzz",
        "decompose_mcrxy0_to_rx_ry",
        "decompose_mcryy0_to_rx_ry_rzz",
        "decompose_mcrz0_to_rx_ry",
        "decompose_mcrzx0_to_rx_ry_rzz",
        "decompose_mcs0_to_rx_ry",
        "decompose_mcsdg0_to_rx_ry",
        "decompose_mcswap0_to_rx_ry_rzz",
        "decompose_mct0_to_rx_ry",
        "decompose_mctdg0_to_rx_ry",
        "decompose_mcu0_to_rx_ry",
        "decompose_mcx0_to_rx",
        "decompose_mcxy0_to_rx_ry",
        "decompose_mcx2p0_to_rx",
        "decompose_mcx2m0_to_rx",
        "decompose_mcxy2p0_to_rx_ry",
        "decompose_mcxy2m0_to_rx_ry",
        "decompose_mcy0_to_ry",
        "decompose_mcy2p0_to_ry",
        "decompose_mcy2m0_to_ry",
        "decompose_mcz0_to_rx_ry",
        "decompose_mcphase0_to_rx_ry",
        "decompose_mccx0_to_rx_ry_rzz",
        "decompose_mcccx0_to_rx_ry_rzz",
        "decompose_mccy0_to_rx_ry_rzz",
        "decompose_mccz0_to_rx_ry_rzz",
        "decompose_mcfsim0_to_rx_ry_rzz",
        "decompose_mcx1_to_rx_ry_rzz",
        "decompose_mcx2_to_rx_ry_rzz",
        "decompose_mcy1_to_rx_ry_rzz",
        "decompose_mcz1_to_rx_ry_rzz",
        "decompose_mcrx1_to_rx_ry_rzz",
        "decompose_mcry1_to_rx_ry_rzz",
        "decompose_mcrz1_to_rx_ry_rzz",
        "decompose_mcy2_to_rx_ry_rzz",
        "decompose_mcz2_to_rx_ry_rzz",
        "decompose_mcs1_to_rx_ry_rzz",
        "decompose_mcsdg1_to_rx_ry_rzz",
        "decompose_mct1_to_rx_ry_rzz",
        "decompose_mctdg1_to_rx_ry_rzz",
        "decompose_mcphase1_to_rx_ry_rzz",
    ] {
        let rule = library.get_by_name(name).expect(name);
        assert_rule_verification_passes(rule);
    }
}

#[test]
fn ion_trap_direct_single_rotation_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in [
        "decompose_h_to_rx_ry",
        "decompose_s_to_rx_ry",
        "decompose_sdg_to_rx_ry",
        "decompose_t_to_rx_ry",
        "decompose_tdg_to_rx_ry",
        "decompose_z_to_rx_ry",
        "decompose_phase_to_rx_ry",
        "decompose_rxy_to_rx_ry",
        "decompose_xy_to_rx_ry",
        "decompose_xy2p_to_rx_ry",
        "decompose_xy2m_to_rx_ry",
        "decompose_u_to_rx_ry",
    ] {
        let rule = library.get_by_name(name).expect(name);
        assert_rule_verification_passes(rule);
    }
}

#[test]
fn new_ising_swapped_merge_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in ["merge_rxx_swapped", "merge_ryy_swapped"] {
        let rule = library.get_by_name(name).expect(name);
        assert_rule_verification_passes(rule);
    }
}

#[test]
fn merge_rxx_swapped_matches_direct_sum_unitary() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    let mut chained = Circuit::new(2);
    chained.rxx(q0, q1, 0.3).unwrap();
    chained.rxx(q1, q0, 0.4).unwrap();

    let mut merged = Circuit::new(2);
    merged.rxx(q0, q1, 0.7).unwrap();

    assert_circuits_equivalent_up_to_global_phase(&chained, &merged, 1e-9);
}

#[test]
fn merge_ryy_swapped_matches_direct_sum_unitary() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    let mut chained = Circuit::new(2);
    chained.ryy(q0, q1, 0.3).unwrap();
    chained.ryy(q1, q0, 0.4).unwrap();

    let mut merged = Circuit::new(2);
    merged.ryy(q0, q1, 0.7).unwrap();

    assert_circuits_equivalent_up_to_global_phase(&chained, &merged, 1e-9);
}

#[test]
fn merge_rzz_swapped_rule_passes_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    let rule = library
        .get_by_name("merge_rzz_swapped")
        .expect("merge_rzz_swapped");
    assert_rule_verification_passes(rule);
}

#[test]
fn merge_rzz_swapped_matches_direct_sum_unitary() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    let mut chained = Circuit::new(2);
    chained.rzz(q0, q1, 0.3).unwrap();
    chained.rzz(q1, q0, 0.4).unwrap();

    let mut merged = Circuit::new(2);
    merged.rzz(q0, q1, 0.7).unwrap();

    assert_circuits_equivalent_up_to_global_phase(&chained, &merged, 1e-9);
}

#[test]
fn ion_trap_rzz_intermediate_rules_are_present() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in [
        "merge_rzz",
        "merge_rzz_swapped",
        "merge_rxx_swapped",
        "merge_ryy_swapped",
        "cancel_rzz_inverse",
        "comm_rzz_rzz",
        "decompose_cry_to_rzz",
        "decompose_crx_to_rzz",
        "decompose_cz_to_rzz",
        "decompose_cx_to_rzz",
        "decompose_cy_to_rzz",
        "decompose_rzz_to_cx",
        "decompose_rzz_to_rxx",
        "decompose_rzz_to_rzx",
        "specialize_rzz_pi_to_cz",
        "decompose_swap_to_ising",
        "decompose_cz_to_rx_ry_rzz",
        "decompose_cx_to_rx_ry_rzz",
        "decompose_cy_to_rx_ry_rzz",
        "decompose_rxx_to_rx_ry_rzz",
        "decompose_ryy_to_rx_ry_rzz",
        "decompose_rzx_to_rx_ry_rzz",
        "decompose_crz_to_rx_ry_rzz",
        "decompose_crx_to_rx_ry_rzz",
        "decompose_cry_to_rx_ry_rzz",
        "decompose_fsim_to_rx_ry_rzz",
        "decompose_swap_to_rx_ry_rzz",
        "decompose_ccx_to_rx_ry_rzz",
    ] {
        assert!(
            library.get_by_name(name).is_some(),
            "expected ion-trap intermediate rule `{name}`"
        );
    }
}

#[test]
fn documented_missing_rules_for_rzz_native_targets() {
    let library = RuleLibrary::builtin_rules().unwrap();
    {
        let missing = "decompose_ms_to_rzz";
        assert!(
            library.get_by_name(missing).is_none(),
            "rule `{missing}` is not implemented yet (documented gap)"
        );
    }
}

#[test]
fn swap_sqrt_pauli_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in [
        "decompose_swap_to_x2p_cz",
        "decompose_swap_to_x2m_cz",
        "decompose_swap_to_y2m_cz",
        "decompose_swap_to_x2p_cx",
        "decompose_swap_to_y2m_cx",
        "decompose_swap_to_y2p_cz",
        "decompose_swap_to_x2p_x2m_cz",
        "decompose_swap_to_y2p_y2m_cz",
        "decompose_swap_to_x2p_y2p_cz",
        "decompose_swap_to_x2p_y2m_cz",
    ] {
        let rule = library.get_by_name(name).expect(name);
        assert_rule_verification_passes(rule);
    }
}

#[test]
fn rzx_target_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in [
        "decompose_cx_to_rzx",
        "decompose_cz_to_rzx",
        "decompose_cy_to_rzx",
        "decompose_swap_to_rzx",
        "decompose_ccx_to_rz_x2p_rzx",
    ] {
        let rule = library.get_by_name(name).expect(name);
        assert_rule_verification_passes(rule);
    }
}

#[test]
fn gphase_and_swap_commutation_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in [
        "comm_cx_t_ctrl",
        "comm_gphase_x",
        "comm_gphase_y",
        "comm_gphase_z",
        "comm_gphase_h",
        "comm_gphase_s",
        "comm_gphase_sdg",
        "comm_gphase_t",
        "comm_gphase_tdg",
        "comm_gphase_x2p",
        "comm_gphase_x2m",
        "comm_gphase_y2p",
        "comm_gphase_y2m",
        "comm_gphase_phase",
        "comm_gphase_rxy",
        "comm_gphase_u",
        "comm_gphase_rxx",
        "comm_gphase_ryy",
        "comm_gphase_rzz",
        "comm_gphase_rzx",
        "comm_gphase_swap",
        "comm_gphase_ccx",
        "comm_gphase_crx",
        "comm_gphase_cry",
        "comm_gphase_crz",
        "comm_gphase_fsim",
        "comm_swap_cz",
        "comm_swap_rxx",
        "comm_swap_ryy",
        "comm_swap_rzz",
        "comm_swap_fsim",
        "comm_rzz_cz",
        "comm_rzz_crz",
        "comm_rzz_crz_swapped",
        "comm_cz_crz",
        "comm_cz_crz_swapped",
        "comm_crz_crz",
        "comm_crz_crz_swapped",
    ] {
        let rule = library.get_by_name(name).expect(name);
        assert_rule_verification_passes(rule);
    }
}

#[test]
fn two_pi_and_special_angle_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in [
        "normalize_rx_2pi",
        "normalize_ry_2pi",
        "normalize_rz_2pi",
        "normalize_rxx_2pi",
        "normalize_ryy_2pi",
        "normalize_rzz_2pi",
        "normalize_rzx_2pi",
        "normalize_crx_2pi",
        "normalize_cry_2pi",
        "normalize_crz_2pi",
        "specialize_rzz_pi_2_to_cz",
        "specialize_rzz_neg_pi_2_to_cz",
        "specialize_rzx_pi",
        "specialize_fsim_pi_zero_to_zz",
        "specialize_fsim_to_cz",
        "merge_fsim",
        "merge_xy2p_xy2m_to_xy",
        "merge_xy2m_xy2p_to_xy",
    ] {
        let rule = library.get_by_name(name).expect(name);
        assert_rule_verification_passes(rule);
    }
}

#[test]
fn conjugation_and_pauli_identity_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in [
        "identity_cx_to_hczh",
        "identity_rz_to_phase_gphase",
        "identity_hx2ph_to_s",
        "identity_hx2mh_to_sdg",
        "identity_hy2ph_to_y2m",
        "identity_hy2mh_to_y2p",
        "identity_sysdg_to_x",
        "identity_sdg_xs_to_y",
        "identity_xy_to_z_phase",
        "identity_yx_to_z_phase",
        "identity_yz_to_x_phase",
        "identity_zy_to_x_phase",
        "identity_hth_to_rx",
        "identity_htdgh_to_rx",
    ] {
        let rule = library.get_by_name(name).expect(name);
        assert_rule_verification_passes(rule);
    }
}

#[test]
fn fragment_compose_and_move_rules_pass_layered_verify() {
    let library = RuleLibrary::builtin_rules().unwrap();
    for name in [
        "compose_cx_bridge_4cx",
        "compose_cx_bridge_4cx_rev",
        "compose_swap_cx_bridge",
        "compose_swap_cx_bridge_far",
        "compose_swap_cz_bridge",
        "compose_swap_swap_swap",
        "compose_cx3_to_swap",
        "compose_cx3_to_swap_rev",
        "compose_swap_cx_swap_to_cx_flip",
        "compose_swap_cz_swap_to_cz",
        "compose_swap_crz_swap",
        "compose_swap_crx_swap",
        "compose_hh_cx_hh_to_cx_flip",
        "compose_h1_cz_h1_to_cx",
        "compose_h0_cz_h0_to_cx_flip",
        "compose_h0_cx_h0_to_cz",
        "compose_h1_cx_h1_to_cz",
        "compose_y2m_cz_y2p_to_cx",
        "compose_y2p_cz_y2m_to_z_cx",
        "compose_x2p_cz_x2m_to_cy",
        "compose_cx_rx_cx_to_rxx",
        "move_x_ctrl_cx",
        "move_z_tgt_cx",
        "move_y_ctrl_cx",
        "move_x0_cz",
        "move_x1_cz",
    ] {
        let rule = library.get_by_name(name).expect(name);
        assert_rule_verification_passes(rule);
    }
}

fn swap_circuit() -> Circuit {
    let mut circuit = Circuit::new(2);
    circuit.swap(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit
}

fn circuits_strictly_equal(actual: &Circuit, expected: &Circuit, epsilon: f64) -> bool {
    let actual_matrix = circuit_to_matrix(actual, None).unwrap();
    let expected_matrix = circuit_to_matrix(expected, None).unwrap();
    actual_matrix
        .iter()
        .zip(expected_matrix.iter())
        .all(|(a, e)| (a - e).norm() < epsilon)
}

/// Asserts that a SWAP decomposition circuit equals the bare SWAP circuit
/// *exactly* (including the global-phase tail), not just up to global phase.
fn assert_strict_swap_equivalence(decomposed: &Circuit) {
    let expected = swap_circuit();
    let actual_matrix = circuit_to_matrix(decomposed, None).unwrap();
    let expected_matrix = circuit_to_matrix(&expected, None).unwrap();
    assert_matrix_approx_eq(&actual_matrix, &expected_matrix, 1e-9);
}

fn repeat_three_times(append_block: impl Fn(&mut Circuit)) -> Circuit {
    let mut circuit = Circuit::new(2);
    for _ in 0..3 {
        append_block(&mut circuit);
    }
    circuit
}

#[test]
fn decompose_swap_to_x2p_cz_formula_is_strictly_equal() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut decomposed = repeat_three_times(|c| {
        c.x2p(q0).unwrap();
        c.x2p(q1).unwrap();
        c.cz(q0, q1).unwrap();
    });
    decomposed.set_global_phase(Parameter::from(PI));
    assert_strict_swap_equivalence(&decomposed);
}

#[test]
fn decompose_swap_to_x2m_cz_formula_is_strictly_equal() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut decomposed = repeat_three_times(|c| {
        c.x2m(q0).unwrap();
        c.x2m(q1).unwrap();
        c.cz(q0, q1).unwrap();
    });
    decomposed.set_global_phase(Parameter::from(PI));
    assert_strict_swap_equivalence(&decomposed);
}

#[test]
fn decompose_swap_to_y2m_cz_formula_is_strictly_equal() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut decomposed = repeat_three_times(|c| {
        c.y2m(q0).unwrap();
        c.y2m(q1).unwrap();
        c.cz(q0, q1).unwrap();
    });
    decomposed.set_global_phase(Parameter::from(PI));
    assert_strict_swap_equivalence(&decomposed);
}

#[test]
fn decompose_swap_to_x2p_cx_formula_is_strictly_equal() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut decomposed = Circuit::new(2);
    for (control, target) in [(q0, q1), (q1, q0), (q0, q1)] {
        decomposed.x2p(q0).unwrap();
        decomposed.x2p(q1).unwrap();
        decomposed.cx(control, target).unwrap();
    }
    decomposed.set_global_phase(Parameter::from(-PI / 2.0));
    assert_strict_swap_equivalence(&decomposed);
}

#[test]
fn decompose_swap_to_y2m_cx_formula_is_strictly_equal() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut decomposed = Circuit::new(2);
    decomposed.y2m(q0).unwrap();
    decomposed.y2m(q0).unwrap();
    decomposed.cx(q0, q1).unwrap();
    for _ in 0..2 {
        decomposed.y2m(q0).unwrap();
        decomposed.y2m(q1).unwrap();
        decomposed.cx(q0, q1).unwrap();
    }
    decomposed.set_global_phase(Parameter::from(PI));
    assert_strict_swap_equivalence(&decomposed);
}

#[test]
fn decompose_swap_x2p_cz_triple_product_differs_from_swap_by_exactly_pi() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let expected = swap_circuit();

    // Without any phase tail the triple product equals exp(iπ)·SWAP = -SWAP:
    // it is equivalent up to global phase but NOT strictly equal.
    let bare = repeat_three_times(|c| {
        c.x2p(q0).unwrap();
        c.x2p(q1).unwrap();
        c.cz(q0, q1).unwrap();
    });
    assert_circuits_equivalent_up_to_global_phase(&bare, &expected, 1e-9);
    assert!(
        !circuits_strictly_equal(&bare, &expected, 1e-9),
        "bare (X2P⊗X2P)·CZ triple product must differ from SWAP by a global phase"
    );

    // The qiskit SX-convention tail -π/2 is wrong for the phase-free X2P
    // convention used here: it yields exp(iπ/2)·SWAP = i·SWAP.
    let mut wrong_phase = repeat_three_times(|c| {
        c.x2p(q0).unwrap();
        c.x2p(q1).unwrap();
        c.cz(q0, q1).unwrap();
    });
    wrong_phase.set_global_phase(Parameter::from(-PI / 2.0));
    assert_circuits_equivalent_up_to_global_phase(&wrong_phase, &expected, 1e-9);
    assert!(
        !circuits_strictly_equal(&wrong_phase, &expected, 1e-9),
        "GPhase(-π/2) tail must not be strictly equal to SWAP for phase-free X2P"
    );
}

#[test]
fn decompose_swap_to_y2p_cz_formula_is_strictly_equal() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut decomposed = repeat_three_times(|c| {
        c.y2p(q0).unwrap();
        c.y2p(q1).unwrap();
        c.cz(q0, q1).unwrap();
    });
    decomposed.set_global_phase(Parameter::from(PI));
    assert_strict_swap_equivalence(&decomposed);
}

#[test]
fn decompose_swap_to_x2p_x2m_cz_formula_is_strictly_equal() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut decomposed = Circuit::new(2);
    for (g0, g1) in [(true, false), (false, true), (true, false)] {
        if g0 {
            decomposed.x2p(q0).unwrap();
        } else {
            decomposed.x2m(q0).unwrap();
        }
        if g1 {
            decomposed.x2p(q1).unwrap();
        } else {
            decomposed.x2m(q1).unwrap();
        }
        decomposed.cz(q0, q1).unwrap();
    }
    assert_strict_swap_equivalence(&decomposed);
}

#[test]
fn decompose_swap_to_y2p_y2m_cz_formula_is_strictly_equal() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut decomposed = Circuit::new(2);
    for (g0, g1) in [(true, false), (false, true), (true, false)] {
        if g0 {
            decomposed.y2p(q0).unwrap();
        } else {
            decomposed.y2m(q0).unwrap();
        }
        if g1 {
            decomposed.y2p(q1).unwrap();
        } else {
            decomposed.y2m(q1).unwrap();
        }
        decomposed.cz(q0, q1).unwrap();
    }
    assert_strict_swap_equivalence(&decomposed);
}

#[test]
fn decompose_swap_to_x2p_y2p_cz_formula_is_strictly_equal() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut decomposed = repeat_three_times(|c| {
        c.x2p(q0).unwrap();
        c.y2p(q1).unwrap();
        c.cz(q0, q1).unwrap();
    });
    decomposed.rz(q0, PI / 2.0).unwrap();
    decomposed.rz(q1, -PI / 2.0).unwrap();
    decomposed.set_global_phase(Parameter::from(PI));
    assert_strict_swap_equivalence(&decomposed);
}

#[test]
fn decompose_swap_to_x2p_y2m_cz_formula_is_strictly_equal() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut decomposed = repeat_three_times(|c| {
        c.x2p(q0).unwrap();
        c.y2m(q1).unwrap();
        c.cz(q0, q1).unwrap();
    });
    decomposed.rz(q0, 3.0 * PI / 2.0).unwrap();
    decomposed.rz(q1, PI / 2.0).unwrap();
    assert_strict_swap_equivalence(&decomposed);
}

#[test]
fn decompose_cx_to_rzx_formula_is_strictly_equal() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut expected = Circuit::new(2);
    expected.cx(q0, q1).unwrap();

    let mut decomposed = Circuit::new(2);
    decomposed.rz(q0, PI / 2.0).unwrap();
    decomposed.rx(q1, PI / 2.0).unwrap();
    decomposed.rzx(q0, q1, -PI / 2.0).unwrap();
    decomposed.set_global_phase(Parameter::from(PI / 4.0));
    assert_strict_circuit_equivalence(&decomposed, &expected);
}

#[test]
fn decompose_cz_to_rzx_formula_is_strictly_equal() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut expected = Circuit::new(2);
    expected.cz(q0, q1).unwrap();

    let mut decomposed = Circuit::new(2);
    decomposed.h(q1).unwrap();
    decomposed.rz(q0, PI / 2.0).unwrap();
    decomposed.rx(q1, PI / 2.0).unwrap();
    decomposed.rzx(q0, q1, -PI / 2.0).unwrap();
    decomposed.h(q1).unwrap();
    decomposed.set_global_phase(Parameter::from(PI / 4.0));
    assert_strict_circuit_equivalence(&decomposed, &expected);
}

#[test]
fn decompose_cy_to_rzx_formula_is_strictly_equal() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut expected = Circuit::new(2);
    expected.cy(q0, q1).unwrap();

    let mut decomposed = Circuit::new(2);
    decomposed.sdg(q1).unwrap();
    decomposed.rz(q0, PI / 2.0).unwrap();
    decomposed.rx(q1, PI / 2.0).unwrap();
    decomposed.rzx(q0, q1, -PI / 2.0).unwrap();
    decomposed.s(q1).unwrap();
    decomposed.set_global_phase(Parameter::from(PI / 4.0));
    assert_strict_circuit_equivalence(&decomposed, &expected);
}

#[test]
fn decompose_swap_to_rzx_formula_is_strictly_equal() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut decomposed = Circuit::new(2);
    decomposed.rz(q0, PI / 2.0).unwrap();
    decomposed.rx(q1, PI / 2.0).unwrap();
    decomposed.rzx(q0, q1, -PI / 2.0).unwrap();
    decomposed.rz(q1, PI / 2.0).unwrap();
    decomposed.rx(q0, PI / 2.0).unwrap();
    decomposed.rzx(q1, q0, -PI / 2.0).unwrap();
    decomposed.rz(q0, PI / 2.0).unwrap();
    decomposed.rx(q1, PI / 2.0).unwrap();
    decomposed.rzx(q0, q1, -PI / 2.0).unwrap();
    decomposed.set_global_phase(Parameter::from(3.0 * PI / 4.0));
    assert_strict_swap_equivalence(&decomposed);
}

#[test]
fn decompose_ccx_to_rz_x2p_rzx_formula_is_strictly_equal() {
    let (q0, q1, q2) = (Qubit::new(0), Qubit::new(1), Qubit::new(2));
    let mut expected = Circuit::new(3);
    expected.ccx(q0, q1, q2).unwrap();

    let mut decomposed = Circuit::new(3);
    decomposed.rz(q2, PI / 2.0).unwrap();
    decomposed.x2p(q2).unwrap();
    decomposed.rz(q2, PI / 2.0).unwrap();
    decomposed.rz(q1, PI / 2.0).unwrap();
    decomposed.x2p(q2).unwrap();
    decomposed.rzx(q1, q2, -PI / 2.0).unwrap();
    decomposed.rz(q2, -PI / 4.0).unwrap();
    decomposed.rz(q0, PI / 2.0).unwrap();
    decomposed.x2p(q2).unwrap();
    decomposed.rzx(q0, q2, -PI / 2.0).unwrap();
    decomposed.rz(q2, PI / 4.0).unwrap();
    decomposed.rz(q1, PI / 2.0).unwrap();
    decomposed.x2p(q2).unwrap();
    decomposed.rzx(q1, q2, -PI / 2.0).unwrap();
    decomposed.rz(q1, PI / 4.0).unwrap();
    decomposed.rz(q2, -PI / 4.0).unwrap();
    decomposed.rz(q0, PI / 2.0).unwrap();
    decomposed.x2p(q2).unwrap();
    decomposed.rzx(q0, q2, -PI / 2.0).unwrap();
    decomposed.rz(q0, PI / 2.0).unwrap();
    decomposed.x2p(q1).unwrap();
    decomposed.rzx(q0, q1, -PI / 2.0).unwrap();
    decomposed.rz(q0, PI / 4.0).unwrap();
    decomposed.rz(q1, -PI / 4.0).unwrap();
    decomposed.rz(q0, PI / 2.0).unwrap();
    decomposed.x2p(q1).unwrap();
    decomposed.rzx(q0, q1, -PI / 2.0).unwrap();
    decomposed.rz(q2, 3.0 * PI / 4.0).unwrap();
    decomposed.x2p(q2).unwrap();
    decomposed.rz(q2, PI / 2.0).unwrap();
    decomposed.set_global_phase(Parameter::from(5.0 * PI / 8.0));
    assert_strict_circuit_equivalence(&decomposed, &expected);
}
fn assert_strict_circuit_equivalence(actual: &Circuit, expected: &Circuit) {
    let actual_matrix = circuit_to_matrix(actual, None).unwrap();
    let expected_matrix = circuit_to_matrix(expected, None).unwrap();
    assert_matrix_approx_eq(&actual_matrix, &expected_matrix, 1e-9);
}

#[test]
fn decompose_cx_to_rx_ry_rzz_formula_is_strictly_equal() {
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    let mut expected = Circuit::new(2);
    expected.cx(q0, q1).unwrap();

    let mut decomposed = Circuit::new(2);
    decomposed.ry(q1, PI / 2.0).unwrap();
    decomposed.rx(q1, PI).unwrap();
    decomposed.rx(q0, -PI / 2.0).unwrap();
    decomposed.ry(q0, PI / 2.0).unwrap();
    decomposed.rx(q0, PI / 2.0).unwrap();
    decomposed.rx(q1, -PI / 2.0).unwrap();
    decomposed.ry(q1, PI / 2.0).unwrap();
    decomposed.rx(q1, PI / 2.0).unwrap();
    decomposed.rzz(q0, q1, -PI / 2.0).unwrap();
    decomposed.ry(q1, PI / 2.0).unwrap();
    decomposed.rx(q1, PI).unwrap();
    decomposed.set_global_phase(Parameter::from(-3.0 * PI / 4.0));
    assert_strict_circuit_equivalence(&decomposed, &expected);
}

#[test]
fn decompose_ccx_to_rz_x2p_cx_formula_is_strictly_equal() {
    let (q0, q1, q2) = (Qubit::new(0), Qubit::new(1), Qubit::new(2));
    let mut expected = Circuit::new(3);
    expected.ccx(q0, q1, q2).unwrap();

    let mut decomposed = Circuit::new(3);
    decomposed.rz(q2, PI / 2.0).unwrap();
    decomposed.x2p(q2).unwrap();
    decomposed.rz(q2, PI / 2.0).unwrap();
    decomposed.cx(q1, q2).unwrap();
    decomposed.rz(q2, -PI / 4.0).unwrap();
    decomposed.cx(q0, q2).unwrap();
    decomposed.rz(q2, PI / 4.0).unwrap();
    decomposed.cx(q1, q2).unwrap();
    decomposed.rz(q1, PI / 4.0).unwrap();
    decomposed.rz(q2, -PI / 4.0).unwrap();
    decomposed.cx(q0, q2).unwrap();
    decomposed.cx(q0, q1).unwrap();
    decomposed.rz(q0, PI / 4.0).unwrap();
    decomposed.rz(q1, -PI / 4.0).unwrap();
    decomposed.cx(q0, q1).unwrap();
    decomposed.rz(q2, 3.0 * PI / 4.0).unwrap();
    decomposed.x2p(q2).unwrap();
    decomposed.rz(q2, PI / 2.0).unwrap();
    decomposed.set_global_phase(Parameter::from(-7.0 * PI / 8.0));
    assert_strict_circuit_equivalence(&decomposed, &expected);
}

#[test]
fn decompose_ccx_to_rz_x2p_x_rzz_formula_is_strictly_equal() {
    let (q0, q1, q2) = (Qubit::new(0), Qubit::new(1), Qubit::new(2));
    let mut expected = Circuit::new(3);
    expected.ccx(q0, q1, q2).unwrap();

    let mut decomposed = Circuit::new(3);
    decomposed.x(q1).unwrap();
    decomposed.rz(q2, PI / 2.0).unwrap();
    decomposed.x2p(q2).unwrap();
    decomposed.rz(q2, PI / 2.0).unwrap();
    decomposed.rzz(q1, q2, PI / 4.0).unwrap();
    decomposed.rz(q1, PI / 4.0).unwrap();
    decomposed.x2p(q1).unwrap();
    decomposed.rz(q1, -PI / 2.0).unwrap();
    decomposed.rzz(q0, q1, -PI / 2.0).unwrap();
    decomposed.rz(q0, PI / 2.0).unwrap();
    decomposed.x2p(q1).unwrap();
    decomposed.rz(q1, -PI).unwrap();
    decomposed.x(q2).unwrap();
    decomposed.rz(q2, -PI / 4.0).unwrap();
    decomposed.rzz(q1, q2, PI / 4.0).unwrap();
    decomposed.rz(q1, PI / 4.0).unwrap();
    decomposed.x2p(q1).unwrap();
    decomposed.rz(q1, -PI / 2.0).unwrap();
    decomposed.rzz(q0, q1, -PI / 2.0).unwrap();
    decomposed.rz(q0, PI / 2.0).unwrap();
    decomposed.x(q0).unwrap();
    decomposed.rz(q1, -PI).unwrap();
    decomposed.x2p(q1).unwrap();
    decomposed.rz(q1, PI / 2.0).unwrap();
    decomposed.x(q2).unwrap();
    decomposed.rz(q2, -PI / 4.0).unwrap();
    decomposed.rzz(q0, q2, PI / 4.0).unwrap();
    decomposed.rz(q0, -PI / 4.0).unwrap();
    decomposed.x(q0).unwrap();
    decomposed.rz(q2, 3.0 * PI / 4.0).unwrap();
    decomposed.x2p(q2).unwrap();
    decomposed.rz(q2, PI / 2.0).unwrap();
    decomposed.set_global_phase(Parameter::from(PI / 8.0));
    assert_strict_circuit_equivalence(&decomposed, &expected);
}

#[test]
fn decompose_ccx_to_rx_ry_rzz_formula_is_strictly_equal() {
    let (q0, q1, q2) = (Qubit::new(0), Qubit::new(1), Qubit::new(2));
    let mut expected = Circuit::new(3);
    expected.ccx(q0, q1, q2).unwrap();

    let mut decomposed = Circuit::new(3);
    decomposed.rx(q1, PI).unwrap();
    decomposed.ry(q2, PI / 2.0).unwrap();
    decomposed.rx(q2, PI).unwrap();
    decomposed.rzz(q1, q2, PI / 4.0).unwrap();
    decomposed.ry(q1, -PI / 2.0).unwrap();
    decomposed.rx(q1, PI / 4.0).unwrap();
    decomposed.rzz(q0, q1, PI / 2.0).unwrap();
    decomposed.rx(q0, PI / 2.0).unwrap();
    decomposed.ry(q0, PI / 2.0).unwrap();
    decomposed.rx(q0, -PI / 2.0).unwrap();
    decomposed.rx(q1, -PI / 2.0).unwrap();
    decomposed.rx(q2, -PI / 2.0).unwrap();
    decomposed.ry(q2, PI / 4.0).unwrap();
    decomposed.rx(q2, -PI / 2.0).unwrap();
    decomposed.rzz(q1, q2, PI / 4.0).unwrap();
    decomposed.ry(q1, -PI / 2.0).unwrap();
    decomposed.rx(q1, PI / 4.0).unwrap();
    decomposed.rzz(q0, q1, PI / 2.0).unwrap();
    decomposed.rx(q0, PI / 2.0).unwrap();
    decomposed.ry(q0, PI / 2.0).unwrap();
    decomposed.rx(q0, PI / 2.0).unwrap();
    decomposed.ry(q1, PI / 2.0).unwrap();
    decomposed.rx(q1, PI / 2.0).unwrap();
    decomposed.rx(q2, -PI / 2.0).unwrap();
    decomposed.ry(q2, PI / 4.0).unwrap();
    decomposed.rx(q2, -PI / 2.0).unwrap();
    decomposed.rzz(q0, q2, PI / 4.0).unwrap();
    decomposed.rx(q0, PI / 2.0).unwrap();
    decomposed.ry(q0, PI / 4.0).unwrap();
    decomposed.rx(q0, PI / 2.0).unwrap();
    decomposed.ry(q2, PI / 2.0).unwrap();
    decomposed.rx(q2, -3.0 * PI / 4.0).unwrap();
    decomposed.set_global_phase(Parameter::from(5.0 * PI / 8.0));
    assert_strict_circuit_equivalence(&decomposed, &expected);
}

#[test]
fn decompose_h_ccx_h_to_rx_ry_rzz_formula_is_strictly_equal() {
    let (q0, q1, q2) = (Qubit::new(0), Qubit::new(1), Qubit::new(2));
    let mut expected = Circuit::new(3);
    expected.h(q2).unwrap();
    expected.ccx(q0, q1, q2).unwrap();
    expected.h(q2).unwrap();

    let mut decomposed = Circuit::new(3);
    decomposed.rx(q1, PI).unwrap();
    decomposed.rzz(q1, q2, PI / 4.0).unwrap();
    decomposed.ry(q1, -PI / 2.0).unwrap();
    decomposed.rx(q1, PI / 4.0).unwrap();
    decomposed.rzz(q0, q1, PI / 2.0).unwrap();
    decomposed.rx(q0, PI / 2.0).unwrap();
    decomposed.ry(q0, PI / 2.0).unwrap();
    decomposed.rx(q0, -PI / 2.0).unwrap();
    decomposed.rx(q1, -PI / 2.0).unwrap();
    decomposed.rx(q2, -PI / 2.0).unwrap();
    decomposed.ry(q2, PI / 4.0).unwrap();
    decomposed.rx(q2, -PI / 2.0).unwrap();
    decomposed.rzz(q1, q2, PI / 4.0).unwrap();
    decomposed.ry(q1, -PI / 2.0).unwrap();
    decomposed.rx(q1, PI / 4.0).unwrap();
    decomposed.rzz(q0, q1, PI / 2.0).unwrap();
    decomposed.rx(q0, PI / 2.0).unwrap();
    decomposed.ry(q0, PI / 2.0).unwrap();
    decomposed.rx(q0, PI / 2.0).unwrap();
    decomposed.ry(q1, PI / 2.0).unwrap();
    decomposed.rx(q1, PI / 2.0).unwrap();
    decomposed.rx(q2, -PI / 2.0).unwrap();
    decomposed.ry(q2, PI / 4.0).unwrap();
    decomposed.rx(q2, -PI / 2.0).unwrap();
    decomposed.rzz(q0, q2, PI / 4.0).unwrap();
    decomposed.rx(q0, PI / 2.0).unwrap();
    decomposed.ry(q0, PI / 4.0).unwrap();
    decomposed.rx(q0, PI / 2.0).unwrap();
    decomposed.rx(q2, -PI / 2.0).unwrap();
    decomposed.ry(q2, PI / 4.0).unwrap();
    decomposed.rx(q2, PI / 2.0).unwrap();
    decomposed.set_global_phase(Parameter::from(5.0 * PI / 8.0));
    assert_strict_circuit_equivalence(&decomposed, &expected);
}
