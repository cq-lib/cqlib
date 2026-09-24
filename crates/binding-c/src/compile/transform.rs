// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// use this file except in compliance with the License. You may obtain
// a copy of the License in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! C ABI for circuit transforms: structural analysis, canonicalization,
//! knowledge rewrite, and standalone pass wrappers returning new circuits.

use crate::circuit::CCircuit;
use crate::device::standard_gate_from_name;
use crate::error::CqlibError;
use cqlib_core::circuit::Instruction;
use cqlib_core::compile::CompilerError;
use cqlib_core::compile::knowledge::RuleKind;
use cqlib_core::compile::transform::canonicalize_circuit as core_canonicalize_circuit;
use cqlib_core::compile::transform::rewrite_circuit as core_rewrite_circuit;
use cqlib_core::compile::transform::{
    CanonicalizeConfig, CanonicalizeResult, Canonicalizer, CircuitAnalysis,
    CommutativeCancellation, KnowledgeRewriteResult, KnowledgeRewriter, LowerToRoutingBasis,
    OptimizeOneQubitRuns, RewriteConfig, RewriteMode, Transformer,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// C snapshot of structural circuit facts. Every flag is 0 or 1.
#[repr(C)]
pub struct CCircuitAnalysis {
    /// Circuit contains classical-data operations.
    pub has_classical_data: u8,
    /// Circuit contains classical control flow.
    pub has_classical_control: u8,
    /// Circuit contains measurement operations.
    pub has_measurement: u8,
    /// Circuit registers classical values.
    pub has_classical_values: u8,
    /// Circuit registers classical variables.
    pub has_classical_vars: u8,
    /// Circuit uses any runtime classical construct.
    pub has_runtime_classical: u8,
    /// Rewrites must preserve classical handles for this circuit.
    pub needs_classical_handle_preservation: u8,
    /// Circuit contains unexpanded circuit-backed gate definitions.
    pub has_circuit_gate_definitions: u8,
    /// Circuit contains unitary definitions with circuit bodies.
    pub has_unitary_circuit_definitions: u8,
    /// Circuit contains matrix-backed unitary gates.
    pub has_unitary_gates: u8,
    /// Circuit contains multi-controlled gates.
    pub has_mc_gates: u8,
}

/// Opaque handle around a [`CanonicalizeResult`].
pub struct CCanonicalizeResult {
    pub inner: CanonicalizeResult,
}

/// Opaque handle around a [`KnowledgeRewriteResult`].
pub struct CKnowledgeRewriteResult {
    pub inner: KnowledgeRewriteResult,
}

/// Rewrite mode tag.
/// | Value | Mode                                   |
/// |-------|----------------------------------------|
/// | 0     | Conservative optimization (production) |
/// | 1     | Explicit knowledge-based lowering      |
pub const REWRITE_MODE_OPTIMIZE: u8 = 0;
pub const REWRITE_MODE_LOWERING: u8 = 1;

/// C form of [`RewriteConfig`].
///
/// `mode` selects the rule-set base (`REWRITE_MODE_OPTIMIZE` uses production
/// defaults, `REWRITE_MODE_LOWERING` adds decomposition and hardware-native
/// rules); the remaining fields override the base values. Boolean fields use
/// 0/1.
#[repr(C)]
pub struct CRewriteConfig {
    /// One of `REWRITE_MODE_*`.
    pub mode: u8,
    /// Fixpoint round limit.
    pub max_rounds: u8,
    /// 1 recurses into control-flow bodies, 0 does not.
    pub recurse_control_flow: u8,
    /// 1 skips labeled operations, 0 does not.
    pub skip_labeled_ops: u8,
    /// 1 preserves undirected two-qubit connectivity, 0 does not.
    pub preserve_two_qubit_connectivity: u8,
    /// Reserved padding to keep the struct layout stable.
    pub _pad: [u8; 3],
    /// Matching-window operation limit.
    pub max_window_ops: usize,
    /// Maximum matched pattern length.
    pub max_pattern_len: usize,
    /// Borrowed target-basis gate names (each a NUL-terminated string), set
    /// with `rewrite_config_with_target_instructions`; NULL when no target
    /// basis is configured. The array and its strings are borrowed and must
    /// stay valid until the rewrite call that consumes this configuration.
    pub target_gate_names: *const *const c_char,
    /// Number of entries in `target_gate_names`.
    pub target_gate_names_len: usize,
}

/// C snapshot of knowledge-rewrite run statistics.
#[repr(C)]
pub struct CKnowledgeRewriteStats {
    /// Fixpoint rounds actually executed.
    pub rounds_executed: u8,
    /// 1 when a stable round was observed before `max_rounds`.
    pub reached_fixpoint: u8,
    /// Reserved padding to keep the struct layout stable.
    pub _pad: [u8; 6],
    /// Rule patches emitted into rebuilt sequences.
    pub rules_applied: usize,
    /// Operation sequences whose patch set was non-empty.
    pub changed_sequences: usize,
}

/// C snapshot of knowledge-rewrite performance diagnostics.
#[repr(C)]
pub struct CKnowledgeRewriteDiagnostics {
    /// Rewrite phases skipped by a reusable session fixpoint proof.
    pub direct_reuses: usize,
    /// Anchors presented by verified dirty ranges before density fallback.
    pub dirty_anchors: usize,
    /// Incremental executions conservatively promoted to full scans.
    pub full_scan_fallbacks: usize,
    /// Cached rule-condition results reused.
    pub condition_cache_hits: usize,
    /// Cacheable rule-condition bindings not found in the local cache.
    pub condition_cache_misses: usize,
    /// Conditions evaluated by the conservative symbolic path.
    pub symbolic_fallbacks: usize,
}

/// Converts a C rewrite configuration into the core form.
fn rewrite_config_from_c(raw: &CRewriteConfig) -> Result<RewriteConfig, i32> {
    let config = match raw.mode {
        REWRITE_MODE_OPTIMIZE => RewriteConfig::production(),
        REWRITE_MODE_LOWERING => RewriteConfig::lowering(),
        _ => return Err(CqlibError::InvalidParam as i32),
    };
    let mut config = config
        .with_max_rounds(raw.max_rounds)
        .with_max_window_ops(raw.max_window_ops)
        .with_max_pattern_len(raw.max_pattern_len)
        .recurse_control_flow(raw.recurse_control_flow != 0)
        .skip_labeled_ops(raw.skip_labeled_ops != 0)
        .with_preserve_two_qubit_connectivity(raw.preserve_two_qubit_connectivity != 0);
    if raw.target_gate_names_len > 0 {
        let instructions =
            standard_instructions_from_names(raw.target_gate_names, raw.target_gate_names_len)?;
        config = match config.try_with_target_instructions(instructions) {
            Ok(config) => config,
            Err(CompilerError::InvalidInput(_)) => return Err(CqlibError::InvalidParam as i32),
            Err(_) => return Err(CqlibError::CompilerError as i32),
        };
    }
    Ok(config)
}

/// Returns the production rewrite configuration by value.
#[unsafe(no_mangle)]
pub extern "C" fn rewrite_config_default() -> CRewriteConfig {
    CRewriteConfig {
        mode: REWRITE_MODE_OPTIMIZE,
        max_rounds: 8,
        recurse_control_flow: 1,
        skip_labeled_ops: 1,
        preserve_two_qubit_connectivity: 0,
        _pad: [0; 3],
        max_window_ops: 16,
        max_pattern_len: 8,
        target_gate_names: std::ptr::null(),
        target_gate_names_len: 0,
    }
}

/// Returns the explicit lowering rewrite configuration by value.
#[unsafe(no_mangle)]
pub extern "C" fn rewrite_config_lowering() -> CRewriteConfig {
    let mut config = rewrite_config_default();
    config.mode = REWRITE_MODE_LOWERING;
    config
}

/// Restricts a by-value rewrite configuration to an explicit standard-gate
/// target instruction basis, parsed from `len` gate names (e.g. `"H"`,
/// `"CX"`).
///
/// The name array is borrowed and must remain valid until the rewrite call
/// that consumes `config`. The target basis only takes effect when `config`
/// uses `REWRITE_MODE_LOWERING`. Returns 0 on success, -1 on NULL pointers,
/// -4 on invalid UTF-8 or an unknown gate name, or -8 for an empty basis
/// (the configuration is then left unchanged).
#[unsafe(no_mangle)]
pub extern "C" fn rewrite_config_with_target_instructions(
    config: *mut CRewriteConfig,
    gate_names: *const *const c_char,
    len: usize,
) -> i32 {
    if config.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if len == 0 {
        return CqlibError::InvalidParam as i32;
    }
    // Validate eagerly so bad names are reported at set time; the parsed
    // instructions are rebuilt lazily in `rewrite_config_from_c`.
    if let Err(code) = standard_instructions_from_names(gate_names, len) {
        return code;
    }
    unsafe {
        (*config).target_gate_names = gate_names;
        (*config).target_gate_names_len = len;
    }
    CqlibError::Ok as i32
}

/// Writes a structural-analysis snapshot of `circuit` to `*out`.
///
/// Returns 0 on success or -1 on NULL arguments.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_analyze(circuit: *const CCircuit, out: *mut CCircuitAnalysis) -> i32 {
    if circuit.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let circuit = unsafe { &(*circuit).inner };
    let analysis = CircuitAnalysis::analyze(circuit);
    unsafe {
        *out = CCircuitAnalysis {
            has_classical_data: analysis.has_classical_data as u8,
            has_classical_control: analysis.has_classical_control as u8,
            has_measurement: analysis.has_measurement as u8,
            has_classical_values: analysis.has_classical_values as u8,
            has_classical_vars: analysis.has_classical_vars as u8,
            has_runtime_classical: analysis.has_runtime_classical as u8,
            needs_classical_handle_preservation: analysis.needs_classical_handle_preservation as u8,
            has_circuit_gate_definitions: analysis.has_circuit_gate_definitions as u8,
            has_unitary_circuit_definitions: analysis.has_unitary_circuit_definitions as u8,
            has_unitary_gates: analysis.has_unitary_gates as u8,
            has_mc_gates: analysis.has_mc_gates as u8,
        };
    }
    CqlibError::Ok as i32
}

