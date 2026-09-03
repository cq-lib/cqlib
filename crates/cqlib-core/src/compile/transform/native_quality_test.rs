// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::native_quality::{
    CriticalPathBuilder, NativeCriticalPathQuality, NativeQualityPolicy, NativeQualityVector,
    NativeQualityViolation,
};
use crate::circuit::{Instruction, ParameterValue, Qubit, StandardGate, ValueOperation};
use crate::compile::device_planning::cost::{
    DevicePhysicalCost, MetricAvailability, NativePlanLeaf, RobustDurationKey, RobustErrorKey,
};
use crate::device::PhysicalQubit;
use smallvec::smallvec;

fn physical_cost(
    two_qubit_ops: u32,
    two_qubit_depth: u32,
    total_depth: u32,
    total_ops: u32,
) -> DevicePhysicalCost {
    DevicePhysicalCost {
        native_two_qubit_ops: two_qubit_ops,
        native_two_qubit_depth: two_qubit_depth,
        error: MetricAvailability::Disabled,
        total_native_depth: total_depth,
        native_total_ops: total_ops,
        duration: MetricAvailability::Disabled,
        makespan: MetricAvailability::Disabled,
    }
}

fn quality(
    two_qubit_ops: u32,
    two_qubit_depth: u32,
    total_depth: u32,
    total_ops: u32,
    critical_path_1q: u32,
    longest_run: u32,
) -> NativeQualityVector {
    NativeQualityVector {
        physical: physical_cost(two_qubit_ops, two_qubit_depth, total_depth, total_ops),
        critical_path: NativeCriticalPathQuality {
            one_qubit_ops: critical_path_1q,
            longest_one_qubit_run: longest_run,
        },
    }
}

#[test]
fn critical_metrics_consider_all_equal_longest_paths() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut path = CriticalPathBuilder::default();
    path.add_gate(&[q0]);
    path.add_gate(&[q0]);
    path.add_gate(&[q1]);
    path.add_gate(&[q1]);
    path.add_gate(&[q0, q1]);

    assert_eq!(
        path.finish(),
        NativeCriticalPathQuality {
            one_qubit_ops: 2,
            longest_one_qubit_run: 2,
        }
    );
}

#[test]
fn balanced_depth_rejects_entangler_gain_that_increases_depth() {
    let entry = quality(10, 8, 20, 40, 8, 3);
    let candidate = quality(9, 7, 120, 140, 108, 100);

    assert!(
        candidate
            .compare(entry, NativeQualityPolicy::EntanglerFirst)
            .is_lt()
    );
    assert!(!candidate.admissible_against(entry, NativeQualityPolicy::BalancedDepth));
}

#[test]
fn balanced_depth_uses_real_lexicographic_order() {
    let entry = quality(10, 8, 20, 40, 8, 3);
    let worse_critical_path = quality(10, 8, 20, 39, 9, 1);

    assert!(worse_critical_path.admissible_against(entry, NativeQualityPolicy::BalancedDepth));
    assert!(
        worse_critical_path
            .compare(entry, NativeQualityPolicy::BalancedDepth)
            .is_gt()
    );
}

#[test]
fn balanced_depth_accepts_lower_total_ops_after_protected_ties() {
    let entry = quality(10, 8, 20, 40, 8, 3);
    let candidate = quality(10, 8, 20, 39, 8, 3);

    assert!(candidate.admissible_against(entry, NativeQualityPolicy::BalancedDepth));
    assert!(
        candidate
            .compare(entry, NativeQualityPolicy::BalancedDepth)
            .is_lt()
    );
}

#[test]
fn balanced_depth_protects_error_confidence_and_value() {
    let mut entry = quality(10, 8, 20, 40, 8, 3);
    entry.physical.error = MetricAvailability::Available(RobustErrorKey {
        unavailable_count: 0,
        imputed_count: 1,
        log_error: 0.2,
    });
    let mut worse_value = entry;
    worse_value.physical.error = MetricAvailability::Available(RobustErrorKey {
        unavailable_count: 0,
        imputed_count: 1,
        log_error: 0.21,
    });
    let mut worse_confidence = entry;
    worse_confidence.physical.error = MetricAvailability::Available(RobustErrorKey {
        unavailable_count: 1,
        imputed_count: 0,
        log_error: 0.1,
    });

    assert!(!worse_value.admissible_against(entry, NativeQualityPolicy::BalancedDepth));
    assert!(!worse_confidence.admissible_against(entry, NativeQualityPolicy::BalancedDepth));
}

