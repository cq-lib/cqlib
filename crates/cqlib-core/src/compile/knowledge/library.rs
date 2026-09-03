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

//! Rule library and indexes for compiler knowledge-base rules.
//!
//! A [`RuleLibrary`] owns validated rules, assigns stable [`RuleId`] values,
//! and precomputes the metadata and first-instruction index needed by matchers. DSL
//! parsing remains in `rule_dsl`; this module only delegates to it.

use crate::circuit::Instruction;
use crate::compile::knowledge::matcher::KnowledgeInstructionKey;
use crate::compile::knowledge::rule::{Rule, RuleValidationError};
use crate::compile::knowledge::rule_dsl::load::{
    LoadError, load_rules_from_file, load_rules_from_str,
};
use smallvec::SmallVec;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::OnceLock;

static BUILTIN_RULES: OnceLock<Result<RuleLibrary, RuleLibraryError>> = OnceLock::new();

/// Stable identifier assigned to a rule when it is inserted into a library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuleId(usize);

impl RuleId {
    /// Returns the rule's current library-local index.
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

/// Coarse compiler use-case for a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleKind {
    /// Algebraic simplification that removes redundant structure.
    Simplify,
    /// Cancellation rule for inverse or repeated gates.
    Cancel,
    /// Merge rule for combining compatible neighboring gates.
    Merge,
    /// Explicit commutation rule of the form `A; B -> B; A`.
    Commute,
    /// Decomposition or lowering rule.
    Decompose,
    /// Canonical representation rule.
    Canonicalize,
    /// Rule that rewrites toward a hardware-native instruction set.
    HardwareNative,
    /// Rule without a more specific compiler use-case.
    Other,
}

/// Precomputed metadata used by rule selection and diagnostics.
#[derive(Debug, Clone)]
pub struct RuleMetadata {
    /// Stable id assigned by the containing library.
    pub id: RuleId,
    /// Coarse compiler use-case for this rule.
    pub kind: RuleKind,
    /// Number of operations in the match pattern.
    pub pattern_len: usize,
    /// Number of operations emitted by the rewrite target.
    pub rewrite_len: usize,
    /// Number of distinct rule-local qubit labels used by the rule.
    pub qubit_count: usize,
    /// First instruction in the match pattern.
    pub first_instruction: Instruction,
    /// Static operation-count delta, `rewrite_len - pattern_len`.
    pub cost_delta: isize,
    /// Whether the rule has non-empty parameter conditions.
    pub has_conditions: bool,
}

#[derive(Debug, Clone, Default)]
struct RuleGateMetadata {
    match_instruction_keys: SmallVec<[KnowledgeInstructionKey; 4]>,
    rewrite_instruction_keys: SmallVec<[KnowledgeInstructionKey; 4]>,
}

/// Errors produced while building or extending a rule library.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RuleLibraryError {
    /// A rule failed structural validation.
    #[error("invalid rule {name}: {source}")]
    InvalidRule {
        /// Name of the invalid rule.
        name: String,
        /// Validation failure.
        source: RuleValidationError,
    },
    /// A library cannot contain two rules with the same name.
    #[error("duplicate rule name: {0}")]
    DuplicateRuleName(String),
    /// A rule uses an instruction that cannot be indexed by the matcher.
    #[error("unsupported instruction for rule library index: {instruction}")]
    UnsupportedInstruction { instruction: String },
    /// Loading or parsing rule DSL failed.
    #[error("failed to load rules: {0}")]
    Load(LoadError),
}

/// Validated rule collection with indexes for matcher candidate lookup.
#[derive(Debug, Clone, Default)]
pub struct RuleLibrary {
    rules: Vec<Rule>,
    metadata: Vec<RuleMetadata>,
    gate_metadata: Vec<RuleGateMetadata>,
    name_map: HashMap<String, RuleId>,
    first_instruction_map: Vec<(KnowledgeInstructionKey, SmallVec<[RuleId; 8]>)>,
    kind_map: HashMap<RuleKind, SmallVec<[RuleId; 8]>>,
}

