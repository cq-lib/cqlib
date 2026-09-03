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

//! Tests for result visualization data preparation.

use super::data::{
    execution_result_to_plot_series, keep_topk_with_rest, normalize_distribution, sorted_labels,
};
use super::options::PlotSeries;
use super::*;
use crate::circuit::Qubit;
use crate::device::{ExecutionResult, Outcome};
use std::collections::HashMap;

fn series(items: &[(&str, f64)]) -> PlotSeries {
    items
        .iter()
        .map(|(label, value)| ((*label).to_string(), *value))
        .collect()
}

#[test]
fn hamming_sort_orders_by_distance() {
    let opts = ResultPlotOptions {
        sort: "hamming".to_string(),
        target_string: Some("100".to_string()),
        ..ResultPlotOptions::default()
    };
    let labels = sorted_labels(
        &[series(&[("101", 1.0), ("010", 1.0), ("100", 1.0)])],
        &opts,
    )
    .unwrap();
    assert_eq!(labels, vec!["100", "101", "010"]);
}

#[test]
fn value_desc_sort_keeps_label_order_for_ties() {
    let opts = ResultPlotOptions {
        sort: "value_desc".to_string(),
        ..ResultPlotOptions::default()
    };
    let labels = sorted_labels(&[series(&[("10", 2.0), ("01", 2.0), ("11", 3.0)])], &opts).unwrap();
    assert_eq!(labels, vec!["11", "01", "10"]);
}

#[test]
fn topk_aggregates_rest() {
    let kept = keep_topk_with_rest(&series(&[("00", 1.0), ("01", 5.0), ("10", 2.0)]), 2);
    assert_eq!(kept.get("rest").copied(), Some(1.0));
    assert!(kept.contains_key("01"));
    assert!(kept.contains_key("10"));
}

#[test]
fn topk_zero_and_large_k_keep_original_series() {
    let input = series(&[("00", 1.0), ("01", 5.0), ("10", 2.0)]);
    assert_eq!(keep_topk_with_rest(&input, 0), input);
    assert_eq!(keep_topk_with_rest(&input, 3), input);
    assert_eq!(keep_topk_with_rest(&input, 4), input);
}

#[test]
fn hamming_sort_requires_target_string() {
    let opts = ResultPlotOptions {
        sort: "hamming".to_string(),
        ..ResultPlotOptions::default()
    };
    let err = sorted_labels(&[series(&[("00", 1.0)])], &opts).unwrap_err();
    assert!(err.to_string().contains("requires target_string"));
}

#[test]
fn hamming_sort_rejects_labels_with_different_length() {
    let opts = ResultPlotOptions {
        sort: "hamming".to_string(),
        target_string: Some("0".to_string()),
        ..ResultPlotOptions::default()
    };
    let err = sorted_labels(&[series(&[("00", 1.0)])], &opts).unwrap_err();
    assert!(err.to_string().contains("equal-length"));
}

#[test]
fn distribution_rejects_zero_sum_series() {
    let err = normalize_distribution(series(&[("0", 0.0), ("1", 0.0)])).unwrap_err();
    assert!(err.to_string().contains("sum is zero"));
}

fn execution_result_with_counts(items: &[(&str, usize)], num_qubits: usize) -> ExecutionResult {
    let mut result = ExecutionResult::new(
        "task-visualization".to_string(),
        (0..num_qubits)
            .map(|idx| Qubit::new(idx as u32))
            .collect::<Vec<_>>(),
        items.iter().map(|(_, count)| *count).sum(),
        num_qubits,
        Some("simulator".to_string()),
        None,
    );
    let counts = items
        .iter()
        .map(|(bits, count)| (Outcome::from_bitstring(bits).unwrap(), *count))
        .collect::<HashMap<_, _>>();
    result.finish(counts, None);
    result
}

#[test]
fn execution_result_converts_to_plot_series() {
    let result = execution_result_with_counts(&[("1", 2), ("10", 3)], 2);
    let series = execution_result_to_plot_series(&result).unwrap();
    assert_eq!(series.get("01").copied(), Some(2.0));
    assert_eq!(series.get("10").copied(), Some(3.0));
}

#[test]
fn empty_execution_result_rejects_plot_series_conversion() {
    let result = ExecutionResult::new(
        "queued-task".to_string(),
        vec![Qubit::new(0)],
        100,
        1,
        None,
        None,
    );
    let err = execution_result_to_plot_series(&result).unwrap_err();
    assert!(err.to_string().contains("no counts"));
}
