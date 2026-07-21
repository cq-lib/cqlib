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

use crate::circuit::gate::circuit_gate::{CircuitGate, FrozenCircuit};
use crate::circuit::{
    Circuit, ClassicalExpr, ClassicalType, ClassicalVar, Instruction, Parameter, ParameterValue,
    Qubit, QubitDomain, StandardGate, UnitaryGate,
};
use oq3_semantics::asg::{
    self, ArithOp, BinaryOp, CmpOp, Expr, ForIterable, GateModifier, GateOperand, IndexOperator,
    LValue, Literal, Stmt, TExpr, UnaryOp,
};
use oq3_semantics::context::Context;
use oq3_semantics::semantic_error::{SemanticErrorKind, SemanticErrorList};
use oq3_semantics::symbols::{SymbolId, SymbolIdResult, SymbolTable, SymbolType};
use oq3_semantics::syntax_to_semantics;
use oq3_semantics::types::{ArrayDims, Type};
use regex::Regex;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const DEFAULT_MAX_RECURSION_DEPTH: usize = 100;

/// Parses an OpenQASM 3 file and lowers it into a Cqlib [`Circuit`].
///
/// Relative includes are resolved from the input file's parent directory.
/// `stdgates.inc` is handled by `oq3_semantics` and does not require a file
/// on disk.
/// Physical qubits such as `$5` retain their numeric identifiers. Programs
/// that mix declared logical qubits with physical qubits are rejected because
/// the current circuit IR does not distinguish the two namespaces.
///
/// # Errors
///
/// Returns [`Qasm3ParseError`] when the source is syntactically invalid,
/// semantically invalid, or uses an OpenQASM 3 feature that cannot be
/// represented by the current Cqlib circuit IR.
///
/// # Example
///
/// ```no_run
/// use cqlib_core::ir::qasm3_load;
///
/// let circuit = qasm3_load("bell.qasm").unwrap();
/// assert_eq!(circuit.num_qubits(), 2);
/// ```
pub fn load<P: AsRef<Path>>(path: P) -> Result<Circuit, Qasm3ParseError> {
    let path = path.as_ref();
    let source = fs::read_to_string(path).map_err(Qasm3ParseError::IoError)?;
    let source = prepare_source(&source)?;
    let source = inject_extension_gate_definitions(&source);
    let search_paths = path.parent().map(|parent| vec![parent.to_path_buf()]);
    let result = catch_unwind(AssertUnwindSafe(|| {
        syntax_to_semantics::parse_source_string(&source, path.to_str(), search_paths.as_deref())
    }))
    .map_err(frontend_panic_error)?;
    let has_syntax_errors = result.any_syntax_errors();
    convert_parse_result(result.take_context(), has_syntax_errors, &source)
}

/// Parse an OpenQASM 3 file and lower it into a Cqlib [`Circuit`].
///
/// Rust-style alias for [`load`]. The Python-style `load` name is retained for
/// compatibility with the rest of the IR module API.
pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Circuit, Qasm3ParseError> {
    load(path)
}

/// Parses an OpenQASM 3 source string and lowers it into a Cqlib [`Circuit`].
///
/// The loader accepts both `OPENQASM 3;` and `OPENQASM 3.0;`. The former is
/// normalized before calling `oq3_semantics` because version `0.7.0` of that
/// crate expects a minor version in the header.
/// Physical qubits such as `$5` retain their numeric identifiers. Programs
/// that mix declared logical qubits with physical qubits are rejected.
///
/// # Example
///
/// ```rust
/// use cqlib_core::ir::qasm3_loads;
///
/// let circuit = qasm3_loads(r#"
///     OPENQASM 3;
///     include "stdgates.inc";
///     qubit q;
///     x q;
/// "#).unwrap();
///
/// assert_eq!(circuit.operations().len(), 1);
/// ```
pub fn loads(source: &str) -> Result<Circuit, Qasm3ParseError> {
    let source = prepare_source(source)?;
    let source = inject_extension_gate_definitions(&source);
    let result = catch_unwind(AssertUnwindSafe(|| {
        syntax_to_semantics::parse_source_string(&source, Some("qasm3_source"), None::<&[PathBuf]>)
    }))
    .map_err(frontend_panic_error)?;
    let has_syntax_errors = result.any_syntax_errors();
    convert_parse_result(result.take_context(), has_syntax_errors, &source)
}

/// Parse an OpenQASM 3 source string and lower it into a Cqlib [`Circuit`].
///
/// Rust-style alias for [`loads`].
pub fn from_str(source: &str) -> Result<Circuit, Qasm3ParseError> {
    loads(source)
}

fn convert_parse_result(
    context: Context,
    has_syntax_errors: bool,
    source: &str,
) -> Result<Circuit, Qasm3ParseError> {
    if has_syntax_errors {
        return Err(Qasm3ParseError::ParseError(
            "OpenQASM 3 parser reported syntax errors".to_string(),
        ));
    }
    let (program, semantic_errors, symbols) = context.as_tuple();
    if semantic_errors.any_semantic_errors()
        && !only_scalar_measurement_frontend_errors(&semantic_errors, source)
    {
        return Err(Qasm3ParseError::SemanticError(
            "OpenQASM 3 parser reported semantic errors".to_string(),
        ));
    }
    let hardware_qubits = collect_hardware_qubits(source)?;
    let mut lowering = LoweringContext::new(&symbols, hardware_qubits);
    lowering.lower_program(&program)
}

fn collect_hardware_qubits(source: &str) -> Result<Vec<Qubit>, Qasm3ParseError> {
    let layout = inspect_source(source)?;
    let mut qubits = HashSet::new();
    for captures in hardware_qubit_regex().captures_iter(&layout.code_mask) {
        let identifier = captures
            .name("identifier")
            .expect("hardware-qubit regex always captures an identifier")
            .as_str();
        let index = identifier.parse::<u32>().map_err(|_| {
            Qasm3ParseError::InvalidArgument(format!(
                "physical qubit index '${identifier}' exceeds the supported range"
            ))
        })?;
        qubits.insert(Qubit::new(index));
    }
    let mut qubits = qubits.into_iter().collect::<Vec<_>>();
    qubits.sort_by_key(|qubit| qubit.index());
    Ok(qubits)
}

fn frontend_panic_error(payload: Box<dyn std::any::Any + Send>) -> Qasm3ParseError {
    let message = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("unknown OpenQASM front-end failure");
    Qasm3ParseError::ParseError(format!("OpenQASM 3 front-end failed: {message}"))
}

fn prepare_source(source: &str) -> Result<String, Qasm3ParseError> {
    let layout = inspect_source(source)?;
    reject_empty_pragmas(&layout.code_mask)?;

    let source = normalize_single_quoted_includes(source, &layout)?;
    let source = normalize_openqasm3_header(&source, &layout.code_mask);
    let source = normalize_top_level_anonymous_scopes(&source)?;
    normalize_bit_boolean_initializers(&source)
}

#[derive(Debug)]
struct SourceLayout {
    code_mask: String,
    strings: Vec<StringSpan>,
}

#[derive(Clone, Copy, Debug)]
struct StringSpan {
    start: usize,
    end: usize,
    quote: u8,
}

#[derive(Clone, Copy)]
enum ScanState {
    Code,
    LineComment,
    BlockComment,
    String { start: usize, quote: u8 },
}

