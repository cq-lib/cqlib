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

//! Rust FFI tests for feature maps, facade constructors, entanglement
//! topology helpers, `StronglyEntanglingLayers` and the
//! `PauliEvolutionAnsatz` time-parameter naming (`circuit/ansatz.rs`).

use binding_c::circuit::{
    ENTANGLEMENT_CIRCULAR, ENTANGLEMENT_CUSTOM, ENTANGLEMENT_FULL, ENTANGLEMENT_LINEAR,
    circuit_free, circuit_num_operations, circuit_num_parameters, circuit_num_qubits,
    efficient_su2, entanglement_generate_k_tuples, entanglement_generate_k_tuples_len,
    entanglement_generate_pairs, entanglement_generate_pairs_len, iqp_feature_map_build_circuit,
    iqp_feature_map_entanglement, iqp_feature_map_free, iqp_feature_map_new,
    iqp_feature_map_num_parameters, iqp_feature_map_num_qubits, iqp_feature_map_reps,
    iqp_feature_map_validate, pauli_evolution_ansatz_free, pauli_evolution_ansatz_new,
    pauli_evolution_ansatz_with_time_param_name, pauli_feature_map_build,
    pauli_feature_map_build_circuit, pauli_feature_map_entanglement, pauli_feature_map_free,
    pauli_feature_map_new, pauli_feature_map_num_parameters, pauli_feature_map_num_qubits,
    pauli_feature_map_parameter_prefix, pauli_feature_map_paulis, pauli_feature_map_reps,
    pauli_feature_map_validate, real_amplitudes, strongly_entangling_layers_build_circuit,
    strongly_entangling_layers_entanglement_gate, strongly_entangling_layers_free,
    strongly_entangling_layers_new, strongly_entangling_layers_num_parameters,
    strongly_entangling_layers_num_qubits, strongly_entangling_layers_ranges,
    strongly_entangling_layers_reps, strongly_entangling_layers_validate, two_local_free,
    two_local_num_parameters, two_local_num_qubits, two_local_validate,
    z_feature_map_build_circuit, z_feature_map_free, z_feature_map_new,
    z_feature_map_num_parameters, z_feature_map_num_qubits, z_feature_map_reps,
    z_feature_map_validate, zz_feature_map_build, zz_feature_map_build_circuit,
    zz_feature_map_entanglement, zz_feature_map_free, zz_feature_map_new,
    zz_feature_map_num_parameters, zz_feature_map_num_qubits, zz_feature_map_reps,
    zz_feature_map_validate,
};
use binding_c::qis::{
    CHamiltonian, hamiltonian_free, hamiltonian_from_pauli, hamiltonian_new, pauli_string_parse,
};
use std::ffi::CString;
use std::os::raw::c_char;

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Builds a Hamiltonian from a single Pauli term (coefficient = 1.0).
fn pauli_hamiltonian(text: &str) -> *mut CHamiltonian {
    let pauli = pauli_string_parse(cstr(text).as_ptr());
    assert!(!pauli.is_null());
    // `hamiltonian_from_pauli` takes ownership of the pauli handle.
    let ham = hamiltonian_from_pauli(pauli);
    assert!(!ham.is_null());
    ham
}

