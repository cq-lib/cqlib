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

//! Pauli evolution gates and operations.
//!
//! This module provides functionality to implement Pauli rotation gates of the form
//! $e^{-i\theta/2 \cdot P}$ where $P$ is a multi-qubit Pauli string operator.
//!
//! # Algorithm
//!
//! The Pauli rotation is implemented using basis transformation and CNOT ladder:
//!
//! 1. **Basis Transformation**: For each non-identity Pauli operator $P_i \in \{X, Y, Z\}$,
//!    apply a basis transformation to convert it to the $Z$ basis:
//!    - $X \to Z$: Apply $H$ gate
//!    - $Y \to Z$: Apply $H \cdot S^\dagger$ (or equivalently $R_X(\pi/2)$)
//!    - $Z \to Z$: No transformation needed
//!
//! 2. **CNOT Chain (Cascade)**: Use CNOT gates to accumulate parity along a chain
//!    to the last non-identity qubit:
//!    - For non-identity qubits at indices $[i_0, i_1, i_2, \ldots, i_{n-1}]$,
//!      apply $CNOT(i_0 \to i_1), CNOT(i_1 \to i_2), \ldots, CNOT(i_{n-2} \to i_{n-1})$
//!    - This computes the product $P_{i_0} \cdot P_{i_1} \cdot \ldots \cdot P_{i_{n-1}}$
//!      in the computational basis
//!
//! 3. **Core Rotation**: Apply $RZ(\theta)$ on the last non-identity qubit.
//!    The multi-qubit Pauli rotation reduces to a single-qubit RZ after the
//!    parity accumulation.
//!
//! 4. **Reverse CNOT Ladder**: Apply the CNOT gates in reverse order to uncompute
//!    the parity.
//!
//! 5. **Inverse Basis Transformation**: Apply the inverse basis transformations in
//!    reverse order to restore the original basis.
//!
//! # Mathematical Background
//!
//! For a Pauli string $P = \bigotimes_{i} P_i$, the rotation is:
//! $$e^{-i\theta/2 \cdot P} = \cos(\theta/2) I - i\sin(\theta/2) P$$
//!
//! Since all Pauli operators can be simultaneously diagonalized in the $Z$ basis,
//! they share eigenstates. The CNOT ladder effectively performs a basis change
//! from the individual qubit basis to the joint parity basis, allowing the
//! multi-qubit rotation to be performed as a single-qubit RZ gate.

use crate::circuit::bit::Qubit;
use crate::circuit::circuit_impl::Circuit;
use crate::circuit::circuit_param::ParameterValue;
use crate::circuit::error::CircuitError;
use crate::circuit::parameter::Parameter;
use crate::qis::error::QisError;
use crate::qis::hamiltonian::Hamiltonian;
use crate::qis::pauli::{Pauli, PauliString};
use num_complex::Complex64;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

/// Trotter-Suzuki decomposition modes for Hamiltonian time evolution.
///
/// These modes determine how the time evolution operator $U(t) = e^{-iHt}$ is
/// approximated as a product of Pauli rotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrotterMode {
    /// First-order Lie-Trotter decomposition.
    ///
    /// $$U(t) \approx \left[ \prod_k e^{-i c_k t/n \cdot P_k} \right]^n$$
    ///
    /// Error scales as $O(t^2/n)$.
    FirstOrder,

    /// Second-order Strang splitting (symmetric decomposition).
    ///
    /// $$U(t) \approx \left[ e^{-i c_1 t/2n \cdot P_1} \cdots e^{-i c_m t/2n \cdot P_m}
    /// \cdot e^{-i c_m t/2n \cdot P_m} \cdots e^{-i c_1 t/2n \cdot P_1} \right]^n$$
    ///
    /// Error scales as $O(t^3/n^2)$.
    SecondOrder,

    /// Randomized first-order Trotter with specified random seed.
    ///
    /// In each Trotter step, the order of Pauli terms is randomly shuffled.
    /// This can help reduce systematic errors and improve convergence in some cases.
    ///
    /// # Arguments
    /// * `seed` - The random seed for reproducibility
    Randomized(u64),
}

