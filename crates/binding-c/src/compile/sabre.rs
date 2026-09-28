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

//! C ABI for the low-level SABRE routing core: `sabre_route`, initial-layout
//! normalization, reachability validation, and the SABRE configuration.

use crate::circuit::CCircuit;
use crate::device::{CDevice, CLayout};
use crate::error::CqlibError;
use cqlib_core::compile::sabre::normalize_initial_layout as core_normalize_initial_layout;
use cqlib_core::compile::sabre::sabre_route as core_sabre_route;
use cqlib_core::compile::sabre::validate_reachable_interactions as core_validate_reachable_interactions;
use cqlib_core::compile::sabre::{
    SabreConfig, SabreHeuristicConfig, SabreRoutingDiagnostics, SabreRoutingResult,
    SabreVf2PrepassConfig,
};
use cqlib_core::device::LogicalQubit;

/// Maximum number of lookahead weights representable in
/// `SabreHeuristicConfigC`.
pub const SABRE_MAX_LOOKAHEAD_WEIGHTS: usize = 8;

/// Opaque handle around a [`SabreRoutingResult`].
pub struct CSabreRoutingResult {
    pub inner: SabreRoutingResult,
}

/// Bounded VF2 prepass configuration used to seed SABRE layout candidates.
#[repr(C)]
pub struct SabreVf2PrepassConfigC {
    /// Maximum number of complete perfect mappings scored by VF2.
    pub candidate_limit: usize,
    /// Maximum number of partial mapping extensions attempted by VF2.
    pub call_limit: usize,
    /// Reserved padding to keep the struct layout stable.
    pub _reserved: [u64; 2],
}

/// SABRE swap-selection heuristic configuration.
///
/// `decay_enabled == 0` disables congestion control (the core
/// `decay_increment: None` case). `lookahead_weights` carries
/// `num_lookahead_weights` leading entries; the rest is padding.
#[repr(C)]
pub struct SabreHeuristicConfigC {
    /// Weight of the current front-layer total distance.
    pub basic_weight: f64,
    /// Weights of each active-layer-scaled lookahead-layer total.
    pub lookahead_weights: [f64; SABRE_MAX_LOOKAHEAD_WEIGHTS],
    /// Number of leading `lookahead_weights` entries in use.
    pub num_lookahead_weights: usize,
    /// Congestion multiplier increment applied after a heuristic SWAP.
    pub decay_increment: f64,
    /// 1 enables congestion control, 0 disables it.
    pub decay_enabled: u8,
    /// Number of heuristic SWAP attempts before decay values reset.
    pub decay_reset: usize,
    /// Heuristic SWAPs allowed without routing a front-layer node before
    /// the shortest-path fallback.
    pub attempt_limit: usize,
    /// Floating-point tolerance for treating candidate scores as tied.
    pub best_epsilon: f64,
    /// Reserved padding to keep the struct layout stable.
    pub _reserved: [u8; 8],
}

/// SABRE layout-refinement and routing configuration.
///
/// `vf2_prepass_enabled == 0` disables the prepass (`vf2_prepass` is then
/// ignored). `has_seed == 0` selects the unseeded default; otherwise `seed`
/// fixes a deterministic configuration.
#[repr(C)]
pub struct SabreConfigC {
    /// Randomized starting layouts added to deterministic candidates.
    pub layout_trials: usize,
    /// Maximum layout component-assignment states explored.
    pub layout_assignment_budget: usize,
    /// 1 enables the bounded VF2 prepass, 0 disables it.
    pub vf2_prepass_enabled: u8,
    /// VF2 prepass limits (used when `vf2_prepass_enabled != 0`).
    pub vf2_prepass: SabreVf2PrepassConfigC,
    /// Forward/backward refinement iterations per layout trial.
    pub refinement_iterations: usize,
    /// Complete routing trials per fully refined layout.
    pub routing_trials: usize,
    /// Deterministic seed (used when `has_seed != 0`).
    pub seed: u64,
    /// 1 uses `seed`, 0 leaves the seed unset.
    pub has_seed: u8,
    /// Swap-selection heuristic configuration.
    pub heuristic: SabreHeuristicConfigC,
    /// Reserved padding to keep the struct layout stable.
    pub _reserved: [u64; 4],
}

