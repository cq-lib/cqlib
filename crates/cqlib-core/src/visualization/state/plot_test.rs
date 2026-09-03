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

//! Tests for quantum-state visualization.

use super::*;
use crate::qis::{DensityMatrix, Statevector};
use crate::visualization::test_utils::assert_svg_visual_match;
use num_complex::Complex64;

fn zero_state() -> Statevector {
    Statevector::from_state(1, vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)]).unwrap()
}

fn plus_state() -> Statevector {
    Statevector::from_state(
        1,
        vec![
            Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
            Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
        ],
    )
    .unwrap()
}

fn bell_state() -> Statevector {
    Statevector::from_state(
        2,
        vec![
            Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
        ],
    )
    .unwrap()
}

fn assert_state_visual_match(svg: &str, filename: &str) {
    assert_svg_visual_match(&["state", "figure"], filename, |output_path| {
        render_state_plot_to_file(svg, &output_path.to_string_lossy())
    });
}

fn attr_f64(tag: &str, name: &str) -> f64 {
    let needle = format!("{name}=\"");
    let start = tag.find(&needle).unwrap() + needle.len();
    let end = tag[start..].find('"').unwrap() + start;
    tag[start..end].parse().unwrap()
}

fn rect_tags_between(svg: &str, start_marker: &str, end_marker: &str) -> Vec<String> {
    let start = svg.find(start_marker).unwrap();
    let end = svg[start..].find(end_marker).unwrap() + start;
    let segment = &svg[start..end];
    let mut tags = Vec::new();
    let mut offset = 0;
    while let Some(relative_start) = segment[offset..].find("<rect ") {
        let tag_start = offset + relative_start;
        let tag_end = tag_start + segment[tag_start..].find("/>").unwrap() + 2;
        tags.push(segment[tag_start..tag_end].to_string());
        offset = tag_end;
    }
    tags
}

fn assert_in_display_label_order(svg: &str, expected: &[&str]) {
    let mut last = 0;
    for label in expected {
        let needle = format!(">{label}</text>");
        let found = svg[last..].find(&needle).unwrap() + last;
        assert!(found >= last);
        last = found;
    }
}

#[test]
fn state_city_matches_reference_image() {
    let svg = plot_state_city(&zero_state(), &StatePlotOptions::default()).unwrap();
    assert_state_visual_match(&svg, "state_city_zero.png");
}

#[test]
fn state_city_reverse_bits_remaps_matrix_values_and_labels() {
    let density = DensityMatrix::from_density_matrix_state(
        2,
        vec![
            Complex64::new(0.05, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.15, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.75, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.05, 0.0),
        ],
    )
    .unwrap();

    let svg = plot_state_city(
        &density,
        &StatePlotOptions {
            reverse_bits: true,
            ..StatePlotOptions::default()
        },
    )
    .unwrap();

    assert_in_display_label_order(&svg, &["00", "10", "01", "11"]);

    let real_rects = rect_tags_between(&svg, "Re[rho]", "Im[rho]");
    let cell_widths: Vec<f64> = real_rects
        .iter()
        .skip(1)
        .map(|tag| attr_f64(tag, "width"))
        .collect();
    assert_eq!(cell_widths.len(), 16);

    let diag_00 = cell_widths[0];
    let diag_10 = cell_widths[5];
    let diag_01 = cell_widths[10];
    let diag_11 = cell_widths[15];
    let off_diagonal = cell_widths[1];
    assert!(diag_10 > diag_01);
    assert!(diag_01 > diag_00);
    assert!((diag_00 - diag_11).abs() < 1e-9);
    assert!(off_diagonal < diag_00);

    assert_state_visual_match(&svg, "state_city_reverse_bits.png");
}

#[test]
fn bloch_vector_matches_reference_image() {
    let svg = plot_bloch_vector([0.0, 0.0, 1.0], &StatePlotOptions::default()).unwrap();
    assert_state_visual_match(&svg, "bloch_vector_z.png");
}

#[test]
fn bloch_multivector_matches_reference_image() {
    let svg = plot_bloch_multivector(&plus_state(), &StatePlotOptions::default()).unwrap();
    assert_state_visual_match(&svg, "bloch_multivector_plus.png");
}

#[test]
fn bloch_multivector_bell_state_matches_reference_image() {
    let svg = plot_bloch_multivector(&bell_state(), &StatePlotOptions::default()).unwrap();
    assert!(svg.contains("q0"));
    assert!(svg.contains("q1"));
    assert_state_visual_match(&svg, "bloch_multivector_bell.png");
}

#[test]
fn paulivec_matches_reference_image() {
    let svg = plot_state_paulivec(&plus_state(), &StatePlotOptions::default()).unwrap();
    assert_state_visual_match(&svg, "paulivec_plus.png");
}

#[test]
fn density_matrix_uses_same_state_plot_api() {
    let statevector = plus_state();
    let density_matrix =
        DensityMatrix::from_state(statevector.num_qubits, statevector.data().to_vec()).unwrap();

    let sv_svg = plot_state_paulivec(&statevector, &StatePlotOptions::default()).unwrap();
    let dm_svg = plot_state_paulivec(&density_matrix, &StatePlotOptions::default()).unwrap();
    assert_eq!(sv_svg, dm_svg);

    let sv_city = plot_state_city(&statevector, &StatePlotOptions::default()).unwrap();
    let dm_city = plot_state_city(&density_matrix, &StatePlotOptions::default()).unwrap();
    assert_eq!(sv_city, dm_city);
}
