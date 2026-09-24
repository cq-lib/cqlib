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

//! C ABI for initial-layout algorithms and layout analysis tools.

use crate::circuit::CCircuit;
use crate::compile::sabre::{SabreConfigC, resolve_sabre_config};
use crate::device::{CDevice, CLayout, standard_gate_from_name, write_u32_list};
use crate::error::CqlibError;
use cqlib_core::compile::CompilerError;
use cqlib_core::compile::physical_target::PhysicalLayoutGraph;
use cqlib_core::compile::transform::layout::analyze_circuit_for_layout as core_analyze_circuit_for_layout;
use cqlib_core::compile::transform::layout::{
    CircuitLayoutAnalysis, Interaction, LayoutObjective, LayoutResult, LayoutScore,
    PreparedSabreCircuit, PreparedSabreTarget, Vf2EdgeRequirement, Vf2LayoutConfig,
};
use cqlib_core::compile::transform::layout::{
    greedy_layout as core_greedy_layout, greedy_layout_prepared as core_greedy_layout_prepared,
};
use cqlib_core::compile::transform::layout::{
    prepare_sabre_circuit as core_prepare_sabre_circuit,
    prepare_sabre_device_target as core_prepare_sabre_device_target,
};
use cqlib_core::compile::transform::layout::{
    sabre_layout as core_sabre_layout, sabre_layout_prepared as core_sabre_layout_prepared,
};
use cqlib_core::compile::transform::layout::{
    trivial_layout as core_trivial_layout, trivial_layout_prepared as core_trivial_layout_prepared,
};
use cqlib_core::compile::transform::layout::{
    vf2_perfect_layout as core_vf2_perfect_layout,
    vf2_perfect_layout_prepared as core_vf2_perfect_layout_prepared,
};
use cqlib_core::device::{Device, PhysicalQubit};
use std::ffi::CString;
use std::os::raw::c_char;

/// Opaque handle around a [`LayoutResult`].
pub struct CLayoutResult {
    pub inner: LayoutResult,
}

/// Opaque handle around a [`CircuitLayoutAnalysis`].
pub struct CCircuitLayoutAnalysis {
    pub inner: CircuitLayoutAnalysis,
}

/// Opaque handle around a [`PhysicalLayoutGraph`].
pub struct CPhysicalLayoutGraph {
    pub inner: PhysicalLayoutGraph,
}

/// Opaque handle around a [`PreparedSabreCircuit`].
pub struct CPreparedSabreCircuit {
    pub inner: PreparedSabreCircuit,
}

/// Opaque handle around a [`PreparedSabreTarget`].
pub struct CPreparedSabreTarget {
    pub inner: PreparedSabreTarget,
}

/// C snapshot of a [`LayoutScore`].
#[repr(C)]
pub struct CLayoutScore {
    /// Weighted total according to the objective.
    pub total: f64,
    /// Raw weighted-distance component.
    pub distance: f64,
    /// Raw direction-mismatch component.
    pub direction: f64,
    /// Unweighted effective two-qubit placement cost.
    pub two_qubit_error: f64,
    /// Raw readout error component.
    pub readout_error: f64,
    /// 1 when the objective used fidelity terms.
    pub used_fidelity: u8,
    /// Reserved padding to keep the struct layout stable.
    pub _reserved: [u8; 7],
}

/// C snapshot of one logical-qubit interaction.
#[repr(C)]
pub struct CInteraction {
    /// Lower-sorted logical qubit endpoint.
    pub left: u32,
    /// Higher-sorted logical qubit endpoint.
    pub right: u32,
    /// Total interaction weight for this unordered pair.
    pub weight: f64,
    /// Weight observed in operation order `left -> right`.
    pub directed_weight_left_to_right: f64,
    /// Weight observed in operation order `right -> left`.
    pub directed_weight_right_to_left: f64,
    /// First appearance index in operation order.
    pub first_seen_order: usize,
}

