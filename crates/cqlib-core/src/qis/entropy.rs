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

//! Quantum entropy and entanglement measures.
//!
//! This module provides a comprehensive collection of entropy measures and
//! entanglement metrics for quantum information analysis.
//!
//! # Entropy Measures
//!
//! - [`linear_entropy`]: Fast O(4^N) approximation, no eigendecomposition required
//! - [`renyi_entropy`]: Generalized entropy family, including collision entropy (α=2)
//! - [`entanglement_entropy_pure`]: Bipartite entanglement for pure states
//!
//! # Entanglement Metrics
//!
//! - [`negativity`]: Based on partial transpose, works for mixed states
//! - [`concurrence`]: Exact 2-qubit entanglement measure
//! - [`entanglement_of_formation`]: Derived from concurrence
//!
//! # Performance Notes
//!
//! | Function | Complexity | Requires EVD |
//! |----------|-----------|--------------|
//! | `linear_entropy` | O(4^N) | No |
//! | `renyi_entropy` | O(8^N) | Yes |
//! | `entanglement_entropy_pure` | O(8^N) | Yes |
//! | `negativity` | O(8^N) | Yes |
//! | `concurrence` | O(8^N) | Yes |

use super::error::QisError;
use super::metrics::{density_matrix_to_faer, partial_transpose};
use super::state::{DensityMatrix, Statevector};
use faer::Side;
use rayon::prelude::*;

/// Calculates the linear entropy of a quantum state.
///
/// The linear entropy is defined as:
/// $$S_L(\rho) = 1 - \text{Tr}(\rho^2) = 1 - \text{Purity}(\rho)$$
///
/// This is a computationally efficient approximation of the Von Neumann entropy,
/// requiring only O(4^N) time without eigendecomposition. It serves as a measure
/// of mixedness: 0 for pure states, approaching $1 - 1/2^N$ for maximally mixed states.
///
/// # Arguments
///
/// * `dm` - The density matrix representing the quantum state.
///
/// # Returns
///
/// The linear entropy as a value in [0, 1), or an error if calculation fails.
///
/// # Examples
///
/// ```
/// use cqlib_core::qis::entropy::linear_entropy;
/// use cqlib_core::qis::state::DensityMatrix;
///
/// // Pure state: linear entropy = 0
/// let dm_pure = DensityMatrix::new(2);
/// let s_l = linear_entropy(&dm_pure).unwrap();
/// assert!(s_l < 1e-10);
/// ```
pub fn linear_entropy(dm: &DensityMatrix) -> Result<f64, QisError> {
    let purity: f64 = dm.data().par_iter().map(|c| c.norm_sqr()).sum();
    Ok(1.0 - purity)
}

/// Calculates the Rényi entropy of order α.
///
/// The Rényi entropy is defined as:
/// $$S_\alpha(\rho) = \frac{1}{1-\alpha} \log_2(\text{Tr}(\rho^\alpha)) = \frac{1}{1-\alpha} \log_2\left(\sum_i \lambda_i^\alpha\right)$$
///
/// where $\lambda_i$ are the eigenvalues of the density matrix.
///
/// # Special Cases
///
/// - When α → 1: Approaches Von Neumann entropy (gracefully handled internally)
/// - When α = 2: Collision entropy, important for quantum cryptography
/// - When α → ∞: Min-entropy
///
/// # Arguments
///
/// * `dm` - The density matrix representing the quantum state.
/// * `alpha` - The order parameter. Must be positive and not equal to 1.0
///   (use Von Neumann entropy directly for α = 1).
///
/// # Returns
///
/// The Rényi entropy in bits (base-2 logarithm), or an error if α ≤ 0.
///
/// # Examples
///
/// ```
/// use cqlib_core::qis::entropy::renyi_entropy;
/// use cqlib_core::qis::state::DensityMatrix;
///
/// let dm = DensityMatrix::new(2);
/// // For pure state, all Rényi entropies equal 0
/// let s2 = renyi_entropy(&dm, 2.0).unwrap();
/// assert!(s2 < 1e-10);
/// ```
///
/// # Notes
///
/// For α very close to 1.0 (within `f64::EPSILON`), this function automatically
/// falls back to computing the Von Neumann entropy for numerical stability.
pub fn renyi_entropy(dm: &DensityMatrix, alpha: f64) -> Result<f64, QisError> {
    if alpha <= 0.0 {
        return Err(QisError::InvalidParameterValue(
            "Rényi entropy order alpha must be positive".to_string(),
        ));
    }

    // Graceful degradation: when alpha is very close to 1, use Von Neumann entropy
    if (alpha - 1.0).abs() < f64::EPSILON {
        return super::metrics::entropy(dm);
    }

    let mat = super::metrics::density_matrix_to_faer(dm);
    let eigenvalues: Vec<f64> = mat.self_adjoint_eigenvalues(Side::Lower).map_err(|e| {
        QisError::UnsupportedOperation(format!("Eigendecomposition failed: {:?}", e))
    })?;

    // Compute sum of eigenvalues^alpha, handling numerical noise
    let mut sum_power = 0.0;
    for &eigval in &eigenvalues {
        if eigval > 1e-12 {
            sum_power += eigval.powf(alpha);
        }
    }

    // Rényi entropy formula: 1/(1-α) * log2(sum(λ_i^α))
    let entropy = sum_power.log2() / (1.0 - alpha);
    Ok(entropy.max(0.0))
}