/// C snapshot of SABRE routing diagnostics.
#[repr(C)]
pub struct CSabreRoutingDiagnostics {
    /// Number of routing trials evaluated.
    pub trials_evaluated: usize,
    /// Zero-based index of the selected routing trial.
    pub selected_trial_index: usize,
    /// Number of times the shortest-path fallback was used.
    pub fallback_count: usize,
    /// Number of recursively routed control-flow blocks.
    pub control_flow_blocks_routed: usize,
    /// ASAP two-qubit depth of the selected routed operation stream.
    pub two_qubit_depth: usize,
    /// Total number of operations in the selected routed stream.
    pub operation_count: usize,
    /// Predicted native two-qubit operation count after device lowering.
    pub native_two_qubit_count: usize,
    /// ASAP two-qubit depth using the selected exact-qargs native plans.
    pub native_two_qubit_depth: usize,
    /// ASAP total depth using the selected exact-qargs native plans.
    pub native_total_depth: usize,
    /// Predicted total native operation count after device lowering.
    pub native_operation_count: usize,
    /// Dynamic `for`/`while` loops whose total execution cost is unknown.
    pub unknown_loop_count: usize,
    /// Number of distinct unary/pair requirement signatures prepared.
    pub requirement_signature_count: usize,
    /// Pair-placement lower-bound states retained eagerly by the target.
    pub eager_pair_state_count: usize,
    /// Lazy pair-state lower-bound probes made by the selected trial.
    pub lazy_pair_l1_lookup_count: usize,
    /// Selected-trial lazy pair-state probes served by its local L1 cache.
    pub lazy_pair_l1_hit_count: usize,
    /// Pair-state entries retained by the selected trial's bounded L1 cache.
    pub lazy_pair_l1_cached_count: usize,
}

/// Converts a C SABRE configuration into the core form.
///
/// Returns `Err(-8)` when `num_lookahead_weights` exceeds
/// `SABRE_MAX_LOOKAHEAD_WEIGHTS`.
pub(crate) fn sabre_config_from_c(raw: &SabreConfigC) -> Result<SabreConfig, i32> {
    if raw.heuristic.num_lookahead_weights > SABRE_MAX_LOOKAHEAD_WEIGHTS {
        return Err(CqlibError::InvalidParam as i32);
    }
    let lookahead_weights =
        raw.heuristic.lookahead_weights[..raw.heuristic.num_lookahead_weights].to_vec();
    let vf2_prepass = if raw.vf2_prepass_enabled != 0 {
        Some(SabreVf2PrepassConfig {
            candidate_limit: raw.vf2_prepass.candidate_limit,
            call_limit: raw.vf2_prepass.call_limit,
        })
    } else {
        None
    };
    Ok(SabreConfig {
        layout_trials: raw.layout_trials,
        layout_assignment_budget: raw.layout_assignment_budget,
        vf2_prepass,
        refinement_iterations: raw.refinement_iterations,
        routing_trials: raw.routing_trials,
        seed: if raw.has_seed != 0 {
            Some(raw.seed)
        } else {
            None
        },
        heuristic: SabreHeuristicConfig {
            basic_weight: raw.heuristic.basic_weight,
            lookahead_weights,
            decay_increment: if raw.heuristic.decay_enabled != 0 {
                Some(raw.heuristic.decay_increment)
            } else {
                None
            },
            decay_reset: raw.heuristic.decay_reset,
            attempt_limit: raw.heuristic.attempt_limit,
            best_epsilon: raw.heuristic.best_epsilon,
        },
    })
}