#[test]
fn z_feature_map_structure() {
    let map = z_feature_map_new(3);
    assert!(!map.is_null());
    assert_eq!(z_feature_map_num_qubits(map), 3);
    assert_eq!(z_feature_map_validate(map), 0);

    // Defaults: reps = 2, one parameter per qubit.
    assert_eq!(z_feature_map_num_parameters(map), 3);
    assert_eq!(z_feature_map_reps(map, 3), 0);
    assert_eq!(z_feature_map_num_parameters(map), 3);

    // Each rep is one H layer plus one RZ layer: 3 reps * (3 + 3) = 18 ops.
    let circuit = z_feature_map_build_circuit(map, cstr("x").as_ptr());
    assert!(!circuit.is_null());
    assert_eq!(circuit_num_qubits(circuit), 3);
    assert_eq!(circuit_num_operations(circuit), 18);
    assert_eq!(circuit_num_parameters(circuit), 3);
    circuit_free(circuit);

    // reps = 0 is legal and yields an empty circuit.
    assert_eq!(z_feature_map_reps(map, 0), 0);
    assert_eq!(z_feature_map_num_parameters(map), 0);
    let empty = z_feature_map_build_circuit(map, cstr("x").as_ptr());
    assert!(!empty.is_null());
    assert_eq!(circuit_num_operations(empty), 0);
    circuit_free(empty);

    // Prefix validation.
    assert!(z_feature_map_build_circuit(map, std::ptr::null()).is_null());
    let invalid_utf8 = CString::new(vec![0xFF, 0xFE]).unwrap();
    assert!(z_feature_map_build_circuit(map, invalid_utf8.as_ptr()).is_null());

    // Zero-qubit maps are created but never valid.
    let zero = z_feature_map_new(0);
    assert!(!zero.is_null());
    assert_eq!(z_feature_map_num_qubits(zero), 0);
    assert_eq!(z_feature_map_validate(zero), -3);
    assert!(z_feature_map_build_circuit(zero, cstr("x").as_ptr()).is_null());
    z_feature_map_free(zero);

    // NULL-handle tolerance.
    assert_eq!(z_feature_map_reps(std::ptr::null_mut(), 1), -1);
    assert_eq!(z_feature_map_validate(std::ptr::null()), -1);
    assert_eq!(z_feature_map_num_parameters(std::ptr::null()), 0);
    assert_eq!(z_feature_map_num_qubits(std::ptr::null()), 0);
    assert!(z_feature_map_build_circuit(std::ptr::null(), cstr("x").as_ptr()).is_null());
    z_feature_map_free(std::ptr::null_mut());

    z_feature_map_free(map);
}

#[test]
fn zz_feature_map_structure() {
    let map = zz_feature_map_new(2);
    assert!(!map.is_null());
    assert_eq!(zz_feature_map_num_qubits(map), 2);
    assert_eq!(zz_feature_map_num_parameters(map), 2);
    assert_eq!(zz_feature_map_validate(map), 0);

    // Full topology on 2 qubits: one evolution per layer. The evolution
    // lowering decides the exact op count, so smoke-check the floor.
    let circuit = zz_feature_map_build_circuit(map, cstr("x").as_ptr());
    assert!(!circuit.is_null());
    assert_eq!(circuit_num_qubits(circuit), 2);
    // 2 first-order RZ angles plus one ZZ angle expression per edge; `pi`
    // appears inside the ZZ angle expression `4 * (pi - x_i) * (pi - x_j)`.
    assert_eq!(circuit_num_parameters(circuit), 3);
    assert!(circuit_num_operations(circuit) >= 2 * (2 + 2 + 1));
    circuit_free(circuit);

    // Custom topology validation: self-loops, out-of-bounds and duplicate
    // edges are stored by the setter but rejected by validate.
    assert_eq!(
        zz_feature_map_entanglement(map, ENTANGLEMENT_CUSTOM, [0u32, 0].as_ptr(), 2),
        0
    );
    assert_eq!(zz_feature_map_validate(map), -3);
    assert!(zz_feature_map_build_circuit(map, cstr("x").as_ptr()).is_null());

    assert_eq!(
        zz_feature_map_entanglement(map, ENTANGLEMENT_CUSTOM, [0u32, 5].as_ptr(), 2),
        0
    );
    assert_eq!(zz_feature_map_validate(map), -3);

    assert_eq!(
        zz_feature_map_entanglement(map, ENTANGLEMENT_CUSTOM, [0u32, 1, 1, 0].as_ptr(), 4),
        0
    );
    assert_eq!(zz_feature_map_validate(map), -3);

    // A valid custom pair restores validity.
    assert_eq!(
        zz_feature_map_entanglement(map, ENTANGLEMENT_CUSTOM, [1u32, 0].as_ptr(), 2),
        0
    );
    assert_eq!(zz_feature_map_validate(map), 0);

    // Setter argument validation.
    assert_eq!(
        zz_feature_map_entanglement(map, 99, std::ptr::null(), 0),
        -8
    );
    assert_eq!(
        zz_feature_map_entanglement(map, ENTANGLEMENT_CUSTOM, [0u32, 1].as_ptr(), 3),
        -8
    );
    assert_eq!(
        zz_feature_map_entanglement(map, ENTANGLEMENT_CUSTOM, std::ptr::null(), 2),
        -1
    );
    assert_eq!(
        zz_feature_map_entanglement(
            std::ptr::null_mut(),
            ENTANGLEMENT_LINEAR,
            std::ptr::null(),
            0
        ),
        -1
    );

    // NULL-handle tolerance.
    assert_eq!(zz_feature_map_reps(std::ptr::null_mut(), 1), -1);
    assert_eq!(zz_feature_map_validate(std::ptr::null()), -1);
    assert_eq!(zz_feature_map_num_parameters(std::ptr::null()), 0);
    assert_eq!(zz_feature_map_num_qubits(std::ptr::null()), 0);
    assert!(zz_feature_map_build_circuit(std::ptr::null(), cstr("x").as_ptr()).is_null());
    zz_feature_map_free(std::ptr::null_mut());

    zz_feature_map_free(map);
}

