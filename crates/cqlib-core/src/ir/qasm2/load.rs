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

//! OpenQASM 2.0 Parser Module
//!
//! This module provides functionality to parse OpenQASM 2.0 quantum programs
//! and convert them into the internal `Circuit` representation.
//!
//! ## Features
//!
//! - **Full OpenQASM 2.0 Support**: Parses all standard statements including quantum registers,
//!   classical registers, gates, measurements, barriers, resets, and conditional execution
//! - **Custom Gate Compilation**: Compiles user-defined gates into internal representations
//! - **Include Processing**: Handles external includes and built-in `qelib1.inc`
//! - **Expression Evaluation**: Evaluates mathematical expressions in gate parameters
//! - **Error Handling**: Provides detailed error messages with line numbers and context
//!
//! ## Architecture
//!
//! The parser works in multiple phases:
//! 1. **Discovery Pass**: Collects register declarations, gate definitions, and processes includes
//! 2. **Compilation Phase**: Compiles custom gates into internal `CircuitGate` representations
//! 3. **Generation Pass**: Converts AST statements into circuit operations
//!
//! ## Example
//!
//! ```rust
//! use cqlib_core::ir::qasm2::load::loads;
//!
//! let qasm = r#"
//!     OPENQASM 2.0;
//!     include "qelib1.inc";
//!     qreg q[2];
//!     creg c[2];
//!     h q[0];
//!     cx q[0], q[1];
//!     measure q[0] -> c[0];
//!     measure q[1] -> c[1];
//! "#;
//!
//! let circuit = loads(qasm).unwrap();
//! assert_eq!(circuit.num_qubits(), 2);
//! ```
//!
//! ## Gate Compilation
//!
//! Custom gates are collected during discovery and compiled before operation
//! generation so invalid or circular definitions fail deterministically:
//!
//! ```rust
//! use cqlib_core::ir::qasm2::load::loads;
//!
//! let qasm = r#"
//!     OPENQASM 2.0;
//!     qreg q[1];
//!     gate my_gate q { h q; }
//!     my_gate q;
//! "#;
//!
//! let circuit = loads(qasm).unwrap();
//! ```

use crate::circuit::bit::Qubit;
use crate::circuit::circuit_param::ParameterValue;
use crate::circuit::gate::circuit_gate::{CircuitGate, FrozenCircuit};
use crate::circuit::gate::{Directive, Instruction, StandardGate};
use crate::circuit::{Circuit, CircuitError, ClassicalExpr, ClassicalType, ClassicalVar};

use crate::circuit::parameter::Parameter;
use crate::ir::qasm2::ast::{
    Argument, Expression, OpCode, OpenQASMProgram, Statement, UnaryOpCode,
};
use smallvec::{SmallVec, smallvec};
use std::collections::{HashMap, HashSet};
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Component, Path, PathBuf};

/// Trait for abstracting file system access during OpenQASM parsing.
///
/// This allows the parser to work in different environments:
/// - Real file system (use `FileSystemResolver`)
/// - Memory-only mode (use `NullResolver`)
/// - Custom resolvers for WASM or embedded files
pub trait QasmSourceResolver {
    /// Resolve the content of a file given its path.
    ///
    /// # Arguments
    /// * `path` - The file path to resolve
    ///
    /// # Returns
    /// * `Ok(String)` - The file contents
    /// * `Err(io::Error)` - I/O error if resolution fails
    fn resolve_source(&self, path: &Path) -> Result<String, io::Error>;
}

/// Default resolver that uses the real file system.
pub struct FileSystemResolver;

impl QasmSourceResolver for FileSystemResolver {
    fn resolve_source(&self, path: &Path) -> Result<String, io::Error> {
        std::fs::read_to_string(path)
    }
}

/// A null resolver that fails on any include. Used when loading raw strings without context.
pub struct NullResolver;

impl QasmSourceResolver for NullResolver {
    fn resolve_source(&self, path: &Path) -> Result<String, io::Error> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("Cannot include files in raw string mode (path: {:?})", path),
        ))
    }
}

const QELIB1_DIRECT_GATES: &[&str] = &[
    "u3", "u2", "u1", "cx", "id", "x", "y", "z", "h", "s", "sdg", "t", "tdg", "rx", "ry", "rz",
    "cz", "cy", "ccx", "crz",
];

const QELIB1_CUSTOM_GATES: &[&str] = &["ch", "cu1", "cu3"];

#[allow(dead_code)]
const CQLIB_BUILTIN_EXTENSION_GATES: &[&str] = &[
    "p", "sx", "sxdg", "rxx", "ryy", "rzz", "rzx", "fsim", "crx", "cry", "swap", "toffoli",
];

/// Gate names declared by OpenQASM 2.0 qelib1.inc.
///
/// Cqlib treats qelib1 gates as compatibility builtins: user sources may call
/// them without an explicit include, and may not redefine them. OpenQASM
/// identifiers are case-sensitive, so this list uses the exact lowercase
/// spellings present in qelib1.inc.
const QELIB1_RESERVED_GATE_NAMES: &[&str] = &[
    "u3", "u2", "u1", "cx", "id", "x", "y", "z", "h", "s", "sdg", "t", "tdg", "rx", "ry", "rz",
    "cz", "cy", "ch", "ccx", "crz", "cu1", "cu3",
];

#[rustfmt::skip]
mod parser {
    include!(concat!(env!("OUT_DIR"), "/ir/qasm2/parser.rs"));
}

/// Parse OpenQASM 2.0 file and convert to Circuit
pub fn load<P: AsRef<Path>>(path: P) -> Result<Circuit, QasmParseError> {
    let path = path.as_ref();

    // Use FileSystemResolver for file-based loading
    let resolver = Box::new(FileSystemResolver);

    // Get content using the resolver
    let content = resolver
        .resolve_source(path)
        .map_err(QasmParseError::IoError)?;

    // Pass the parent directory as the base path for includes
    let base_path = path.parent().map(|p| p.to_path_buf());

    parse_qasm_file_with_context(&content, normalize_path(path), base_path, resolver)
}

/// Parse an OpenQASM 2.0 file and convert it to a [`Circuit`].
///
/// Rust-style alias for [`load`]. The Python-style `load` name is retained for
/// compatibility with the rest of the IR module API.
pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Circuit, QasmParseError> {
    load(path)
}

/// Parse OpenQASM 2.0 string and convert to Circuit
pub fn loads(source: &str) -> Result<Circuit, QasmParseError> {
    // Use NullResolver for string-based loading (no file includes allowed)
    parse_qasm_with_context(source, None, Box::new(NullResolver))
}

/// Parse an OpenQASM 2.0 source string and convert it to a [`Circuit`].
///
/// Rust-style alias for [`loads`].
pub fn from_str(source: &str) -> Result<Circuit, QasmParseError> {
    loads(source)
}

fn parse_qasm_with_context(
    source: &str,
    base_path: Option<PathBuf>,
    resolver: Box<dyn QasmSourceResolver>,
) -> Result<Circuit, QasmParseError> {
    parse_qasm_source_with_context(source, None, base_path, resolver)
}

fn parse_qasm_file_with_context(
    source: &str,
    root_path: PathBuf,
    base_path: Option<PathBuf>,
    resolver: Box<dyn QasmSourceResolver>,
) -> Result<Circuit, QasmParseError> {
    parse_qasm_source_with_context(source, Some(root_path), base_path, resolver)
}