/// Extension trait for Circuit to add Pauli evolution functionality.
///
/// This trait provides methods to append Pauli rotation gates to a circuit.
pub trait PauliEvolution {
    /// Appends a Pauli evolution gate $e^{-i\theta/2 \cdot P}$ to the circuit.
    ///
    /// This method implements the Pauli rotation using the basis transformation
    /// and CNOT ladder algorithm.
    ///
    /// # Arguments
    ///
    /// * `pauli` - The Pauli string operator $P$ to exponentiate. Must be Hermitian
    ///   (i.e., its phase must be $\pm 1$) for the evolution to be unitary.
    /// * `angle` - The rotation angle $\theta$ (can be symbolic or fixed)
    /// * `qubits` - The qubits to apply the operation on (must match `pauli.num_qubits`)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the operation was successfully appended
    /// * `Err(CircuitError)` - If qubit count mismatch, PauliString has non-Hermitian
    ///   phase ($\pm i$), or other error occurs
    ///
    /// # Algorithm Details
    ///
    /// The rotation $e^{-i\theta/2 \cdot P}$ is implemented as follows:
    ///
    /// 1. **Phase Validation**: The PauliString's internal phase must be $\pm 1$
    ///    (Hermitian). If the phase is $\pm i$, an error is returned because
    ///    such operators cannot generate unitary evolution.
    ///
    /// 2. **Phase Absorption**: The PauliString's phase ($\pm 1$) is absorbed into
    ///    the rotation angle: $\theta_{eff} = \theta \cdot \text{phase}$
    ///
    /// 3. **Basis Transformation**: For each qubit $i$ with non-identity Pauli $P_i$:
    ///    - $P_i = X$: Apply $H$ gate (transforms $X$ to $Z$ basis)
    ///    - $P_i = Y$: Apply $S^\dagger$ then $H$ (circuit order: $S^\dagger \to H$,
    ///      i.e. $S^\dagger$ is applied first, then $H$). This transforms $Y \to Z$:
    ///      $S^\dagger |y+\rangle = |x+\rangle$, $H |x+\rangle = |z+\rangle$.
    ///    - $P_i = Z$: No transformation needed
    ///
    /// 4. **CNOT Chain**: For non-identity qubits $[i_0, i_1, \ldots, i_{n-1}]$,
    ///    apply $CNOT(i_0 \to i_1), CNOT(i_1 \to i_2), \ldots$ to accumulate parity.
    ///
    /// 5. **Core Rotation**: Apply $RZ(\theta_{eff})$ on the last non-identity qubit.
    ///
    /// 6. **Reverse CNOT Ladder**: Apply CNOT gates in reverse order.
    ///
    /// 7. **Inverse Transformation**: Apply inverse basis transformations.
    ///
    /// # Global Phase
    ///
    /// If the Pauli string consists entirely of identity operators (e.g., "III"),
    /// the operation reduces to a global phase $e^{-i\theta/2}$, which is added
    /// to the circuit's global phase.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::Circuit;
    /// use cqlib_core::qis::pauli::PauliString;
    /// use cqlib_core::qis::evolution::PauliEvolution;
    ///
    /// let mut circuit = Circuit::new(3);
    /// let qubits = circuit.qubits();
    ///
    /// // Create a Pauli string X ⊗ Z ⊗ I
    /// let pauli: PauliString = "XZI".parse().unwrap();
    ///
    /// // Apply e^(-i * π/4 * XZI)
    /// circuit.pauli_evolution(&pauli, std::f64::consts::PI / 2.0, &qubits).unwrap();
    /// ```
    fn pauli_evolution(
        &mut self,
        pauli: &PauliString,
        angle: impl Into<ParameterValue>,
        qubits: &[Qubit],
    ) -> Result<(), CircuitError>;
}