#[test]
fn iqp_feature_map_structure() {
    let map = iqp_feature_map_new(3);
    assert!(!map.is_null());
    assert_eq!(iqp_feature_map_num_qubits(map), 3);
    assert_eq!(iqp_feature_map_num_parameters(map), 3);
    assert_eq!(iqp_feature_map_validate(map), 0);

    // IQP is a named ZZ wrapper: one smoke-built circuit with reps = 1.
    assert_eq!(iqp_feature_map_reps(map, 1), 0);
    assert_eq!(
        iqp_feature_map_entanglement(map, ENTANGLEMENT_CIRCULAR, std::ptr::null(), 0),
        0
    );
    let circuit = iqp_feature_map_build_circuit(map, cstr("x").as_ptr());
    assert!(!circuit.is_null());
    assert_eq!(circuit_num_qubits(circuit), 3);
    // Interned parameter expressions: 3 first-order RZ angles plus one ZZ
    // angle expression per circular edge (3 edges); `pi` appears inside the
    // ZZ expressions, not as its own entry.
    assert_eq!(circuit_num_parameters(circuit), 6);
    // Per layer: 3 H + 3 RZ + 3 circular evolutions (reps = 1).
    assert!(circuit_num_operations(circuit) >= 3 + 3 + 3);
    circuit_free(circuit);

    // Topology argument validation.
    assert_eq!(
        iqp_feature_map_entanglement(map, 42, std::ptr::null(), 0),
        -8
    );
    assert_eq!(
        iqp_feature_map_entanglement(std::ptr::null_mut(), ENTANGLEMENT_FULL, std::ptr::null(), 0),
        -1
    );

    // NULL-handle tolerance.
    assert_eq!(iqp_feature_map_reps(std::ptr::null_mut(), 1), -1);
    assert_eq!(iqp_feature_map_validate(std::ptr::null()), -1);
    assert_eq!(iqp_feature_map_num_parameters(std::ptr::null()), 0);
    assert_eq!(iqp_feature_map_num_qubits(std::ptr::null()), 0);
    assert!(iqp_feature_map_build_circuit(std::ptr::null(), cstr("x").as_ptr()).is_null());
    iqp_feature_map_free(std::ptr::null_mut());

    iqp_feature_map_free(map);
}

