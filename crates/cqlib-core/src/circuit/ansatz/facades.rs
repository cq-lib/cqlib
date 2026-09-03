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

//! Convenient constructor functions for common ansatz patterns.
//!
//! This module provides high-level factory functions that create pre-configured
//! ansatze for common use cases in variational quantum algorithms.
//!
//! # Available Ansatze
//!
//! - [`real_amplitudes`]: Ansatz with real-valued amplitudes, suitable for chemistry
//!   and optimization problems where the wavefunction can be constrained to real numbers.
//!
//! - [`efficient_su2`]: Hardware-efficient ansatz spanning SU(2), widely used in
//!   VQE and Quantum Machine Learning.
//!
//! - [`zz_feature_map`]: Second-order Pauli-Z feature map for quantum kernel methods.
//!
//! - [`pauli_feature_map`]: General-purpose Pauli feature map with arbitrary Pauli strings.
//!
//! # Comparison
//!
//! | Ansatz | Rotation Gates | Entanglement | Use Case |
//! |--------|---------------|--------------|----------|
//! | RealAmplitudes | RY | CX | Chemistry, QAOA with real amplitudes |
//! | EfficientSU2 | RY, RZ | CX | General VQE, QML |
//! | ZZFeatureMap | Z + ZZ | (topology) | Quantum kernel methods |
//! | PauliFeatureMap | arbitrary Paulis | (topology) | QML feature encoding |
//!
//! # Example
//!
//! ```
//! use cqlib_core::circuit::ansatz::{real_amplitudes, efficient_su2, zz_feature_map, EntanglementTopology, Ansatz};
//!
//! // Create a RealAmplitudes ansatz for 4 qubits
//! let ra = real_amplitudes(4, 3, EntanglementTopology::Linear);
//! assert_eq!(ra.num_parameters(), 16); // 4 qubits * 4 layers
//!
//! // Create an EfficientSU2 ansatz
//! let su2 = efficient_su2(3, 2, EntanglementTopology::Full);
//! assert_eq!(su2.num_parameters(), 18); // 3 qubits * 2 gates * 3 layers
//!
//! // Create a ZZFeatureMap for 3 qubits
//! let fm = zz_feature_map(3, 2, EntanglementTopology::Full);
//! assert_eq!(fm.num_parameters(), 3);
//! ```

use super::feature_map::{PauliFeatureMap, ZZFeatureMap};
use super::two_local::{EntanglementTopology, TwoLocal};
use crate::circuit::gate::StandardGate;
use crate::qis::pauli::PauliString;

/// Creates a RealAmplitudes ansatz.
///
/// This is a heuristic trial wave function used as an ansatz in chemistry applications or
/// QAOA where the wave function is constrained to have real amplitudes (i.e., no imaginary parts).
///
/// The ansatz consists of alternating layers of $R_Y$ rotations and $CX$ entanglements.
/// This structure ensures that the resulting statevector has only real components when
/// starting from $|0\rangle^{\otimes n}$.
///
/// # Mathematical Structure
///
/// For $n$ qubits and $r$ repetitions:
///
/// $$|\psi(\theta)\rangle = \left( \prod_{i=0}^{n-1} R_Y(\theta_{r,i}) \right) \prod_{l=0}^{r-1} \left[ \left( \prod_{(i,j) \in E} CX_{i,j} \right) \left( \prod_{i=0}^{n-1} R_Y(\theta_{l,i}) \right) \right] |0\rangle^{\otimes n}$$
///
/// where $E$ is the set of edges defined by the entanglement topology.
///
/// # Arguments
///
/// * `num_qubits` - The number of qubits in the ansatz. Must be at least 1.
/// * `reps` - The number of repetition layers. Each repetition adds a rotation
///   layer and an entanglement layer. The final rotation layer is always included.
/// * `entanglement` - The topology of the entanglement (e.g., Linear, Full).
///   See [`EntanglementTopology`] for options.
///
/// # Returns
///
/// A [`TwoLocal`] instance configured for Real Amplitudes with:
/// - Rotation gates: `[RY]`
/// - Entanglement gate: `CX`
/// - Total parameters: `(reps + 1) × num_qubits`
///
/// # Example
///
/// ```
/// use cqlib_core::circuit::ansatz::{real_amplitudes, EntanglementTopology, Ansatz};
///
/// // 3 qubits, 2 reps, linear entanglement
/// let ansatz = real_amplitudes(3, 2, EntanglementTopology::Linear);
///
/// assert_eq!(ansatz.num_qubits(), 3);
/// assert_eq!(ansatz.num_parameters(), 9); // (2+1) * 3 = 9
///
/// let circuit = ansatz.build_circuit("theta").unwrap();
/// ```
pub fn real_amplitudes(
    num_qubits: usize,
    reps: usize,
    entanglement: EntanglementTopology,
) -> TwoLocal {
    TwoLocal::new(num_qubits)
        .reps(reps)
        .rotation_gates(vec![StandardGate::RY])
        .entanglement_gate(StandardGate::CX)
        .entanglement(entanglement)
}

