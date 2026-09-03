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

//! Tests for the parameter formatter module.

use super::*;
use crate::circuit::circuit_param::CircuitParam;
use crate::circuit::parameter::Parameter;
use crate::circuit::{Circuit, Qubit};
use crate::visualization::ParameterDisplayMode;
use std::f64::consts::PI;

#[test]
fn test_default_parameter_format_options() {
    let opts = ParameterFormatOptions::default();
    assert_eq!(opts.mode, ParameterDisplayMode::Numeric);
    assert_eq!(opts.decimal_precision, 2);
    assert_eq!(opts.scientific_lower_bound, 1e-3);
    assert_eq!(opts.scientific_upper_bound, 1e4);
    assert_eq!(opts.pi_tolerance, 1e-3);
    assert_eq!(opts.pi_max_denominator, 16);
}

#[test]
fn test_formatter_with_numeric_mode() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions {
        mode: ParameterDisplayMode::Numeric,
        ..ParameterFormatOptions::default()
    });

    let circuit = Circuit::new(0);

    assert_eq!(
        formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(1.2345))
            .unwrap(),
        "1.23"
    );

    assert_eq!(
        formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(-2.567))
            .unwrap(),
        "-2.57"
    );
}

#[test]
fn test_formatter_with_scientific_notation_for_small_values() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions::default());
    let circuit = Circuit::new(0);

    assert_eq!(
        formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(0.0004))
            .unwrap(),
        "4e-4"
    );

    assert_eq!(
        formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(0.00001))
            .unwrap(),
        "1e-5"
    );
}

#[test]
fn test_formatter_with_scientific_notation_for_large_values() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions::default());
    let circuit = Circuit::new(0);

    assert_eq!(
        formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(15000.0))
            .unwrap(),
        "1.5e4"
    );

    assert_eq!(
        formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(100000.0))
            .unwrap(),
        "1e5"
    );
}

#[test]
fn test_formatter_with_pi_fraction_preferred_mode() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions {
        mode: ParameterDisplayMode::PiFractionPreferred,
        ..ParameterFormatOptions::default()
    });
    let circuit = Circuit::new(0);

    assert_eq!(
        formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(PI))
            .unwrap(),
        "π"
    );

    assert_eq!(
        formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(PI / 2.0))
            .unwrap(),
        "π/2"
    );

    assert_eq!(
        formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(PI / 4.0))
            .unwrap(),
        "π/4"
    );

    assert_eq!(
        formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(-PI / 2.0))
            .unwrap(),
        "-π/2"
    );

    assert_eq!(
        formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(2.0 * PI))
            .unwrap(),
        "2π"
    );
}

#[test]
fn test_formatter_pi_fraction_with_common_angles() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions {
        mode: ParameterDisplayMode::PiFractionPreferred,
        pi_tolerance: 1e-3,
        pi_max_denominator: 16,
        ..ParameterFormatOptions::default()
    });
    let circuit = Circuit::new(0);

    let common_angles = vec![
        (PI / 3.0, "π/3"),
        (PI / 6.0, "π/6"),
        (3.0 * PI / 4.0, "3π/4"),
        (5.0 * PI / 6.0, "5π/6"),
    ];

    for (angle, expected) in common_angles {
        let result = formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(angle))
            .unwrap();
        assert_eq!(result, expected);
    }
}

#[test]
fn test_formatter_falls_back_to_numeric_for_non_common_pi_values() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions {
        mode: ParameterDisplayMode::PiFractionPreferred,
        ..ParameterFormatOptions::default()
    });
    let circuit = Circuit::new(0);

    let result = formatter
        .format_circuit_param(&circuit, &CircuitParam::Fixed(0.12345))
        .unwrap();
    assert_eq!(result, "0.12");
}

#[test]
fn test_formatter_with_symbolic_mode() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions {
        mode: ParameterDisplayMode::Symbolic,
        ..ParameterFormatOptions::default()
    });

    let mut circuit = Circuit::new(1);
    let theta = Parameter::symbol("theta");
    circuit.rx(Qubit::new(0), theta).unwrap();

    let result = formatter
        .format_circuit_param(&circuit, &CircuitParam::Index(0))
        .unwrap();
    assert_eq!(result, "theta");
}

#[test]
fn test_formatter_with_symbolic_with_value_mode() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions {
        mode: ParameterDisplayMode::SymbolicWithValue,
        ..ParameterFormatOptions::default()
    });

    let mut circuit = Circuit::new(1);
    let theta = Parameter::symbol("theta");
    circuit.rx(Qubit::new(0), theta).unwrap();

    let result = formatter
        .format_circuit_param(&circuit, &CircuitParam::Index(0))
        .unwrap();
    assert_eq!(result, "theta");
}

#[test]
fn test_formatter_with_symbolic_expression() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions {
        mode: ParameterDisplayMode::Symbolic,
        ..ParameterFormatOptions::default()
    });

    let mut circuit = Circuit::new(1);
    let theta = Parameter::symbol("theta");
    let expr = theta + 1.0;
    circuit.rx(Qubit::new(0), expr).unwrap();

    let result = formatter
        .format_circuit_param(&circuit, &CircuitParam::Index(0))
        .unwrap();
    assert!(result.contains("theta"));
}

