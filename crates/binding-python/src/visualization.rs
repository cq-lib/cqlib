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

//! Python bindings for cqlib-core visualization APIs.
//!
//! The Rust core owns all rendering logic. This module exposes thin Python
//! wrappers that convert Python options into core option structs, return SVG
//! strings, and optionally write the same SVG to `.svg` or `.png` files.

use crate::circuit::PyCircuit;
use crate::device::result::PyExecutionResult;
use crate::qis::state::density_matrix::PyDensityMatrix;
use crate::qis::state::statevector::PyStatevector;
use cqlib_core::visualization::{
    FigureDrawerOptions, ResultPlotOptions, StatePlotOptions, TextDrawerOptions,
    VisualizationError, circuit_to_figure, circuit_to_text, plot_bloch_multivector,
    plot_bloch_vector, plot_distribution, plot_histogram, plot_state_city, plot_state_paulivec,
    render_figure_to_file, render_result_plot_to_file, render_state_plot_to_file,
};
use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::prelude::*;

/// State visualization function selected by the Python entry point.
#[derive(Clone, Copy)]
enum StatePlotKind {
    /// Plot one reduced Bloch vector per qubit.
    BlochMultivector,
    /// Plot real and imaginary density-matrix components.
    StateCity,
    /// Plot Pauli-basis expectation values.
    StatePaulivec,
}

/// Convert a visualization error into a Python exception.
fn visualization_error_to_py_err(context: &str, err: VisualizationError) -> PyErr {
    match err {
        VisualizationError::Io(err) => PyIOError::new_err(format!("{context}: {err}")),
        other => PyValueError::new_err(format!("{context}: {other}")),
    }
}

/// Build result plot options from Python keyword arguments.
#[allow(clippy::too_many_arguments)]
fn result_plot_options(
    figsize: Option<(f64, f64)>,
    color: Option<Vec<String>>,
    number_to_keep: Option<usize>,
    sort: Option<String>,
    target_string: Option<String>,
    legend: Option<Vec<String>>,
    bar_labels: bool,
    title: Option<String>,
) -> ResultPlotOptions {
    ResultPlotOptions {
        figsize,
        color: color.unwrap_or_default(),
        number_to_keep,
        sort: sort.unwrap_or_else(|| "asc".to_string()),
        target_string,
        legend,
        bar_labels,
        title,
    }
}

/// Build state plot options from Python keyword arguments.
fn state_plot_options(
    title: Option<String>,
    color: Option<Vec<String>>,
    alpha: f64,
    reverse_bits: bool,
    figsize: Option<(f64, f64)>,
) -> StatePlotOptions {
    StatePlotOptions {
        title,
        color: color.unwrap_or_default(),
        alpha,
        reverse_bits,
        figsize,
    }
}

/// Optionally write a result plot SVG to disk.
fn write_result_plot_if_requested(svg: &str, output_path: Option<&str>) -> PyResult<()> {
    if let Some(path) = output_path {
        render_result_plot_to_file(svg, path)
            .map_err(|e| visualization_error_to_py_err("result plot render error", e))?;
    }
    Ok(())
}

/// Optionally write a state plot SVG to disk.
fn write_state_plot_if_requested(svg: &str, output_path: Option<&str>) -> PyResult<()> {
    if let Some(path) = output_path {
        render_state_plot_to_file(svg, path)
            .map_err(|e| visualization_error_to_py_err("state plot render error", e))?;
    }
    Ok(())
}

/// Render a state plot from either Statevector or DensityMatrix Python input.
fn render_state_plot(
    state: &Bound<'_, PyAny>,
    options: &StatePlotOptions,
    kind: StatePlotKind,
) -> PyResult<String> {
    if let Ok(statevector) = state.extract::<PyRef<'_, PyStatevector>>() {
        return match kind {
            StatePlotKind::BlochMultivector => plot_bloch_multivector(&statevector.inner, options),
            StatePlotKind::StateCity => plot_state_city(&statevector.inner, options),
            StatePlotKind::StatePaulivec => plot_state_paulivec(&statevector.inner, options),
        }
        .map_err(|e| visualization_error_to_py_err("state visualization error", e));
    }

    if let Ok(density_matrix) = state.extract::<PyRef<'_, PyDensityMatrix>>() {
        return match kind {
            StatePlotKind::BlochMultivector => {
                plot_bloch_multivector(&density_matrix.inner, options)
            }
            StatePlotKind::StateCity => plot_state_city(&density_matrix.inner, options),
            StatePlotKind::StatePaulivec => plot_state_paulivec(&density_matrix.inner, options),
        }
        .map_err(|e| visualization_error_to_py_err("state visualization error", e));
    }

    Err(PyValueError::new_err(
        "state must be a cqlib.qis.Statevector or cqlib.qis.DensityMatrix",
    ))
}

