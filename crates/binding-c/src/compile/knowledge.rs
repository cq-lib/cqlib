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

//! C ABI for the compiler knowledge base: rule-library construction (empty,
//! builtin, DSL text, DSL file), single-rule parsing, rule insertion, rule
//! metadata queries, and the matching engine (transactional pattern
//! matching, condition checks, item equivalence, candidate and
//! instruction-key filtering, rule extension, and rewrite-target
//! instantiation).

use crate::circuit::COperation;
use crate::device::standard_gate_from_name;
use crate::error::CqlibError;
use cqlib_core::circuit::{CircuitParam, Instruction, Parameter, ParameterValue};
use cqlib_core::compile::knowledge::matcher::{
    ConcreteOperationView, MatchBindings, MatchError, MatchedReplacement, conditions_hold,
    instantiate_target, match_rule_item,
};
use cqlib_core::compile::knowledge::rule::Rule;
use cqlib_core::compile::knowledge::rule_dsl::load::LoadError;
use cqlib_core::compile::knowledge::{RuleKind, RuleLibrary, RuleLibraryError, RuleMetadata};
use std::collections::HashSet;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Rule-kind tag.
/// | Value | Kind           |
/// |-------|----------------|
/// | 0     | Simplify       |
/// | 1     | Cancel         |
/// | 2     | Merge          |
/// | 3     | Commute        |
/// | 4     | Decompose      |
/// | 5     | Canonicalize   |
/// | 6     | HardwareNative |
/// | 7     | Other          |
pub const RULE_KIND_SIMPLIFY: u8 = 0;
pub const RULE_KIND_CANCEL: u8 = 1;
pub const RULE_KIND_MERGE: u8 = 2;
pub const RULE_KIND_COMMUTE: u8 = 3;
pub const RULE_KIND_DECOMPOSE: u8 = 4;
pub const RULE_KIND_CANONICALIZE: u8 = 5;
pub const RULE_KIND_HARDWARE_NATIVE: u8 = 6;
pub const RULE_KIND_OTHER: u8 = 7;

/// Opaque handle around a [`RuleLibrary`].
pub struct CKnowledgeLibrary {
    pub inner: RuleLibrary,
}

/// Opaque handle around a single knowledge [`Rule`].
///
/// Handles are produced by [`knowledge_rule_from_dsl`] and consumed by
/// [`knowledge_library_add_rule`].
pub struct CKnowledgeRule {
    pub inner: Rule,
}

/// Opaque handle around the mutable bindings produced while matching a rule
/// instance (`MatchBindings`).
pub struct CKnowledgeBindings {
    pub inner: MatchBindings,
}

/// Opaque handle around the rewrite-target instantiation result (a list of
/// `MatchedReplacement` items).
pub struct CKnowledgeReplacements {
    pub inner: Vec<MatchedReplacement>,
}

// =====  Kind and error mapping helpers  =====

/// Converts a `RULE_KIND_*` tag into the core form.
fn rule_kind_from_tag(tag: u8) -> Option<RuleKind> {
    Some(match tag {
        RULE_KIND_SIMPLIFY => RuleKind::Simplify,
        RULE_KIND_CANCEL => RuleKind::Cancel,
        RULE_KIND_MERGE => RuleKind::Merge,
        RULE_KIND_COMMUTE => RuleKind::Commute,
        RULE_KIND_DECOMPOSE => RuleKind::Decompose,
        RULE_KIND_CANONICALIZE => RuleKind::Canonicalize,
        RULE_KIND_HARDWARE_NATIVE => RuleKind::HardwareNative,
        RULE_KIND_OTHER => RuleKind::Other,
        _ => return None,
    })
}

/// Converts a core rule kind into its `RULE_KIND_*` tag.
fn rule_kind_to_tag(kind: RuleKind) -> u8 {
    match kind {
        RuleKind::Simplify => RULE_KIND_SIMPLIFY,
        RuleKind::Cancel => RULE_KIND_CANCEL,
        RuleKind::Merge => RULE_KIND_MERGE,
        RuleKind::Commute => RULE_KIND_COMMUTE,
        RuleKind::Decompose => RULE_KIND_DECOMPOSE,
        RuleKind::Canonicalize => RULE_KIND_CANONICALIZE,
        RuleKind::HardwareNative => RULE_KIND_HARDWARE_NATIVE,
        RuleKind::Other => RULE_KIND_OTHER,
    }
}

/// Maps a rule-library error onto the C error codes.
///
/// DSL parse/lowering failures, duplicate rule names, and structural
/// validation failures map to -4; file-level I/O failures map to -5;
/// instructions that cannot be indexed by the matcher map to -6.
fn library_error_code(error: &RuleLibraryError) -> i32 {
    match error {
        RuleLibraryError::Load(LoadError::Io(_)) => CqlibError::IoError as i32,
        RuleLibraryError::Load(_)
        | RuleLibraryError::InvalidRule { .. }
        | RuleLibraryError::DuplicateRuleName(_) => CqlibError::ParseError as i32,
        RuleLibraryError::UnsupportedInstruction { .. } => CqlibError::CompilerError as i32,
    }
}

