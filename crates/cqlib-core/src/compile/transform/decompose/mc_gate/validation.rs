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

//! Multi-controlled gate qubit validation helpers.

use crate::circuit::Qubit;
use std::collections::HashSet;

/// Returns the first qubit that occurs more than once across the provided
/// groups.
///
/// Groups and qubits within each group are inspected in slice order.
pub(super) fn find_duplicate_qubit(qubit_groups: &[&[Qubit]]) -> Option<Qubit> {
    let mut seen = HashSet::with_capacity(qubit_groups.iter().map(|qubits| qubits.len()).sum());
    qubit_groups
        .iter()
        .flat_map(|qubits| qubits.iter().copied())
        .find(|&qubit| !seen.insert(qubit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_first_duplicate_qubit_across_groups() {
        let first = [Qubit::new(0), Qubit::new(1)];
        let second = [Qubit::new(2), Qubit::new(1)];

        assert_eq!(
            find_duplicate_qubit(&[&first, &second]),
            Some(Qubit::new(1))
        );
    }

    #[test]
    fn returns_none_for_distinct_qubit_groups() {
        let first = [Qubit::new(0), Qubit::new(1)];
        let second = [Qubit::new(2), Qubit::new(3)];

        assert_eq!(find_duplicate_qubit(&[&first, &second]), None);
    }

    #[test]
    fn finds_duplicate_qubit_within_one_group() {
        let qubits = [Qubit::new(0), Qubit::new(1), Qubit::new(0)];

        assert_eq!(find_duplicate_qubit(&[&qubits]), Some(Qubit::new(0)));
    }

    #[test]
    fn returns_first_duplicate_in_traversal_order() {
        let first = [Qubit::new(0), Qubit::new(1)];
        let second = [Qubit::new(2), Qubit::new(1), Qubit::new(0)];

        assert_eq!(
            find_duplicate_qubit(&[&first, &second]),
            Some(Qubit::new(1))
        );
    }

    #[test]
    fn accepts_empty_input_and_empty_groups() {
        assert_eq!(find_duplicate_qubit(&[]), None);
        assert_eq!(find_duplicate_qubit(&[&[]]), None);
    }
}