fn inspect_source(source: &str) -> Result<SourceLayout, Qasm3ParseError> {
    let bytes = source.as_bytes();
    let mut mask = bytes.to_vec();
    let mut strings = Vec::new();
    let mut state = ScanState::Code;
    let mut index = 0usize;

    while index < bytes.len() {
        match state {
            ScanState::Code if bytes[index..].starts_with(b"//") => {
                mask[index] = b' ';
                mask[index + 1] = b' ';
                state = ScanState::LineComment;
                index += 2;
            }
            ScanState::Code if bytes[index..].starts_with(b"/*") => {
                mask[index] = b' ';
                mask[index + 1] = b' ';
                state = ScanState::BlockComment;
                index += 2;
            }
            ScanState::Code if matches!(bytes[index], b'\'' | b'"') => {
                let quote = bytes[index];
                mask[index] = b' ';
                state = ScanState::String {
                    start: index,
                    quote,
                };
                index += 1;
            }
            ScanState::Code => index += 1,
            ScanState::LineComment if matches!(bytes[index], b'\n' | b'\r') => {
                state = ScanState::Code;
                index += 1;
            }
            ScanState::LineComment => {
                mask[index] = b' ';
                index += 1;
            }
            ScanState::BlockComment if bytes[index..].starts_with(b"/*") => {
                return Err(Qasm3ParseError::ParseError(
                    "nested block comments are not valid OpenQASM 3".to_string(),
                ));
            }
            ScanState::BlockComment if bytes[index..].starts_with(b"*/") => {
                mask[index] = b' ';
                mask[index + 1] = b' ';
                state = ScanState::Code;
                index += 2;
            }
            ScanState::BlockComment => {
                if !matches!(bytes[index], b'\n' | b'\r') {
                    mask[index] = b' ';
                }
                index += 1;
            }
            ScanState::String { start, quote } if bytes[index] == quote => {
                mask[index] = b' ';
                strings.push(StringSpan {
                    start,
                    end: index,
                    quote,
                });
                state = ScanState::Code;
                index += 1;
            }
            ScanState::String { .. } if matches!(bytes[index], b'\n' | b'\r') => {
                return Err(Qasm3ParseError::ParseError(
                    "unterminated string literal".to_string(),
                ));
            }
            ScanState::String { .. } => {
                mask[index] = b' ';
                index += 1;
            }
        }
    }

    match state {
        ScanState::BlockComment => {
            return Err(Qasm3ParseError::ParseError(
                "unterminated block comment".to_string(),
            ));
        }
        ScanState::String { .. } => {
            return Err(Qasm3ParseError::ParseError(
                "unterminated string literal".to_string(),
            ));
        }
        ScanState::Code | ScanState::LineComment => {}
    }

    let code_mask = String::from_utf8(mask).expect("source mask preserves UTF-8");
    Ok(SourceLayout { code_mask, strings })
}

fn reject_empty_pragmas(code_mask: &str) -> Result<(), Qasm3ParseError> {
    if code_mask
        .lines()
        .any(|line| matches!(line.trim(), "pragma" | "#pragma"))
    {
        return Err(Qasm3ParseError::ParseError(
            "pragma requires content on the same line".to_string(),
        ));
    }
    Ok(())
}

fn normalize_single_quoted_includes(
    source: &str,
    layout: &SourceLayout,
) -> Result<String, Qasm3ParseError> {
    let mut bytes = source.as_bytes().to_vec();
    for span in layout.strings.iter().filter(|span| span.quote == b'\'') {
        let line_start = layout.code_mask[..span.start]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let prefix = layout.code_mask[line_start..span.start].trim_end();
        if prefix.split_whitespace().last() != Some("include") {
            continue;
        }
        let line_end = layout.code_mask[span.end + 1..]
            .find('\n')
            .map_or(layout.code_mask.len(), |offset| span.end + 1 + offset);
        if !layout.code_mask[span.end + 1..line_end]
            .trim_start()
            .starts_with(';')
        {
            continue;
        }
        if source.as_bytes()[span.start + 1..span.end].contains(&b'"') {
            return Err(Qasm3ParseError::UnsupportedFeature(
                "single-quoted include path containing a double quote".to_string(),
            ));
        }
        bytes[span.start] = b'"';
        bytes[span.end] = b'"';
    }
    Ok(String::from_utf8(bytes).expect("quote normalization preserves UTF-8"))
}

fn normalize_openqasm3_header(source: &str, code_mask: &str) -> String {
    let Some(captures) = short_header_regex().captures(code_mask) else {
        return source.to_string();
    };
    let Some(major) = captures.name("major") else {
        return source.to_string();
    };
    let mut normalized = source.to_string();
    normalized.insert_str(major.end(), ".0");
    normalized
}

fn normalize_top_level_anonymous_scopes(source: &str) -> Result<String, Qasm3ParseError> {
    let layout = inspect_source(source)?;
    let bytes = layout.code_mask.as_bytes();
    let mut depth = 0usize;
    let mut statement_start = 0usize;
    let mut insertions = Vec::new();

    for (index, byte) in bytes.iter().copied().enumerate() {
        match byte {
            b';' if depth == 0 => statement_start = index + 1,
            b'{' if depth == 0 => {
                if layout.code_mask[statement_start..index].trim().is_empty() {
                    insertions.push(index);
                }
                depth = 1;
            }
            b'{' => depth += 1,
            b'}' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    statement_start = index + 1;
                }
            }
            _ => {}
        }
    }

    let mut normalized = source.to_string();
    for index in insertions.into_iter().rev() {
        // oq3_semantics 0.7.0 panics on anonymous BlockExpr. A constant-true
        // branch gives its parser the same lexical scope and execution semantics.
        normalized.insert_str(index, "if (true) ");
    }
    Ok(normalized)
}

fn normalize_bit_boolean_initializers(source: &str) -> Result<String, Qasm3ParseError> {
    let layout = inspect_source(source)?;
    let mut insertions = bit_boolean_initializer_regex()
        .captures_iter(&layout.code_mask)
        .filter_map(|captures| captures.name("value"))
        .flat_map(|value| [(value.start(), "bit("), (value.end(), ")")])
        .collect::<Vec<_>>();
    insertions.sort_unstable_by_key(|(index, _)| *index);

    let mut normalized = source.to_string();
    for (index, value) in insertions.into_iter().rev() {
        normalized.insert_str(index, value);
    }
    Ok(normalized)
}

fn inject_extension_gate_definitions(source: &str) -> String {
    let definitions = [
        (
            "rxx",
            "gate rxx(theta) a,b { h a; h b; cx a,b; rz(theta) b; cx a,b; h a; h b; }",
        ),
        (
            "ryy",
            "gate ryy(theta) a,b { rx(pi/2) a; rx(pi/2) b; cx a,b; rz(theta) b; cx a,b; rx(-pi/2) a; rx(-pi/2) b; }",
        ),
        (
            "rzz",
            "gate rzz(theta) a,b { cx a,b; rz(theta) b; cx a,b; }",
        ),
        (
            "rzx",
            "gate rzx(theta) a,b { h b; cx a,b; rz(theta) b; cx a,b; h b; }",
        ),
        (
            "fsim",
            "gate fsim(theta,phi) a,b { rxx(theta) a,b; ryy(theta) a,b; gphase(-phi/4); rz(-phi/2) a; rz(-phi/2) b; rzz(phi/2) a,b; }",
        ),
    ];
    let injected = definitions
        .iter()
        .filter_map(|(name, definition)| {
            if has_gate_definition(source, name) || !has_gate_call(source, name) {
                None
            } else {
                Some(*definition)
            }
        })
        .collect::<Vec<_>>();
    if injected.is_empty() {
        return source.to_string();
    }

    let insertion = format!("\n{}\n", injected.join("\n\n"));
    let mut lines = source
        .lines()
        .map(ToString::to_string)
        .collect::<Vec<String>>();
    let insert_after = lines
        .iter()
        .rposition(|line| line.trim() == "include \"stdgates.inc\";")
        .or_else(|| {
            lines
                .iter()
                .position(|line| line.trim_start().starts_with("OPENQASM "))
        });
    if let Some(index) = insert_after {
        lines.insert(index + 1, insertion);
    } else {
        lines.insert(0, insertion);
    }

    let mut rewritten = lines.join("\n");
    if source.ends_with('\n') {
        rewritten.push('\n');
    }
    rewritten
}

fn has_gate_definition(source: &str, name: &str) -> bool {
    gate_definition_regex(name).is_match(source)
}

fn has_gate_call(source: &str, name: &str) -> bool {
    gate_call_regex(name).is_match(source)
}

fn only_scalar_measurement_frontend_errors(errors: &SemanticErrorList, source: &str) -> bool {
    errors.include_errors().is_empty()
        && !errors.is_empty()
        && errors
            .iter()
            .all(|error| scalar_measurement_frontend_error(error, source))
}

fn scalar_measurement_frontend_error(
    error: &oq3_semantics::semantic_error::SemanticError,
    source: &str,
) -> bool {
    if !matches!(error.kind(), SemanticErrorKind::IncompatibleTypesError) {
        return false;
    }
    let range = error.range();
    let start = u32::from(range.start()) as usize;
    let end = u32::from(range.end()) as usize;
    let Some(snippet) = source.get(start..end) else {
        return false;
    };
    let Some(captures) = scalar_measurement_error_regex().captures(snippet) else {
        return false;
    };
    let Some(target) = captures.name("target") else {
        return false;
    };
    scalar_bit_declaration_count(source, target.as_str()) == 1
}

fn scalar_bit_declaration_count(source: &str, identifier: &str) -> usize {
    let Ok(layout) = inspect_source(source) else {
        return 0;
    };
    let regex = Regex::new(&format!(
        r"(?m)^[\t ]*bit[\t ]+{}[\t ]*;",
        regex::escape(identifier)
    ))
    .expect("escaped identifier produces a valid regex");
    regex.find_iter(&layout.code_mask).count()
}

