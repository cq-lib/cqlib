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

//! Gate style maps for figure visualization.
//!
//! This module loads built-in JSON style dictionaries (`default`, `gray`) and merges
//! optional runtime overrides for figure rendering.

use serde::Deserialize;
use std::collections::HashMap;

/// Per-gate visual style settings (compatible with Python style JSON schema).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct GateStyle {
    /// Optional gate border color in CSS color syntax.
    #[serde(default)]
    pub border_color: Option<String>,
    /// Optional gate fill color in CSS color syntax.
    #[serde(default)]
    pub background_color: Option<String>,
    /// Optional gate label font size.
    #[serde(default)]
    pub font_size: Option<f64>,
    /// Optional gate label text color in CSS color syntax.
    #[serde(default)]
    pub text_color: Option<String>,
    /// Optional connector or wire color in CSS color syntax.
    #[serde(default)]
    pub line_color: Option<String>,
    /// Optional connector or wire stroke width.
    #[serde(default)]
    pub line_width: Option<f64>,
}

/// Style dictionary keyed by gate name (with mandatory `default` fallback).
#[derive(Debug, Clone)]
pub struct StyleBook {
    styles: HashMap<String, GateStyle>,
    default_style: GateStyle,
}

impl StyleBook {
    /// Load a built-in style map by name and apply optional runtime overrides.
    ///
    /// Supported built-in names are `default` and `gray`. Unknown names fall back to
    /// `default`.
    pub fn new(style_name: &str, overrides: &HashMap<String, GateStyle>) -> Self {
        let mut styles = load_style(style_name);
        for (k, v) in overrides {
            styles.insert(k.clone(), v.clone());
        }
        if !styles.contains_key("default") {
            styles.insert("default".to_string(), GateStyle::default());
        }
        let default_style = styles.get("default").cloned().unwrap_or_default();
        Self {
            styles,
            default_style,
        }
    }

    /// Return style for gate name, falling back to `default`.
    pub fn get(&self, gate_name: &str) -> &GateStyle {
        self.styles.get(gate_name).unwrap_or(&self.default_style)
    }
}

/// Load built-in style JSON by name.
///
/// Supported names:
/// - `default`
/// - `gray`
///
/// Unknown names fall back to `default`.
fn load_style(style_name: &str) -> HashMap<String, GateStyle> {
    let style_key = style_name.trim().to_ascii_lowercase();
    let json = match style_key.as_str() {
        "gray" => include_str!("styles/gray.json"),
        _ => include_str!("styles/default.json"),
    };
    serde_json::from_str(json).unwrap_or_else(|_| {
        let mut fallback = HashMap::new();
        fallback.insert("default".to_string(), GateStyle::default());
        fallback
    })
}