#[test]
fn pauli_feature_map_structure() {
    let map = pauli_feature_map_new(2);
    assert!(!map.is_null());
    assert_eq!(pauli_feature_map_num_qubits(map), 2);
    assert_eq!(pauli_feature_map_num_parameters(map), 2);
    assert_eq!(pauli_feature_map_validate(map), 0);

    // Defaults (Z + ZZ, full) build a parameterized circuit.
    assert_eq!(pauli_feature_map_reps(map, 1), 0);
    let circuit = pauli_feature_map_build_circuit(map, cstr("x").as_ptr());
    assert!(!circuit.is_null());
    assert_eq!(circuit_num_qubits(circuit), 2);
    // 2 first-order RZ angles plus one ZZ angle expression for the full
    // topology's single edge.
    assert_eq!(circuit_num_parameters(circuit), 3);
    assert!(circuit_num_operations(circuit) > 2 + 2);
    circuit_free(circuit);

    // Custom Pauli templates.
    let z = cstr("Z");
    let zz = cstr("ZZ");
    let texts = [z.as_ptr(), zz.as_ptr()];
    let labels = [z.as_ptr(), zz.as_ptr()];
    assert_eq!(
        pauli_feature_map_paulis(map, texts.as_ptr(), labels.as_ptr(), 2),
        0
    );
    assert_eq!(pauli_feature_map_validate(map), 0);

    // An empty prefix falls back to the configured parameter prefix.
    assert_eq!(
        pauli_feature_map_parameter_prefix(map, cstr("p").as_ptr()),
        0
    );
    let prefixed = pauli_feature_map_build_circuit(map, cstr("").as_ptr());
    assert!(!prefixed.is_null());
    // 2 first-order RZ angles plus one ZZ angle expression for the full
    // topology's single edge.
    assert_eq!(circuit_num_parameters(prefixed), 3);
    circuit_free(prefixed);

    // Pauli text / label argument validation.
    let bad = cstr("NOT_A_PAULI");
    let bad_texts = [bad.as_ptr()];
    assert_eq!(
        pauli_feature_map_paulis(map, bad_texts.as_ptr(), labels.as_ptr(), 1),
        -8
    );
    let invalid_utf8 = CString::new(vec![0xFF]).unwrap();
    let utf8_texts = [invalid_utf8.as_ptr()];
    assert_eq!(
        pauli_feature_map_paulis(map, utf8_texts.as_ptr(), labels.as_ptr(), 1),
        -4
    );
    let null_texts: [*const c_char; 1] = [std::ptr::null()];
    assert_eq!(
        pauli_feature_map_paulis(map, null_texts.as_ptr(), labels.as_ptr(), 1),
        -1
    );
    assert_eq!(
        pauli_feature_map_paulis(map, std::ptr::null(), labels.as_ptr(), 1),
        -1
    );
    assert_eq!(
        pauli_feature_map_paulis(map, texts.as_ptr(), std::ptr::null(), 1),
        -1
    );
    assert_eq!(
        pauli_feature_map_paulis(std::ptr::null_mut(), texts.as_ptr(), labels.as_ptr(), 1),
        -1
    );
    // The failed setters left the valid configuration in place.
    assert_eq!(pauli_feature_map_validate(map), 0);

    // A Pauli string longer than the qubit count fails validation.
    let zzz = cstr("ZZZ");
    let wide_texts = [zzz.as_ptr()];
    let wide_labels = [zzz.as_ptr()];
    assert_eq!(
        pauli_feature_map_paulis(map, wide_texts.as_ptr(), wide_labels.as_ptr(), 1),
        0
    );
    assert_eq!(pauli_feature_map_validate(map), -3);
    assert!(pauli_feature_map_build_circuit(map, cstr("x").as_ptr()).is_null());

    // Topology and prefix argument validation.
    assert_eq!(
        pauli_feature_map_entanglement(map, 7, std::ptr::null(), 0),
        -8
    );
    assert_eq!(
        pauli_feature_map_entanglement(
            std::ptr::null_mut(),
            ENTANGLEMENT_LINEAR,
            std::ptr::null(),
            0
        ),
        -1
    );
    assert_eq!(
        pauli_feature_map_parameter_prefix(map, std::ptr::null()),
        -1
    );
    assert_eq!(
        pauli_feature_map_parameter_prefix(map, invalid_utf8.as_ptr()),
        -4
    );
    assert_eq!(
        pauli_feature_map_parameter_prefix(std::ptr::null_mut(), cstr("p").as_ptr()),
        -1
    );

    // NULL-handle tolerance.
    assert_eq!(pauli_feature_map_reps(std::ptr::null_mut(), 1), -1);
    assert_eq!(pauli_feature_map_validate(std::ptr::null()), -1);
    assert_eq!(pauli_feature_map_num_parameters(std::ptr::null()), 0);
    assert_eq!(pauli_feature_map_num_qubits(std::ptr::null()), 0);
    assert!(pauli_feature_map_build_circuit(std::ptr::null(), cstr("x").as_ptr()).is_null());
    pauli_feature_map_free(std::ptr::null_mut());

    pauli_feature_map_free(map);
}