// =====  Canonicalization  =====

/// Canonicalizes `circuit` with production defaults.
///
/// Returns a heap-allocated `CCanonicalizeResult*`, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_circuit(circuit: *const CCircuit) -> *mut CCanonicalizeResult {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner };
    match core_canonicalize_circuit(circuit) {
        Ok(result) => Box::into_raw(Box::new(CCanonicalizeResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a canonicalize result. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_result_free(ptr: *mut CCanonicalizeResult) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the canonicalized circuit as an owned `CCircuit*` (free with
/// `circuit_free`). Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_result_circuit(ptr: *const CCanonicalizeResult) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    Box::into_raw(Box::new(CCircuit {
        inner: result.inner.circuit.clone(),
    }))
}

/// Returns 1 when the output differs from the input representation, 0
/// otherwise, or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_result_changed(ptr: *const CCanonicalizeResult) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.changed as i32 }
}

/// Returns the number of canonicalization rounds executed, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_result_rounds(ptr: *const CCanonicalizeResult) -> u8 {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.rounds }
}

// =====  Canonicalize configuration  =====

/// Opaque handle around a [`CanonicalizeConfig`].
pub struct CCanonicalizeConfig {
    pub inner: CanonicalizeConfig,
}

/// Creates a canonicalize config with production defaults (round limit 8,
/// all optional behaviors enabled).
///
/// Returns a heap-allocated `CCanonicalizeConfig*`, or NULL on allocation
/// failure. Free with `canonicalize_config_free`.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_config_default() -> *mut CCanonicalizeConfig {
    Box::into_raw(Box::new(CCanonicalizeConfig {
        inner: CanonicalizeConfig::production(),
    }))
}