fn parse_qasm_source_with_context(
    source: &str,
    root_path: Option<PathBuf>,
    base_path: Option<PathBuf>,
    resolver: Box<dyn QasmSourceResolver>,
) -> Result<Circuit, QasmParseError> {
    if let Some(literal) = first_oversized_integer_literal(source) {
        return Err(QasmParseError::ParseError(format!(
            "Integer literal '{literal}' exceeds supported i64 range"
        )));
    }

    let parser = parser::MainParser::new();
    let parsed = catch_unwind(AssertUnwindSafe(|| parser.parse(source))).map_err(|payload| {
        let message = if let Some(message) = payload.downcast_ref::<String>() {
            message.clone()
        } else if let Some(message) = payload.downcast_ref::<&'static str>() {
            (*message).to_string()
        } else {
            "OpenQASM 2.0 parser panicked".to_string()
        };
        QasmParseError::ParseError(message)
    })?;
    let program = match parsed {
        Ok(program) => program,
        Err(e) => return Err(QasmParseError::ParseError(format!("{:?}", e))),
    };
    if (program.version - 2.0).abs() > f64::EPSILON {
        return Err(QasmParseError::UnsupportedVersion(program.version));
    }

    let mut converter = AstToCircuit::new_with_root(base_path, root_path, resolver);
    converter.convert(&program)
}

fn first_oversized_integer_literal(source: &str) -> Option<String> {
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            index += 1;
            while index < bytes.len() && bytes[index] != b'"' {
                index += 1;
            }
            index += usize::from(index < bytes.len());
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' && bytes[index] != b'\r' {
                index += 1;
            }
            continue;
        }
        if !bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }

        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        let prev = start.checked_sub(1).and_then(|idx| bytes.get(idx)).copied();
        let next = bytes.get(index).copied();
        if matches!(prev, Some(b'.' | b'e' | b'E')) || matches!(next, Some(b'.' | b'e' | b'E')) {
            continue;
        }

        let literal = &source[start..index];
        if literal.parse::<i64>().is_err() {
            return Some(literal.to_string());
        }
    }
    None
}

/// Errors that can occur during OpenQASM parsing and conversion.
///
/// Each variant represents a distinct error category that can occur
/// when processing OpenQASM 2.0 source code.
#[derive(Debug)]
pub enum QasmParseError {
    /// File system or I/O error (file not found, permission denied, etc.)
    IoError(io::Error),
    /// Syntax error during parsing (invalid QASM syntax)
    ParseError(String),
    /// Semantic error during AST to Circuit conversion
    ConversionError(String),
    /// Reference to undefined quantum register or qubit
    UndefinedQubit(String),
    /// Reference to undefined classical register
    UndefinedRegister(String),
    /// Reference to undefined gate
    UndefinedGate(String),
    /// Attempt to redefine a gate declared by qelib1.inc
    ReservedGateName(String),
    /// Invalid argument format or usage
    InvalidArgument(String),
    /// Gate applied to wrong number of qubits
    MismatchedQubitCount { expected: usize, actual: usize },
    /// Gate called with wrong number of parameters
    MismatchedParameterCount { expected: usize, actual: usize },
    /// Gate expansion exceeded maximum recursion depth (circular definition likely)
    RecursionLimitExceeded(String),
    /// Failed to evaluate parameter expression (undefined variable, division by zero, etc.)
    EvaluationError(String),
    /// Circular gate dependency detected (gate calls itself directly or indirectly)
    CircularGateDependency { gate: String, dependency: String },
    /// Unsupported OpenQASM version for this loader
    UnsupportedVersion(f64),
    /// Duplicate declaration or namespace conflict
    DuplicateDeclaration(String),
    /// Include cycle detected
    IncludeCycle(Vec<PathBuf>),
    /// Opaque gate invocation is not representable in the Circuit IR
    UnsupportedOpaqueGate(String),
}

impl std::fmt::Display for QasmParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QasmParseError::IoError(s) => write!(f, "IO error: {}", s),
            QasmParseError::ParseError(s) => write!(f, "Parse error: {}", s),
            QasmParseError::ConversionError(s) => write!(f, "Conversion error: {}", s),
            QasmParseError::UndefinedQubit(s) => write!(f, "Undefined qubit: {}", s),
            QasmParseError::UndefinedRegister(s) => write!(f, "Undefined register: {}", s),
            QasmParseError::UndefinedGate(s) => write!(f, "Undefined gate: {}", s),
            QasmParseError::ReservedGateName(s) => write!(
                f,
                "Gate name '{}' is reserved by qelib1.inc and cannot be redefined",
                s
            ),
            QasmParseError::InvalidArgument(s) => write!(f, "Invalid argument: {}", s),
            QasmParseError::MismatchedQubitCount { expected, actual } => {
                write!(
                    f,
                    "Mismatched qubit count: expected {}, got {}",
                    expected, actual
                )
            }
            QasmParseError::MismatchedParameterCount { expected, actual } => {
                write!(
                    f,
                    "Mismatched parameter count: expected {}, got {}",
                    expected, actual
                )
            }
            QasmParseError::RecursionLimitExceeded(s) => {
                write!(f, "Recursion limit exceeded: {}", s)
            }
            QasmParseError::EvaluationError(s) => write!(f, "Evaluation error: {}", s),
            QasmParseError::CircularGateDependency { gate, dependency } => {
                write!(
                    f,
                    "Circular gate dependency detected: '{}' depends on '{}'",
                    gate, dependency
                )
            }
            QasmParseError::UnsupportedVersion(version) => {
                write!(f, "Unsupported OpenQASM version {version}; expected 2.0")
            }
            QasmParseError::DuplicateDeclaration(s) => {
                write!(f, "Duplicate declaration: {}", s)
            }
            QasmParseError::IncludeCycle(stack) => {
                write!(f, "Include cycle detected: ")?;
                for (index, path) in stack.iter().enumerate() {
                    if index > 0 {
                        write!(f, " -> ")?;
                    }
                    write!(f, "{}", path.display())?;
                }
                Ok(())
            }
            QasmParseError::UnsupportedOpaqueGate(s) => {
                write!(f, "Opaque gate '{}' cannot be invoked", s)
            }
        }
    }
}

impl std::error::Error for QasmParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            QasmParseError::IoError(error) => Some(error),
            _ => None,
        }
    }
}

impl From<CircuitError> for QasmParseError {
    fn from(value: CircuitError) -> Self {
        Self::ConversionError(value.to_string())
    }
}

/// Default maximum recursion depth for gate expansion
const DEFAULT_MAX_RECURSION_DEPTH: usize = 100;

