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

//! Circuit depth calculation via a per-wire ASAP longest-path DP.
//!
//! Every instruction node contributes one layer on the qubits it touches;
//! barriers synchronize their listed qubits (an empty list is a global barrier
//! over all circuit qubits). With `recurse = true`, structured control flow is
//! unfolded into an estimated depth: `if`/`switch` take the max branch, `while`
//! counts the body once, `for` with a statically-known unsigned constant range
//! is fully unrolled (else counted once). `CircuitGate` is opaque (depth 1).
//!
//! The algorithm runs in O(n) without constructing a graph: it keeps a
//! per-qubit frontier depth and, for each operation in program order, advances
//! every synchronized wire to `base + local_depth`.

use crate::circuit::control_flow::{ClassicalControlOp, ControlBody, ForOp, IfOp};
use crate::circuit::error::CircuitError;
use crate::circuit::gate::Instruction;
use crate::circuit::gate::classical_data::ClassicalDataOp;
use crate::circuit::gate::directive::Directive;
use crate::circuit::{Operation, Qubit};
use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};

/// Computes the depth of a circuit's flat operation list.
///
/// `all_qubits` is the enclosing circuit's qubit set; it is consulted only when
/// a global (empty-qubit) barrier appears. When `recurse` is false and any
/// control-flow operation is present at any nesting depth, this returns
/// [`CircuitError::ControlFlowPresent`].
pub(crate) fn circuit_depth(
    all_qubits: impl IntoIterator<Item = Qubit>,
    operations: &[Operation],
    recurse: bool,
) -> Result<usize, CircuitError> {
    if !recurse && contains_control_flow(operations) {
        return Err(CircuitError::ControlFlowPresent);
    }
    let universe: Vec<Qubit> = all_qubits.into_iter().collect();
    operations_depth(operations, &universe, recurse)
}

/// Per-wire ASAP longest-path DP. Returns the depth of `operations`.
///
/// `qubit_universe` is consulted only if a global (empty-qubit) barrier
/// appears, in which case the barrier synchronizes every qubit in the
/// universe.
fn operations_depth(
    operations: &[Operation],
    qubit_universe: &[Qubit],
    recurse: bool,
) -> Result<usize, CircuitError> {
    let mut qubit_depths: HashMap<Qubit, usize> = HashMap::new();
    let mut max_depth = 0usize;
    for operation in operations {
        let (synced, local_depth) =
            operation_sync_and_local_depth(operation, qubit_universe, recurse)?;
        if local_depth == 0 {
            // Store (0 qubits), Break, Continue: no qubit timeline advanced.
            continue;
        }
        let base = synced
            .iter()
            .map(|qubit| qubit_depths.get(qubit).copied().unwrap_or(0))
            .max()
            .unwrap_or(0);
        let depth = base + local_depth;
        for qubit in synced.iter() {
            qubit_depths.insert(*qubit, depth);
        }
        if depth > max_depth {
            max_depth = depth;
        }
    }
    Ok(max_depth)
}

/// Returns `(synchronized_qubits, local_depth)` for a single operation.
///
/// `local_depth` is the depth contributed by this operation alone, NOT
/// including the base depth inherited from its qubits. For control flow it is
/// `1 + body_contribution` (the op is a scheduling boundary).
///
/// The synchronized-qubit slice is borrowed from the operation for the common
/// case. An empty-qubit (global) barrier expands to the full qubit universe,
/// and control-flow ops return an owned union across their bodies.
fn operation_sync_and_local_depth<'a>(
    operation: &'a Operation,
    qubit_universe: &[Qubit],
    recurse: bool,
) -> Result<(Cow<'a, [Qubit]>, usize), CircuitError> {
    match &operation.instruction {
        Instruction::Standard(_)
        | Instruction::McGate(_)
        | Instruction::UnitaryGate(_)
        | Instruction::Delay
        | Instruction::Directive(Directive::Measure | Directive::Reset)
        | Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
        | Instruction::ClassicalData(ClassicalDataOp::MeasureBits { .. }) => {
            Ok((Cow::Borrowed(&operation.qubits[..]), 1))
        }
        Instruction::Directive(Directive::Barrier) => {
            if operation.qubits.is_empty() {
                // Global barrier: synchronize every circuit qubit.
                Ok((Cow::Owned(qubit_universe.to_vec()), 1))
            } else {
                Ok((Cow::Borrowed(&operation.qubits[..]), 1))
            }
        }
        Instruction::CircuitGate(_) => {
            // Opaque: counts as a single layer on its declared qubit args.
            Ok((Cow::Borrowed(&operation.qubits[..]), 1))
        }
        Instruction::ClassicalData(ClassicalDataOp::Store { .. }) => {
            // 0 qubits by construction: no qubit timeline advanced.
            Ok((Cow::Borrowed(&[][..]), 0))
        }
        Instruction::ClassicalControl(control) => {
            if !recurse {
                // Caller is expected to have rejected this via `circuit_depth`,
                // but guard against direct calls into the recursion.
                return Err(CircuitError::ControlFlowPresent);
            }
            control_flow_sync_and_depth(control, qubit_universe, recurse)
                .map(|(qubits, depth)| (Cow::Owned(qubits), depth))
        }
    }
}

