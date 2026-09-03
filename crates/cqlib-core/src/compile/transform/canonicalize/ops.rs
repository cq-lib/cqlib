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

//! Operation-level canonicalization helpers.

use crate::circuit::{
    ClassicalDataOp, ClassicalExprKind, Directive, Instruction, Operation, Parameter, Qubit,
    StandardGate,
};
use crate::compile::CompilerError;
use smallvec::SmallVec;

use super::config::CanonicalizeConfig;

pub(super) fn is_strict_noop(
    instruction: &Instruction,
    params: &[Parameter],
) -> Result<bool, CompilerError> {
    Ok(match instruction {
        Instruction::Standard(StandardGate::I) => true,
        Instruction::Standard(StandardGate::GPhase) => match params.first() {
            Some(param) => parameter_is_exact_zero(param)?,
            None => false,
        },
        Instruction::Delay => match params.first() {
            Some(param) => parameter_is_exact_zero(param)?,
            None => false,
        },
        Instruction::Standard(
            StandardGate::RX
            | StandardGate::RY
            | StandardGate::RZ
            | StandardGate::Phase
            | StandardGate::RXX
            | StandardGate::RYY
            | StandardGate::RZZ
            | StandardGate::RZX
            | StandardGate::CRX
            | StandardGate::CRY
            | StandardGate::CRZ,
        ) => match params.first() {
            Some(param) => parameter_is_exact_zero(param)?,
            None => false,
        },
        Instruction::Standard(StandardGate::RXY) => match params.first() {
            Some(param) => parameter_is_exact_zero(param)?,
            None => false,
        },
        Instruction::Standard(StandardGate::FSIM) => {
            params.len() == 2
                && parameter_is_exact_zero(&params[0])?
                && parameter_is_exact_zero(&params[1])?
        }
        Instruction::Standard(StandardGate::U) => {
            params.len() == 3
                && parameter_is_exact_zero(&params[0])?
                && parameter_is_exact_zero(&params[1])?
                && parameter_is_exact_zero(&params[2])?
        }
        Instruction::ClassicalData(ClassicalDataOp::Store { target, value }) => {
            matches!(value.kind(), ClassicalExprKind::Var(v) if *v == *target)
        }
        _ => false,
    })
}

pub(super) fn parameter_is_exact_zero(param: &Parameter) -> Result<bool, CompilerError> {
    param.is_exact_zero().map_err(|error| {
        CompilerError::InvalidInput(format!("parameter cannot be evaluated: {error}"))
    })
}

pub(super) fn canonicalize_operation_qubits(
    instruction: &Instruction,
    qubits: &SmallVec<[Qubit; 3]>,
    config: &CanonicalizeConfig,
) -> SmallVec<[Qubit; 3]> {
    if !config.canonicalizes_barriers()
        || !matches!(instruction, Instruction::Directive(Directive::Barrier))
    {
        return qubits.clone();
    }

    // A non-empty barrier scope is a set for canonicalization purposes. An
    // empty scope is the global-barrier sentinel and must remain empty.
    // Sorting by the stable qubit id gives deterministic output independent of
    // construction order, and deduplication removes redundant synchronization
    // operands.
    let mut out = qubits.clone();
    out.sort_unstable_by_key(|qubit| qubit.id());
    out.dedup();
    out
}

pub(super) fn push_operation(
    out: &mut Vec<Operation>,
    mut operation: Operation,
    config: &CanonicalizeConfig,
) {
    if !config.canonicalizes_barriers()
        || !matches!(
            operation.instruction,
            Instruction::Directive(Directive::Barrier)
        )
    {
        out.push(operation);
        return;
    }

    operation.label = None;
    if let Some(last) = out.last_mut()
        && matches!(last.instruction, Instruction::Directive(Directive::Barrier))
    {
        // Adjacent barriers are a single synchronization boundary whenever
        // one scope covers the other. An empty scope is global and therefore
        // covers every local scope. Partial local overlap is deliberately not
        // merged because neither barrier fully subsumes the other.
        match barrier_relation(&last.qubits, &operation.qubits) {
            BarrierRelation::Equal | BarrierRelation::LeftSuperset => {
                last.label = None;
                return;
            }
            BarrierRelation::RightSuperset => {
                *last = operation;
                last.label = None;
                return;
            }
            BarrierRelation::DisjointOrOverlap => {}
        }
    }
    out.push(operation);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BarrierRelation {
    Equal,
    LeftSuperset,
    RightSuperset,
    DisjointOrOverlap,
}

pub(super) fn barrier_relation(lhs: &[Qubit], rhs: &[Qubit]) -> BarrierRelation {
    // Empty barrier operands represent the full circuit qubit universe, not
    // the empty set. Handle that sentinel before ordinary set comparison.
    match (lhs.is_empty(), rhs.is_empty()) {
        (true, true) => return BarrierRelation::Equal,
        (true, false) => return BarrierRelation::LeftSuperset,
        (false, true) => return BarrierRelation::RightSuperset,
        (false, false) => {}
    }

    if lhs == rhs {
        return BarrierRelation::Equal;
    }

    let lhs_contains_rhs = rhs.iter().all(|qubit| lhs.contains(qubit));
    let rhs_contains_lhs = lhs.iter().all(|qubit| rhs.contains(qubit));
    match (lhs_contains_rhs, rhs_contains_lhs) {
        (true, false) => BarrierRelation::LeftSuperset,
        (false, true) => BarrierRelation::RightSuperset,
        _ => BarrierRelation::DisjointOrOverlap,
    }
}