#[test]
fn facade_constructors() {
    // EfficientSU2: [RY, RZ] + CX => (1 + 1) * 2 * 2 = 8 parameters.
    let su2 = efficient_su2(2, 1, ENTANGLEMENT_FULL, std::ptr::null(), 0);
    assert!(!su2.is_null());
    assert_eq!(two_local_num_qubits(su2), 2);
    assert_eq!(two_local_num_parameters(su2), 8);
    assert_eq!(two_local_validate(su2), 0);
    two_local_free(su2);

    // Custom topology passes straight through.
    let pairs = [0u32, 1, 1, 2];
    let custom = efficient_su2(3, 1, ENTANGLEMENT_CUSTOM, pairs.as_ptr(), 4);
    assert!(!custom.is_null());
    assert_eq!(two_local_validate(custom), 0);
    two_local_free(custom);

    // RealAmplitudes: [RY] + CX => (2 + 1) * 3 * 1 = 9 parameters.
    let ra = real_amplitudes(3, 2, ENTANGLEMENT_LINEAR, std::ptr::null(), 0);
    assert!(!ra.is_null());
    assert_eq!(two_local_num_parameters(ra), 9);
    assert_eq!(two_local_validate(ra), 0);
    two_local_free(ra);

    // Facade argument validation returns NULL.
    assert!(efficient_su2(2, 1, 99, std::ptr::null(), 0).is_null());
    assert!(efficient_su2(2, 1, ENTANGLEMENT_CUSTOM, pairs.as_ptr(), 3).is_null());
    assert!(real_amplitudes(2, 1, ENTANGLEMENT_CUSTOM, std::ptr::null(), 2).is_null());

    // ZZ feature map facade.
    let zz = zz_feature_map_build(3, 2, ENTANGLEMENT_CIRCULAR, std::ptr::null(), 0);
    assert!(!zz.is_null());
    assert_eq!(zz_feature_map_num_qubits(zz), 3);
    assert_eq!(zz_feature_map_num_parameters(zz), 3);
    assert_eq!(zz_feature_map_validate(zz), 0);
    zz_feature_map_free(zz);
    assert!(zz_feature_map_build(3, 2, 99, std::ptr::null(), 0).is_null());
    assert!(zz_feature_map_build(3, 1, ENTANGLEMENT_CUSTOM, std::ptr::null(), 2).is_null());

    // Pauli feature map facade.
    let z = cstr("Z");
    let zz_text = cstr("ZZ");
    let texts = [z.as_ptr(), zz_text.as_ptr()];
    let labels = [z.as_ptr(), zz_text.as_ptr()];
    let pf = pauli_feature_map_build(
        2,
        1,
        texts.as_ptr(),
        labels.as_ptr(),
        2,
        ENTANGLEMENT_FULL,
        std::ptr::null(),
        0,
    );
    assert!(!pf.is_null());
    assert_eq!(pauli_feature_map_num_parameters(pf), 2);
    assert_eq!(pauli_feature_map_validate(pf), 0);
    pauli_feature_map_free(pf);

    // Facade failure paths return NULL.
    let bad_text = cstr("NOT_A_PAULI");
    let bad_texts = [bad_text.as_ptr()];
    assert!(
        pauli_feature_map_build(
            2,
            1,
            bad_texts.as_ptr(),
            labels.as_ptr(),
            1,
            ENTANGLEMENT_FULL,
            std::ptr::null(),
            0
        )
        .is_null()
    );
    assert!(
        pauli_feature_map_build(
            2,
            1,
            std::ptr::null(),
            labels.as_ptr(),
            1,
            ENTANGLEMENT_FULL,
            std::ptr::null(),
            0
        )
        .is_null()
    );
    assert!(
        pauli_feature_map_build(
            2,
            1,
            texts.as_ptr(),
            labels.as_ptr(),
            2,
            99,
            std::ptr::null(),
            0
        )
        .is_null()
    );
}