/// Calculates the entanglement entropy for a bipartite pure state.
///
/// For a pure state |ψ⟩ of a composite system AB, the entanglement entropy is
/// defined as the Von Neumann entropy of the reduced density matrix of subsystem A:
/// $$E(|\psi\rangle) = S(\rho_A) = -\text{Tr}(\rho_A \log_2 \rho_A)$$
///
/// where ρ_A = Tr_B(|ψ⟩⟨ψ|) is the reduced density matrix obtained by tracing
/// out subsystem B.
///
/// # Arguments
///
/// * `sv` - The statevector representing the pure quantum state.
/// * `subsys_a` - Indices of qubits belonging to subsystem A. All other qubits
///   are considered part of subsystem B.
///
/// # Returns
///
/// The entanglement entropy in bits, or an error if validation fails.
///
/// # Errors
///
/// Returns `QisError::InvalidSubsystem` if:
/// - `subsys_a` is empty or contains all qubits
/// - There are duplicate indices in `subsys_a`
///
/// Returns `QisError::IndexOutOfBounds` if any index in `subsys_a` is out of bounds.
///
/// # Examples
///
/// ```
/// use cqlib_core::qis::entropy::entanglement_entropy_pure;
/// use cqlib_core::qis::state::Statevector;
///
/// // Create Bell state |Φ+⟩ = (|00⟩ + |11⟩)/√2
/// let mut sv = Statevector::new(2);
/// sv.apply_h(0);
/// sv.apply_cx(0, 1);
///
/// // Entanglement entropy should be 1.0 for maximally entangled Bell state
/// let ee = entanglement_entropy_pure(&sv, &[0]).unwrap();
/// assert!((ee - 1.0).abs() < 1e-10);
/// ```
pub fn entanglement_entropy_pure(sv: &Statevector, subsys_a: &[usize]) -> Result<f64, QisError> {
    // Validate subsystem A
    if subsys_a.is_empty() {
        return Err(QisError::InvalidSubsystem(
            "Subsystem A cannot be empty".to_string(),
        ));
    }

    if subsys_a.len() >= sv.num_qubits {
        return Err(QisError::InvalidSubsystem(
            "Subsystem A must be a proper subset of all qubits".to_string(),
        ));
    }

    // Check for out-of-bounds and duplicates
    let mut sorted_a = subsys_a.to_vec();
    sorted_a.sort_unstable();
    sorted_a.dedup();

    if sorted_a.len() != subsys_a.len() {
        return Err(QisError::InvalidSubsystem(
            "Duplicate qubit indices in subsystem A".to_string(),
        ));
    }

    if let Some(&max_idx) = sorted_a.last()
        && max_idx >= sv.num_qubits
    {
        return Err(QisError::IndexOutOfBounds {
            index: max_idx,
            max: sv.num_qubits - 1,
        });
    }

    // Build density matrix from pure state: ρ = |ψ⟩⟨ψ|
    let dm = DensityMatrix::from_state(sv.num_qubits, sv.data().to_vec())?;

    // Compute reduced density matrix by tracing out subsystem B
    let rho_a = dm.partial_trace(subsys_a)?;

    // Entanglement entropy is the Von Neumann entropy of ρ_A
    super::metrics::entropy(&rho_a)
}

