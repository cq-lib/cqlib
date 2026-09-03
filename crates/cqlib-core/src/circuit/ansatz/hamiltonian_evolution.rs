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

//! # Hamiltonian-to-Circuit Ansatz
//!
//! This module provides [`PauliEvolutionAnsatz`], a variational ansatz that compiles
//! a [`Hamiltonian`] into a parameterized quantum circuit implementing (or approximating)
//! the time evolution operator $U(t) = e^{-iHt}$, where the evolution time $t$ is a
//! symbolic [`Parameter`].
//!
//! ## Mathematical Background
//!
//! For $H = \sum_k c_k P_k$, the time evolution operator is:
//!
//! $$U(t) = e^{-iHt} = e^{-i \sum_k c_k t P_k}$$
//!
//! ### Exact Evolution (Commuting Terms)
//!
//! If all terms $P_j$ mutually commute ($[P_j, P_k] = 0$ for all $j, k$), then the
//! exponential factors exactly:
//!
//! $$e^{-iHt} = \prod_k e^{-i c_k t P_k}$$
//!
//! Each factor is a single Pauli rotation, directly implementable without error.
//!
//! ### Approximate Evolution (Non-Commuting Terms)
//!
//! When terms do not commute, we use product-formula approximations:
//!
//! **First-order Lie-Trotter** (error $O(t^2/n)$):
//! $$U(t) \approx \left[\prod_k e^{-i c_k (t/n) P_k}\right]^n$$
//!
//! **Second-order Suzuki** (error $O(t^3/n^2)$):
//! $$U(t) \approx \left[\prod_k e^{-i c_k (t/2n) P_k} \cdot
//!   \prod_k^{\leftarrow} e^{-i c_k (t/2n) P_k}\right]^n$$
//!
//! ## Angle Convention
//!
//! The underlying [`PauliEvolution`] trait implements $e^{-i\theta/2 \cdot P}$.
//! To realize $e^{-i c t P}$, the angle must be $\theta = 2 c t$.
//!
//! This convention is identical to the one used in
//! [`QAOAAnsatz`](crate::circuit::ansatz::QAOAAnsatz):
//! ```text
//! let angle = t_param * (2.0 * coeff.re);
//! ```
//!
//! ## Downstream Use Cases
//!
//! - **QAOA cost layers**: variational $e^{-i\gamma H_C}$
//! - **VQE operator ansatz**: parameterized Hamiltonian evolution
//! - **Variational time evolution**: $t$ optimized to match target dynamics
//! - **Chemistry / spin-model simulation**: Trotter decomposition of physical Hamiltonians
//!
//! ## Example
//!
//! ```rust
//! use cqlib_core::circuit::ansatz::{Ansatz, PauliEvolutionAnsatz};
//! use cqlib_core::circuit::ansatz::hamiltonian_evolution::EvolutionStrategy;
//! use cqlib_core::qis::hamiltonian::Hamiltonian;
//!
//! // H = 0.5 ZZ + 0.3 XI  (commuting diagonal terms)
//! let mut h = Hamiltonian::new(2);
//! h.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();
//! h.add_term("ZI".parse().unwrap(), 0.3.into()).unwrap();
//!
//! // Auto-detects exact evolution for commuting Hamiltonian
//! let ansatz = PauliEvolutionAnsatz::new(h).unwrap();
//! assert_eq!(ansatz.num_parameters(), 1);  // only time t
//!
//! let circuit = ansatz.build_circuit("evo").unwrap();
//! println!("circuit {:?}", circuit);
//! // circuit has parameter "evo_t"
//! ```

use super::traits::Ansatz;
use crate::circuit::Parameter;
use crate::circuit::circuit_impl::Circuit;
use crate::circuit::circuit_param::ParameterValue;
use crate::circuit::error::CircuitError;
use crate::qis::error::QisError;
use crate::qis::evolution::{
    PauliEvolution, TrotterMode, trotter_first_order_core, trotter_second_order_core,
};
use crate::qis::hamiltonian::Hamiltonian;
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Strategy for compiling a [`Hamiltonian`] into a parameterized circuit.
///
/// The strategy determines whether the resulting circuit is an exact or approximate
/// implementation of $e^{-iHt}$.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvolutionStrategy {
    /// **Automatically detect** the best strategy:
    ///
    /// - If all Hamiltonian terms mutually commute, uses [`EvolutionStrategy::Exact`].
    /// - Otherwise, applies first-order Trotter with the given number of steps.
    ///
    /// This is the recommended default for most use cases.
    Auto {
        /// Number of Trotter steps to use when the Hamiltonian is non-commuting.
        /// Ignored when all terms commute (exact case).
        steps: usize,
    },

    /// **Exact decomposition** — no Trotter approximation.
    ///
    /// Compiles each Pauli term independently as $e^{-i c_k t P_k}$ in a single pass.
    /// This is valid **only** when all Hamiltonian terms mutually commute.
    ///
    /// Returns [`CircuitError::InvalidOperation`] from [`Ansatz::validate`] if any
    /// two terms fail to commute.
    Exact,

    /// **Trotter-Suzuki approximation** with explicit mode and step count.
    ///
    /// Use this when you need fine-grained control over the approximation quality
    /// and the Trotter mode (first-order, second-order, or randomized).
    Trotter {
        /// The Trotter decomposition algorithm to apply.
        mode: TrotterMode,
        /// Number of Trotter repetitions $n$. Must be $\geq 1$.
        steps: usize,
    },
}