/// Resolves a control-flow op into `(owned_synchronized_qubits, local_depth)`.
///
/// The returned qubit slice is the recursive union of qubits touched across all
/// bodies — the op occupies every one of those wires for its full local depth.
/// Ownership is forced into a `Vec<Qubit>` because the union may span multiple
/// bodies and cannot borrow a single `&[Qubit]`.
#[allow(clippy::type_complexity)]
fn control_flow_sync_and_depth(
    control: &ClassicalControlOp,
    qubit_universe: &[Qubit],
    recurse: bool,
) -> Result<(Vec<Qubit>, usize), CircuitError> {
    match control {
        ClassicalControlOp::If(op) => {
            let then_d = control_body_depth(op.then_body(), qubit_universe, recurse)?;
            let else_d = match op.else_body() {
                Some(body) => control_body_depth(body, qubit_universe, recurse)?,
                None => 0,
            };
            let local_depth = 1 + then_d.max(else_d);
            let synced = if_union_qubits(op);
            Ok((synced, local_depth))
        }
        ClassicalControlOp::While(op) => {
            let body_d = control_body_depth(op.body(), qubit_universe, recurse)?;
            let local_depth = 1 + body_d;
            let synced = used_qubits_recursive(op.body().operations());
            Ok((synced.into_iter().collect(), local_depth))
        }
        ClassicalControlOp::For(op) => {
            let body_d = control_body_depth(op.body(), qubit_universe, recurse)?;
            let iterations = for_loop_iterations(op).unwrap_or(1);
            let local_depth = 1 + iterations.saturating_mul(body_d);
            let synced = used_qubits_recursive(op.body().operations());
            Ok((synced.into_iter().collect(), local_depth))
        }
        ClassicalControlOp::Switch(op) => {
            let mut max_depth = 0usize;
            let mut synced = BTreeSet::new();
            for case in op.cases() {
                max_depth =
                    max_depth.max(control_body_depth(case.body(), qubit_universe, recurse)?);
                synced.extend(used_qubits_recursive(case.body().operations()));
            }
            if let Some(default) = op.default() {
                max_depth = max_depth.max(control_body_depth(default, qubit_universe, recurse)?);
                synced.extend(used_qubits_recursive(default.operations()));
            }
            let local_depth = 1 + max_depth;
            Ok((synced.into_iter().collect(), local_depth))
        }
        ClassicalControlOp::Break | ClassicalControlOp::Continue => {
            // Terminal control transfer: no qubit, no gate layer.
            Ok((Vec::new(), 0))
        }
    }
}

/// Depth of a control-flow body, computed as the depth of its operation list.
fn control_body_depth(
    body: &ControlBody,
    qubit_universe: &[Qubit],
    recurse: bool,
) -> Result<usize, CircuitError> {
    operations_depth(body.operations(), qubit_universe, recurse)
}

/// Returns the recursive union of qubits touched by an `if` op's branches.
fn if_union_qubits(op: &IfOp) -> Vec<Qubit> {
    let mut qubits = used_qubits_recursive(op.then_body().operations());
    if let Some(else_body) = op.else_body() {
        qubits.extend(used_qubits_recursive(else_body.operations()));
    }
    qubits.into_iter().collect()
}