/// Frees a canonicalize config. Passing NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_config_free(ptr: *mut CCanonicalizeConfig) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Sets the maximum number of canonicalization rounds. Returns 0, or -1 on
/// NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_config_with_round_limit(
    ptr: *mut CCanonicalizeConfig,
    round_limit: u8,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe {
        (*ptr).inner = (*ptr).inner.clone().with_round_limit(round_limit);
    }
    CqlibError::Ok as i32
}

fn canonicalize_config_set_flag(
    ptr: *mut CCanonicalizeConfig,
    enabled: i32,
    apply: impl FnOnce(CanonicalizeConfig, bool) -> CanonicalizeConfig,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe {
        let current = (*ptr).inner.clone();
        (*ptr).inner = apply(current, enabled != 0);
    }
    CqlibError::Ok as i32
}

fn canonicalize_config_get_flag(
    ptr: *const CCanonicalizeConfig,
    read: impl FnOnce(&CanonicalizeConfig) -> bool,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { read(&(*ptr).inner) as i32 }
}

/// Sets whether control-flow bodies are recursively canonicalized (1) or not
/// (0). Returns 0, or -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_config_with_recurse_control_flow(
    ptr: *mut CCanonicalizeConfig,
    enabled: i32,
) -> i32 {
    canonicalize_config_set_flag(ptr, enabled, |cfg, on| cfg.recurse_control_flow(on))
}

