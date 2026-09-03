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
use crate::circuit::Qubit;

#[test]
fn test_outcome_new() {
    let chunks = smallvec::smallvec![0b1010u64];
    let out = Outcome::new(chunks);
    assert!(out.is_one(1));
    assert!(out.is_one(3));
    assert!(!out.is_one(0));
    assert!(!out.is_one(2));
}

#[test]
fn test_outcome_from_bitstring_basic() {
    // "10" -> Qubit 1 = 1, Qubit 0 = 0
    let out = Outcome::from_bitstring("10").unwrap();

    // Verify internal bit state (Little-Endian index)
    assert!(!out.is_one(0), "Bit 0 (LSB) should be 0");
    assert!(out.is_one(1), "Bit 1 (MSB) should be 1");
    assert!(!out.is_one(2), "Bit 2 (Out of bounds) should be 0");

    // Verify round-trip string (Big-Endian visual)
    assert_eq!(out.to_bitstring(2), "10");
}

#[test]
fn test_outcome_endianness_mapping() {
    // String "001" (3 qubits)
    // Visual: Q2=0, Q1=0, Q0=1
    let out = Outcome::from_bitstring("001").unwrap();

    // Storage verification:
    // Q0 (Index 0) should be 1
    assert!(out.is_one(0));
    // Q1, Q2 should be 0
    assert!(!out.is_one(1));
    assert!(!out.is_one(2));

    // Print verification
    assert_eq!(out.to_bitstring(3), "001");
}

#[test]
fn test_outcome_large_bitstring() {
    // Cross u64 boundary (66 bits)
    // Construct: "1" + 64 "0"s + "1"
    // Meaning: Qubit 65 = 1, Qubit 1..64 = 0, Qubit 0 = 1
    let mut s = String::from("1");
    for _ in 0..64 {
        s.push('0');
    }
    s.push('1');

    let out = Outcome::from_bitstring(&s).unwrap();

    // Verify boundaries
    assert!(out.is_one(0), "Qubit 0 should be 1");
    assert!(!out.is_one(1), "Qubit 1 should be 0");
    assert!(!out.is_one(64), "Qubit 64 should be 0");
    assert!(out.is_one(65), "Qubit 65 should be 1");

    // Verify chunk count
    assert_eq!(out.0.len(), 2, "Should require 2 u64 chunks");

    // Verify round-trip
    assert_eq!(out.to_bitstring(66), s);
}

#[test]
fn test_outcome_padding() {
    // Result is 1 (binary 1), but in 5-qubit system should be "00001"
    let out = Outcome::from_bitstring("1").unwrap();

    assert_eq!(out.to_bitstring(5), "00001");
    assert_eq!(out.to_bitstring(1), "1");
}

#[test]
fn test_outcome_invalid_input() {
    let res = Outcome::from_bitstring("10201");
    assert!(res.is_err());

    match res {
        Err(OutcomeError::InvalidCharacter(idx, c)) => {
            // String "10201", '2' is at index 2
            assert_eq!(idx, 2);
            assert_eq!(c, '2');
        }
        _ => panic!("Expected InvalidCharacter error"),
    }
}

#[test]
fn test_outcome_empty() {
    let out = Outcome::from_bitstring("").unwrap();
    assert_eq!(out.0.len(), 0);
    assert_eq!(out.to_bitstring(0), "");
}

#[test]
fn test_status_transitions() {
    let s = Status::Queued;
    assert!(!s.is_terminal());
    assert!(!s.is_success());

    let s = Status::Running;
    assert!(!s.is_terminal());

    let s = Status::Completed;
    assert!(s.is_terminal());
    assert!(s.is_success());

    let s = Status::Failed {
        error_msg: "Boom".into(),
        error_code: 500,
    };
    assert!(s.is_terminal());
    assert!(!s.is_success());
    // Test Display
    assert_eq!(format!("{}", s), "Failed (Code 500): Boom");

    let s = Status::Cancelled;
    assert!(s.is_terminal());
    assert!(!s.is_success());
    assert_eq!(format!("{}", s), "Cancelled");
}

