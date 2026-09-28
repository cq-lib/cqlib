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

use super::{Parameter, ParameterValue};
use std::collections::HashMap;

#[test]
fn scaled_fixed_values_preserve_floating_point_results() {
    for (value, factor, expected) in [
        (8.0, -0.25, -2.0_f64),
        (8.0, 1.0, 8.0),
        (8.0, 0.0, 0.0),
        (0.0, -1.0, -0.0),
        (f64::MAX, 2.0, f64::INFINITY),
    ] {
        let ParameterValue::Fixed(actual) = ParameterValue::Fixed(value).scaled(factor) else {
            panic!("fixed input must remain fixed");
        };
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
}

#[test]
fn scaled_symbolic_values_preserve_variant_and_binding_semantics() {
    let input = ParameterValue::Param(Parameter::symbol("theta"));
    for factor in [-0.25, 0.0, 1.0, 2.0] {
        let ParameterValue::Param(actual) = input.scaled(factor) else {
            panic!("symbolic input must remain Param, even when scaled by zero");
        };
        for theta in [-4.0, 0.0, 8.0] {
            let bindings = Some(HashMap::from([("theta", theta)]));
            assert_eq!(actual.evaluate(&bindings).unwrap(), theta * factor);
        }
    }
    assert_eq!(input, ParameterValue::Param(Parameter::symbol("theta")));
}

#[test]
fn scaled_constant_expressions_remain_param() {
    for parameter in [Parameter::from(2.0), Parameter::pi()] {
        let value = parameter.evaluate(&None).unwrap();
        for factor in [-0.5, 0.0, 1.0] {
            let ParameterValue::Param(actual) =
                ParameterValue::Param(parameter.clone()).scaled(factor)
            else {
                panic!("constant expressions must not be converted to Fixed");
            };
            assert_eq!(actual.evaluate(&None).unwrap(), value * factor);
        }
    }
}

#[test]
fn scaled_rejects_non_finite_factors_for_both_variants() {
    for input in [
        ParameterValue::Fixed(2.0),
        ParameterValue::Param(Parameter::symbol("theta")),
    ] {
        for factor in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(std::panic::catch_unwind(|| input.scaled(factor)).is_err());
        }
    }
}