/// Metadata about the compiled evolution returned by [`PauliEvolutionAnsatz::evolution_info`].
///
/// This struct documents whether the circuit is exact or approximate, which is
/// essential for downstream callers that need to reason about approximation quality.
#[derive(Debug, Clone)]
pub struct EvolutionInfo {
    /// Whether the decomposition is mathematically **exact**.
    ///
    /// `true` iff all Hamiltonian terms mutually commute.
    ///
    /// Note: [`EvolutionStrategy::Exact`] on a **non-commuting** Hamiltonian will report
    /// `is_exact = false` here and will also fail [`Ansatz::validate`].
    pub is_exact: bool,

    /// The number of decomposition repetitions actually emitted into the circuit.
    ///
    /// - [`EvolutionStrategy::Exact`]: always `1`.
    /// - [`EvolutionStrategy::Auto`] with a commuting Hamiltonian: `1` because the
    ///   exact single-pass shortcut is taken.
    /// - Explicit [`EvolutionStrategy::Trotter`]: the configured `steps` value, even
    ///   when the Hamiltonian happens to commute and the result is mathematically exact.
    pub steps: usize,

    /// The decomposition mode used to build the circuit.
    ///
    /// `None` means the single-pass exact path was used. `Some(mode)` means an explicit
    /// Trotter-style decomposition was emitted, which may still be mathematically exact
    /// when all Hamiltonian terms commute.
    pub trotter_mode: Option<TrotterMode>,

    /// Whether all Hamiltonian terms mutually commute.
    ///
    /// When `true`, exact evolution is mathematically valid.
    pub all_terms_commute: bool,

    /// Number of Pauli terms in the (simplified) Hamiltonian.
    pub num_terms: usize,
}

/// Ansatz for parameterized Hamiltonian time evolution.
///
/// Compiles $H = \sum_k c_k P_k$ into a parameterized quantum circuit that
/// implements $U(t) = e^{-iHt}$ (or its Trotter approximation), where the
/// evolution time $t$ is a single symbolic [`Parameter`].
///
/// # Requirements
///
/// - The Hamiltonian must be **Hermitian**: after calling `simplify()`, all
///   coefficients must have negligible imaginary part ($|\text{Im}(c_k)| < 10^{-10}$).
/// - The Hamiltonian must be **non-empty**.
/// - For [`EvolutionStrategy::Exact`], all terms must mutually commute.
/// - For `Trotter` / `Auto`, `steps` must be $\geq 1$.
///
/// # Angle Convention
///
/// The underlying [`PauliEvolution::pauli_evolution`] implements $e^{-i\theta/2 \cdot P}$.
/// To realize $e^{-i c t P}$, we pass $\theta = 2 c t$ as the angle. This is the same
/// convention used in [`QAOAAnsatz`](super::qaoa::QAOAAnsatz).
///
/// # Output Circuit
///
/// The built circuit always has exactly **one symbolic parameter**: the evolution time $t$.
/// Its name follows this priority:
///
/// 1. If [`with_time_param_name`](PauliEvolutionAnsatz::with_time_param_name) was called
///    with a non-empty name: use that name exactly.
/// 2. Otherwise: `"{prefix}_t"` where `prefix` is the argument to
///    [`build_circuit`](Ansatz::build_circuit).
///
/// # Example
///
/// ```rust
/// use cqlib_core::circuit::ansatz::{Ansatz, PauliEvolutionAnsatz};
/// use cqlib_core::circuit::ansatz::hamiltonian_evolution::EvolutionStrategy;
/// use cqlib_core::qis::hamiltonian::Hamiltonian;
/// use cqlib_core::qis::evolution::TrotterMode;
///
/// // Non-commuting Hamiltonian: H = X + Z
/// let mut h = Hamiltonian::new(1);
/// h.add_term("X".parse().unwrap(), 1.0.into()).unwrap();
/// h.add_term("Z".parse().unwrap(), 1.0.into()).unwrap();
///
/// let ansatz = PauliEvolutionAnsatz::new(h)
///     .unwrap()
///     .with_strategy(EvolutionStrategy::Trotter {
///         mode: TrotterMode::SecondOrder,
///         steps: 10,
///     })
///     .with_time_param_name("tau");
///
/// let circuit = ansatz.build_circuit("evo").unwrap();
/// // Parameter is named "tau" (explicit override)
/// ```
#[derive(Debug, Clone)]
pub struct PauliEvolutionAnsatz {
    /// The simplified Hamiltonian (normalized, de-duplicated).
    ///
    /// **Invariant**: always in simplified form (phases absorbed, duplicates merged,
    /// near-zero terms removed). Any future mutating API must re-run `simplify()` and
    /// re-validate Hermiticity before updating this field.
    hamiltonian: Hamiltonian,
    /// Cached result of `hamiltonian.all_terms_commute()`, computed once in `new()`.
    ///
    /// Stored to avoid repeated O(m² · n/64) computation across `validate()`,
    /// `evolution_info()`, and `build_circuit()`.
    all_terms_commute: bool,
    /// Optional explicit name for the time parameter.
    ///
    /// `None` → derive name as `"{prefix}_t"` in `build_circuit`.
    /// `Some(name)` → use `name` exactly, ignoring `prefix`.
    time_param_name: Option<String>,
    /// The compilation strategy.
    strategy: EvolutionStrategy,
}