/// Resolves the precomputed metadata of the rule at `index`.
///
/// Rule ids equal the insertion index and rule names are unique, so the
/// metadata is recovered through the rule's name.
fn metadata_for_index(library: &RuleLibrary, index: usize) -> Option<&RuleMetadata> {
    let name = library.rules().get(index)?.name.as_str();
    let id = library.id_by_name(name)?;
    library.metadata(id)
}

// =====  Library construction  =====

/// Creates an empty rule library. Free with `knowledge_library_free`.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_new() -> *mut CKnowledgeLibrary {
    Box::into_raw(Box::new(CKnowledgeLibrary {
        inner: RuleLibrary::new(),
    }))
}

/// Creates a library containing the builtin compiler rules. Free with
/// `knowledge_library_free`. Returns NULL when the builtin sources fail to
/// validate.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_builtin() -> *mut CKnowledgeLibrary {
    match RuleLibrary::builtin_rules() {
        Ok(library) => Box::into_raw(Box::new(CKnowledgeLibrary {
            inner: library.clone(),
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Parses and validates rules from a DSL source string.
///
/// `kind` is one of `RULE_KIND_*` and classifies every parsed rule. On success
/// writes a heap-allocated `CKnowledgeLibrary*` (free with
/// `knowledge_library_free`) to `out` and returns 0. Returns -1 on NULL input,
/// -8 on an unknown kind tag, and -4 on DSL parse, lowering, or validation
/// failures.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_from_dsl_str(
    source: *const c_char,
    kind: u8,
    out: *mut *mut CKnowledgeLibrary,
) -> i32 {
    if source.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let Some(kind) = rule_kind_from_tag(kind) else {
        return CqlibError::InvalidParam as i32;
    };
    let source = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(source) => source,
        Err(_) => return CqlibError::ParseError as i32,
    };
    match RuleLibrary::from_dsl_str(source, kind) {
        Ok(inner) => {
            unsafe { *out = Box::into_raw(Box::new(CKnowledgeLibrary { inner })) };
            CqlibError::Ok as i32
        }
        Err(err) => library_error_code(&err),
    }
}

/// Loads, parses, and validates rules from a DSL file.
///
/// `kind` is one of `RULE_KIND_*`. On success writes a heap-allocated
/// `CKnowledgeLibrary*` (free with `knowledge_library_free`) to `out` and
/// returns 0. Returns -1 on NULL input, -8 on an unknown kind tag, -4 on DSL
/// parse, lowering, or validation failures, and -5 when the file cannot be
/// read.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_from_dsl_file(
    path: *const c_char,
    kind: u8,
    out: *mut *mut CKnowledgeLibrary,
) -> i32 {
    if path.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let Some(kind) = rule_kind_from_tag(kind) else {
        return CqlibError::InvalidParam as i32;
    };
    let path = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(path) => path,
        Err(_) => return CqlibError::ParseError as i32,
    };
    match RuleLibrary::from_dsl_file(path, kind) {
        Ok(inner) => {
            unsafe { *out = Box::into_raw(Box::new(CKnowledgeLibrary { inner })) };
            CqlibError::Ok as i32
        }
        Err(err) => library_error_code(&err),
    }
}

/// Frees a rule library. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_free(ptr: *mut CKnowledgeLibrary) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

// =====  Library queries  =====

/// Returns the number of rules in the library, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_len(ptr: *const CKnowledgeLibrary) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.len() }
}

/// Returns 1 when the library contains no rules, 0 otherwise, or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_is_empty(ptr: *const CKnowledgeLibrary) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.is_empty() as i32 }
}

/// Returns 1 when a rule with the given name exists, 0 otherwise, or -1 on
/// NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_contains(
    ptr: *const CKnowledgeLibrary,
    name: *const c_char,
) -> i32 {
    if ptr.is_null() || name.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(name) => name,
        Err(_) => return CqlibError::ParseError as i32,
    };
    unsafe { (*ptr).inner.contains(name) as i32 }
}

/// Looks up the rule id (insertion index) of `name`.
///
/// Returns the id as a non-negative value, or -1 on NULL input or when no
/// rule with that name exists.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_id_by_name(
    ptr: *const CKnowledgeLibrary,
    name: *const c_char,
) -> i64 {
    if ptr.is_null() || name.is_null() {
        return CqlibError::NullPtr as i32 as i64;
    }
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(name) => name,
        Err(_) => return CqlibError::ParseError as i32 as i64,
    };
    match unsafe { (*ptr).inner.id_by_name(name) } {
        Some(id) => id.as_usize() as i64,
        None => -1,
    }
}

/// Returns the number of rules registered under `kind` (one of
/// `RULE_KIND_*`), or 0 for NULL input or an unknown kind tag.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_rules_by_kind_len(
    ptr: *const CKnowledgeLibrary,
    kind: u8,
) -> usize {
    let Some(kind) = rule_kind_from_tag(kind) else {
        return 0;
    };
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.rules_by_kind(kind).len() }
}