fn short_header_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"\A\s*OPENQASM\s+(?P<major>3)[\t \r\n]*;").expect("valid short header regex")
    })
}

fn bit_boolean_initializer_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?m)^[\t ]*bit[\t ]+[\p{L}\p{Nl}_][\p{L}\p{Nl}\p{Nd}_]*[\t ]*=[\t ]*(?P<value>true|false)[\t ]*;",
        )
        .expect("valid bit boolean initializer regex")
    })
}

fn scalar_measurement_error_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?s)^\s*(?P<target>[\p{L}\p{Nl}_][\p{L}\p{Nl}\p{Nd}_]*)\s*=\s*measure\s+[\p{L}\p{Nl}_][\p{L}\p{Nl}\p{Nd}_]*\s*\[\s*\d+\s*\]\s*;?\s*$",
        )
        .expect("valid scalar measurement compatibility regex")
    })
}

fn hardware_qubit_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"\$(?P<identifier>[0-9]+)").expect("valid hardware-qubit regex")
    })
}

fn gate_definition_regex(name: &str) -> Regex {
    Regex::new(&format!(r"(?m)^\s*gate\s+{}\b", regex::escape(name)))
        .expect("valid gate definition regex")
}

fn gate_call_regex(name: &str) -> Regex {
    Regex::new(&format!(
        r"(?m)^\s*{}\s*(?:\(|[A-Za-z_])",
        regex::escape(name)
    ))
    .expect("valid gate call regex")
}

/// Error returned while parsing or lowering OpenQASM 3 into Cqlib.
///
/// `ParseError` and `SemanticError` come from the OpenQASM front-end. The
/// other variants are produced by Cqlib's lowering layer when a valid ASG
/// cannot be represented as a [`Circuit`] without losing semantics.
#[derive(Debug)]
pub enum Qasm3ParseError {
    /// File system or I/O error while reading OpenQASM source.
    IoError(io::Error),
    /// The OpenQASM parser reported syntax errors.
    ParseError(String),
    /// The OpenQASM semantic analyzer reported unresolved symbols, invalid
    /// gate arity, or type errors before Cqlib lowering began.
    SemanticError(String),
    /// A Cqlib circuit construction error occurred while appending lowered IR.
    ConversionError(String),
    /// The source uses a valid OpenQASM 3 feature not supported by this loader.
    UnsupportedFeature(String),
    /// A symbol reference could not be resolved in Cqlib's lowering context.
    UndefinedSymbol(String),
    /// A gate reference could not be resolved as a standard or custom gate.
    UndefinedGate(String),
    /// A value had an incompatible OpenQASM/Cqlib type.
    TypeError(String),
    /// A literal, index, range, or declaration argument is invalid.
    InvalidArgument(String),
    /// A gate application used the wrong number of qubits.
    MismatchedQubitCount { expected: usize, actual: usize },
    /// A gate application used the wrong number of parameters.
    MismatchedParameterCount { expected: usize, actual: usize },
    /// A custom-gate dependency chain exceeded the configured recursion limit.
    RecursionLimitExceeded(String),
    /// A custom gate depends on itself directly or indirectly.
    CircularGateDependency { gate: String, dependency: String },
}

impl PartialEq for Qasm3ParseError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::IoError(lhs), Self::IoError(rhs)) => {
                lhs.kind() == rhs.kind() && lhs.to_string() == rhs.to_string()
            }
            (Self::ParseError(lhs), Self::ParseError(rhs)) => lhs == rhs,
            (Self::SemanticError(lhs), Self::SemanticError(rhs)) => lhs == rhs,
            (Self::ConversionError(lhs), Self::ConversionError(rhs)) => lhs == rhs,
            (Self::UnsupportedFeature(lhs), Self::UnsupportedFeature(rhs)) => lhs == rhs,
            (Self::UndefinedSymbol(lhs), Self::UndefinedSymbol(rhs)) => lhs == rhs,
            (Self::UndefinedGate(lhs), Self::UndefinedGate(rhs)) => lhs == rhs,
            (Self::TypeError(lhs), Self::TypeError(rhs)) => lhs == rhs,
            (Self::InvalidArgument(lhs), Self::InvalidArgument(rhs)) => lhs == rhs,
            (
                Self::MismatchedQubitCount {
                    expected: lhs_expected,
                    actual: lhs_actual,
                },
                Self::MismatchedQubitCount {
                    expected: rhs_expected,
                    actual: rhs_actual,
                },
            ) => lhs_expected == rhs_expected && lhs_actual == rhs_actual,
            (
                Self::MismatchedParameterCount {
                    expected: lhs_expected,
                    actual: lhs_actual,
                },
                Self::MismatchedParameterCount {
                    expected: rhs_expected,
                    actual: rhs_actual,
                },
            ) => lhs_expected == rhs_expected && lhs_actual == rhs_actual,
            (Self::RecursionLimitExceeded(lhs), Self::RecursionLimitExceeded(rhs)) => lhs == rhs,
            (
                Self::CircularGateDependency {
                    gate: lhs_gate,
                    dependency: lhs_dependency,
                },
                Self::CircularGateDependency {
                    gate: rhs_gate,
                    dependency: rhs_dependency,
                },
            ) => lhs_gate == rhs_gate && lhs_dependency == rhs_dependency,
            _ => false,
        }
    }
}

impl std::fmt::Display for Qasm3ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(error) => write!(f, "IO error: {error}"),
            Self::ParseError(s) => write!(f, "Parse error: {s}"),
            Self::SemanticError(s) => write!(f, "Semantic error: {s}"),
            Self::ConversionError(s) => write!(f, "Conversion error: {s}"),
            Self::UnsupportedFeature(s) => write!(f, "Unsupported OpenQASM 3 feature: {s}"),
            Self::UndefinedSymbol(s) => write!(f, "Undefined symbol: {s}"),
            Self::UndefinedGate(s) => write!(f, "Undefined gate: {s}"),
            Self::TypeError(s) => write!(f, "Type error: {s}"),
            Self::InvalidArgument(s) => write!(f, "Invalid argument: {s}"),
            Self::MismatchedQubitCount { expected, actual } => {
                write!(
                    f,
                    "Mismatched qubit count: expected {expected}, got {actual}"
                )
            }
            Self::MismatchedParameterCount { expected, actual } => {
                write!(
                    f,
                    "Mismatched parameter count: expected {expected}, got {actual}"
                )
            }
            Self::RecursionLimitExceeded(s) => write!(f, "Recursion limit exceeded: {s}"),
            Self::CircularGateDependency { gate, dependency } => write!(
                f,
                "Circular gate dependency detected: '{gate}' depends on '{dependency}'"
            ),
        }
    }
}

impl std::error::Error for Qasm3ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(error) => Some(error),
            _ => None,
        }
    }
}

impl From<crate::circuit::CircuitError> for Qasm3ParseError {
    fn from(value: crate::circuit::CircuitError) -> Self {
        Self::ConversionError(value.to_string())
    }
}

#[derive(Clone, Debug)]
enum QuantumBinding {
    Qubit(Qubit),
    Register(Vec<Qubit>),
}

impl QuantumBinding {
    fn qubits(&self) -> Vec<Qubit> {
        match self {
            Self::Qubit(q) => vec![*q],
            Self::Register(qs) => qs.clone(),
        }
    }
}

#[derive(Clone, Debug)]
enum GateDef {
    Circuit(CircuitGate),
    Opaque {
        name: String,
        qubits: usize,
        params: usize,
    },
    Source(asg::GateDefinition),
}

struct LoweringContext<'a> {
    symbols: &'a SymbolTable,
    quantum: HashMap<SymbolId, QuantumBinding>,
    hardware_qubits: Vec<Qubit>,
    classical: HashMap<SymbolId, ClassicalVar>,
    gates: HashMap<SymbolId, GateDef>,
    loop_constants: HashMap<SymbolId, u128>,
    compiling_gates: HashSet<SymbolId>,
    recursion_depth: usize,
    max_recursion_depth: usize,
}

impl<'a> LoweringContext<'a> {
    fn new(symbols: &'a SymbolTable, hardware_qubits: Vec<Qubit>) -> Self {
        Self {
            symbols,
            quantum: HashMap::new(),
            hardware_qubits,
            classical: HashMap::new(),
            gates: HashMap::new(),
            loop_constants: HashMap::new(),
            compiling_gates: HashSet::new(),
            recursion_depth: 0,
            max_recursion_depth: DEFAULT_MAX_RECURSION_DEPTH,
        }
    }