/// Creates an EfficientSU2 ansatz.
///
/// A hardware-efficient, heuristic ansatz that consists of layers of single-qubit
/// operations spanning SU(2) and $CX$ entanglements. It is widely used in VQE and
/// Quantum Machine Learning models due to its expressibility and relatively low
/// circuit depth.
///
/// The ansatz consists of alternating layers of $[R_Y, R_Z]$ rotations and $CX$ entanglements.
/// Using both $R_Y$ and $R_Z$ allows the ansatz to span the full SU(2) space for each qubit.
///
/// # Mathematical Structure
///
/// For $n$ qubits and $r$ repetitions:
///
/// $$|\psi(\theta, \phi)\rangle = \left( \prod_{i=0}^{n-1} R_Z(\phi_{r,i}) R_Y(\theta_{r,i}) \right) \prod_{l=0}^{r-1} \left[ \left( \prod_{(i,j) \in E} CX_{i,j} \right) \left( \prod_{i=0}^{n-1} R_Z(\phi_{l,i}) R_Y(\theta_{l,i}) \right) \right] |0\rangle^{\otimes n}$$
///
/// where $E$ is the set of edges defined by the entanglement topology.
///
/// # Arguments
///
/// * `num_qubits` - The number of qubits in the ansatz. Must be at least 1.
/// * `reps` - The number of repetition layers. Each repetition adds a rotation
///   layer and an entanglement layer. The final rotation layer is always included.
/// * `entanglement` - The topology of the entanglement (e.g., Linear, Full).
///   See [`EntanglementTopology`] for options.
///
/// # Returns
///
/// A [`TwoLocal`] instance configured for Efficient SU2 with:
/// - Rotation gates: `[RY, RZ]`
/// - Entanglement gate: `CX`
/// - Total parameters: `(reps + 1) × num_qubits × 2`
///
/// # Example
///
/// ```
/// use cqlib_core::circuit::ansatz::{efficient_su2, EntanglementTopology, Ansatz};
///
/// // 2 qubits, 1 rep, full entanglement
/// let ansatz = efficient_su2(2, 1, EntanglementTopology::Full);
///
/// assert_eq!(ansatz.num_qubits(), 2);
/// assert_eq!(ansatz.num_parameters(), 8); // (1+1) * 2 * 2 = 8
///
/// let circuit = ansatz.build_circuit("p").unwrap();
/// ```
pub fn efficient_su2(
    num_qubits: usize,
    reps: usize,
    entanglement: EntanglementTopology,
) -> TwoLocal {
    TwoLocal::new(num_qubits)
        .reps(reps)
        .rotation_gates(vec![StandardGate::RY, StandardGate::RZ])
        .entanglement_gate(StandardGate::CX)
        .entanglement(entanglement)
}