/// Copies the rule ids registered under `kind` into `out` (two-step pattern;
/// call `knowledge_library_rules_by_kind_len` first). Returns the total number
/// of rules of that kind, or 0 for NULL input or an unknown kind tag.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_rules_by_kind(
    ptr: *const CKnowledgeLibrary,
    kind: u8,
    out: *mut u32,
    len: usize,
) -> usize {
    let Some(kind) = rule_kind_from_tag(kind) else {
        return 0;
    };
    if ptr.is_null() {
        return 0;
    }
    let ids: Vec<u32> = unsafe { (*ptr).inner.rules_by_kind(kind) }
        .iter()
        .map(|id| id.as_usize() as u32)
        .collect();
    crate::device::write_u32_list(&ids, out, len)
}

// =====  Rule metadata accessors (by insertion index)  =====

/// Returns the name of the rule at `index` as a heap-allocated C string (free
/// with `cqlib_string_free`). Returns NULL on NULL input or an out-of-bounds
/// index.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_rule_name(
    ptr: *const CKnowledgeLibrary,
    index: usize,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { (*ptr).inner.rules() }.get(index) {
        Some(rule) => match CString::new(rule.name.as_str()) {
            Ok(name) => name.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Returns the kind tag (one of `RULE_KIND_*`) of the rule at `index`, or -1
/// on NULL input and -8 on an out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_rule_kind(ptr: *const CKnowledgeLibrary, index: usize) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    match metadata_for_index(unsafe { &(*ptr).inner }, index) {
        Some(metadata) => rule_kind_to_tag(metadata.kind) as i32,
        None => CqlibError::InvalidParam as i32,
    }
}

/// Returns the instruction name starting the rule's match pattern as a
/// heap-allocated C string (free with `cqlib_string_free`). Returns NULL on
/// NULL input or an out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_rule_first_instruction(
    ptr: *const CKnowledgeLibrary,
    index: usize,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match metadata_for_index(unsafe { &(*ptr).inner }, index) {
        Some(metadata) => match CString::new(metadata.first_instruction.to_string()) {
            Ok(name) => name.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Returns the number of operations in the rule's match pattern, or 0 for
/// NULL input or an out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_rule_pattern_len(
    ptr: *const CKnowledgeLibrary,
    index: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    match metadata_for_index(unsafe { &(*ptr).inner }, index) {
        Some(metadata) => metadata.pattern_len,
        None => 0,
    }
}

/// Returns the number of operations emitted by the rule's rewrite target, or
/// 0 for NULL input or an out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_rule_rewrite_len(
    ptr: *const CKnowledgeLibrary,
    index: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    match metadata_for_index(unsafe { &(*ptr).inner }, index) {
        Some(metadata) => metadata.rewrite_len,
        None => 0,
    }
}

/// Returns the number of distinct rule-local qubit labels used by the rule,
/// or 0 for NULL input or an out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_rule_qubit_count(
    ptr: *const CKnowledgeLibrary,
    index: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    match metadata_for_index(unsafe { &(*ptr).inner }, index) {
        Some(metadata) => metadata.qubit_count,
        None => 0,
    }
}

/// Returns the static operation-count delta (`rewrite_len - pattern_len`) of
/// the rule, or 0 for NULL input or an out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_rule_cost_delta(
    ptr: *const CKnowledgeLibrary,
    index: usize,
) -> i64 {
    if ptr.is_null() {
        return 0;
    }
    match metadata_for_index(unsafe { &(*ptr).inner }, index) {
        Some(metadata) => metadata.cost_delta as i64,
        None => 0,
    }
}

/// Returns 1 when the rule carries non-empty parameter conditions, 0
/// otherwise, or -1 on NULL input and -8 on an out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_rule_has_conditions(
    ptr: *const CKnowledgeLibrary,
    index: usize,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    match metadata_for_index(unsafe { &(*ptr).inner }, index) {
        Some(metadata) => metadata.has_conditions as i32,
        None => CqlibError::InvalidParam as i32,
    }
}

// =====  Single-rule parsing and insertion  =====

/// Parses a DSL source string that must define exactly one rule.
///
/// On success writes a heap-allocated `CKnowledgeRule*` (free with
/// `knowledge_rule_free`) to `out` and returns 0. Returns -1 on NULL input,
/// -4 on DSL parse, lowering, or validation failures, and -8 when the source
/// does not define exactly one rule.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rule_from_dsl(
    source: *const c_char,
    out: *mut *mut CKnowledgeRule,
) -> i32 {
    if source.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let source = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(source) => source,
        Err(_) => return CqlibError::ParseError as i32,
    };
    match RuleLibrary::from_dsl_str(source, RuleKind::Other) {
        Ok(library) if library.len() == 1 => {
            let rule = library.rules()[0].clone();
            unsafe { *out = Box::into_raw(Box::new(CKnowledgeRule { inner: rule })) };
            CqlibError::Ok as i32
        }
        Ok(_) => CqlibError::InvalidParam as i32,
        Err(err) => library_error_code(&err),
    }
}

