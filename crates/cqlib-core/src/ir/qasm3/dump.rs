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

//! OpenQASM 3 serializer.

use crate::circuit::circuit_param::CircuitParam;
use crate::circuit::gate::{
    CircuitGate, ClassicalDataOp, Directive, FrozenCircuit, Instruction, StandardGate,
};
use crate::circuit::operation::Operation;
use crate::circuit::parameter::Parameter;
use crate::circuit::{
    Circuit, ClassicalBinaryOp, ClassicalCast, ClassicalCompareOp, ClassicalControlOp,
    ClassicalExpr, ClassicalExprKind, ClassicalType, ClassicalUnaryOp, ClassicalValue,
    ClassicalVar, Qubit,
};
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::fs::File;
use std::io::{self, Write as IoWrite};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug)]
pub enum Qasm3DumpError {
    IoError(io::Error),
    FormatError(String),
    UnsupportedInstruction(String),
    UnsupportedClassicalData(String),
    UnsupportedClassicalControl(String),
    MeasureInGateNotAllowed,
    ConflictingGateDefinition {
        name: String,
        existing_qubits: usize,
        existing_params: usize,
        conflicting_qubits: usize,
        conflicting_params: usize,
    },
}

impl PartialEq for Qasm3DumpError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::IoError(lhs), Self::IoError(rhs)) => {
                lhs.kind() == rhs.kind() && lhs.to_string() == rhs.to_string()
            }
            (Self::FormatError(lhs), Self::FormatError(rhs)) => lhs == rhs,
            (Self::UnsupportedInstruction(lhs), Self::UnsupportedInstruction(rhs)) => lhs == rhs,
            (Self::UnsupportedClassicalData(lhs), Self::UnsupportedClassicalData(rhs)) => {
                lhs == rhs
            }
            (Self::UnsupportedClassicalControl(lhs), Self::UnsupportedClassicalControl(rhs)) => {
                lhs == rhs
            }
            (Self::MeasureInGateNotAllowed, Self::MeasureInGateNotAllowed) => true,
            (
                Self::ConflictingGateDefinition {
                    name: lhs_name,
                    existing_qubits: lhs_existing_qubits,
                    existing_params: lhs_existing_params,
                    conflicting_qubits: lhs_conflicting_qubits,
                    conflicting_params: lhs_conflicting_params,
                },
                Self::ConflictingGateDefinition {
                    name: rhs_name,
                    existing_qubits: rhs_existing_qubits,
                    existing_params: rhs_existing_params,
                    conflicting_qubits: rhs_conflicting_qubits,
                    conflicting_params: rhs_conflicting_params,
                },
            ) => {
                lhs_name == rhs_name
                    && lhs_existing_qubits == rhs_existing_qubits
                    && lhs_existing_params == rhs_existing_params
                    && lhs_conflicting_qubits == rhs_conflicting_qubits
                    && lhs_conflicting_params == rhs_conflicting_params
            }
            _ => false,
        }
    }
}

impl std::fmt::Display for Qasm3DumpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(error) => write!(f, "IO error: {error}"),
            Self::FormatError(s) => write!(f, "Format error: {s}"),
            Self::UnsupportedInstruction(s) => write!(f, "Unsupported OpenQASM 3 instruction: {s}"),
            Self::UnsupportedClassicalData(s) => {
                write!(f, "Unsupported OpenQASM 3 classical data: {s}")
            }
            Self::UnsupportedClassicalControl(s) => {
                write!(f, "Unsupported OpenQASM 3 classical control: {s}")
            }
            Self::MeasureInGateNotAllowed => {
                write!(f, "Measurement inside gate definition is not allowed")
            }
            Self::ConflictingGateDefinition {
                name,
                existing_qubits,
                existing_params,
                conflicting_qubits,
                conflicting_params,
            } => write!(
                f,
                "Conflicting definitions for gate '{name}': existing definition has {existing_qubits} qubits and {existing_params} parameters, conflicting definition has {conflicting_qubits} qubits and {conflicting_params} parameters"
            ),
        }
    }
}

impl std::error::Error for Qasm3DumpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(error) => Some(error),
            _ => None,
        }
    }
}
// 019ec904-7234-75b0-b459-9fe87d49cdbb
impl From<std::fmt::Error> for Qasm3DumpError {
    fn from(value: std::fmt::Error) -> Self {
        Self::FormatError(value.to_string())
    }
}

impl From<io::Error> for Qasm3DumpError {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}

pub fn dump<P: AsRef<Path>>(circuit: &Circuit, path: P) -> Result<(), Qasm3DumpError> {
    let qasm = dumps(circuit)?;
    let mut file = File::create(path)?;
    file.write_all(qasm.as_bytes())?;
    Ok(())
}

/// Write a circuit to an OpenQASM 3 file.
///
/// Rust-style alias for [`dump`]. The Python-style `dump` name is retained for
/// compatibility with the rest of the IR module API.
pub fn to_path<P: AsRef<Path>>(circuit: &Circuit, path: P) -> Result<(), Qasm3DumpError> {
    dump(circuit, path)
}

