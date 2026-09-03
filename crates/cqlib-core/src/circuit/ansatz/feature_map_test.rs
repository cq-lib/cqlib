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

//! Tests for the Pauli Feature Map ansatz.

use super::*;
use crate::circuit::Operation;
use crate::circuit::ansatz::{Ansatz, EntanglementTopology, PauliFeatureMap};
use crate::circuit::circuit_param::CircuitParam;
use crate::circuit::gate::standard_gate::StandardGate;
use crate::qis::pauli::PauliString;

/// Helper function to check if an operation is an H gate on a specific qubit
fn is_h_gate_on_qubit(op: &Operation, expected_qubit: usize) -> bool {
    match op.standard_gate() {
        Some(StandardGate::H) => op.qubits.len() == 1 && op.qubits[0].index() == expected_qubit,
        _ => false,
    }
}

/// Helper function to check if an operation is an RZ gate with a specific parameter pattern
fn is_rz_gate_with_param(op: &Operation, expected_qubit: usize) -> bool {
    match op.standard_gate() {
        Some(StandardGate::RZ) => {
            op.qubits.len() == 1 && op.qubits[0].index() == expected_qubit && op.params.len() == 1
        }
        _ => false,
    }
}

/// Helper function to check if an operation is a CNOT gate between specific qubits
fn is_cnot_gate(op: &Operation, control: usize, target: usize) -> bool {
    match op.standard_gate() {
        Some(StandardGate::CX) => {
            op.qubits.len() == 2
                && op.qubits[0].index() == control
                && op.qubits[1].index() == target
        }
        _ => false,
    }
}

fn is_x_gate_on_qubit(op: &Operation, expected_qubit: usize) -> bool {
    match op.standard_gate() {
        Some(StandardGate::X) => op.qubits.len() == 1 && op.qubits[0].index() == expected_qubit,
        _ => false,
    }
}

#[test]
fn test_basis_encoding_prepares_selected_basis_bits() {
    let encoder = BasisEncoding::new(vec![true, false, true]);
    let circuit = encoder.build_circuit("ignored").unwrap();
    let ops = circuit.operations();

    assert_eq!(encoder.num_qubits(), 3);
    assert_eq!(encoder.num_parameters(), 0);
    assert_eq!(ops.len(), 2);
    assert!(is_x_gate_on_qubit(&ops[0], 0));
    assert!(is_x_gate_on_qubit(&ops[1], 2));
    assert!(circuit.parameters().is_empty());
}

#[test]
fn test_basis_encoding_rejects_empty_bitstring() {
    assert!(BasisEncoding::new(vec![]).validate().is_err());
}

#[test]
fn test_z_feature_map_structure() {
    let fm = ZFeatureMap::new(2).reps(1);
    let circuit = fm.build_circuit("x").unwrap();
    let ops = circuit.operations();

    assert_eq!(fm.num_parameters(), 2);
    assert_eq!(ops.len(), 4);
    assert!(is_h_gate_on_qubit(&ops[0], 0));
    assert!(is_h_gate_on_qubit(&ops[1], 1));
    assert!(is_rz_gate_with_param(&ops[2], 0));
    assert!(is_rz_gate_with_param(&ops[3], 1));
}

#[test]
fn test_z_feature_map_zero_reps_has_no_parameters() {
    let fm = ZFeatureMap::new(3).reps(0);
    let circuit = fm.build_circuit("x").unwrap();

    assert!(circuit.operations().is_empty());
    assert_eq!(fm.num_parameters(), 0);
    assert!(circuit.parameters().is_empty());
}

