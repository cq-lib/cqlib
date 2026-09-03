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

//! Tests for the visualization style module.

use super::style::*;
use std::collections::HashMap;

#[test]
fn test_gate_style_default_values() {
    let style = GateStyle::default();
    assert!(style.border_color.is_none());
    assert!(style.background_color.is_none());
    assert!(style.font_size.is_none());
    assert!(style.text_color.is_none());
    assert!(style.line_color.is_none());
    assert!(style.line_width.is_none());
}

#[test]
fn test_gate_style_clone() {
    let style = GateStyle {
        border_color: Some("black".to_string()),
        background_color: Some("white".to_string()),
        font_size: Some(12.0),
        text_color: Some("blue".to_string()),
        line_color: Some("gray".to_string()),
        line_width: Some(2.0),
    };
    let cloned = style.clone();
    assert_eq!(style.border_color, cloned.border_color);
    assert_eq!(style.background_color, cloned.background_color);
    assert_eq!(style.font_size, cloned.font_size);
    assert_eq!(style.text_color, cloned.text_color);
    assert_eq!(style.line_color, cloned.line_color);
    assert_eq!(style.line_width, cloned.line_width);
}

#[test]
fn test_gate_style_partial_values() {
    let style = GateStyle {
        border_color: Some("red".to_string()),
        ..GateStyle::default()
    };
    assert_eq!(style.border_color, Some("red".to_string()));
    assert!(style.background_color.is_none());
}

#[test]
fn test_style_book_new_creates_default_fallback() {
    let overrides = HashMap::new();
    let book = StyleBook::new("unknown_style", &overrides);

    let default_style = book.get("default");
    assert!(default_style.text_color.is_some());
}

#[test]
fn test_style_book_get_unknown_gate_returns_default() {
    let overrides = HashMap::new();
    let book = StyleBook::new("default", &overrides);

    let style = book.get("UNKNOWN_GATE_NAME");
    let default_style = book.get("default");

    assert_eq!(style.background_color, default_style.background_color);
    assert_eq!(style.text_color, default_style.text_color);
}

#[test]
fn test_style_book_overrides_are_applied() {
    let mut overrides = HashMap::new();
    let custom_style = GateStyle {
        background_color: Some("custom_bg".to_string()),
        text_color: Some("custom_text".to_string()),
        ..GateStyle::default()
    };
    overrides.insert("H".to_string(), custom_style.clone());

    let book = StyleBook::new("default", &overrides);
    let h_style = book.get("H");

    assert_eq!(h_style.background_color, Some("custom_bg".to_string()));
    assert_eq!(h_style.text_color, Some("custom_text".to_string()));
}

#[test]
fn test_style_book_default_style_is_accessible() {
    let overrides = HashMap::new();
    let book = StyleBook::new("default", &overrides);

    let default_style = book.get("default");
    assert!(default_style.text_color.is_some());
}

#[test]
fn test_style_book_loads_default_style() {
    let overrides = HashMap::new();
    let book = StyleBook::new("default", &overrides);

    let h_style = book.get("H");
    // Just verify that H gate has some background color and text color set
    assert!(h_style.background_color.is_some());
    assert!(h_style.text_color.is_some());
}

#[test]
fn test_style_book_loads_gray_style() {
    let overrides = HashMap::new();
    let book = StyleBook::new("gray", &overrides);

    let default_style = book.get("default");
    assert_eq!(default_style.background_color, Some("white".to_string()));
}

#[test]
fn test_style_book_case_insensitive_style_name() {
    let overrides = HashMap::new();

    let book_default = StyleBook::new("default", &overrides);
    let book_upper = StyleBook::new("DEFAULT", &overrides);
    let book_mixed = StyleBook::new("DeFaUlT", &overrides);

    let default_style = book_default.get("default");
    let upper_style = book_upper.get("default");
    let mixed_style = book_mixed.get("default");

    assert_eq!(default_style.background_color, upper_style.background_color);
    assert_eq!(default_style.background_color, mixed_style.background_color);
}

