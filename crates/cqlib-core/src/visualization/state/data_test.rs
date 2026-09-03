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

//! Tests for state visualization data conversion.

use super::*;
use crate::qis::Statevector;
use crate::visualization::VisualizationError;
use num_complex::Complex64;

struct BadDensitySource {
    num_qubits: usize,
    data: Vec<Complex64>,
}

impl StateVisualizationSource for BadDensitySource {
    fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    fn density_matrix_data(&self) -> Result<Vec<Complex64>, VisualizationError> {
        Ok(self.data.clone())
    }
}

#[test]
fn statevector_converts_to_density_matrix() {
    let zero = Statevector::from_state(1, vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)])
        .unwrap();
    let (_, rho) = state_to_density_matrix(&zero).unwrap();
    assert_eq!(rho[0], Complex64::new(1.0, 0.0));
    assert_eq!(rho[3], Complex64::new(0.0, 0.0));
}

#[test]
fn density_matrix_rejects_wrong_length() {
    let source = BadDensitySource {
        num_qubits: 2,
        data: vec![Complex64::new(1.0, 0.0); 4],
    };
    let err = state_to_density_matrix(&source).unwrap_err();
    assert!(
        err.to_string()
            .contains("density matrix length 4 does not match 4^2")
    );
}