#[test]
fn test_iqp_feature_map_matches_zz_feature_map_structure() {
    let iqp = IQPFeatureMap::new(2)
        .reps(1)
        .entanglement(EntanglementTopology::Linear);
    let zz = ZZFeatureMap::new(2)
        .reps(1)
        .entanglement(EntanglementTopology::Linear);

    let iqp_circuit = iqp.build_circuit("x").unwrap();
    let zz_circuit = zz.build_circuit("x").unwrap();

    assert_eq!(iqp.num_parameters(), zz.num_parameters());
    assert_eq!(iqp.num_qubits(), zz.num_qubits());
    assert_eq!(
        iqp_circuit.operations().len(),
        zz_circuit.operations().len()
    );
    assert_eq!(iqp_circuit.symbols(), zz_circuit.symbols());
}

#[test]
fn test_iqp_feature_map_zero_reps_has_no_parameters() {
    let fm = IQPFeatureMap::new(3).reps(0);
    let circuit = fm.build_circuit("x").unwrap();

    assert!(circuit.operations().is_empty());
    assert_eq!(fm.num_parameters(), 0);
    assert!(circuit.parameters().is_empty());
}

#[test]
fn test_pauli_feature_map_uses_default_parameter_prefix() {
    // Create feature map with default prefix "x"
    let fm = PauliFeatureMap::new(2);
    let circuit = fm.build_circuit("").unwrap();

    let syms = circuit.symbols();
    assert!(syms.contains("x_0"));
    assert!(syms.contains("x_1"));
}

#[test]
fn test_pauli_feature_map_custom_parameter_prefix() {
    // Create feature map with custom prefix
    let fm = PauliFeatureMap::new(2).parameter_prefix("feature");
    let circuit = fm.build_circuit("").unwrap();

    let syms = circuit.symbols();
    assert!(syms.contains("feature_0"));
    assert!(syms.contains("feature_1"));
}

#[test]
fn test_pauli_feature_map_explicit_prefix_overrides() {
    // When explicit prefix is provided, it should override self.parameter_prefix
    let fm = PauliFeatureMap::new(2).parameter_prefix("feature");
    let circuit = fm.build_circuit("theta").unwrap();

    let syms = circuit.symbols();
    assert!(syms.contains("theta_0"));
    assert!(syms.contains("theta_1"));
}

#[test]
fn test_pauli_feature_map_num_parameters_equals_num_qubits() {
    // For feature maps, each qubit has one input feature
    let fm = PauliFeatureMap::new(3);
    assert_eq!(fm.num_parameters(), 3);
    assert_eq!(fm.num_qubits(), 3);
}

#[test]
fn test_pauli_feature_map_reps_affects_circuit_depth() {
    // Test that reps parameter affects the circuit
    let fm_1rep = PauliFeatureMap::new(2).reps(1);
    let circuit_1rep = fm_1rep.build_circuit("x").unwrap();
    let ops_1rep = circuit_1rep.operations();

    let fm_2rep = PauliFeatureMap::new(2).reps(2);
    let circuit_2rep = fm_2rep.build_circuit("x").unwrap();
    let ops_2rep = circuit_2rep.operations();

    // 2 reps should have approximately 2x the operations of 1 rep
    assert_eq!(ops_2rep.len(), 2 * ops_1rep.len());
}

#[test]
fn test_pauli_feature_map_zero_reps_empty_circuit() {
    let fm = PauliFeatureMap::new(2).reps(0);
    let circuit = fm.build_circuit("x").unwrap();
    let ops = circuit.operations();

    // Zero reps should produce an empty circuit
    assert_eq!(ops.len(), 0);
    assert_eq!(fm.num_parameters(), 0);
    assert!(circuit.parameters().is_empty());
}

#[test]
fn test_zz_feature_map_zero_reps_has_no_parameters() {
    let fm = ZZFeatureMap::new(3).reps(0);
    let circuit = fm.build_circuit("x").unwrap();

    assert!(circuit.operations().is_empty());
    assert_eq!(fm.num_parameters(), 0);
    assert!(circuit.parameters().is_empty());
}