/// Frees a single-rule handle. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rule_free(ptr: *mut CKnowledgeRule) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Adds a parsed rule to the library under `kind` (one of `RULE_KIND_*`)
/// and returns its assigned id.
///
/// The rule is structurally validated before insertion. Returns -1 on NULL
/// input, -8 on an unknown kind tag, -4 on a validation failure or duplicate
/// name, and -6 when the rule's first instruction cannot be indexed.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_library_add_rule(
    ptr: *mut CKnowledgeLibrary,
    rule: *const CKnowledgeRule,
    kind: u8,
) -> i64 {
    if ptr.is_null() || rule.is_null() {
        return CqlibError::NullPtr as i32 as i64;
    }
    let Some(kind) = rule_kind_from_tag(kind) else {
        return CqlibError::InvalidParam as i32 as i64;
    };
    let library = unsafe { &mut *ptr };
    let rule = unsafe { (*rule).inner.clone() };
    match library.inner.add_rule(rule, kind, true) {
        Ok(id) => id.as_usize() as i64,
        Err(err) => library_error_code(&err) as i64,
    }
}

// =====  Single-rule accessors  =====

/// Returns the rule's name as a heap-allocated C string (free with
/// `cqlib_string_free`). Returns NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rule_name(ptr: *const CKnowledgeRule) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match CString::new(unsafe { (*ptr).inner.name.as_str() }) {
        Ok(name) => name.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Returns the number of qubit labels the rule involves, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rule_num_qubits(ptr: *const CKnowledgeRule) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Returns the number of operations in the rule's match pattern, or 0 for
/// NULL.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rule_pattern_len(ptr: *const CKnowledgeRule) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.operations.len() }
}

/// Returns the number of operations emitted by the rule's rewrite target, or
/// 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rule_rewrite_len(ptr: *const CKnowledgeRule) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.target.len() }
}

/// Returns 1 when the rule carries non-empty parameter conditions, 0
/// otherwise, or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rule_has_conditions(ptr: *const CKnowledgeRule) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let rule = unsafe { &(*ptr).inner };
    rule.conditions
        .as_ref()
        .is_some_and(|conditions| !conditions.is_empty()) as i32
}

/// Runs the rule's structural validation.
///
/// Returns 0 when the rule is well formed, -1 on NULL input, and -4 when
/// validation fails.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rule_validate(ptr: *const CKnowledgeRule) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    match unsafe { (*ptr).inner.validate() } {
        Ok(()) => CqlibError::Ok as i32,
        Err(_) => CqlibError::ParseError as i32,
    }
}

// =====  Matching helpers  =====

/// Maps a matcher error onto the C error codes: instructions the matcher
/// cannot represent map to -6, unbound rewrite qubits map to -2, and unbound
/// rewrite symbols map to -8.
fn match_error_code(error: &MatchError) -> i32 {
    match error {
        MatchError::UnsupportedRuleInstruction { .. } => CqlibError::CompilerError as i32,
        MatchError::UnboundRewriteQubit { .. } => CqlibError::QubitOutOfBounds as i32,
        MatchError::UnboundRewriteSymbol { .. } => CqlibError::InvalidParam as i32,
    }
}

/// Converts an operation's parameters into core parameters. Indexed
/// parameters reference a circuit-local table the matcher cannot resolve, so
/// they are rejected.
fn operation_params_as_parameters(op: &COperation) -> Result<Vec<Parameter>, ()> {
    op.inner
        .params
        .iter()
        .map(|param| match param {
            CircuitParam::Fixed(value) => Ok(Parameter::from(*value)),
            CircuitParam::Index(_) => Err(()),
        })
        .collect()
}

/// Resolves a gate name into a standard-gate instruction.
fn instruction_from_name(name: &str) -> Option<Instruction> {
    standard_gate_from_name(name).map(Instruction::Standard)
}

/// Resolves a C array of gate names into standard-gate instructions.
/// Returns `None` on a NULL entry or an unknown gate name.
fn instructions_from_names(names: *const *const c_char, len: usize) -> Option<Vec<Instruction>> {
    if len > 0 && names.is_null() {
        return None;
    }
    let mut result = Vec::with_capacity(len);
    for i in 0..len {
        let ptr = unsafe { *names.add(i) };
        if ptr.is_null() {
            return None;
        }
        let name = unsafe { CStr::from_ptr(ptr) }.to_str().ok()?;
        result.push(instruction_from_name(name)?);
    }
    Some(result)
}

/// Writes a list of strings into a caller buffer of C string pointers
/// (two-step pattern). Returns the total number of available strings; only
/// the first `min(total, len)` entries are written.
fn write_cstring_list(items: &[String], out: *mut *mut c_char, len: usize) -> usize {
    let total = items.len();
    if !out.is_null() {
        for (i, item) in items.iter().take(total.min(len)).enumerate() {
            if let Ok(text) = CString::new(item.as_str()) {
                unsafe { *out.add(i) = text.into_raw() };
            }
        }
    }
    total
}

/// Resolves the candidate rule ids for a first-instruction gate name.
/// Unknown names, invalid UTF-8, and NULL input yield an empty list.
fn candidate_ids(library: &CKnowledgeLibrary, name: *const c_char) -> Vec<u32> {
    if name.is_null() {
        return Vec::new();
    }
    let Ok(name) = unsafe { CStr::from_ptr(name) }.to_str() else {
        return Vec::new();
    };
    let Some(instruction) = instruction_from_name(name) else {
        return Vec::new();
    };
    match library.inner.candidates_for_first_instruction(&instruction) {
        Ok(ids) => ids.iter().map(|id| id.as_usize() as u32).collect(),
        Err(_) => Vec::new(),
    }
}