/// Sets whether `GPhase` operations are folded into scope-local phase.
/// Returns 0, or -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_config_with_fold_gphase(
    ptr: *mut CCanonicalizeConfig,
    enabled: i32,
) -> i32 {
    canonicalize_config_set_flag(ptr, enabled, |cfg, on| cfg.fold_gphase(on))
}

/// Sets whether `McGate` forms are collapsed into existing standard gates.
/// Returns 0, or -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_config_with_canonicalize_instruction_form(
    ptr: *mut CCanonicalizeConfig,
    enabled: i32,
) -> i32 {
    canonicalize_config_set_flag(ptr, enabled, |cfg, on| {
        cfg.canonicalize_instruction_form(on)
    })
}

/// Sets whether strict no-op removal is performed. Returns 0, or -1 on NULL
/// input.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_config_with_drop_noops(
    ptr: *mut CCanonicalizeConfig,
    enabled: i32,
) -> i32 {
    canonicalize_config_set_flag(ptr, enabled, |cfg, on| cfg.drop_noops(on))
}

/// Sets whether barrier scopes are canonicalized and adjacent barriers
/// merged. Returns 0, or -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_config_with_canonicalize_barriers(
    ptr: *mut CCanonicalizeConfig,
    enabled: i32,
) -> i32 {
    canonicalize_config_set_flag(ptr, enabled, |cfg, on| cfg.canonicalize_barriers(on))
}

/// Returns the configured round limit, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_config_round_limit(ptr: *const CCanonicalizeConfig) -> u8 {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.round_limit() }
}

/// Returns 1 when control-flow recursion is enabled, 0 otherwise, or -1 for
/// NULL.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_config_recurse_control_flow(ptr: *const CCanonicalizeConfig) -> i32 {
    canonicalize_config_get_flag(ptr, |cfg| cfg.recurses_control_flow())
}

/// Returns 1 when `GPhase` folding is enabled, 0 otherwise, or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_config_fold_gphase(ptr: *const CCanonicalizeConfig) -> i32 {
    canonicalize_config_get_flag(ptr, |cfg| cfg.folds_gphase())
}

/// Returns 1 when instruction form canonicalization is enabled, 0 otherwise,
/// or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_config_canonicalize_instruction_form(
    ptr: *const CCanonicalizeConfig,
) -> i32 {
    canonicalize_config_get_flag(ptr, |cfg| cfg.canonicalizes_instruction_form())
}

/// Returns 1 when strict no-op removal is enabled, 0 otherwise, or -1 for
/// NULL.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_config_drop_noops(ptr: *const CCanonicalizeConfig) -> i32 {
    canonicalize_config_get_flag(ptr, |cfg| cfg.drops_noops())
}

/// Returns 1 when barrier canonicalization is enabled, 0 otherwise, or -1
/// for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_config_canonicalize_barriers(
    ptr: *const CCanonicalizeConfig,
) -> i32 {
    canonicalize_config_get_flag(ptr, |cfg| cfg.canonicalizes_barriers())
}