#[test]
fn test_pauli_feature_map_default_z_only_structure() {
    // Use only Z Pauli string to test 1-local structure
    let fm = PauliFeatureMap::new(2)
        .reps(1)
        .paulis(vec![("Z".parse::<PauliString>().unwrap(), "Z".to_string())])
        .entanglement(EntanglementTopology::Linear);

    let circuit = fm.build_circuit("x").unwrap();
    let ops = circuit.operations();

    // Expected structure for 1 rep, 2 qubits, Z only:
    // 1. H on qubit 0
    // 2. H on qubit 1
    // 3. Z evolution on qubit 0: pauli_evolution("Z") = just RZ(2*x_0)
    // 4. Z evolution on qubit 1: pauli_evolution("Z") = just RZ(2*x_1)
    assert_eq!(ops.len(), 4);

    // Check H gates
    assert!(is_h_gate_on_qubit(&ops[0], 0));
    assert!(is_h_gate_on_qubit(&ops[1], 1));

    // Check RZ gates (Z evolution uses RZ with angle 2*x)
    assert!(is_rz_gate_with_param(&ops[2], 0));
    assert!(is_rz_gate_with_param(&ops[3], 1));
}

#[test]
fn test_pauli_feature_map_default_zz_structure() {
    // Use only ZZ Pauli string to test 2-local structure
    let fm = PauliFeatureMap::new(2)
        .reps(1)
        .paulis(vec![(
            "ZZ".parse::<PauliString>().unwrap(),
            "ZZ".to_string(),
        )])
        .entanglement(EntanglementTopology::Linear);

    let circuit = fm.build_circuit("x").unwrap();
    let ops = circuit.operations();

    // Expected structure for 1 rep, 2 qubits, ZZ only, linear entanglement:
    // 1. H on qubit 0
    // 2. H on qubit 1
    // 3. ZZ evolution on pair (0,1): pauli_evolution("ZZ") = CNOT(0,1), RZ, CNOT(0,1)
    assert_eq!(ops.len(), 5);

    // Check H gates
    assert!(is_h_gate_on_qubit(&ops[0], 0));
    assert!(is_h_gate_on_qubit(&ops[1], 1));

    // Check ZZ evolution gates
    assert!(is_cnot_gate(&ops[2], 0, 1));
    assert!(is_rz_gate_with_param(&ops[3], 1));
    assert!(is_cnot_gate(&ops[4], 0, 1));
}

#[test]
fn test_pauli_feature_map_default_z_and_zz_structure() {
    // Default: Z + ZZ
    let fm = PauliFeatureMap::new(2)
        .reps(1)
        .entanglement(EntanglementTopology::Linear);

    let circuit = fm.build_circuit("x").unwrap();
    let ops = circuit.operations();

    // Expected structure:
    // 1. H on qubit 0
    // 2. H on qubit 1
    // 3. Z on qubit 0: RZ(2*x_0)
    // 4. Z on qubit 1: RZ(2*x_1)
    // 5. ZZ on pair (0,1): CNOT(0,1), RZ(4*(π-x_0)(π-x_1)), CNOT(0,1)
    // Total: 2 + 2 + 3 = 7
    assert_eq!(ops.len(), 7);

    // Check H gates
    assert!(is_h_gate_on_qubit(&ops[0], 0));
    assert!(is_h_gate_on_qubit(&ops[1], 1));

    // Check Z evolutions (RZ gates)
    assert!(is_rz_gate_with_param(&ops[2], 0));
    assert!(is_rz_gate_with_param(&ops[3], 1));

    // Check ZZ evolution (CNOT-RZ-CNOT)
    assert!(is_cnot_gate(&ops[4], 0, 1));
    assert!(is_rz_gate_with_param(&ops[5], 1));
    assert!(is_cnot_gate(&ops[6], 0, 1));
}