/// Returns `Some(iterations)` when the range reduces to unsigned constants and
/// the half-open range `[start, stop)` is well-formed. Runtime-dependent ranges
/// and zero steps return `None`, so the caller counts the body once.
fn for_loop_iterations(op: &ForOp) -> Option<usize> {
    usize::try_from(op.static_iteration_count()?).ok()
}

/// Returns the recursive union of qubits touched by `operations`, descending
/// into nested control-flow bodies. Unlike [`ControlBody::used_qubits`] (which
/// is shallow), this traverses the full nesting tree. `CircuitGate` interiors
/// are not scanned — the gate's own `op.qubits` already represent its
/// interface.
fn used_qubits_recursive(operations: &[Operation]) -> BTreeSet<Qubit> {
    let mut qubits = BTreeSet::new();
    for operation in operations {
        qubits.extend(operation.qubits.iter().copied());
        if let Instruction::ClassicalControl(control) = &operation.instruction {
            for body in control_bodies(control) {
                qubits.extend(used_qubits_recursive(body.operations()));
            }
        }
    }
    qubits
}

/// Borrows the structured bodies of a control-flow op for recursion.
fn control_bodies(control: &ClassicalControlOp) -> Vec<&ControlBody> {
    match control {
        ClassicalControlOp::If(op) => {
            let mut bodies = vec![op.then_body()];
            if let Some(else_body) = op.else_body() {
                bodies.push(else_body);
            }
            bodies
        }
        ClassicalControlOp::While(op) => vec![op.body()],
        ClassicalControlOp::For(op) => vec![op.body()],
        ClassicalControlOp::Switch(op) => {
            let mut bodies: Vec<&ControlBody> = op.cases().iter().map(|case| case.body()).collect();
            if let Some(default) = op.default() {
                bodies.push(default);
            }
            bodies
        }
        ClassicalControlOp::Break | ClassicalControlOp::Continue => Vec::new(),
    }
}