/// Layout objective tag.
/// | Value | Objective                                        |
/// |-------|--------------------------------------------------|
/// | 0     | Topology only (distance + direction mismatch)   |
/// | 1     | Fidelity aware (adds error-rate terms)           |
/// | 2     | Auto from the device's calibration data         |
/// | 3     | Fidelity required (errors without calibration)  |
pub const LAYOUT_OBJECTIVE_TOPOLOGY_ONLY: u8 = 0;
pub const LAYOUT_OBJECTIVE_FIDELITY_AWARE: u8 = 1;
pub const LAYOUT_OBJECTIVE_AUTO: u8 = 2;
pub const LAYOUT_OBJECTIVE_FIDELITY_REQUIRED: u8 = 3;

/// VF2 edge-requirement tag.
/// | Value | Requirement                  |
/// |-------|------------------------------|
/// | 0     | Positive interactions only   |
/// | 1     | All interactions             |
pub const VF2_EDGE_REQUIREMENT_POSITIVE_INTERACTIONS: u8 = 0;
pub const VF2_EDGE_REQUIREMENT_ALL_INTERACTIONS: u8 = 1;

/// Sentinel for `CVf2LayoutConfig::call_limit` meaning "no explicit limit".
pub const VF2_CALL_LIMIT_NONE: usize = usize::MAX;

/// C form of [`Vf2LayoutConfig`].
#[repr(C)]
pub struct CVf2LayoutConfig {
    /// Maximum number of complete perfect candidates to score.
    pub candidate_limit: usize,
    /// Maximum number of partial mapping extensions attempted
    /// (`VF2_CALL_LIMIT_NONE` means no explicit limit).
    pub call_limit: usize,
    /// One of `VF2_EDGE_REQUIREMENT_*`.
    pub edge_requirement: u8,
    /// Reserved padding to keep the struct layout stable.
    pub _reserved: [u8; 7],
}

/// Resolves a layout-objective tag against an optional prepared physical
/// graph and/or device.
///
/// `LAYOUT_OBJECTIVE_AUTO` resolves `LayoutObjective::auto_from_physical`
/// against the physical graph (derived from `device` when no prepared graph
/// is supplied). `LAYOUT_OBJECTIVE_FIDELITY_REQUIRED` resolves
/// `LayoutObjective::fidelity_required` and errors when the target has no
/// usable calibration data. Returns `Ok(None)` for an unknown tag or when the
/// physical graph cannot be derived from the device.
pub(crate) fn objective_from_tag(
    tag: u8,
    physical: Option<&PhysicalLayoutGraph>,
    device: Option<&Device>,
) -> Result<Option<LayoutObjective>, CompilerError> {
    match tag {
        LAYOUT_OBJECTIVE_TOPOLOGY_ONLY => Ok(Some(LayoutObjective::topology_only())),
        LAYOUT_OBJECTIVE_FIDELITY_AWARE => Ok(Some(LayoutObjective::fidelity_aware())),
        LAYOUT_OBJECTIVE_AUTO => {
            let physical = match physical {
                Some(physical) => Some(physical.clone()),
                None => device.and_then(|device| PhysicalLayoutGraph::from_device(device).ok()),
            };
            Ok(physical.map(|physical| LayoutObjective::auto_from_physical(&physical)))
        }
        LAYOUT_OBJECTIVE_FIDELITY_REQUIRED => {
            let physical = match physical {
                Some(physical) => physical.clone(),
                None => {
                    let Some(physical) =
                        device.and_then(|device| PhysicalLayoutGraph::from_device(device).ok())
                    else {
                        return Ok(None);
                    };
                    physical
                }
            };
            LayoutObjective::fidelity_required(&physical).map(Some)
        }
        _ => Ok(None),
    }
}

/// Converts a C VF2 layout configuration pointer into the core form.
/// A NULL pointer selects the core default. Returns None for an unknown
/// edge-requirement tag.
fn vf2_config_from_c(config: *const CVf2LayoutConfig) -> Option<Vf2LayoutConfig> {
    if config.is_null() {
        return Some(Vf2LayoutConfig::default());
    }
    let raw = unsafe { &*config };
    let edge_requirement = match raw.edge_requirement {
        VF2_EDGE_REQUIREMENT_POSITIVE_INTERACTIONS => Vf2EdgeRequirement::PositiveInteractions,
        VF2_EDGE_REQUIREMENT_ALL_INTERACTIONS => Vf2EdgeRequirement::AllInteractions,
        _ => return None,
    };
    Some(Vf2LayoutConfig {
        candidate_limit: raw.candidate_limit,
        call_limit: if raw.call_limit == VF2_CALL_LIMIT_NONE {
            None
        } else {
            Some(raw.call_limit)
        },
        edge_requirement,
    })
}