/// Converts OpenQASM AST to internal Circuit representation.
///
/// This struct performs multi-phase conversion:
/// 1. **Discovery**: Collects register declarations and gate definitions
/// 2. **Compilation**: Compiles custom gates into CircuitGate
/// 3. **Generation**: Converts statements to circuit operations
struct AstToCircuit {
    /// Quantum register name -> size mapping
    qregs: HashMap<String, i64>,
    /// Declaration order for deterministic qubit indexing
    qreg_order: Vec<String>,
    /// Classical register name -> size mapping
    cregs: HashMap<String, i64>,
    /// Declaration order for deterministic classical-variable allocation.
    creg_order: Vec<String>,
    /// Classical register name -> circuit-local BitVec variable.
    creg_vars: HashMap<String, ClassicalVar>,
    /// Custom gate definitions (name -> definition)
    custom_gates: HashMap<String, CustomGateDef>,
    /// User declarations across the OpenQASM namespace.
    declarations: HashMap<String, DeclarationKind>,
    /// Base path for resolving relative include paths
    base_path: Option<PathBuf>,
    /// Optional path of the top-level file being loaded.
    root_path: Option<PathBuf>,
    /// Parsed include cache (path -> AST) to avoid re-parsing
    file_cache: HashMap<PathBuf, OpenQASMProgram>,
    /// Current gate expansion depth (for recursion limiting)
    recursion_depth: usize,
    /// Maximum allowed recursion depth (default: 100)
    max_recursion_depth: usize,
    /// Gates currently being compiled (for cycle detection)
    compiling_gates: HashSet<String>,
    /// File system abstraction
    resolver: Box<dyn QasmSourceResolver>,
    qelib1_included: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeclarationKind {
    QReg,
    CReg,
    Gate,
    Opaque,
}

/// A custom gate definition that stores either AST or compiled CircuitGate
#[derive(Clone, Debug)]
struct CustomGateDef {
    #[allow(dead_code)]
    name: String,
    params: Vec<String>,
    qubits: Vec<String>,
    /// The AST body compiled after discovery
    body: Vec<Statement>,
    /// The compiled circuit gate - None until compiled, None for opaque gates
    circuit_gate: Option<CircuitGate>,
    /// Whether this is an opaque gate
    is_opaque: bool,
}

impl AstToCircuit {
    #[cfg(test)]
    fn new(base_path: Option<PathBuf>, resolver: Box<dyn QasmSourceResolver>) -> Self {
        Self::new_with_root(base_path, None, resolver)
    }

    fn new_with_root(
        base_path: Option<PathBuf>,
        root_path: Option<PathBuf>,
        resolver: Box<dyn QasmSourceResolver>,
    ) -> Self {
        let mut converter = Self {
            qregs: HashMap::new(),
            qreg_order: Vec::new(),
            cregs: HashMap::new(),
            creg_order: Vec::new(),
            creg_vars: HashMap::new(),
            custom_gates: HashMap::new(),
            declarations: HashMap::new(),
            base_path,
            root_path,
            file_cache: HashMap::new(),
            recursion_depth: 0,
            max_recursion_depth: DEFAULT_MAX_RECURSION_DEPTH,
            compiling_gates: HashSet::new(),
            resolver,
            qelib1_included: false,
        };
        converter
            .register_qelib1_include()
            .expect("built-in qelib1 gate construction should not fail");
        converter
    }

    fn convert(&mut self, program: &OpenQASMProgram) -> Result<Circuit, QasmParseError> {
        // Phase 1: Discovery
        // Recursively traverse includes to find all qregs and gate definitions
        let current_dir = self.base_path.clone();
        let mut include_stack = self.initial_include_stack();
        self.discovery_pass_with_context(program, current_dir.as_deref(), &mut include_stack)?;

        // Phase 1.5: Compile all custom gates
        // This must happen after all gate definitions are discovered
        self.compile_all_gates()?;

        // Calculate total qubits
        let total_qubits = self.qregs.values().try_fold(0usize, |total, size| {
            total.checked_add(*size as usize).ok_or_else(|| {
                QasmParseError::ConversionError(
                    "Total quantum register size exceeds platform limits".to_string(),
                )
            })
        })?;
        let mut circuit = Circuit::new(total_qubits);

        for name in &self.creg_order {
            let width = self.cregs[name] as u32;
            let ty = ClassicalType::bit_vec(width).ok_or_else(|| {
                QasmParseError::ConversionError(format!(
                    "Classical register '{name}' must have non-zero width"
                ))
            })?;
            let var = circuit.var(ty);
            let zero =
                ClassicalExpr::pack_bits((0..width).map(|_| ClassicalExpr::bit_literal(false)))
                    .map_err(|e| QasmParseError::ConversionError(e.to_string()))?;
            circuit
                .store(var, zero)
                .map_err(|e| QasmParseError::ConversionError(e.to_string()))?;
            self.creg_vars.insert(name.clone(), var);
        }

        // Create the register start map: reg_name -> global_start_index
        let mut reg_start_map: HashMap<String, usize> = HashMap::new();
        let mut global_idx = 0;

        // Use declaration order for qubit mapping
        for name in &self.qreg_order {
            reg_start_map.insert(name.clone(), global_idx);
            let size = self.qregs[name];
            global_idx += size as usize;
        }

        // Phase 2: Generation
        // Process operations
        let mut include_stack = self.initial_include_stack();
        self.generation_pass(
            program,
            &mut circuit,
            &reg_start_map,
            current_dir.as_deref(),
            &mut include_stack,
        )?;

        Ok(circuit)
    }

    fn initial_include_stack(&self) -> Vec<PathBuf> {
        self.root_path.iter().cloned().collect()
    }

    /// Compile all custom gate definitions
    fn compile_all_gates(&mut self) -> Result<(), QasmParseError> {
        // Get list of gate names to compile
        let gate_names: Vec<String> = self.custom_gates.keys().cloned().collect();

        for name in gate_names {
            self.compile_gate_if_needed(&name)?;
        }

        Ok(())
    }

    /// Compile a single gate if not already compiled
    fn compile_gate_if_needed(
        &mut self,
        name: &str,
    ) -> Result<Option<CircuitGate>, QasmParseError> {
        // Check recursion depth
        if self.recursion_depth >= self.max_recursion_depth {
            return Err(QasmParseError::RecursionLimitExceeded(format!(
                "Gate expansion depth exceeded limit of {} (compiling {})",
                self.max_recursion_depth, name
            )));
        }

        // Check for circular dependency: if the gate is already being compiled, we have a cycle
        if self.compiling_gates.contains(name) {
            return Err(QasmParseError::CircularGateDependency {
                gate: name.to_string(),
                dependency: name.to_string(),
            });
        }

        // Check if already compiled
        if let Some(def) = self.custom_gates.get(name) {
            if def.is_opaque {
                return Ok(None);
            }
            if def.circuit_gate.is_some() {
                return Ok(def.circuit_gate.clone());
            }
        } else {
            return Err(QasmParseError::UndefinedGate(name.to_string()));
        }

        // Clone needed data to avoid borrow issues
        let (params, qubits, body) = if let Some(def) = self.custom_gates.get(name) {
            (def.params.clone(), def.qubits.clone(), def.body.clone())
        } else {
            return Err(QasmParseError::UndefinedGate(name.to_string()));
        };

        // Mark this gate as being compiled (for cycle detection)
        self.compiling_gates.insert(name.to_string());

        // Increment recursion depth
        self.recursion_depth += 1;

        let result = (|| {
            // Ensure dependencies are compiled
            for stmt in &body {
                if let Statement::CustomGate(dep_name, _, _) = stmt
                    && self.custom_gates.contains_key(dep_name)
                {
                    // Check if this dependency creates a cycle
                    if self.compiling_gates.contains(dep_name) {
                        return Err(QasmParseError::CircularGateDependency {
                            gate: name.to_string(),
                            dependency: dep_name.to_string(),
                        });
                    }
                    self.compile_gate_if_needed(dep_name)?;
                }
            }

            self.build_circuit_gate(name, &params, &qubits, &body)
        })();

        // Restore compilation state before propagating any error.
        self.recursion_depth -= 1;
        self.compiling_gates.remove(name);

        let circuit_gate = result?;

        // Store the compiled result
        if let Some(def) = self.custom_gates.get_mut(name) {
            def.circuit_gate = Some(circuit_gate.clone());
        }

        Ok(Some(circuit_gate))
    }