impl RuleLibrary {
    /// Creates an empty rule library.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the lazily loaded builtin compiler rule library.
    ///
    /// Builtin sources are embedded at compile time and validated while the
    /// library is initialized.
    pub fn builtin_rules() -> Result<&'static RuleLibrary, RuleLibraryError> {
        match BUILTIN_RULES.get_or_init(|| {
            let mut library = RuleLibrary::new();

            for (source, kind) in BUILTIN_RULE_SOURCES {
                let rules = load_rules_from_str(source).map_err(RuleLibraryError::Load)?;
                library.extend_rules_with_validation(rules, *kind, false)?;
            }

            Ok(library)
        }) {
            Ok(library) => Ok(library),
            Err(err) => Err(err.clone()),
        }
    }

    /// Builds a library from already constructed rules.
    ///
    /// Rules are structurally validated before insertion.
    pub fn from_rules(rules: Vec<Rule>, kind: RuleKind) -> Result<Self, RuleLibraryError> {
        let mut library = Self::new();
        library.extend_rules(rules, kind)?;
        Ok(library)
    }

    /// Parses and validates rules from a DSL source string.
    pub fn from_dsl_str(source: &str, kind: RuleKind) -> Result<Self, RuleLibraryError> {
        Self::from_rules(
            load_rules_from_str(source).map_err(RuleLibraryError::Load)?,
            kind,
        )
    }

    /// Loads, parses, and validates rules from a DSL file.
    pub fn from_dsl_file(path: impl AsRef<Path>, kind: RuleKind) -> Result<Self, RuleLibraryError> {
        Self::from_rules(
            load_rules_from_file(path).map_err(RuleLibraryError::Load)?,
            kind,
        )
    }

    /// Adds one rule to the library and returns its assigned id.
    ///
    /// When `validate` is `true`, structural validation runs before indexing.
    /// Builtin rule loading may pass `false` after DSL lowering has already
    /// produced trusted validated rules.
    pub fn add_rule(
        &mut self,
        rule: Rule,
        kind: RuleKind,
        validate: bool,
    ) -> Result<RuleId, RuleLibraryError> {
        if validate {
            rule.validate()
                .map_err(|source| RuleLibraryError::InvalidRule {
                    name: rule.name.clone(),
                    source,
                })?;
        }

        if self.name_map.contains_key(&rule.name) {
            return Err(RuleLibraryError::DuplicateRuleName(rule.name));
        }

        let id = RuleId(self.rules.len());
        let first_operation =
            rule.operations
                .first()
                .ok_or_else(|| RuleLibraryError::InvalidRule {
                    name: rule.name.clone(),
                    source: RuleValidationError::EmptyMatch,
                })?;
        let first_instruction_key = KnowledgeInstructionKey::from_instruction(
            &first_operation.instruction,
        )
        .ok_or_else(|| RuleLibraryError::UnsupportedInstruction {
            instruction: first_operation.instruction.to_string(),
        })?;
        let pattern_len = rule.operations.len();
        let rewrite_len = rule.target.len();
        let metadata = RuleMetadata {
            id,
            kind,
            pattern_len,
            rewrite_len,
            qubit_count: rule
                .operations
                .iter()
                .chain(&rule.target)
                .flat_map(|item| item.qubits.iter().copied())
                .collect::<BTreeSet<_>>()
                .len(),
            first_instruction: first_operation.instruction.clone(),
            cost_delta: rewrite_len as isize - pattern_len as isize,
            has_conditions: rule
                .conditions
                .as_ref()
                .is_some_and(|conditions| !conditions.is_empty()),
        };
        let gate_metadata = build_gate_metadata(&rule)?;

        self.name_map.insert(rule.name.clone(), id);
        if let Some((_, ids)) = self
            .first_instruction_map
            .iter_mut()
            .find(|(key, _)| key == &first_instruction_key)
        {
            ids.push(id);
        } else {
            self.first_instruction_map
                .push((first_instruction_key, SmallVec::from_vec(vec![id])));
        }
        self.kind_map.entry(kind).or_default().push(id);
        self.metadata.push(metadata);
        self.gate_metadata.push(gate_metadata);
        self.rules.push(rule);

        Ok(id)
    }

    /// Extends the library with structurally validated rules of one kind.
    ///
    /// The update is atomic with respect to this library: if any rule fails,
    /// the original library is left unchanged.
    pub fn extend_rules(
        &mut self,
        rules: Vec<Rule>,
        kind: RuleKind,
    ) -> Result<SmallVec<[RuleId; 8]>, RuleLibraryError> {
        self.extend_rules_with_validation(rules, kind, true)
    }

    fn extend_rules_with_validation(
        &mut self,
        rules: Vec<Rule>,
        kind: RuleKind,
        validate: bool,
    ) -> Result<SmallVec<[RuleId; 8]>, RuleLibraryError> {
        let mut updated = self.clone();
        let mut ids = SmallVec::new();

        for rule in rules {
            ids.push(updated.add_rule(rule, kind, validate)?);
        }

        *self = updated;
        Ok(ids)
    }

    /// Returns the number of rules in this library.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Returns whether this library contains no rules.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Returns all rules in insertion order.
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Returns a rule by id.
    pub fn get(&self, id: RuleId) -> Option<&Rule> {
        self.rules.get(id.0)
    }

    /// Returns precomputed metadata for a rule id.
    pub fn metadata(&self, id: RuleId) -> Option<&RuleMetadata> {
        self.metadata.get(id.0)
    }

    /// Looks up a rule id by rule name.
    pub fn id_by_name(&self, name: &str) -> Option<RuleId> {
        self.name_map.get(name).copied()
    }

    /// Looks up a rule by rule name.
    pub fn get_by_name(&self, name: &str) -> Option<&Rule> {
        self.id_by_name(name).and_then(|id| self.get(id))
    }

    /// Returns whether a rule with `name` exists.
    pub fn contains(&self, name: &str) -> bool {
        self.name_map.contains_key(name)
    }

    /// Returns rules whose first match instruction has the same matcher key.
    pub fn candidates_for_first_instruction(
        &self,
        instruction: &Instruction,
    ) -> Result<&[RuleId], RuleLibraryError> {
        let key = KnowledgeInstructionKey::from_instruction(instruction).ok_or_else(|| {
            RuleLibraryError::UnsupportedInstruction {
                instruction: instruction.to_string(),
            }
        })?;

        Ok(self
            .first_instruction_map
            .iter()
            .find(|(candidate, _)| candidate == &key)
            .map(|(_, ids)| ids.as_slice())
            .unwrap_or(&[]))
    }

    /// Returns rules registered under a coarse compiler kind.
    pub fn rules_by_kind(&self, kind: RuleKind) -> &[RuleId] {
        self.kind_map
            .get(&kind)
            .map(SmallVec::as_slice)
            .unwrap_or(&[])
    }

    /// Filters rules by required match-side and rewrite-side instruction keys.
    ///
    /// A rule is returned only when every instruction key in its match pattern
    /// is present in `op_instructions` and every rewrite target key is present
    /// in `target_instructions`.
    pub fn filter_rule_ids_by_instruction_keys(
        &self,
        op_instructions: &[Instruction],
        target_instructions: &[Instruction],
    ) -> Result<SmallVec<[RuleId; 16]>, RuleLibraryError> {
        let op_keys = op_instructions
            .iter()
            .map(|instruction| {
                KnowledgeInstructionKey::from_instruction(instruction).ok_or_else(|| {
                    RuleLibraryError::UnsupportedInstruction {
                        instruction: instruction.to_string(),
                    }
                })
            })
            .collect::<Result<SmallVec<[_; 8]>, _>>()?;
        let target_keys = target_instructions
            .iter()
            .map(|instruction| {
                KnowledgeInstructionKey::from_instruction(instruction).ok_or_else(|| {
                    RuleLibraryError::UnsupportedInstruction {
                        instruction: instruction.to_string(),
                    }
                })
            })
            .collect::<Result<SmallVec<[_; 8]>, _>>()?;

        let mut ids = SmallVec::new();

        for (index, metadata) in self.gate_metadata.iter().enumerate() {
            let match_supported = metadata
                .match_instruction_keys
                .iter()
                .all(|required| op_keys.iter().any(|available| available == required));
            let rewrite_supported = metadata
                .rewrite_instruction_keys
                .iter()
                .all(|required| target_keys.iter().any(|available| available == required));

            if match_supported && rewrite_supported {
                ids.push(RuleId(index));
            }
        }

        Ok(ids)
    }
}

