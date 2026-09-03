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

//! Tests for result visualization options.

use super::*;

#[test]
fn default_result_options_match_public_plot_defaults() {
    let options = ResultPlotOptions::default();
    assert_eq!(options.sort, "asc");
    assert!(options.bar_labels);
    assert!(options.color.is_empty());
}
