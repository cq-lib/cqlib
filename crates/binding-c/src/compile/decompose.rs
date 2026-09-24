// This code is part of Cqlib.
//
// (C) Copyright China Telecom Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! C ABI for gate decomposition, numeric unitary synthesis, two-qubit block
//! resynthesis, and target-basis / device lowering.

use crate::circuit::CCircuit;
use crate::compile::resource::CResourcePolicy;
use crate::compile::transform::{standard_instructions_from_names, write_name_list};
use crate::device::{CDevice, standard_gate_from_name};
use crate::error::CqlibError;
use cqlib_core::circuit::{Instruction, ParameterValue, Qubit, StandardGate, ValueOperation};
use cqlib_core::compile::commutation::CommutationConfig;
use cqlib_core::compile::resource::{ResourceLimits, ResourcePolicy};
use cqlib_core::compile::transform::decompose::mc_gate::decompose_mc_gates_for_device as core_decompose_mc_gates_for_device;
use cqlib_core::compile::transform::decompose::unitary::{
    KakDecomposition, TwoQubitSynthesisTarget, TwoQubitUnitaryDecomposeBasis,
    TwoQubitUnitarySynthesisResult, kak_decompose as core_kak_decompose,
    synthesize_numeric_1q_unitary as core_synthesize_numeric_1q_unitary,
    synthesize_numeric_2q_unitary as core_synthesize_numeric_2q_unitary,
};
use cqlib_core::compile::transform::decompose::{
    DecompositionRuleStats, McGateDecomposeConfig, UnitaryDecomposeConfig,
    decompose_mc_gates_with_rule_stats as core_decompose_mc_gates_with_rule_stats,
    decompose_unitaries_with_rule_stats as core_decompose_unitaries_with_rule_stats,
    expand_definitions as core_expand_definitions,
};
use cqlib_core::compile::transform::resynthesis::{
    TwoQubitBlockResynthesisConfig, resynthesize_two_qubit_blocks as core_resynthesize_blocks,
};
use cqlib_core::compile::transform::target_basis::TargetBasisLowerer;
use cqlib_core::compile::transform::target_basis::{TargetBasisCost, TargetBasisCostModel};
use cqlib_core::compile::transform::{DeviceLowerer, Transformer};
use ndarray::Array2;
use num_complex::Complex64;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Two-qubit synthesis basis tag.
/// | Value | Basis                                    |
/// |-------|------------------------------------------|
/// | 0     | Local `U` + `RXX`/`RYY`/`RZZ`            |
/// | 1     | Local `U` + `CX` templates               |
/// | 2     | Local `U` + `CY` templates               |
/// | 3     | Local `U` + `CZ` templates               |
/// | 4     | Local `U` + `RZZ`                        |
pub const TWO_QUBIT_BASIS_PAULI_ROTATIONS: u8 = 0;
pub const TWO_QUBIT_BASIS_CX: u8 = 1;
pub const TWO_QUBIT_BASIS_CY: u8 = 2;
pub const TWO_QUBIT_BASIS_CZ: u8 = 3;
pub const TWO_QUBIT_BASIS_RZZ: u8 = 4;

/// KAK local-factor tag.
/// | Value | Factor                                    |
/// |-------|-------------------------------------------|
/// | 0     | `k1l` (left, after the interaction)       |
/// | 1     | `k1r` (right, after the interaction)      |
/// | 2     | `k2l` (left, before the interaction)      |
/// | 3     | `k2r` (right, before the interaction)     |
pub const KAK_FACTOR_K1L: u8 = 0;
pub const KAK_FACTOR_K1R: u8 = 1;
pub const KAK_FACTOR_K2L: u8 = 2;
pub const KAK_FACTOR_K2R: u8 = 3;

/// C snapshot of pass-local decomposition-rule cache statistics.
#[repr(C)]
pub struct CDecompositionRuleStats {
    /// Cache hits during the run.
    pub hits: usize,
    /// Cache misses during the run.
    pub misses: usize,
    /// Cache writes during the run.
    pub inserts: usize,
}

/// C form of [`UnitaryDecomposeConfig`].
///
/// The two-qubit synthesis target is fixed to the unconstrained default
/// (neutral Pauli-rotation fallback); building a constrained target needs an
/// `Instruction` list, which the C ABI does not parse.
#[repr(C)]
pub struct CUnitaryDecomposeConfig {
    /// 1 recurses into control-flow bodies, 0 does not.
    pub recurse_control_flow: u8,
    /// Reserved padding to keep the struct layout stable.
    pub _reserved: [u8; 7],
}

/// C form of [`McGateDecomposeConfig`].
#[repr(C)]
pub struct CMcGateDecomposeConfig {
    /// Total clean logical ancillas the pass may create before layout.
    pub max_pre_layout_clean_ancillas: usize,
    /// 1 allows borrowing input qubits under the dirty contract.
    pub allow_dirty_borrowing: u8,
    /// 1 enforces `max_total_qubits`, 0 leaves the limit unset.
    pub has_max_total_qubits: u8,
    /// Reserved padding to keep the struct layout stable.
    pub _pad: [u8; 6],
    /// Hard limit on total logical qubits (used when `has_max_total_qubits != 0`).
    pub max_total_qubits: usize,
    /// Reserved padding to keep the struct layout stable.
    pub _reserved: [u64; 2],
}

