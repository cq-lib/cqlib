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

//! Parameter label formatting policies for visualization IR.
//!
//! This module decouples parameter display from the IR build pipeline. Builder code
//! collects parameters and delegates formatting to [`ParameterFormatter`].

use crate::circuit::Circuit;
use crate::circuit::circuit_param::CircuitParam;
use crate::circuit::parameter::Parameter;
use crate::visualization::VisualizationError;
use std::f64::consts::PI;

/// Display mode used by the parameter formatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterDisplayMode {
    /// Prefer numeric display when value is evaluable.
    Numeric,
    /// Prefer symbolic expression display.
    Symbolic,
    /// Show symbolic expression and append numeric value when evaluable.
    SymbolicWithValue,
    /// Prefer `kπ/n` representation for values close to common angle fractions.
    PiFractionPreferred,
}

/// Options for visualization parameter formatting.
///
/// Controls numeric precision, scientific-notation thresholds, and π-fraction matching.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::visualization::{ParameterDisplayMode, ParameterFormatOptions};
///
/// let options = ParameterFormatOptions {
///     mode: ParameterDisplayMode::PiFractionPreferred,
///     decimal_precision: 3,
///     ..ParameterFormatOptions::default()
/// };
/// assert_eq!(options.decimal_precision, 3);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ParameterFormatOptions {
    /// Display mode strategy.
    pub mode: ParameterDisplayMode,
    /// Decimal precision for fixed-point/scientific formatting.
    pub decimal_precision: usize,
    /// Values in `(0, scientific_lower_bound)` use scientific notation.
    pub scientific_lower_bound: f64,
    /// Values `>= scientific_upper_bound` use scientific notation.
    pub scientific_upper_bound: f64,
    /// Tolerance when matching `value / π` to rational fractions.
    pub pi_tolerance: f64,
    /// Maximum denominator used for π fraction matching.
    pub pi_max_denominator: i64,
}

impl Default for ParameterFormatOptions {
    fn default() -> Self {
        Self {
            mode: ParameterDisplayMode::Numeric,
            decimal_precision: 2,
            scientific_lower_bound: 1e-3,
            scientific_upper_bound: 1e4,
            pi_tolerance: 1e-3,
            pi_max_denominator: 16,
        }
    }
}

/// Parameter formatter used by visualization IR builder.
///
/// The formatter resolves fixed and symbolic circuit parameters into display labels according
/// to [`ParameterFormatOptions`].
#[derive(Debug, Clone, Copy)]
pub struct ParameterFormatter {
    options: ParameterFormatOptions,
}

impl ParameterFormatter {
    /// Create a formatter with the given options.
    ///
    /// # Arguments
    ///
    /// * `options` - Parameter display and numeric-formatting policy.
    ///
    /// # Returns
    ///
    /// A formatter that can convert fixed or symbolic circuit parameters into display labels.
    pub fn new(options: ParameterFormatOptions) -> Self {
        Self { options }
    }

    /// Format one circuit parameter entry.
    ///
    /// # Arguments
    ///
    /// * `circuit` - Circuit that owns the symbolic parameter table.
    /// * `param` - Fixed value or symbolic parameter reference to format.
    ///
    /// # Returns
    ///
    /// A short label suitable for text and figure rendering.
    ///
    /// # Errors
    ///
    /// Returns [`VisualizationError::ParameterIndexOutOfBounds`] when a symbolic index is invalid.
    pub fn format_circuit_param(
        &self,
        circuit: &Circuit,
        param: &CircuitParam,
    ) -> Result<String, VisualizationError> {
        match param {
            CircuitParam::Fixed(v) => Ok(self.format_from_expr_and_value(None, Some(*v))),
            CircuitParam::Index(idx) => {
                let p = circuit.parameters().get_index(*idx as usize).ok_or(
                    VisualizationError::ParameterIndexOutOfBounds {
                        index: *idx,
                        len: circuit.parameters().len(),
                    },
                )?;
                Ok(self.format_parameter(p))
            }
        }
    }

    fn format_parameter(&self, parameter: &Parameter) -> String {
        let expr = parameter.to_string();
        let value = parameter.evaluate(&None).ok();
        self.format_from_expr_and_value(Some(expr.as_str()), value)
    }

    fn format_from_expr_and_value(&self, expr: Option<&str>, value: Option<f64>) -> String {
        match self.options.mode {
            ParameterDisplayMode::Numeric => value
                .map(|v| self.format_numeric(v))
                .or_else(|| expr.map(|e| self.format_symbolic_expr(e)))
                .unwrap_or_else(|| "0".to_string()),
            ParameterDisplayMode::Symbolic => expr
                .map(|e| self.format_symbolic_expr(e))
                .or_else(|| value.map(|v| self.format_numeric(v)))
                .unwrap_or_else(|| "0".to_string()),
            ParameterDisplayMode::SymbolicWithValue => match (expr, value) {
                (Some(e), Some(v)) => {
                    let expr_str = self.format_symbolic_expr(e);
                    let value_str = self.format_numeric(v);
                    if expr_str == value_str {
                        expr_str
                    } else {
                        format!("{expr_str} ≈ {value_str}")
                    }
                }
                (Some(e), None) => self.format_symbolic_expr(e),
                (None, Some(v)) => self.format_numeric(v),
                (None, None) => "0".to_string(),
            },
            ParameterDisplayMode::PiFractionPreferred => value
                .and_then(|v| self.format_pi_fraction(v))
                .or_else(|| value.map(|v| self.format_numeric(v)))
                .or_else(|| expr.map(|e| self.format_symbolic_expr(e)))
                .unwrap_or_else(|| "0".to_string()),
        }
    }