impl PauliEvolution for Circuit {
    fn pauli_evolution(
        &mut self,
        pauli: &PauliString,
        angle: impl Into<ParameterValue>,
        qubits: &[Qubit],
    ) -> Result<(), CircuitError> {
        // Validate qubit count
        if qubits.len() != pauli.num_qubits {
            return Err(CircuitError::QubitCountMismatch {
                expected: pauli.num_qubits,
                actual: qubits.len(),
            });
        }

        // Check that the PauliString phase is Hermitian (±1)
        // Only Hermitian operators can generate unitary evolution
        let phase_complex = pauli.phase.to_complex();

        // For Hermitian operators, phase must be real (±1)
        // If phase is ±i, the operator is not Hermitian
        if phase_complex.im.abs() > 1e-10 {
            return Err(CircuitError::InvalidOperation(
                "Pauli string phase must be Hermitian (±1)".to_string(),
            ));
        }

        // Absorb PauliString's phase into the rotation angle
        // phase_complex.re is either 1.0 or -1.0
        let angle_param: ParameterValue = angle.into();
        let adjusted_angle = if phase_complex.re < 0.0 {
            multiply_angle_by_factor(angle_param, -1.0)
        } else {
            angle_param
        };

        // Collect non-identity Pauli operators with their positions
        let mut non_identity_positions: Vec<(usize, Pauli)> = Vec::new();
        for i in 0..pauli.num_qubits {
            let x_bit = pauli.x[i];
            let z_bit = pauli.z[i];

            let pauli_op = match (x_bit, z_bit) {
                (false, false) => Pauli::I,
                (true, false) => Pauli::X,
                (true, true) => Pauli::Y,
                (false, true) => Pauli::Z,
            };

            if pauli_op != Pauli::I {
                non_identity_positions.push((i, pauli_op));
            }
        }

        // Handle the case where all Pauli operators are identity
        if non_identity_positions.is_empty() {
            // The operation is e^(-iθ/2 * I) = e^(-iθ/2) * I
            // This is just a global phase
            let global_phase_param = multiply_angle_by_factor(adjusted_angle, -0.5);

            // Add to circuit's global phase
            let current_phase = self.global_phase();
            let global_phase_param = Parameter::from(global_phase_param);
            let new_phase = current_phase + global_phase_param;
            self.set_global_phase(new_phase);

            return Ok(());
        }

        // Apply basis transformations to convert non-Z Paulis to Z basis
        // Store the transformations for later inversion
        let mut transformations: Vec<(Qubit, Pauli)> =
            Vec::with_capacity(non_identity_positions.len());

        for (idx, pauli_op) in &non_identity_positions {
            let qubit = qubits[*idx];
            transformations.push((qubit, *pauli_op));

            match pauli_op {
                Pauli::X => {
                    // X -> Z: Apply H
                    self.h(qubit)?;
                }
                Pauli::Y => {
                    // Y -> Z basis change: circuit order S† → H
                    // S† maps |y+⟩→|x+⟩, then H maps |x+⟩→|z+⟩.
                    // Inverse (uncompute): H → S (see inverse block below).
                    self.sdg(qubit)?;
                    self.h(qubit)?;
                }
                Pauli::Z => {
                    // Already in Z basis, no transformation needed
                }
                Pauli::I => unreachable!(),
            }
        }

        // Chain CNOT ladder: accumulate parity along the chain to the last qubit
        // For non-identity qubits at positions [i0, i1, i2, ...], apply:
        // CNOT(i0 -> i1), CNOT(i1 -> i2), ...
        let num_non_identity = non_identity_positions.len();
        let last_qubit = qubits[non_identity_positions[num_non_identity - 1].0];

        // Forward chain: propagate parity from first to last
        for i in 0..num_non_identity - 1 {
            let curr_qubit = qubits[non_identity_positions[i].0];
            let next_qubit = qubits[non_identity_positions[i + 1].0];
            self.cx(curr_qubit, next_qubit)?;
        }

        // Apply RZ(θ) on the last qubit
        // After the CNOT chain, this effectively rotates the joint parity eigenstate
        self.rz(last_qubit, adjusted_angle)?;

        // Reverse chain: uncompute parity from last-1 down to 0
        for i in (0..num_non_identity - 1).rev() {
            let curr_qubit = qubits[non_identity_positions[i].0];
            let next_qubit = qubits[non_identity_positions[i + 1].0];
            self.cx(curr_qubit, next_qubit)?;
        }

        // Apply inverse basis transformations in reverse order
        for (qubit, pauli_op) in transformations.iter().rev() {
            match pauli_op {
                Pauli::X => {
                    // H† = H
                    self.h(*qubit)?;
                }
                Pauli::Y => {
                    // Inverse of (S† → H): apply H† = H, then S†† = S.
                    // (H · S†)⁻¹ = S · H
                    self.h(*qubit)?;
                    self.s(*qubit)?;
                }
                Pauli::Z => {
                    // No transformation needed
                }
                Pauli::I => unreachable!(),
            }
        }

        Ok(())
    }
}