#[test]
fn test_formatter_normalizes_numeric_literals_in_symbolic_expression() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions {
        mode: ParameterDisplayMode::Symbolic,
        ..ParameterFormatOptions::default()
    });

    let mut circuit = Circuit::new(1);
    let evo_t = Parameter::symbol("evo_t");
    circuit
        .rz(Qubit::new(0), Parameter::from(0.07333333333333333) * evo_t)
        .unwrap();

    let result = formatter
        .format_circuit_param(&circuit, &CircuitParam::Index(0))
        .unwrap();
    assert_eq!(result, "0.07*evo_t");
}

#[test]
fn test_formatter_keeps_digits_inside_symbol_names() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions {
        mode: ParameterDisplayMode::Symbolic,
        ..ParameterFormatOptions::default()
    });

    let mut circuit = Circuit::new(1);
    let x_11 = Parameter::symbol("x_11");
    circuit
        .rz(Qubit::new(0), x_11 + Parameter::from(0.07333333333333333))
        .unwrap();

    let result = formatter
        .format_circuit_param(&circuit, &CircuitParam::Index(0))
        .unwrap();
    assert!(result.contains("x_11"));
    assert!(result.contains("0.07"));
    assert!(!result.contains("0.07333333333333333"));
}

#[test]
fn test_formatter_with_zero_value() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions::default());
    let circuit = Circuit::new(0);

    assert_eq!(
        formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(0.0))
            .unwrap(),
        "0"
    );
}

#[test]
fn test_formatter_with_negative_zero() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions::default());
    let circuit = Circuit::new(0);

    let result = formatter
        .format_circuit_param(&circuit, &CircuitParam::Fixed(-0.0))
        .unwrap();
    assert_eq!(result, "0");
}

#[test]
fn test_formatter_precision_option() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions {
        decimal_precision: 4,
        ..ParameterFormatOptions::default()
    });
    let circuit = Circuit::new(0);

    assert_eq!(
        formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(1.23456))
            .unwrap(),
        "1.2346"
    );
}

#[test]
fn test_formatter_boundary_values_for_scientific_notation() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions::default());
    let circuit = Circuit::new(0);

    // Very small value should use scientific notation
    let very_small = 0.0004;
    let result = formatter
        .format_circuit_param(&circuit, &CircuitParam::Fixed(very_small))
        .unwrap();
    assert!(result.contains("e"));

    // Value at lower bound - just verify it formats without error
    let at_lower = 0.001;
    let result = formatter
        .format_circuit_param(&circuit, &CircuitParam::Fixed(at_lower))
        .unwrap();
    assert!(!result.is_empty());

    // Large value below upper bound should not use scientific notation
    let large = 9999.0;
    let result = formatter
        .format_circuit_param(&circuit, &CircuitParam::Fixed(large))
        .unwrap();
    assert!(!result.contains("e"));

    // Value at/above upper bound should use scientific notation
    let very_large = 10000.0;
    let result = formatter
        .format_circuit_param(&circuit, &CircuitParam::Fixed(very_large))
        .unwrap();
    assert!(result.contains("e"));
}

#[test]
fn test_formatter_pi_tolerance_matching() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions {
        mode: ParameterDisplayMode::PiFractionPreferred,
        pi_tolerance: 1e-2,
        ..ParameterFormatOptions::default()
    });
    let circuit = Circuit::new(0);

    let slightly_off_pi = PI + 0.005;
    let result = formatter
        .format_circuit_param(&circuit, &CircuitParam::Fixed(slightly_off_pi))
        .unwrap();
    assert_eq!(result, "π");
}

#[test]
fn test_formatter_pi_max_denominator_limit() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions {
        mode: ParameterDisplayMode::PiFractionPreferred,
        pi_max_denominator: 4,
        ..ParameterFormatOptions::default()
    });
    let circuit = Circuit::new(0);

    let result = formatter
        .format_circuit_param(&circuit, &CircuitParam::Fixed(PI / 5.0))
        .unwrap();
    assert_ne!(result, "π/5");
}

#[test]
fn test_parameter_display_mode_clone_copy() {
    let mode = ParameterDisplayMode::Numeric;
    let mode_clone = mode;
    assert_eq!(mode, mode_clone);

    let mode_copy = mode;
    assert_eq!(mode, mode_copy);
}

#[test]
fn test_parameter_format_options_clone_copy() {
    let opts = ParameterFormatOptions::default();
    let opts_clone = opts;
    assert_eq!(opts.mode, opts_clone.mode);

    let opts_copy = opts;
    assert_eq!(opts_copy.mode, ParameterDisplayMode::Numeric);
}

#[test]
fn test_formatter_new_constructor() {
    let opts = ParameterFormatOptions {
        mode: ParameterDisplayMode::PiFractionPreferred,
        decimal_precision: 3,
        ..ParameterFormatOptions::default()
    };
    let _formatter = ParameterFormatter::new(opts);
    // ParameterFormatter is constructed successfully
}

#[test]
fn test_format_fixed_param_with_negative_pi() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions {
        mode: ParameterDisplayMode::PiFractionPreferred,
        ..ParameterFormatOptions::default()
    });
    let circuit = Circuit::new(0);

    assert_eq!(
        formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(-PI))
            .unwrap(),
        "-π"
    );

    assert_eq!(
        formatter
            .format_circuit_param(&circuit, &CircuitParam::Fixed(-PI / 4.0))
            .unwrap(),
        "-π/4"
    );
}

#[test]
fn test_format_index_param_out_of_bounds() {
    let formatter = ParameterFormatter::new(ParameterFormatOptions::default());
    let circuit = Circuit::new(0);

    let result = formatter.format_circuit_param(&circuit, &CircuitParam::Index(999));
    assert!(result.is_err());
}