/// Returns true iff `operations` contains any `ClassicalControl` instruction.
/// `CircuitGate` interiors are not scanned (they are opaque sub-circuits, not
/// control flow at this level); control flow nested inside another control
/// flow's body is itself a top-level `ClassicalControl` op in that body, so a
/// shallow scan of each operation list suffices when combined with recursion
/// in [`circuit_depth`]'s caller path (which only invokes this on the top
/// level).
fn contains_control_flow(operations: &[Operation]) -> bool {
    operations
        .iter()
        .any(|operation| matches!(operation.instruction, Instruction::ClassicalControl(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::circuit_impl::Circuit;
    use crate::circuit::gate::Instruction;
    use crate::circuit::{
        CircuitError, ClassicalControlOp, ClassicalExpr, ClassicalType, ClassicalVar, Operation,
        Qubit,
    };
    use smallvec::smallvec;
    use std::slice::from_ref;

    fn q(n: u32) -> Qubit {
        Qubit::new(n)
    }

    #[test]
    fn test_depth_empty_circuit() {
        let c = Circuit::new(3);
        assert_eq!(c.depth(false).unwrap(), 0);
    }

    #[test]
    fn test_depth_single_qubit_chain() {
        let mut c = Circuit::new(1);
        c.h(q(0)).unwrap();
        c.x(q(0)).unwrap();
        c.z(q(0)).unwrap();
        assert_eq!(c.depth(false).unwrap(), 3);
    }

    #[test]
    fn test_depth_parallel_single_qubit() {
        let mut c = Circuit::new(3);
        c.h(q(0)).unwrap();
        c.h(q(1)).unwrap();
        c.h(q(2)).unwrap();
        assert_eq!(c.depth(false).unwrap(), 1);
    }

    #[test]
    fn test_depth_cx_chain() {
        let mut c = Circuit::new(3);
        c.cx(q(0), q(1)).unwrap();
        c.cx(q(1), q(2)).unwrap();
        assert_eq!(c.depth(false).unwrap(), 2);
    }

    #[test]
    fn test_depth_barrier_forces_serialization() {
        let mut c = Circuit::new(2);
        c.h(q(0)).unwrap();
        c.h(q(1)).unwrap();
        c.barrier(vec![q(0), q(1)]).unwrap();
        c.h(q(0)).unwrap();
        c.h(q(1)).unwrap();
        // layer(1) + barrier(1) + layer(1) = 3
        assert_eq!(c.depth(false).unwrap(), 3);
    }

    #[test]
    fn test_depth_global_barrier() {
        let mut c = Circuit::new(2);
        c.h(q(0)).unwrap();
        // Empty-qubit barrier = global, synchronizes q0 and q1.
        c.barrier(Vec::new()).unwrap();
        c.h(q(1)).unwrap();
        // h(0)=1, global barrier advances all wires to 2, h(1) on q1 (was 0) -> 3.
        assert_eq!(c.depth(false).unwrap(), 3);
    }

    #[test]
    fn test_depth_measure_and_reset_count() {
        let mut c = Circuit::new(1);
        c.measure(q(0)).unwrap();
        c.reset(q(0)).unwrap();
        assert_eq!(c.depth(false).unwrap(), 2);
    }

    #[test]
    fn test_depth_store_zero_qubits_noop() {
        let mut c = Circuit::new(1);
        c.h(q(0)).unwrap();
        // A Store op touches 0 qubits: depth 0, does not serialize q0.
        let target = c.var(ClassicalType::uint(8).unwrap());
        c.store(target, ClassicalExpr::uint_literal(8, 0).unwrap())
            .unwrap();
        c.h(q(0)).unwrap();
        assert_eq!(c.depth(false).unwrap(), 2);
    }

    #[test]
    fn test_depth_recurse_false_raises_on_if() {
        let mut c = Circuit::new(2);
        c.if_(ClassicalExpr::bool_literal(true), |body| {
            body.x(q(1))?;
            Ok(())
        })
        .unwrap();
        assert!(matches!(
            c.depth(false),
            Err(CircuitError::ControlFlowPresent)
        ));
    }

    #[test]
    fn test_depth_recurse_false_raises_on_nested_cf() {
        // A for-loop whose body contains an if: recurse=false must still reject.
        let mut c = Circuit::new(1);
        let counter = c.var(ClassicalType::uint(4).unwrap());
        c.for_uint(
            counter,
            ClassicalExpr::uint_literal(4, 0).unwrap(),
            ClassicalExpr::uint_literal(4, 2).unwrap(),
            ClassicalExpr::uint_literal(4, 1).unwrap(),
            |body, _| {
                body.if_(ClassicalExpr::bool_literal(true), |inner| {
                    inner.x(q(0))?;
                    Ok(())
                })?;
                Ok(())
            },
        )
        .unwrap();
        assert!(matches!(
            c.depth(false),
            Err(CircuitError::ControlFlowPresent)
        ));
    }

    #[test]
    fn test_depth_recurse_true_if_else_max() {
        let mut c = Circuit::new(1);
        c.if_else(
            ClassicalExpr::bool_literal(true),
            |then_body| {
                then_body.x(q(0))?;
                then_body.z(q(0))?;
                Ok(())
            },
            |else_body| {
                else_body.y(q(0))?;
                Ok(())
            },
        )
        .unwrap();
        // then depth = 2, else depth = 1 -> 1 + max(2,1) = 3
        assert_eq!(c.depth(true).unwrap(), 3);
    }

    #[test]
    fn test_depth_recurse_true_while_body_once() {
        let mut c = Circuit::new(1);
        c.while_(ClassicalExpr::bool_literal(true), |body| {
            body.h(q(0))?;
            body.x(q(0))?;
            body.break_loop()?;
            Ok(())
        })
        .unwrap();
        // body depth = 2 -> 1 + 2 = 3
        assert_eq!(c.depth(true).unwrap(), 3);
    }

    #[test]
    fn test_depth_recurse_true_for_uint_unrolled() {
        let mut c = Circuit::new(1);
        let counter = c.var(ClassicalType::uint(8).unwrap());
        c.for_uint(
            counter,
            ClassicalExpr::uint_literal(8, 0).unwrap(),
            ClassicalExpr::uint_literal(8, 3).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            |body, _| {
                body.h(q(0))?;
                Ok(())
            },
        )
        .unwrap();
        // body depth = 1, 3 iterations -> 1 + 3*1 = 4
        assert_eq!(c.depth(true).unwrap(), 4);
    }

    #[test]
    fn test_depth_recurse_true_for_var_falls_back_once() {
        let mut c = Circuit::new(1);
        let counter = c.var(ClassicalType::uint(8).unwrap());
        let runtime_start = c.var(ClassicalType::uint(8).unwrap());
        c.for_uint(
            counter,
            runtime_start.expr(),
            ClassicalExpr::uint_literal(8, 3).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            |body, _| {
                body.h(q(0))?;
                Ok(())
            },
        )
        .unwrap();
        // Non-static start -> body counted once -> 1 + 1*1 = 2
        assert_eq!(c.depth(true).unwrap(), 2);
    }

    #[test]
    fn test_depth_recurse_true_for_step_zero_falls_back_once() {
        let mut c = Circuit::new(1);
        let counter = c.var(ClassicalType::uint(8).unwrap());
        c.for_uint(
            counter,
            ClassicalExpr::uint_literal(8, 0).unwrap(),
            ClassicalExpr::uint_literal(8, 3).unwrap(),
            ClassicalExpr::uint_literal(8, 0).unwrap(), // step == 0
            |body, _| {
                body.h(q(0))?;
                Ok(())
            },
        )
        .unwrap();
        // step 0 -> body counted once -> 1 + 1*1 = 2
        assert_eq!(c.depth(true).unwrap(), 2);
    }

    #[test]
    fn test_depth_recurse_true_for_empty_range() {
        let mut c = Circuit::new(1);
        let counter = c.var(ClassicalType::uint(8).unwrap());
        c.for_uint(
            counter,
            ClassicalExpr::uint_literal(8, 5).unwrap(),
            ClassicalExpr::uint_literal(8, 2).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            |body, _| {
                body.h(q(0))?;
                Ok(())
            },
        )
        .unwrap();
        // Empty range -> 0 iterations -> 1 + 0*1 = 1
        assert_eq!(c.depth(true).unwrap(), 1);
    }

    #[test]
    fn test_depth_recurse_true_for_uneven_step() {
        let mut c = Circuit::new(1);
        let counter = c.var(ClassicalType::uint(8).unwrap());
        c.for_uint(
            counter,
            ClassicalExpr::uint_literal(8, 0).unwrap(),
            ClassicalExpr::uint_literal(8, 5).unwrap(),
            ClassicalExpr::uint_literal(8, 2).unwrap(),
            |body, _| {
                body.h(q(0))?;
                Ok(())
            },
        )
        .unwrap();
        // ceil((5-0)/2) = 3 iterations (0,2,4) -> 1 + 3*1 = 4
        assert_eq!(c.depth(true).unwrap(), 4);
    }

    #[test]
    fn test_depth_recurse_true_switch_max() {
        let mut c = Circuit::new(1);
        let state = c.var(ClassicalType::uint(2).unwrap());
        c.switch(state.expr(), |case| {
            case.value(0, |body| {
                body.x(q(0))?;
                Ok(())
            })?;
            case.value(1, |body| {
                body.x(q(0))?;
                body.z(q(0))?;
                Ok(())
            })?;
            case.default(|body| {
                body.y(q(0))?;
                Ok(())
            })?;
            Ok(())
        })
        .unwrap();
        // case depths: 1, 2; default: 1 -> 1 + max(1,2,1) = 3
        assert_eq!(c.depth(true).unwrap(), 3);
    }

    #[test]
    fn test_depth_recurse_true_break_continue_zero() {
        let mut c = Circuit::new(1);
        c.while_(ClassicalExpr::bool_literal(true), |body| {
            body.break_loop()?;
            Ok(())
        })
        .unwrap();
        // break is depth 0 -> 1 + 0 = 1
        assert_eq!(c.depth(true).unwrap(), 1);
    }

    #[test]
    fn test_depth_recurse_true_nested_if_in_for() {
        let mut c = Circuit::new(1);
        let counter = c.var(ClassicalType::uint(8).unwrap());
        c.for_uint(
            counter,
            ClassicalExpr::uint_literal(8, 0).unwrap(),
            ClassicalExpr::uint_literal(8, 2).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            |body, _| {
                body.if_(ClassicalExpr::bool_literal(true), |inner| {
                    inner.x(q(0))?;
                    inner.y(q(0))?;
                    Ok(())
                })?;
                Ok(())
            },
        )
        .unwrap();
        // if body depth = 2, if local = 1+2 = 3; for body depth = 3;
        // 2 iterations -> 1 + 2*3 = 7
        assert_eq!(c.depth(true).unwrap(), 7);
    }

    #[test]
    fn test_depth_cf_synchronizes_union_qubits() {
        // if(then: cx(0,1), else: cx(2,3)) occupies {0,1,2,3}; a following
        // h(0) must wait for the if to complete, so total depth is if_local + 1.
        let mut c = Circuit::new(4);
        c.if_else(
            ClassicalExpr::bool_literal(true),
            |then_body| {
                then_body.cx(q(0), q(1))?;
                Ok(())
            },
            |else_body| {
                else_body.cx(q(2), q(3))?;
                Ok(())
            },
        )
        .unwrap();
        c.h(q(0)).unwrap();
        // if local = 1 + max(1,1) = 2; h(0) waits on q0 (depth 2) -> 3.
        assert_eq!(c.depth(true).unwrap(), 3);
    }

    #[test]
    fn test_depth_for_loop_iterations_helper() {
        // Directly exercise for_loop_iterations via a constructed ForOp.
        let mk = |start: u128, stop: u128, step: u128| {
            let var = ClassicalVar::new(
                crate::circuit::CircuitId::new(),
                0,
                ClassicalType::uint(8).unwrap(),
            );
            ForOp::new(
                var,
                ClassicalExpr::uint_literal(8, start).unwrap(),
                ClassicalExpr::uint_literal(8, stop).unwrap(),
                ClassicalExpr::uint_literal(8, step).unwrap(),
                crate::circuit::ControlBody::new(vec![]),
            )
            .unwrap()
        };
        assert_eq!(for_loop_iterations(&mk(0, 3, 1)), Some(3));
        assert_eq!(for_loop_iterations(&mk(0, 5, 2)), Some(3)); // 0,2,4
        assert_eq!(for_loop_iterations(&mk(5, 2, 1)), Some(0)); // empty
        assert_eq!(for_loop_iterations(&mk(5, 0, 1)), Some(0)); // descending with +step is empty
        assert_eq!(for_loop_iterations(&mk(0, 3, 0)), None); // step 0
    }

    #[test]
    fn test_depth_circuit_gate_opaque() {
        // Build a 2-gate sub-circuit, wrap it as a CircuitGate, append on 2
        // qubits. depth must treat it as opaque (depth 1), not 2.
        let mut inner = Circuit::new(2);
        inner.h(q(0)).unwrap();
        inner.cx(q(0), q(1)).unwrap();
        let gate_instruction = inner.to_gate("mygate").unwrap();

        let mut c = Circuit::new(2);
        c.append(gate_instruction, [q(0), q(1)], [], None).unwrap();
        assert_eq!(c.depth(false).unwrap(), 1);
    }

    #[test]
    fn test_contains_control_flow_helper() {
        let mk_cf = |op: ClassicalControlOp| Operation {
            instruction: Instruction::ClassicalControl(op),
            qubits: smallvec![],
            params: smallvec![],
            label: None,
        };
        let plain = Operation {
            instruction: crate::circuit::gate::Instruction::Standard(
                crate::circuit::gate::StandardGate::H,
            ),
            qubits: smallvec![q(0)],
            params: smallvec![],
            label: None,
        };
        assert!(!contains_control_flow(from_ref(&plain)));

        let if_op = IfOp::new(
            ClassicalExpr::bool_literal(true),
            crate::circuit::ControlBody::new(vec![]),
            None,
        )
        .unwrap();
        assert!(contains_control_flow(&[mk_cf(ClassicalControlOp::If(
            if_op
        ))]));
    }
}