/// Helper function to multiply an angle parameter by a factor
pub(crate) fn multiply_angle_by_factor(angle: ParameterValue, factor: f64) -> ParameterValue {
    match angle {
        ParameterValue::Fixed(val) => ParameterValue::Fixed(val * factor),
        ParameterValue::Param(param) => ParameterValue::Param(Parameter::from(factor) * param),
    }
}

// These `pub(crate)` functions implement the product-formula math for both the
// numeric API (`to_trotter_circuit` / `to_evolution_circuit`) and the parametric
// ansatz (`PauliEvolutionAnsatz` in `circuit/ansatz/hamiltonian_evolution.rs`).
//
// Angle convention: `pauli_evolution` implements e^{-iθ/2 · P}; to realize
// e^{-i c t P} we pass θ = 2 c t.
//
// Both `ParameterValue::Fixed(f64)` and `ParameterValue::Param(Parameter)` are
// handled uniformly via `multiply_angle_by_factor`.

/// Applies first-order Lie-Trotter (or randomized product-formula) decomposition.
///
/// Realizes:
/// $$U(t) \approx \left[\prod_k e^{-i c_k t/n \cdot P_k}\right]^n$$
///
/// # Arguments
///
/// * `terms`  – Pauli terms of the (simplified) Hamiltonian.
/// * `t`      – Total evolution time, numeric (`Fixed`) or symbolic (`Param`).
/// * `steps`  – Number of Trotter repetitions $n$.
/// * `qubits` – Circuit qubits (must cover all Pauli positions).
/// * `rng`    – If `Some`, term order is shuffled each step (randomized product formula).
///   If `None`, terms are applied in fixed order (standard first-order).
///
/// # Performance
///
/// Angle `ParameterValue`s are pre-computed once per term (outside the step loop)
/// to avoid O(m·n) Arc-node allocations for the symbolic case.
pub(crate) fn trotter_first_order_core(
    circuit: &mut Circuit,
    terms: &[(PauliString, Complex64)],
    t: ParameterValue,
    steps: usize,
    qubits: &[Qubit],
    mut rng: Option<&mut StdRng>,
) -> Result<(), CircuitError> {
    // Pre-compute one ParameterValue per term: θ_k = 2 c_k t / n.
    let angles: Vec<ParameterValue> = terms
        .iter()
        .map(|(_, coeff)| multiply_angle_by_factor(t.clone(), 2.0 * coeff.re / steps as f64))
        .collect();

    let mut indices: Vec<usize> = (0..terms.len()).collect();

    for _ in 0..steps {
        if let Some(ref mut r) = rng {
            indices.shuffle(*r);
        }
        for &idx in &indices {
            let (pauli, _) = &terms[idx];
            circuit
                .pauli_evolution(pauli, angles[idx].clone(), qubits)
                .map_err(|e| {
                    CircuitError::InvalidOperation(format!("Failed to add Pauli evolution: {}", e))
                })?;
        }
    }
    Ok(())
}