/// Resolves the rules whose match-side and rewrite-side instruction keys
/// cover the requested gate names. Invalid input yields an empty list.
fn filtered_rule_ids(
    library: &CKnowledgeLibrary,
    ops: *const *const c_char,
    ops_len: usize,
    targets: *const *const c_char,
    targets_len: usize,
) -> Vec<u32> {
    let Some(op_instructions) = instructions_from_names(ops, ops_len) else {
        return Vec::new();
    };
    let Some(target_instructions) = instructions_from_names(targets, targets_len) else {
        return Vec::new();
    };
    match library
        .inner
        .filter_rule_ids_by_instruction_keys(&op_instructions, &target_instructions)
    {
        Ok(ids) => ids.iter().map(|id| id.as_usize() as u32).collect(),
        Err(_) => Vec::new(),
    }
}

/// Collects the rule's free symbols in sorted order.
fn sorted_free_symbols(rule: &Rule) -> Vec<String> {
    let mut symbols: Vec<String> = rule.collect_free_symbols().into_iter().collect();
    symbols.sort();
    symbols
}

/// Collects a rule symbol set in sorted order.
fn sorted_symbols(symbols: HashSet<String>) -> Vec<String> {
    let mut symbols: Vec<String> = symbols.into_iter().collect();
    symbols.sort();
    symbols
}

/// Evaluates a replacement parameter to a fixed double. Symbolic values are
/// evaluated without free bindings; unresolvable expressions yield NaN.
fn replacement_param_value(value: &ParameterValue) -> f64 {
    match value {
        ParameterValue::Fixed(value) => *value,
        ParameterValue::Param(parameter) => parameter.evaluate(&None).unwrap_or(f64::NAN),
    }
}

// =====  Match bindings  =====

/// Creates empty match bindings. Free with `knowledge_bindings_free`.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_bindings_new() -> *mut CKnowledgeBindings {
    Box::into_raw(Box::new(CKnowledgeBindings {
        inner: MatchBindings::new(),
    }))
}

/// Frees match bindings. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_bindings_free(ptr: *mut CKnowledgeBindings) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

// =====  Rule listing and lookup  =====

/// Returns the number of rules in the library, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rules_len(ptr: *const CKnowledgeLibrary) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.len() }
}

/// Copies the rule ids (insertion indices) into `out` (two-step pattern;
/// call `knowledge_rules_len` first). Returns the total number of rules, or
/// 0 for NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rules(
    ptr: *const CKnowledgeLibrary,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let total = unsafe { (*ptr).inner.len() };
    let ids: Vec<u32> = (0..total).map(|i| i as u32).collect();
    crate::device::write_u32_list(&ids, out, len)
}

/// Looks up a rule by name and returns a cloned handle (free with
/// `knowledge_rule_free`). Returns NULL on NULL input or when no rule with
/// that name exists.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_get_by_name(
    ptr: *const CKnowledgeLibrary,
    name: *const c_char,
) -> *mut CKnowledgeRule {
    if ptr.is_null() || name.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(name) = unsafe { CStr::from_ptr(name) }.to_str() else {
        return std::ptr::null_mut();
    };
    match unsafe { (*ptr).inner.get_by_name(name) } {
        Some(rule) => Box::into_raw(Box::new(CKnowledgeRule {
            inner: rule.clone(),
        })),
        None => std::ptr::null_mut(),
    }
}

// =====  Matching  =====

/// Matches the rule's match-pattern item at `item_index` against a concrete
/// operation and records the bound qubits and parameters into `bindings`.
///
/// The match is transactional: when the item does not match, the bindings
/// are left unchanged. Returns 1 on a match, 0 on no match, -1 on NULL
/// input, -2 on an out-of-bounds `item_index`, -8 when the operation carries
/// indexed (table-resolved) parameters, and -6 when the rule item uses an
/// instruction the matcher cannot represent.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_match_rule_item(
    ptr: *const CKnowledgeRule,
    item_index: usize,
    op: *const COperation,
    bindings: *mut CKnowledgeBindings,
) -> i32 {
    if ptr.is_null() || op.is_null() || bindings.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let Some(item) = unsafe { &(*ptr).inner }.operations.get(item_index) else {
        return CqlibError::QubitOutOfBounds as i32;
    };
    let operation = unsafe { &*op };
    let params = match operation_params_as_parameters(operation) {
        Ok(params) => params,
        Err(_) => return CqlibError::InvalidParam as i32,
    };
    let concrete = ConcreteOperationView::new(
        &operation.inner.instruction,
        &operation.inner.qubits,
        &params,
    );
    let bindings = unsafe { &mut *bindings };
    match match_rule_item(item, concrete, &mut bindings.inner) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(err) => match_error_code(&err),
    }
}

