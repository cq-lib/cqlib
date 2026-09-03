// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Serialization helpers for writing [`Rule`] or [`RuleDef`] back to the DSL text format.

use crate::circuit::{Instruction, MCGate, Parameter, ParameterValue};
use crate::compile::knowledge::rule::{Condition, Rule, RuleItem};
use crate::compile::knowledge::rule_dsl::ast::{GatePattern, RuleDef};
use std::fmt::Write;
use std::path::Path;

/// Serializes a [`Rule`] into a `.rule` formatted string.
pub fn dump_rule_to_string(rule: &Rule) -> String {
    let mut buf = String::new();
    write_rule(&mut buf, rule).expect("writing to String cannot fail");
    buf
}

/// Serializes a [`RuleDef`] into a `.rule` formatted string.
pub fn dump_rule_def_to_string(rule: &RuleDef) -> String {
    let mut buf = String::new();
    write_rule_def(&mut buf, rule).expect("writing to String cannot fail");
    buf
}

/// Writes a single [`Rule`] to the given file path.
pub fn dump_rule_to_file(rule: &Rule, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
    std::fs::write(path.as_ref(), dump_rule_to_string(rule).as_bytes())
}

/// Writes multiple [`Rule`]s to the given file path, separated by blank lines.
pub fn dump_rules_to_file(rules: &[Rule], path: impl AsRef<Path>) -> Result<(), std::io::Error> {
    let mut buf = String::new();
    for (i, rule) in rules.iter().enumerate() {
        if i > 0 {
            buf.push('\n');
        }
        write_rule(&mut buf, rule).expect("writing to String cannot fail");
    }
    std::fs::write(path.as_ref(), buf.as_bytes())
}

fn write_rule(w: &mut impl Write, rule: &Rule) -> std::fmt::Result {
    writeln!(w, "rule {} {{", rule.name)?;

    writeln!(w, "    match {{")?;
    for op in &rule.operations {
        write!(w, "        ")?;
        write_rule_item(w, op)?;
        writeln!(w)?;
    }
    writeln!(w, "    }}")?;

    if let Some(conditions) = &rule.conditions
        && !conditions.is_empty()
    {
        writeln!(w, "    require {{")?;
        for cond in conditions {
            write!(w, "        ")?;
            write_condition(w, cond)?;
            writeln!(w)?;
        }
        writeln!(w, "    }}")?;
    }

    writeln!(w, "    rewrite {{")?;
    for op in &rule.target {
        write!(w, "        ")?;
        write_rule_item(w, op)?;
        writeln!(w)?;
    }
    writeln!(w, "    }}")?;

    writeln!(w, "}}")
}

fn write_rule_def(w: &mut impl Write, rule: &RuleDef) -> std::fmt::Result {
    writeln!(w, "rule {} {{", rule.name)?;

    writeln!(w, "    match {{")?;
    for pat in &rule.match_ops {
        write!(w, "        ")?;
        write_gate_pattern(w, pat)?;
        writeln!(w)?;
    }
    writeln!(w, "    }}")?;

    if !rule.conditions.is_empty() {
        writeln!(w, "    require {{")?;
        for cond in &rule.conditions {
            write!(w, "        ")?;
            write_condition(w, cond)?;
            writeln!(w)?;
        }
        writeln!(w, "    }}")?;
    }

    writeln!(w, "    rewrite {{")?;
    for pat in &rule.rewrite_ops {
        write!(w, "        ")?;
        write_gate_pattern(w, pat)?;
        writeln!(w)?;
    }
    writeln!(w, "    }}")?;

    writeln!(w, "}}")
}

fn write_gate_pattern(w: &mut impl Write, pat: &GatePattern) -> std::fmt::Result {
    write!(w, "{}", pat.gate.display_name())?;
    if !pat.params.is_empty() {
        write!(w, "(")?;
        for (i, p) in pat.params.iter().enumerate() {
            if i > 0 {
                write!(w, ", ")?;
            }
            write!(w, "{}", p)?;
        }
        write!(w, ")")?;
    }
    for q in &pat.qubits {
        write!(w, " {}", q)?;
    }
    Ok(())
}

fn write_condition(w: &mut impl Write, cond: &Condition) -> std::fmt::Result {
    match cond {
        Condition::Eq(lhs, rhs) => write!(w, "{} == {}", lhs, rhs),
        Condition::EqMod(lhs, rhs, modulus) => write!(w, "{} == {} mod {}", lhs, rhs, modulus),
    }
}

fn write_rule_item(w: &mut impl Write, op: &RuleItem) -> std::fmt::Result {
    let gate_name = match &op.instruction {
        Instruction::Standard(gate) => gate.to_string(),
        Instruction::McGate(gate) => mc_gate_name(gate),
        _ => unreachable!("unsupported instruction cannot appear in a RuleItem"),
    };
    write!(w, "{}", gate_name)?;
    if let Some(params) = &op.params
        && !params.is_empty()
    {
        write!(w, "(")?;
        for (i, pp) in params.iter().enumerate() {
            if i > 0 {
                write!(w, ", ")?;
            }
            let p = match pp {
                ParameterValue::Fixed(v) => Parameter::from(*v),
                ParameterValue::Param(p) => p.clone(),
            };
            write!(w, "{}", p)?;
        }
        write!(w, ")")?;
    }
    for q in &op.qubits {
        write!(w, " {}", q)?;
    }
    Ok(())
}

fn mc_gate_name(gate: &MCGate) -> String {
    let added_controls = gate.num_qubits() - gate.base_gate().num_qubits();
    format!("MC{}[{added_controls}]", gate.base_gate())
}

#[cfg(test)]
#[path = "./dump_test.rs"]
mod dump_test;