/// C form of [`TwoQubitBlockResynthesisConfig`].
///
/// The two-qubit synthesis target is fixed to the unconstrained default; the
/// embedded commutation-engine configuration is exposed as scalar fields.
#[repr(C)]
pub struct CResynthesisConfig {
    /// Maximum source operations in one bounded candidate block.
    pub max_block_ops: usize,
    /// Maximum non-block operations crossed while collecting a block.
    pub max_crossed_ops: usize,
    /// Collection budget per side of a two-qubit anchor.
    pub max_scan_span: usize,
    /// 1 treats labeled operations as hard boundaries.
    pub skip_labeled_ops: u8,
    /// 1 recurses into structured classical-control bodies.
    pub recurse_control_flow: u8,
    /// 1 enables the knowledge-rule commutation oracle.
    pub enable_rule_oracle: u8,
    /// 1 enables the local matrix fallback in the commutation engine.
    pub enable_matrix_fallback: u8,
    /// Reserved padding to keep the struct layout stable.
    pub _pad: [u8; 4],
    /// Maximum union-support size for the matrix fallback.
    pub max_matrix_qubits: usize,
    /// Reserved padding to keep the struct layout stable.
    pub _reserved: [u64; 2],
}

/// C snapshot of a one-qubit numeric synthesis result. The represented
/// matrix is `exp(i * global_phase) * U(theta, phi, lambda)`.
#[repr(C)]
pub struct COneQubitUnitaryDecomposition {
    /// Polar rotation angle.
    pub theta: f64,
    /// First azimuthal angle.
    pub phi: f64,
    /// Second azimuthal angle.
    pub lambda: f64,
    /// Scalar phase multiplying the synthesized gate.
    pub global_phase: f64,
}

/// Opaque handle around a [`TwoQubitUnitarySynthesisResult`].
pub struct CTwoQubitUnitarySynthesis {
    pub inner: TwoQubitUnitarySynthesisResult,
}

/// Opaque handle around a [`KakDecomposition`].
pub struct CKakDecomposition {
    pub inner: KakDecomposition,
}

/// Opaque handle around a [`TargetBasisLowerer`].
pub struct CTargetBasisLowerer {
    pub inner: TargetBasisLowerer,
}

/// Returns the default unitary-decompose configuration by value.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitary_config_default() -> CUnitaryDecomposeConfig {
    CUnitaryDecomposeConfig {
        recurse_control_flow: 1,
        _reserved: [0; 7],
    }
}

/// Returns the default MC-gate decompose configuration by value.
#[unsafe(no_mangle)]
pub extern "C" fn mc_gate_config_default() -> CMcGateDecomposeConfig {
    CMcGateDecomposeConfig {
        max_pre_layout_clean_ancillas: 0,
        allow_dirty_borrowing: 0,
        has_max_total_qubits: 0,
        _pad: [0; 6],
        max_total_qubits: 0,
        _reserved: [0; 2],
    }
}

/// Returns the normal-budget resynthesis configuration by value.
#[unsafe(no_mangle)]
pub extern "C" fn resynthesis_config_normal() -> CResynthesisConfig {
    resynthesis_config_from_core(TwoQubitBlockResynthesisConfig::normal(Default::default()))
}

/// Returns the enhanced-budget resynthesis configuration by value.
#[unsafe(no_mangle)]
pub extern "C" fn resynthesis_config_enhanced() -> CResynthesisConfig {
    resynthesis_config_from_core(TwoQubitBlockResynthesisConfig::enhanced(Default::default()))
}

/// Converts a core resynthesis configuration into the C snapshot form.
fn resynthesis_config_from_core(config: TwoQubitBlockResynthesisConfig) -> CResynthesisConfig {
    CResynthesisConfig {
        max_block_ops: config.max_block_ops,
        max_crossed_ops: config.max_crossed_ops,
        max_scan_span: config.max_scan_span,
        skip_labeled_ops: config.skip_labeled_ops as u8,
        recurse_control_flow: config.recurse_control_flow as u8,
        enable_rule_oracle: config.commutation.enable_rule_oracle as u8,
        enable_matrix_fallback: config.commutation.enable_matrix_fallback as u8,
        _pad: [0; 4],
        max_matrix_qubits: config.commutation.max_matrix_qubits,
        _reserved: [0; 2],
    }
}