pub fn dumps(circuit: &Circuit) -> Result<String, Qasm3DumpError> {
    let mut output = String::new();
    writeln!(&mut output, "OPENQASM 3.0;")?;
    writeln!(&mut output, "include \"stdgates.inc\";")?;

    let mut used_standard_gates = HashSet::new();
    collect_used_standard_gates(circuit.operations(), &mut used_standard_gates);
    for gate in extension_definition_order(&used_standard_gates) {
        writeln!(&mut output)?;
        writeln!(&mut output, "{}", extension_gate_definition(gate).unwrap())?;
    }

    let mut defined_gates = IndexMap::new();
    let mut unitary_gate_defs = IndexMap::new();
    collect_gates(
        circuit.operations(),
        &mut defined_gates,
        &mut unitary_gate_defs,
    )?;
    for gate in defined_gates.values() {
        writeln!(&mut output)?;
        dump_gate_definition(gate, &mut output)?;
    }
    for (label, frozen) in unitary_gate_defs.iter() {
        writeln!(&mut output)?;
        dump_unitary_gate_definition(label, frozen, &mut output)?;
    }

    writeln!(&mut output)?;
    let mut name_allocator =
        NameAllocator::new(reserved_top_level_names(&defined_gates, &unitary_gate_defs));
    let param_map = dump_input_parameters(circuit, &mut output, &mut name_allocator)?;
    let skipped_values = skipped_classical_value_declarations(circuit.operations())?;
    let qubit_register = name_allocator.take("q");
    let auto_measurements =
        AutoMeasurementMap::new(circuit.operations(), name_allocator.take("meas"))?;
    let qubit_map = logical_qubit_map(circuit, &qubit_register);
    writeln!(&mut output, "qubit[{}] {qubit_register};", qubit_map.len())?;
    dump_qubit_mapping_comment(&mut output, &qubit_map)?;
    let classical_names =
        ClassicalNameMap::new(circuit, &mut output, &skipped_values, &mut name_allocator)?;
    auto_measurements.dump_declaration(&mut output)?;
    writeln!(&mut output)?;
    dump_global_phase(circuit, &mut output, &param_map)?;

    dump_operations(
        circuit,
        circuit.operations(),
        &mut output,
        &qubit_map,
        &qubit_register,
        &param_map,
        &classical_names,
        &auto_measurements,
        false,
    )?;
    Ok(output)
}

#[derive(Debug)]
struct NameAllocator {
    used: HashSet<String>,
}

impl NameAllocator {
    fn new(used: HashSet<String>) -> Self {
        Self { used }
    }

    fn take(&mut self, preferred: &str) -> String {
        if self.used.insert(preferred.to_string()) {
            return preferred.to_string();
        }

        let mut suffix = 0usize;
        loop {
            let candidate = format!("{preferred}{suffix}");
            if self.used.insert(candidate.clone()) {
                return candidate;
            }
            suffix += 1;
        }
    }

    fn take_indexed(&mut self, prefix: &str, index: usize) -> String {
        self.take(&format!("{prefix}{index}"))
    }
}

fn reserved_top_level_names(
    defined_gates: &IndexMap<String, CircuitGate>,
    unitary_gate_defs: &IndexMap<String, Arc<FrozenCircuit>>,
) -> HashSet<String> {
    defined_gates
        .keys()
        .chain(unitary_gate_defs.keys())
        .cloned()
        .collect()
}

fn dump_input_parameters(
    circuit: &Circuit,
    output: &mut String,
    name_allocator: &mut NameAllocator,
) -> Result<HashMap<String, Parameter>, Qasm3DumpError> {
    let mut param_map = HashMap::new();
    for symbol in circuit.symbols() {
        let name = name_allocator.take(symbol);
        writeln!(output, "input angle[64] {name};")?;
        if name != *symbol {
            param_map.insert(symbol.clone(), Parameter::symbol(&name));
        }
    }
    Ok(param_map)
}

fn logical_qubit_map(circuit: &Circuit, register_name: &str) -> HashMap<Qubit, String> {
    let mut qubits = circuit.qubits();
    qubits.sort_by_key(|qubit| qubit.index());
    qubits
        .into_iter()
        .enumerate()
        .map(|(logical_index, qubit)| (qubit, format!("{register_name}[{logical_index}]")))
        .collect()
}

fn dump_qubit_mapping_comment(
    output: &mut String,
    qubit_map: &HashMap<Qubit, String>,
) -> Result<(), Qasm3DumpError> {
    if qubit_map.is_empty() {
        return Ok(());
    }

    let mut mappings = qubit_map.iter().collect::<Vec<_>>();
    mappings.sort_by_key(|(_, name)| {
        name.trim_start_matches("q[")
            .trim_end_matches(']')
            .parse::<usize>()
            .unwrap_or(usize::MAX)
    });
    if mappings
        .iter()
        .all(|(qubit, name)| **name == format!("q[{}]", qubit.index()))
    {
        return Ok(());
    }

    for (qubit, name) in mappings {
        writeln!(output, "// {name} -> Q{}", qubit.index())?;
    }
    Ok(())
}

/// Serialize a circuit to an OpenQASM 3 string.
///
/// Rust-style alias for [`dumps`].
pub fn to_string(circuit: &Circuit) -> Result<String, Qasm3DumpError> {
    dumps(circuit)
}

fn dump_global_phase(
    circuit: &Circuit,
    output: &mut String,
    param_map: &HashMap<String, Parameter>,
) -> Result<(), Qasm3DumpError> {
    let phase = circuit.global_phase();
    if phase.is_zero() {
        return Ok(());
    }
    let mut phase = phase.clone();
    for (symbol, replacement) in param_map {
        phase = phase.replace(symbol, replacement.clone());
    }
    writeln!(output, "gphase({});", phase.to_string().replace("π", "pi"))?;
    Ok(())
}

#[derive(Debug, Default)]
struct ClassicalNameMap {
    vars: HashMap<ClassicalVar, String>,
    values: HashMap<ClassicalValue, String>,
}

impl ClassicalNameMap {
    fn new(
        circuit: &Circuit,
        output: &mut String,
        skipped_values: &HashSet<ClassicalValue>,
        name_allocator: &mut NameAllocator,
    ) -> Result<Self, Qasm3DumpError> {
        let mut map = Self::default();
        for (index, ty) in circuit.classical_vars().iter().copied().enumerate() {
            let name = name_allocator.take_indexed("c", index);
            writeln!(output, "{} {name};", classical_decl_type(ty)?)?;
            map.vars
                .insert(ClassicalVar::new(circuit.id(), index as u32, ty), name);
        }
        for (index, ty) in circuit.classical_values().iter().copied().enumerate() {
            let value = ClassicalValue::new(circuit.id(), index as u32, ty);
            if skipped_values.contains(&value) {
                continue;
            }
            let name = name_allocator.take_indexed("v", index);
            writeln!(output, "{} {name};", classical_decl_type(ty)?)?;
            map.values.insert(value, name);
        }
        Ok(map)
    }