/// Canonicalizes `circuit` using the supplied configuration. The config is
/// not consumed and remains owned by the caller.
///
/// Returns a heap-allocated `CCanonicalizeResult*`, or NULL on error or NULL
/// input.
#[unsafe(no_mangle)]
pub extern "C" fn canonicalize_circuit_with_config(
    circuit: *const CCircuit,
    config: *const CCanonicalizeConfig,
) -> *mut CCanonicalizeResult {
    if circuit.is_null() || config.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner };
    let canonicalizer = Canonicalizer::new(unsafe { &(*config).inner }.clone());
    match canonicalizer.run(circuit) {
        Ok(result) => Box::into_raw(Box::new(CCanonicalizeResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

// =====  Knowledge rewrite  =====

/// Applies the knowledge rewriter to `circuit` with the supplied
/// configuration.
///
/// Returns a heap-allocated `CKnowledgeRewriteResult*`, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn rewrite_circuit(
    circuit: *const CCircuit,
    config: CRewriteConfig,
) -> *mut CKnowledgeRewriteResult {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(config) = rewrite_config_from_c(&config) else {
        return std::ptr::null_mut();
    };
    let circuit = unsafe { &(*circuit).inner };
    match core_rewrite_circuit(circuit, config) {
        Ok(result) => Box::into_raw(Box::new(CKnowledgeRewriteResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a knowledge-rewrite result. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rewrite_result_free(ptr: *mut CKnowledgeRewriteResult) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the rewritten circuit as an owned `CCircuit*` (free with
/// `circuit_free`). Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rewrite_result_circuit(
    ptr: *const CKnowledgeRewriteResult,
) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    Box::into_raw(Box::new(CCircuit {
        inner: result.inner.circuit.clone(),
    }))
}

/// Returns 1 when the output differs from the input representation, 0
/// otherwise, or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rewrite_result_changed(ptr: *const CKnowledgeRewriteResult) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.changed as i32 }
}

/// Writes the rewrite-statistics snapshot to `*out`.
///
/// Returns 0 on success or -1 on NULL arguments.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rewrite_result_stats(
    ptr: *const CKnowledgeRewriteResult,
    out: *mut CKnowledgeRewriteStats,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let result = unsafe { &*ptr };
    let stats = &result.inner.stats;
    unsafe {
        *out = CKnowledgeRewriteStats {
            rounds_executed: stats.rounds_executed,
            reached_fixpoint: stats.reached_fixpoint as u8,
            _pad: [0; 6],
            rules_applied: stats.rules_applied,
            changed_sequences: stats.changed_sequences,
        };
    }
    CqlibError::Ok as i32
}

/// Writes the rewrite-diagnostics snapshot to `*out`.
///
/// Returns 0 on success or -1 on NULL arguments.
#[unsafe(no_mangle)]
pub extern "C" fn knowledge_rewrite_result_diagnostics(
    ptr: *const CKnowledgeRewriteResult,
    out: *mut CKnowledgeRewriteDiagnostics,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let result = unsafe { &*ptr };
    let diagnostics = &result.inner.diagnostics;
    unsafe {
        *out = CKnowledgeRewriteDiagnostics {
            direct_reuses: diagnostics.direct_reuses,
            dirty_anchors: diagnostics.dirty_anchors,
            full_scan_fallbacks: diagnostics.full_scan_fallbacks,
            condition_cache_hits: diagnostics.condition_cache_hits,
            condition_cache_misses: diagnostics.condition_cache_misses,
            symbolic_fallbacks: diagnostics.symbolic_fallbacks,
        };
    }
    CqlibError::Ok as i32
}

// =====  Standalone pass wrappers  =====

/// Runs one `Transformer` pass and returns the resolved circuit.
fn run_transform_pass<T: Transformer>(pass: &T, circuit: *const CCircuit) -> *mut CCircuit {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner };
    match pass.transform(circuit, None) {
        Ok(outcome) => Box::into_raw(Box::new(CCircuit {
            inner: outcome.into_circuit(circuit),
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Canonicalizes `circuit` with production defaults and returns the rebuilt
/// circuit as an owned `CCircuit*` (free with `circuit_free`).
///
/// Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn transform_canonicalize(circuit: *const CCircuit) -> *mut CCircuit {
    run_transform_pass(&Canonicalizer::production(), circuit)
}

/// Applies the knowledge rewriter to `circuit` and returns the rebuilt circuit
/// as an owned `CCircuit*` (free with `circuit_free`).
///
/// Returns NULL on error (including an invalid `config.mode`).
#[unsafe(no_mangle)]
pub extern "C" fn transform_knowledge_rewrite(
    circuit: *const CCircuit,
    config: CRewriteConfig,
) -> *mut CCircuit {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(config) = rewrite_config_from_c(&config) else {
        return std::ptr::null_mut();
    };
    run_transform_pass(&KnowledgeRewriter::new(config), circuit)
}

/// Fuses fixed numeric one-qubit runs through exact synthesis (target-neutral
/// logical cost) and returns the rebuilt circuit as an owned `CCircuit*` (free
/// with `circuit_free`).
///
/// Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn transform_optimize_one_qubit_runs(circuit: *const CCircuit) -> *mut CCircuit {
    run_transform_pass(&OptimizeOneQubitRuns::logical(), circuit)
}

/// Cancels self-inverse gate pairs through global commutation analysis and
/// returns the rebuilt circuit as an owned `CCircuit*` (free with
/// `circuit_free`).
///
/// Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn transform_commutative_cancellation(circuit: *const CCircuit) -> *mut CCircuit {
    run_transform_pass(&CommutativeCancellation::new(), circuit)
}

/// Lowers `circuit` to the routing basis and returns the rebuilt circuit as
/// an owned `CCircuit*` (free with `circuit_free`).
///
/// Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn transform_lower_to_routing_basis(circuit: *const CCircuit) -> *mut CCircuit {
    run_transform_pass(&LowerToRoutingBasis::new(None), circuit)
}

// =====  Rewrite configuration handle  =====

/// Rewrite rule-category tag used by `transform_config_with_enabled_kinds`
/// and `transform_config_enabled_kinds`.
/// | Value | Kind                      |
/// |-------|---------------------------|
/// | 0     | Algebraic simplification  |
/// | 1     | Inverse/repeat cancellation |
/// | 2     | Neighbor-gate merge       |
/// | 3     | Explicit commutation      |
/// | 4     | Decomposition / lowering  |
/// | 5     | Canonical representation  |
/// | 6     | Hardware-native rewriting |
/// | 7     | Other                     |
pub const REWRITE_KIND_SIMPLIFY: u8 = 0;
pub const REWRITE_KIND_CANCEL: u8 = 1;
pub const REWRITE_KIND_MERGE: u8 = 2;
pub const REWRITE_KIND_COMMUTE: u8 = 3;
pub const REWRITE_KIND_DECOMPOSE: u8 = 4;
pub const REWRITE_KIND_CANONICALIZE: u8 = 5;
pub const REWRITE_KIND_HARDWARE_NATIVE: u8 = 6;
pub const REWRITE_KIND_OTHER: u8 = 7;

/// Opaque handle around a core [`RewriteConfig`] (the knowledge-rewrite /
/// transform configuration).
pub struct CTransformConfig {
    pub inner: RewriteConfig,
}

/// Maps a `REWRITE_KIND_*` tag to the core rule kind.
fn rule_kind_from_tag(tag: u8) -> Option<RuleKind> {
    Some(match tag {
        REWRITE_KIND_SIMPLIFY => RuleKind::Simplify,
        REWRITE_KIND_CANCEL => RuleKind::Cancel,
        REWRITE_KIND_MERGE => RuleKind::Merge,
        REWRITE_KIND_COMMUTE => RuleKind::Commute,
        REWRITE_KIND_DECOMPOSE => RuleKind::Decompose,
        REWRITE_KIND_CANONICALIZE => RuleKind::Canonicalize,
        REWRITE_KIND_HARDWARE_NATIVE => RuleKind::HardwareNative,
        REWRITE_KIND_OTHER => RuleKind::Other,
        _ => return None,
    })
}

/// Maps a core rule kind to its `REWRITE_KIND_*` tag.
fn rule_kind_tag(kind: RuleKind) -> u8 {
    match kind {
        RuleKind::Simplify => REWRITE_KIND_SIMPLIFY,
        RuleKind::Cancel => REWRITE_KIND_CANCEL,
        RuleKind::Merge => REWRITE_KIND_MERGE,
        RuleKind::Commute => REWRITE_KIND_COMMUTE,
        RuleKind::Decompose => REWRITE_KIND_DECOMPOSE,
        RuleKind::Canonicalize => REWRITE_KIND_CANONICALIZE,
        RuleKind::HardwareNative => REWRITE_KIND_HARDWARE_NATIVE,
        RuleKind::Other => REWRITE_KIND_OTHER,
    }
}

/// Parses a C array of standard-gate names into core instructions.
///
/// Returns -1 on NULL entries, -4 on invalid UTF-8 or an unknown gate name.
pub(crate) fn standard_instructions_from_names(
    names: *const *const c_char,
    len: usize,
) -> Result<Vec<Instruction>, i32> {
    if len == 0 {
        return Ok(Vec::new());
    }
    if names.is_null() {
        return Err(CqlibError::NullPtr as i32);
    }
    let raw = unsafe { std::slice::from_raw_parts(names, len) };
    let mut instructions = Vec::with_capacity(len);
    for name in raw {
        if name.is_null() {
            return Err(CqlibError::NullPtr as i32);
        }
        let name = match unsafe { CStr::from_ptr(*name) }.to_str() {
            Ok(name) => name,
            Err(_) => return Err(CqlibError::ParseError as i32),
        };
        let Some(gate) = standard_gate_from_name(name) else {
            return Err(CqlibError::ParseError as i32);
        };
        instructions.push(Instruction::Standard(gate));
    }
    Ok(instructions)
}

/// Copies a name list into `out` (two-step pattern). Each written entry is a
/// freshly allocated C string the caller frees with `cqlib_string_free`.
/// Returns the total number of available entries.
pub(crate) fn write_name_list(names: &[String], out: *mut *mut c_char, len: usize) -> usize {
    let total = names.len();
    if !out.is_null() {
        for (i, name) in names.iter().take(total.min(len)).enumerate() {
            let written = CString::new(name.as_str())
                .map(|s| s.into_raw())
                .unwrap_or(std::ptr::null_mut());
            unsafe { *out.add(i) = written };
        }
    }
    total
}

/// Creates a transform configuration with production (conservative
/// optimization) defaults.
///
/// Returns a heap-allocated `CTransformConfig*` (free with
/// `transform_config_free`), or NULL on allocation failure.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_default() -> *mut CTransformConfig {
    Box::into_raw(Box::new(CTransformConfig {
        inner: RewriteConfig::production(),
    }))
}

/// Creates a transform configuration with explicit knowledge-based lowering
/// defaults.
///
/// Returns a heap-allocated `CTransformConfig*` (free with
/// `transform_config_free`), or NULL on allocation failure.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_lowering() -> *mut CTransformConfig {
    Box::into_raw(Box::new(CTransformConfig {
        inner: RewriteConfig::lowering(),
    }))
}

/// Frees a transform configuration. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_free(ptr: *mut CTransformConfig) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Sets the rewrite mode of `config` in place.
///
/// `mode` must be one of `REWRITE_MODE_*`. Returns 0 on success, -1 on NULL,
/// or -8 on an unknown mode tag.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_with_mode(ptr: *mut CTransformConfig, mode: u8) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let mode = match mode {
        REWRITE_MODE_OPTIMIZE => RewriteMode::Optimize,
        REWRITE_MODE_LOWERING => RewriteMode::Lowering,
        _ => return CqlibError::InvalidParam as i32,
    };
    unsafe {
        (*ptr).inner = (*ptr).inner.clone().with_mode(mode);
    }
    CqlibError::Ok as i32
}