/// Converts a C resynthesis configuration into the core form.
fn resynthesis_config_to_core(raw: &CResynthesisConfig) -> TwoQubitBlockResynthesisConfig {
    TwoQubitBlockResynthesisConfig {
        two_qubit_target: Default::default(),
        max_block_ops: raw.max_block_ops,
        max_crossed_ops: raw.max_crossed_ops,
        max_scan_span: raw.max_scan_span,
        skip_labeled_ops: raw.skip_labeled_ops != 0,
        recurse_control_flow: raw.recurse_control_flow != 0,
        commutation: CommutationConfig {
            enable_rule_oracle: raw.enable_rule_oracle != 0,
            enable_matrix_fallback: raw.enable_matrix_fallback != 0,
            max_matrix_qubits: raw.max_matrix_qubits,
        },
    }
}

/// Converts a C MC-gate decompose configuration into the core form.
fn mc_gate_config_to_core(raw: &CMcGateDecomposeConfig) -> McGateDecomposeConfig {
    McGateDecomposeConfig {
        resource_policy: ResourcePolicy {
            max_pre_layout_clean_ancillas: raw.max_pre_layout_clean_ancillas,
            allow_dirty_borrowing: raw.allow_dirty_borrowing != 0,
        },
        resource_limits: ResourceLimits {
            max_total_qubits: if raw.has_max_total_qubits != 0 {
                Some(raw.max_total_qubits)
            } else {
                None
            },
        },
    }
}

// =====  Circuit-level decomposition passes  =====

