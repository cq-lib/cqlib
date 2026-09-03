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

//! Tests for result/statistics visualization.

use super::*;
use crate::circuit::Qubit;
use crate::device::{ExecutionResult, Outcome};
use crate::visualization::test_utils::assert_svg_visual_match;
use std::collections::HashMap;

fn assert_result_visual_match(svg: &str, filename: &str) {
    assert_svg_visual_match(&["result", "figure"], filename, |output_path| {
        render_result_plot_to_file(svg, &output_path.to_string_lossy())
    });
}

fn execution_result(items: &[(&str, usize)], num_qubits: usize) -> ExecutionResult {
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

fn empty_execution_result(num_qubits: usize) -> ExecutionResult {
    ExecutionResult::new(
        "queued-task".to_string(),
        (0..num_qubits)
            .map(|idx| Qubit::new(idx as u32))
            .collect::<Vec<_>>(),
        0,
        num_qubits,
        Some("simulator".to_string()),
        None,
    )
}

#[test]
fn distribution_matches_reference_image() {
    let result = execution_result(&[("0", 25), ("1", 75)], 1);
    let svg = plot_distribution(&result, &ResultPlotOptions::default()).unwrap();
    assert_result_visual_match(&svg, "distribution_probabilities.png");
}

#[test]
fn histogram_rejects_empty_execution_result() {
    let result = empty_execution_result(1);
    let err = plot_histogram(&result, &ResultPlotOptions::default()).unwrap_err();
    assert!(err.to_string().contains("no counts"));
}

#[test]
fn legend_length_must_match_single_execution_result_dataset() {
    let result = execution_result(&[("0", 1)], 1);
    let options = ResultPlotOptions {
        legend: Some(vec!["first".to_string(), "second".to_string()]),
        ..ResultPlotOptions::default()
    };
    let err = plot_histogram(&result, &options).unwrap_err();
    assert!(err.to_string().contains("legend length"));
}

#[test]
fn histogram_matches_reference_image() {
    let result = execution_result(&[("00", 2), ("11", 5)], 2);
    let svg = plot_histogram(&result, &ResultPlotOptions::default()).unwrap();
    assert_result_visual_match(&svg, "histogram_counts.png");
}

#[test]
fn histogram_hamming_sorted_matches_reference_image() {
    let result = execution_result(&[("100", 4), ("101", 3), ("010", 2), ("001", 1)], 3);
    let options = ResultPlotOptions {
        color: vec!["#123456".to_string()],
        number_to_keep: Some(3),
        sort: "hamming".to_string(),
        target_string: Some("100".to_string()),
        legend: Some(vec!["sim".to_string()]),
        bar_labels: false,
        title: Some("Hamming order".to_string()),
        ..ResultPlotOptions::default()
    };
    let svg = plot_histogram(&result, &options).unwrap();
    assert!(svg.contains("Hamming order"));
    assert!(svg.contains("#123456"));
    assert!(svg.contains(">sim</text>"));
    assert!(svg.contains(">rest</text>"));
    assert!(!svg.contains(">4</text>"));
    assert_result_visual_match(&svg, "histogram_hamming_sorted.png");
}

#[test]
fn distribution_custom_options_have_expected_svg_structure() {
    let result = execution_result(&[("0", 25), ("1", 75)], 1);
    let options = ResultPlotOptions {
        figsize: Some((3.2, 2.4)),
        color: vec!["#0f766e".to_string()],
        legend: Some(vec!["probability".to_string()]),
        bar_labels: false,
        title: Some("Distribution options".to_string()),
        ..ResultPlotOptions::default()
    };
    let svg = plot_distribution(&result, &options).unwrap();
    assert!(svg.contains("width=\"320\""));
    assert!(svg.contains("height=\"240\""));
    assert!(svg.contains("Distribution options"));
    assert!(svg.contains("#0f766e"));
    assert!(svg.contains(">probability</text>"));
    assert!(!svg.contains(">0.750</text>"));
    assert_result_visual_match(&svg, "distribution_custom_options.png");
}