impl PauliEvolutionAnsatz {
    /// Creates a new ansatz from a Hamiltonian.
    ///
    /// Internally calls [`Hamiltonian::simplify`] to normalize the Hamiltonian
    /// (merge duplicate terms, absorb phases into coefficients, remove near-zero terms)
    /// before any further processing.
    ///
    /// The default strategy is [`EvolutionStrategy::Auto`] with `steps = 1`.
    ///
    /// # Errors
    ///
    /// Returns [`QisError::InvalidParameterValue`] if the Hamiltonian is empty after
    /// simplification, or [`QisError::NotHermitian`] if any coefficient has a non-zero
    /// imaginary part after simplification.
    pub fn new(hamiltonian: Hamiltonian) -> Result<Self, QisError> {
        let mut h = hamiltonian;
        h.simplify();

        if h.terms.is_empty() {
            return Err(QisError::InvalidParameterValue(
                "Hamiltonian is empty after simplification; cannot build evolution circuit"
                    .to_string(),
            ));
        }

        // Hermitian check: all coefficients must be real after simplify().
        // simplify() absorbs any PauliString phase into the coefficient, so a
        // term like (0.5) * (-iY) becomes (-0.5i) * Y — which fails this check.
        for (_pauli, coeff) in &h.terms {
            if coeff.im.abs() > 1e-10 {
                return Err(QisError::NotHermitian);
            }
        }

        // Cache commutativity once; reused by validate(), evolution_info(), build_circuit().
        let all_terms_commute = h.all_terms_commute();

        Ok(Self {
            hamiltonian: h,
            all_terms_commute,
            time_param_name: None,
            strategy: EvolutionStrategy::Auto { steps: 1 },
        })
    }

    /// Overrides the name of the time parameter in the built circuit.
    ///
    /// - `Some(name)` or any non-empty string: [`build_circuit`](Ansatz::build_circuit)
    ///   will use that name exactly, ignoring the `prefix` argument for the time parameter.
    ///   Useful when composing multiple ansatze that must share a common time parameter.
    /// - `None` (the default): the time parameter is named `"{prefix}_t"`.
    ///
    /// The name must be a single symbolic identifier and must not shadow the built-in
    /// mathematical constants `e` or `π`.
    ///
    /// Accepts `impl Into<String>` for ergonomic use: `with_time_param_name("tau")`.
    pub fn with_time_param_name(mut self, name: impl Into<String>) -> Self {
        let s = name.into();
        self.time_param_name = if s.is_empty() { None } else { Some(s) };
        self
    }