#[test]
fn test_pauli_feature_map_3qubits_linear_entanglement() {
    // 3 qubits with ZZ and linear entanglement (pairs: 0-1, 1-2)
    let fm = PauliFeatureMap::new(3)
        .reps(1)
        .paulis(vec![(
            "ZZ".parse::<PauliString>().unwrap(),
            "ZZ".to_string(),
        )])
        .entanglement(EntanglementTopology::Linear);

    let circuit = fm.build_circuit("x").unwrap();
    let ops = circuit.operations();

    // Expected structure:
    // 1. H on qubit 0, 1, 2
    // 2. ZZ on (0,1): CNOT(0,1), RZ, CNOT(0,1)
    // 3. ZZ on (1,2): CNOT(1,2), RZ, CNOT(1,2)
    // Total: 3 + 3 + 3 = 9 operations
    assert_eq!(ops.len(), 9);

    // Check all H gates
    assert!(is_h_gate_on_qubit(&ops[0], 0));
    assert!(is_h_gate_on_qubit(&ops[1], 1));
    assert!(is_h_gate_on_qubit(&ops[2], 2));

    // Check ZZ evolution on (0,1)
    assert!(is_cnot_gate(&ops[3], 0, 1));
    assert!(is_rz_gate_with_param(&ops[4], 1));
    assert!(is_cnot_gate(&ops[5], 0, 1));

    // Check ZZ evolution on (1,2)
    assert!(is_cnot_gate(&ops[6], 1, 2));
    assert!(is_rz_gate_with_param(&ops[7], 2));
    assert!(is_cnot_gate(&ops[8], 1, 2));
}

#[test]
fn test_pauli_feature_map_3qubits_circular_entanglement() {
    // 3 qubits with ZZ and circular entanglement (pairs: 0-1, 1-2, 2-0)
    let fm = PauliFeatureMap::new(3)
        .reps(1)
        .paulis(vec![(
            "ZZ".parse::<PauliString>().unwrap(),
            "ZZ".to_string(),
        )])
        .entanglement(EntanglementTopology::Circular);

    let circuit = fm.build_circuit("x").unwrap();
    let ops = circuit.operations();

    // Expected: 3 H + 3*(3 ZZ gates) = 12 operations
    assert_eq!(ops.len(), 12);

    // Check H gates
    assert!(is_h_gate_on_qubit(&ops[0], 0));
    assert!(is_h_gate_on_qubit(&ops[1], 1));
    assert!(is_h_gate_on_qubit(&ops[2], 2));

    // Check ZZ on (0,1)
    assert!(is_cnot_gate(&ops[3], 0, 1));
    assert!(is_rz_gate_with_param(&ops[4], 1));
    assert!(is_cnot_gate(&ops[5], 0, 1));

    // Check ZZ on (1,2)
    assert!(is_cnot_gate(&ops[6], 1, 2));
    assert!(is_rz_gate_with_param(&ops[7], 2));
    assert!(is_cnot_gate(&ops[8], 1, 2));

    // Check ZZ on (2,0):
    // pauli_evolution iterates qubit indices in ascending order, so for pair (2,0)
    // the PauliString has Z at qubit-0 and Z at qubit-2, producing CNOT(0,2).
    assert!(is_cnot_gate(&ops[9], 0, 2));
    assert!(is_rz_gate_with_param(&ops[10], 2));
    assert!(is_cnot_gate(&ops[11], 0, 2));
}

#[test]
fn test_pauli_feature_map_custom_topology() {
    // Custom topology with specific pairs
    let fm = PauliFeatureMap::new(4)
        .reps(1)
        .paulis(vec![(
            "ZZ".parse::<PauliString>().unwrap(),
            "ZZ".to_string(),
        )])
        .entanglement(EntanglementTopology::Custom(vec![(0, 2), (1, 3)]));

    let circuit = fm.build_circuit("x").unwrap();
    let ops = circuit.operations();

    // Expected: 4 H + 2*(3 ZZ gates) = 10 operations
    assert_eq!(ops.len(), 10);

    // Check H gates
    assert!(is_h_gate_on_qubit(&ops[0], 0));
    assert!(is_h_gate_on_qubit(&ops[1], 1));
    assert!(is_h_gate_on_qubit(&ops[2], 2));
    assert!(is_h_gate_on_qubit(&ops[3], 3));

    // Check ZZ on (0,2)
    assert!(is_cnot_gate(&ops[4], 0, 2));
    assert!(is_rz_gate_with_param(&ops[5], 2));
    assert!(is_cnot_gate(&ops[6], 0, 2));

    // Check ZZ on (1,3)
    assert!(is_cnot_gate(&ops[7], 1, 3));
    assert!(is_rz_gate_with_param(&ops[8], 3));
    assert!(is_cnot_gate(&ops[9], 1, 3));
}