/// Sets the fixpoint round limit of `config` in place.
///
/// Returns 0 on success or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_with_max_rounds(
    ptr: *mut CTransformConfig,
    max_rounds: u8,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe {
        (*ptr).inner = (*ptr).inner.clone().with_max_rounds(max_rounds);
    }
    CqlibError::Ok as i32
}

/// Sets the matching-window operation limit of `config` in place.
///
/// Returns 0 on success or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_with_max_window_ops(
    ptr: *mut CTransformConfig,
    max_window_ops: usize,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe {
        (*ptr).inner = (*ptr).inner.clone().with_max_window_ops(max_window_ops);
    }
    CqlibError::Ok as i32
}

/// Sets the maximum matched pattern length of `config` in place.
///
/// Returns 0 on success or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_with_max_pattern_len(
    ptr: *mut CTransformConfig,
    max_pattern_len: usize,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe {
        (*ptr).inner = (*ptr).inner.clone().with_max_pattern_len(max_pattern_len);
    }
    CqlibError::Ok as i32
}

/// Replaces the enabled rule categories of `config` in place.
///
/// `kinds` points at `len` `REWRITE_KIND_*` tags (NULL when `len` is 0, which
/// disables every rule category). Returns 0 on success, -1 on NULL, or -8 on
/// an unknown kind tag (the configuration is then left unchanged).
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_with_enabled_kinds(
    ptr: *mut CTransformConfig,
    kinds: *const u8,
    len: usize,
) -> i32 {
    if ptr.is_null() || (len > 0 && kinds.is_null()) {
        return CqlibError::NullPtr as i32;
    }
    let raw = if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(kinds, len) }
    };
    let mut parsed = Vec::with_capacity(len);
    for tag in raw {
        match rule_kind_from_tag(*tag) {
            Some(kind) => parsed.push(kind),
            None => return CqlibError::InvalidParam as i32,
        }
    }
    unsafe {
        (*ptr).inner = (*ptr).inner.clone().with_enabled_kinds(parsed);
    }
    CqlibError::Ok as i32
}