    fn lower_program(&mut self, program: &asg::Program) -> Result<Circuit, Qasm3ParseError> {
        if let Some(version) = program.version()
            && version.major() != 3
        {
            return Err(Qasm3ParseError::UnsupportedFeature(format!(
                "OPENQASM major version {}",
                version.major()
            )));
        }

        let total_qubits = self.discover_quantum(program.stmts())?;
        self.discover_gates(program.stmts())?;

        if total_qubits > 0 && !self.hardware_qubits.is_empty() {
            return Err(Qasm3ParseError::UnsupportedFeature(
                "mixing logical and physical qubits".to_string(),
            ));
        }

        let mut circuit = if self.hardware_qubits.is_empty() {
            Circuit::new(total_qubits)
        } else {
            let mut circuit = Circuit::from_qubits(self.hardware_qubits.clone())?;
            circuit.set_qubit_domain(QubitDomain::Physical);
            circuit
        };

        for stmt in program.stmts() {
            self.lower_stmt(stmt, &mut circuit)?;
        }

        Ok(circuit)
    }

    fn discover_quantum(&mut self, stmts: &[Stmt]) -> Result<usize, Qasm3ParseError> {
        let mut next = 0usize;
        for stmt in stmts {
            if let Stmt::DeclareQuantum(decl) = stmt {
                let id = self.symbol_id(decl.name())?;
                let ty = self.symbol_type(&id);
                let width = match ty {
                    Type::Qubit => 1,
                    Type::QubitArray(ArrayDims::D1(width)) => *width,
                    _ => {
                        return Err(Qasm3ParseError::TypeError(format!(
                            "quantum declaration '{}' has type {:?}",
                            self.symbol_name(&id),
                            ty
                        )));
                    }
                };
                let qubits = (next..next + width)
                    .map(|index| Qubit::new(index as u32))
                    .collect::<Vec<_>>();
                next += width;
                let binding = if width == 1 {
                    QuantumBinding::Qubit(qubits[0])
                } else {
                    QuantumBinding::Register(qubits)
                };
                self.quantum.insert(id, binding);
            }
        }
        Ok(next)
    }

    fn discover_gates(&mut self, stmts: &[Stmt]) -> Result<(), Qasm3ParseError> {
        for stmt in stmts {
            match stmt {
                Stmt::GateDefinition(def) => {
                    let id = self.symbol_id(def.name())?;
                    self.gates.insert(id, GateDef::Source(def.clone()));
                }
                Stmt::Include(_) => {}
                _ => {}
            }
        }

        for (name, id, params, qubits) in self.symbols.gates() {
            self.gates.entry(id).or_insert_with(|| GateDef::Opaque {
                name: name.to_string(),
                qubits,
                params,
            });
        }
        Ok(())
    }

    fn lower_classical_declaration(
        &mut self,
        decl: &asg::DeclareClassical,
        circuit: &mut Circuit,
    ) -> Result<(), Qasm3ParseError> {
        let id = self.symbol_id(decl.name())?;
        let ty = self.classical_type(self.symbol_type(&id))?;
        let var = circuit.var(ty);
        self.classical.insert(id, var);
        if let Some(initializer) = decl.initializer() {
            let value = self.lower_classical_expr(initializer)?;
            circuit.store(var, value)?;
        }
        Ok(())
    }

    fn lower_stmt(&mut self, stmt: &Stmt, circuit: &mut Circuit) -> Result<(), Qasm3ParseError> {
        match stmt {
            Stmt::DeclareQuantum(_) | Stmt::Include(_) => Ok(()),
            Stmt::DeclareClassical(decl) => self.lower_classical_declaration(decl, circuit),
            Stmt::GateDefinition(_) => Ok(()),
            Stmt::GateCall(call) => self.lower_gate_call(call, circuit),
            Stmt::GPhaseCall(call) => {
                let phase = self.lower_angle(call.arg())?;
                let new_phase = circuit.global_phase() + phase;
                circuit.set_global_phase(new_phase);
                Ok(())
            }
            Stmt::ModifiedGPhaseCall(_) => Err(Qasm3ParseError::UnsupportedFeature(
                "modified gphase".to_string(),
            )),
            Stmt::Barrier(barrier) => {
                let qubits = match barrier.qubits() {
                    Some(exprs) => self.expand_qubit_exprs(exprs)?,
                    None => circuit.qubits(),
                };
                circuit.barrier(qubits)?;
                Ok(())
            }
            Stmt::Reset(reset) => {
                for qubit in self.expand_qubit_expr(reset.gate_operand())? {
                    circuit.reset(qubit)?;
                }
                Ok(())
            }
            Stmt::Assignment(assignment) => self.lower_assignment(assignment, circuit),
            Stmt::ExprStmt(expr) => {
                if let Expr::MeasureExpression(measure) = expr.expression() {
                    for qubit in self.expand_qubit_expr(measure.operand())? {
                        circuit.measure(qubit)?;
                    }
                    Ok(())
                } else {
                    Err(Qasm3ParseError::UnsupportedFeature(
                        "expression statement".to_string(),
                    ))
                }
            }
            Stmt::If(op) => {
                let condition = self.lower_condition(op.condition())?;
                if condition.is_bool_true() {
                    return self.lower_stmt_slice(op.then_branch().statements(), circuit);
                }
                if condition.is_bool_false() {
                    return op.else_branch().map_or(Ok(()), |else_branch| {
                        self.lower_stmt_slice(else_branch.statements(), circuit)
                    });
                }
                if let Some(else_branch) = op.else_branch() {
                    let ctx = RefCell::new(self);
                    circuit.if_else(
                        condition,
                        |then_circuit| ctx.borrow_mut().lower_block(op.then_branch(), then_circuit),
                        |else_circuit| ctx.borrow_mut().lower_block(else_branch, else_circuit),
                    )?;
                } else {
                    circuit.if_(condition, |then_circuit| {
                        self.lower_block(op.then_branch(), then_circuit)
                    })?;
                }
                Ok(())
            }
            Stmt::While(op) => {
                let condition = self.lower_condition(op.condition())?;
                circuit.while_(condition, |body| self.lower_block(op.loop_body(), body))?;
                Ok(())
            }
            Stmt::ForStmt(op) => self.lower_for(op, circuit),
            Stmt::SwitchCaseStmt(op) => self.lower_switch(op, circuit),
            Stmt::Break => {
                circuit.break_loop()?;
                Ok(())
            }
            Stmt::Continue => {
                circuit.continue_loop()?;
                Ok(())
            }
            Stmt::Block(block) => {
                self.lower_block(block, circuit)?;
                Ok(())
            }
            Stmt::Pragma(_) | Stmt::AnnotatedStmt(_) => Err(Qasm3ParseError::UnsupportedFeature(
                "pragma or annotation".to_string(),
            )),
            Stmt::Delay(_) => Err(Qasm3ParseError::UnsupportedFeature(
                "delay/timing".to_string(),
            )),
            Stmt::DefStmt(_) => Err(Qasm3ParseError::UnsupportedFeature(
                "subroutine def".to_string(),
            )),
            Stmt::Extern => Err(Qasm3ParseError::UnsupportedFeature("extern".to_string())),
            Stmt::Cal | Stmt::DefCal => Err(Qasm3ParseError::UnsupportedFeature(
                "calibration/defcal".to_string(),
            )),
            Stmt::InputDeclaration(input) => {
                let id = self.symbol_id(input.name())?;
                match self.symbol_type(&id) {
                    Type::Angle(_, _) | Type::Float(_, _) => Ok(()),
                    ty => Err(Qasm3ParseError::UnsupportedFeature(format!(
                        "input declaration of type {ty:?}"
                    ))),
                }
            }
            Stmt::OutputDeclaration(_) => Err(Qasm3ParseError::UnsupportedFeature(
                "output declaration".to_string(),
            )),
            Stmt::Alias(_) => Err(Qasm3ParseError::UnsupportedFeature("alias".to_string())),
            Stmt::DeclareHardwareQubit(_) => Err(Qasm3ParseError::UnsupportedFeature(
                "hardware qubit".to_string(),
            )),
            Stmt::Box => Err(Qasm3ParseError::UnsupportedFeature("box".to_string())),
            Stmt::End | Stmt::NullStmt => Ok(()),
            Stmt::OldStyleDeclaration => Err(Qasm3ParseError::UnsupportedFeature(
                "old-style declaration".to_string(),
            )),
        }
    }