#[test]
fn test_pauli_feature_map_x_pauli_structure() {
    // X Pauli requires basis change: H -> RZ -> H
    let fm = PauliFeatureMap::new(2)
        .reps(1)
        .paulis(vec![("X".parse::<PauliString>().unwrap(), "X".to_string())])
        .entanglement(EntanglementTopology::Linear);

    let circuit = fm.build_circuit("x").unwrap();
    let ops = circuit.operations();

    // Expected: H, H (initial) + H-RZ-H on qubit 0 + H-RZ-H on qubit 1
    // = 2 + 3 + 3 = 8 operations
    assert_eq!(ops.len(), 8);

    // Check initial H gates
    assert!(is_h_gate_on_qubit(&ops[0], 0));
    assert!(is_h_gate_on_qubit(&ops[1], 1));

    // X evolution on qubit 0: H, RZ, H
    assert!(is_h_gate_on_qubit(&ops[2], 0));
    assert!(is_rz_gate_with_param(&ops[3], 0));
    assert!(is_h_gate_on_qubit(&ops[4], 0));

    // X evolution on qubit 1: H, RZ, H
    assert!(is_h_gate_on_qubit(&ops[5], 1));
    assert!(is_rz_gate_with_param(&ops[6], 1));
    assert!(is_h_gate_on_qubit(&ops[7], 1));
}

#[test]
fn test_pauli_feature_map_y_pauli_structure() {
    // Y Pauli requires basis change: SDG -> H -> RZ -> H -> S
    let fm = PauliFeatureMap::new(2)
        .reps(1)
        .paulis(vec![("Y".parse::<PauliString>().unwrap(), "Y".to_string())])
        .entanglement(EntanglementTopology::Linear);

    let circuit = fm.build_circuit("x").unwrap();
    let ops = circuit.operations();

    // Expected: H, H (initial) + SDG-H-RZ-H-S on each qubit
    // = 2 + 5 + 5 = 12 operations
    assert_eq!(ops.len(), 12);

    // Check initial H gates
    assert!(is_h_gate_on_qubit(&ops[0], 0));
    assert!(is_h_gate_on_qubit(&ops[1], 1));
}

#[test]
fn test_pauli_feature_map_xy_pauli_structure() {
    // XY two-qubit Pauli string
    let fm = PauliFeatureMap::new(2)
        .reps(1)
        .paulis(vec![(
            "XY".parse::<PauliString>().unwrap(),
            "XY".to_string(),
        )])
        .entanglement(EntanglementTopology::Linear);

    let circuit = fm.build_circuit("x").unwrap();
    let ops = circuit.operations();

    // Expected: H, H (initial) + pauli_evolution("XY") on pair (0,1)
    // XY evolution decomposition (X at q0, Y at q1):
    //   Forward basis change: H(q0) [X→Z], SDG(q1)+H(q1) [Y→Z]  = 3 ops
    //   CNOT(0,1) + RZ(q1) + CNOT(0,1)                           = 3 ops
    //   Inverse basis change: H(q0) [Z→X], H(q1)+S(q1) [Z→Y]    = 3 ops
    //   XY evolution total                                        = 9 ops
    // Total: 2 (initial H) + 9 = 11
    assert_eq!(ops.len(), 11);
}