/// Creates a ZZFeatureMap.
///
/// A second-order Pauli-Z feature map widely used in quantum kernel methods.
/// Encodes classical data using single-qubit Z-rotations and two-qubit ZZ-entanglement,
/// making the feature kernel hard to evaluate classically for large circuits.
///
/// The circuit structure for each repetition layer:
/// 1. Hadamard layer on all qubits.
/// 2. `RZ(2 · x_i)` on each qubit i.
/// 3. `e^{-i · 2 · (π − x_i)(π − x_j) · Z_i Z_j}` for each entangled pair `(i, j)`.
///
/// # Arguments
///
/// * `num_qubits` - The number of qubits (= number of input features). Must be ≥ 1.
/// * `reps` - The number of repetition layers. More reps → richer feature space.
/// * `entanglement` - Connectivity pattern for ZZ interactions.
///
/// # Returns
///
/// A [`ZZFeatureMap`] instance. Always has `num_qubits` parameters (one per feature).
///
/// # Example
///
/// ```
/// use cqlib_core::circuit::ansatz::{zz_feature_map, EntanglementTopology, Ansatz};
///
/// let fm = zz_feature_map(3, 2, EntanglementTopology::Full);
/// assert_eq!(fm.num_qubits(), 3);
/// assert_eq!(fm.num_parameters(), 3); // one parameter x_i per qubit
///
/// let circuit = fm.build_circuit("x").unwrap();
/// ```
pub fn zz_feature_map(
    num_qubits: usize,
    reps: usize,
    entanglement: EntanglementTopology,
) -> ZZFeatureMap {
    ZZFeatureMap::new(num_qubits)
        .reps(reps)
        .entanglement(entanglement)
}

/// Creates a PauliFeatureMap.
///
/// A general-purpose data encoding circuit using Pauli evolution gates.
/// Supports arbitrary Pauli strings (e.g., "Z", "ZZ", "XY", "ZZZ") and flexible
/// entanglement topologies.
///
/// For each repetition layer:
/// 1. Hadamard layer on all qubits.
/// 2. For each Pauli template P and each k-tuple of qubit indices from the topology:
///    - If k=1: apply `e^{-i x_i P}` with angle `2 · x_i`.
///    - If k≥2: apply `e^{-i 2 · ∏(π − x_j) P}` with angle `4 · ∏(π − x_j)`.
///
/// # Arguments
///
/// * `num_qubits` - The number of qubits (= number of input features). Must be ≥ 1.
/// * `reps` - The number of repetition layers.
/// * `paulis` - Pauli string templates with labels. Use the [`std::str::FromStr`]
///   implementation for construction.
/// * `entanglement` - Connectivity pattern for multi-qubit interactions.
///
/// # Returns
///
/// A [`PauliFeatureMap`] instance. Always has `num_qubits` parameters (one per feature).
///
/// # Example
///
/// ```
/// use cqlib_core::circuit::ansatz::{pauli_feature_map, EntanglementTopology, Ansatz};
/// use cqlib_core::qis::pauli::PauliString;
///
/// // ZZFeatureMap-equivalent: Z + ZZ paulis
/// let fm = pauli_feature_map(
///     3, 2,
///     vec![
///         ("Z".parse::<PauliString>().unwrap(),  "Z".to_string()),
///         ("ZZ".parse::<PauliString>().unwrap(), "ZZ".to_string()),
///     ],
///     EntanglementTopology::Full,
/// );
/// assert_eq!(fm.num_qubits(), 3);
/// assert_eq!(fm.num_parameters(), 3);
///
/// let circuit = fm.build_circuit("x").unwrap();
/// ```
pub fn pauli_feature_map(
    num_qubits: usize,
    reps: usize,
    paulis: Vec<(PauliString, String)>,
    entanglement: EntanglementTopology,
) -> PauliFeatureMap {
    PauliFeatureMap::new(num_qubits)
        .reps(reps)
        .paulis(paulis)
        .entanglement(entanglement)
}

#[cfg(test)]
#[path = "facades_test.rs"]
mod facades_test;