    fn var(&self, var: ClassicalVar) -> Result<&str, Qasm3DumpError> {
        self.vars.get(&var).map(String::as_str).ok_or_else(|| {
            Qasm3DumpError::UnsupportedClassicalData(format!(
                "classical variable {} is not owned by this circuit",
                var.index()
            ))
        })
    }

    fn value(&self, value: ClassicalValue) -> Result<&str, Qasm3DumpError> {
        self.values.get(&value).map(String::as_str).ok_or_else(|| {
            Qasm3DumpError::UnsupportedClassicalData(format!(
                "classical value {} is not owned by this circuit",
                value.index()
            ))
        })
    }
}

fn classical_decl_type(ty: ClassicalType) -> Result<String, Qasm3DumpError> {
    Ok(match ty {
        ClassicalType::Bit => "bit".to_string(),
        ClassicalType::Bool => "bool".to_string(),
        ClassicalType::BitVec(width) => format!("bit[{}]", width.get()),
        ClassicalType::UInt(width) => format!("uint[{}]", width.get()),
    })
}

fn collect_gates(
    operations: &[Operation],
    defined_gates: &mut IndexMap<String, CircuitGate>,
    unitary_gate_defs: &mut IndexMap<String, Arc<FrozenCircuit>>,
) -> Result<(), Qasm3DumpError> {
    for op in operations {
        match &op.instruction {
            Instruction::CircuitGate(gate) => {
                collect_gates(
                    gate.circuit.circuit.operations(),
                    defined_gates,
                    unitary_gate_defs,
                )?;
                if let Some(existing) = defined_gates.get(gate.name.as_str()) {
                    if !Arc::ptr_eq(&existing.circuit, &gate.circuit) {
                        return Err(Qasm3DumpError::ConflictingGateDefinition {
                            name: gate.name.to_string(),
                            existing_qubits: existing.num_qubits(),
                            existing_params: existing.num_params(),
                            conflicting_qubits: gate.num_qubits(),
                            conflicting_params: gate.num_params(),
                        });
                    }
                } else {
                    defined_gates.insert(gate.name.to_string(), *gate.clone());
                }
            }
            Instruction::UnitaryGate(gate) => {
                let Some(circuit) = gate.circuit() else {
                    return Err(Qasm3DumpError::UnsupportedInstruction(format!(
                        "matrix-only unitary gate '{}'",
                        gate.label()
                    )));
                };
                collect_gates(
                    circuit.circuit.operations(),
                    defined_gates,
                    unitary_gate_defs,
                )?;
                unitary_gate_defs
                    .entry(gate.label().to_string())
                    .or_insert_with(|| circuit.clone());
            }
            Instruction::ClassicalControl(control) => {
                collect_control_gates(control, defined_gates, unitary_gate_defs)?
            }
            Instruction::Delay => {
                return Err(Qasm3DumpError::UnsupportedInstruction("delay".to_string()));
            }
            _ => {}
        }
    }
    Ok(())
}

fn collect_control_gates(
    control: &ClassicalControlOp,
    defined_gates: &mut IndexMap<String, CircuitGate>,
    unitary_gate_defs: &mut IndexMap<String, Arc<FrozenCircuit>>,
) -> Result<(), Qasm3DumpError> {
    match control {
        ClassicalControlOp::If(op) => {
            collect_gates(
                op.then_body().operations(),
                defined_gates,
                unitary_gate_defs,
            )?;
            if let Some(body) = op.else_body() {
                collect_gates(body.operations(), defined_gates, unitary_gate_defs)?;
            }
        }
        ClassicalControlOp::Switch(op) => {
            for case in op.cases() {
                collect_gates(case.body().operations(), defined_gates, unitary_gate_defs)?;
            }
            if let Some(body) = op.default() {
                collect_gates(body.operations(), defined_gates, unitary_gate_defs)?;
            }
        }
        ClassicalControlOp::While(_) => {
            return Err(Qasm3DumpError::UnsupportedClassicalControl(
                "while".to_string(),
            ));
        }
        ClassicalControlOp::For(_) => {
            return Err(Qasm3DumpError::UnsupportedClassicalControl(
                "for".to_string(),
            ));
        }
        ClassicalControlOp::Break | ClassicalControlOp::Continue => {
            return Err(Qasm3DumpError::UnsupportedClassicalControl(
                "break/continue".to_string(),
            ));
        }
    }
    Ok(())
}

fn dump_gate_definition(gate: &CircuitGate, output: &mut String) -> Result<(), Qasm3DumpError> {
    if gate
        .circuit
        .circuit
        .operations()
        .iter()
        .any(|operation| operation.instruction.has_measurement())
    {
        return Err(Qasm3DumpError::MeasureInGateNotAllowed);
    }

    let params: Vec<String> = gate.signature_params().iter().cloned().collect();
    let params = if params.is_empty() {
        String::new()
    } else {
        format!("({})", params.join(","))
    };
    let qubits: Vec<String> = (0..gate.num_qubits()).map(|i| format!("q{i}")).collect();
    writeln!(
        output,
        "gate {}{} {} {{",
        gate.name,
        params,
        qubits.join(",")
    )?;

    let mut qubit_map = HashMap::new();
    for (index, qubit) in gate.circuit.circuit.qubits().iter().copied().enumerate() {
        qubit_map.insert(qubit, format!("q{index}"));
    }
    dump_operations(
        &gate.circuit.circuit,
        gate.circuit.circuit.operations(),
        output,
        &qubit_map,
        "",
        &HashMap::new(),
        &ClassicalNameMap::default(),
        &AutoMeasurementMap::default(),
        true,
    )?;
    writeln!(output, "}}")?;
    Ok(())
}

