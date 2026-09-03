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

use super::*;
use crate::circuit::StandardGate;
use crate::compile::transform::decompose::unitary::TwoQubitSynthesisTarget;

fn target(gate: StandardGate) -> TwoQubitSynthesisTarget {
    TwoQubitSynthesisTarget::from_standard_gates(vec![StandardGate::U], vec![gate], true).unwrap()
}

#[test]
fn normal_config_uses_bounded_default_search_budget() {
    let config = TwoQubitBlockResynthesisConfig::normal(target(StandardGate::CX));

    assert_eq!(config.two_qubit_target.native_2q(), &[StandardGate::CX]);
    assert_eq!(config.max_block_ops, 16);
    assert_eq!(config.max_crossed_ops, 4);
    assert_eq!(config.max_scan_span, 32);
    assert!(config.skip_labeled_ops);
    assert!(config.recurse_control_flow);
    assert!(config.commutation.enable_rule_oracle);
    assert!(!config.commutation.enable_matrix_fallback);
}

#[test]
fn enhanced_config_increases_only_search_budgets() {
    let normal = TwoQubitBlockResynthesisConfig::normal(target(StandardGate::CZ));
    let enhanced = TwoQubitBlockResynthesisConfig::enhanced(target(StandardGate::CZ));

    assert_eq!(enhanced.two_qubit_target, normal.two_qubit_target);
    assert!(enhanced.max_block_ops > normal.max_block_ops);
    assert!(enhanced.max_crossed_ops > normal.max_crossed_ops);
    assert!(enhanced.max_scan_span > normal.max_scan_span);
    assert_eq!(enhanced.skip_labeled_ops, normal.skip_labeled_ops);
    assert_eq!(enhanced.recurse_control_flow, normal.recurse_control_flow);
    assert_eq!(enhanced.commutation, normal.commutation);
}