/// Helper: create mock qubits
fn mock_qubits(n: usize) -> Vec<Qubit> {
    (0..n).map(|i| Qubit::new(i as u32)).collect()
}

#[test]
fn test_result_lifecycle_success() {
    let task_id = "task_001".to_string();
    let qubits = mock_qubits(2);
    let mut res = ExecutionResult::new(
        task_id.clone(),
        qubits.clone(),
        1000,
        2,
        Some("sim".to_string()),
        None,
    );

    // Verify initial state
    assert_eq!(res.task_id(), "task_001");
    assert_eq!(res.shots(), 1000);
    assert_eq!(res.num_qubits(), 2);
    assert_eq!(res.qubits().len(), 2);
    assert_eq!(res.status(), &Status::Queued);
    assert!(res.backend().is_some());
    assert_eq!(res.backend().unwrap(), "sim");
    assert!(res.started_at().is_none());
    assert!(res.finished_at().is_none());

    // Start
    res.start(None);
    assert_eq!(res.status(), &Status::Running);
    assert!(res.started_at().is_some());

    // Finish
    let mut counts = HashMap::new();
    counts.insert(Outcome::from_bitstring("00").unwrap(), 500);
    counts.insert(Outcome::from_bitstring("11").unwrap(), 500);

    res.finish(counts, None);
    assert_eq!(res.status(), &Status::Completed);
    assert!(res.finished_at().is_some());
    assert_eq!(res.counts().len(), 2);
}

#[test]
fn test_result_calc_probabilities() {
    let mut res = ExecutionResult::new("t1".into(), mock_qubits(1), 1000, 1, None, None);

    let mut counts = HashMap::new();
    let out0 = Outcome::from_bitstring("0").unwrap();
    let out1 = Outcome::from_bitstring("1").unwrap();

    // Scenario: total is 800 (e.g., filtering occurred), not shots 1000
    // Probability calculation must be based on 800, not 1000
    counts.insert(out0.clone(), 600);
    counts.insert(out1.clone(), 200);

    res.finish(counts, None);
    res.calc_probabilities();

    let probs = res.probabilities().as_ref().unwrap();

    // Verify normalization: 600/800 = 0.75, 200/800 = 0.25
    assert_eq!(*probs.get(&out0).unwrap(), 0.75);
    assert_eq!(*probs.get(&out1).unwrap(), 0.25);
}

#[test]
fn finish_clears_probabilities_computed_from_previous_counts() {
    let mut res = ExecutionResult::new("t1".into(), mock_qubits(1), 10, 1, None, None);
    let zero = Outcome::from_bitstring("0").unwrap();
    let one = Outcome::from_bitstring("1").unwrap();

    res.finish(HashMap::from([(zero.clone(), 10)]), None);
    res.calc_probabilities();
    assert!(res.probabilities().is_some());

    res.finish(HashMap::from([(one, 10)]), None);
    assert!(res.probabilities().is_none());
}

#[test]
fn test_result_fail_and_cancel() {
    let mut res = ExecutionResult::new("t2".into(), vec![], 100, 0, None, None);

    res.start(None);
    res.fail("Timeout".into(), -1);

    match res.status() {
        Status::Failed {
            error_msg,
            error_code,
        } => {
            assert_eq!(error_msg, "Timeout");
            assert_eq!(*error_code, -1);
        }
        _ => panic!("Should be Failed"),
    }

    let mut res2 = ExecutionResult::new("t3".into(), vec![], 100, 0, None, None);
    res2.cancel();
    assert_eq!(res2.status(), &Status::Cancelled);
}

#[test]
fn test_qubit_outcome_mapping_golden_rule() {
    // Outcome string "10"
    // String "10" -> Index 1 ('1'), Index 0 ('0')
    // Meaning qubits[1] is 1, qubits[0] is 0
    let outcome = Outcome::from_bitstring("10").unwrap();

    // Verify: Bit 1 corresponds to qubits[1]
    assert!(outcome.is_one(1));
    // Verify: Bit 0 corresponds to qubits[0]
    assert!(!outcome.is_one(0));
}

