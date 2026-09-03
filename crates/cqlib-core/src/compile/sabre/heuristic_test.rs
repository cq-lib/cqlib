// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::*;

#[test]
fn deterministic_seeded_config_uses_compact_reproducible_settings() {
    let config = SabreConfig::deterministic_seeded(7);

    assert_eq!(config.layout_trials, 2);
    assert_eq!(config.layout_assignment_budget, 100_000);
    assert_eq!(config.vf2_prepass.unwrap().call_limit, 100_000);
    assert_eq!(config.refinement_iterations, 1);
    assert_eq!(config.routing_trials, 1);
    assert_eq!(config.seed, Some(7));
    assert_eq!(
        config.heuristic.lookahead_weights,
        vec![0.5, 0.25, 0.125, 0.0625, 0.03125]
    );
    assert_eq!(config.heuristic.decay_increment, Some(0.002));
    assert_eq!(config.heuristic.decay_reset, 10);
    assert_eq!(config.heuristic.attempt_limit, 20);
}

#[test]
fn validation_accepts_zero_weights_and_disabled_decay_with_zero_reset() {
    let config = SabreHeuristicConfig {
        basic_weight: 0.0,
        lookahead_weights: vec![0.0, 1.0],
        decay_increment: None,
        decay_reset: 0,
        best_epsilon: 0.0,
        ..SabreHeuristicConfig::default()
    };

    config.validate().unwrap();
}

#[test]
fn validation_rejects_non_finite_and_negative_weights() {
    for invalid in [-1.0, f64::NAN, f64::INFINITY] {
        let basic = SabreHeuristicConfig {
            basic_weight: invalid,
            ..SabreHeuristicConfig::default()
        };
        let lookahead = SabreHeuristicConfig {
            lookahead_weights: vec![0.5, invalid],
            ..SabreHeuristicConfig::default()
        };
        let decay = SabreHeuristicConfig {
            decay_increment: Some(invalid),
            ..SabreHeuristicConfig::default()
        };

        assert!(
            matches!(basic.validate(), Err(CompilerError::InvalidInput(message)) if message.contains("basic_weight"))
        );
        assert!(
            matches!(lookahead.validate(), Err(CompilerError::InvalidInput(message)) if message.contains("lookahead_weights[1]"))
        );
        assert!(
            matches!(decay.validate(), Err(CompilerError::InvalidInput(message)) if message.contains("decay_increment"))
        );
    }
}

#[test]
fn validation_requires_a_positive_reset_when_decay_is_enabled() {
    let config = SabreHeuristicConfig {
        decay_increment: Some(0.0),
        decay_reset: 0,
        ..SabreHeuristicConfig::default()
    };

    assert!(matches!(
        config.validate(),
        Err(CompilerError::InvalidInput(message)) if message.contains("decay_reset")
    ));
}

#[test]
fn validation_rejects_invalid_tie_epsilon() {
    for invalid in [-1.0, f64::NAN, f64::INFINITY] {
        let config = SabreHeuristicConfig {
            best_epsilon: invalid,
            ..SabreHeuristicConfig::default()
        };

        assert!(matches!(
            config.validate(),
            Err(CompilerError::InvalidInput(message)) if message.contains("best_epsilon")
        ));
    }
}