    fn register_qelib1_include(&mut self) -> Result<(), QasmParseError> {
        if self.qelib1_included {
            return Ok(());
        }

        debug_assert!(
            QELIB1_DIRECT_GATES
                .iter()
                .chain(QELIB1_CUSTOM_GATES.iter())
                .all(|name| QELIB1_RESERVED_GATE_NAMES.contains(name))
        );

        for decl in [
            Self::qelib1_ch_gate()?,
            Self::qelib1_cu1_gate()?,
            Self::qelib1_cu3_gate()?,
        ] {
            self.custom_gates.insert(decl.name.clone(), decl);
        }
        self.qelib1_included = true;
        Ok(())
    }

    fn qelib1_ch_gate() -> Result<CustomGateDef, QasmParseError> {
        let a = Qubit::new(0);
        let b = Qubit::new(1);
        let mut circuit = Circuit::new(2);

        circuit.h(b)?;
        circuit.sdg(b)?;
        circuit.cx(a, b)?;
        circuit.h(b)?;
        circuit.t(b)?;
        circuit.cx(a, b)?;
        circuit.t(b)?;
        circuit.h(b)?;
        circuit.s(b)?;
        circuit.x(b)?;
        circuit.s(a)?;

        Self::qelib1_gate_def("ch", &[], &["a", "b"], circuit)
    }

    fn qelib1_cu1_gate() -> Result<CustomGateDef, QasmParseError> {
        let a = Qubit::new(0);
        let b = Qubit::new(1);
        let lambda = Parameter::symbol("lambda");
        let mut circuit = Circuit::new(2);
        circuit.add_parameter(lambda.clone());

        circuit.phase(a, lambda.clone() / 2.0)?;
        circuit.cx(a, b)?;
        circuit.phase(b, (Parameter::from(0.0) - lambda.clone()) / 2.0)?;
        circuit.cx(a, b)?;
        circuit.phase(b, lambda / 2.0)?;

        Self::qelib1_gate_def("cu1", &["lambda"], &["a", "b"], circuit)
    }

    fn qelib1_cu3_gate() -> Result<CustomGateDef, QasmParseError> {
        let c = Qubit::new(0);
        let t = Qubit::new(1);
        let theta = Parameter::symbol("theta");
        let phi = Parameter::symbol("phi");
        let lambda = Parameter::symbol("lambda");
        let mut circuit = Circuit::new(2);
        circuit.add_parameter(theta.clone());
        circuit.add_parameter(phi.clone());
        circuit.add_parameter(lambda.clone());

        circuit.phase(t, (lambda.clone() - phi.clone()) / 2.0)?;
        circuit.cx(c, t)?;
        circuit.u(
            t,
            (Parameter::from(0.0) - theta.clone()) / 2.0,
            0.0,
            (Parameter::from(0.0) - (phi.clone() + lambda.clone())) / 2.0,
        )?;
        circuit.cx(c, t)?;
        circuit.u(t, theta / 2.0, phi, 0.0)?;

        Self::qelib1_gate_def("cu3", &["theta", "phi", "lambda"], &["c", "t"], circuit)
    }

    fn qelib1_gate_def(
        name: &str,
        params: &[&str],
        qubits: &[&str],
        circuit: Circuit,
    ) -> Result<CustomGateDef, QasmParseError> {
        let circuit_gate = CircuitGate::with_signature(
            name,
            FrozenCircuit::new(circuit),
            params.iter().map(|param| (*param).to_string()),
        )
        .map_err(|e| QasmParseError::ConversionError(e.to_string()))?;
        Ok(CustomGateDef {
            name: name.to_string(),
            params: params.iter().map(|param| (*param).to_string()).collect(),
            qubits: qubits.iter().map(|qubit| (*qubit).to_string()).collect(),
            body: Vec::new(),
            circuit_gate: Some(circuit_gate),
            is_opaque: false,
        })
    }

    fn declare_name(&mut self, name: &str, kind: DeclarationKind) -> Result<(), QasmParseError> {
        if let Some(existing) = self.declarations.get(name) {
            return Err(QasmParseError::DuplicateDeclaration(format!(
                "'{name}' declared as {existing:?} and {kind:?}"
            )));
        }
        self.declarations.insert(name.to_string(), kind);
        Ok(())
    }

    fn ensure_gate_name_allowed(&self, name: &str) -> Result<(), QasmParseError> {
        if QELIB1_RESERVED_GATE_NAMES.contains(&name) {
            return Err(QasmParseError::ReservedGateName(name.to_string()));
        }
        Ok(())
    }

    fn validate_gate_formals(params: &[String], qubits: &[String]) -> Result<(), QasmParseError> {
        let mut seen_params = HashSet::new();
        for param in params {
            if !seen_params.insert(param) {
                return Err(QasmParseError::InvalidArgument(format!(
                    "Duplicate gate parameter formal '{param}'"
                )));
            }
        }

        let mut seen_qubits = HashSet::new();
        for qubit in qubits {
            if !seen_qubits.insert(qubit) {
                return Err(QasmParseError::InvalidArgument(format!(
                    "Duplicate gate qubit formal '{qubit}'"
                )));
            }
            if seen_params.contains(qubit) {
                return Err(QasmParseError::InvalidArgument(format!(
                    "Gate formal '{qubit}' is both a parameter and a qubit"
                )));
            }
        }

        Ok(())
    }

    fn resolve_include_path(&self, current_dir: Option<&Path>, filename: &str) -> PathBuf {
        let path = if let Some(base) = current_dir.or(self.base_path.as_deref()) {
            base.join(filename)
        } else {
            PathBuf::from(filename)
        };
        normalize_path(path)
    }

    fn include_cycle_error(stack: &[PathBuf], target: &Path) -> QasmParseError {
        let mut cycle = stack.to_vec();
        cycle.push(target.to_path_buf());
        QasmParseError::IncludeCycle(cycle)
    }

    fn parse_include(
        &mut self,
        target_path: &Path,
        filename: &str,
    ) -> Result<OpenQASMProgram, QasmParseError> {
        if let Some(program) = self.file_cache.get(target_path) {
            return Ok(program.clone());
        }

        let content = self.resolver.resolve_source(target_path).map_err(|e| {
            QasmParseError::IoError(io::Error::new(
                e.kind(),
                format!("Include {}: {}", filename, e),
            ))
        })?;
        let parser = parser::ProgramBodyParser::new();
        let included_program = parser
            .parse(&content)
            .map_err(|e| QasmParseError::ParseError(format!("In {}: {:?}", filename, e)))?;

        self.file_cache
            .insert(target_path.to_path_buf(), included_program.clone());
        Ok(included_program)
    }

    #[cfg(test)]
    fn discovery_pass(&mut self, program: &OpenQASMProgram) -> Result<(), QasmParseError> {
        let current_dir = self.base_path.clone();
        let mut include_stack = self.initial_include_stack();
        self.discovery_pass_with_context(program, current_dir.as_deref(), &mut include_stack)
    }

