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

use super::*;
use crate::circuit::Qubit;
use crate::circuit::ansatz::Ansatz;

#[test]
fn test_qaoa_ansatz_default_mixer() {
    // Cost Hamiltonian: H_C = 0.5 * ZZ on 2 qubits
    let mut h_c = Hamiltonian::new(2);
    h_c.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();

    let ansatz = QAOAAnsatz::new(h_c).unwrap().reps(2);

    // 2 qubits, 2 layers => 4 parameters (gamma_0, beta_0, gamma_1, beta_1)
    assert_eq!(ansatz.num_qubits(), 2);
    assert_eq!(ansatz.num_parameters(), 4);

    let circuit = ansatz.build_circuit("p").unwrap();

    // Check parameters
    assert_eq!(circuit.parameters().len(), 4);
    let syms = circuit.symbols();
    assert!(syms.contains("p_gamma_0"));
    assert!(syms.contains("p_beta_0"));
    assert!(syms.contains("p_gamma_1"));
    assert!(syms.contains("p_beta_1"));

    // Check circuit operations
    let ops = circuit.operations();
    // Initial state: 2 H gates
    // Layer 0:
    //   Cost (ZZ): CNOT, RZ(gamma_0), CNOT (3 gates)
    //   Mixer (XI, IX): H-RZ(beta_0)-H, H-RZ(beta_0)-H (6 gates) -> Total 9 gates
    // Layer 1:
    //   Cost (ZZ): CNOT, RZ(gamma_1), CNOT (3 gates)
    //   Mixer (XI, IX): H-RZ(beta_1)-H, H-RZ(beta_1)-H (6 gates) -> Total 9 gates
    // Total ops: 2 + 9 + 9 = 20
    assert_eq!(ops.len(), 20);
}

#[test]
fn test_qaoa_non_finite_scale_factor_returns_error() {
    let mut h = Hamiltonian::new(1);
    for _ in 0..2 {
        h.add_term("Z".parse().unwrap(), f64::MAX.into()).unwrap();
    }

    for strategy in [
        EvolutionStrategy::Exact,
        EvolutionStrategy::Auto { steps: 2 },
        EvolutionStrategy::Trotter {
            mode: TrotterMode::FirstOrder,
            steps: 2,
        },
        EvolutionStrategy::Trotter {
            mode: TrotterMode::SecondOrder,
            steps: 2,
        },
        EvolutionStrategy::Trotter {
            mode: TrotterMode::Randomized(42),
            steps: 2,
        },
    ] {
        let result = QAOAAnsatz::new(h.clone())
            .unwrap()
            .evolution_strategy(strategy)
            .build_circuit("p");
        assert!(matches!(
            result,
            Err(CircuitError::InvalidOperation(message)) if message.contains("scale factor")
        ));
    }
}

#[test]
fn test_qaoa_custom_mixer() {
    let mut h_c = Hamiltonian::new(2);
    h_c.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();

    // Custom Mixer: H_B = 1.0 * XX
    let mut h_b = Hamiltonian::new(2);
    h_b.add_term("XX".parse().unwrap(), 1.0.into()).unwrap();

    let ansatz = QAOAAnsatz::new(h_c).unwrap().mixer(h_b).unwrap().reps(1);
    let circuit = ansatz.build_circuit("p").unwrap();

    // Ops:
    // Init: 2 H gates
    // Layer 0 Cost: CNOT, RZ, CNOT (3 gates)
    // Layer 0 Mixer(XX): H, H, CNOT, RZ, CNOT, H, H (7 gates)
    // Total: 2 + 3 + 7 = 12
    assert_eq!(circuit.operations().len(), 12);
}

#[test]
fn test_qaoa_mixer_mismatch_error() {
    let mut h_c = Hamiltonian::new(2); // 2 qubits
    h_c.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();

    let mut h_b = Hamiltonian::new(3); // 3 qubits
    h_b.add_term("XXX".parse().unwrap(), 1.0.into()).unwrap();

    let result = QAOAAnsatz::new(h_c).unwrap().mixer(h_b);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CircuitError::QubitCountMismatch {
            expected: 2,
            actual: 3
        }
    ));
}

#[test]
fn test_qaoa_custom_initial_state() {
    // Cost Hamiltonian: H_C = Z_0 + Z_1 on 2 qubits
    let mut h_c = Hamiltonian::new(2);
    h_c.add_term("ZI".parse().unwrap(), 1.0.into()).unwrap();
    h_c.add_term("IZ".parse().unwrap(), 1.0.into()).unwrap();

    // Custom initial state: |01> instead of |++>
    let mut initial_circuit = Circuit::new(2);
    initial_circuit.x(Qubit::new(1)).unwrap();

    let ansatz = QAOAAnsatz::new(h_c)
        .unwrap()
        .initial_state(initial_circuit.clone())
        .unwrap()
        .reps(1);

    assert_eq!(ansatz.num_qubits(), 2);
    assert_eq!(ansatz.num_parameters(), 2);

    let circuit = ansatz.build_circuit("p").unwrap();

    // Check initial state is prepended (should have X gate, no H gates)
    let ops = circuit.operations();
    // Initial: 1 X gate
    // Layer 0 Cost (ZI, IZ): 2 * RZ = 2 gates (Z evolution is just RZ)
    // Layer 0 Mixer (XI, IX): 2 * (H-RZ-H) = 6 gates (X evolution needs basis change)
    // Total: 1 + 2 + 6 = 9
    assert_eq!(ops.len(), 9);

    // First operation should be X gate (from custom initial state)
    let first_op = &ops[0];
    assert!(format!("{:?}", first_op).contains("X"));
}