/// Matches the rule's full match pattern against `operations_len` adjacent
/// operations. Parameter conditions are not evaluated here; check them
/// separately with `knowledge_conditions_hold` on the returned bindings. On
/// success writes a new `CKnowledgeBindings*` (free with
/// `knowledge_bindings_free`) to `out` and returns 1. Returns 0 when the
/// pattern does not match (with `out` left untouched), -1 on NULL input, and
/// -8 when `operations_len` differs from the pattern length or an operation
/// carries indexed parameters.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rule_matches_operations(
    ptr: *const CKnowledgeRule,
    operations: *const *const COperation,
    operations_len: usize,
    out: *mut *mut CKnowledgeBindings,
) -> i32 {
    if ptr.is_null() || out.is_null() || (operations_len > 0 && operations.is_null()) {
        return CqlibError::NullPtr as i32;
    }
    let rule = unsafe { &(*ptr).inner };
    if rule.operations.len() != operations_len {
        return CqlibError::InvalidParam as i32;
    }
    // Parameter tables are built first and never resized afterwards so the
    // borrowed views below stay valid.
    let mut param_tables = Vec::with_capacity(operations_len);
    for i in 0..operations_len {
        let handle = unsafe { *operations.add(i) };
        if handle.is_null() {
            return CqlibError::NullPtr as i32;
        }
        match operation_params_as_parameters(unsafe { &*handle }) {
            Ok(params) => param_tables.push(params),
            Err(_) => return CqlibError::InvalidParam as i32,
        }
    }
    let mut views = Vec::with_capacity(operations_len);
    for (i, params) in param_tables.iter().enumerate() {
        let operation = unsafe { &(*(*operations.add(i))).inner };
        views.push(ConcreteOperationView::new(
            &operation.instruction,
            &operation.qubits,
            params,
        ));
    }
    // Match every pattern item without evaluating conditions so callers can
    // inspect and report condition failures through `knowledge_conditions_hold`.
    let mut bindings = MatchBindings::new();
    for (item, view) in rule.operations.iter().zip(views.iter()) {
        match match_rule_item(item, *view, &mut bindings) {
            Ok(true) => {}
            Ok(false) => return 0,
            Err(err) => return match_error_code(&err),
        }
    }
    unsafe {
        *out = Box::into_raw(Box::new(CKnowledgeBindings { inner: bindings }));
    }
    1
}

/// Returns 1 when all of the rule's parameter conditions hold under the
/// bindings, 0 when any fails, or -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_conditions_hold(
    ptr: *const CKnowledgeRule,
    bindings: *const CKnowledgeBindings,
) -> i32 {
    if ptr.is_null() || bindings.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let rule = unsafe { &(*ptr).inner };
    let bindings = unsafe { &(*bindings).inner };
    conditions_hold(rule.conditions.as_deref(), bindings) as i32
}

/// Returns 1 when the match-pattern item at `lhs_item` of `lhs` is
/// equivalent to the item at `rhs_item` of `rhs`, 0 when not, -1 on NULL
/// input, and -2 on an out-of-bounds item index.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_equivalent_to(
    lhs: *const CKnowledgeRule,
    lhs_item: usize,
    rhs: *const CKnowledgeRule,
    rhs_item: usize,
) -> i32 {
    if lhs.is_null() || rhs.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let Some(left) = unsafe { &(*lhs).inner }.operations.get(lhs_item) else {
        return CqlibError::QubitOutOfBounds as i32;
    };
    let Some(right) = unsafe { &(*rhs).inner }.operations.get(rhs_item) else {
        return CqlibError::QubitOutOfBounds as i32;
    };
    left.equivalent_to(right) as i32
}

// =====  Candidate and instruction-key filtering  =====

/// Returns the number of rules whose first match instruction has the same
/// matcher key as the gate `name`, or 0 for NULL input or an unknown gate
/// name.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_candidates_for_first_instruction_len(
    ptr: *const CKnowledgeLibrary,
    name: *const c_char,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    candidate_ids(unsafe { &*ptr }, name).len()
}

/// Copies the candidate rule ids for the first-instruction gate `name` into
/// `out` (two-step pattern; call
/// `knowledge_candidates_for_first_instruction_len` first). Returns the
/// total number of candidates, or 0 for NULL input or an unknown gate name.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_candidates_for_first_instruction(
    ptr: *const CKnowledgeLibrary,
    name: *const c_char,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let ids = candidate_ids(unsafe { &*ptr }, name);
    crate::device::write_u32_list(&ids, out, len)
}

/// Returns the number of distinct free symbols in the rule, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_collect_free_symbols_len(ptr: *const CKnowledgeRule) -> usize {
    if ptr.is_null() {
        return 0;
    }
    sorted_free_symbols(unsafe { &(*ptr).inner }).len()
}

/// Copies the rule's free symbols (sorted) into `out` as newly allocated C
/// strings (two-step pattern; call `knowledge_collect_free_symbols_len`
/// first). Each string must be released with `cqlib_string_free`. Returns
/// the total number of symbols, or 0 for NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_collect_free_symbols(
    ptr: *const CKnowledgeRule,
    out: *mut *mut c_char,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let symbols = sorted_free_symbols(unsafe { &(*ptr).inner });
    write_cstring_list(&symbols, out, len)
}

// =====  Rule symbol queries  =====