fn dump_unitary_gate_definition(
    label: &str,
    frozen: &FrozenCircuit,
    output: &mut String,
) -> Result<(), Qasm3DumpError> {
    if frozen
        .circuit
        .operations()
        .iter()
        .any(|operation| operation.instruction.has_measurement())
    {
        return Err(Qasm3DumpError::MeasureInGateNotAllowed);
    }
    let qubits: Vec<String> = (0..frozen.circuit.qubits().len())
        .map(|i| format!("q{i}"))
        .collect();
    writeln!(output, "gate {label} {} {{", qubits.join(","))?;
    let mut qubit_map = HashMap::new();
    for (index, qubit) in frozen.circuit.qubits().iter().copied().enumerate() {
        qubit_map.insert(qubit, format!("q{index}"));
    }
    dump_operations(
        &frozen.circuit,
        frozen.circuit.operations(),
        output,
        &qubit_map,
        "",
        &HashMap::new(),
        &ClassicalNameMap::default(),
        &AutoMeasurementMap::default(),
        true,
    )?;
    writeln!(output, "}}")?;
    Ok(())
}

fn dump_operations(
    circuit: &Circuit,
    operations: &[Operation],
    output: &mut String,
    qubit_map: &HashMap<Qubit, String>,
    qubit_register: &str,
    param_map: &HashMap<String, Parameter>,
    classical_names: &ClassicalNameMap,
    auto_measurements: &AutoMeasurementMap,
    in_gate_body: bool,
) -> Result<(), Qasm3DumpError> {
    let mut index = 0;
    let mut accepting_initializers = true;
    while index < operations.len() {
        let op = &operations[index];
        if let Instruction::ClassicalData(ClassicalDataOp::Store { target, value }) =
            &op.instruction
        {
            if accepting_initializers && is_zero_initializer(*target, value) {
                index += 1;
                continue;
            }
        }
        accepting_initializers = false;

        match &op.instruction {
            Instruction::Standard(gate) => {
                dump_standard_gate(gate, op, circuit, output, qubit_map, param_map)?
            }
            Instruction::McGate(gate) => {
                dump_mc_gate(gate, op, circuit, output, qubit_map, param_map)?
            }
            Instruction::Directive(directive) => dump_directive(directive, op, output, qubit_map)?,
            Instruction::CircuitGate(gate) => {
                let params = format_params(op, circuit, param_map);
                let qubits = map_qubits(op, qubit_map);
                writeln!(output, "{}{} {};", gate.name, params, qubits.join(","))?;
            }
            Instruction::UnitaryGate(gate) => {
                let qubits = map_qubits(op, qubit_map);
                writeln!(output, "{} {};", gate.label(), qubits.join(","))?;
            }
            Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result })
            | Instruction::ClassicalData(ClassicalDataOp::MeasureBits { result }) => {
                if in_gate_body {
                    return Err(Qasm3DumpError::MeasureInGateNotAllowed);
                }
                let next = operations.get(index + 1);
                let (mut destination, consumes_store) =
                    measurement_destination(&op.instruction, *result, next)?;
                if consumes_store
                    && operations[index + 2..]
                        .iter()
                        .any(|operation| operation.instruction.reads_value(*result))
                {
                    return Err(Qasm3DumpError::UnsupportedClassicalData(format!(
                        "measurement value {} is read after being stored into a mutable variable",
                        result.index()
                    )));
                }
                if !consumes_store
                    && !operations[index + 1..]
                        .iter()
                        .any(|operation| operation.instruction.reads_value(*result))
                {
                    destination = MeasurementDestination::Discard(*result);
                }
                dump_measurement(
                    op,
                    *result,
                    destination,
                    output,
                    qubit_map,
                    qubit_register,
                    classical_names,
                    auto_measurements,
                )?;
                if consumes_store {
                    index += 1;
                }
            }
            Instruction::ClassicalData(ClassicalDataOp::Store { .. }) => {
                dump_classical_store(op, output, classical_names)?;
            }
            Instruction::ClassicalControl(control) => {
                if in_gate_body {
                    return Err(Qasm3DumpError::UnsupportedClassicalControl(
                        "control flow in gate body".to_string(),
                    ));
                }
                dump_control_flow(
                    control,
                    circuit,
                    output,
                    qubit_map,
                    qubit_register,
                    param_map,
                    classical_names,
                    auto_measurements,
                )?;
            }
            Instruction::Delay => {
                return Err(Qasm3DumpError::UnsupportedInstruction("delay".to_string()));
            }
        }
        index += 1;
    }
    Ok(())
}

fn is_zero_initializer(target: ClassicalVar, value: &ClassicalExpr) -> bool {
    match (target.ty(), value.kind()) {
        (ClassicalType::Bit, ClassicalExprKind::BitLiteral(false)) => true,
        (ClassicalType::Bool, ClassicalExprKind::BoolLiteral(false)) => true,
        (ClassicalType::UInt(target_width), ClassicalExprKind::UIntLiteral { width, value: 0 }) => {
            target_width == *width
        }
        (
            ClassicalType::BitVec(target_width),
            ClassicalExprKind::BitVecLiteral { width, value: 0 },
        ) => target_width == *width,
        _ => false,
    }
}

