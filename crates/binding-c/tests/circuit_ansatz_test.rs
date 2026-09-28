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

//! Rust FFI tests for parameterized ansatz generation (`circuit/ansatz.rs`).

use binding_c::circuit::{
    ENTANGLEMENT_CIRCULAR, ENTANGLEMENT_CUSTOM, ENTANGLEMENT_FULL, ENTANGLEMENT_LINEAR,
    EVOLUTION_STRATEGY_AUTO, EVOLUTION_STRATEGY_EXACT, EVOLUTION_STRATEGY_TROTTER,
    TROTTER_FIRST_ORDER, TROTTER_RANDOMIZED, TROTTER_SECOND_ORDER,
    basic_entangler_layers_build_circuit, basic_entangler_layers_entanglement_gate,
    basic_entangler_layers_free, basic_entangler_layers_new, basic_entangler_layers_num_parameters,
    basic_entangler_layers_num_qubits, basic_entangler_layers_reps,
    basic_entangler_layers_rotation_gate, circuit_free, circuit_new, circuit_num_operations,
    circuit_num_parameters, circuit_num_qubits, qaoa_ansatz_build_circuit,
    qaoa_ansatz_evolution_strategy, qaoa_ansatz_free, qaoa_ansatz_initial_state, qaoa_ansatz_mixer,
    qaoa_ansatz_new, qaoa_ansatz_num_parameters, qaoa_ansatz_num_qubits, qaoa_ansatz_reps,
    qaoa_ansatz_validate, two_local_build_circuit, two_local_entanglement,
    two_local_entanglement_gate, two_local_free, two_local_new, two_local_num_parameters,
    two_local_num_qubits, two_local_reps, two_local_rotation_gates,
    two_local_skip_final_rotation_layer, two_local_validate,
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
fn two_local_structure() {
    let cz = cstr("CZ");
    let ansatz = two_local_new(3);
    assert!(!ansatz.is_null());
    assert_eq!(two_local_num_qubits(ansatz), 3);
    assert_eq!(two_local_validate(ansatz), 0);

    // Defaults: reps = 3, RY rotations, final rotation layer kept =>
    // (3 + 1) layers * 3 qubits * 1 gate = 12 parameters.
    assert_eq!(two_local_num_parameters(ansatz), 12);

    // reps = 2 => (2 + 1) * 3 * 1 = 9.
    assert_eq!(two_local_reps(ansatz, 2), 0);
    assert_eq!(two_local_num_parameters(ansatz), 9);

    // Skipping the final rotation layer leaves exactly `reps` layers.
    assert_eq!(two_local_skip_final_rotation_layer(ansatz, true), 0);
    assert_eq!(two_local_num_parameters(ansatz), 2 * 3);

    // The entangler choice never changes the parameter count.
    assert_eq!(two_local_entanglement_gate(ansatz, cz.as_ptr()), 0);
    assert_eq!(two_local_num_parameters(ansatz), 6);
    assert_eq!(two_local_validate(ansatz), 0);

    // Two rotation gates per qubit: 2 layers * 3 qubits * 2 gates = 12.
    let ry = cstr("RY");
    let rz = cstr("RZ");
    let names = [ry.as_ptr(), rz.as_ptr()];
    assert_eq!(two_local_rotation_gates(ansatz, names.as_ptr(), 2), 0);
    assert_eq!(two_local_num_parameters(ansatz), 12);

    // Topologies are accepted and orthogonal to the parameter count.
    assert_eq!(
        two_local_entanglement(ansatz, ENTANGLEMENT_CIRCULAR, std::ptr::null(), 0),
        0
    );
    assert_eq!(
        two_local_entanglement(ansatz, ENTANGLEMENT_FULL, std::ptr::null(), 0),
        0
    );
    assert_eq!(
        two_local_entanglement(ansatz, ENTANGLEMENT_LINEAR, std::ptr::null(), 0),
        0
    );
    let pairs = [1u32, 2, 0, 2];
    assert_eq!(
        two_local_entanglement(ansatz, ENTANGLEMENT_CUSTOM, pairs.as_ptr(), 4),
        0
    );
    assert_eq!(two_local_num_parameters(ansatz), 12);
    assert_eq!(two_local_validate(ansatz), 0);

    two_local_free(ansatz);
}

#[test]
fn two_local_invalid_inputs() {
    // Gate-name parsing: unknown names, case sensitivity, invalid UTF-8.
    let bad_name = cstr("NOT_A_GATE");
    let lowercase = cstr("cx");
    let invalid_utf8 = CString::new(vec![0xFF, 0xFF]).unwrap();
    let ansatz = two_local_new(3);
    assert!(!ansatz.is_null());
    assert_eq!(two_local_entanglement_gate(ansatz, bad_name.as_ptr()), -8);
    assert_eq!(two_local_entanglement_gate(ansatz, lowercase.as_ptr()), -8);
    assert_eq!(two_local_entanglement_gate(ansatz, std::ptr::null()), -1);
    assert_eq!(
        two_local_entanglement_gate(ansatz, invalid_utf8.as_ptr()),
        -4
    );

    // A known single-qubit gate parses fine, but as an entangler it fails
    // validate and build (the entanglement gate must be CX, CY or CZ).
    let h = cstr("H");
    assert_eq!(two_local_entanglement_gate(ansatz, h.as_ptr()), 0);
    assert_eq!(two_local_validate(ansatz), -3);
    assert!(two_local_build_circuit(ansatz, cstr("t").as_ptr()).is_null());
    two_local_free(ansatz);

    // Zero-qubit templates are created but never valid.
    let zero = two_local_new(0);
    assert!(!zero.is_null());
    assert_eq!(two_local_num_qubits(zero), 0);
    assert_eq!(two_local_validate(zero), -3);
    assert!(two_local_build_circuit(zero, cstr("t").as_ptr()).is_null());
    two_local_free(zero);

    // reps = 0 is legal: exactly one rotation layer remains.
    let sparse = two_local_new(3);
    assert_eq!(two_local_reps(sparse, 0), 0);
    assert_eq!(two_local_num_parameters(sparse), 3);
    assert_eq!(two_local_validate(sparse), 0);
    // Skipping the final layer removes those last parameters.
    assert_eq!(two_local_skip_final_rotation_layer(sparse, true), 0);
    assert_eq!(two_local_num_parameters(sparse), 0);
    two_local_free(sparse);

    // rotation_gates argument validation.
    let setter = two_local_new(2);
    assert!(!setter.is_null());
    assert_eq!(two_local_rotation_gates(setter, std::ptr::null(), 1), -1);
    let null_names: [*const c_char; 1] = [std::ptr::null()];
    assert_eq!(two_local_rotation_gates(setter, null_names.as_ptr(), 1), -1);
    // SWAP parses as a standard gate but is rejected by validate as a
    // rotation gate (must be RX, RY or RZ).
    let swap = cstr("SWAP");
    let swap_names = [swap.as_ptr()];
    assert_eq!(two_local_rotation_gates(setter, swap_names.as_ptr(), 1), 0);
    assert_eq!(two_local_validate(setter), -3);
    two_local_free(setter);

    // Topology argument validation.
    let topo = two_local_new(3);
    assert_eq!(two_local_entanglement(topo, 99, std::ptr::null(), 0), -8);
    let pairs = [0u32, 1];
    assert_eq!(
        two_local_entanglement(topo, ENTANGLEMENT_CUSTOM, pairs.as_ptr(), 3),
        -8
    );
    assert_eq!(
        two_local_entanglement(topo, ENTANGLEMENT_CUSTOM, std::ptr::null(), 2),
        -1
    );
    // A self-loop pair is stored but fails validate.
    let self_loop = [1u32, 1];
    assert_eq!(
        two_local_entanglement(topo, ENTANGLEMENT_CUSTOM, self_loop.as_ptr(), 2),
        0
    );
    assert_eq!(two_local_validate(topo), -3);
    two_local_free(topo);

    // build_circuit prefix validation.
    let builder = two_local_new(2);
    assert!(two_local_build_circuit(builder, std::ptr::null()).is_null());
    assert!(two_local_build_circuit(builder, invalid_utf8.as_ptr()).is_null());
    two_local_free(builder);
}

#[test]
fn basic_entangler_layers_structure() {
    let ry = cstr("RY");
    let cz = cstr("CZ");
    let layers = basic_entangler_layers_new(3);
    assert!(!layers.is_null());
    assert_eq!(basic_entangler_layers_num_qubits(layers), 3);

    // Defaults: reps = 1, one rotation per qubit per layer => 3 parameters.
    assert_eq!(basic_entangler_layers_num_parameters(layers), 3);

    // reps = 2 => 6.
    assert_eq!(basic_entangler_layers_reps(layers, 2), 0);
    assert_eq!(basic_entangler_layers_num_parameters(layers), 6);

    // Gate swaps keep the one-parameter-per-qubit-per-layer count.
    assert_eq!(basic_entangler_layers_rotation_gate(layers, ry.as_ptr()), 0);
    assert_eq!(
        basic_entangler_layers_entanglement_gate(layers, cz.as_ptr()),
        0
    );
    assert_eq!(basic_entangler_layers_num_parameters(layers), 6);

    // Bad gate names: unknown, non-UTF-8 and NULL.
    let bad_name = cstr("NOT_A_GATE");
    let invalid_utf8 = CString::new(vec![0xFF]).unwrap();
    assert_eq!(
        basic_entangler_layers_rotation_gate(layers, bad_name.as_ptr()),
        -8
    );
    assert_eq!(
        basic_entangler_layers_entanglement_gate(layers, invalid_utf8.as_ptr()),
        -4
    );
    assert_eq!(
        basic_entangler_layers_rotation_gate(std::ptr::null_mut(), ry.as_ptr()),
        -1
    );
    assert_eq!(
        basic_entangler_layers_entanglement_gate(std::ptr::null_mut(), cz.as_ptr()),
        -1
    );
    // The failed setters leave the template configured.
    assert_eq!(basic_entangler_layers_num_parameters(layers), 6);

    basic_entangler_layers_free(layers);
}

#[test]
fn qaoa_ansatz_structure() {
    // Cost Hamiltonian: a single ZZ term on 2 qubits.
    let cost = pauli_hamiltonian("ZZ");
    let ansatz = qaoa_ansatz_new(cost);
    assert!(!ansatz.is_null());

    // Qubit count is inferred from the cost operator; default reps = 1
    // gives one gamma and one beta angle.
    assert_eq!(qaoa_ansatz_num_qubits(ansatz), 2);
    assert_eq!(qaoa_ansatz_num_parameters(ansatz), 2);
    assert_eq!(qaoa_ansatz_validate(ansatz), 0);

    // reps = 2 doubles the parameter count.
    assert_eq!(qaoa_ansatz_reps(ansatz, 2), 0);
    assert_eq!(qaoa_ansatz_num_parameters(ansatz), 4);

    // A same-width mixer override succeeds and stays valid.
    let mixer = pauli_hamiltonian("XX");
    assert_eq!(qaoa_ansatz_mixer(ansatz, mixer), 0);
    assert_eq!(qaoa_ansatz_validate(ansatz), 0);
    assert_eq!(qaoa_ansatz_num_parameters(ansatz), 4);

    // Mixers on a different qubit count are rejected.
    let wide = pauli_hamiltonian("XXX");
    assert_eq!(qaoa_ansatz_mixer(ansatz, wide), -3);
    assert_eq!(qaoa_ansatz_mixer(ansatz, std::ptr::null()), -1);

    // Initial-state circuits must match the qubit count.
    let init = circuit_new(2);
    assert_eq!(qaoa_ansatz_initial_state(ansatz, init), 0);
    circuit_free(init);
    let big = circuit_new(3);
    assert_eq!(qaoa_ansatz_initial_state(ansatz, big), -3);
    circuit_free(big);
    assert_eq!(qaoa_ansatz_initial_state(ansatz, std::ptr::null()), -1);

    // Evolution strategy selection. ZZ and XX are single terms, so the
    // exact decomposition is valid; Trotter accepts any Hermitian input.
    assert_eq!(
        qaoa_ansatz_evolution_strategy(ansatz, EVOLUTION_STRATEGY_AUTO, TROTTER_FIRST_ORDER, 1, 0),
        0
    );
    assert_eq!(
        qaoa_ansatz_evolution_strategy(ansatz, EVOLUTION_STRATEGY_EXACT, TROTTER_FIRST_ORDER, 0, 0),
        0
    );
    assert_eq!(qaoa_ansatz_validate(ansatz), 0);
    assert_eq!(
        qaoa_ansatz_evolution_strategy(
            ansatz,
            EVOLUTION_STRATEGY_TROTTER,
            TROTTER_SECOND_ORDER,
            2,
            7
        ),
        0
    );
    assert_eq!(qaoa_ansatz_validate(ansatz), 0);
    assert_eq!(
        qaoa_ansatz_evolution_strategy(
            ansatz,
            EVOLUTION_STRATEGY_TROTTER,
            TROTTER_RANDOMIZED,
            2,
            42
        ),
        0
    );
    assert_eq!(qaoa_ansatz_validate(ansatz), 0);

    // Unknown strategy or Trotter-mode tags are rejected.
    assert_eq!(
        qaoa_ansatz_evolution_strategy(ansatz, EVOLUTION_STRATEGY_TROTTER, 99, 1, 0),
        -8
    );
    assert_eq!(
        qaoa_ansatz_evolution_strategy(ansatz, 99, TROTTER_FIRST_ORDER, 1, 0),
        -8
    );

    // Zero-step strategies are stored but fail validation until restored.
    assert_eq!(
        qaoa_ansatz_evolution_strategy(ansatz, EVOLUTION_STRATEGY_AUTO, TROTTER_FIRST_ORDER, 0, 0),
        0
    );
    assert_eq!(qaoa_ansatz_validate(ansatz), -3);
    assert_eq!(
        qaoa_ansatz_evolution_strategy(ansatz, EVOLUTION_STRATEGY_AUTO, TROTTER_FIRST_ORDER, 1, 0),
        0
    );
    assert_eq!(qaoa_ansatz_validate(ansatz), 0);

    qaoa_ansatz_free(ansatz);
    hamiltonian_free(mixer);
    hamiltonian_free(wide);
    hamiltonian_free(cost);

    // The constructor rejects NULL, empty and identity-only cost operators.
    assert!(qaoa_ansatz_new(std::ptr::null()).is_null());
    let empty = hamiltonian_new(2);
    assert!(qaoa_ansatz_new(empty).is_null());
    hamiltonian_free(empty);
    let identity = pauli_hamiltonian("II");
    assert!(qaoa_ansatz_new(identity).is_null());
    hamiltonian_free(identity);
}

#[test]
fn ansatz_null_handles() {
    // TwoLocal: every exported entry point tolerates a NULL handle.
    assert_eq!(two_local_reps(std::ptr::null_mut(), 1), -1);
    assert_eq!(
        two_local_rotation_gates(std::ptr::null_mut(), std::ptr::null(), 0),
        -1
    );
    assert_eq!(
        two_local_entanglement_gate(std::ptr::null_mut(), std::ptr::null()),
        -1
    );
    assert_eq!(
        two_local_entanglement(
            std::ptr::null_mut(),
            ENTANGLEMENT_LINEAR,
            std::ptr::null(),
            0
        ),
        -1
    );
    assert_eq!(
        two_local_skip_final_rotation_layer(std::ptr::null_mut(), true),
        -1
    );
    assert_eq!(two_local_validate(std::ptr::null()), -1);
    assert_eq!(two_local_num_parameters(std::ptr::null()), 0);
    assert_eq!(two_local_num_qubits(std::ptr::null()), 0);
    assert!(two_local_build_circuit(std::ptr::null(), cstr("t").as_ptr()).is_null());

    // BasicEntanglerLayers.
    assert_eq!(basic_entangler_layers_reps(std::ptr::null_mut(), 1), -1);
    assert_eq!(
        basic_entangler_layers_rotation_gate(std::ptr::null_mut(), std::ptr::null()),
        -1
    );
    assert_eq!(
        basic_entangler_layers_entanglement_gate(std::ptr::null_mut(), std::ptr::null()),
        -1
    );
    assert_eq!(basic_entangler_layers_num_parameters(std::ptr::null()), 0);
    assert_eq!(basic_entangler_layers_num_qubits(std::ptr::null()), 0);
    assert!(basic_entangler_layers_build_circuit(std::ptr::null(), cstr("t").as_ptr()).is_null());

    // QAOAAnsatz.
    assert!(qaoa_ansatz_new(std::ptr::null()).is_null());
    assert_eq!(qaoa_ansatz_reps(std::ptr::null_mut(), 1), -1);
    assert_eq!(
        qaoa_ansatz_mixer(std::ptr::null_mut(), std::ptr::null()),
        -1
    );
    assert_eq!(
        qaoa_ansatz_initial_state(std::ptr::null_mut(), std::ptr::null()),
        -1
    );
    assert_eq!(
        qaoa_ansatz_evolution_strategy(
            std::ptr::null_mut(),
            EVOLUTION_STRATEGY_EXACT,
            TROTTER_FIRST_ORDER,
            1,
            0
        ),
        -1
    );
    assert_eq!(qaoa_ansatz_validate(std::ptr::null()), -1);
    assert_eq!(qaoa_ansatz_num_parameters(std::ptr::null()), 0);
    assert_eq!(qaoa_ansatz_num_qubits(std::ptr::null()), 0);
    assert!(qaoa_ansatz_build_circuit(std::ptr::null(), cstr("p").as_ptr()).is_null());

    // Freeing NULL is a no-op everywhere.
    two_local_free(std::ptr::null_mut());
    basic_entangler_layers_free(std::ptr::null_mut());
    qaoa_ansatz_free(std::ptr::null_mut());
    hamiltonian_free(std::ptr::null_mut());
}

#[test]
fn ansatz_build_circuit_smoke() {
    let prefix = cstr("t");

    // TwoLocal: reps = 2, RY, CX, linear. Three rotation layers of 3 RY
    // plus two entanglement layers of 2 CX => 13 operations, 9 parameters.
    let ansatz = two_local_new(3);
    assert_eq!(two_local_reps(ansatz, 2), 0);
    let circuit = two_local_build_circuit(ansatz, prefix.as_ptr());
    assert!(!circuit.is_null());
    assert_eq!(circuit_num_qubits(circuit), 3);
    assert_eq!(circuit_num_operations(circuit), 13);
    assert_eq!(circuit_num_parameters(circuit), 9);
    circuit_free(circuit);

    // Skipping the final rotation layer drops the last 3 RY gates.
    assert_eq!(two_local_skip_final_rotation_layer(ansatz, true), 0);
    let skipped = two_local_build_circuit(ansatz, prefix.as_ptr());
    assert!(!skipped.is_null());
    assert_eq!(circuit_num_operations(skipped), 2 * 3 + 2 * 2);
    assert_eq!(circuit_num_parameters(skipped), 6);
    circuit_free(skipped);

    // Full topology: C(3,2) = 3 CX per entanglement layer => 15 operations.
    assert_eq!(two_local_skip_final_rotation_layer(ansatz, false), 0);
    assert_eq!(
        two_local_entanglement(ansatz, ENTANGLEMENT_FULL, std::ptr::null(), 0),
        0
    );
    let full = two_local_build_circuit(ansatz, prefix.as_ptr());
    assert!(!full.is_null());
    assert_eq!(circuit_num_operations(full), 9 + 2 * 3);
    circuit_free(full);
    two_local_free(ansatz);

    // BasicEntanglerLayers: reps = 2 on a 3-qubit ring => per layer
    // 3 rotations + 3 entanglers => 12 operations, 6 parameters.
    let layers = basic_entangler_layers_new(3);
    assert_eq!(basic_entangler_layers_reps(layers, 2), 0);
    let ring = basic_entangler_layers_build_circuit(layers, prefix.as_ptr());
    assert!(!ring.is_null());
    assert_eq!(circuit_num_qubits(ring), 3);
    assert_eq!(circuit_num_operations(ring), 12);
    assert_eq!(circuit_num_parameters(ring), 6);
    assert!(basic_entangler_layers_build_circuit(layers, std::ptr::null()).is_null());
    circuit_free(ring);
    basic_entangler_layers_free(layers);

    // A single qubit has no entanglement: only the rotations remain.
    let single = basic_entangler_layers_new(1);
    assert_eq!(basic_entangler_layers_reps(single, 2), 0);
    let solo = basic_entangler_layers_build_circuit(single, prefix.as_ptr());
    assert!(!solo.is_null());
    assert_eq!(circuit_num_operations(solo), 2);
    assert_eq!(circuit_num_parameters(solo), 2);
    circuit_free(solo);
    basic_entangler_layers_free(single);

    // QAOA: the H layer plus cost/mixer evolution for one rep. The exact
    // operation count depends on the Pauli-evolution lowering, so only
    // smoke-check the floor while pinning the shape.
    let cost = pauli_hamiltonian("ZZ");
    let qaoa = qaoa_ansatz_new(cost);
    assert!(!qaoa.is_null());
    let qc = qaoa_ansatz_build_circuit(qaoa, prefix.as_ptr());
    assert!(!qc.is_null());
    assert_eq!(circuit_num_qubits(qc), 2);
    assert_eq!(circuit_num_parameters(qc), 2);
    assert!(circuit_num_operations(qc) >= 2);
    assert!(qaoa_ansatz_build_circuit(qaoa, std::ptr::null()).is_null());
    circuit_free(qc);
    qaoa_ansatz_free(qaoa);
    hamiltonian_free(cost);
}