    fn discovery_pass_with_context(
        &mut self,
        program: &OpenQASMProgram,
        current_dir: Option<&Path>,
        include_stack: &mut Vec<PathBuf>,
    ) -> Result<(), QasmParseError> {
        for stmt in &program.statements {
            match stmt {
                Statement::QReg(name, size) => {
                    if *size <= 0 {
                        return Err(QasmParseError::InvalidArgument(format!(
                            "Quantum register '{name}' must have positive width, got {size}"
                        )));
                    }
                    self.declare_name(name, DeclarationKind::QReg)?;
                    self.qregs.insert(name.clone(), *size);
                    self.qreg_order.push(name.clone());
                }
                Statement::CReg(name, size) => {
                    if *size <= 0 {
                        return Err(QasmParseError::InvalidArgument(format!(
                            "Classical register '{name}' must have positive width, got {size}"
                        )));
                    }
                    self.declare_name(name, DeclarationKind::CReg)?;
                    self.cregs.insert(name.clone(), *size);
                    self.creg_order.push(name.clone());
                }
                Statement::GateDecl(data) => {
                    self.ensure_gate_name_allowed(&data.name)?;
                    self.declare_name(&data.name, DeclarationKind::Gate)?;
                    Self::validate_gate_formals(&data.params, &data.qubits)?;

                    // Store AST for compilation after discovery
                    let decl = CustomGateDef {
                        name: data.name.clone(),
                        params: data.params.clone(),
                        qubits: data.qubits.clone(),
                        body: data.body.clone(),
                        circuit_gate: None,
                        is_opaque: false,
                    };
                    self.custom_gates.insert(data.name.clone(), decl);
                }
                Statement::Include(filename) => {
                    if filename == "qelib1.inc" {
                        self.register_qelib1_include()?;
                        continue;
                    }

                    let target_path = self.resolve_include_path(current_dir, filename);
                    if include_stack.contains(&target_path) {
                        return Err(Self::include_cycle_error(include_stack, &target_path));
                    }

                    let should_discover = !self.file_cache.contains_key(&target_path);
                    let included_program = self.parse_include(&target_path, filename)?;
                    if should_discover {
                        include_stack.push(target_path.clone());
                        let next_dir = target_path.parent().map(Path::to_path_buf);
                        self.discovery_pass_with_context(
                            &included_program,
                            next_dir.as_deref(),
                            include_stack,
                        )?;
                        include_stack.pop();
                    }
                }
                Statement::Opaque(name, params, qubits) => {
                    self.ensure_gate_name_allowed(name)?;
                    self.declare_name(name, DeclarationKind::Opaque)?;
                    Self::validate_gate_formals(params, qubits)?;

                    // Opaque gates have no body - they cannot be expanded
                    let decl = CustomGateDef {
                        name: name.clone(),
                        params: params.clone(),
                        qubits: qubits.clone(),
                        body: vec![],
                        circuit_gate: None,
                        is_opaque: true,
                    };
                    self.custom_gates.insert(name.clone(), decl);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Build a CircuitGate from a gate declaration
    fn build_circuit_gate(
        &self,
        name: &str,
        params: &[String],
        qubits: &[String],
        body: &[Statement],
    ) -> Result<CircuitGate, QasmParseError> {
        // Create a circuit with qubits named after the gate's formal parameters
        let num_qubits = qubits.len();
        let mut gate_circuit = Circuit::new(num_qubits);

        // Map formal qubit names to actual qubit indices
        let mut qubit_map: HashMap<String, Qubit> = HashMap::new();
        for (i, qubit_name) in qubits.iter().enumerate() {
            qubit_map.insert(qubit_name.clone(), Qubit::new(i as u32));
        }

        // Build parameter map for symbol resolution
        // In the gate body, parameter symbols should be bound to circuit parameters
        let mut param_map: HashMap<String, Parameter> = HashMap::new();
        for param_name in params.iter() {
            // Create a parameter symbol for this gate parameter
            let param = Parameter::symbol(param_name);
            param_map.insert(param_name.clone(), param);
        }

        // Process each statement in the gate body
        for stmt in body {
            self.build_gate_body_statement(stmt, &mut gate_circuit, &qubit_map, &param_map)?;
        }

        // Convert to CircuitGate
        let frozen = FrozenCircuit::new(gate_circuit);
        CircuitGate::with_signature(name, frozen, params.iter().cloned())
            .map_err(|e| QasmParseError::ConversionError(e.to_string()))
    }

    /// Build a single statement in a gate body
    fn build_gate_body_statement(
        &self,
        stmt: &Statement,
        circuit: &mut Circuit,
        qubit_map: &HashMap<String, Qubit>,
        param_map: &HashMap<String, Parameter>,
    ) -> Result<(), QasmParseError> {
        match stmt {
            Statement::CustomGate(name, args, qargs) => {
                // Resolve qubits
                let mut resolved_qubits = Vec::new();
                for arg in qargs {
                    match arg {
                        Argument::Id(qname) => {
                            if let Some(&q) = qubit_map.get(qname) {
                                resolved_qubits.push(q);
                            } else {
                                return Err(QasmParseError::UndefinedQubit(qname.clone()));
                            }
                        }
                        Argument::IndexedId(_, _) => {
                            return Err(QasmParseError::InvalidArgument(
                                "Indexed arguments not supported in gate body".to_string(),
                            ));
                        }
                    }
                }

                // Resolve parameters - convert expressions to Parameters
                let mut resolved_params: SmallVec<[ParameterValue; 3]> = smallvec![];
                for expr in args {
                    let param = Self::expr_to_parameter(expr, param_map)?;
                    resolved_params.push(ParameterValue::from(param));
                }

                // Try to find the gate definition
                if let Some(gate_def) = self.custom_gates.get(name) {
                    if let Some(ref cg) = gate_def.circuit_gate {
                        // Add the CircuitGate directly
                        circuit
                            .append(
                                Instruction::CircuitGate(Box::new(cg.clone())),
                                resolved_qubits,
                                resolved_params,
                                None,
                            )
                            .map_err(|e| QasmParseError::ConversionError(e.to_string()))?;
                    } else {
                        // Opaque gate - cannot be added to circuit body
                        return Err(QasmParseError::UndefinedGate(format!(
                            "Opaque gate {} cannot be used in gate body",
                            name
                        )));
                    }
                } else {
                    // Try standard gate
                    self.append_standard_gate(circuit, name, &resolved_params, &resolved_qubits)?;
                }
            }
            Statement::Barrier(args) => {
                let mut resolved_qubits = Vec::new();
                for arg in args {
                    match arg {
                        Argument::Id(qname) => {
                            if let Some(&q) = qubit_map.get(qname) {
                                resolved_qubits.push(q);
                            } else {
                                return Err(QasmParseError::UndefinedQubit(qname.clone()));
                            }
                        }
                        Argument::IndexedId(_, _) => {
                            return Err(QasmParseError::InvalidArgument(
                                "Indexed barrier arguments are not supported in gate body"
                                    .to_string(),
                            ));
                        }
                    }
                }
                circuit
                    .append(
                        Instruction::Directive(Directive::Barrier),
                        resolved_qubits,
                        std::iter::empty::<ParameterValue>(),
                        None,
                    )
                    .map_err(|e| QasmParseError::ConversionError(e.to_string()))?;
            }
            _ => {
                return Err(QasmParseError::ConversionError(format!(
                    "Unsupported statement type in gate body: {:?}",
                    stmt
                )));
            }
        }
        Ok(())
    }

    /// Convert an expression to a Parameter
    fn expr_to_parameter(
        expr: &Expression,
        param_map: &HashMap<String, Parameter>,
    ) -> Result<Parameter, QasmParseError> {
        match expr {
            Expression::Real(v) => Ok(Parameter::from(*v)),
            Expression::Integer(v) => Ok(Parameter::from(*v as f64)),
            Expression::Pi => Ok(Parameter::pi()),
            Expression::Id(name) => {
                if let Some(param) = param_map.get(name) {
                    Ok(param.clone())
                } else {
                    // Try to evaluate as a constant or return error
                    Err(QasmParseError::EvaluationError(format!(
                        "Unknown parameter: {}",
                        name
                    )))
                }
            }
            Expression::BinaryOp(left, op, right) => {
                let l = Self::expr_to_parameter(left, param_map)?;
                let r = Self::expr_to_parameter(right, param_map)?;
                Ok(match op {
                    OpCode::Add => l + r,
                    OpCode::Sub => l - r,
                    OpCode::Mul => l * r,
                    OpCode::Div => {
                        // Check for division by zero in constant case
                        if let Ok(val) = r.evaluate(&None)
                            && val == 0.0
                        {
                            return Err(QasmParseError::EvaluationError(
                                "Division by zero".to_string(),
                            ));
                        }
                        l / r
                    }
                    OpCode::Pow => l.pow(r),
                })
            }
            Expression::UnaryOp(op, expr) => {
                let v = Self::expr_to_parameter(expr, param_map)?;
                Ok(match op {
                    UnaryOpCode::Sin => v.sin(),
                    UnaryOpCode::Cos => v.cos(),
                    UnaryOpCode::Tan => v.tan(),
                    UnaryOpCode::Exp => v.exp(),
                    UnaryOpCode::Ln => v.ln(),
                    UnaryOpCode::Sqrt => v.sqrt(),
                    UnaryOpCode::Asin => v.asin(),
                    UnaryOpCode::Acos => v.acos(),
                    UnaryOpCode::Atan => v.atan(),
                    UnaryOpCode::Neg => Parameter::from(0.0) - v,
                })
            }
        }
    }

    /// Build a standard gate instruction with validated parameters.
    ///
    /// This centralized function handles all standard gate name matching, qubit/parameter count
    /// validation, and special parameter transformations (like U2).
    ///
    /// Returns the Instruction and the validated ParameterValues to use.
    fn build_standard_gate_instruction(
        name: &str,
        params: &[ParameterValue],
        qubits: &[Qubit],
    ) -> Result<(Instruction, Vec<ParameterValue>), QasmParseError> {
        // Helper closure for checking qubit and parameter counts
        let check_counts = |req_q: usize, req_p: usize| -> Result<(), QasmParseError> {
            if qubits.len() != req_q {
                return Err(QasmParseError::MismatchedQubitCount {
                    expected: req_q,
                    actual: qubits.len(),
                });
            }
            if params.len() != req_p {
                return Err(QasmParseError::MismatchedParameterCount {
                    expected: req_p,
                    actual: params.len(),
                });
            }
            Ok(())
        };

        match name {
            "u2" | "U2" => {
                check_counts(1, 2)?;
                // U2(phi, lambda) = U(pi/2, phi, lambda)
                let pi_2 = ParameterValue::Param(Parameter::pi() / 2.0);
                let new_params = vec![pi_2, params[0].clone(), params[1].clone()];
                Ok((Instruction::Standard(StandardGate::U), new_params))
            }

            "rx" | "RX" => {
                check_counts(1, 1)?;
                Ok((Instruction::Standard(StandardGate::RX), params.to_vec()))
            }
            "ry" | "RY" => {
                check_counts(1, 1)?;
                Ok((Instruction::Standard(StandardGate::RY), params.to_vec()))
            }
            "rz" | "RZ" => {
                check_counts(1, 1)?;
                Ok((Instruction::Standard(StandardGate::RZ), params.to_vec()))
            }
            "p" | "P" => {
                check_counts(1, 1)?;
                Ok((Instruction::Standard(StandardGate::Phase), params.to_vec()))
            }
            "u1" | "U1" => {
                check_counts(1, 1)?;
                Ok((Instruction::Standard(StandardGate::Phase), params.to_vec()))
            }

            // --- Standard Parametrized Gates (1 qubit, 3 params) ---
            "u3" | "U3" | "u" | "U" => {
                check_counts(1, 3)?;
                Ok((Instruction::Standard(StandardGate::U), params.to_vec()))
            }

            "rxx" | "RXX" => {
                check_counts(2, 1)?;
                Ok((Instruction::Standard(StandardGate::RXX), params.to_vec()))
            }
            "ryy" | "RYY" => {
                check_counts(2, 1)?;
                Ok((Instruction::Standard(StandardGate::RYY), params.to_vec()))
            }
            "rzz" | "RZZ" => {
                check_counts(2, 1)?;
                Ok((Instruction::Standard(StandardGate::RZZ), params.to_vec()))
            }
            "fsim" | "FSIM" => {
                check_counts(2, 2)?;
                Ok((Instruction::Standard(StandardGate::FSIM), params.to_vec()))
            }

            // --- Controlled rotation gates (2 qubits, 1 param) ---
            "crx" | "CRX" => {
                check_counts(2, 1)?;
                Ok((Instruction::Standard(StandardGate::CRX), params.to_vec()))
            }
            "cry" | "CRY" => {
                check_counts(2, 1)?;
                Ok((Instruction::Standard(StandardGate::CRY), params.to_vec()))
            }
            "crz" | "CRZ" => {
                check_counts(2, 1)?;
                Ok((Instruction::Standard(StandardGate::CRZ), params.to_vec()))
            }

            "h" | "H" => {
                check_counts(1, 0)?;
                Ok((Instruction::Standard(StandardGate::H), vec![]))
            }
            "x" | "X" => {
                check_counts(1, 0)?;
                Ok((Instruction::Standard(StandardGate::X), vec![]))
            }
            "y" | "Y" => {
                check_counts(1, 0)?;
                Ok((Instruction::Standard(StandardGate::Y), vec![]))
            }
            "z" | "Z" => {
                check_counts(1, 0)?;
                Ok((Instruction::Standard(StandardGate::Z), vec![]))
            }
            "s" | "S" => {
                check_counts(1, 0)?;
                Ok((Instruction::Standard(StandardGate::S), vec![]))
            }
            "sdg" | "SDG" => {
                check_counts(1, 0)?;
                Ok((Instruction::Standard(StandardGate::SDG), vec![]))
            }
            "t" | "T" => {
                check_counts(1, 0)?;
                Ok((Instruction::Standard(StandardGate::T), vec![]))
            }
            "tdg" | "TDG" => {
                check_counts(1, 0)?;
                Ok((Instruction::Standard(StandardGate::TDG), vec![]))
            }
            "sx" | "SX" => {
                check_counts(1, 0)?;
                Ok((Instruction::Standard(StandardGate::X2P), vec![]))
            }
            "sxdg" | "SXDG" => {
                check_counts(1, 0)?;
                Ok((Instruction::Standard(StandardGate::X2M), vec![]))
            }
            "id" | "ID" | "i" | "I" => {
                check_counts(1, 0)?;
                Ok((Instruction::Standard(StandardGate::I), vec![]))
            }
            "cx" | "CX" => {
                check_counts(2, 0)?;
                Ok((Instruction::Standard(StandardGate::CX), vec![]))
            }
            "cy" | "CY" => {
                check_counts(2, 0)?;
                Ok((Instruction::Standard(StandardGate::CY), vec![]))
            }
            "cz" | "CZ" => {
                check_counts(2, 0)?;
                Ok((Instruction::Standard(StandardGate::CZ), vec![]))
            }
            "swap" | "SWAP" => {
                check_counts(2, 0)?;
                Ok((Instruction::Standard(StandardGate::SWAP), vec![]))
            }
            "ccx" | "CCX" | "toffoli" | "TOFFOLI" => {
                check_counts(3, 0)?;
                Ok((Instruction::Standard(StandardGate::CCX), vec![]))
            }

            _ => Err(QasmParseError::UndefinedGate(name.to_string())),
        }
    }

    /// Append a standard gate to the circuit (used in gate declarations)
    fn append_standard_gate(
        &self,
        circuit: &mut Circuit,
        name: &str,
        params: &[ParameterValue],
        qubits: &[Qubit],
    ) -> Result<(), QasmParseError> {
        let (instruction, valid_params) =
            Self::build_standard_gate_instruction(name, params, qubits)?;
        circuit
            .append(instruction, qubits.to_vec(), valid_params, None)
            .map_err(|e| QasmParseError::ConversionError(e.to_string()))?;
        Ok(())
    }

    fn generation_pass(
        &mut self,
        program: &OpenQASMProgram,
        circuit: &mut Circuit,
        reg_start_map: &HashMap<String, usize>,
        current_dir: Option<&Path>,
        include_stack: &mut Vec<PathBuf>,
    ) -> Result<(), QasmParseError> {
        for stmt in &program.statements {
            match stmt {
                Statement::Include(filename) => {
                    if filename == "qelib1.inc" {
                        continue;
                    }

                    let target_path = self.resolve_include_path(current_dir, filename);
                    if include_stack.contains(&target_path) {
                        return Err(Self::include_cycle_error(include_stack, &target_path));
                    }

                    // Retrieve from cache instead of re-parsing
                    // Clone the program to avoid borrow issues
                    let included_program = self.file_cache.get(&target_path).cloned();
                    if let Some(included) = included_program {
                        include_stack.push(target_path.clone());
                        let next_dir = target_path.parent().map(Path::to_path_buf);
                        self.generation_pass(
                            &included,
                            circuit,
                            reg_start_map,
                            next_dir.as_deref(),
                            include_stack,
                        )?;
                        include_stack.pop();
                    } else {
                        // Should not happen if discovery pass worked correctly
                        return Err(QasmParseError::ConversionError(format!(
                            "Included file not found in cache: {:?}",
                            target_path
                        )));
                    }
                }
                Statement::Barrier(args) => {
                    let qubits = self.resolve_global_args(args, reg_start_map)?;
                    circuit
                        .barrier(qubits)
                        .map_err(|e| QasmParseError::ConversionError(e.to_string()))?;
                }
                Statement::Reset(arg) => {
                    // Reset can be applied to single qubit or entire register
                    let qubits = self.resolve_global_args_single_or_register(arg, reg_start_map)?;
                    for qubit in qubits {
                        circuit
                            .reset(qubit)
                            .map_err(|e| QasmParseError::ConversionError(e.to_string()))?;
                    }
                }
                Statement::Measure(qarg, carg) => {
                    self.append_measurement(circuit, qarg, carg, reg_start_map)?;
                }
                Statement::CustomGate(name, params, args) => {
                    self.append_gate_invocation(circuit, name, params, args, reg_start_map)?;
                }
                Statement::If(creg, value, stmt) => {
                    let condition = self.build_condition(creg, *value)?;
                    circuit
                        .if_(condition, |body| {
                            self.append_conditional_statement(stmt, body, reg_start_map)
                                .map_err(|e| CircuitError::InvalidOperation(e.to_string()))
                        })
                        .map_err(|e| QasmParseError::ConversionError(e.to_string()))?;
                }
                _ => {} // Declarations already handled
            }
        }
        Ok(())
    }

    fn append_gate_invocation(
        &self,
        circuit: &mut Circuit,
        name: &str,
        params: &[Expression],
        args: &[Argument],
        reg_start_map: &HashMap<String, usize>,
    ) -> Result<(), QasmParseError> {
        let param_values = Self::resolve_invocation_params(params)?;
        let arg_groups = args
            .iter()
            .map(|arg| self.resolve_global_args_single_or_register(arg, reg_start_map))
            .collect::<Result<Vec<_>, _>>()?;

        let width = arg_groups.iter().map(Vec::len).max().ok_or_else(|| {
            QasmParseError::InvalidArgument(format!("Gate '{name}' has no arguments"))
        })?;

        for group in &arg_groups {
            if group.len() != 1 && group.len() != width {
                return Err(QasmParseError::InvalidArgument(format!(
                    "Cannot broadcast gate '{name}': argument widths must be 1 or {width}, got {}",
                    group.len()
                )));
            }
        }

        for index in 0..width {
            let qubits = arg_groups
                .iter()
                .map(|group| group[if group.len() == 1 { 0 } else { index }])
                .collect::<Vec<_>>();
            self.append_single_gate_invocation(circuit, name, param_values.clone(), qubits)?;
        }

        Ok(())
    }

    fn resolve_invocation_params(
        params: &[Expression],
    ) -> Result<SmallVec<[ParameterValue; 3]>, QasmParseError> {
        let mut param_values: SmallVec<[ParameterValue; 3]> = smallvec![];
        let empty_param_map: HashMap<String, Parameter> = HashMap::new();
        for expr in params {
            let param = Self::expr_to_parameter(expr, &empty_param_map)?;
            param_values.push(ParameterValue::from(param));
        }
        Ok(param_values)
    }

    fn append_single_gate_invocation(
        &self,
        circuit: &mut Circuit,
        name: &str,
        params: SmallVec<[ParameterValue; 3]>,
        qubits: Vec<Qubit>,
    ) -> Result<(), QasmParseError> {
        if let Some(gate_def) = self.custom_gates.get(name) {
            if gate_def.is_opaque {
                return Err(QasmParseError::UnsupportedOpaqueGate(name.to_string()));
            }
            if let Some(ref cg) = gate_def.circuit_gate {
                circuit
                    .append(
                        Instruction::CircuitGate(Box::new(cg.clone())),
                        qubits,
                        params,
                        None,
                    )
                    .map_err(|e| QasmParseError::ConversionError(e.to_string()))?;
                return Ok(());
            }
            return Err(QasmParseError::UndefinedGate(name.to_string()));
        }

        self.append_standard_gate_to_circuit(circuit, name, &params, &qubits)
    }

    fn append_measurement(
        &self,
        circuit: &mut Circuit,
        qarg: &Argument,
        carg: &Argument,
        reg_start_map: &HashMap<String, usize>,
    ) -> Result<(), QasmParseError> {
        let qubits = self.resolve_global_args_single_or_register(qarg, reg_start_map)?;
        let creg_indices = self.resolve_creg_indices(carg)?;

        if qubits.len() != creg_indices.len() {
            return Err(QasmParseError::InvalidArgument(format!(
                "Measure qubit count ({}) does not match classical register count ({})",
                qubits.len(),
                creg_indices.len()
            )));
        }

        let creg_name = match carg {
            Argument::Id(name) | Argument::IndexedId(name, _) => name,
        };
        let target = *self
            .creg_vars
            .get(creg_name)
            .ok_or_else(|| QasmParseError::UndefinedRegister(creg_name.clone()))?;

        if matches!(carg, Argument::Id(_)) {
            let result = circuit.measure_bits(qubits)?;
            circuit.store(target, result.expr())?;
        } else {
            let result = circuit.measure(qubits[0])?;
            let width = target.ty().width();
            let measured_index = creg_indices[0] as u32;
            let mut bits = Vec::with_capacity(width as usize);
            for index in 0..width {
                let bit = if index == measured_index {
                    result.expr()
                } else {
                    ClassicalExpr::extract_bit(target.expr(), index)
                        .map_err(|e| QasmParseError::ConversionError(e.to_string()))?
                };
                bits.push(bit);
            }
            let packed = ClassicalExpr::pack_bits(bits)
                .map_err(|e| QasmParseError::ConversionError(e.to_string()))?;
            circuit.store(target, packed)?;
        }
        Ok(())
    }

    fn append_conditional_statement(
        &mut self,
        stmt: &Statement,
        circuit: &mut Circuit,
        reg_start_map: &HashMap<String, usize>,
    ) -> Result<(), QasmParseError> {
        match stmt {
            Statement::CustomGate(name, params, args) => {
                self.append_gate_invocation(circuit, name, params, args, reg_start_map)?;
            }
            Statement::Reset(arg) => {
                let qubits = self.resolve_global_args_single_or_register(arg, reg_start_map)?;
                for qubit in qubits {
                    circuit.reset(qubit)?;
                }
            }
            Statement::Measure(qarg, carg) => {
                self.append_measurement(circuit, qarg, carg, reg_start_map)?;
            }
            Statement::Barrier(_) => {
                return Err(QasmParseError::ConversionError(
                    "Barrier in an OpenQASM 2.0 conditional body is not supported".to_string(),
                ));
            }
            _ => {
                return Err(QasmParseError::ConversionError(format!(
                    "Unsupported statement type in if-body: {:?}",
                    stmt
                )));
            }
        }
        Ok(())
    }

    fn resolve_global_args(
        &self,
        args: &[Argument],
        reg_start_map: &HashMap<String, usize>,
    ) -> Result<Vec<Qubit>, QasmParseError> {
        let mut qubits = Vec::new();
        for arg in args {
            match arg {
                Argument::Id(name) => {
                    // Expand whole register
                    if let Some(size) = self.qregs.get(name) {
                        if let Some(&start_idx) = reg_start_map.get(name) {
                            for i in 0..*size {
                                qubits.push(Qubit::new((start_idx as i64 + i) as u32));
                            }
                        } else {
                            return Err(QasmParseError::UndefinedRegister(name.clone()));
                        }
                    } else {
                        return Err(QasmParseError::UndefinedRegister(name.clone()));
                    }
                }
                Argument::IndexedId(name, idx) => {
                    if let Some(&start_idx) = reg_start_map.get(name) {
                        // Validate index
                        if let Some(size) = self.qregs.get(name) {
                            if *idx < 0 || *idx >= *size {
                                return Err(QasmParseError::UndefinedQubit(format!(
                                    "{}[{}] out of bounds",
                                    name, idx
                                )));
                            }
                            qubits.push(Qubit::new((start_idx as i64 + *idx) as u32));
                        } else {
                            return Err(QasmParseError::UndefinedRegister(name.clone()));
                        }
                    } else {
                        return Err(QasmParseError::UndefinedQubit(format!("{}[{}]", name, idx)));
                    }
                }
            }
        }
        Ok(qubits)
    }

    /// Resolve argument to either single qubit or expanded register
    fn resolve_global_args_single_or_register(
        &self,
        arg: &Argument,
        reg_start_map: &HashMap<String, usize>,
    ) -> Result<Vec<Qubit>, QasmParseError> {
        let mut qubits = Vec::new();
        match arg {
            Argument::Id(name) => {
                // Expand entire register
                if let Some(size) = self.qregs.get(name) {
                    if let Some(&start_idx) = reg_start_map.get(name) {
                        for i in 0..*size {
                            qubits.push(Qubit::new((start_idx as i64 + i) as u32));
                        }
                    } else {
                        return Err(QasmParseError::UndefinedRegister(name.clone()));
                    }
                } else {
                    return Err(QasmParseError::UndefinedRegister(name.clone()));
                }
            }
            Argument::IndexedId(name, idx) => {
                // Single indexed qubit
                if let Some(&start_idx) = reg_start_map.get(name) {
                    if let Some(size) = self.qregs.get(name) {
                        if *idx < 0 || *idx >= *size {
                            return Err(QasmParseError::UndefinedQubit(format!(
                                "{}[{}]",
                                name, idx
                            )));
                        }
                        qubits.push(Qubit::new((start_idx as i64 + *idx) as u32));
                    } else {
                        return Err(QasmParseError::UndefinedRegister(name.clone()));
                    }
                } else {
                    return Err(QasmParseError::UndefinedQubit(format!("{}[{}]", name, idx)));
                }
            }
        }
        Ok(qubits)
    }

    /// Resolve classical register argument to indices
    fn resolve_creg_indices(&self, arg: &Argument) -> Result<Vec<usize>, QasmParseError> {
        let mut indices = Vec::new();
        match arg {
            Argument::Id(name) => {
                // Entire classical register
                if let Some(size) = self.cregs.get(name) {
                    for i in 0..*size {
                        indices.push(i as usize);
                    }
                } else {
                    return Err(QasmParseError::UndefinedRegister(format!(
                        "Classical register '{}' not defined",
                        name
                    )));
                }
            }
            Argument::IndexedId(name, idx) => {
                let size = self.cregs.get(name).ok_or_else(|| {
                    QasmParseError::UndefinedRegister(format!(
                        "Classical register '{}' not defined",
                        name
                    ))
                })?;
                if *idx < 0 || *idx >= *size {
                    return Err(QasmParseError::InvalidArgument(format!(
                        "Classical register index {name}[{idx}] is out of bounds for width {size}"
                    )));
                }
                indices.push(*idx as usize);
            }
        }
        Ok(indices)
    }

    fn build_condition(&self, name: &str, value: i64) -> Result<ClassicalExpr, QasmParseError> {
        let var = *self
            .creg_vars
            .get(name)
            .ok_or_else(|| QasmParseError::UndefinedRegister(name.to_string()))?;
        let width = var.ty().width();

        if value < 0 {
            return Err(QasmParseError::InvalidArgument(format!(
                "Classical condition value must be non-negative, got {value}"
            )));
        }

        let value = value as u128;
        if width < 128 && value >= (1u128 << width) {
            return Err(QasmParseError::InvalidArgument(format!(
                "Condition value {value} does not fit classical register '{name}' width {width}"
            )));
        }
        let lhs = ClassicalExpr::bit_vec_to_uint(var.expr())
            .map_err(|e| QasmParseError::ConversionError(e.to_string()))?;
        let rhs = ClassicalExpr::uint_literal(width, value)
            .map_err(|e| QasmParseError::ConversionError(e.to_string()))?;

        ClassicalExpr::eq(lhs, rhs).map_err(|e| QasmParseError::ConversionError(e.to_string()))
    }

    /// Append a standard gate to the circuit using ParameterValues (for top-level circuit)
    fn append_standard_gate_to_circuit(
        &self,
        circuit: &mut Circuit,
        name: &str,
        params: &[ParameterValue],
        qubits: &[Qubit],
    ) -> Result<(), QasmParseError> {
        let (instruction, valid_params) =
            Self::build_standard_gate_instruction(name, params, qubits)?;
        circuit
            .append(instruction, qubits.to_vec(), valid_params, None)
            .map_err(|e| QasmParseError::ConversionError(e.to_string()))?;
        Ok(())
    }
}

fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.as_ref().components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
#[path = "./load_test.rs"]
mod load_test;