/// Restricts the transform configuration to an explicit standard-gate target
/// instruction basis, parsed from `len` gate names (e.g. `"H"`, `"CX"`).
///
/// An empty basis is rejected. Returns 0 on success, -1 on NULL, -4 on
/// invalid UTF-8 or an unknown gate name, -8 for an empty basis, or -6 when
/// the core rejects the basis (the configuration is then left unchanged).
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_with_target_instructions(
    ptr: *mut CTransformConfig,
    gate_names: *const *const c_char,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let instructions = match standard_instructions_from_names(gate_names, len) {
        Ok(instructions) => instructions,
        Err(code) => return code,
    };
    let config = unsafe { &mut (*ptr).inner };
    match config.clone().try_with_target_instructions(instructions) {
        Ok(updated) => {
            *config = updated;
            CqlibError::Ok as i32
        }
        Err(CompilerError::InvalidInput(_)) => CqlibError::InvalidParam as i32,
        Err(_) => CqlibError::CompilerError as i32,
    }
}

/// Returns the `REWRITE_MODE_*` mode tag of `config`, or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_mode(ptr: *const CTransformConfig) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    match unsafe { (*ptr).inner.mode() } {
        RewriteMode::Optimize => REWRITE_MODE_OPTIMIZE as i32,
        RewriteMode::Lowering => REWRITE_MODE_LOWERING as i32,
    }
}