fn dump_classical_store(
    op: &Operation,
    output: &mut String,
    names: &ClassicalNameMap,
) -> Result<(), Qasm3DumpError> {
    let Instruction::ClassicalData(ClassicalDataOp::Store { target, value }) = &op.instruction
    else {
        return Err(Qasm3DumpError::UnsupportedClassicalData(
            "non-store classical operation".to_string(),
        ));
    };
    if target.ty() == ClassicalType::Bit {
        return Err(Qasm3DumpError::UnsupportedClassicalData(
            "scalar bit store assignment".to_string(),
        ));
    }
    writeln!(
        output,
        "{} = {};",
        names.var(*target)?,
        classical_expr_to_qasm(value, names)?
    )?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum MeasurementDestination {
    Value(ClassicalValue),
    Discard(ClassicalValue),
    VarWhole(ClassicalVar),
    VarBit(ClassicalVar, u32),
}

#[derive(Debug, Clone, Copy)]
struct AutoMeasurementTarget {
    start: usize,
    width: usize,
}

#[derive(Debug, Default)]
struct AutoMeasurementMap {
    targets: HashMap<ClassicalValue, AutoMeasurementTarget>,
    width: usize,
    name: String,
}

impl AutoMeasurementMap {
    fn new(operations: &[Operation], name: String) -> Result<Self, Qasm3DumpError> {
        let mut map = Self {
            name,
            ..Self::default()
        };
        collect_auto_measurements(operations, &mut map)?;
        Ok(map)
    }

    fn insert(&mut self, value: ClassicalValue) {
        let width = value.ty().width() as usize;
        let target = AutoMeasurementTarget {
            start: self.width,
            width,
        };
        self.width += width;
        self.targets.insert(value, target);
    }

    fn target(&self, value: ClassicalValue) -> Option<AutoMeasurementTarget> {
        self.targets.get(&value).copied()
    }

    fn dump_declaration(&self, output: &mut String) -> Result<(), Qasm3DumpError> {
        if self.width > 0 {
            writeln!(output, "bit[{}] {};", self.width, self.name)?;
        }
        Ok(())
    }
}

fn collect_auto_measurements(
    operations: &[Operation],
    map: &mut AutoMeasurementMap,
) -> Result<(), Qasm3DumpError> {
    let mut index = 0;
    while index < operations.len() {
        let op = &operations[index];
        match &op.instruction {
            Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result })
            | Instruction::ClassicalData(ClassicalDataOp::MeasureBits { result }) => {
                let next = operations.get(index + 1);
                let (_, consumes_store) = measurement_destination(&op.instruction, *result, next)?;
                let remaining_start = index + 1 + usize::from(consumes_store);
                let read_later = operations[remaining_start..]
                    .iter()
                    .any(|operation| operation.instruction.reads_value(*result));
                if !consumes_store && !read_later {
                    map.insert(*result);
                }
                if consumes_store {
                    index += 1;
                }
            }
            Instruction::ClassicalControl(control) => {
                collect_auto_measurements_in_control(control, map)?;
            }
            _ => {}
        }
        index += 1;
    }
    Ok(())
}

fn collect_auto_measurements_in_control(
    control: &ClassicalControlOp,
    map: &mut AutoMeasurementMap,
) -> Result<(), Qasm3DumpError> {
    match control {
        ClassicalControlOp::If(op) => {
            collect_auto_measurements(op.then_body().operations(), map)?;
            if let Some(body) = op.else_body() {
                collect_auto_measurements(body.operations(), map)?;
            }
        }
        ClassicalControlOp::Switch(op) => {
            for case in op.cases() {
                collect_auto_measurements(case.body().operations(), map)?;
            }
            if let Some(body) = op.default() {
                collect_auto_measurements(body.operations(), map)?;
            }
        }
        ClassicalControlOp::While(op) => {
            collect_auto_measurements(op.body().operations(), map)?;
        }
        ClassicalControlOp::For(op) => {
            collect_auto_measurements(op.body().operations(), map)?;
        }
        ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
    }
    Ok(())
}

fn skipped_classical_value_declarations(
    operations: &[Operation],
) -> Result<HashSet<ClassicalValue>, Qasm3DumpError> {
    let mut skipped = HashSet::new();
    collect_skipped_classical_values(operations, &mut skipped)?;
    Ok(skipped)
}

fn collect_skipped_classical_values(
    operations: &[Operation],
    skipped: &mut HashSet<ClassicalValue>,
) -> Result<(), Qasm3DumpError> {
    let mut index = 0;
    while index < operations.len() {
        let op = &operations[index];
        match &op.instruction {
            Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result })
            | Instruction::ClassicalData(ClassicalDataOp::MeasureBits { result }) => {
                let next = operations.get(index + 1);
                let (_, consumes_store) = measurement_destination(&op.instruction, *result, next)?;
                let remaining_start = index + 1 + usize::from(consumes_store);
                let read_later = operations[remaining_start..]
                    .iter()
                    .any(|operation| operation.instruction.reads_value(*result));
                if !read_later {
                    skipped.insert(*result);
                }
                if consumes_store {
                    index += 1;
                }
            }
            Instruction::ClassicalControl(control) => {
                collect_skipped_classical_values_in_control(control, skipped)?;
            }
            _ => {}
        }
        index += 1;
    }
    Ok(())
}

fn collect_skipped_classical_values_in_control(
    control: &ClassicalControlOp,
    skipped: &mut HashSet<ClassicalValue>,
) -> Result<(), Qasm3DumpError> {
    match control {
        ClassicalControlOp::If(op) => {
            collect_skipped_classical_values(op.then_body().operations(), skipped)?;
            if let Some(body) = op.else_body() {
                collect_skipped_classical_values(body.operations(), skipped)?;
            }
        }
        ClassicalControlOp::Switch(op) => {
            for case in op.cases() {
                collect_skipped_classical_values(case.body().operations(), skipped)?;
            }
            if let Some(body) = op.default() {
                collect_skipped_classical_values(body.operations(), skipped)?;
            }
        }
        ClassicalControlOp::While(op) => {
            collect_skipped_classical_values(op.body().operations(), skipped)?;
        }
        ClassicalControlOp::For(op) => {
            collect_skipped_classical_values(op.body().operations(), skipped)?;
        }
        ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
    }
    Ok(())
}

fn measurement_destination(
    instruction: &Instruction,
    result: ClassicalValue,
    next: Option<&Operation>,
) -> Result<(MeasurementDestination, bool), Qasm3DumpError> {
    let Some(Operation {
        instruction: Instruction::ClassicalData(ClassicalDataOp::Store { target, value }),
        ..
    }) = next
    else {
        return Ok((MeasurementDestination::Value(result), false));
    };

    if matches!(value.kind(), ClassicalExprKind::Value(value) if *value == result) {
        return match instruction {
            Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
                if target.ty() == ClassicalType::Bit =>
            {
                Ok((MeasurementDestination::VarWhole(*target), true))
            }
            Instruction::ClassicalData(ClassicalDataOp::MeasureBits { .. })
                if target.ty() == result.ty() =>
            {
                Ok((MeasurementDestination::VarWhole(*target), true))
            }
            _ => Err(Qasm3DumpError::UnsupportedClassicalData(
                "measurement store has incompatible target type".to_string(),
            )),
        };
    }

    if matches!(
        instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ) {
        if let Some(bit) = value.measurement_store_bit(*target, result) {
            return Ok((MeasurementDestination::VarBit(*target, bit), true));
        }
    }

    Err(Qasm3DumpError::UnsupportedClassicalData(
        "measurement is followed by an unsupported store".to_string(),
    ))
}