/// Applies second-order Suzuki (Strang) splitting decomposition.
///
/// Realizes:
/// $$U(t) \approx \left[\prod_k e^{-i c_k t/(2n) P_k}
///   \cdot \prod_k^{\leftarrow} e^{-i c_k t/(2n) P_k}\right]^n$$
///
/// # Arguments
///
/// * `terms`  – Pauli terms of the (simplified) Hamiltonian.
/// * `t`      – Total evolution time, numeric (`Fixed`) or symbolic (`Param`).
/// * `steps`  – Number of Suzuki repetitions $n$.
/// * `qubits` – Circuit qubits.
///
/// # Performance
///
/// Half-step angles are pre-computed per term (θ_k = c_k t / n) outside the step
/// loop; forward and backward passes share the same angle array.
pub(crate) fn trotter_second_order_core(
    circuit: &mut Circuit,
    terms: &[(PauliString, Complex64)],
    t: ParameterValue,
    steps: usize,
    qubits: &[Qubit],
) -> Result<(), CircuitError> {
    // Half-step angle: θ_k = 2 c_k (t/2n) = c_k t / n.
    // Forward and backward passes share the same value by symmetry.
    let angles: Vec<ParameterValue> = terms
        .iter()
        .map(|(_, coeff)| multiply_angle_by_factor(t.clone(), coeff.re / steps as f64))
        .collect();

    for _ in 0..steps {
        // Forward half-step
        for (idx, (pauli, _)) in terms.iter().enumerate() {
            circuit
                .pauli_evolution(pauli, angles[idx].clone(), qubits)
                .map_err(|e| {
                    CircuitError::InvalidOperation(format!("Failed to add Pauli evolution: {}", e))
                })?;
        }
        // Backward half-step in reverse term order
        for (idx, (pauli, _)) in terms.iter().enumerate().rev() {
            circuit
                .pauli_evolution(pauli, angles[idx].clone(), qubits)
                .map_err(|e| {
                    CircuitError::InvalidOperation(format!("Failed to add Pauli evolution: {}", e))
                })?;
        }
    }
    Ok(())
}

