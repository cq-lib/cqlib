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

use super::{OperationReplacement, RewriteEdits};
use crate::circuit::{Circuit, Qubit};

#[test]
fn route_provenance_marks_only_insertions_when_source_order_is_stable() {
    let edits = RewriteEdits::from_route_provenance(3, &[Some(0), None, Some(1), Some(2)]);
    assert_eq!(
        edits,
        RewriteEdits::linear(
            3,
            4,
            vec![OperationReplacement {
                old: 1..1,
                new: 1..2,
            }],
        )
    );
}

#[test]
fn route_provenance_turns_scheduler_moves_into_ordered_replacements() {
    let edits = RewriteEdits::from_route_provenance(3, &[Some(1), Some(0), Some(2)]);
    assert_eq!(
        edits,
        RewriteEdits::linear(
            3,
            3,
            vec![
                OperationReplacement {
                    old: 0..0,
                    new: 0..1,
                },
                OperationReplacement {
                    old: 1..2,
                    new: 2..2,
                },
            ],
        )
    );
}

#[test]
fn duplicate_route_provenance_is_unknown() {
    assert_eq!(
        RewriteEdits::from_route_provenance(2, &[Some(0), Some(0)]),
        RewriteEdits::Unknown
    );
}

#[test]
fn linear_derivation_preserves_sparse_edits() {
    let q0 = Qubit::new(0);
    let mut before = Circuit::new(1);
    let mut after = Circuit::new(1);
    for order in 0..20 {
        if order == 4 || order == 15 {
            before.x(q0).unwrap();
            after.y(q0).unwrap();
        } else {
            before.h(q0).unwrap();
            after.h(q0).unwrap();
        }
    }

    assert_eq!(
        RewriteEdits::between_linear_circuits(&before, &after),
        RewriteEdits::linear(
            20,
            20,
            vec![
                OperationReplacement {
                    old: 4..5,
                    new: 4..5,
                },
                OperationReplacement {
                    old: 15..16,
                    new: 15..16,
                },
            ],
        )
    );
}

#[test]
fn linear_derivation_handles_insertions_and_deletions() {
    let q0 = Qubit::new(0);
    let mut before = Circuit::new(1);
    before.h(q0).unwrap();
    before.x(q0).unwrap();
    before.h(q0).unwrap();
    let mut after = Circuit::new(1);
    after.h(q0).unwrap();
    after.z(q0).unwrap();
    after.h(q0).unwrap();

    assert_eq!(
        RewriteEdits::between_linear_circuits(&before, &after),
        RewriteEdits::linear(
            3,
            3,
            vec![OperationReplacement {
                old: 1..2,
                new: 1..2,
            }],
        )
    );
}