fn dump_measurement(
    op: &Operation,
    result: ClassicalValue,
    destination: MeasurementDestination,
    output: &mut String,
    qubit_map: &HashMap<Qubit, String>,
    qubit_register: &str,
    names: &ClassicalNameMap,
    auto_measurements: &AutoMeasurementMap,
) -> Result<(), Qasm3DumpError> {
    let qubits = map_qubits(op, qubit_map);
    let (target, width) = match destination {
        MeasurementDestination::Value(value) => {
            (names.value(value)?.to_string(), value.ty().width())
        }
        MeasurementDestination::Discard(value) => {
            let Some(target) = auto_measurements.target(value) else {
                return Err(Qasm3DumpError::UnsupportedClassicalData(format!(
                    "discarded measurement value {} has no generated classical target",
                    value.index()
                )));
            };
            (auto_measurements.name.clone(), target.width as u32)
        }
        MeasurementDestination::VarWhole(var) => (names.var(var)?.to_string(), var.ty().width()),
        MeasurementDestination::VarBit(var, bit) => (format!("{}[{bit}]", names.var(var)?), 1),
    };

    if qubits.len() != width as usize || result.ty().width() != width {
        return Err(Qasm3DumpError::UnsupportedClassicalData(format!(
            "measurement width {} does not match destination width {width}",
            qubits.len()
        )));
    }

    if width == 1 {
        let source = measurement_source(&qubits[0], qubit_map.len());
        if let MeasurementDestination::Discard(value) = destination {
            let target = auto_measurements.target(value).unwrap();
            writeln!(
                output,
                "{}[{}] = measure {source};",
                auto_measurements.name, target.start
            )?;
        } else {
            writeln!(output, "{target} = measure {source};")?;
        }
    } else if is_full_register_measurement(&qubits, qubit_register, qubit_map.len()) {
        if let MeasurementDestination::Discard(value) = destination {
            let target = auto_measurements.target(value).unwrap();
            for (index, qubit) in qubits.iter().enumerate() {
                let source = measurement_source(qubit, qubit_map.len());
                writeln!(
                    output,
                    "{}[{}] = measure {source};",
                    auto_measurements.name,
                    target.start + index
                )?;
            }
        } else {
            writeln!(output, "{target} = measure {qubit_register};")?;
        }
    } else {
        for (index, qubit) in qubits.iter().enumerate() {
            let source = measurement_source(qubit, qubit_map.len());
            if let MeasurementDestination::Discard(value) = destination {
                let target = auto_measurements.target(value).unwrap();
                writeln!(
                    output,
                    "{}[{}] = measure {source};",
                    auto_measurements.name,
                    target.start + index
                )?;
            } else {
                writeln!(output, "{target}[{index}] = measure {source};")?;
            }
        }
    }
    Ok(())
}

fn measurement_source(qubit: &str, qubit_count: usize) -> &str {
    let _ = qubit_count;
    qubit
}

fn is_full_register_measurement(
    qubits: &[String],
    qubit_register: &str,
    qubit_count: usize,
) -> bool {
    qubits.len() == qubit_count
        && qubits
            .iter()
            .enumerate()
            .all(|(index, qubit)| qubit == &format!("{qubit_register}[{index}]"))
}

fn dump_standard_gate(
    gate: &StandardGate,
    op: &Operation,
    circuit: &Circuit,
    output: &mut String,
    qubit_map: &HashMap<Qubit, String>,
    param_map: &HashMap<String, Parameter>,
) -> Result<(), Qasm3DumpError> {
    let name = standard_gate_name(*gate)?;
    let params = format_params(op, circuit, param_map);
    let qubits = map_qubits(op, qubit_map);
    if *gate == StandardGate::GPhase {
        writeln!(output, "gphase{};", params)?;
    } else {
        writeln!(output, "{}{} {};", name, params, qubits.join(","))?;
    }
    Ok(())
}

fn standard_gate_name(gate: StandardGate) -> Result<&'static str, Qasm3DumpError> {
    Ok(match gate {
        StandardGate::I => "id",
        StandardGate::X => "x",
        StandardGate::Y => "y",
        StandardGate::Z => "z",
        StandardGate::H => "h",
        StandardGate::S => "s",
        StandardGate::SDG => "sdg",
        StandardGate::T => "t",
        StandardGate::TDG => "tdg",
        StandardGate::X2P => "x2p",
        StandardGate::X2M => "x2m",
        StandardGate::Y2P => "y2p",
        StandardGate::Y2M => "y2m",
        StandardGate::RX => "rx",
        StandardGate::RY => "ry",
        StandardGate::RZ => "rz",
        StandardGate::RXX => "rxx",
        StandardGate::RYY => "ryy",
        StandardGate::RZZ => "rzz",
        StandardGate::RZX => "rzx",
        StandardGate::Phase => "p",
        StandardGate::GPhase => "gphase",
        StandardGate::U => "u3",
        StandardGate::CX => "cx",
        StandardGate::CY => "cy",
        StandardGate::CZ => "cz",
        StandardGate::CRX => "crx",
        StandardGate::CRY => "cry",
        StandardGate::CRZ => "crz",
        StandardGate::SWAP => "swap",
        StandardGate::CCX => "ccx",
        StandardGate::XY2P => "xy2p",
        StandardGate::XY2M => "xy2m",
        StandardGate::FSIM => "fsim",
        StandardGate::XY | StandardGate::RXY => {
            return Err(Qasm3DumpError::UnsupportedInstruction(format!(
                "standard gate {gate}"
            )));
        }
    })
}