impl Hamiltonian {
    /// Converts the Hamiltonian to a Trotterized time evolution circuit.
    ///
    /// Implements Trotter-Suzuki decomposition to approximate the time evolution
    /// operator $U(t) = e^{-iHt}$ as a sequence of Pauli rotations.
    ///
    /// This method **always** applies the product formula regardless of whether
    /// the Hamiltonian terms commute. For an auto-selecting method that uses the
    /// mathematically exact single-pass decomposition when all terms commute, see
    /// [`to_evolution_circuit`](Hamiltonian::to_evolution_circuit).
    ///
    /// # Arguments
    ///
    /// * `time` - The total evolution time $t$
    /// * `steps` - The number of Trotter steps $n$ (must be > 0)
    /// * `mode` - The Trotter decomposition mode (FirstOrder, SecondOrder, Randomized)
    ///
    /// # Returns
    ///
    /// * `Ok(Circuit)` - The approximated time evolution circuit
    /// * `Err(QisError)` - If steps is 0, Hamiltonian is empty, or other error occurs
    ///
    /// # Mathematical Formulation
    ///
    /// For $H = \sum_k c_k P_k$, the decomposition approximates:
    ///
    /// **First Order:**
    /// $$U(t) \approx \prod_{s=1}^{n} \prod_k e^{-i c_k \Delta t \cdot P_k}, \quad \Delta t = t/n$$
    ///
    /// **Second Order:**
    /// $$U(t) \approx \prod_{s=1}^{n} \left[ \prod_k e^{-i c_k \Delta t/2 \cdot P_k}
    /// \cdot \prod_k e^{-i c_k \Delta t/2 \cdot P_k} \right]$$
    /// (with reverse order in second product)
    ///
    /// **Randomized:**
    /// Like first order, but with randomized term order in each step.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::qis::{Hamiltonian, PauliString};
    /// use cqlib_core::qis::evolution::{PauliEvolution, TrotterMode};
    ///
    /// // Create H = 0.5 * ZZ + 0.3 * XX
    /// let mut h = Hamiltonian::new(2);
    /// h.add_term("ZZ".parse::<PauliString>().unwrap(), 0.5.into()).unwrap();
    /// h.add_term("XX".parse::<PauliString>().unwrap(), 0.3.into()).unwrap();
    ///
    /// // Generate Trotter circuit for t=1.0 with 10 steps
    /// let circuit = h.to_trotter_circuit(1.0, 10, TrotterMode::FirstOrder).unwrap();
    ///
    /// // Circuit contains Pauli evolution gates
    /// assert!(circuit.num_qubits() >= 2);
    /// ```
    pub fn to_trotter_circuit(
        &self,
        time: f64,
        steps: usize,
        mode: TrotterMode,
    ) -> Result<Circuit, QisError> {
        if steps == 0 {
            return Err(QisError::InvalidParameterValue(
                "Trotter steps must be greater than 0".to_string(),
            ));
        }

        if self.terms.is_empty() {
            return Err(QisError::InvalidParameterValue(
                "Cannot create Trotter circuit from empty Hamiltonian".to_string(),
            ));
        }

        let mut simplified_h = self.clone();
        simplified_h.simplify();

        for (_, coeff) in &simplified_h.terms {
            if coeff.im.abs() > 1e-10 {
                return Err(QisError::NotHermitian);
            }
        }

        let mut circuit = Circuit::new(simplified_h.num_qubits);
        let qubits: Vec<Qubit> = circuit.qubits();
        let t = ParameterValue::Fixed(time);

        // Dispatch to shared Trotter core helpers.
        //
        // NOTE: The parametric counterpart of this logic lives in
        // `PauliEvolutionAnsatz::build_circuit()` (circuit/ansatz/hamiltonian_evolution.rs),
        // which calls the same `trotter_first_order_core` / `trotter_second_order_core`
        // with a symbolic `ParameterValue::Param(t)` instead of `Fixed(f64)`.
        let err_map = |e: CircuitError| {
            QisError::UnsupportedOperation(format!("Failed to add Pauli evolution: {}", e))
        };

        match mode {
            TrotterMode::FirstOrder => {
                trotter_first_order_core(
                    &mut circuit,
                    &simplified_h.terms,
                    t,
                    steps,
                    &qubits,
                    None,
                )
                .map_err(err_map)?;
            }
            TrotterMode::Randomized(seed) => {
                let mut rng = StdRng::seed_from_u64(seed);
                trotter_first_order_core(
                    &mut circuit,
                    &simplified_h.terms,
                    t,
                    steps,
                    &qubits,
                    Some(&mut rng),
                )
                .map_err(err_map)?;
            }
            TrotterMode::SecondOrder => {
                trotter_second_order_core(&mut circuit, &simplified_h.terms, t, steps, &qubits)
                    .map_err(err_map)?;
            }
        }

        Ok(circuit)
    }