/// Returns the default VF2 layout configuration by value.
#[unsafe(no_mangle)]
pub extern "C" fn vf2_layout_config_default() -> CVf2LayoutConfig {
    CVf2LayoutConfig {
        candidate_limit: 10,
        call_limit: VF2_CALL_LIMIT_NONE,
        edge_requirement: VF2_EDGE_REQUIREMENT_POSITIVE_INTERACTIONS,
        _reserved: [0; 7],
    }
}

// =====  Layout entry points  =====

/// Maps logical qubits to usable physical qubits in their existing order.
///
/// `objective` is one of the `LAYOUT_OBJECTIVE_*` tags. Returns a
/// heap-allocated `CLayoutResult*`, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn trivial_layout(
    circuit: *const CCircuit,
    device: *const CDevice,
    objective: u8,
) -> *mut CLayoutResult {
    if circuit.is_null() || device.is_null() {
        return std::ptr::null_mut();
    }
    let device = unsafe { &(*device).inner };
    let Ok(Some(objective)) = objective_from_tag(objective, None, Some(device)) else {
        return std::ptr::null_mut();
    };
    let circuit = unsafe { &(*circuit).inner };
    match core_trivial_layout(circuit, device, &objective) {
        Ok(result) => Box::into_raw(Box::new(CLayoutResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Builds a greedy layout by placing qubits in interaction-strength order.
///
/// `objective` is one of the `LAYOUT_OBJECTIVE_*` tags. Returns a
/// heap-allocated `CLayoutResult*`, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn greedy_layout(
    circuit: *const CCircuit,
    device: *const CDevice,
    objective: u8,
) -> *mut CLayoutResult {
    if circuit.is_null() || device.is_null() {
        return std::ptr::null_mut();
    }
    let device = unsafe { &(*device).inner };
    let Ok(Some(objective)) = objective_from_tag(objective, None, Some(device)) else {
        return std::ptr::null_mut();
    };
    let circuit = unsafe { &(*circuit).inner };
    match core_greedy_layout(circuit, device, &objective) {
        Ok(result) => Box::into_raw(Box::new(CLayoutResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Searches for a perfect initial layout using non-induced VF2++ matching.
///
/// `objective` is one of the `LAYOUT_OBJECTIVE_*` tags; `config` may be NULL
/// to use the default. Returns a heap-allocated `CLayoutResult*`, or NULL
/// when no perfect mapping exists or on error.
#[unsafe(no_mangle)]
pub extern "C" fn vf2_perfect_layout(
    circuit: *const CCircuit,
    device: *const CDevice,
    objective: u8,
    config: *const CVf2LayoutConfig,
) -> *mut CLayoutResult {
    if circuit.is_null() || device.is_null() {
        return std::ptr::null_mut();
    }
    let device = unsafe { &(*device).inner };
    let Ok(Some(objective)) = objective_from_tag(objective, None, Some(device)) else {
        return std::ptr::null_mut();
    };
    let Some(config) = vf2_config_from_c(config) else {
        return std::ptr::null_mut();
    };
    let circuit = unsafe { &(*circuit).inner };
    match core_vf2_perfect_layout(circuit, device, &objective, &config) {
        Ok(result) => Box::into_raw(Box::new(CLayoutResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Selects an initial layout using SABRE layout refinement.
///
/// `objective` is one of the `LAYOUT_OBJECTIVE_*` tags; `config` may be NULL
/// to use the default. Returns a heap-allocated `CLayoutResult*`, or NULL on
/// error.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_layout(
    circuit: *const CCircuit,
    device: *const CDevice,
    objective: u8,
    config: *const SabreConfigC,
) -> *mut CLayoutResult {
    if circuit.is_null() || device.is_null() {
        return std::ptr::null_mut();
    }
    let device = unsafe { &(*device).inner };
    let Ok(Some(objective)) = objective_from_tag(objective, None, Some(device)) else {
        return std::ptr::null_mut();
    };
    let config = match resolve_sabre_config(config) {
        Ok(config) => config,
        Err(_) => return std::ptr::null_mut(),
    };
    let circuit = unsafe { &(*circuit).inner };
    match core_sabre_layout(circuit, device, &objective, &config) {
        Ok(result) => Box::into_raw(Box::new(CLayoutResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

// =====  Prepared entry points  =====

/// Analyzes a circuit for layout planning (weighted logical interactions).
///
/// Returns a heap-allocated `CCircuitLayoutAnalysis*`, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn analyze_circuit_for_layout(
    circuit: *const CCircuit,
) -> *mut CCircuitLayoutAnalysis {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner };
    match core_analyze_circuit_for_layout(circuit) {
        Ok(analysis) => Box::into_raw(Box::new(CCircuitLayoutAnalysis { inner: analysis })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a circuit layout analysis. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_layout_analysis_free(ptr: *mut CCircuitLayoutAnalysis) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the number of logical qubits in the analysis, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_layout_analysis_num_logical(ptr: *const CCircuitLayoutAnalysis) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.logical_qubits.len() }
}

/// Copies the logical qubit IDs in source-circuit order into `out`
/// (two-step pattern; pair with `circuit_layout_analysis_num_logical`).
/// Returns the total count.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_layout_analysis_logical_qubits(
    ptr: *const CCircuitLayoutAnalysis,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let items: Vec<u32> = unsafe { (*ptr).inner.logical_qubits.iter().map(|q| q.id()).collect() };
    write_u32_list(&items, out, len)
}

/// Returns the number of weighted logical interactions, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_layout_analysis_interactions_len(
    ptr: *const CCircuitLayoutAnalysis,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.interactions.interactions().len() }
}

/// Writes the interaction at `index` to `*out`.
///
/// Returns 0 on success, -1 on NULL arguments, or -8 when `index` is out of
/// bounds.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_layout_analysis_interaction(
    ptr: *const CCircuitLayoutAnalysis,
    index: usize,
    out: *mut CInteraction,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let analysis = unsafe { &(*ptr).inner };
    match analysis.interactions.interactions().get(index) {
        Some(interaction) => {
            write_interaction(interaction, out);
            CqlibError::Ok as i32
        }
        None => CqlibError::InvalidParam as i32,
    }
}

/// Writes one interaction snapshot.
fn write_interaction(interaction: &Interaction, out: *mut CInteraction) {
    unsafe {
        *out = CInteraction {
            left: interaction.left.id(),
            right: interaction.right.id(),
            weight: interaction.weight,
            directed_weight_left_to_right: interaction.directed_weight_left_to_right,
            directed_weight_right_to_left: interaction.directed_weight_right_to_left,
            first_seen_order: interaction.first_seen_order,
        };
    }
}

// =====  Physical layout graph  =====

/// Builds the compiler-local physical topology and calibration view of a
/// device. Returns a heap-allocated `CPhysicalLayoutGraph*`, or NULL on
/// error.
#[unsafe(no_mangle)]
pub extern "C" fn physical_layout_graph_from_device(
    device: *const CDevice,
) -> *mut CPhysicalLayoutGraph {
    if device.is_null() {
        return std::ptr::null_mut();
    }
    let device = unsafe { &(*device).inner };
    match PhysicalLayoutGraph::from_device(device) {
        Ok(graph) => Box::into_raw(Box::new(CPhysicalLayoutGraph { inner: graph })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a physical layout graph. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn physical_layout_graph_free(ptr: *mut CPhysicalLayoutGraph) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the number of usable physical qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn physical_layout_graph_num_physical(ptr: *const CPhysicalLayoutGraph) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.physical_qubits().len() }
}

/// Copies the usable physical qubit IDs in ascending order into `out`
/// (two-step pattern; pair with `physical_layout_graph_num_physical`).
/// Returns the total count.
#[unsafe(no_mangle)]
pub extern "C" fn physical_layout_graph_physical_qubits(
    ptr: *const CPhysicalLayoutGraph,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let items: Vec<u32> = unsafe { (*ptr).inner.physical_qubits() }
        .iter()
        .map(|q| q.id())
        .collect();
    write_u32_list(&items, out, len)
}

/// Returns the undirected shortest-path distance between two physical
/// qubits, or `UINT32_MAX` when the pair is disconnected or unknown.
#[unsafe(no_mangle)]
pub extern "C" fn physical_layout_graph_distance(
    ptr: *const CPhysicalLayoutGraph,
    a: u32,
    b: u32,
) -> u32 {
    if ptr.is_null() {
        return u32::MAX;
    }
    let graph = unsafe { &(*ptr).inner };
    graph
        .distance(PhysicalQubit::new(a), PhysicalQubit::new(b))
        .unwrap_or(u32::MAX)
}

/// Returns 1 when the two physical qubits are adjacent (undirected), 0
/// otherwise, or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn physical_layout_graph_is_adjacent_undirected(
    ptr: *const CPhysicalLayoutGraph,
    a: u32,
    b: u32,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let graph = unsafe { &(*ptr).inner };
    graph.is_adjacent_undirected(PhysicalQubit::new(a), PhysicalQubit::new(b)) as i32
}

/// Returns the readout error rate of one physical qubit.
///
/// Returns 0 with the value in `*out`, or -8 when no readout data is
/// recorded for the qubit.
#[unsafe(no_mangle)]
pub extern "C" fn physical_layout_graph_readout_error(
    ptr: *const CPhysicalLayoutGraph,
    qubit: u32,
    out: *mut f64,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let graph = unsafe { &(*ptr).inner };
    match graph.readout_error(PhysicalQubit::new(qubit)) {
        Some(error) => {
            unsafe { *out = error };
            CqlibError::Ok as i32
        }
        None => CqlibError::InvalidParam as i32,
    }
}

/// Returns 1 when the directed coupling `control -> target` exists, 0
/// otherwise, or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn physical_layout_graph_supports_directed_coupling(
    ptr: *const CPhysicalLayoutGraph,
    control: u32,
    target: u32,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let graph = unsafe { &(*ptr).inner };
    graph.supports_directed_coupling(PhysicalQubit::new(control), PhysicalQubit::new(target)) as i32
}

/// Returns 1 when `gate_name` is a native capability on the directed edge
/// `control -> target`, 0 when not, -1 on NULL, or -4 for an unknown gate
/// name.
#[unsafe(no_mangle)]
pub extern "C" fn physical_layout_graph_supports_two_qubit_gate_directed(
    ptr: *const CPhysicalLayoutGraph,
    gate_name: *const c_char,
    control: u32,
    target: u32,
) -> i32 {
    let gate = match parse_gate_name(gate_name) {
        Ok(gate) => gate,
        Err(code) => return code,
    };
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let graph = unsafe { &(*ptr).inner };
    graph.supports_two_qubit_gate_directed(
        PhysicalQubit::new(control),
        PhysicalQubit::new(target),
        gate,
    ) as i32
}

/// Returns the calibrated error rate of `gate_name` on the directed edge
/// `control -> target`.
///
/// Returns 0 with the value in `*out`, -8 when the gate is unsupported or
/// uncalibrated on the edge, -1 on NULL, or -4 for an unknown gate name.
#[unsafe(no_mangle)]
pub extern "C" fn physical_layout_graph_two_qubit_gate_error_directed(
    ptr: *const CPhysicalLayoutGraph,
    gate_name: *const c_char,
    control: u32,
    target: u32,
    out: *mut f64,
) -> i32 {
    let gate = match parse_gate_name(gate_name) {
        Ok(gate) => gate,
        Err(code) => return code,
    };
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let graph = unsafe { &(*ptr).inner };
    match graph.two_qubit_gate_error_directed(
        PhysicalQubit::new(control),
        PhysicalQubit::new(target),
        gate,
    ) {
        Some(error) => {
            unsafe { *out = error };
            CqlibError::Ok as i32
        }
        None => CqlibError::InvalidParam as i32,
    }
}

/// Returns 1 when any readout or two-qubit calibration data is available, 0
/// otherwise, or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn physical_layout_graph_has_fidelity_data(ptr: *const CPhysicalLayoutGraph) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.has_fidelity_data() as i32 }
}

/// Returns 1 when readout-error data is available, 0 otherwise, or -1 for
/// NULL.
#[unsafe(no_mangle)]
pub extern "C" fn physical_layout_graph_has_readout_error_data(
    ptr: *const CPhysicalLayoutGraph,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.has_readout_error_data() as i32 }
}

/// Returns 1 when two-qubit error data is available, 0 otherwise, or -1 for
/// NULL.
#[unsafe(no_mangle)]
pub extern "C" fn physical_layout_graph_has_two_qubit_error_data(
    ptr: *const CPhysicalLayoutGraph,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.has_two_qubit_error_data() as i32 }
}

/// Resolves a two-qubit standard-gate name to a `StandardGate`.
fn parse_gate_name(
    gate_name: *const c_char,
) -> Result<cqlib_core::circuit::gate::StandardGate, i32> {
    if gate_name.is_null() {
        return Err(CqlibError::NullPtr as i32);
    }
    let name = match unsafe { std::ffi::CStr::from_ptr(gate_name) }.to_str() {
        Ok(name) => name,
        Err(_) => return Err(CqlibError::ParseError as i32),
    };
    standard_gate_from_name(name).ok_or(CqlibError::ParseError as i32)
}

// =====  Prepared SABRE circuit/target  =====

/// Prepares the circuit-side analysis and dependency models used by SABRE.
///
/// Returns a heap-allocated `CPreparedSabreCircuit*`, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn prepare_sabre_circuit(circuit: *const CCircuit) -> *mut CPreparedSabreCircuit {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner };
    match core_prepare_sabre_circuit(circuit) {
        Ok(prepared) => Box::into_raw(Box::new(CPreparedSabreCircuit { inner: prepared })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a prepared SABRE circuit. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn prepared_sabre_circuit_free(ptr: *mut CPreparedSabreCircuit) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the number of logical qubits of the prepared circuit, or 0 for
/// NULL.
#[unsafe(no_mangle)]
pub extern "C" fn prepared_sabre_circuit_logical_qubits_len(
    ptr: *const CPreparedSabreCircuit,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.logical_qubits().len() }
}

/// Copies the prepared circuit's logical qubit IDs in source-circuit order
/// into `out` (two-step pattern; pair with
/// `prepared_sabre_circuit_logical_qubits_len`). Returns the total count.
#[unsafe(no_mangle)]
pub extern "C" fn prepared_sabre_circuit_logical_qubits(
    ptr: *const CPreparedSabreCircuit,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let items: Vec<u32> = unsafe { (*ptr).inner.logical_qubits() }
        .iter()
        .map(|q| q.id())
        .collect();
    write_u32_list(&items, out, len)
}

/// Prepares exact device-native SABRE data for a prepared circuit.
///
/// Returns a heap-allocated `CPreparedSabreTarget*`, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn prepare_sabre_device_target(
    prepared: *const CPreparedSabreCircuit,
    device: *const CDevice,
) -> *mut CPreparedSabreTarget {
    if prepared.is_null() || device.is_null() {
        return std::ptr::null_mut();
    }
    let prepared = unsafe { &(*prepared).inner };
    let device = unsafe { &(*device).inner };
    match core_prepare_sabre_device_target(prepared, device) {
        Ok(target) => Box::into_raw(Box::new(CPreparedSabreTarget { inner: target })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a prepared SABRE target. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn prepared_sabre_target_free(ptr: *mut CPreparedSabreTarget) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the physical graph used by layout objective scoring, as an owned
/// `CPhysicalLayoutGraph*` (free with `physical_layout_graph_free`).
/// Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn prepared_sabre_target_physical(
    ptr: *const CPreparedSabreTarget,
) -> *mut CPhysicalLayoutGraph {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let target = unsafe { &*ptr };
    Box::into_raw(Box::new(CPhysicalLayoutGraph {
        inner: target.inner.physical().clone(),
    }))
}

// =====  Prepared layout algorithms  =====

/// Trivial layout over a prepared analysis and physical graph.
///
/// `objective` is one of the `LAYOUT_OBJECTIVE_*` tags. Returns a
/// heap-allocated `CLayoutResult*`, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn trivial_layout_prepared(
    analysis: *const CCircuitLayoutAnalysis,
    physical: *const CPhysicalLayoutGraph,
    objective: u8,
) -> *mut CLayoutResult {
    if analysis.is_null() || physical.is_null() {
        return std::ptr::null_mut();
    }
    let physical = unsafe { &(*physical).inner };
    let Ok(Some(objective)) = objective_from_tag(objective, Some(physical), None) else {
        return std::ptr::null_mut();
    };
    let analysis = unsafe { &(*analysis).inner };
    match core_trivial_layout_prepared(analysis, physical, &objective) {
        Ok(result) => Box::into_raw(Box::new(CLayoutResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Greedy layout over a prepared analysis and physical graph.
#[unsafe(no_mangle)]
pub extern "C" fn greedy_layout_prepared(
    analysis: *const CCircuitLayoutAnalysis,
    physical: *const CPhysicalLayoutGraph,
    objective: u8,
) -> *mut CLayoutResult {
    if analysis.is_null() || physical.is_null() {
        return std::ptr::null_mut();
    }
    let physical = unsafe { &(*physical).inner };
    let Ok(Some(objective)) = objective_from_tag(objective, Some(physical), None) else {
        return std::ptr::null_mut();
    };
    let analysis = unsafe { &(*analysis).inner };
    match core_greedy_layout_prepared(analysis, physical, &objective) {
        Ok(result) => Box::into_raw(Box::new(CLayoutResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// VF2 perfect layout over a prepared analysis and physical graph.
///
/// `config` may be NULL to use the default.
#[unsafe(no_mangle)]
pub extern "C" fn vf2_perfect_layout_prepared(
    analysis: *const CCircuitLayoutAnalysis,
    physical: *const CPhysicalLayoutGraph,
    objective: u8,
    config: *const CVf2LayoutConfig,
) -> *mut CLayoutResult {
    if analysis.is_null() || physical.is_null() {
        return std::ptr::null_mut();
    }
    let physical = unsafe { &(*physical).inner };
    let Ok(Some(objective)) = objective_from_tag(objective, Some(physical), None) else {
        return std::ptr::null_mut();
    };
    let Some(config) = vf2_config_from_c(config) else {
        return std::ptr::null_mut();
    };
    let analysis = unsafe { &(*analysis).inner };
    match core_vf2_perfect_layout_prepared(analysis, physical, &objective, &config) {
        Ok(result) => Box::into_raw(Box::new(CLayoutResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// SABRE layout over a prepared circuit and prepared target.
///
/// `config` may be NULL to use the default.
#[unsafe(no_mangle)]
pub extern "C" fn sabre_layout_prepared(
    prepared: *const CPreparedSabreCircuit,
    prepared_target: *const CPreparedSabreTarget,
    objective: u8,
    config: *const SabreConfigC,
) -> *mut CLayoutResult {
    if prepared.is_null() || prepared_target.is_null() {
        return std::ptr::null_mut();
    }
    let prepared = unsafe { &(*prepared).inner };
    let prepared_target = unsafe { &(*prepared_target).inner };
    let Ok(Some(objective)) = objective_from_tag(objective, Some(prepared_target.physical()), None)
    else {
        return std::ptr::null_mut();
    };
    let config = match resolve_sabre_config(config) {
        Ok(config) => config,
        Err(_) => return std::ptr::null_mut(),
    };
    match core_sabre_layout_prepared(prepared, prepared_target, &objective, &config) {
        Ok(result) => Box::into_raw(Box::new(CLayoutResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Scores a candidate layout against circuit and physical-graph analysis.
///
/// Returns 0 with the score written to `*out`, -1 on NULL arguments, -8 for
/// an unknown objective tag, or -6 when the core scoring fails.
#[unsafe(no_mangle)]
pub extern "C" fn score_layout(
    objective: u8,
    analysis: *const CCircuitLayoutAnalysis,
    physical: *const CPhysicalLayoutGraph,
    layout: *const CLayout,
    out: *mut CLayoutScore,
) -> i32 {
    if analysis.is_null() || physical.is_null() || layout.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let physical = unsafe { &(*physical).inner };
    let objective = match objective_from_tag(objective, Some(physical), None) {
        Ok(Some(objective)) => objective,
        Ok(None) => return CqlibError::InvalidParam as i32,
        Err(CompilerError::InvalidInput(_)) => return CqlibError::InvalidParam as i32,
        Err(_) => return CqlibError::CompilerError as i32,
    };
    let analysis = unsafe { &(*analysis).inner };
    let layout = unsafe { &(*layout).inner };
    match objective.score_layout(analysis, physical, layout) {
        Ok(score) => {
            write_layout_score(&score, out);
            CqlibError::Ok as i32
        }
        Err(_) => CqlibError::CompilerError as i32,
    }
}

/// Writes one layout-score snapshot.
pub(crate) fn write_layout_score(score: &LayoutScore, out: *mut CLayoutScore) {
    unsafe {
        *out = CLayoutScore {
            total: score.total,
            distance: score.distance,
            direction: score.direction,
            two_qubit_error: score.two_qubit_error,
            readout_error: score.readout_error,
            used_fidelity: score.used_fidelity as u8,
            _reserved: [0; 7],
        };
    }
}

// =====  LayoutResult accessors  =====

/// Frees a layout result. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn layout_result_free(ptr: *mut CLayoutResult) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the selected logical-to-physical mapping as an owned `CLayout*`
/// (free with `layout_free`). Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn layout_result_layout(ptr: *const CLayoutResult) -> *mut CLayout {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    Box::into_raw(Box::new(CLayout {
        inner: result.inner.layout.clone(),
    }))
}

/// Returns 1 when the result carries an observed score, 0 otherwise, or -1
/// for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_result_has_score(ptr: *const CLayoutResult) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.score.is_some() as i32 }
}

/// Writes the layout-score snapshot to `*out`.
///
/// Returns 0 on success, -1 on NULL arguments, or -8 when the result has no
/// score.
#[unsafe(no_mangle)]
pub extern "C" fn layout_result_score(ptr: *const CLayoutResult, out: *mut CLayoutScore) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let result = unsafe { &*ptr };
    match &result.inner.score {
        Some(score) => {
            write_layout_score(score, out);
            CqlibError::Ok as i32
        }
        None => CqlibError::InvalidParam as i32,
    }
}

/// Returns 1 when every positive-weight interaction lands on an adjacent
/// hardware edge, 0 otherwise, or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_result_is_perfect(ptr: *const CLayoutResult) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.diagnostics.is_perfect as i32 }
}

/// Returns the number of candidate layouts considered by the method, or 0
/// for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_result_candidates_evaluated(ptr: *const CLayoutResult) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.diagnostics.candidates_evaluated }
}

/// Returns 1 when fidelity data contributed to the selected score, 0
/// otherwise, or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_result_used_fidelity(ptr: *const CLayoutResult) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.diagnostics.used_fidelity as i32 }
}

/// Returns the number of diagnostic notes, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn layout_result_notes_len(ptr: *const CLayoutResult) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.diagnostics.notes.len() }
}

/// Returns the diagnostic note at `index` as a heap-allocated C string.
///
/// Caller must free with `cqlib_string_free`. Returns NULL on out-of-bounds
/// or error.
#[unsafe(no_mangle)]
pub extern "C" fn layout_result_note(ptr: *const CLayoutResult, index: usize) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    match result.inner.diagnostics.notes.get(index) {
        Some(note) => match CString::new(note.as_str()) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}