    fn lower_block(
        &mut self,
        block: &asg::Block,
        circuit: &mut Circuit,
    ) -> Result<(), crate::circuit::CircuitError> {
        self.lower_stmt_slice(block.statements(), circuit)
            .map_err(|e| crate::circuit::CircuitError::InvalidOperation(e.to_string()))
    }

    fn lower_stmt_slice(
        &mut self,
        stmts: &[Stmt],
        circuit: &mut Circuit,
    ) -> Result<(), Qasm3ParseError> {
        for stmt in stmts {
            self.lower_stmt(stmt, circuit)?;
        }
        Ok(())
    }

    fn lower_assignment(
        &mut self,
        assignment: &asg::Assignment,
        circuit: &mut Circuit,
    ) -> Result<(), Qasm3ParseError> {
        match assignment.lvalue() {
            LValue::Identifier(target_id_result) => {
                let target_id = self.symbol_id(target_id_result)?;
                let Some(target) = self.classical.get(&target_id).copied() else {
                    return Err(Qasm3ParseError::UndefinedSymbol(
                        self.symbol_name(&target_id),
                    ));
                };

                if let Expr::MeasureExpression(measure) = assignment.rvalue().expression() {
                    let qubits = self.expand_qubit_expr(measure.operand())?;
                    if qubits.len() == 1 {
                        circuit.measure_into(qubits[0], target)?;
                    } else {
                        circuit.measure_bits_into(qubits, target)?;
                    }
                    return Ok(());
                }

                let value = self.lower_classical_expr(assignment.rvalue())?;
                circuit.store(target, value)?;
                Ok(())
            }
            LValue::IndexedIdentifier(indexed) => {
                self.lower_indexed_assignment(indexed, assignment.rvalue(), circuit)
            }
        }
    }

    fn lower_indexed_assignment(
        &mut self,
        indexed: &asg::IndexedIdentifier,
        rvalue: &TExpr,
        circuit: &mut Circuit,
    ) -> Result<(), Qasm3ParseError> {
        let target = self.indexed_classical_target(indexed)?;
        let bit = match rvalue.expression() {
            Expr::MeasureExpression(measure) => {
                let qubits = self.expand_qubit_expr(measure.operand())?;
                if qubits.len() != 1 {
                    return Err(Qasm3ParseError::MismatchedQubitCount {
                        expected: 1,
                        actual: qubits.len(),
                    });
                }
                circuit.measure(qubits[0])?.expr()
            }
            _ => self.lower_classical_expr(rvalue)?,
        };
        self.store_indexed_classical_bit(target, bit, circuit)
    }

    fn indexed_classical_target(
        &self,
        indexed: &asg::IndexedIdentifier,
    ) -> Result<(ClassicalVar, u32), Qasm3ParseError> {
        let id = self.symbol_id(indexed.identifier())?;
        let Some(var) = self.classical.get(&id).copied() else {
            return Err(Qasm3ParseError::UndefinedSymbol(self.symbol_name(&id)));
        };
        if indexed.indexes().len() != 1 {
            return Err(Qasm3ParseError::UnsupportedFeature(
                "multi-dimensional classical assignment index".to_string(),
            ));
        }
        let ClassicalType::BitVec(width) = var.ty() else {
            return Err(Qasm3ParseError::TypeError(format!(
                "indexed assignment target '{}' must be bit array, got {:?}",
                self.symbol_name(&id),
                var.ty()
            )));
        };
        let index = self.single_index(&indexed.indexes()[0])?;
        if index >= width.get() {
            return Err(Qasm3ParseError::InvalidArgument(format!(
                "classical index {index} out of bounds for '{}'",
                self.symbol_name(&id)
            )));
        }
        Ok((var, index))
    }