/// Draw circuit as unicode text diagram.
#[pyfunction(
    name = "draw_text",
    signature = (
        circuit,
        *,
        line_width = None,
        initial_state = false,
        reverse_bits = false,
        show_params = true,
        decompose_circuit_gates = false
    )
)]
pub fn py_draw_text(
    circuit: &PyCircuit,
    line_width: Option<isize>,
    initial_state: bool,
    reverse_bits: bool,
    show_params: bool,
    decompose_circuit_gates: bool,
) -> PyResult<String> {
    let mut options = TextDrawerOptions {
        initial_state,
        reverse_bits,
        show_params,
        decompose_circuit_gates,
        ..TextDrawerOptions::default()
    };
    if let Some(width) = line_width {
        options.line_width = width;
    }

    circuit_to_text(&circuit.inner, &options)
        .map_err(|e| visualization_error_to_py_err("text visualization error", e))
}

/// Draw circuit as SVG string.
#[pyfunction(
    name = "draw_figure",
    signature = (
        circuit,
        *,
        fold = None,
        initial_state = false,
        reverse_bits = false,
        show_params = true,
        decompose_circuit_gates = false,
        output_path = None
    )
)]
pub fn py_draw_figure(
    circuit: &PyCircuit,
    fold: Option<i32>,
    initial_state: bool,
    reverse_bits: bool,
    show_params: bool,
    decompose_circuit_gates: bool,
    output_path: Option<&str>,
) -> PyResult<String> {
    let mut options = FigureDrawerOptions {
        initial_state,
        reverse_bits,
        show_params,
        decompose_circuit_gates,
        ..FigureDrawerOptions::default()
    };
    if let Some(fold_value) = fold {
        options.fold = fold_value;
    }

    let svg = circuit_to_figure(&circuit.inner, &options)
        .map_err(|e| visualization_error_to_py_err("figure visualization error", e))?;

    if let Some(path) = output_path {
        render_figure_to_file(&circuit.inner, path, &options)
            .map_err(|e| visualization_error_to_py_err("figure render error", e))?;
    }

    Ok(svg)
}

/// Plot an execution result as a raw-count histogram.
#[pyfunction(
    name = "plot_histogram",
    signature = (
        result,
        *,
        figsize = None,
        color = None,
        number_to_keep = None,
        sort = None,
        target_string = None,
        legend = None,
        bar_labels = true,
        title = None,
        output_path = None
    )
)]
#[allow(clippy::too_many_arguments)]
pub fn py_plot_histogram(
    result: &PyExecutionResult,
    figsize: Option<(f64, f64)>,
    color: Option<Vec<String>>,
    number_to_keep: Option<usize>,
    sort: Option<String>,
    target_string: Option<String>,
    legend: Option<Vec<String>>,
    bar_labels: bool,
    title: Option<String>,
    output_path: Option<&str>,
) -> PyResult<String> {
    let options = result_plot_options(
        figsize,
        color,
        number_to_keep,
        sort,
        target_string,
        legend,
        bar_labels,
        title,
    );
    let svg = plot_histogram(&result.inner, &options)
        .map_err(|e| visualization_error_to_py_err("histogram visualization error", e))?;
    write_result_plot_if_requested(&svg, output_path)?;
    Ok(svg)
}

/// Plot an execution result as a normalized probability distribution.
#[pyfunction(
    name = "plot_distribution",
    signature = (
        result,
        *,
        figsize = None,
        color = None,
        number_to_keep = None,
        sort = None,
        target_string = None,
        legend = None,
        bar_labels = true,
        title = None,
        output_path = None
    )
)]
#[allow(clippy::too_many_arguments)]
pub fn py_plot_distribution(
    result: &PyExecutionResult,
    figsize: Option<(f64, f64)>,
    color: Option<Vec<String>>,
    number_to_keep: Option<usize>,
    sort: Option<String>,
    target_string: Option<String>,
    legend: Option<Vec<String>>,
    bar_labels: bool,
    title: Option<String>,
    output_path: Option<&str>,
) -> PyResult<String> {
    let options = result_plot_options(
        figsize,
        color,
        number_to_keep,
        sort,
        target_string,
        legend,
        bar_labels,
        title,
    );
    let svg = plot_distribution(&result.inner, &options)
        .map_err(|e| visualization_error_to_py_err("distribution visualization error", e))?;
    write_result_plot_if_requested(&svg, output_path)?;
    Ok(svg)
}