/// Converts a core SABRE configuration into the C form.
fn sabre_config_to_c(config: &SabreConfig) -> SabreConfigC {
    let mut lookahead_weights = [0.0f64; SABRE_MAX_LOOKAHEAD_WEIGHTS];
    let num = config
        .heuristic
        .lookahead_weights
        .len()
        .min(SABRE_MAX_LOOKAHEAD_WEIGHTS);
    lookahead_weights[..num].copy_from_slice(&config.heuristic.lookahead_weights[..num]);
    SabreConfigC {
        layout_trials: config.layout_trials,
        layout_assignment_budget: config.layout_assignment_budget,
        vf2_prepass_enabled: config.vf2_prepass.is_some() as u8,
        vf2_prepass: SabreVf2PrepassConfigC {
            candidate_limit: config.vf2_prepass.map(|c| c.candidate_limit).unwrap_or(0),
            call_limit: config.vf2_prepass.map(|c| c.call_limit).unwrap_or(0),
            _reserved: [0; 2],
        },
        refinement_iterations: config.refinement_iterations,
        routing_trials: config.routing_trials,
        seed: config.seed.unwrap_or(0),
        has_seed: config.seed.is_some() as u8,
        heuristic: SabreHeuristicConfigC {
            basic_weight: config.heuristic.basic_weight,
            lookahead_weights,
            num_lookahead_weights: num,
            decay_increment: config.heuristic.decay_increment.unwrap_or(0.0),
            decay_enabled: config.heuristic.decay_increment.is_some() as u8,
            decay_reset: config.heuristic.decay_reset,
            attempt_limit: config.heuristic.attempt_limit,
            best_epsilon: config.heuristic.best_epsilon,
            _reserved: [0; 8],
        },
        _reserved: [0; 4],
    }
}

/// Resolves a C SABRE configuration pointer, falling back to the core
/// default when `config` is NULL.
pub(crate) fn resolve_sabre_config(config: *const SabreConfigC) -> Result<SabreConfig, i32> {
    if config.is_null() {
        Ok(SabreConfig::default())
    } else {
        sabre_config_from_c(unsafe { &*config })
    }
}

/// Writes a routing-diagnostics snapshot to `out`.
pub(crate) fn write_sabre_routing_diagnostics(
    diagnostics: &SabreRoutingDiagnostics,
    out: *mut CSabreRoutingDiagnostics,
) {
    unsafe {
        *out = CSabreRoutingDiagnostics {
            trials_evaluated: diagnostics.trials_evaluated,
            selected_trial_index: diagnostics.selected_trial_index,
            fallback_count: diagnostics.fallback_count,
            control_flow_blocks_routed: diagnostics.control_flow_blocks_routed,
            two_qubit_depth: diagnostics.two_qubit_depth,
            operation_count: diagnostics.operation_count,
            native_two_qubit_count: diagnostics.native_two_qubit_count,
            native_two_qubit_depth: diagnostics.native_two_qubit_depth,
            native_total_depth: diagnostics.native_total_depth,
            native_operation_count: diagnostics.native_operation_count,
            unknown_loop_count: diagnostics.unknown_loop_count,
            requirement_signature_count: diagnostics.requirement_signature_count,
            eager_pair_state_count: diagnostics.eager_pair_state_count,
            lazy_pair_l1_lookup_count: diagnostics.lazy_pair_l1_lookup_count,
            lazy_pair_l1_hit_count: diagnostics.lazy_pair_l1_hit_count,
            lazy_pair_l1_cached_count: diagnostics.lazy_pair_l1_cached_count,
        };
    }
}

/// Returns the default SABRE configuration by value.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_config_default() -> SabreConfigC {
    sabre_config_to_c(&SabreConfig::default())
}

/// Returns a compact deterministic SABRE configuration for reproducible
/// runs, fixing the random seed to `seed`.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_config_deterministic_seeded(seed: u64) -> SabreConfigC {
    sabre_config_to_c(&SabreConfig::deterministic_seeded(seed))
}

/// Validates the routing fields of a C SABRE configuration.
///
/// Returns 0 when valid, -1 on NULL, -8 when the configuration cannot be
/// represented, or -6 when the core validation rejects it.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_config_validate(config: *const SabreConfigC) -> i32 {
    if config.is_null() {
        return CqlibError::NullPtr as i32;
    }
    match sabre_config_from_c(unsafe { &*config }) {
        Ok(config) => match config.validate() {
            Ok(()) => CqlibError::Ok as i32,
            Err(_) => CqlibError::CompilerError as i32,
        },
        Err(code) => code,
    }
}