    /// Sets the compilation strategy.
    ///
    /// See [`EvolutionStrategy`] for available options and their trade-offs.
    pub fn with_strategy(mut self, strategy: EvolutionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Returns metadata about the compiled evolution based on the current configuration.
    ///
    /// This is a **cheap** introspection method — it reads the cached commutativity
    /// result computed at construction time and does not build the circuit.
    ///
    /// ### `EvolutionStrategy::Exact` and non-commuting Hamiltonians
    ///
    /// If the strategy is `Exact` but the Hamiltonian is non-commuting,
    /// `is_exact` will be `false` and `all_terms_commute` will be `false`.
    /// Calling [`Ansatz::build_circuit`] in this configuration will fail at `validate()`.
    pub fn evolution_info(&self) -> EvolutionInfo {
        let all_commute = self.all_terms_commute;

        let (is_exact, steps, trotter_mode) = match &self.strategy {
            EvolutionStrategy::Exact => (all_commute, 1, None),
            EvolutionStrategy::Auto { steps } => {
                if all_commute {
                    (true, 1, None)
                } else {
                    (false, *steps, Some(TrotterMode::FirstOrder))
                }
            }
            EvolutionStrategy::Trotter { mode, steps } => (all_commute, *steps, Some(*mode)),
        };

        EvolutionInfo {
            is_exact,
            steps,
            trotter_mode,
            all_terms_commute: all_commute,
            num_terms: self.hamiltonian.terms.len(),
        }
    }
}

impl Ansatz for PauliEvolutionAnsatz {
    /// Validates the ansatz configuration without building the circuit.
    ///
    /// Checks:
    /// - Hamiltonian is non-empty.
    /// - All coefficients are real (Hermitian requirement).
    /// - [`EvolutionStrategy::Exact`] is only used when all terms commute.
    /// - Trotter step count is $\geq 1$.
    fn validate(&self) -> Result<(), CircuitError> {
        if self.hamiltonian.terms.is_empty() {
            return Err(CircuitError::InvalidOperation(
                "Hamiltonian is empty; cannot build evolution circuit".to_string(),
            ));
        }

        // Hermitian check (redundant with new(), but guards against any future
        // internal mutation that bypasses the constructor invariant).
        for (pauli, coeff) in &self.hamiltonian.terms {
            if coeff.im.abs() > 1e-10 {
                return Err(CircuitError::InvalidOperation(format!(
                    "Hamiltonian is not Hermitian: coefficient of {} has imaginary part {}",
                    pauli, coeff.im
                )));
            }
        }

        // Strategy-specific validation — uses cached commutativity result.
        match &self.strategy {
            EvolutionStrategy::Exact => {
                if !self.all_terms_commute {
                    return Err(CircuitError::InvalidOperation(
                        "EvolutionStrategy::Exact requires all Hamiltonian terms to \
                         mutually commute. Use EvolutionStrategy::Auto or \
                         EvolutionStrategy::Trotter for non-commuting Hamiltonians."
                            .to_string(),
                    ));
                }
            }
            EvolutionStrategy::Auto { steps } => {
                if *steps == 0 {
                    return Err(CircuitError::InvalidOperation(
                        "EvolutionStrategy::Auto requires steps >= 1".to_string(),
                    ));
                }
            }
            EvolutionStrategy::Trotter { steps, .. } => {
                if *steps == 0 {
                    return Err(CircuitError::InvalidOperation(
                        "EvolutionStrategy::Trotter requires steps >= 1".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Builds the parameterized time evolution circuit.
    ///
    /// The resulting circuit has exactly one symbolic parameter: the evolution time $t$.
    ///
    /// ## Parameter Naming
    ///
    /// - If [`with_time_param_name`](PauliEvolutionAnsatz::with_time_param_name) was called
    ///   with a non-empty name, that name is used exactly.
    /// - Otherwise the parameter is named `"{prefix}_t"`.
    ///
    /// ## Angle Convention
    ///
    /// Each Pauli term $c_k P_k$ contributes a rotation angle $\theta_k = 2 c_k t$
    /// (or $\theta_k = 2 c_k t / n$ per Trotter step). This realizes $e^{-i c_k t P_k}$
    /// via the underlying `pauli_evolution` call which implements $e^{-i \theta/2 P}$.
    ///
    /// ## Approximation Labeling
    ///
    /// The docstring of the returned circuit is not annotated internally, but callers can
    /// use [`evolution_info`](PauliEvolutionAnsatz::evolution_info) to query whether the
    /// circuit is exact or approximate.
    fn build_circuit(&self, prefix: &str) -> Result<Circuit, CircuitError> {
        self.validate()?;

        // Resolve time parameter name: explicit override or prefix-derived.
        let param_name = match &self.time_param_name {
            Some(name) => name.clone(),
            None => format!("{}_t", prefix),
        };

        validate_time_parameter_name(&param_name)?;
        let t = Parameter::try_from(param_name.as_str()).map_err(|_| {
            CircuitError::InvalidOperation(format!(
                "Invalid time parameter name '{param_name}': expected a single symbolic identifier"
            ))
        })?;

        let mut circuit = Circuit::new(self.hamiltonian.num_qubits);
        let qubits = circuit.qubits();

        // Use cached commutativity — not re-computed here.
        let all_commute = self.all_terms_commute;

        match &self.strategy {
            EvolutionStrategy::Exact => {
                // Exact single-pass; validate() already ensures all_commute == true
                apply_exact_evolution(&mut circuit, &self.hamiltonian, &t, &qubits)?;
            }

            EvolutionStrategy::Auto { steps } => {
                if all_commute {
                    // Exact path: no approximation error
                    apply_exact_evolution(&mut circuit, &self.hamiltonian, &t, &qubits)?;
                } else {
                    // Fall back to first-order Trotter via shared core helper
                    trotter_first_order_core(
                        &mut circuit,
                        &self.hamiltonian.terms,
                        ParameterValue::Param(t),
                        *steps,
                        &qubits,
                        None,
                    )?;
                }
            }

            EvolutionStrategy::Trotter { mode, steps } => {
                let n = *steps;
                match mode {
                    TrotterMode::FirstOrder => {
                        trotter_first_order_core(
                            &mut circuit,
                            &self.hamiltonian.terms,
                            ParameterValue::Param(t),
                            n,
                            &qubits,
                            None,
                        )?;
                    }
                    TrotterMode::SecondOrder => {
                        trotter_second_order_core(
                            &mut circuit,
                            &self.hamiltonian.terms,
                            ParameterValue::Param(t),
                            n,
                            &qubits,
                        )?;
                    }
                    TrotterMode::Randomized(seed) => {
                        let mut rng = StdRng::seed_from_u64(*seed);
                        trotter_first_order_core(
                            &mut circuit,
                            &self.hamiltonian.terms,
                            ParameterValue::Param(t),
                            n,
                            &qubits,
                            Some(&mut rng),
                        )?;
                    }
                }
            }
        }

        Ok(circuit)
    }

    /// Returns `1`: the single evolution time parameter $t$.
    fn num_parameters(&self) -> usize {
        1
    }

    fn num_qubits(&self) -> usize {
        self.hamiltonian.num_qubits
    }
}

fn validate_time_parameter_name(name: &str) -> Result<(), CircuitError> {
    if matches!(name, "e" | "π") {
        return Err(CircuitError::InvalidOperation(format!(
            "Invalid time parameter name '{name}': reserved mathematical constants cannot be used"
        )));
    }

    let parsed = Parameter::try_from(name).map_err(|_| {
        CircuitError::InvalidOperation(format!(
            "Invalid time parameter name '{name}': expected a single symbolic identifier"
        ))
    })?;
    let symbols = parsed.get_symbols();
    if symbols.len() != 1 || !symbols.contains(name) {
        return Err(CircuitError::InvalidOperation(format!(
            "Invalid time parameter name '{name}': expected a single symbolic identifier"
        )));
    }

    Ok(())
}

// The Trotter step helpers (first-order, second-order, randomized) are now the
// shared `pub(crate)` functions `trotter_first_order_core` and
// `trotter_second_order_core` defined in `qis/evolution.rs`. Both the numeric
// API (`to_trotter_circuit` / `to_evolution_circuit`) and the parametric ansatz
// call those same functions, eliminating duplicate implementations.
//
// Only the *exact* evolution helper remains here because it is specific to the
// parametric (symbolic) path and has no numeric counterpart at this layer.

/// Applies a single-pass exact evolution: ∏_k e^{-i c_k t P_k}.
///
/// Valid only when all Hamiltonian terms mutually commute.
///
/// **Identity terms** (`P_k = I⊗…⊗I`) contribute only a global phase `e^{-i c_k t}`.
/// The underlying `pauli_evolution` handles all-identity strings correctly; no
/// special skip logic is needed here. The global phase is not observable and is
/// consistent across all evolution strategies.
fn apply_exact_evolution(
    circuit: &mut Circuit,
    hamiltonian: &Hamiltonian,
    t: &Parameter,
    qubits: &[crate::circuit::bit::Qubit],
) -> Result<(), CircuitError> {
    for (pauli, coeff) in &hamiltonian.terms {
        // Target: e^{-i c t P}
        // pauli_evolution implements e^{-i θ/2 P}, so θ = 2 * c * t
        let angle = t.clone() * (2.0 * coeff.re);
        circuit.pauli_evolution(pauli, angle, qubits)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "hamiltonian_evolution_test.rs"]
mod hamiltonian_evolution_test;