#[test]
fn balanced_depth_allows_only_roundoff_sized_error_drift() {
    let mut entry = quality(10, 8, 20, 40, 8, 3);
    entry.physical.error = MetricAvailability::Available(RobustErrorKey {
        unavailable_count: 0,
        imputed_count: 0,
        log_error: 0.2,
    });
    let mut roundoff = entry;
    roundoff.physical.error = MetricAvailability::Available(RobustErrorKey {
        log_error: 0.2 + 5e-13,
        ..RobustErrorKey::default()
    });
    let mut regression = entry;
    regression.physical.error = MetricAvailability::Available(RobustErrorKey {
        log_error: 0.2 + 2e-12,
        ..RobustErrorKey::default()
    });

    assert!(roundoff.admissible_against(entry, NativeQualityPolicy::BalancedDepth));
    assert_eq!(
        regression.admissibility_violation_against(entry, NativeQualityPolicy::BalancedDepth),
        Some(NativeQualityViolation::Error)
    );
}

#[test]
fn balanced_depth_reports_each_hard_guard() {
    let mut calibrated_entry = quality(10, 8, 20, 40, 8, 3);
    calibrated_entry.physical.error = MetricAvailability::Available(RobustErrorKey::default());
    calibrated_entry.physical.makespan = MetricAvailability::Available(100.0);

    let cases = [
        (
            quality(11, 8, 20, 40, 8, 3),
            NativeQualityViolation::TwoQubitOps,
        ),
        (
            quality(10, 9, 20, 40, 8, 3),
            NativeQualityViolation::TwoQubitDepth,
        ),
        (
            quality(10, 8, 21, 40, 8, 3),
            NativeQualityViolation::TotalDepth,
        ),
    ];
    let uncalibrated_entry = quality(10, 8, 20, 40, 8, 3);
    for (candidate, expected) in cases {
        assert_eq!(
            candidate.admissibility_violation_against(
                uncalibrated_entry,
                NativeQualityPolicy::BalancedDepth,
            ),
            Some(expected)
        );
    }

    let mut worse_error = calibrated_entry;
    worse_error.physical.error = MetricAvailability::Available(RobustErrorKey {
        log_error: 0.01,
        ..RobustErrorKey::default()
    });
    let mut worse_makespan = calibrated_entry;
    worse_makespan.physical.makespan = MetricAvailability::Available(101.0);
    assert_eq!(
        worse_error
            .admissibility_violation_against(calibrated_entry, NativeQualityPolicy::BalancedDepth,),
        Some(NativeQualityViolation::Error)
    );
    assert_eq!(
        worse_makespan
            .admissibility_violation_against(calibrated_entry, NativeQualityPolicy::BalancedDepth,),
        Some(NativeQualityViolation::Makespan)
    );
}

#[test]
fn balanced_depth_requires_stable_metric_availability() {
    let entry = quality(10, 8, 20, 40, 8, 3);
    let mut candidate = entry;
    candidate.physical.error = MetricAvailability::Available(RobustErrorKey::default());
    candidate.physical.duration = MetricAvailability::Available(RobustDurationKey::default());

    assert!(!candidate.admissible_against(entry, NativeQualityPolicy::BalancedDepth));
}

#[test]
fn balanced_depth_protects_calibrated_makespan() {
    let mut entry = quality(10, 8, 20, 40, 8, 3);
    entry.physical.makespan = MetricAvailability::Available(100.0);
    let mut candidate = quality(9, 7, 19, 39, 7, 2);
    candidate.physical.makespan = MetricAvailability::Available(101.0);

    assert!(!candidate.admissible_against(entry, NativeQualityPolicy::BalancedDepth));
}

#[test]
fn critical_run_resets_at_two_qubit_gate() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut path = CriticalPathBuilder::default();
    path.add_gate(&[q0]);
    path.add_gate(&[q0]);
    path.add_gate(&[q0, q1]);
    path.add_gate(&[q1]);
    path.add_gate(&[q1]);
    path.add_gate(&[q1]);

    assert_eq!(
        path.finish(),
        NativeCriticalPathQuality {
            one_qubit_ops: 5,
            longest_one_qubit_run: 3,
        }
    );
}

#[test]
fn planned_native_leaves_expose_hidden_one_qubit_expansion() {
    let q0 = Qubit::new(0);
    let high_level = vec![ValueOperation::from_standard(
        StandardGate::U,
        [q0],
        [
            ParameterValue::Fixed(0.1),
            ParameterValue::Fixed(0.2),
            ParameterValue::Fixed(0.3),
        ],
    )];
    let leaves = [StandardGate::RZ, StandardGate::X2P, StandardGate::RZ]
        .into_iter()
        .map(|gate| NativePlanLeaf {
            instruction: Instruction::Standard(gate),
            ordered_qargs: smallvec![PhysicalQubit::new(0)],
            error_rate: None,
            duration: None,
        })
        .collect::<Vec<_>>();
    let physical = physical_cost(0, 0, 3, 3);

    let logical_shape = NativeQualityVector::for_value_operations(physical, &high_level);
    let native_shape = NativeQualityVector::for_native_plan_leaves(physical, &leaves);

    assert_eq!(logical_shape.critical_path.one_qubit_ops, 1);
    assert_eq!(native_shape.critical_path.one_qubit_ops, 3);
    assert_eq!(native_shape.critical_path.longest_one_qubit_run, 3);
}
