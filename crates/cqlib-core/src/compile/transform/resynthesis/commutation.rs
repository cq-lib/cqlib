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

//! Local commutation adapter for numeric block resynthesis.
//!
//! The compiler-wide [`CommutationChecker`] remains the semantic proof engine.
//! This adapter adds the operation-level shape and per-pass cache needed by
//! resynthesis collectors, without changing the public commutation API.

use crate::circuit::{Operation, Parameter, ValueInstruction, ValueOperation};
use crate::compile::commutation::{CommutationChecker, CommutationConfig};
use smallvec::SmallVec;
use std::collections::HashMap;

/// Source operation plus parameters resolved against its owning circuit.
pub(crate) struct OperationView<'a> {
    /// Stable source-operation order within the pass-local operation stream.
    pub(crate) order: usize,
    /// Source operation.
    pub(crate) operation: &'a Operation,
    /// Resolved parameters.
    ///
    /// Every [`crate::circuit::CircuitParam::Index`] has already been expanded
    /// from the source circuit parameter table. This slice is ready to pass
    /// directly to [`CommutationChecker::check`].
    pub(crate) params: SmallVec<[Parameter; 3]>,
}

impl<'a> OperationView<'a> {
    pub(crate) fn new(
        order: usize,
        operation: &'a Operation,
        params: SmallVec<[Parameter; 3]>,
    ) -> Self {
        Self {
            order,
            operation,
            params,
        }
    }
}

/// Pass-local cached exact commutation queries for source operations.
pub(crate) struct CachedCommutation {
    checker: CommutationChecker,
    cache: HashMap<(usize, usize), bool>,
    exact_only: bool,
}

impl CachedCommutation {
    /// Builds a cached adapter around a configured commutation checker.
    pub(crate) fn new(config: CommutationConfig) -> Self {
        Self {
            checker: CommutationChecker::with_config(config),
            cache: HashMap::new(),
            exact_only: true,
        }
    }

    /// Returns whether two source operations commute under the adapter policy.
    ///
    /// Results are cached by normalized source order. Replacement operations are
    /// intentionally excluded from this cache because they do not have stable
    /// source-order identities.
    pub(crate) fn commute_ops(&mut self, lhs: &OperationView<'_>, rhs: &OperationView<'_>) -> bool {
        if lhs.order == rhs.order {
            return true;
        }

        let key = if lhs.order <= rhs.order {
            (lhs.order, rhs.order)
        } else {
            (rhs.order, lhs.order)
        };
        if let Some(result) = self.cache.get(&key) {
            return *result;
        }

        let result = self
            .checker
            .check(
                &lhs.operation.instruction,
                &lhs.operation.qubits,
                &lhs.params,
                &rhs.operation.instruction,
                &rhs.operation.qubits,
                &rhs.params,
            )
            .is_some_and(|commutation| !self.exact_only || commutation.is_exact());

        self.cache.insert(key, result);
        result
    }

    /// Verifies that every replacement can remain at the patch site while
    /// crossed source operations stay around it.
    ///
    /// This is a post-synthesis safety check: proving a matched source
    /// operation can cross a skipped operation is not enough once the matched
    /// operations have been replaced by a newly synthesized sequence.
    pub(crate) fn replacements_commute_with_crossed(
        &self,
        crossed: &[&OperationView<'_>],
        replacements: &[ValueOperation],
    ) -> bool {
        for crossed_op in crossed {
            for replacement in replacements {
                if !crossed_op
                    .operation
                    .qubits
                    .iter()
                    .any(|qubit| replacement.qubits.contains(qubit))
                {
                    continue;
                }
                if !self.replacement_commutes_with_op(crossed_op, replacement) {
                    return false;
                }
            }
        }
        true
    }

    pub(crate) fn commute_ops_skip_cache(
        &self,
        lhs: &OperationView<'_>,
        rhs: &OperationView<'_>,
    ) -> bool {
        if lhs.order == rhs.order {
            return true;
        }
        self.checker
            .check(
                &lhs.operation.instruction,
                &lhs.operation.qubits,
                &lhs.params,
                &rhs.operation.instruction,
                &rhs.operation.qubits,
                &rhs.params,
            )
            .is_some_and(|commutation| !self.exact_only || commutation.is_exact())
    }

    /// Rechecks an exact source crossing while reusing a collector proof when
    /// one is already cached. A forged or independently materialized block
    /// still falls back to the exact checker.
    pub(crate) fn commute_ops_rechecked(
        &self,
        lhs: &OperationView<'_>,
        rhs: &OperationView<'_>,
    ) -> bool {
        if lhs.order == rhs.order {
            return true;
        }
        let key = if lhs.order <= rhs.order {
            (lhs.order, rhs.order)
        } else {
            (rhs.order, lhs.order)
        };
        self.cache
            .get(&key)
            .copied()
            .unwrap_or_else(|| self.commute_ops_skip_cache(lhs, rhs))
    }

    fn replacement_commutes_with_op(
        &self,
        operation: &OperationView<'_>,
        replacement: &ValueOperation,
    ) -> bool {
        let replacement_instruction = match &replacement.instruction {
            ValueInstruction::Instruction(instruction) => instruction,
            ValueInstruction::ClassicalControl(_) => return false,
        };

        let replacement_params = replacement
            .params
            .iter()
            .map(Parameter::from)
            .collect::<SmallVec<[Parameter; 3]>>();

        self.checker
            .check(
                &operation.operation.instruction,
                &operation.operation.qubits,
                &operation.params,
                replacement_instruction,
                &replacement.qubits,
                &replacement_params,
            )
            .is_some_and(|commutation| !self.exact_only || commutation.is_exact())
    }

    #[cfg(test)]
    pub(super) fn cache_len(&self) -> usize {
        self.cache.len()
    }
}

#[cfg(test)]
#[path = "commutation_test.rs"]
mod commutation_test;