#[test]
fn test_style_book_gray_style_has_white_background() {
    let overrides = HashMap::new();
    let book = StyleBook::new("gray", &overrides);

    let default_style = book.get("default");
    assert_eq!(default_style.background_color, Some("white".to_string()));
}

#[test]
fn test_style_book_override_takes_precedence_over_builtin() {
    let mut overrides = HashMap::new();
    let custom_h = GateStyle {
        background_color: Some("override_bg".to_string()),
        ..GateStyle::default()
    };
    overrides.insert("H".to_string(), custom_h);

    let book = StyleBook::new("default", &overrides);
    let h_style = book.get("H");

    assert_eq!(h_style.background_color, Some("override_bg".to_string()));
}

#[test]
fn test_style_book_can_override_default() {
    let mut overrides = HashMap::new();
    let custom_default = GateStyle {
        background_color: Some("custom_default_bg".to_string()),
        text_color: Some("custom_default_text".to_string()),
        ..GateStyle::default()
    };
    overrides.insert("default".to_string(), custom_default);

    let book = StyleBook::new("default", &overrides);
    let default_style = book.get("default");

    assert_eq!(
        default_style.background_color,
        Some("custom_default_bg".to_string())
    );
    assert_eq!(
        default_style.text_color,
        Some("custom_default_text".to_string())
    );
}

#[test]
fn test_style_book_debug_format() {
    let overrides = HashMap::new();
    let book = StyleBook::new("default", &overrides);
    let debug_str = format!("{book:?}");
    assert!(debug_str.contains("StyleBook"));
}

#[test]
fn test_gate_style_debug_format() {
    let style = GateStyle::default();
    let debug_str = format!("{style:?}");
    assert!(debug_str.contains("GateStyle"));
}

#[test]
fn test_style_book_multiple_overrides() {
    let mut overrides = HashMap::new();

    let h_style = GateStyle {
        background_color: Some("h_bg".to_string()),
        ..GateStyle::default()
    };
    let x_style = GateStyle {
        background_color: Some("x_bg".to_string()),
        ..GateStyle::default()
    };

    overrides.insert("H".to_string(), h_style);
    overrides.insert("X".to_string(), x_style);

    let book = StyleBook::new("default", &overrides);

    assert_eq!(book.get("H").background_color, Some("h_bg".to_string()));
    assert_eq!(book.get("X").background_color, Some("x_bg".to_string()));
    assert_ne!(
        book.get("H").background_color,
        book.get("X").background_color
    );
}

#[test]
fn test_style_book_trim_style_name() {
    let overrides = HashMap::new();

    let book_trimmed = StyleBook::new("  default  ", &overrides);
    let book_normal = StyleBook::new("default", &overrides);

    let trimmed_style = book_trimmed.get("default");
    let normal_style = book_normal.get("default");

    assert_eq!(
        trimmed_style.background_color,
        normal_style.background_color
    );
}

#[test]
fn test_gate_style_deserialize_from_json_like_map() {
    use serde_json;

    let json = r#"{
        "border_color": "black",
        "background_color": "white",
        "font_size": 14.0,
        "text_color": "blue",
        "line_color": "gray",
        "line_width": 2.5
    }"#;

    let style: GateStyle = serde_json::from_str(json).unwrap();
    assert_eq!(style.border_color, Some("black".to_string()));
    assert_eq!(style.background_color, Some("white".to_string()));
    assert_eq!(style.font_size, Some(14.0));
    assert_eq!(style.text_color, Some("blue".to_string()));
    assert_eq!(style.line_color, Some("gray".to_string()));
    assert_eq!(style.line_width, Some(2.5));
}

#[test]
fn test_gate_style_deserialize_empty() {
    use serde_json;

    let json = r#"{}"#;
    let style: GateStyle = serde_json::from_str(json).unwrap();
    assert!(style.border_color.is_none());
    assert!(style.background_color.is_none());
    assert!(style.font_size.is_none());
    assert!(style.text_color.is_none());
    assert!(style.line_color.is_none());
    assert!(style.line_width.is_none());
}