/// Calculates the negativity entanglement measure.
///
/// The negativity is based on the Peres-Horodecki criterion (PPT criterion). For a
/// bipartite state ρ, perform partial transpose on subsystem A. If the resulting
/// matrix has negative eigenvalues, the state is entangled.
///
/// Formula: $\mathcal{N}(\rho) = \sum_{\lambda_i < 0} |\lambda_i| = \frac{||\rho^{T_A}||_1 - 1}{2}$
///
/// where $||\cdot||_1$ denotes the trace norm (sum of singular values).
///
/// # Arguments
///
/// * `dm` - The density matrix of the bipartite quantum state.
/// * `subsys_a` - The qubit indices comprising subsystem A to be transposed.
///
/// # Returns
///
/// The negativity value (≥ 0), or an error if calculation fails.
/// A value of 0 indicates the state is separable (for 2⊗2 and 2⊗3 systems,
/// this is a necessary and sufficient condition).
///
/// # Examples
///
/// ```
/// use cqlib_core::qis::entropy::negativity;
/// use cqlib_core::qis::state::DensityMatrix;
///
/// // Bell state |Φ+⟩
/// let mut dm = DensityMatrix::new(2);
/// dm.apply_h(0);
/// dm.apply_cx(0, 1);
///
/// // Negativity should be 0.5 for maximally entangled Bell state
/// let neg = negativity(&dm, &[0]).unwrap();
/// assert!((neg - 0.5).abs() < 1e-10);
/// ```
///
/// # Notes
///
/// Unlike logarithmic negativity, this measure is not additive under tensor products.
/// For an additive measure, use `logarithmic_negativity` from the metrics module.
pub fn negativity(dm: &DensityMatrix, subsys_a: &[usize]) -> Result<f64, QisError> {
    // Perform partial transpose on subsystem A
    let pt_dm = partial_transpose(dm, subsys_a)?;

    // Convert to faer matrix and compute eigenvalues
    let mat = density_matrix_to_faer(&pt_dm);
    let eigenvalues: Vec<f64> = mat.self_adjoint_eigenvalues(Side::Lower).map_err(|e| {
        QisError::UnsupportedOperation(format!("Eigendecomposition failed: {:?}", e))
    })?;

    // Sum absolute values of negative eigenvalues
    let negativity: f64 = eigenvalues
        .iter()
        .filter(|&&e| e < 0.0)
        .map(|e| e.abs())
        .sum();

    Ok(negativity)
}

/// Calculates the concurrence for a 2-qubit quantum state.
///
/// The concurrence is an exact entanglement measure specifically for two-qubit systems.
/// It ranges from 0 (separable state) to 1 (maximally entangled state).
///
/// Formula: $C(\rho) = \max(0, \lambda_1 - \lambda_2 - \lambda_3 - \lambda_4)$
///
/// where $\lambda_i$ are the eigenvalues (in descending order) of the matrix
/// $R = \sqrt{\sqrt{\rho} \tilde{\rho} \sqrt{\rho}}$, and $\tilde{\rho}$ is the
/// "spin-flipped" density matrix:
/// $$\tilde{\rho} = (\sigma_y \otimes \sigma_y) \rho^* (\sigma_y \otimes \sigma_y)$$
///
/// # Arguments
///
/// * `dm` - The density matrix of a 2-qubit quantum state.
///
/// # Returns
///
/// The concurrence value in [0, 1], or an error if the state is not 2-qubit.
///
/// # Errors
///
/// Returns `QisError::UnsupportedDimension` if `dm.num_qubits != 2`.
///
/// # Examples
///
/// ```
/// use cqlib_core::qis::entropy::concurrence;
/// use cqlib_core::qis::state::DensityMatrix;
///
/// // Bell state |Φ+⟩: concurrence = 1.0
/// let mut dm = DensityMatrix::new(2);
/// dm.apply_h(0);
/// dm.apply_cx(0, 1);
///
/// let c = concurrence(&dm).unwrap();
/// assert!((c - 1.0).abs() < 1e-10);
///
/// // Separable state: concurrence = 0
/// let dm_sep = DensityMatrix::new(2);
/// let c_sep = concurrence(&dm_sep).unwrap();
/// assert!(c_sep < 1e-10);
/// ```
pub fn concurrence(dm: &DensityMatrix) -> Result<f64, QisError> {
    if dm.num_qubits != 2 {
        return Err(QisError::UnsupportedDimension {
            expected: 2,
            actual: dm.num_qubits,
        });
    }

    use faer::Mat;

    let dim = 4; // 2^2 for 2 qubits

    // σ_y ⊗ σ_y (Kronecker product)
    // σ_y = [[0, -i], [i, 0]]
    // σ_y ⊗ σ_y = [[0, 0, 0, -1], [0, 0, 1, 0], [0, 1, 0, 0], [-1, 0, 0, 0]]
    let sy_sy = Mat::from_fn(4, 4, |i, j| {
        let val = match (i, j) {
            (0, 3) => -1.0,
            (1, 2) => 1.0,
            (2, 1) => 1.0,
            (3, 0) => -1.0,
            _ => 0.0,
        };
        faer::c64::new(val, 0.0)
    });

    // Build density matrix as faer Mat
    let rho = Mat::from_fn(dim, dim, |row, col| {
        faer::c64::new(dm.data()[row * dim + col].re, dm.data()[row * dim + col].im)
    });

    // Compute spin-flipped matrix: ρ̃ = (σ_y ⊗ σ_y) ρ* (σ_y ⊗ σ_y)
    let rho_conj: Mat<faer::c64> = rho.map(|c| faer::c64::new(c.re, -c.im));
    let rho_tilde = &sy_sy * &rho_conj * &sy_sy;

    // Compute sqrt(ρ)
    let sqrt_rho = matrix_sqrt(&rho).map_err(|e| {
        QisError::UnsupportedOperation(format!("Failed to compute sqrt(rho): {:?}", e))
    })?;

    // Compute M = sqrt(ρ) * ρ̃ * sqrt(ρ)
    let temp = &sqrt_rho * &rho_tilde * &sqrt_rho;

    // Directly compute eigenvalues of M
    // λ_i of R = sqrt(M) are sqrt of eigenvalues of M
    let m_eigenvalues: Vec<f64> = temp.self_adjoint_eigenvalues(Side::Lower).map_err(|e| {
        QisError::UnsupportedOperation(format!("Eigendecomposition failed: {:?}", e))
    })?;

    // λ_i are square roots of M's eigenvalues
    let mut sorted_eigenvalues: Vec<f64> = m_eigenvalues
        .into_iter()
        .map(|val| val.max(0.0).sqrt())
        .collect();

    // Sort in descending order
    sorted_eigenvalues.sort_by(|a: &f64, b: &f64| b.partial_cmp(a).unwrap());

    // Concurrence: max(0, λ₁ - λ₂ - λ₃ - λ₄)
    let concurrence = (sorted_eigenvalues[0]
        - sorted_eigenvalues[1]
        - sorted_eigenvalues[2]
        - sorted_eigenvalues[3])
        .max(0.0);

    Ok(concurrence)
}