/// Expands circuit-gate definitions of `circuit` and returns the rebuilt
/// circuit as an owned `CCircuit*` (free with `circuit_free`).
///
/// Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_expand_definitions(circuit: *const CCircuit) -> *mut CCircuit {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner };
    match core_expand_definitions(circuit) {
        Ok(inner) => Box::into_raw(Box::new(CCircuit { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Synthesizes matrix-backed unitary gates of `circuit` and returns the
/// rebuilt circuit as an owned `CCircuit*` (free with `circuit_free`).
///
/// Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitaries(
    circuit: *const CCircuit,
    config: CUnitaryDecomposeConfig,
) -> *mut CCircuit {
    decompose_unitaries_with_rule_stats(circuit, config, std::ptr::null_mut())
}

/// Diagnostic form of [`decompose_unitaries`]: additionally writes the
/// pass-local rule-cache statistics to `stats` when it is non-NULL.
///
/// Returns the rebuilt circuit as an owned `CCircuit*` (free with
/// `circuit_free`), or NULL on error (`stats` is then left untouched).
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitaries_with_rule_stats(
    circuit: *const CCircuit,
    config: CUnitaryDecomposeConfig,
    stats: *mut CDecompositionRuleStats,
) -> *mut CCircuit {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let config = UnitaryDecomposeConfig {
        two_qubit_target: Default::default(),
        recurse_control_flow: config.recurse_control_flow != 0,
    };
    let circuit = unsafe { &(*circuit).inner };
    match core_decompose_unitaries_with_rule_stats(circuit, config) {
        Ok((outcome, rule_stats)) => {
            write_rule_stats(stats, &rule_stats);
            Box::into_raw(Box::new(CCircuit {
                inner: outcome.into_circuit(circuit),
            }))
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Rewrites multi-controlled gates of `circuit` and returns the rebuilt
/// circuit as an owned `CCircuit*` (free with `circuit_free`).
///
/// Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_mc_gates(
    circuit: *const CCircuit,
    config: CMcGateDecomposeConfig,
) -> *mut CCircuit {
    decompose_mc_gates_with_rule_stats(circuit, config, std::ptr::null_mut())
}

/// Diagnostic form of [`decompose_mc_gates`]: additionally writes the
/// pass-local rule-cache statistics to `stats` when it is non-NULL.
///
/// Returns the rebuilt circuit as an owned `CCircuit*` (free with
/// `circuit_free`), or NULL on error (`stats` is then left untouched).
#[unsafe(no_mangle)]
pub extern "C" fn decompose_mc_gates_with_rule_stats(
    circuit: *const CCircuit,
    config: CMcGateDecomposeConfig,
    stats: *mut CDecompositionRuleStats,
) -> *mut CCircuit {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let config = mc_gate_config_to_core(&config);
    let circuit = unsafe { &(*circuit).inner };
    match core_decompose_mc_gates_with_rule_stats(circuit, config) {
        Ok((outcome, rule_stats)) => {
            write_rule_stats(stats, &rule_stats);
            Box::into_raw(Box::new(CCircuit {
                inner: outcome.into_circuit(circuit),
            }))
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Rewrites multi-controlled gates of `circuit` within the usable-qubit
/// capacity of `device` (a pre-layout logical transform).
///
/// `policy` may be NULL to select the default resource policy. Returns the
/// rebuilt circuit as an owned `CCircuit*` (free with `circuit_free`), or
/// NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_mc_gates_for_device(
    circuit: *const CCircuit,
    device: *const CDevice,
    policy: *const CResourcePolicy,
) -> *mut CCircuit {
    if circuit.is_null() || device.is_null() {
        return std::ptr::null_mut();
    }
    let policy = if policy.is_null() {
        ResourcePolicy::default()
    } else {
        let raw = unsafe { &*policy };
        ResourcePolicy {
            max_pre_layout_clean_ancillas: raw.max_pre_layout_clean_ancillas,
            allow_dirty_borrowing: raw.allow_dirty_borrowing != 0,
        }
    };
    let device = unsafe { &(*device).inner };
    let circuit = unsafe { &(*circuit).inner };
    match core_decompose_mc_gates_for_device(circuit, device, policy) {
        Ok(outcome) => Box::into_raw(Box::new(CCircuit {
            inner: outcome.into_circuit(circuit),
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Writes a rule-stats snapshot to `stats` when it is non-NULL.
fn write_rule_stats(stats: *mut CDecompositionRuleStats, rule_stats: &DecompositionRuleStats) {
    if !stats.is_null() {
        unsafe {
            *stats = CDecompositionRuleStats {
                hits: rule_stats.hits,
                misses: rule_stats.misses,
                inserts: rule_stats.inserts,
            };
        }
    }
}

// =====  Two-qubit block resynthesis  =====

/// Resynthesizes two-qubit blocks of `circuit` and returns the rebuilt
/// circuit as an owned `CCircuit*` (free with `circuit_free`).
///
/// Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn resynthesize_two_qubit_blocks(
    circuit: *const CCircuit,
    config: CResynthesisConfig,
) -> *mut CCircuit {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let config = resynthesis_config_to_core(&config);
    let circuit = unsafe { &(*circuit).inner };
    match core_resynthesize_blocks(circuit, config) {
        Ok(outcome) => Box::into_raw(Box::new(CCircuit {
            inner: outcome.into_circuit(circuit),
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

// =====  Numeric unitary synthesis primitives  =====

/// Synthesizes a 2x2 unitary matrix into Cqlib's `U` convention.
///
/// `matrix` points at 4 row-major `Complex64` elements. Returns 0 on
/// success, -1 on NULL arguments, -6 when the matrix is non-finite,
/// non-unitary, or otherwise rejected by the synthesizer.
#[unsafe(no_mangle)]
pub extern "C" fn synthesize_numeric_1q_unitary(
    matrix: *const Complex64,
    out: *mut COneQubitUnitaryDecomposition,
) -> i32 {
    if matrix.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let flat = unsafe { std::slice::from_raw_parts(matrix, 4) }.to_vec();
    let matrix = match Array2::from_shape_vec((2, 2), flat) {
        Ok(matrix) => matrix,
        Err(_) => return CqlibError::InvalidParam as i32,
    };
    match core_synthesize_numeric_1q_unitary(&matrix) {
        Ok(decomposition) => {
            unsafe {
                *out = COneQubitUnitaryDecomposition {
                    theta: decomposition.theta,
                    phi: decomposition.phi,
                    lambda: decomposition.lambda,
                    global_phase: decomposition.global_phase,
                };
            }
            CqlibError::Ok as i32
        }
        Err(_) => CqlibError::CompilerError as i32,
    }
}

/// Synthesizes a 4x4 unitary matrix into standard-gate operations.
///
/// `matrix` points at 16 row-major `Complex64` elements; `first` and
/// `second` are the target qubits; `basis` is one of `TWO_QUBIT_BASIS_*`.
/// Returns a heap-allocated `CTwoQubitUnitarySynthesis*` (free with
/// `two_qubit_synthesis_free`), or NULL on NULL input, an unknown basis tag,
/// or a synthesis failure.
#[unsafe(no_mangle)]
pub extern "C" fn synthesize_numeric_2q_unitary(
    matrix: *const Complex64,
    first: u32,
    second: u32,
    basis: u8,
) -> *mut CTwoQubitUnitarySynthesis {
    if matrix.is_null() {
        return std::ptr::null_mut();
    }
    let basis = match basis {
        TWO_QUBIT_BASIS_PAULI_ROTATIONS => TwoQubitUnitaryDecomposeBasis::PauliRotations,
        TWO_QUBIT_BASIS_CX => TwoQubitUnitaryDecomposeBasis::Cx,
        TWO_QUBIT_BASIS_CY => TwoQubitUnitaryDecomposeBasis::Cy,
        TWO_QUBIT_BASIS_CZ => TwoQubitUnitaryDecomposeBasis::Cz,
        TWO_QUBIT_BASIS_RZZ => TwoQubitUnitaryDecomposeBasis::Rzz,
        _ => return std::ptr::null_mut(),
    };
    let flat = unsafe { std::slice::from_raw_parts(matrix, 16) }.to_vec();
    let matrix = match Array2::from_shape_vec((4, 4), flat) {
        Ok(matrix) => matrix,
        Err(_) => return std::ptr::null_mut(),
    };
    match core_synthesize_numeric_2q_unitary(
        &matrix,
        [Qubit::new(first), Qubit::new(second)],
        basis,
    ) {
        Ok(inner) => Box::into_raw(Box::new(CTwoQubitUnitarySynthesis { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a two-qubit synthesis result. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn two_qubit_synthesis_free(ptr: *mut CTwoQubitUnitarySynthesis) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the number of emitted operations, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn two_qubit_synthesis_num_operations(
    ptr: *const CTwoQubitUnitarySynthesis,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.operations.len() }
}

/// Returns the global phase multiplying the emitted sequence, or 0.0 for
/// NULL.
#[unsafe(no_mangle)]
pub extern "C" fn two_qubit_synthesis_global_phase(ptr: *const CTwoQubitUnitarySynthesis) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    unsafe { (*ptr).inner.global_phase }
}

/// Returns the instruction name of operation `index` as a heap-allocated C
/// string (free with `cqlib_string_free`). Returns NULL on NULL input or
/// out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn two_qubit_synthesis_operation_name(
    ptr: *const CTwoQubitUnitarySynthesis,
    index: usize,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &(*ptr).inner };
    match result.operations.get(index) {
        Some(operation) => match CString::new(operation.instruction.name()) {
            Ok(name) => name.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Copies the qubits of operation `index` into `out` (two-step pattern;
/// call `two_qubit_synthesis_operation_qubits_len` first). Returns the total
/// number of qubits, or 0 for NULL input or an out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn two_qubit_synthesis_operation_qubits(
    ptr: *const CTwoQubitUnitarySynthesis,
    index: usize,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let result = unsafe { &(*ptr).inner };
    match result.operations.get(index) {
        Some(operation) => {
            let qubits: Vec<u32> = operation.qubits.iter().map(|qubit| qubit.id()).collect();
            crate::device::write_u32_list(&qubits, out, len)
        }
        None => 0,
    }
}

/// Returns the number of qubits of operation `index`, or 0 for NULL input or
/// an out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn two_qubit_synthesis_operation_qubits_len(
    ptr: *const CTwoQubitUnitarySynthesis,
    index: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let result = unsafe { &(*ptr).inner };
    result
        .operations
        .get(index)
        .map(|operation| operation.qubits.len())
        .unwrap_or(0)
}

// =====  KAK decomposition  =====

/// Decomposes a 4x4 unitary matrix into its KAK (Weyl) coordinates and local
/// factors.
///
/// `matrix` points at 16 row-major `Complex64` elements. Returns a
/// heap-allocated `CKakDecomposition*` (free with `kak_decomposition_free`),
/// or NULL on NULL input or a decomposition failure.
#[unsafe(no_mangle)]
pub extern "C" fn kak_decompose(matrix: *const Complex64) -> *mut CKakDecomposition {
    if matrix.is_null() {
        return std::ptr::null_mut();
    }
    let flat = unsafe { std::slice::from_raw_parts(matrix, 16) }.to_vec();
    let matrix = match Array2::from_shape_vec((4, 4), flat) {
        Ok(matrix) => matrix,
        Err(_) => return std::ptr::null_mut(),
    };
    match core_kak_decompose(&matrix) {
        Ok(inner) => Box::into_raw(Box::new(CKakDecomposition { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a KAK decomposition. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn kak_decomposition_free(ptr: *mut CKakDecomposition) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the scalar phase multiplying the complete decomposition, or 0.0
/// for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn kak_decomposition_global_phase(ptr: *const CKakDecomposition) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    unsafe { (*ptr).inner.global_phase }
}

/// Writes the canonical Pauli-XX/YY/ZZ interaction coordinates `a`, `b`,
/// `c` to the given out pointers.
///
/// Returns 0 on success or -1 on NULL arguments.
#[unsafe(no_mangle)]
pub extern "C" fn kak_decomposition_coordinates(
    ptr: *const CKakDecomposition,
    a: *mut f64,
    b: *mut f64,
    c: *mut f64,
) -> i32 {
    if ptr.is_null() || a.is_null() || b.is_null() || c.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let decomposition = unsafe { &(*ptr).inner };
    unsafe {
        *a = decomposition.a;
        *b = decomposition.b;
        *c = decomposition.c;
    }
    CqlibError::Ok as i32
}

/// Copies one 2x2 local factor (selected by a `KAK_FACTOR_*` tag) into `out`
/// as 4 row-major `Complex64` elements.
///
/// Returns 0 on success, -1 on NULL input, -8 on an unknown factor tag.
#[unsafe(no_mangle)]
pub extern "C" fn kak_decomposition_local_factor(
    ptr: *const CKakDecomposition,
    factor: u8,
    out: *mut Complex64,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let decomposition = unsafe { &(*ptr).inner };
    let matrix = match factor {
        KAK_FACTOR_K1L => &decomposition.k1l,
        KAK_FACTOR_K1R => &decomposition.k1r,
        KAK_FACTOR_K2L => &decomposition.k2l,
        KAK_FACTOR_K2R => &decomposition.k2r,
        _ => return CqlibError::InvalidParam as i32,
    };
    let slice = unsafe { std::slice::from_raw_parts_mut(out, 4) };
    for (slot, value) in slice.iter_mut().zip(matrix.iter()) {
        *slot = *value;
    }
    CqlibError::Ok as i32
}

// =====  Target-basis and device lowering  =====

/// Creates a target-basis lowerer from standard-gate names.
///
/// Each entry of `gate_names` must be a standard-gate name accepted elsewhere
/// by the C ABI (e.g. `"H"`, `"RZ"`, `"CX"`); the names become the target
/// basis. Returns a heap-allocated `CTargetBasisLowerer*` (free with
/// `target_basis_lowerer_free`), or NULL on NULL input, invalid UTF-8, an
/// unknown gate name, or an empty basis.
#[unsafe(no_mangle)]
pub extern "C" fn target_basis_lowerer_new(
    gate_names: *const *const c_char,
    len: usize,
) -> *mut CTargetBasisLowerer {
    if gate_names.is_null() && len > 0 {
        return std::ptr::null_mut();
    }
    let mut basis = Vec::with_capacity(len);
    if len > 0 {
        let names = unsafe { std::slice::from_raw_parts(gate_names, len) };
        for name in names {
            if name.is_null() {
                return std::ptr::null_mut();
            }
            let name = match unsafe { CStr::from_ptr(*name) }.to_str() {
                Ok(name) => name,
                Err(_) => return std::ptr::null_mut(),
            };
            let gate = match standard_gate_from_name(name) {
                Some(gate) => gate,
                None => return std::ptr::null_mut(),
            };
            basis.push(Instruction::Standard(gate));
        }
    }
    match TargetBasisLowerer::new(basis) {
        Ok(inner) => Box::into_raw(Box::new(CTargetBasisLowerer { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a target-basis lowerer. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn target_basis_lowerer_free(ptr: *mut CTargetBasisLowerer) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the number of gates in the target basis, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn target_basis_lowerer_num_gates(ptr: *const CTargetBasisLowerer) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.target_basis().len() }
}

/// Returns 1 when translating `circuit` to the target basis can change or
/// reject its gate-like operations, 0 otherwise, or -1 on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn target_basis_lowerer_requires_lowering(
    ptr: *const CTargetBasisLowerer,
    circuit: *const CCircuit,
) -> i32 {
    if ptr.is_null() || circuit.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let lowerer = unsafe { &(*ptr).inner };
    let circuit = unsafe { &(*circuit).inner };
    lowerer.requires_lowering(circuit) as i32
}

/// Lowers `circuit` to the target basis and returns the rebuilt circuit as
/// an owned `CCircuit*` (free with `circuit_free`).
///
/// Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn target_basis_lowerer_apply(
    ptr: *const CTargetBasisLowerer,
    circuit: *const CCircuit,
) -> *mut CCircuit {
    if ptr.is_null() || circuit.is_null() {
        return std::ptr::null_mut();
    }
    let lowerer = unsafe { &(*ptr).inner };
    let circuit = unsafe { &(*circuit).inner };
    match lowerer.transform(circuit, None) {
        Ok(outcome) => Box::into_raw(Box::new(CCircuit {
            inner: outcome.into_circuit(circuit),
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Lowers `circuit` to the native instruction set of `device` and returns
/// the rebuilt circuit as an owned `CCircuit*` (free with `circuit_free`).
///
/// Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_lower_to_device(
    circuit: *const CCircuit,
    device: *const CDevice,
) -> *mut CCircuit {
    if circuit.is_null() || device.is_null() {
        return std::ptr::null_mut();
    }
    let device = unsafe { &(*device).inner };
    let circuit = unsafe { &(*circuit).inner };
    match DeviceLowerer::new(device).transform(circuit, None) {
        Ok(outcome) => Box::into_raw(Box::new(CCircuit {
            inner: outcome.into_circuit(circuit),
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

// =====  Unitary-decompose two-qubit synthesis target  =====

/// C snapshot of a target-basis lowering cost.
#[repr(C)]
pub struct CTargetBasisCost {
    /// Two-qubit operations in the lowered sequence.
    pub two_qubit_ops: usize,
    /// Critical-path depth of the lowered sequence.
    pub depth: usize,
    /// Total operation count of the lowered sequence (global phase excluded).
    pub total_ops: usize,
    /// Operations carrying parameters in the lowered sequence.
    pub parameterized_ops: usize,
}

/// Opaque handle around a core [`TwoQubitSynthesisTarget`]: the two-qubit
/// synthesis capability plugged into the unitary-decompose configuration
/// (`UnitaryDecomposeConfig::two_qubit_target`).
pub struct CDecomposeUnitaryTarget {
    pub inner: TwoQubitSynthesisTarget,
}

/// Extracts the standard gates from parsed instructions (the shared parser
/// only ever produces standard instructions).
fn standard_gates_from_instructions(instructions: Vec<Instruction>) -> Vec<StandardGate> {
    instructions
        .into_iter()
        .filter_map(|instruction| match instruction {
            Instruction::Standard(gate) => Some(gate),
            _ => None,
        })
        .collect()
}

/// Collects the native gate names of `target` (one-qubit gates followed by
/// two-qubit gates).
fn target_gate_names(target: &TwoQubitSynthesisTarget) -> Vec<String> {
    target
        .native_1q()
        .iter()
        .chain(target.native_2q().iter())
        .map(|gate| Instruction::Standard(*gate).name())
        .collect()
}

/// Creates an unconstrained synthesis target: no native-basis restrictions
/// and the neutral exact Pauli-rotation fallback.
///
/// Returns a heap-allocated `CDecomposeUnitaryTarget*` (free with
/// `decompose_unitary_target_free`).
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitary_target_unconstrained() -> *mut CDecomposeUnitaryTarget {
    Box::into_raw(Box::new(CDecomposeUnitaryTarget {
        inner: TwoQubitSynthesisTarget::unconstrained(),
    }))
}

/// Creates a synthesis target from a workflow-style target-basis list of
/// standard-gate names (e.g. `"H"`, `"RZ"`, `"CX"`): one-qubit names become
/// the native one-qubit basis, two-qubit names the native two-qubit basis,
/// and the exact Pauli-rotation fallback stays enabled.
///
/// Returns a heap-allocated `CDecomposeUnitaryTarget*` (free with
/// `decompose_unitary_target_free`), or NULL on NULL input, invalid UTF-8,
/// an unknown gate name, or when the core rejects the basis (e.g. empty).
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitary_target_from_instructions(
    gate_names: *const *const c_char,
    len: usize,
) -> *mut CDecomposeUnitaryTarget {
    let instructions = match standard_instructions_from_names(gate_names, len) {
        Ok(instructions) => instructions,
        Err(_) => return std::ptr::null_mut(),
    };
    match TwoQubitSynthesisTarget::from_instructions(Some(&instructions)) {
        Ok(inner) => Box::into_raw(Box::new(CDecomposeUnitaryTarget { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Creates a synthesis target from explicit native one-qubit and two-qubit
/// standard-gate name lists and a Pauli-rotation fallback flag.
///
/// Every name in `native_1q_names` must be a one-qubit standard gate and
/// every name in `native_2q_names` a two-qubit standard gate. Returns a
/// heap-allocated `CDecomposeUnitaryTarget*` (free with
/// `decompose_unitary_target_free`), or NULL on NULL input, invalid UTF-8,
/// an unknown gate name, a wrong-arity gate, or an empty combined basis.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitary_target_from_standard_gates(
    native_1q_names: *const *const c_char,
    num_1q: usize,
    native_2q_names: *const *const c_char,
    num_2q: usize,
    fallback_pauli: u8,
) -> *mut CDecomposeUnitaryTarget {
    let native_1q = match standard_instructions_from_names(native_1q_names, num_1q) {
        Ok(instructions) => standard_gates_from_instructions(instructions),
        Err(_) => return std::ptr::null_mut(),
    };
    let native_2q = match standard_instructions_from_names(native_2q_names, num_2q) {
        Ok(instructions) => standard_gates_from_instructions(instructions),
        Err(_) => return std::ptr::null_mut(),
    };
    match TwoQubitSynthesisTarget::from_standard_gates(native_1q, native_2q, fallback_pauli != 0) {
        Ok(inner) => Box::into_raw(Box::new(CDecomposeUnitaryTarget { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a unitary-decompose synthesis target. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitary_target_free(ptr: *mut CDecomposeUnitaryTarget) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns 1 when Pauli-rotation fallback is permitted for `target`, 0
/// otherwise, or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitary_target_fallback_pauli(
    ptr: *const CDecomposeUnitaryTarget,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    unsafe { (*ptr).inner.fallback_pauli() as i32 }
}

/// Sets whether Pauli-rotation fallback is permitted for `target`.
///
/// The target is rebuilt around its native gate lists, which requires a
/// non-empty basis. Returns 0 on success, -1 on NULL, or -8 when the target
/// is unconstrained (no native gates), in which case the target is left
/// unchanged.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitary_target_set_fallback_pauli(
    ptr: *mut CDecomposeUnitaryTarget,
    value: u8,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let native_1q = wrapper.inner.native_1q().to_vec();
    let native_2q = wrapper.inner.native_2q().to_vec();
    match TwoQubitSynthesisTarget::from_standard_gates(native_1q, native_2q, value != 0) {
        Ok(inner) => {
            wrapper.inner = inner;
            CqlibError::Ok as i32
        }
        Err(_) => CqlibError::InvalidParam as i32,
    }
}

/// Returns the number of native one-qubit gates of `target`, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitary_target_native_1q_len(
    ptr: *const CDecomposeUnitaryTarget,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.native_1q().len() }
}

/// Copies the native one-qubit gate names of `target` into `out` (two-step
/// pattern; pair with `decompose_unitary_target_native_1q_len`). Each written
/// entry is a freshly allocated C string freed with `cqlib_string_free`.
/// Returns the total number of gates, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitary_target_native_1q(
    ptr: *const CDecomposeUnitaryTarget,
    out: *mut *mut c_char,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let names: Vec<String> = unsafe { (*ptr).inner.native_1q() }
        .iter()
        .map(|gate| Instruction::Standard(*gate).name())
        .collect();
    write_name_list(&names, out, len)
}

/// Returns the number of native two-qubit gates of `target`, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitary_target_native_2q_len(
    ptr: *const CDecomposeUnitaryTarget,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.native_2q().len() }
}

/// Copies the native two-qubit gate names of `target` into `out` (two-step
/// pattern; pair with `decompose_unitary_target_native_2q_len`). Each written
/// entry is a freshly allocated C string freed with `cqlib_string_free`.
/// Returns the total number of gates, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitary_target_native_2q(
    ptr: *const CDecomposeUnitaryTarget,
    out: *mut *mut c_char,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let names: Vec<String> = unsafe { (*ptr).inner.native_2q() }
        .iter()
        .map(|gate| Instruction::Standard(*gate).name())
        .collect();
    write_name_list(&names, out, len)
}

/// Returns the number of native basis gates of `target` (one-qubit gates
/// followed by two-qubit gates), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitary_target_basis_len(ptr: *const CDecomposeUnitaryTarget) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let target = unsafe { &(*ptr).inner };
    target.native_1q().len() + target.native_2q().len()
}

/// Copies the native basis gate names of `target` into `out` (two-step
/// pattern; pair with `decompose_unitary_target_basis_len`). Each written
/// entry is a freshly allocated C string freed with `cqlib_string_free`.
/// Returns the total number of gates, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitary_target_basis(
    ptr: *const CDecomposeUnitaryTarget,
    out: *mut *mut c_char,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let names = target_gate_names(unsafe { &(*ptr).inner });
    write_name_list(&names, out, len)
}

/// Computes the exact target-basis cost of a fixed standard-gate operation
/// sequence under the native basis of `target`.
///
/// The sequence is given as `num_ops` gate names (`gate_names`), with each
/// operation's qubit IDs flattened into `qubit_ids` and per-operation counts
/// in `qubit_counts`. Parameters are treated as fixed zeros. Requires a
/// constrained target (an unconstrained target has no basis to cost
/// against).
///
/// Writes the cost snapshot to `*out`. Returns 0 on success, -1 on NULL,
/// -4 on invalid UTF-8 or an unknown gate name, -8 when the target is
/// unconstrained, or -6 when the core rejects the sequence.
#[unsafe(no_mangle)]
pub extern "C" fn decompose_unitary_target_cost_of_fixed_operations(
    ptr: *const CDecomposeUnitaryTarget,
    gate_names: *const *const c_char,
    qubit_ids: *const u32,
    qubit_counts: *const u32,
    num_ops: usize,
    out: *mut CTargetBasisCost,
) -> i32 {
    if ptr.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let target = unsafe { &(*ptr).inner };
    let basis: Vec<Instruction> = target
        .native_1q()
        .iter()
        .chain(target.native_2q().iter())
        .map(|gate| Instruction::Standard(*gate))
        .collect();
    let model = match TargetBasisCostModel::new(basis) {
        Ok(model) => model,
        Err(_) => return CqlibError::InvalidParam as i32,
    };

    let cost = if num_ops == 0 {
        TargetBasisCost::default()
    } else {
        if gate_names.is_null() || qubit_counts.is_null() {
            return CqlibError::NullPtr as i32;
        }
        let names = unsafe { std::slice::from_raw_parts(gate_names, num_ops) };
        let counts = unsafe { std::slice::from_raw_parts(qubit_counts, num_ops) };
        let mut operations: Vec<ValueOperation> = Vec::with_capacity(num_ops);
        let mut qubits: Vec<Qubit> = Vec::new();
        let mut cursor = 0usize;
        for (name, &count) in names.iter().zip(counts) {
            if name.is_null() {
                return CqlibError::NullPtr as i32;
            }
            let name = match unsafe { CStr::from_ptr(*name) }.to_str() {
                Ok(name) => name,
                Err(_) => return CqlibError::ParseError as i32,
            };
            let Some(gate) = standard_gate_from_name(name) else {
                return CqlibError::ParseError as i32;
            };
            let mut op_qubits = Vec::with_capacity(count as usize);
            for k in 0..count as usize {
                if qubit_ids.is_null() {
                    return CqlibError::NullPtr as i32;
                }
                let id = unsafe { *qubit_ids.add(cursor + k) };
                let qubit = Qubit::new(id);
                if !qubits.contains(&qubit) {
                    qubits.push(qubit);
                }
                op_qubits.push(qubit);
            }
            cursor += count as usize;
            let params = (0..gate.num_params()).map(|_| ParameterValue::Fixed(0.0));
            operations.push(ValueOperation::from_standard(gate, op_qubits, params));
        }
        match model.cost_of_fixed_operations(qubits, operations) {
            Ok(cost) => cost,
            Err(_) => return CqlibError::CompilerError as i32,
        }
    };
    unsafe {
        *out = CTargetBasisCost {
            two_qubit_ops: cost.two_qubit_ops,
            depth: cost.depth,
            total_ops: cost.total_ops,
            parameterized_ops: cost.parameterized_ops,
        };
    }
    CqlibError::Ok as i32
}