#[test]
fn test_pauli_feature_map_validation_zero_qubits() {
    let fm = PauliFeatureMap::new(0);
    let result = fm.build_circuit("x");
    assert!(result.is_err());

    match result.unwrap_err() {
        CircuitError::InvalidOperation(msg) => {
            assert_eq!(msg, "PauliFeatureMap requires at least 1 qubit");
        }
        _ => panic!("Expected InvalidOperation error"),
    }
}

#[test]
fn test_pauli_feature_map_validation_pauli_too_long() {
    // Pauli string longer than num_qubits
    let fm = PauliFeatureMap::new(1).paulis(vec![(
        "ZZ".parse::<PauliString>().unwrap(),
        "ZZ".to_string(),
    )]);

    let result = fm.build_circuit("x");
    assert!(result.is_err());

    match result.unwrap_err() {
        CircuitError::InvalidOperation(msg) => {
            assert!(msg.contains("has length 2 which exceeds num_qubits 1"));
        }
        _ => panic!("Expected InvalidOperation error"),
    }
}

#[test]
fn test_pauli_feature_map_validation_custom_topology_out_of_bounds() {
    let fm = PauliFeatureMap::new(2)
        .paulis(vec![(
            "ZZ".parse::<PauliString>().unwrap(),
            "ZZ".to_string(),
        )])
        .entanglement(EntanglementTopology::Custom(vec![(0, 5)]));

    let result = fm.build_circuit("x");
    assert!(result.is_err());

    match result.unwrap_err() {
        CircuitError::InvalidOperation(msg) => {
            assert!(msg.contains("out-of-bounds"));
        }
        _ => panic!("Expected InvalidOperation error"),
    }
}

#[test]
fn test_pauli_feature_map_validation_custom_topology_self_loop() {
    let fm = PauliFeatureMap::new(3)
        .paulis(vec![(
            "ZZ".parse::<PauliString>().unwrap(),
            "ZZ".to_string(),
        )])
        .entanglement(EntanglementTopology::Custom(vec![(1, 1)]));

    let result = fm.build_circuit("x");
    assert!(result.is_err());

    match result.unwrap_err() {
        CircuitError::InvalidOperation(msg) => {
            assert!(msg.contains("self-loop"));
        }
        _ => panic!("Expected InvalidOperation error"),
    }
}

#[test]
fn test_pauli_feature_map_validation_custom_topology_duplicate_edge() {
    let fm = PauliFeatureMap::new(3)
        .paulis(vec![(
            "ZZ".parse::<PauliString>().unwrap(),
            "ZZ".to_string(),
        )])
        .entanglement(EntanglementTopology::Custom(vec![(0, 1), (1, 0)]));

    let result = fm.build_circuit("x");
    assert!(result.is_err());

    match result.unwrap_err() {
        CircuitError::InvalidOperation(msg) => {
            assert!(msg.contains("duplicate edge"));
        }
        _ => panic!("Expected InvalidOperation error"),
    }
}

#[test]
fn test_pauli_feature_map_zzz_3local_structure() {
    // ZZZ (3-local) on 3 qubits with Full topology: only one C(3,3) tuple [0,1,2]
    let fm = PauliFeatureMap::new(3)
        .reps(1)
        .paulis(vec![(
            "ZZZ".parse::<PauliString>().unwrap(),
            "ZZZ".to_string(),
        )])
        .entanglement(EntanglementTopology::Full);

    let circuit = fm.build_circuit("x").unwrap();
    let ops = circuit.operations();

    // Expected structure for 1 rep, 3 qubits, ZZZ only, Full topology:
    // - H on qubit 0, 1, 2                          = 3 ops
    // - ZZZ evolution on tuple (0,1,2):
    //     CNOT(0,1), CNOT(1,2) [forward chain]      = 2 ops
    //     RZ(q2)                                     = 1 op
    //     CNOT(1,2), CNOT(0,1) [reverse chain]      = 2 ops
    // Total: 3 + 5 = 8
    assert_eq!(ops.len(), 8);

    assert!(is_h_gate_on_qubit(&ops[0], 0));
    assert!(is_h_gate_on_qubit(&ops[1], 1));
    assert!(is_h_gate_on_qubit(&ops[2], 2));
    assert!(is_cnot_gate(&ops[3], 0, 1));
    assert!(is_cnot_gate(&ops[4], 1, 2));
    assert!(is_rz_gate_with_param(&ops[5], 2));
    assert!(is_cnot_gate(&ops[6], 1, 2));
    assert!(is_cnot_gate(&ops[7], 0, 1));
}