#[test]
fn entanglement_topology_helpers() {
    let mut out = [0u32; 16];

    // Linear: (0,1), (1,2), (2,3).
    assert_eq!(
        entanglement_generate_pairs_len(ENTANGLEMENT_LINEAR, std::ptr::null(), 0, 4),
        6
    );
    assert_eq!(
        entanglement_generate_pairs(
            ENTANGLEMENT_LINEAR,
            std::ptr::null(),
            0,
            4,
            out.as_mut_ptr(),
            6
        ),
        0
    );
    assert_eq!(&out[..6], &[0, 1, 1, 2, 2, 3]);

    // Circular adds the wrap-around edge (3, 0).
    assert_eq!(
        entanglement_generate_pairs_len(ENTANGLEMENT_CIRCULAR, std::ptr::null(), 0, 4),
        8
    );
    assert_eq!(
        entanglement_generate_pairs(
            ENTANGLEMENT_CIRCULAR,
            std::ptr::null(),
            0,
            4,
            out.as_mut_ptr(),
            8
        ),
        0
    );
    assert_eq!(&out[..8], &[0, 1, 1, 2, 2, 3, 3, 0]);

    // Full on 3 qubits: C(3,2) = 3 pairs.
    assert_eq!(
        entanglement_generate_pairs_len(ENTANGLEMENT_FULL, std::ptr::null(), 0, 3),
        6
    );
    assert_eq!(
        entanglement_generate_pairs(
            ENTANGLEMENT_FULL,
            std::ptr::null(),
            0,
            3,
            out.as_mut_ptr(),
            6
        ),
        0
    );
    assert_eq!(&out[..6], &[0, 1, 0, 2, 1, 2]);

    // Custom pairs are echoed verbatim; num_qubits is ignored.
    let custom = [1u32, 2, 0, 2];
    assert_eq!(
        entanglement_generate_pairs_len(ENTANGLEMENT_CUSTOM, custom.as_ptr(), 4, 3),
        4
    );
    assert_eq!(
        entanglement_generate_pairs(
            ENTANGLEMENT_CUSTOM,
            custom.as_ptr(),
            4,
            3,
            out.as_mut_ptr(),
            4
        ),
        0
    );
    assert_eq!(&out[..4], &[1, 2, 0, 2]);

    // Pair argument validation.
    assert_eq!(
        entanglement_generate_pairs(
            ENTANGLEMENT_LINEAR,
            std::ptr::null(),
            0,
            4,
            out.as_mut_ptr(),
            5
        ),
        -8
    );
    assert_eq!(
        entanglement_generate_pairs(
            ENTANGLEMENT_LINEAR,
            std::ptr::null(),
            0,
            2,
            std::ptr::null_mut(),
            2
        ),
        -1
    );
    assert_eq!(
        entanglement_generate_pairs(99, std::ptr::null(), 0, 4, out.as_mut_ptr(), 6),
        -8
    );
    assert_eq!(
        entanglement_generate_pairs_len(99, std::ptr::null(), 0, 4),
        0
    );
    assert_eq!(
        entanglement_generate_pairs_len(ENTANGLEMENT_CUSTOM, custom.as_ptr(), 3, 3),
        0
    );
    assert_eq!(
        entanglement_generate_pairs(
            ENTANGLEMENT_CUSTOM,
            custom.as_ptr(),
            3,
            3,
            out.as_mut_ptr(),
            4
        ),
        -8
    );
    assert_eq!(
        entanglement_generate_pairs(
            ENTANGLEMENT_CUSTOM,
            std::ptr::null(),
            2,
            3,
            out.as_mut_ptr(),
            2
        ),
        -1
    );

    // k-tuples, full topology: C(4,3) = 4 lexicographic triples.
    assert_eq!(
        entanglement_generate_k_tuples_len(ENTANGLEMENT_FULL, std::ptr::null(), 0, 3, 4),
        12
    );
    assert_eq!(
        entanglement_generate_k_tuples(
            ENTANGLEMENT_FULL,
            std::ptr::null(),
            0,
            3,
            4,
            out.as_mut_ptr(),
            12
        ),
        0
    );
    assert_eq!(&out[..12], &[0, 1, 2, 0, 1, 3, 0, 2, 3, 1, 2, 3]);

    // Linear k-tuples are consecutive windows.
    assert_eq!(
        entanglement_generate_k_tuples_len(ENTANGLEMENT_LINEAR, std::ptr::null(), 0, 3, 4),
        6
    );
    assert_eq!(
        entanglement_generate_k_tuples(
            ENTANGLEMENT_LINEAR,
            std::ptr::null(),
            0,
            3,
            4,
            out.as_mut_ptr(),
            6
        ),
        0
    );
    assert_eq!(&out[..6], &[0, 1, 2, 1, 2, 3]);

    // Circular k-tuples are rotational windows (mod n).
    assert_eq!(
        entanglement_generate_k_tuples_len(ENTANGLEMENT_CIRCULAR, std::ptr::null(), 0, 3, 4),
        12
    );
    assert_eq!(
        entanglement_generate_k_tuples(
            ENTANGLEMENT_CIRCULAR,
            std::ptr::null(),
            0,
            3,
            4,
            out.as_mut_ptr(),
            12
        ),
        0
    );
    assert_eq!(&out[..12], &[0, 1, 2, 1, 2, 3, 2, 3, 0, 3, 0, 1]);

    // Custom topologies only define pairs: k > 2 falls back to full.
    assert_eq!(
        entanglement_generate_k_tuples_len(ENTANGLEMENT_CUSTOM, custom.as_ptr(), 4, 3, 4),
        12
    );

    // k = 1 yields each qubit on its own.
    assert_eq!(
        entanglement_generate_k_tuples_len(ENTANGLEMENT_LINEAR, std::ptr::null(), 0, 1, 3),
        3
    );
    assert_eq!(
        entanglement_generate_k_tuples(
            ENTANGLEMENT_LINEAR,
            std::ptr::null(),
            0,
            1,
            3,
            out.as_mut_ptr(),
            3
        ),
        0
    );
    assert_eq!(&out[..3], &[0, 1, 2]);

    // Degenerate and invalid inputs.
    assert_eq!(
        entanglement_generate_k_tuples_len(ENTANGLEMENT_FULL, std::ptr::null(), 0, 0, 3),
        0
    );
    assert_eq!(
        entanglement_generate_k_tuples_len(ENTANGLEMENT_FULL, std::ptr::null(), 0, 4, 3),
        0
    );
    assert_eq!(
        entanglement_generate_k_tuples_len(ENTANGLEMENT_FULL, std::ptr::null(), 0, 3, 0),
        0
    );
    assert_eq!(
        entanglement_generate_k_tuples(
            ENTANGLEMENT_FULL,
            std::ptr::null(),
            0,
            3,
            4,
            out.as_mut_ptr(),
            11
        ),
        -8
    );
    assert_eq!(
        entanglement_generate_k_tuples(99, std::ptr::null(), 0, 3, 4, out.as_mut_ptr(), 12),
        -8
    );
    assert_eq!(
        entanglement_generate_k_tuples_len(99, std::ptr::null(), 0, 3, 4),
        0
    );
}