#[test]
fn test_qaoa_initial_state_mismatch_error() {
    let mut h_c = Hamiltonian::new(2); // 2 qubits
    h_c.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();

    // Initial state with wrong number of qubits (3 instead of 2)
    let mut initial_circuit = Circuit::new(3);
    initial_circuit.h(Qubit::new(0)).unwrap();

    let result = QAOAAnsatz::new(h_c).unwrap().initial_state(initial_circuit);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CircuitError::QubitCountMismatch {
            expected: 2,
            actual: 3
        }
    ));
}

#[test]
fn test_qaoa_zero_reps() {
    // Cost Hamiltonian: H_C = 0.5 * ZZ on 2 qubits
    let mut h_c = Hamiltonian::new(2);
    h_c.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();

    let ansatz = QAOAAnsatz::new(h_c).unwrap().reps(0);

    // 0 layers => 0 parameters
    assert_eq!(ansatz.num_parameters(), 0);

    let circuit = ansatz.build_circuit("p").unwrap();

    // Only initial state: 2 H gates
    let ops = circuit.operations();
    assert_eq!(ops.len(), 2);

    // No parameters
    assert_eq!(circuit.parameters().len(), 0);
}

#[test]
fn test_qaoa_multi_qubit_parameter_naming() {
    // 4 qubits, 3 layers - verify parameter naming is correct
    let mut h_c = Hamiltonian::new(4);
    h_c.add_term("ZZII".parse().unwrap(), 1.0.into()).unwrap();
    h_c.add_term("IZZI".parse().unwrap(), 1.0.into()).unwrap();
    h_c.add_term("IIZZ".parse().unwrap(), 1.0.into()).unwrap();

    let ansatz = QAOAAnsatz::new(h_c).unwrap().reps(3);

    // 4 qubits, 3 layers => 6 parameters (gamma_0, beta_0, gamma_1, beta_1, gamma_2, beta_2)
    assert_eq!(ansatz.num_parameters(), 6);

    let circuit = ansatz.build_circuit("theta").unwrap();
    let syms = circuit.symbols();

    // Verify all expected parameter names are present
    assert!(syms.contains("theta_gamma_0"));
    assert!(syms.contains("theta_beta_0"));
    assert!(syms.contains("theta_gamma_1"));
    assert!(syms.contains("theta_beta_1"));
    assert!(syms.contains("theta_gamma_2"));
    assert!(syms.contains("theta_beta_2"));

    // Verify no extra parameters with wrong indices
    assert!(!syms.contains("theta_gamma_3"));
    assert!(!syms.contains("theta_beta_3"));
}

#[test]
fn test_qaoa_complex_coefficient_error() {
    use num_complex::Complex64;

    // Cost Hamiltonian with complex coefficient (non-Hermitian)
    let mut h_c = Hamiltonian::new(2);
    h_c.add_term("ZZ".parse().unwrap(), Complex64::new(0.5, 0.3))
        .unwrap();

    let ansatz = QAOAAnsatz::new(h_c).unwrap().reps(1);

    // Should fail validation due to complex coefficient
    let result = ansatz.build_circuit("p");
    assert!(result.is_err());
    assert!(matches!(
        &result.unwrap_err(),
        CircuitError::InvalidOperation(msg) if msg.contains("non-zero imaginary part")
    ));
}

#[test]
fn test_qaoa_complex_mixer_coefficient_error() {
    use num_complex::Complex64;

    // Cost Hamiltonian with real coefficient (valid)
    let mut h_c = Hamiltonian::new(2);
    h_c.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();

    // Custom Mixer with complex coefficient (invalid)
    let mut h_b = Hamiltonian::new(2);
    h_b.add_term("XX".parse().unwrap(), Complex64::new(1.0, 0.5))
        .unwrap();

    let ansatz = QAOAAnsatz::new(h_c).unwrap().mixer(h_b).unwrap().reps(1);

    // Should fail validation due to complex mixer coefficient
    let result = ansatz.build_circuit("p");
    assert!(result.is_err());
    assert!(matches!(
        &result.unwrap_err(),
        CircuitError::InvalidOperation(msg) if msg.contains("non-zero imaginary part")
    ));
}