/// Routes `circuit` on `device` from a caller-supplied initial layout.
///
/// `config` may be NULL to use the core default. Returns a heap-allocated
/// `CSabreRoutingResult*`, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_route(
    circuit: *const CCircuit,
    device: *const CDevice,
    initial_layout: *const CLayout,
    config: *const SabreConfigC,
) -> *mut CSabreRoutingResult {
    if circuit.is_null() || device.is_null() || initial_layout.is_null() {
        return std::ptr::null_mut();
    }
    let config = match resolve_sabre_config(config) {
        Ok(config) => config,
        Err(_) => return std::ptr::null_mut(),
    };
    let circuit = unsafe { &(*circuit).inner };
    let device = unsafe { &(*device).inner };
    let layout = unsafe { &(*initial_layout).inner };
    match core_sabre_route(circuit, device, layout, &config) {
        Ok(result) => Box::into_raw(Box::new(CSabreRoutingResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a SABRE routing result. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_routing_result_free(ptr: *mut CSabreRoutingResult) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the routed physical circuit as an owned `CCircuit*` (free with
/// `circuit_free`). Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_routing_result_circuit(ptr: *const CSabreRoutingResult) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    Box::into_raw(Box::new(CCircuit {
        inner: result.inner.circuit.clone(),
    }))
}

/// Returns the initial logical-to-physical layout used by the selected
/// trial, as an owned `CLayout*` (free with `layout_free`). Returns NULL on
/// error.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_routing_result_initial_layout(
    ptr: *const CSabreRoutingResult,
) -> *mut CLayout {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    Box::into_raw(Box::new(CLayout {
        inner: result.inner.initial_layout.clone(),
    }))
}

/// Returns the final logical-to-physical layout after all routed
/// operations, as an owned `CLayout*` (free with `layout_free`). Returns
/// NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_routing_result_final_layout(
    ptr: *const CSabreRoutingResult,
) -> *mut CLayout {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    Box::into_raw(Box::new(CLayout {
        inner: result.inner.final_layout.clone(),
    }))
}

/// Returns the number of inserted SWAP operations (including control-flow
/// epilogues), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_routing_result_swap_count(ptr: *const CSabreRoutingResult) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.swap_count }
}

/// Writes the routing-diagnostics snapshot to `*out`.
///
/// Returns 0 on success or -1 on NULL arguments.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_routing_result_diagnostics(
    ptr: *const CSabreRoutingResult,
    out: *mut CSabreRoutingDiagnostics,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let result = unsafe { &*ptr };
    write_sabre_routing_diagnostics(&result.inner.diagnostics, out);
    CqlibError::Ok as i32
}

/// Normalizes a full logical-to-physical layout onto the device's usable
/// qubits.
///
/// `logical_qubits` is an array of `num_logical` u32 logical qubit IDs.
/// Returns an owned `CLayout*` (free with `layout_free`), or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn normalize_initial_layout(
    logical_qubits: *const u32,
    num_logical: usize,
    device: *const CDevice,
    initial_layout: *const CLayout,
) -> *mut CLayout {
    if device.is_null() || initial_layout.is_null() || (num_logical > 0 && logical_qubits.is_null())
    {
        return std::ptr::null_mut();
    }
    let logical: Vec<LogicalQubit> = if num_logical == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(logical_qubits, num_logical) }
            .iter()
            .map(|&q| LogicalQubit::new(q))
            .collect()
    };
    let device = unsafe { &(*device).inner };
    let layout = unsafe { &(*initial_layout).inner };
    match core_normalize_initial_layout(&logical, device, layout) {
        Ok(layout) => Box::into_raw(Box::new(CLayout { inner: layout })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Validates that every circuit interaction is reachable from
/// `initial_layout` without executing routing.
///
/// Returns 0 when reachable, -1 on NULL arguments, or -6 when the core
/// validation fails.
#[unsafe(no_mangle)]
pub extern "C" fn validate_reachable_interactions(
    circuit: *const CCircuit,
    device: *const CDevice,
    initial_layout: *const CLayout,
) -> i32 {
    if circuit.is_null() || device.is_null() || initial_layout.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let circuit = unsafe { &(*circuit).inner };
    let device = unsafe { &(*device).inner };
    let layout = unsafe { &(*initial_layout).inner };
    match core_validate_reachable_interactions(circuit, device, layout) {
        Ok(()) => CqlibError::Ok as i32,
        Err(_) => CqlibError::CompilerError as i32,
    }
}