#[test]
fn strongly_entangling_layers_structure() {
    let layers = strongly_entangling_layers_new(3);
    assert!(!layers.is_null());
    assert_eq!(strongly_entangling_layers_num_qubits(layers), 3);
    assert_eq!(strongly_entangling_layers_num_parameters(layers), 9);
    assert_eq!(strongly_entangling_layers_validate(layers), 0);

    // reps = 2 => 2 * 3 * 3 = 18 parameters.
    assert_eq!(strongly_entangling_layers_reps(layers, 2), 0);
    assert_eq!(strongly_entangling_layers_num_parameters(layers), 18);

    // Explicit ranges are reused cyclically across layers.
    let ranges = [1usize, 2];
    assert_eq!(
        strongly_entangling_layers_ranges(layers, ranges.as_ptr(), 2),
        0
    );
    assert_eq!(strongly_entangling_layers_validate(layers), 0);

    // Entangler selection.
    let cz = cstr("CZ");
    let bad_name = cstr("NOT_A_GATE");
    let invalid_utf8 = CString::new(vec![0xFF]).unwrap();
    assert_eq!(
        strongly_entangling_layers_entanglement_gate(layers, cz.as_ptr()),
        0
    );
    assert_eq!(
        strongly_entangling_layers_entanglement_gate(layers, bad_name.as_ptr()),
        -8
    );
    assert_eq!(
        strongly_entangling_layers_entanglement_gate(layers, invalid_utf8.as_ptr()),
        -4
    );
    assert_eq!(
        strongly_entangling_layers_entanglement_gate(std::ptr::null_mut(), cz.as_ptr()),
        -1
    );
    assert_eq!(strongly_entangling_layers_validate(layers), 0);

    // Per layer: 3 U gates + 3 entanglers => 12 operations, 18 parameters.
    let circuit = strongly_entangling_layers_build_circuit(layers, cstr("t").as_ptr());
    assert!(!circuit.is_null());
    assert_eq!(circuit_num_qubits(circuit), 3);
    assert_eq!(circuit_num_operations(circuit), 12);
    assert_eq!(circuit_num_parameters(circuit), 18);
    circuit_free(circuit);

    // Range validation: zero and out-of-range distances fail.
    let zero_range = [0usize];
    assert_eq!(
        strongly_entangling_layers_ranges(layers, zero_range.as_ptr(), 1),
        0
    );
    assert_eq!(strongly_entangling_layers_validate(layers), -3);
    assert!(strongly_entangling_layers_build_circuit(layers, cstr("t").as_ptr()).is_null());

    let big_range = [3usize];
    assert_eq!(
        strongly_entangling_layers_ranges(layers, big_range.as_ptr(), 1),
        0
    );
    assert_eq!(strongly_entangling_layers_validate(layers), -3);

    // An empty explicit list also fails validation until replaced.
    assert_eq!(
        strongly_entangling_layers_ranges(layers, std::ptr::null(), 0),
        0
    );
    assert_eq!(strongly_entangling_layers_validate(layers), -3);
    assert_eq!(
        strongly_entangling_layers_ranges(layers, ranges.as_ptr(), 2),
        0
    );
    assert_eq!(strongly_entangling_layers_validate(layers), 0);

    // ranges NULL array with entries is rejected.
    assert_eq!(
        strongly_entangling_layers_ranges(layers, std::ptr::null(), 1),
        -1
    );
    assert_eq!(
        strongly_entangling_layers_ranges(std::ptr::null_mut(), ranges.as_ptr(), 2),
        -1
    );

    // Prefix validation.
    assert!(strongly_entangling_layers_build_circuit(layers, std::ptr::null()).is_null());
    assert!(strongly_entangling_layers_build_circuit(layers, invalid_utf8.as_ptr()).is_null());

    // A single qubit has no entanglement: only the U gate remains.
    let solo = strongly_entangling_layers_new(1);
    assert!(!solo.is_null());
    assert_eq!(strongly_entangling_layers_num_parameters(solo), 3);
    assert_eq!(strongly_entangling_layers_validate(solo), 0);
    let solo_circuit = strongly_entangling_layers_build_circuit(solo, cstr("t").as_ptr());
    assert!(!solo_circuit.is_null());
    assert_eq!(circuit_num_operations(solo_circuit), 1);
    assert_eq!(circuit_num_parameters(solo_circuit), 3);
    circuit_free(solo_circuit);
    strongly_entangling_layers_free(solo);

    // Zero qubits are never valid.
    let zero = strongly_entangling_layers_new(0);
    assert!(!zero.is_null());
    assert_eq!(strongly_entangling_layers_validate(zero), -3);
    strongly_entangling_layers_free(zero);

    // NULL-handle tolerance.
    assert_eq!(strongly_entangling_layers_reps(std::ptr::null_mut(), 1), -1);
    assert_eq!(strongly_entangling_layers_validate(std::ptr::null()), -1);
    assert_eq!(
        strongly_entangling_layers_num_parameters(std::ptr::null()),
        0
    );
    assert_eq!(strongly_entangling_layers_num_qubits(std::ptr::null()), 0);
    assert!(
        strongly_entangling_layers_build_circuit(std::ptr::null(), cstr("t").as_ptr()).is_null()
    );
    strongly_entangling_layers_free(std::ptr::null_mut());

    strongly_entangling_layers_free(layers);
}