/// Returns the number of distinct symbols bound by the rule's match block,
/// or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rule_operation_symbols_len(ptr: *const CKnowledgeRule) -> usize {
    if ptr.is_null() {
        return 0;
    }
    sorted_symbols(unsafe { &(*ptr).inner }.operation_symbols()).len()
}

/// Copies the symbols bound by the rule's match block (sorted) into `out` as
/// newly allocated C strings (two-step pattern; call
/// `knowledge_rule_operation_symbols_len` first). Each string must be
/// released with `cqlib_string_free`. Returns 0 on success, -1 on NULL
/// input, and -8 when `len` does not match the symbol count.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rule_operation_symbols(
    ptr: *const CKnowledgeRule,
    out: *mut *mut c_char,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let symbols = sorted_symbols(unsafe { &(*ptr).inner }.operation_symbols());
    if symbols.len() != len {
        return CqlibError::InvalidParam as i32;
    }
    if len > 0 && out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    write_cstring_list(&symbols, out, len);
    CqlibError::Ok as i32
}

/// Returns the number of distinct symbols referenced by the rule's rewrite
/// target, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rule_target_symbols_len(ptr: *const CKnowledgeRule) -> usize {
    if ptr.is_null() {
        return 0;
    }
    sorted_symbols(unsafe { &(*ptr).inner }.target_symbols()).len()
}

/// Copies the symbols referenced by the rule's rewrite target (sorted) into
/// `out` as newly allocated C strings (two-step pattern; call
/// `knowledge_rule_target_symbols_len` first). Each string must be released
/// with `cqlib_string_free`. Returns 0 on success, -1 on NULL input, and -8
/// when `len` does not match the symbol count.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rule_target_symbols(
    ptr: *const CKnowledgeRule,
    out: *mut *mut c_char,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let symbols = sorted_symbols(unsafe { &(*ptr).inner }.target_symbols());
    if symbols.len() != len {
        return CqlibError::InvalidParam as i32;
    }
    if len > 0 && out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    write_cstring_list(&symbols, out, len);
    CqlibError::Ok as i32
}

/// Returns the number of distinct symbols referenced by the rule's
/// parameter conditions, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rule_condition_symbols_len(ptr: *const CKnowledgeRule) -> usize {
    if ptr.is_null() {
        return 0;
    }
    sorted_symbols(unsafe { &(*ptr).inner }.condition_symbols()).len()
}

/// Copies the symbols referenced by the rule's parameter conditions (sorted)
/// into `out` as newly allocated C strings (two-step pattern; call
/// `knowledge_rule_condition_symbols_len` first). Each string must be
/// released with `cqlib_string_free`. Returns 0 on success, -1 on NULL
/// input, and -8 when `len` does not match the symbol count.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rule_condition_symbols(
    ptr: *const CKnowledgeRule,
    out: *mut *mut c_char,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let symbols = sorted_symbols(unsafe { &(*ptr).inner }.condition_symbols());
    if symbols.len() != len {
        return CqlibError::InvalidParam as i32;
    }
    if len > 0 && out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    write_cstring_list(&symbols, out, len);
    CqlibError::Ok as i32
}

/// Returns the number of rules whose match pattern covers every gate in
/// `ops` and whose rewrite target covers every gate in `targets` (both as
/// arrays of gate names), or 0 for NULL input or an unknown gate name.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_filter_rule_ids_by_instruction_keys_len(
    ptr: *const CKnowledgeLibrary,
    ops: *const *const c_char,
    ops_len: usize,
    targets: *const *const c_char,
    targets_len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    filtered_rule_ids(unsafe { &*ptr }, ops, ops_len, targets, targets_len).len()
}

/// Copies the rule ids filtered by match-side (`ops`) and rewrite-side
/// (`targets`) gate names into `out` (two-step pattern; call
/// `knowledge_filter_rule_ids_by_instruction_keys_len` first). Returns the
/// total number of matching rules, or 0 for NULL input or an unknown gate
/// name.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_filter_rule_ids_by_instruction_keys(
    ptr: *const CKnowledgeLibrary,
    ops: *const *const c_char,
    ops_len: usize,
    targets: *const *const c_char,
    targets_len: usize,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let ids = filtered_rule_ids(unsafe { &*ptr }, ops, ops_len, targets, targets_len);
    crate::device::write_u32_list(&ids, out, len)
}

// =====  Rule extension  =====

/// Adds several parsed rules to the library under `kind` (one of
/// `RULE_KIND_*`) atomically and writes the assigned rule ids into `out`
/// (which must have room for `rules_len` entries). If any rule fails, the
/// library is left unchanged. Returns 0 on success, -1 on NULL input, -8 on
/// an unknown kind tag or an `out` buffer smaller than `rules_len`, and -4
/// on a validation failure or duplicate rule name.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_extend_rules(
    ptr: *mut CKnowledgeLibrary,
    rules: *const *const CKnowledgeRule,
    rules_len: usize,
    kind: u8,
    out: *mut u32,
    out_len: usize,
) -> i32 {
    if ptr.is_null() || (rules_len > 0 && rules.is_null()) {
        return CqlibError::NullPtr as i32;
    }
    let Some(kind) = rule_kind_from_tag(kind) else {
        return CqlibError::InvalidParam as i32;
    };
    if out_len < rules_len {
        return CqlibError::InvalidParam as i32;
    }
    let mut handles = Vec::with_capacity(rules_len);
    for i in 0..rules_len {
        let handle = unsafe { *rules.add(i) };
        if handle.is_null() {
            return CqlibError::NullPtr as i32;
        }
        handles.push(unsafe { (*handle).inner.clone() });
    }
    match unsafe { &mut *ptr }.inner.extend_rules(handles, kind) {
        Ok(ids) => {
            let values: Vec<u32> = ids.iter().map(|id| id.as_usize() as u32).collect();
            crate::device::write_u32_list(&values, out, out_len);
            CqlibError::Ok as i32
        }
        Err(err) => library_error_code(&err),
    }
}