fn build_gate_metadata(rule: &Rule) -> Result<RuleGateMetadata, RuleLibraryError> {
    let mut match_instruction_keys = SmallVec::new();
    let mut rewrite_instruction_keys = SmallVec::new();

    for item in &rule.operations {
        match_instruction_keys.push(
            KnowledgeInstructionKey::from_instruction(&item.instruction).ok_or_else(|| {
                RuleLibraryError::UnsupportedInstruction {
                    instruction: item.instruction.to_string(),
                }
            })?,
        );
    }

    for item in &rule.target {
        rewrite_instruction_keys.push(
            KnowledgeInstructionKey::from_instruction(&item.instruction).ok_or_else(|| {
                RuleLibraryError::UnsupportedInstruction {
                    instruction: item.instruction.to_string(),
                }
            })?,
        );
    }

    Ok(RuleGateMetadata {
        match_instruction_keys,
        rewrite_instruction_keys,
    })
}

const BUILTIN_RULE_SOURCES: &[(&str, RuleKind)] = &[
    (include_str!("rules/normalize.rule"), RuleKind::Canonicalize),
    (include_str!("rules/cancel.rule"), RuleKind::Cancel),
    (include_str!("rules/merge.rule"), RuleKind::Merge),
    (include_str!("rules/identity.rule"), RuleKind::Simplify),
    (include_str!("rules/specialize.rule"), RuleKind::Simplify),
    (include_str!("rules/commutation.rule"), RuleKind::Commute),
    (
        include_str!("rules/decompose_ccx.rule"),
        RuleKind::Decompose,
    ),
    (
        include_str!("rules/decompose_controlled_pauli.rule"),
        RuleKind::Decompose,
    ),
    (
        include_str!("rules/decompose_controlled_rotation.rule"),
        RuleKind::Decompose,
    ),
    (
        include_str!("rules/decompose_mc_gate.rule"),
        RuleKind::Decompose,
    ),
    (
        include_str!("rules/decompose_fsim.rule"),
        RuleKind::Decompose,
    ),
    (
        include_str!("rules/decompose_ising.rule"),
        RuleKind::Decompose,
    ),
    (
        include_str!("rules/decompose_phase.rule"),
        RuleKind::Decompose,
    ),
    (
        include_str!("rules/decompose_qcis.rule"),
        RuleKind::Decompose,
    ),
    (
        include_str!("rules/decompose_single_clifford.rule"),
        RuleKind::Decompose,
    ),
    (
        include_str!("rules/decompose_single_rotation.rule"),
        RuleKind::Decompose,
    ),
    (
        include_str!("rules/decompose_swap.rule"),
        RuleKind::Decompose,
    ),
];

#[cfg(test)]
#[path = "./library_test.rs"]
mod library_test;

#[cfg(test)]
#[path = "builtin_rules_contract_test.rs"]
mod builtin_rules_contract_test;