#[test]
fn pauli_evolution_ansatz_time_param_name() {
    // Non-commuting Hamiltonian H = X + Z on one qubit.
    let ham = pauli_hamiltonian("X");
    let ansatz = pauli_evolution_ansatz_new(ham);
    assert!(!ansatz.is_null());

    // Overrides and reset behave per the core semantics.
    assert_eq!(
        pauli_evolution_ansatz_with_time_param_name(ansatz, cstr("tau").as_ptr()),
        0
    );
    assert_eq!(
        pauli_evolution_ansatz_with_time_param_name(ansatz, cstr("").as_ptr()),
        0
    );

    // Argument validation.
    assert_eq!(
        pauli_evolution_ansatz_with_time_param_name(ansatz, std::ptr::null()),
        -1
    );
    let invalid_utf8 = CString::new(vec![0xFF]).unwrap();
    assert_eq!(
        pauli_evolution_ansatz_with_time_param_name(ansatz, invalid_utf8.as_ptr()),
        -4
    );
    assert_eq!(
        pauli_evolution_ansatz_with_time_param_name(std::ptr::null_mut(), cstr("tau").as_ptr()),
        -1
    );

    pauli_evolution_ansatz_free(ansatz);
    hamiltonian_free(ham);

    // Constructor rejects NULL and invalid Hamiltonians.
    assert!(pauli_evolution_ansatz_new(std::ptr::null()).is_null());
    let empty = hamiltonian_new(2);
    assert!(pauli_evolution_ansatz_new(empty).is_null());
    hamiltonian_free(empty);

    pauli_evolution_ansatz_free(std::ptr::null_mut());
}