fn dump_mc_gate(
    gate: &crate::circuit::gate::MCGate,
    op: &Operation,
    circuit: &Circuit,
    output: &mut String,
    qubit_map: &HashMap<Qubit, String>,
    param_map: &HashMap<String, Parameter>,
) -> Result<(), Qasm3DumpError> {
    let name = match (gate.num_ctrl_qubits(), gate.base_gate()) {
        (1, StandardGate::X) => "cx",
        (1, StandardGate::Y) => "cy",
        (1, StandardGate::Z) => "cz",
        (1, StandardGate::H) => "ch",
        (1, StandardGate::RX) => "crx",
        (1, StandardGate::RY) => "cry",
        (1, StandardGate::RZ) => "crz",
        (1, StandardGate::Phase) => "cp",
        (1, StandardGate::CX) | (2, StandardGate::X) => "ccx",
        (1, StandardGate::SWAP) => "cswap",
        _ => {
            return Err(Qasm3DumpError::UnsupportedInstruction(format!(
                "multi-controlled {} with {} controls",
                gate.base_gate(),
                gate.num_ctrl_qubits()
            )));
        }
    };
    let params = format_params(op, circuit, param_map);
    let qubits = map_qubits(op, qubit_map);
    writeln!(output, "{}{} {};", name, params, qubits.join(","))?;
    Ok(())
}

fn dump_directive(
    directive: &Directive,
    op: &Operation,
    output: &mut String,
    qubit_map: &HashMap<Qubit, String>,
) -> Result<(), Qasm3DumpError> {
    let qubits = map_qubits(op, qubit_map);
    match directive {
        Directive::Measure => Err(Qasm3DumpError::UnsupportedClassicalData(
            "legacy measurement has no classical destination".to_string(),
        )),
        Directive::Barrier => {
            writeln!(output, "barrier {};", qubits.join(","))?;
            Ok(())
        }
        Directive::Reset => {
            for qubit in qubits {
                writeln!(output, "reset {qubit};")?;
            }
            Ok(())
        }
    }
}

fn dump_control_flow(
    control: &ClassicalControlOp,
    circuit: &Circuit,
    output: &mut String,
    qubit_map: &HashMap<Qubit, String>,
    qubit_register: &str,
    param_map: &HashMap<String, Parameter>,
    classical_names: &ClassicalNameMap,
    auto_measurements: &AutoMeasurementMap,
) -> Result<(), Qasm3DumpError> {
    match control {
        ClassicalControlOp::If(op) => {
            let condition = classical_expr_to_qasm(op.condition(), classical_names)?;
            writeln!(output, "if ({condition}) {{")?;
            dump_operations(
                circuit,
                op.then_body().operations(),
                output,
                qubit_map,
                qubit_register,
                param_map,
                classical_names,
                auto_measurements,
                false,
            )?;
            if let Some(body) = op.else_body() {
                writeln!(output, "}} else {{")?;
                dump_operations(
                    circuit,
                    body.operations(),
                    output,
                    qubit_map,
                    qubit_register,
                    param_map,
                    classical_names,
                    auto_measurements,
                    false,
                )?;
            }
            writeln!(output, "}}")?;
        }
        ClassicalControlOp::Switch(op) => {
            let target = classical_expr_to_qasm(op.target(), classical_names)?;
            writeln!(output, "switch ({target}) {{")?;
            for case in op.cases() {
                writeln!(output, "case {} {{", case.value())?;
                dump_operations(
                    circuit,
                    case.body().operations(),
                    output,
                    qubit_map,
                    qubit_register,
                    param_map,
                    classical_names,
                    auto_measurements,
                    false,
                )?;
                writeln!(output, "}}")?;
            }
            if let Some(body) = op.default() {
                writeln!(output, "default {{")?;
                dump_operations(
                    circuit,
                    body.operations(),
                    output,
                    qubit_map,
                    qubit_register,
                    param_map,
                    classical_names,
                    auto_measurements,
                    false,
                )?;
                writeln!(output, "}}")?;
            }
            writeln!(output, "}}")?;
        }
        ClassicalControlOp::While(_) => {
            return Err(Qasm3DumpError::UnsupportedClassicalControl(
                "while".to_string(),
            ));
        }
        ClassicalControlOp::For(_) => {
            return Err(Qasm3DumpError::UnsupportedClassicalControl(
                "for".to_string(),
            ));
        }
        ClassicalControlOp::Break | ClassicalControlOp::Continue => {
            return Err(Qasm3DumpError::UnsupportedClassicalControl(
                "break/continue".to_string(),
            ));
        }
    }
    Ok(())
}

fn classical_expr_to_qasm(
    expr: &ClassicalExpr,
    names: &ClassicalNameMap,
) -> Result<String, Qasm3DumpError> {
    Ok(match expr.kind() {
        ClassicalExprKind::Var(var) => names.var(*var)?.to_string(),
        ClassicalExprKind::Value(value) => names.value(*value)?.to_string(),
        ClassicalExprKind::BoolLiteral(value) => value.to_string(),
        ClassicalExprKind::BitLiteral(value) => value.to_string(),
        ClassicalExprKind::UIntLiteral { value, .. } => value.to_string(),
        ClassicalExprKind::BitVecLiteral { width, value } => {
            format!("\"{value:0width$b}\"", width = width.get() as usize)
        }
        ClassicalExprKind::Unary {
            op: ClassicalUnaryOp::Not,
            expr,
        } => format!("!({})", classical_expr_to_qasm(expr, names)?),
        ClassicalExprKind::Binary { op, lhs, rhs } => {
            let op = match op {
                ClassicalBinaryOp::And => "&",
                ClassicalBinaryOp::Or => "|",
                ClassicalBinaryOp::Xor => "^",
            };
            format!(
                "({} {op} {})",
                classical_expr_to_qasm(lhs, names)?,
                classical_expr_to_qasm(rhs, names)?
            )
        }
        ClassicalExprKind::Compare { op, lhs, rhs } => {
            let op = match op {
                ClassicalCompareOp::Eq => "==",
                ClassicalCompareOp::Ne => "!=",
                ClassicalCompareOp::Lt => "<",
                ClassicalCompareOp::Le => "<=",
                ClassicalCompareOp::Gt => ">",
                ClassicalCompareOp::Ge => ">=",
            };
            format!(
                "({} {op} {})",
                classical_expr_to_qasm(lhs, names)?,
                classical_expr_to_qasm(rhs, names)?
            )
        }
        ClassicalExprKind::Cast { cast, expr } => match cast {
            ClassicalCast::BitToBool => format!("bool({})", classical_expr_to_qasm(expr, names)?),
            ClassicalCast::BitVecToUInt => {
                format!("uint({})", classical_expr_to_qasm(expr, names)?)
            }
        },
        ClassicalExprKind::ExtractBit { value, index } => {
            format!("{}[{index}]", classical_expr_to_qasm(value, names)?)
        }
        other => {
            return Err(Qasm3DumpError::UnsupportedClassicalControl(format!(
                "classical expression {other:?}"
            )));
        }
    })
}