#[test]
fn test_zero_shots_robustness() {
    // Scenario: no measurement results
    let mut res = ExecutionResult::new("t4".into(), vec![], 1000, 0, None, None);
    res.finish(HashMap::new(), None);
    res.calc_probabilities();

    assert!(res.probabilities().is_none());
}

#[test]
fn test_result_created_at() {
    let now = OffsetDateTime::now_utc();
    let res = ExecutionResult::new("t5".into(), vec![], 100, 0, None, Some(now));

    assert_eq!(res.created_at().unix_timestamp(), now.unix_timestamp());
}

#[test]
fn test_result_custom_timestamps() {
    let created = OffsetDateTime::from_unix_timestamp(1000).unwrap();
    let started = OffsetDateTime::from_unix_timestamp(2000).unwrap();
    let finished = OffsetDateTime::from_unix_timestamp(3000).unwrap();

    let mut res = ExecutionResult::new("t6".into(), vec![], 100, 0, None, Some(created));
    res.start(Some(started));
    res.finish(HashMap::new(), Some(finished));

    assert_eq!(res.created_at().unix_timestamp(), 1000);
    assert_eq!(res.started_at().unwrap().unix_timestamp(), 2000);
    assert_eq!(res.finished_at().unwrap().unix_timestamp(), 3000);
}

#[test]
fn test_outcome_all_zeros() {
    let out = Outcome::from_bitstring("00000").unwrap();
    for i in 0..5 {
        assert!(!out.is_one(i), "Bit {} should be 0", i);
    }
    assert_eq!(out.to_bitstring(5), "00000");
}

#[test]
fn test_outcome_all_ones() {
    let out = Outcome::from_bitstring("1111").unwrap();
    for i in 0..4 {
        assert!(out.is_one(i), "Bit {} should be 1", i);
    }
    assert_eq!(out.to_bitstring(4), "1111");
}

#[test]
fn test_outcome_to_bitstring_truncate() {
    // Value is "101" (5), but only print 2 bits -> "01"
    let out = Outcome::from_bitstring("101").unwrap();
    assert_eq!(out.to_bitstring(2), "01");
}

#[test]
fn outcome_implements_from_str() {
    let outcome: Outcome = "101001".parse().unwrap();
    assert_eq!(outcome.to_bitstring(6), "101001");
    assert_eq!("10x".parse::<Outcome>(), Outcome::from_bitstring("10x"));
}

#[test]
fn outcome_does_not_retain_input_width() {
    let padded: Outcome = "001".parse().unwrap();
    let compact: Outcome = "1".parse().unwrap();

    assert_eq!(padded, compact);
    assert_eq!(padded.to_bitstring(3), "001");
    assert_eq!(padded.to_bitstring(1), "1");
    assert_eq!(
        Outcome::from_bitstring("101").unwrap().to_bitstring(2),
        "01"
    );
}

#[test]
fn status_compares_by_value_and_hashes_consistently() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    assert_eq!(Status::Completed, Status::Completed);

    let failed = Status::Failed {
        error_msg: "boom".to_string(),
        error_code: 1,
    };
    assert_eq!(
        failed,
        Status::Failed {
            error_msg: "boom".to_string(),
            error_code: 1,
        }
    );
    assert_ne!(
        failed,
        Status::Failed {
            error_msg: "other".to_string(),
            error_code: 1,
        }
    );
    assert_ne!(Status::Completed, Status::Cancelled);

    let hash_of = |status: &Status| {
        let mut hasher = DefaultHasher::new();
        status.hash(&mut hasher);
        hasher.finish()
    };
    assert_eq!(hash_of(&Status::Completed), hash_of(&Status::Completed));
    assert_eq!(
        hash_of(&failed),
        hash_of(&Status::Failed {
            error_msg: "boom".to_string(),
            error_code: 1,
        })
    );
}