    /// Normalize numeric literals embedded in symbolic expressions without touching identifiers.
    fn format_symbolic_expr(&self, expr: &str) -> String {
        let bytes = expr.as_bytes();
        let mut out = String::with_capacity(expr.len());
        let mut i = 0usize;

        while i < bytes.len() {
            // Treat a numeric token as standalone only when both neighbors are
            // outside identifier syntax, so names such as `x_11` stay intact.
            if is_number_start(bytes, i)
                && !is_ascii_identifier_byte(previous_byte(bytes, i).unwrap_or_default())
            {
                let end = scan_number_literal(bytes, i);
                let next = bytes.get(end).copied().unwrap_or_default();
                if !is_ascii_identifier_byte(next) {
                    let token = &expr[i..end];
                    if let Ok(value) = token.parse::<f64>() {
                        out.push_str(&self.format_numeric(value));
                        i = end;
                        continue;
                    }
                }
            }

            let ch = expr[i..].chars().next().unwrap_or_default();
            out.push(ch);
            i += ch.len_utf8();
        }

        out
    }

    fn format_pi_fraction(&self, value: f64) -> Option<String> {
        if !value.is_finite() {
            return None;
        }
        if value == 0.0 {
            return Some("0".to_string());
        }

        let ratio = value / PI;
        let max_den = self.options.pi_max_denominator.max(1);
        let mut best: Option<(i64, i64, f64)> = None;

        for den in 1..=max_den {
            let num = (ratio * den as f64).round() as i64;
            if num == 0 {
                continue;
            }
            let approx = num as f64 / den as f64;
            let err = (ratio - approx).abs();
            if err > self.options.pi_tolerance {
                continue;
            }

            let divisor = gcd_i64(num.abs(), den);
            let reduced_num = num / divisor;
            let reduced_den = den / divisor;
            match best {
                None => best = Some((reduced_num, reduced_den, err)),
                Some((b_num, b_den, b_err)) => {
                    if err < b_err
                        || (err == b_err
                            && (reduced_den < b_den
                                || (reduced_den == b_den && reduced_num.abs() < b_num.abs())))
                    {
                        best = Some((reduced_num, reduced_den, err));
                    }
                }
            }
        }

        let (num, den, _) = best?;
        Some(format_pi_ratio(num, den))
    }

    fn format_numeric(&self, value: f64) -> String {
        if value.is_nan() || value.is_infinite() {
            return value.to_string();
        }

        let abs = value.abs();
        if abs > 0.0
            && (abs < self.options.scientific_lower_bound
                || abs >= self.options.scientific_upper_bound)
        {
            return format_scientific(value, self.options.decimal_precision.max(1));
        }

        let mut s = format!("{:.*}", self.options.decimal_precision, value);
        if self.options.decimal_precision > 0 {
            s = s.trim_end_matches('0').trim_end_matches('.').to_string();
        }
        if s == "-0" {
            s = "0".to_string();
        }
        if s == "0" && value != 0.0 {
            return format_scientific(value, self.options.decimal_precision.max(1));
        }
        s
    }
}

fn format_pi_ratio(num: i64, den: i64) -> String {
    if den == 1 {
        return match num {
            1 => "π".to_string(),
            -1 => "-π".to_string(),
            _ => format!("{num}π"),
        };
    }
    match num {
        1 => format!("π/{den}"),
        -1 => format!("-π/{den}"),
        _ => format!("{num}π/{den}"),
    }
}

fn format_scientific(value: f64, precision: usize) -> String {
    let s = format!("{:.*e}", precision, value);
    let (mantissa, exponent) = match s.split_once('e') {
        Some(parts) => parts,
        None => return s,
    };
    let mut mantissa = mantissa
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string();
    if mantissa == "-0" {
        mantissa = "0".to_string();
    }
    let exp_value = exponent.parse::<i32>().unwrap_or(0);
    format!("{mantissa}e{exp_value}")
}

/// Return the previous byte when scanning ASCII-oriented symbolic expressions.
fn previous_byte(bytes: &[u8], index: usize) -> Option<u8> {
    index.checked_sub(1).and_then(|idx| bytes.get(idx)).copied()
}

/// Return whether a byte belongs to the identifier syntax used by parameters.
fn is_ascii_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Return whether the current byte starts a decimal number literal.
fn is_number_start(bytes: &[u8], index: usize) -> bool {
    bytes[index].is_ascii_digit()
        || (bytes[index] == b'.' && bytes.get(index + 1).is_some_and(u8::is_ascii_digit))
}

/// Scan a decimal or scientific-notation literal and return its exclusive end.
fn scan_number_literal(bytes: &[u8], start: usize) -> usize {
    let mut end = start;

    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }

    if bytes.get(end) == Some(&b'.') {
        end += 1;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
    }

    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let exp_start = end;
        let mut exp_end = end + 1;
        if matches!(bytes.get(exp_end), Some(b'+' | b'-')) {
            exp_end += 1;
        }
        let digit_start = exp_end;
        while bytes.get(exp_end).is_some_and(u8::is_ascii_digit) {
            exp_end += 1;
        }
        if exp_end > digit_start {
            end = exp_end;
        } else {
            end = exp_start;
        }
    }

    end
}

fn gcd_i64(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a.abs().max(1)
}
