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

//! Tests for the visualization error module.

use super::error::VisualizationError;
use std::error::Error;
use std::io;

#[test]
fn test_unknown_qubit_error_display() {
    let err = VisualizationError::UnknownQubit(5);
    assert_eq!(format!("{err}"), "operation references unknown qubit Q5");
}

#[test]
fn test_parameter_index_out_of_bounds_error_display() {
    let err = VisualizationError::ParameterIndexOutOfBounds { index: 10, len: 5 };
    assert_eq!(format!("{err}"), "parameter index 10 out of bounds (len=5)");
}

#[test]
fn test_svg_render_failed_error_display() {
    let err = VisualizationError::SvgRenderFailed("parse error".to_string());
    assert_eq!(format!("{err}"), "svg rendering failed: parse error");
}

#[test]
fn test_io_error_conversion() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let vis_err: VisualizationError = io_err.into();

    match vis_err {
        VisualizationError::Io(e) => {
            assert_eq!(e.kind(), io::ErrorKind::NotFound);
            assert_eq!(e.to_string(), "file not found");
        }
        _ => panic!("expected Io variant"),
    }
}

#[test]
fn test_io_error_from_std_error() {
    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
    let vis_err = VisualizationError::from(io_err);

    match vis_err {
        VisualizationError::Io(e) => {
            assert_eq!(e.kind(), io::ErrorKind::PermissionDenied);
        }
        _ => panic!("expected Io variant"),
    }
}

#[test]
fn test_error_debug_format() {
    let err = VisualizationError::UnknownQubit(3);
    let debug_str = format!("{err:?}");
    assert!(debug_str.contains("UnknownQubit"));
}

#[test]
fn test_error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<VisualizationError>();
}

#[test]
fn test_error_can_be_boxed() {
    let err = VisualizationError::UnknownQubit(1);
    let boxed: Box<dyn std::error::Error> = Box::new(err);
    assert!(boxed.to_string().contains("Q1"));
}

#[test]
fn test_error_source_for_io_variant() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "not found");
    let vis_err = VisualizationError::from(io_err);

    match vis_err {
        VisualizationError::Io(e) => {
            assert_eq!(e.kind(), io::ErrorKind::NotFound);
            assert_eq!(e.to_string(), "not found");
        }
        _ => panic!("expected Io variant"),
    }
}

#[test]
fn test_error_source_for_other_variants() {
    let err = VisualizationError::UnknownQubit(0);
    assert!(err.source().is_none());

    let err = VisualizationError::ParameterIndexOutOfBounds { index: 5, len: 3 };
    assert!(err.source().is_none());

    let err = VisualizationError::SvgRenderFailed("error".to_string());
    assert!(err.source().is_none());
}

#[test]
fn test_all_error_variants() {
    let errors = vec![
        VisualizationError::UnknownQubit(0),
        VisualizationError::ParameterIndexOutOfBounds { index: 1, len: 1 },
        VisualizationError::SvgRenderFailed("render error".to_string()),
        VisualizationError::Io(io::Error::other("other")),
    ];

    for err in errors {
        let _display = format!("{err}");
        let _debug = format!("{err:?}");
    }
}