    /// Converts the Hamiltonian to a time evolution circuit, automatically selecting
    /// the exact decomposition when possible.
    ///
    /// Unlike [`to_trotter_circuit`](Hamiltonian::to_trotter_circuit), this method
    /// checks whether all Hamiltonian terms mutually commute:
    ///
    /// - **Commuting terms**: uses an exact single-pass decomposition
    ///   $U(t) = \prod_k e^{-i c_k t P_k}$ (no approximation error, `steps` is ignored).
    /// - **Non-commuting terms**: falls back to the Trotter approximation specified by
    ///   `mode` and `steps`.
    ///
    /// This mirrors the behavior of [`PauliEvolutionAnsatz`] with
    /// [`EvolutionStrategy::Auto`], closing the capability gap between the numeric
    /// and parametric APIs.
    ///
    /// # Arguments
    ///
    /// * `time`  - Total evolution time $t$
    /// * `steps` - Trotter steps (used only when terms are non-commuting; must be ≥ 1)
    /// * `mode`  - Trotter mode (used only when terms are non-commuting)
    ///
    /// # Returns
    ///
    /// * `Ok(Circuit)` - The evolution circuit (exact or approximate)
    /// * `Err(QisError)` - If the Hamiltonian is empty, non-Hermitian, or `steps == 0`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::qis::PauliString;
    /// use cqlib_core::qis::Hamiltonian;
    /// use cqlib_core::qis::evolution::TrotterMode;
    ///
    /// // Commuting Hamiltonian: ZZ and ZI share eigenstates — exact evolution
    /// let mut h = Hamiltonian::new(2);
    /// h.add_term("ZZ".parse::<PauliString>().unwrap(), 0.5.into()).unwrap();
    /// h.add_term("ZI".parse::<PauliString>().unwrap(), 0.3.into()).unwrap();
    /// // steps/mode are irrelevant here — exact path is taken automatically
    /// let circuit = h.to_evolution_circuit(1.0, 1, TrotterMode::FirstOrder).unwrap();
    ///
    /// // Non-commuting: uses Trotter
    /// let mut h2 = Hamiltonian::new(1);
    /// h2.add_term("X".parse::<PauliString>().unwrap(), 1.0.into()).unwrap();
    /// h2.add_term("Z".parse::<PauliString>().unwrap(), 1.0.into()).unwrap();
    /// let circuit2 = h2.to_evolution_circuit(0.5, 10, TrotterMode::SecondOrder).unwrap();
    /// ```
    pub fn to_evolution_circuit(
        &self,
        time: f64,
        steps: usize,
        mode: TrotterMode,
    ) -> Result<Circuit, QisError> {
        if steps == 0 {
            return Err(QisError::InvalidParameterValue(
                "Trotter steps must be greater than 0".to_string(),
            ));
        }

        if self.terms.is_empty() {
            return Err(QisError::InvalidParameterValue(
                "Cannot create evolution circuit from empty Hamiltonian".to_string(),
            ));
        }

        let mut simplified_h = self.clone();
        simplified_h.simplify();

        for (_, coeff) in &simplified_h.terms {
            if coeff.im.abs() > 1e-10 {
                return Err(QisError::NotHermitian);
            }
        }

        let mut circuit = Circuit::new(simplified_h.num_qubits);
        let qubits: Vec<Qubit> = circuit.qubits();

        let err_map = |e: CircuitError| {
            QisError::UnsupportedOperation(format!("Failed to add Pauli evolution: {}", e))
        };

        if simplified_h.all_terms_commute() {
            // Exact decomposition: ∏_k e^{-i c_k t P_k}
            // Angle convention: pauli_evolution implements e^{-iθ/2 P}, so θ = 2 c t.
            for (pauli, coeff) in &simplified_h.terms {
                circuit
                    .pauli_evolution(pauli, 2.0 * coeff.re * time, &qubits)
                    .map_err(err_map)?;
            }
        } else {
            // Non-commuting: Trotter approximation
            let t = ParameterValue::Fixed(time);
            match mode {
                TrotterMode::FirstOrder => {
                    trotter_first_order_core(
                        &mut circuit,
                        &simplified_h.terms,
                        t,
                        steps,
                        &qubits,
                        None,
                    )
                    .map_err(err_map)?;
                }
                TrotterMode::Randomized(seed) => {
                    let mut rng = StdRng::seed_from_u64(seed);
                    trotter_first_order_core(
                        &mut circuit,
                        &simplified_h.terms,
                        t,
                        steps,
                        &qubits,
                        Some(&mut rng),
                    )
                    .map_err(err_map)?;
                }
                TrotterMode::SecondOrder => {
                    trotter_second_order_core(&mut circuit, &simplified_h.terms, t, steps, &qubits)
                        .map_err(err_map)?;
                }
            }
        }

        Ok(circuit)
    }
}

#[cfg(test)]
#[path = "evolution_test.rs"]
mod evolution_test;