/// Returns the fixpoint round limit of `config`, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_max_rounds(ptr: *const CTransformConfig) -> u8 {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.max_rounds() }
}

/// Returns the matching-window operation limit of `config`, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_max_window_ops(ptr: *const CTransformConfig) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.max_window_ops() }
}

/// Returns the maximum matched pattern length of `config`, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_max_pattern_len(ptr: *const CTransformConfig) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.max_pattern_len() }
}

/// Returns 1 when `config` recurses into control-flow bodies, 0 otherwise,
/// or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_recurse_control_flow(ptr: *const CTransformConfig) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.recurses_control_flow() as i32 }
}

/// Returns 1 when `config` skips labeled operations, 0 otherwise, or -1 on
/// NULL.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_skip_labeled_ops(ptr: *const CTransformConfig) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.skips_labeled_ops() as i32 }
}

/// Returns the number of enabled rule categories of `config`, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_enabled_kinds_len(ptr: *const CTransformConfig) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.enabled_kinds().len() }
}

/// Copies the enabled rule categories of `config` into `out` as
/// `REWRITE_KIND_*` tags (two-step pattern; pair with
/// `transform_config_enabled_kinds_len`). Returns the total number of
/// categories, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_enabled_kinds(
    ptr: *const CTransformConfig,
    out: *mut u8,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let kinds = unsafe { (*ptr).inner.enabled_kinds() };
    let total = kinds.len();
    if !out.is_null() {
        for (i, kind) in kinds.iter().take(total.min(len)).enumerate() {
            unsafe { *out.add(i) = rule_kind_tag(*kind) };
        }
    }
    total
}

/// Returns the number of configured target-basis instructions of `config`
/// (0 when no target basis is set or on NULL).
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_target_instruction_basis_len(
    ptr: *const CTransformConfig,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe {
        (*ptr)
            .inner
            .target_instruction_basis()
            .map_or(0, |basis| basis.len())
    }
}

/// Copies the configured target-basis gate names of `config` into `out`
/// (two-step pattern; pair with
/// `transform_config_target_instruction_basis_len`). Each written entry is a
/// freshly allocated C string freed with `cqlib_string_free`. Returns the
/// total number of instructions, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn transform_config_target_instruction_basis(
    ptr: *const CTransformConfig,
    out: *mut *mut c_char,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let Some(basis) = (unsafe { (*ptr).inner.target_instruction_basis() }) else {
        return 0;
    };
    let names: Vec<String> = basis.iter().map(|instruction| instruction.name()).collect();
    write_name_list(&names, out, len)
}