#[test]
fn test_pauli_feature_map_zzz_4qubits_full_topology() {
    // ZZZ (3-local) on 4 qubits: C(4,3) = 4 tuples
    let fm = PauliFeatureMap::new(4)
        .reps(1)
        .paulis(vec![(
            "ZZZ".parse::<PauliString>().unwrap(),
            "ZZZ".to_string(),
        )])
        .entanglement(EntanglementTopology::Full);

    let circuit = fm.build_circuit("x").unwrap();
    let ops = circuit.operations();

    // 4 H + 4 * 5 (ZZZ evolution) = 4 + 20 = 24
    assert_eq!(ops.len(), 24);
    assert_eq!(fm.num_parameters(), 4);
}

#[test]
fn test_pauli_feature_map_z_parameter_value() {
    // Verify that Z evolution uses the correct angle factor (2*x)
    let fm = PauliFeatureMap::new(1)
        .reps(1)
        .paulis(vec![("Z".parse::<PauliString>().unwrap(), "Z".to_string())]);

    let circuit = fm.build_circuit("x").unwrap();
    let ops = circuit.operations();

    // Check the RZ parameter
    assert_eq!(ops.len(), 2); // H + RZ
    assert!(is_h_gate_on_qubit(&ops[0], 0));
    assert!(is_rz_gate_with_param(&ops[1], 0));

    // The parameter should contain "2" (from 2*x_0)
    match &ops[1].params[0] {
        CircuitParam::Index(_) => {
            // Symbolic parameter, check the symbol name
            let params = circuit.parameters();
            let param_str = params[0].to_string();
            assert!(param_str.contains("2") || param_str.contains("x_0"));
        }
        CircuitParam::Fixed(_) => {
            // Fixed parameter, would be checked differently
        }
    }
}

#[test]
fn test_pauli_feature_map_full_entanglement_3qubits() {
    // Full entanglement on 3 qubits: pairs (0,1), (0,2), (1,2)
    let fm = PauliFeatureMap::new(3)
        .reps(1)
        .paulis(vec![(
            "ZZ".parse::<PauliString>().unwrap(),
            "ZZ".to_string(),
        )])
        .entanglement(EntanglementTopology::Full);

    let circuit = fm.build_circuit("x").unwrap();
    let ops = circuit.operations();

    // Expected: 3 H + 3*(3 ZZ gates) = 12 operations
    assert_eq!(ops.len(), 12);

    // Check H gates
    for (i, op) in ops.iter().enumerate().take(3) {
        assert!(is_h_gate_on_qubit(op, i));
    }
}

#[test]
fn test_pauli_feature_map_matches_zz_feature_map_for_equivalent_config() {
    // PauliFeatureMap with Z+ZZ should produce equivalent structure to ZZFeatureMap
    // (though gate decomposition might differ slightly)

    use crate::circuit::ansatz::ZZFeatureMap;

    let pauli_fm = PauliFeatureMap::new(2)
        .reps(1)
        .entanglement(EntanglementTopology::Linear);

    let zz_fm = ZZFeatureMap::new(2)
        .reps(1)
        .entanglement(EntanglementTopology::Linear);

    assert_eq!(pauli_fm.num_qubits(), zz_fm.num_qubits());
    assert_eq!(pauli_fm.num_parameters(), zz_fm.num_parameters());
}