// =====  Target queries and instantiation  =====

/// Returns the number of distinct rule-local qubit labels used by the
/// rule's rewrite target, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_target_qubits_len(ptr: *const CKnowledgeRule) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.target_qubits().len() }
}

/// Copies the rule-local qubit labels (sorted) used by the rule's rewrite
/// target into `out` (two-step pattern; call `knowledge_target_qubits_len`
/// first). Returns the total number of labels, or 0 for NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_target_qubits(
    ptr: *const CKnowledgeRule,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let qubits: Vec<u32> = unsafe { (*ptr).inner.target_qubits() }
        .into_iter()
        .collect();
    crate::device::write_u32_list(&qubits, out, len)
}

/// Instantiates the rule's rewrite target under the match bindings. On
/// success writes a new `CKnowledgeReplacements*` (free with
/// `knowledge_replacements_free`) to `out` and returns 0. Returns -1 on NULL
/// input, -2 when the target references a qubit not bound by the match, -8
/// when the target references a symbol not bound by the match, and -6 when
/// the target uses an instruction the matcher cannot represent.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_instantiate_target(
    ptr: *const CKnowledgeRule,
    bindings: *const CKnowledgeBindings,
    out: *mut *mut CKnowledgeReplacements,
) -> i32 {
    if ptr.is_null() || bindings.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let rule = unsafe { &(*ptr).inner };
    let bindings = unsafe { &(*bindings).inner };
    match instantiate_target(&rule.target, bindings) {
        Ok(replacements) => {
            unsafe {
                *out = Box::into_raw(Box::new(CKnowledgeReplacements {
                    inner: replacements,
                }));
            }
            CqlibError::Ok as i32
        }
        Err(err) => match_error_code(&err),
    }
}

/// Returns the number of instantiated replacement items, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_replacements_len(ptr: *const CKnowledgeReplacements) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.len() }
}

/// Returns the instruction name (e.g. "RZ") of the replacement item at
/// `index` as a newly allocated C string (free with `cqlib_string_free`).
/// Returns NULL on NULL input or an out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_replacement_instruction(
    ptr: *const CKnowledgeReplacements,
    index: usize,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { &(*ptr).inner }.get(index) {
        Some(replacement) => match CString::new(replacement.instruction.name()) {
            Ok(text) => text.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Returns the number of concrete qubits of the replacement item at
/// `index`, or 0 for NULL input or an out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_replacement_qubits_len(
    ptr: *const CKnowledgeReplacements,
    index: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    match unsafe { &(*ptr).inner }.get(index) {
        Some(replacement) => replacement.qubits.len(),
        None => 0,
    }
}

/// Copies the concrete qubits of the replacement item at `index` into `out`
/// (two-step pattern; call `knowledge_replacement_qubits_len` first).
/// Returns the total number of qubits, or 0 for NULL input or an
/// out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_replacement_qubits(
    ptr: *const CKnowledgeReplacements,
    index: usize,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let Some(replacement) = unsafe { &(*ptr).inner }.get(index) else {
        return 0;
    };
    let qubits: Vec<u32> = replacement.qubits.iter().map(|qubit| qubit.id()).collect();
    crate::device::write_u32_list(&qubits, out, len)
}

/// Returns the number of evaluated parameters of the replacement item at
/// `index`, or 0 for NULL input or an out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_replacement_params_len(
    ptr: *const CKnowledgeReplacements,
    index: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    match unsafe { &(*ptr).inner }.get(index) {
        Some(replacement) => replacement.params.len(),
        None => 0,
    }
}

/// Copies the evaluated double parameters of the replacement item at
/// `index` into `out` (two-step pattern; call
/// `knowledge_replacement_params_len` first). Returns the total number of
/// parameters, or 0 for NULL input or an out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_replacement_params(
    ptr: *const CKnowledgeReplacements,
    index: usize,
    out: *mut f64,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let Some(replacement) = unsafe { &(*ptr).inner }.get(index) else {
        return 0;
    };
    let values: Vec<f64> = replacement
        .params
        .iter()
        .map(replacement_param_value)
        .collect();
    let total = values.len();
    if !out.is_null() {
        for (i, value) in values.iter().take(total.min(len)).enumerate() {
            unsafe { *out.add(i) = *value };
        }
    }
    total
}

/// Frees a rewrite-target instantiation result. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_replacements_free(ptr: *mut CKnowledgeReplacements) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}