/// Calculates the entanglement of formation for a 2-qubit state.
///
/// The entanglement of formation quantifies the minimum amount of entanglement
/// required to prepare a given mixed state. For 2-qubit systems, it has a closed-form
/// expression in terms of the concurrence.
///
/// Formula: $E_F(\rho) = H\left(\frac{1 + \sqrt{1 - C^2}}{2}\right)$
///
/// where $H(x) = -x \log_2(x) - (1-x) \log_2(1-x)$ is the binary entropy function,
/// and $C$ is the concurrence.
///
/// # Arguments
///
/// * `dm` - The density matrix of a 2-qubit quantum state.
///
/// # Returns
///
/// The entanglement of formation in bits, or an error if the state is not 2-qubit.
///
/// # Examples
///
/// ```
/// use cqlib_core::qis::entropy::entanglement_of_formation;
/// use cqlib_core::qis::state::DensityMatrix;
///
/// // Bell state |Φ+⟩: EOF = 1.0
/// let mut dm = DensityMatrix::new(2);
/// dm.apply_h(0);
/// dm.apply_cx(0, 1);
///
/// let eof = entanglement_of_formation(&dm).unwrap();
/// assert!((eof - 1.0).abs() < 1e-10);
/// ```
///
/// # Notes
///
/// This measure represents the minimum number of Bell pairs required to asymptotically
/// create the state ρ using LOCC (Local Operations and Classical Communication).
pub fn entanglement_of_formation(dm: &DensityMatrix) -> Result<f64, QisError> {
    let c = concurrence(dm)?;

    if c < 1e-15 {
        return Ok(0.0);
    }

    // Compute x = (1 + sqrt(1 - C^2)) / 2
    let sqrt_term = (1.0 - c * c).sqrt();
    let x = (1.0 + sqrt_term) / 2.0;

    // Binary entropy: H(x) = -x log2(x) - (1-x) log2(1-x)
    // Guard against NaN when x is 0 or 1 (0 * log(0) should be 0)
    let mut binary_entropy = 0.0;
    if x > 1e-15 {
        binary_entropy -= x * x.log2();
    }
    if (1.0 - x) > 1e-15 {
        binary_entropy -= (1.0 - x) * (1.0 - x).log2();
    }

    Ok(binary_entropy)
}

/// Helper function to compute the matrix square root of a positive semi-definite matrix.
///
/// Computes √M = U √D U† where M = U D U† is the eigendecomposition.
fn matrix_sqrt(mat: &faer::Mat<faer::c64>) -> Result<faer::Mat<faer::c64>, String> {
    let decomp = mat
        .self_adjoint_eigen(Side::Lower)
        .map_err(|e| format!("{:?}", e))?;
    let u = decomp.U();
    let s = decomp.S();

    // Build sqrt(D)
    let dim = s.dim();
    let mut sqrt_d = faer::Mat::zeros(dim, dim);
    for i in 0..dim {
        let val = s[i].re.max(0.0).sqrt();
        sqrt_d[(i, i)] = faer::c64::new(val, 0.0);
    }

    // √M = U √D U†
    Ok(u * &sqrt_d * u.adjoint())
}

#[cfg(test)]
#[path = "entropy_test.rs"]
mod entropy_test;