fn resolve_param(
    circuit_param: &CircuitParam,
    circuit: &Circuit,
    param_map: &HashMap<String, Parameter>,
) -> Parameter {
    let mut param = match circuit_param {
        CircuitParam::Fixed(value) => Parameter::from(*value),
        CircuitParam::Index(index) => circuit.parameters()[*index as usize].clone(),
    };

    for (symbol, replacement) in param_map {
        param = param.replace(symbol, replacement.clone());
    }
    param
}

fn format_params(
    op: &Operation,
    circuit: &Circuit,
    param_map: &HashMap<String, Parameter>,
) -> String {
    if op.params.is_empty() {
        return String::new();
    }
    let params = op
        .params
        .iter()
        .map(|param| {
            resolve_param(param, circuit, param_map)
                .to_string()
                .replace("π", "pi")
        })
        .collect::<Vec<_>>();
    format!("({})", params.join(","))
}

fn map_qubits(op: &Operation, qubit_map: &HashMap<Qubit, String>) -> Vec<String> {
    op.qubits
        .iter()
        .map(|qubit| {
            qubit_map
                .get(qubit)
                .cloned()
                .unwrap_or_else(|| format!("q[{}]", qubit.index()))
        })
        .collect()
}

fn collect_used_standard_gates(operations: &[Operation], used: &mut HashSet<StandardGate>) {
    for op in operations {
        match &op.instruction {
            Instruction::Standard(gate) => {
                used.insert(*gate);
            }
            Instruction::CircuitGate(gate) => {
                collect_used_standard_gates(gate.circuit.circuit.operations(), used);
            }
            Instruction::UnitaryGate(gate) => {
                if let Some(circuit) = gate.circuit() {
                    collect_used_standard_gates(circuit.circuit.operations(), used);
                }
            }
            Instruction::ClassicalControl(control) => match control {
                ClassicalControlOp::If(op) => {
                    collect_used_standard_gates(op.then_body().operations(), used);
                    if let Some(body) = op.else_body() {
                        collect_used_standard_gates(body.operations(), used);
                    }
                }
                ClassicalControlOp::Switch(op) => {
                    for case in op.cases() {
                        collect_used_standard_gates(case.body().operations(), used);
                    }
                    if let Some(body) = op.default() {
                        collect_used_standard_gates(body.operations(), used);
                    }
                }
                ClassicalControlOp::While(op) => {
                    collect_used_standard_gates(op.body().operations(), used)
                }
                ClassicalControlOp::For(op) => {
                    collect_used_standard_gates(op.body().operations(), used)
                }
                ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
            },
            _ => {}
        }
    }
}

fn extension_definition_order(used: &HashSet<StandardGate>) -> Vec<StandardGate> {
    let mut out = Vec::new();
    for gate in [
        StandardGate::X2P,
        StandardGate::X2M,
        StandardGate::Y2P,
        StandardGate::Y2M,
        StandardGate::XY2P,
        StandardGate::XY2M,
        StandardGate::RXX,
        StandardGate::RYY,
        StandardGate::RZZ,
        StandardGate::RZX,
    ] {
        if used.contains(&gate) {
            out.push(gate);
        }
    }
    if used.contains(&StandardGate::FSIM) {
        for gate in [StandardGate::RXX, StandardGate::RYY, StandardGate::RZZ] {
            if !out.contains(&gate) {
                out.push(gate);
            }
        }
        out.push(StandardGate::FSIM);
    }
    out
}

fn extension_gate_definition(gate: StandardGate) -> Option<&'static str> {
    match gate {
        StandardGate::X2P => Some("gate x2p q { rx(pi/2) q; }"),
        StandardGate::X2M => Some("gate x2m q { rx(-pi/2) q; }"),
        StandardGate::Y2P => Some("gate y2p q { ry(pi/2) q; }"),
        StandardGate::Y2M => Some("gate y2m q { ry(-pi/2) q; }"),
        StandardGate::XY2P => Some("gate xy2p(phi) q { rz(-phi) q; x2p q; rz(phi) q; }"),
        StandardGate::XY2M => Some("gate xy2m(phi) q { rz(-phi) q; x2m q; rz(phi) q; }"),
        StandardGate::RXX => {
            Some("gate rxx(theta) a,b { h a; h b; cx a,b; rz(theta) b; cx a,b; h a; h b; }")
        }
        StandardGate::RYY => Some(
            "gate ryy(theta) a,b { rx(pi/2) a; rx(pi/2) b; cx a,b; rz(theta) b; cx a,b; rx(-pi/2) a; rx(-pi/2) b; }",
        ),
        StandardGate::RZZ => Some("gate rzz(theta) a,b { cx a,b; rz(theta) b; cx a,b; }"),
        StandardGate::RZX => Some("gate rzx(theta) a,b { h b; cx a,b; rz(theta) b; cx a,b; h b; }"),
        StandardGate::FSIM => Some(
            "gate fsim(theta,phi) a,b { rxx(theta) a,b; ryy(theta) a,b; gphase(-phi/4); rz(-phi/2) a; rz(-phi/2) b; rzz(phi/2) a,b; }",
        ),
        _ => None,
    }
}

#[cfg(test)]
#[path = "./dump_test.rs"]
mod dump_test;