/// Plot a single Bloch vector as SVG.
#[pyfunction(
    name = "plot_bloch_vector",
    signature = (
        vector,
        *,
        title = None,
        color = None,
        alpha = 1.0,
        reverse_bits = false,
        figsize = None,
        output_path = None
    )
)]
pub fn py_plot_bloch_vector(
    vector: Vec<f64>,
    title: Option<String>,
    color: Option<Vec<String>>,
    alpha: f64,
    reverse_bits: bool,
    figsize: Option<(f64, f64)>,
    output_path: Option<&str>,
) -> PyResult<String> {
    if vector.len() != 3 {
        return Err(PyValueError::new_err(format!(
            "Bloch vector must contain exactly 3 values; got {}",
            vector.len()
        )));
    }

    let options = state_plot_options(title, color, alpha, reverse_bits, figsize);
    let svg = plot_bloch_vector([vector[0], vector[1], vector[2]], &options)
        .map_err(|e| visualization_error_to_py_err("Bloch vector visualization error", e))?;
    write_state_plot_if_requested(&svg, output_path)?;
    Ok(svg)
}

/// Plot one reduced Bloch vector per qubit.
#[pyfunction(
    name = "plot_bloch_multivector",
    signature = (
        state,
        *,
        title = None,
        color = None,
        alpha = 1.0,
        reverse_bits = false,
        figsize = None,
        output_path = None
    )
)]
pub fn py_plot_bloch_multivector(
    state: &Bound<'_, PyAny>,
    title: Option<String>,
    color: Option<Vec<String>>,
    alpha: f64,
    reverse_bits: bool,
    figsize: Option<(f64, f64)>,
    output_path: Option<&str>,
) -> PyResult<String> {
    let options = state_plot_options(title, color, alpha, reverse_bits, figsize);
    let svg = render_state_plot(state, &options, StatePlotKind::BlochMultivector)?;
    write_state_plot_if_requested(&svg, output_path)?;
    Ok(svg)
}

/// Plot real and imaginary density-matrix components.
#[pyfunction(
    name = "plot_state_city",
    signature = (
        state,
        *,
        title = None,
        color = None,
        alpha = 1.0,
        reverse_bits = false,
        figsize = None,
        output_path = None
    )
)]
pub fn py_plot_state_city(
    state: &Bound<'_, PyAny>,
    title: Option<String>,
    color: Option<Vec<String>>,
    alpha: f64,
    reverse_bits: bool,
    figsize: Option<(f64, f64)>,
    output_path: Option<&str>,
) -> PyResult<String> {
    let options = state_plot_options(title, color, alpha, reverse_bits, figsize);
    let svg = render_state_plot(state, &options, StatePlotKind::StateCity)?;
    write_state_plot_if_requested(&svg, output_path)?;
    Ok(svg)
}

/// Plot Pauli-basis expectation values.
#[pyfunction(
    name = "plot_state_paulivec",
    signature = (
        state,
        *,
        title = None,
        color = None,
        alpha = 1.0,
        reverse_bits = false,
        figsize = None,
        output_path = None
    )
)]
pub fn py_plot_state_paulivec(
    state: &Bound<'_, PyAny>,
    title: Option<String>,
    color: Option<Vec<String>>,
    alpha: f64,
    reverse_bits: bool,
    figsize: Option<(f64, f64)>,
    output_path: Option<&str>,
) -> PyResult<String> {
    let options = state_plot_options(title, color, alpha, reverse_bits, figsize);
    let svg = render_state_plot(state, &options, StatePlotKind::StatePaulivec)?;
    write_state_plot_if_requested(&svg, output_path)?;
    Ok(svg)
}

/// Registers visualization functions as `_native.visualization`.
pub(crate) fn register_visualization_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "visualization")?;

    m.add_function(wrap_pyfunction!(py_draw_text, &m)?)?;
    m.add_function(wrap_pyfunction!(py_draw_figure, &m)?)?;
    m.add_function(wrap_pyfunction!(py_plot_histogram, &m)?)?;
    m.add_function(wrap_pyfunction!(py_plot_distribution, &m)?)?;
    m.add_function(wrap_pyfunction!(py_plot_bloch_vector, &m)?)?;
    m.add_function(wrap_pyfunction!(py_plot_bloch_multivector, &m)?)?;
    m.add_function(wrap_pyfunction!(py_plot_state_city, &m)?)?;
    m.add_function(wrap_pyfunction!(py_plot_state_paulivec, &m)?)?;

    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.visualization", &m)?;

    Ok(())
}