    fn store_indexed_classical_bit(
        &self,
        (target, index): (ClassicalVar, u32),
        bit: ClassicalExpr,
        circuit: &mut Circuit,
    ) -> Result<(), Qasm3ParseError> {
        if bit.ty() != ClassicalType::Bit {
            return Err(Qasm3ParseError::TypeError(format!(
                "indexed bit assignment expects Bit expression, got {:?}",
                bit.ty()
            )));
        }
        let width = match target.ty() {
            ClassicalType::BitVec(width) => width.get(),
            ty => {
                return Err(Qasm3ParseError::TypeError(format!(
                    "indexed assignment target must be BitVec, got {ty:?}"
                )));
            }
        };
        let current = target.expr();
        let bits = (0..width)
            .map(|bit_index| {
                if bit_index == index {
                    Ok(bit.clone())
                } else {
                    ClassicalExpr::extract_bit(current.clone(), bit_index)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let updated = ClassicalExpr::pack_bits(bits)?;
        circuit.store(target, updated)?;
        Ok(())
    }

    fn lower_gate_call(
        &mut self,
        call: &asg::GateCall,
        circuit: &mut Circuit,
    ) -> Result<(), Qasm3ParseError> {
        let gate_id = self.symbol_id(call.name())?;
        let gate_name = self.symbol_name(&gate_id);
        let params = self.lower_params(call.params().unwrap_or(&[]))?;
        let mut qubits = self.expand_qubit_exprs(call.qubits())?;

        if let Some((instruction, params)) =
            self.standard_gate(&gate_name, &params, qubits.len())?
        {
            let instruction = self.apply_modifiers(instruction, call.modifiers())?;
            let (expected_qubits, expected_params) = instruction.gate_arity().ok_or_else(|| {
                Qasm3ParseError::UnsupportedFeature(format!(
                    "gate '{gate_name}' has variable arity"
                ))
            })?;
            self.check_counts(expected_qubits, qubits.len(), expected_params, params.len())?;
            circuit.append(instruction, qubits, params, None)?;
            return Ok(());
        }

        let gate = self.compile_gate_if_needed(&gate_id)?;
        match gate {
            GateDef::Circuit(circuit_gate) => {
                self.check_counts(
                    circuit_gate.num_qubits(),
                    qubits.len(),
                    circuit_gate.num_params(),
                    params.len(),
                )?;
                if !call.modifiers().is_empty() {
                    return Err(Qasm3ParseError::UnsupportedFeature(
                        "modifiers on circuit-defined gates".to_string(),
                    ));
                }
                circuit.append(
                    Instruction::CircuitGate(Box::new(circuit_gate)),
                    qubits,
                    params,
                    None,
                )?;
            }
            GateDef::Opaque {
                name,
                qubits: expected_qubits,
                params: expected_params,
            } => {
                self.check_counts(expected_qubits, qubits.len(), expected_params, params.len())?;
                let mut instruction = Instruction::UnitaryGate(Box::new(UnitaryGate::new(
                    &name,
                    expected_qubits as u16,
                    expected_params as u16,
                )));
                instruction = self.apply_modifiers(instruction, call.modifiers())?;
                let expected = instruction.gate_arity().ok_or_else(|| {
                    Qasm3ParseError::UnsupportedFeature(format!("opaque gate '{name}' arity"))
                })?;
                if qubits.len() < expected.0 {
                    return Err(Qasm3ParseError::MismatchedQubitCount {
                        expected: expected.0,
                        actual: qubits.len(),
                    });
                }
                circuit.append(instruction, qubits.split_off(0), params, None)?;
            }
            GateDef::Source(_) => unreachable!("compile_gate_if_needed resolves source gates"),
        }
        Ok(())
    }

    fn compile_gate_if_needed(&mut self, id: &SymbolId) -> Result<GateDef, Qasm3ParseError> {
        let Some(gate) = self.gates.get(id).cloned() else {
            return Err(Qasm3ParseError::UndefinedGate(self.symbol_name(id)));
        };
        if !matches!(gate, GateDef::Source(_)) {
            return Ok(gate);
        }
        if self.recursion_depth >= self.max_recursion_depth {
            return Err(Qasm3ParseError::RecursionLimitExceeded(
                self.symbol_name(id),
            ));
        }
        if !self.compiling_gates.insert(id.clone()) {
            return Err(Qasm3ParseError::CircularGateDependency {
                gate: self.symbol_name(id),
                dependency: self.symbol_name(id),
            });
        }

        self.recursion_depth += 1;
        let result = self.build_circuit_gate(id);
        self.recursion_depth -= 1;
        self.compiling_gates.remove(id);

        let compiled = result?;
        self.gates.insert(id.clone(), compiled.clone());
        Ok(compiled)
    }

    fn build_circuit_gate(&mut self, id: &SymbolId) -> Result<GateDef, Qasm3ParseError> {
        let GateDef::Source(def) = self
            .gates
            .get(id)
            .cloned()
            .ok_or_else(|| Qasm3ParseError::UndefinedGate(format!("gate id {:?}", id)))?
        else {
            return self
                .gates
                .get(id)
                .cloned()
                .ok_or_else(|| Qasm3ParseError::UndefinedGate(format!("gate id {:?}", id)));
        };

        let saved_quantum = self.quantum.clone();
        let mut gate_circuit = Circuit::new(def.qubits().len());
        for (index, qubit_id_result) in def.qubits().iter().enumerate() {
            let qubit_id = self.symbol_id(qubit_id_result)?;
            self.quantum
                .insert(qubit_id, QuantumBinding::Qubit(Qubit::new(index as u32)));
        }

        for stmt in def.block().statements() {
            match stmt {
                Stmt::GateCall(_) | Stmt::GPhaseCall(_) | Stmt::Barrier(_) => {
                    self.lower_stmt(stmt, &mut gate_circuit)?;
                }
                _ => {
                    self.quantum = saved_quantum;
                    return Err(Qasm3ParseError::UnsupportedFeature(format!(
                        "statement in gate body: {stmt:?}"
                    )));
                }
            }
        }

        self.quantum = saved_quantum;
        let signature_params = def
            .params()
            .unwrap_or(&[])
            .iter()
            .map(|param| self.symbol_id(param).map(|id| self.symbol_name(&id)))
            .collect::<Result<Vec<_>, _>>()?;
        let gate = CircuitGate::with_signature(
            self.symbol_name(id),
            FrozenCircuit::new(gate_circuit),
            signature_params,
        )?;
        Ok(GateDef::Circuit(gate))
    }

    fn lower_for(
        &mut self,
        op: &asg::ForStmt,
        circuit: &mut Circuit,
    ) -> Result<(), Qasm3ParseError> {
        let loop_id = self.symbol_id(op.loop_var())?;
        let ForIterable::RangeExpression(range) = op.iterable() else {
            return Err(Qasm3ParseError::UnsupportedFeature(
                "non-range for iterable".to_string(),
            ));
        };
        let start = self.const_u128(range.start())?;
        let stop = self.const_u128(range.stop())?;
        let step = match range.step() {
            Some(step) => self.const_u128(step)?,
            None => 1,
        };
        if step == 0 {
            return Err(Qasm3ParseError::InvalidArgument(
                "for loop step must be non-zero".to_string(),
            ));
        }
        let old = self.loop_constants.get(&loop_id).copied();
        let mut value = start;
        while value <= stop {
            self.loop_constants.insert(loop_id.clone(), value);
            self.lower_block(op.loop_body(), circuit)?;
            value = value.checked_add(step).ok_or_else(|| {
                Qasm3ParseError::InvalidArgument("for loop value overflow".to_string())
            })?;
        }
        match old {
            Some(value) => {
                self.loop_constants.insert(loop_id, value);
            }
            None => {
                self.loop_constants.remove(&loop_id);
            }
        }
        Ok(())
    }

    fn lower_switch(
        &mut self,
        op: &asg::SwitchCaseStmt,
        circuit: &mut Circuit,
    ) -> Result<(), Qasm3ParseError> {
        let target = self.lower_classical_expr(op.control())?;
        let target = match target.ty() {
            ClassicalType::UInt(_) => target,
            ClassicalType::BitVec(_) => ClassicalExpr::bit_vec_to_uint(target)?,
            ty => {
                return Err(Qasm3ParseError::TypeError(format!(
                    "switch target must be UInt or BitVec, got {ty:?}"
                )));
            }
        };

        circuit.switch(target, |builder| {
            for case in op.cases() {
                for value_expr in case.control_values() {
                    let value = self.const_u128(value_expr).map_err(|e| {
                        crate::circuit::CircuitError::InvalidOperation(e.to_string())
                    })?;
                    builder.value(value, |case_circuit| {
                        self.lower_stmt_slice(case.statements(), case_circuit)
                            .map_err(|e| {
                                crate::circuit::CircuitError::InvalidOperation(e.to_string())
                            })
                    })?;
                }
            }
            if let Some(default_block) = op.default_block() {
                builder.default(|default_circuit| {
                    self.lower_stmt_slice(default_block, default_circuit)
                        .map_err(|e| crate::circuit::CircuitError::InvalidOperation(e.to_string()))
                })?;
            }
            Ok(())
        })?;
        Ok(())
    }

    fn standard_gate(
        &self,
        name: &str,
        params: &[ParameterValue],
        qubit_count: usize,
    ) -> Result<Option<(Instruction, Vec<ParameterValue>)>, Qasm3ParseError> {
        let instruction = match name {
            "id" | "i" => Some(Instruction::Standard(StandardGate::I)),
            "x" => Some(Instruction::Standard(StandardGate::X)),
            "y" => Some(Instruction::Standard(StandardGate::Y)),
            "z" => Some(Instruction::Standard(StandardGate::Z)),
            "h" => Some(Instruction::Standard(StandardGate::H)),
            "s" => Some(Instruction::Standard(StandardGate::S)),
            "sdg" => Some(Instruction::Standard(StandardGate::SDG)),
            "t" => Some(Instruction::Standard(StandardGate::T)),
            "tdg" => Some(Instruction::Standard(StandardGate::TDG)),
            "x2p" => Some(Instruction::Standard(StandardGate::X2P)),
            "x2m" => Some(Instruction::Standard(StandardGate::X2M)),
            "y2p" => Some(Instruction::Standard(StandardGate::Y2P)),
            "y2m" => Some(Instruction::Standard(StandardGate::Y2M)),
            "sx" => Some(Instruction::Standard(StandardGate::X2P)),
            "sxdg" => Some(Instruction::Standard(StandardGate::X2M)),
            "rx" => Some(Instruction::Standard(StandardGate::RX)),
            "ry" => Some(Instruction::Standard(StandardGate::RY)),
            "rz" => Some(Instruction::Standard(StandardGate::RZ)),
            "p" | "phase" | "u1" => Some(Instruction::Standard(StandardGate::Phase)),
            "u" | "U" | "u3" => Some(Instruction::Standard(StandardGate::U)),
            "u2" => {
                if params.len() != 2 {
                    return Err(Qasm3ParseError::MismatchedParameterCount {
                        expected: 2,
                        actual: params.len(),
                    });
                }
                let mut new_params = vec![ParameterValue::Param(Parameter::pi() / 2.0)];
                new_params.extend_from_slice(params);
                return Ok(Some((Instruction::Standard(StandardGate::U), new_params)));
            }
            "cx" | "CX" => Some(Instruction::Standard(StandardGate::CX)),
            "cy" => Some(Instruction::Standard(StandardGate::CY)),
            "cz" => Some(Instruction::Standard(StandardGate::CZ)),
            "swap" => Some(Instruction::Standard(StandardGate::SWAP)),
            "ccx" => Some(Instruction::Standard(StandardGate::CCX)),
            "rxx" => Some(Instruction::Standard(StandardGate::RXX)),
            "ryy" => Some(Instruction::Standard(StandardGate::RYY)),
            "rzz" => Some(Instruction::Standard(StandardGate::RZZ)),
            "rzx" => Some(Instruction::Standard(StandardGate::RZX)),
            "xy2p" => Some(Instruction::Standard(StandardGate::XY2P)),
            "xy2m" => Some(Instruction::Standard(StandardGate::XY2M)),
            "fsim" => Some(Instruction::Standard(StandardGate::FSIM)),
            "crx" => Some(Instruction::Standard(StandardGate::CRX)),
            "cry" => Some(Instruction::Standard(StandardGate::CRY)),
            "crz" => Some(Instruction::Standard(StandardGate::CRZ)),
            "gphase" => Some(Instruction::Standard(StandardGate::GPhase)),
            "ch" | "cp" | "cu" | "cswap" => {
                return Err(Qasm3ParseError::UnsupportedFeature(format!(
                    "standard gate '{name}'"
                )));
            }
            _ => None,
        };
        if let Some(instruction) = instruction {
            let (expected_qubits, expected_params) = instruction.gate_arity().ok_or_else(|| {
                Qasm3ParseError::UnsupportedFeature(format!("gate '{name}' arity"))
            })?;
            self.check_counts(expected_qubits, qubit_count, expected_params, params.len())?;
            Ok(Some((instruction, params.to_vec())))
        } else {
            Ok(None)
        }
    }

    fn apply_modifiers(
        &self,
        mut instruction: Instruction,
        modifiers: &[GateModifier],
    ) -> Result<Instruction, Qasm3ParseError> {
        for modifier in modifiers.iter().rev() {
            match modifier {
                GateModifier::Inv => {
                    let params = Vec::<Parameter>::new();
                    let Some((inverse, _)) = instruction.inverse(&params) else {
                        return Err(Qasm3ParseError::UnsupportedFeature(
                            "inverse modifier for this gate".to_string(),
                        ));
                    };
                    instruction = inverse;
                }
                GateModifier::Ctrl(count) => {
                    let count = match count {
                        Some(expr) => self.const_u128(expr)? as usize,
                        None => 1,
                    };
                    let Some(controlled) = instruction.control(count) else {
                        return Err(Qasm3ParseError::UnsupportedFeature(
                            "control modifier for this gate".to_string(),
                        ));
                    };
                    instruction = controlled;
                }
                GateModifier::NegCtrl(_) | GateModifier::Pow(_) => {
                    return Err(Qasm3ParseError::UnsupportedFeature(
                        "negctrl or pow gate modifier".to_string(),
                    ));
                }
            }
        }
        Ok(instruction)
    }

    fn check_counts(
        &self,
        expected_qubits: usize,
        actual_qubits: usize,
        expected_params: usize,
        actual_params: usize,
    ) -> Result<(), Qasm3ParseError> {
        if expected_qubits != actual_qubits {
            return Err(Qasm3ParseError::MismatchedQubitCount {
                expected: expected_qubits,
                actual: actual_qubits,
            });
        }
        if expected_params != actual_params {
            return Err(Qasm3ParseError::MismatchedParameterCount {
                expected: expected_params,
                actual: actual_params,
            });
        }
        Ok(())
    }

    fn lower_params(&self, params: &[TExpr]) -> Result<Vec<ParameterValue>, Qasm3ParseError> {
        params
            .iter()
            .map(|expr| {
                let param = self.lower_angle(expr)?;
                if let Ok(value) = param.evaluate(&None) {
                    Ok(ParameterValue::Fixed(value))
                } else {
                    Ok(ParameterValue::Param(param))
                }
            })
            .collect()
    }

    fn lower_angle(&self, expr: &TExpr) -> Result<Parameter, Qasm3ParseError> {
        match expr.expression() {
            Expr::Literal(Literal::Int(value)) => {
                if !*value.sign() {
                    return Err(Qasm3ParseError::InvalidArgument(
                        "negative integer literal".to_string(),
                    ));
                }
                Ok(Parameter::from(*value.value() as f64))
            }
            Expr::Literal(Literal::Float(value)) => value
                .value()
                .parse::<f64>()
                .map(Parameter::from)
                .map_err(|e| Qasm3ParseError::InvalidArgument(e.to_string())),
            Expr::Identifier(id_result) => {
                let id = self.symbol_id(id_result)?;
                let name = self.symbol_name(&id);
                match name.as_str() {
                    "pi" | "π" => Ok(Parameter::pi()),
                    "tau" | "τ" => Ok(Parameter::pi() * 2.0),
                    "euler" | "ℇ" => Ok(Parameter::symbol("e")),
                    _ => Ok(Parameter::symbol(&name)),
                }
            }
            Expr::Cast(cast) => self.lower_angle(cast.operand()),
            Expr::UnaryExpr(unary) => {
                let value = self.lower_angle(unary.operand())?;
                match unary.op() {
                    UnaryOp::Minus => Ok(Parameter::from(0.0) - value),
                    _ => Err(Qasm3ParseError::UnsupportedFeature(format!(
                        "angle unary operator {:?}",
                        unary.op()
                    ))),
                }
            }
            Expr::BinaryExpr(binary) => {
                let left = self.lower_angle(binary.left())?;
                let right = self.lower_angle(binary.right())?;
                match binary.op() {
                    BinaryOp::ArithOp(ArithOp::Add) => Ok(left + right),
                    BinaryOp::ArithOp(ArithOp::Sub) => Ok(left - right),
                    BinaryOp::ArithOp(ArithOp::Mul) => Ok(left * right),
                    BinaryOp::ArithOp(ArithOp::Div) => Ok(left / right),
                    _ => Err(Qasm3ParseError::UnsupportedFeature(format!(
                        "angle binary operator {:?}",
                        binary.op()
                    ))),
                }
            }
            Expr::Call => Err(Qasm3ParseError::UnsupportedFeature(
                "function call in angle expression".to_string(),
            )),
            _ => Err(Qasm3ParseError::UnsupportedFeature(format!(
                "angle expression {:?}",
                expr.expression()
            ))),
        }
    }

    fn lower_condition(&self, expr: &TExpr) -> Result<ClassicalExpr, Qasm3ParseError> {
        let expr = self.lower_classical_expr(expr)?;
        match expr.ty() {
            ClassicalType::Bool => Ok(expr),
            ClassicalType::Bit => Ok(ClassicalExpr::bit_to_bool(expr)?),
            ty => Err(Qasm3ParseError::TypeError(format!(
                "condition must be Bool or Bit, got {ty:?}"
            ))),
        }
    }

    fn lower_classical_expr(&self, expr: &TExpr) -> Result<ClassicalExpr, Qasm3ParseError> {
        match expr.expression() {
            Expr::Literal(Literal::Bool(value)) => Ok(ClassicalExpr::bool_literal(*value.value())),
            Expr::Literal(Literal::Int(value)) => {
                if !*value.sign() {
                    return Err(Qasm3ParseError::UnsupportedFeature(
                        "signed integer classical literal".to_string(),
                    ));
                }
                let width = expr.get_type().width().unwrap_or(128);
                Ok(ClassicalExpr::uint_literal(width, *value.value())?)
            }
            Expr::Literal(Literal::BitString(bits)) => {
                let clean = bits
                    .value()
                    .chars()
                    .filter(|ch| *ch == '0' || *ch == '1')
                    .collect::<String>();
                let value = u128::from_str_radix(&clean, 2)
                    .map_err(|e| Qasm3ParseError::InvalidArgument(e.to_string()))?;
                Ok(ClassicalExpr::bit_vec_literal(clean.len() as u32, value)?)
            }
            Expr::Identifier(id_result) => {
                let id = self.symbol_id(id_result)?;
                if let Some(value) = self.loop_constants.get(&id) {
                    let width = self.symbol_type(&id).width().unwrap_or(128);
                    return Ok(ClassicalExpr::uint_literal(width, *value)?);
                }
                let Some(var) = self.classical.get(&id).copied() else {
                    return Err(Qasm3ParseError::UndefinedSymbol(self.symbol_name(&id)));
                };
                Ok(var.expr())
            }
            Expr::Cast(cast) => {
                if matches!(cast.get_type(), Type::Bit(_))
                    && let Expr::Literal(Literal::Bool(value)) = cast.operand().expression()
                {
                    return Ok(ClassicalExpr::bit_literal(*value.value()));
                }
                if let Type::UInt(Some(width), _) = cast.get_type()
                    && let Ok(value) = self.const_u128(cast.operand())
                {
                    return Ok(ClassicalExpr::uint_literal(*width, value)?);
                }
                let operand = self.lower_classical_expr(cast.operand())?;
                match (operand.ty(), cast.get_type()) {
                    (ClassicalType::Bit, Type::Bool(_)) => Ok(ClassicalExpr::bit_to_bool(operand)?),
                    (ClassicalType::BitVec(_), Type::UInt(_, _)) => {
                        Ok(ClassicalExpr::bit_vec_to_uint(operand)?)
                    }
                    (_, _) if self.classical_type(cast.get_type()).ok() == Some(operand.ty()) => {
                        Ok(operand)
                    }
                    _ => Err(Qasm3ParseError::UnsupportedFeature(format!(
                        "classical cast from {:?} to {:?}",
                        operand.ty(),
                        cast.get_type()
                    ))),
                }
            }
            Expr::UnaryExpr(unary) => {
                let operand = self.lower_classical_expr(unary.operand())?;
                match unary.op() {
                    UnaryOp::Not | UnaryOp::BitNot => Ok(ClassicalExpr::try_not(operand)?),
                    UnaryOp::Minus => Err(Qasm3ParseError::UnsupportedFeature(
                        "runtime unary minus".to_string(),
                    )),
                }
            }
            Expr::BinaryExpr(binary) => {
                let left = self.lower_classical_expr(binary.left())?;
                let right = self.lower_classical_expr(binary.right())?;
                match binary.op() {
                    BinaryOp::CmpOp(CmpOp::Eq) => Ok(ClassicalExpr::eq(left, right)?),
                    BinaryOp::CmpOp(CmpOp::Neq) => Ok(ClassicalExpr::ne(left, right)?),
                    BinaryOp::ArithOp(ArithOp::BitAnd) => Ok(ClassicalExpr::try_and(left, right)?),
                    BinaryOp::ArithOp(ArithOp::BitXOr) => Ok(ClassicalExpr::try_xor(left, right)?),
                    _ => Err(Qasm3ParseError::UnsupportedFeature(format!(
                        "runtime classical binary operator {:?}",
                        binary.op()
                    ))),
                }
            }
            Expr::IndexedIdentifier(indexed) => {
                let base = self.lower_classical_indexed_base(indexed)?;
                if indexed.indexes().len() != 1 {
                    return Err(Qasm3ParseError::UnsupportedFeature(
                        "multi-dimensional classical index".to_string(),
                    ));
                }
                let index = self.single_index(&indexed.indexes()[0])?;
                Ok(ClassicalExpr::extract_bit(base, index)?)
            }
            _ => Err(Qasm3ParseError::UnsupportedFeature(format!(
                "classical expression {:?}",
                expr.expression()
            ))),
        }
    }

    fn lower_classical_indexed_base(
        &self,
        indexed: &asg::IndexedIdentifier,
    ) -> Result<ClassicalExpr, Qasm3ParseError> {
        let id = self.symbol_id(indexed.identifier())?;
        let Some(var) = self.classical.get(&id).copied() else {
            return Err(Qasm3ParseError::UndefinedSymbol(self.symbol_name(&id)));
        };
        Ok(var.expr())
    }

    fn expand_qubit_exprs(&self, exprs: &[TExpr]) -> Result<Vec<Qubit>, Qasm3ParseError> {
        let mut out = Vec::new();
        for expr in exprs {
            out.extend(self.expand_qubit_expr(expr)?);
        }
        Ok(out)
    }

    fn expand_qubit_expr(&self, expr: &TExpr) -> Result<Vec<Qubit>, Qasm3ParseError> {
        match expr.expression() {
            Expr::GateOperand(GateOperand::Identifier(id_result)) | Expr::Identifier(id_result) => {
                let id = self.symbol_id(id_result)?;
                self.quantum
                    .get(&id)
                    .map(QuantumBinding::qubits)
                    .ok_or_else(|| Qasm3ParseError::UndefinedSymbol(self.symbol_name(&id)))
            }
            Expr::GateOperand(GateOperand::IndexedIdentifier(indexed))
            | Expr::IndexedIdentifier(indexed) => {
                self.resolve_indexed_qubit(indexed).map(|q| vec![q])
            }
            Expr::GateOperand(GateOperand::HardwareQubit(qubit)) | Expr::HardwareQubit(qubit) => {
                self.lower_hardware_qubit(qubit).map(|q| vec![q])
            }
            Expr::Cast(cast) => self.expand_qubit_expr(cast.operand()),
            _ => Err(Qasm3ParseError::TypeError(format!(
                "expected qubit operand, got {:?}",
                expr.expression()
            ))),
        }
    }

    fn lower_hardware_qubit(
        &self,
        hardware_qubit: &asg::HardwareQubit,
    ) -> Result<Qubit, Qasm3ParseError> {
        let identifier = hardware_qubit
            .identifier()
            .strip_prefix('$')
            .unwrap_or_else(|| hardware_qubit.identifier());
        let index = identifier.parse::<u32>().map_err(|_| {
            Qasm3ParseError::InvalidArgument(format!(
                "invalid physical qubit identifier '{}'",
                hardware_qubit.identifier()
            ))
        })?;
        let qubit = Qubit::new(index);
        if self.hardware_qubits.contains(&qubit) {
            Ok(qubit)
        } else {
            Err(Qasm3ParseError::InvalidArgument(format!(
                "unregistered physical qubit identifier '{}'",
                hardware_qubit.identifier()
            )))
        }
    }

    fn resolve_indexed_qubit(
        &self,
        indexed: &asg::IndexedIdentifier,
    ) -> Result<Qubit, Qasm3ParseError> {
        let id = self.symbol_id(indexed.identifier())?;
        if indexed.indexes().len() != 1 {
            return Err(Qasm3ParseError::UnsupportedFeature(
                "multi-dimensional qubit index".to_string(),
            ));
        }
        let index = self.single_index(&indexed.indexes()[0])? as usize;
        match self.quantum.get(&id) {
            Some(QuantumBinding::Qubit(q)) if index == 0 => Ok(*q),
            Some(QuantumBinding::Register(qs)) => qs.get(index).copied().ok_or_else(|| {
                Qasm3ParseError::InvalidArgument(format!(
                    "qubit index {index} out of bounds for '{}'",
                    self.symbol_name(&id)
                ))
            }),
            Some(QuantumBinding::Qubit(_)) => Err(Qasm3ParseError::InvalidArgument(format!(
                "qubit index {index} out of bounds for '{}'",
                self.symbol_name(&id)
            ))),
            None => Err(Qasm3ParseError::UndefinedSymbol(self.symbol_name(&id))),
        }
    }

    fn single_index(&self, index: &IndexOperator) -> Result<u32, Qasm3ParseError> {
        match index {
            IndexOperator::ExpressionList(list) if list.expressions.len() == 1 => {
                let value = self.const_u128(&list.expressions[0])?;
                u32::try_from(value).map_err(|_| {
                    Qasm3ParseError::InvalidArgument(format!("index {value} exceeds u32"))
                })
            }
            _ => Err(Qasm3ParseError::UnsupportedFeature(
                "non-scalar index".to_string(),
            )),
        }
    }

    fn const_u128(&self, expr: &TExpr) -> Result<u128, Qasm3ParseError> {
        match expr.expression() {
            Expr::Literal(Literal::Int(value)) if *value.sign() => Ok(*value.value()),
            Expr::Cast(cast) => self.const_u128(cast.operand()),
            Expr::Identifier(id_result) => {
                let id = self.symbol_id(id_result)?;
                self.loop_constants
                    .get(&id)
                    .copied()
                    .ok_or_else(|| Qasm3ParseError::InvalidArgument(self.symbol_name(&id)))
            }
            Expr::BinaryExpr(binary) => {
                let left = self.const_u128(binary.left())?;
                let right = self.const_u128(binary.right())?;
                match binary.op() {
                    BinaryOp::ArithOp(ArithOp::Add) => left.checked_add(right).ok_or_else(|| {
                        Qasm3ParseError::InvalidArgument("constant add overflow".to_string())
                    }),
                    BinaryOp::ArithOp(ArithOp::Sub) => left.checked_sub(right).ok_or_else(|| {
                        Qasm3ParseError::InvalidArgument("constant sub underflow".to_string())
                    }),
                    BinaryOp::ArithOp(ArithOp::Mul) => left.checked_mul(right).ok_or_else(|| {
                        Qasm3ParseError::InvalidArgument("constant mul overflow".to_string())
                    }),
                    BinaryOp::ArithOp(ArithOp::Div) => {
                        if right == 0 {
                            Err(Qasm3ParseError::InvalidArgument(
                                "division by zero".to_string(),
                            ))
                        } else {
                            Ok(left / right)
                        }
                    }
                    _ => Err(Qasm3ParseError::UnsupportedFeature(format!(
                        "constant operator {:?}",
                        binary.op()
                    ))),
                }
            }
            _ => Err(Qasm3ParseError::InvalidArgument(format!(
                "expected unsigned integer constant, got {:?}",
                expr.expression()
            ))),
        }
    }

    fn classical_type(&self, ty: &Type) -> Result<ClassicalType, Qasm3ParseError> {
        match ty {
            Type::Bit(_) => Ok(ClassicalType::Bit),
            Type::Bool(_) => Ok(ClassicalType::Bool),
            Type::UInt(Some(width), _) => ClassicalType::uint(*width).ok_or_else(|| {
                Qasm3ParseError::InvalidArgument("uint width must be non-zero".to_string())
            }),
            Type::BitArray(ArrayDims::D1(width), _) => ClassicalType::bit_vec(*width as u32)
                .ok_or_else(|| {
                    Qasm3ParseError::InvalidArgument("bit array width must be non-zero".to_string())
                }),
            _ => Err(Qasm3ParseError::UnsupportedFeature(format!(
                "classical type {ty:?}"
            ))),
        }
    }

    fn symbol_id(&self, result: &SymbolIdResult) -> Result<SymbolId, Qasm3ParseError> {
        result
            .clone()
            .map_err(|err| Qasm3ParseError::UndefinedSymbol(format!("{err:?}")))
    }

    fn symbol_name(&self, id: &SymbolId) -> String {
        self.symbols[id].name().to_string()
    }

    fn symbol_type(&self, id: &SymbolId) -> &Type {
        self.symbols[id].symbol_type()
    }
}

#[cfg(test)]
#[path = "./load_test.rs"]
mod load_test;
