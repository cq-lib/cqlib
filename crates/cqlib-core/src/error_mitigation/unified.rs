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

use crate::circuit::{Circuit, Instruction};
use crate::qis::Hamiltonian;

use super::{
    ErrorMitigationError, Estimator, ExtrapolateMethod, VirtualDistillation, ZNEMitigation,
};

/// Method-specific configuration for zero-noise extrapolation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZneConfig {
    pub fold_levels: Vec<i32>,
}

/// Method-specific configuration for virtual distillation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualDistillationConfig {
    pub copies: usize,
}

/// Supported error-mitigation methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MitigationMethod {
    Zne(ZneConfig),
    VirtualDistillation(VirtualDistillationConfig),
}

/// Runtime arguments required to execute one mitigation pipeline.
#[derive(Debug, Clone)]
pub enum RunArgs {
    Zne {
        gate_set: Option<Vec<Instruction>>,
        shots: Option<usize>,
    },
    VirtualDistillation {
        shots_numerator: usize,
        shots_denominator: usize,
    },
}

impl PartialEq for RunArgs {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Zne {
                    gate_set: lhs_gate_set,
                    shots: lhs_shots,
                },
                Self::Zne {
                    gate_set: rhs_gate_set,
                    shots: rhs_shots,
                },
            ) => {
                lhs_shots == rhs_shots
                    && match (lhs_gate_set, rhs_gate_set) {
                        (None, None) => true,
                        (Some(lhs), Some(rhs)) => {
                            lhs.len() == rhs.len()
                                && lhs
                                    .iter()
                                    .zip(rhs.iter())
                                    .all(|(lhs, rhs)| lhs.name() == rhs.name())
                        }
                        _ => false,
                    }
            }
            (
                Self::VirtualDistillation {
                    shots_numerator: lhs_numerator,
                    shots_denominator: lhs_denominator,
                },
                Self::VirtualDistillation {
                    shots_numerator: rhs_numerator,
                    shots_denominator: rhs_denominator,
                },
            ) => lhs_numerator == rhs_numerator && lhs_denominator == rhs_denominator,
            _ => false,
        }
    }
}

impl Eq for RunArgs {}

/// Processing arguments required to produce the final mitigated result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessArgs {
    Zne {
        method: ExtrapolateMethod,
        degree: Option<usize>,
    },
    VirtualDistillation,
}

/// Final mitigated observable estimate.
#[derive(Debug, Clone, PartialEq)]
pub struct MitigatedResult {
    pub expectation: f64,
    pub variance: Option<f64>,
}

#[derive(Debug, Clone)]
enum ErrorMitigationState {
    Initialized,
    RunCompleted(Box<RunRecord>),
    Mitigated(MitigatedResult),
}

#[derive(Debug, Clone)]
enum RunRecord {
    Zne(ZneRunRecord),
    VirtualDistillation(Box<VirtualDistillationRunRecord>),
}

#[derive(Debug, Clone)]
struct ZneRunRecord {
    folded_circuits: Vec<Circuit>,
    noisy_expectations: Vec<f64>,
    noise_factors: Vec<i32>,
    gate_set: Option<Vec<Instruction>>,
    shots: Option<usize>,
}

#[derive(Debug, Clone)]
struct VirtualDistillationRunRecord {
    copy_swap_circuit: Circuit,
    numerator: (f64, f64),
    denominator: (f64, f64),
    shots_numerator: usize,
    shots_denominator: usize,
}

/// Unified facade over supported error-mitigation methods.
///
/// The workflow is always sequential:
/// 1. create an [`ErrorMitigation`] instance with one mitigation method,
/// 2. call [`ErrorMitigation::run`] to construct and execute the required circuits,
/// 3. call [`ErrorMitigation::get_mitigated`] to post-process the stored raw data.
///
/// # Example
///
/// Zero-noise extrapolation:
///
/// ```rust
/// use cqlib_core::circuit::{Circuit, Qubit};
/// use cqlib_core::error_mitigation::{
///     ErrorMitigation, ExtrapolateMethod, MitigationMethod, ProcessArgs, RunArgs, ZneConfig,
/// };
/// use cqlib_core::qis::{Hamiltonian, Pauli, PauliString};
/// use num_complex::Complex64;
///
/// let q0 = Qubit::new(0);
/// let mut circuit = Circuit::new(1);
/// circuit.h(q0).unwrap();
///
/// let mut pauli = PauliString::new(1);
/// pauli.set_pauli(0, Pauli::Z);
/// let hamiltonian = Hamiltonian::from_list(vec![(pauli, Complex64::new(1.0, 0.0))]).unwrap();
///
/// let mut mitigation = ErrorMitigation::new(
///     circuit,
///     MitigationMethod::Zne(ZneConfig {
///         fold_levels: vec![0, 1, 2],
///     }),
/// )
/// .unwrap();
///
/// // A portal to real quantum device or classical simulation backend that calculates <h> and its variance
/// let estimator = |circuit: &Circuit, hamiltonian: Option<&Hamiltonian>, shots: Option<usize>| {
///     assert!(hamiltonian.is_some());
///     assert_eq!(shots, Some(1024));
///     (circuit.operations().len() as f64, 0.0)
/// };
///
/// mitigation
///     .run(
///         &hamiltonian,
///         RunArgs::Zne {
///             gate_set: None,
///             shots: Some(1024),
///         },
///         &estimator,
///     )
///     .unwrap();
///
/// let mitigated = mitigation
///     .get_mitigated(ProcessArgs::Zne {
///         method: ExtrapolateMethod::Polynomial,
///         degree: Some(1),
///     })
///     .unwrap();
///
/// assert!(mitigated.variance.is_none());
/// ```
///
/// Virtual distillation:
///
/// ```rust
/// use cqlib_core::circuit::Circuit;
/// use cqlib_core::error_mitigation::{
///     ErrorMitigation, MitigationMethod, ProcessArgs, RunArgs, VirtualDistillationConfig,
/// };
/// use cqlib_core::qis::{Hamiltonian, Pauli, PauliString};
/// use num_complex::Complex64;
///
/// let circuit = Circuit::new(1);
///
/// let mut pauli = PauliString::new(1);
/// pauli.set_pauli(0, Pauli::Z);
/// let hamiltonian = Hamiltonian::from_list(vec![(pauli, Complex64::new(1.0, 0.0))]).unwrap();
///
/// let mut mitigation = ErrorMitigation::new(
///     circuit,
///     MitigationMethod::VirtualDistillation(VirtualDistillationConfig { copies: 2 }),
/// )
/// .unwrap();
///
/// // A portal to real quantum device or classical simulation backend that calculates <h> and its variance
/// let estimator = |_circuit: &Circuit, hamiltonian: Option<&Hamiltonian>, _shots: Option<usize>| {
///     if hamiltonian.is_some() {
///         (1.5, 0.25)
///     } else {
///         (2.0, 1.0)
///     }
/// };
///
/// mitigation
///     .run(
///         &hamiltonian,
///         RunArgs::VirtualDistillation {
///             shots_numerator: 512,
///             shots_denominator: 512,
///         },
///         &estimator,
///     )
///     .unwrap();
///
/// let mitigated = mitigation
///     .get_mitigated(ProcessArgs::VirtualDistillation)
///     .unwrap();
///
/// assert!(mitigated.variance.is_some());
/// ```
#[derive(Debug, Clone)]
pub struct ErrorMitigation {
    circuit: Circuit,
    method: MitigationMethod,
    state: ErrorMitigationState,
}

impl ErrorMitigation {
    /// Creates a new unified mitigation pipeline for the given circuit and method.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cqlib_core::circuit::Circuit;
    /// use cqlib_core::error_mitigation::{
    ///     ErrorMitigation, MitigationMethod, VirtualDistillationConfig,
    /// };
    ///
    /// let mitigation = ErrorMitigation::new(
    ///     Circuit::new(1),
    ///     MitigationMethod::VirtualDistillation(VirtualDistillationConfig { copies: 2 }),
    /// );
    ///
    /// assert!(mitigation.is_ok());
    /// ```
    pub fn new(circuit: Circuit, method: MitigationMethod) -> Result<Self, ErrorMitigationError> {
        Self::validate_method(&method)?;
        Ok(Self {
            circuit,
            method,
            state: ErrorMitigationState::Initialized,
        })
    }

    /// Executes the configured mitigation pipeline and stores raw results.
    ///
    /// This method does not return the final mitigated value directly. Call
    /// [`ErrorMitigation::get_mitigated`] afterwards to perform the method-specific
    /// post-processing step.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cqlib_core::circuit::Circuit;
    /// use cqlib_core::error_mitigation::{
    ///     ErrorMitigation, MitigationMethod, RunArgs, VirtualDistillationConfig,
    /// };
    /// use cqlib_core::qis::{Hamiltonian, Pauli, PauliString};
    /// use num_complex::Complex64;
    ///
    /// let mut pauli = PauliString::new(1);
    /// pauli.set_pauli(0, Pauli::Z);
    /// let hamiltonian = Hamiltonian::from_list(vec![(pauli, Complex64::new(1.0, 0.0))]).unwrap();
    ///
    /// let mut mitigation = ErrorMitigation::new(
    ///     Circuit::new(1),
    ///     MitigationMethod::VirtualDistillation(VirtualDistillationConfig { copies: 2 }),
    /// )
    /// .unwrap();
    ///
    /// // A portal to real quantum device or classical simulation backend that calculates <h> and its variance
    /// let estimator = |_circuit: &Circuit, hamiltonian: Option<&Hamiltonian>, _shots: Option<usize>| {
    ///     if hamiltonian.is_some() {
    ///         (1.0, 0.1)
    ///     } else {
    ///         (2.0, 0.2)
    ///     }
    /// };
    ///
    /// mitigation
    ///     .run(
    ///         &hamiltonian,
    ///         RunArgs::VirtualDistillation {
    ///             shots_numerator: 256,
    ///             shots_denominator: 256,
    ///         },
    ///         &estimator,
    ///     )
    ///     .unwrap();
    /// ```
    pub fn run(
        &mut self,
        hamiltonian: &Hamiltonian,
        run_args: RunArgs,
        estimator: &Estimator<'_>,
    ) -> Result<(), ErrorMitigationError> {
        match self.state {
            ErrorMitigationState::Initialized => {}
            ErrorMitigationState::RunCompleted(_) => return Err(ErrorMitigationError::AlreadyRun),
            ErrorMitigationState::Mitigated(_) => {
                return Err(ErrorMitigationError::AlreadyMitigated);
            }
        }

        let run_record = match (&self.method, run_args) {
            (MitigationMethod::Zne(config), RunArgs::Zne { gate_set, shots }) => {
                RunRecord::Zne(self.run_zne(config, hamiltonian, gate_set, shots, estimator)?)
            }
            (
                MitigationMethod::VirtualDistillation(config),
                RunArgs::VirtualDistillation {
                    shots_numerator,
                    shots_denominator,
                },
            ) => RunRecord::VirtualDistillation(Box::new(self.run_virtual_distillation(
                config,
                hamiltonian,
                shots_numerator,
                shots_denominator,
                estimator,
            )?)),
            (MitigationMethod::Zne(_), _) | (MitigationMethod::VirtualDistillation(_), _) => {
                return Err(ErrorMitigationError::RunArgsMethodMismatch);
            }
        };

        self.state = ErrorMitigationState::RunCompleted(Box::new(run_record));
        Ok(())
    }

    /// Produces the final mitigated result from the stored raw execution outputs.
    ///
    /// `run()` must be called first. The provided [`ProcessArgs`] must match the
    /// configured mitigation method.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cqlib_core::circuit::Circuit;
    /// use cqlib_core::error_mitigation::{
    ///     ErrorMitigation, ExtrapolateMethod, MitigationMethod, ProcessArgs, RunArgs, ZneConfig,
    /// };
    /// use cqlib_core::qis::{Hamiltonian, Pauli, PauliString};
    /// use num_complex::Complex64;
    ///
    /// let mut pauli = PauliString::new(1);
    /// pauli.set_pauli(0, Pauli::Z);
    /// let hamiltonian = Hamiltonian::from_list(vec![(pauli, Complex64::new(1.0, 0.0))]).unwrap();
    ///
    /// let mut mitigation = ErrorMitigation::new(
    ///     Circuit::new(1),
    ///     MitigationMethod::Zne(ZneConfig {
    ///         fold_levels: vec![0, 1, 2],
    ///     }),
    /// )
    /// .unwrap();
    ///
    /// // A portal to real quantum device or classical simulation backend that calculates <h> and its variance
    /// let estimator = |circuit: &Circuit, _hamiltonian: Option<&Hamiltonian>, _shots: Option<usize>| {
    ///     (circuit.operations().len() as f64, 0.0)
    /// };
    ///
    /// mitigation
    ///     .run(
    ///         &hamiltonian,
    ///         RunArgs::Zne {
    ///             gate_set: None,
    ///             shots: None,
    ///         },
    ///         &estimator,
    ///     )
    ///     .unwrap();
    ///
    /// let mitigated = mitigation
    ///     .get_mitigated(ProcessArgs::Zne {
    ///         method: ExtrapolateMethod::Polynomial,
    ///         degree: Some(1),
    ///     })
    ///     .unwrap();
    ///
    /// assert!(mitigated.expectation.is_finite());
    /// ```
    pub fn get_mitigated(
        &mut self,
        process_args: ProcessArgs,
    ) -> Result<MitigatedResult, ErrorMitigationError> {
        let mitigated = match (&self.method, &self.state, process_args) {
            (MitigationMethod::Zne(_), ErrorMitigationState::Initialized, _) => {
                return Err(ErrorMitigationError::RunRequiredBeforeMitigation);
            }
            (MitigationMethod::VirtualDistillation(_), ErrorMitigationState::Initialized, _) => {
                return Err(ErrorMitigationError::RunRequiredBeforeMitigation);
            }
            (_, ErrorMitigationState::Mitigated(cached), _) => {
                let _ = cached;
                return Err(ErrorMitigationError::AlreadyMitigated);
            }
            (
                MitigationMethod::Zne(config),
                ErrorMitigationState::RunCompleted(record),
                ProcessArgs::Zne { method, degree },
            ) => match record.as_ref() {
                RunRecord::Zne(record) => self.get_zne_mitigated(config, record, method, degree)?,
                RunRecord::VirtualDistillation(_) => {
                    return Err(ErrorMitigationError::ProcessArgsMethodMismatch);
                }
            },
            (
                MitigationMethod::VirtualDistillation(_),
                ErrorMitigationState::RunCompleted(record),
                ProcessArgs::VirtualDistillation,
            ) => match record.as_ref() {
                RunRecord::VirtualDistillation(record) => {
                    self.get_virtual_distillation_mitigated(record.as_ref())?
                }
                RunRecord::Zne(_) => {
                    return Err(ErrorMitigationError::ProcessArgsMethodMismatch);
                }
            },
            (MitigationMethod::Zne(_), ErrorMitigationState::RunCompleted(_), _) => {
                return Err(ErrorMitigationError::ProcessArgsMethodMismatch);
            }
            (
                MitigationMethod::VirtualDistillation(_),
                ErrorMitigationState::RunCompleted(_),
                _,
            ) => {
                return Err(ErrorMitigationError::ProcessArgsMethodMismatch);
            }
        };

        self.state = ErrorMitigationState::Mitigated(mitigated.clone());
        Ok(mitigated)
    }

    fn validate_method(method: &MitigationMethod) -> Result<(), ErrorMitigationError> {
        match method {
            MitigationMethod::Zne(config) => {
                for &level in &config.fold_levels {
                    if level < 0 {
                        return Err(ErrorMitigationError::InvalidFoldLevel(level));
                    }
                }
                Ok(())
            }
            MitigationMethod::VirtualDistillation(config) => {
                if config.copies < 2 {
                    return Err(ErrorMitigationError::InvalidCopies(config.copies));
                }
                Ok(())
            }
        }
    }

    fn run_zne(
        &self,
        config: &ZneConfig,
        hamiltonian: &Hamiltonian,
        gate_set: Option<Vec<Instruction>>,
        shots: Option<usize>,
        estimator: &Estimator<'_>,
    ) -> Result<ZneRunRecord, ErrorMitigationError> {
        let zne = ZNEMitigation::new(self.circuit.clone(), config.fold_levels.clone());
        if hamiltonian.num_qubits != self.circuit.width() {
            return Err(ErrorMitigationError::HamiltonianQubitCountMismatch {
                expected: self.circuit.width(),
                actual: hamiltonian.num_qubits,
            });
        }

        let folded_circuits = zne.fold_circuits(gate_set.as_deref())?;
        let noisy_expectations = folded_circuits
            .iter()
            .map(|circuit| estimator(circuit, Some(hamiltonian), shots).0)
            .collect();

        Ok(ZneRunRecord {
            folded_circuits,
            noisy_expectations,
            noise_factors: zne.noise_factors().to_vec(),
            gate_set,
            shots,
        })
    }

    fn run_virtual_distillation(
        &self,
        config: &VirtualDistillationConfig,
        hamiltonian: &Hamiltonian,
        shots_numerator: usize,
        shots_denominator: usize,
        estimator: &Estimator<'_>,
    ) -> Result<VirtualDistillationRunRecord, ErrorMitigationError> {
        let vd = VirtualDistillation::new(self.circuit.clone(), config.copies)?;
        if hamiltonian.num_qubits != self.circuit.width() {
            return Err(ErrorMitigationError::HamiltonianQubitCountMismatch {
                expected: self.circuit.width(),
                actual: hamiltonian.num_qubits,
            });
        }

        let copy_swap_circuit = vd.build_copy_swap_circuit()?;
        let extra_qubits = copy_swap_circuit.width() - hamiltonian.num_qubits;
        let expanded_hamiltonian =
            VirtualDistillation::expand_hamiltonian(hamiltonian, extra_qubits)?;
        let numerator = estimator(
            &copy_swap_circuit,
            Some(&expanded_hamiltonian),
            Some(shots_numerator),
        );
        let denominator = estimator(&copy_swap_circuit, None, Some(shots_denominator));

        Ok(VirtualDistillationRunRecord {
            copy_swap_circuit,
            numerator,
            denominator,
            shots_numerator,
            shots_denominator,
        })
    }

    fn get_zne_mitigated(
        &self,
        config: &ZneConfig,
        record: &ZneRunRecord,
        method: ExtrapolateMethod,
        degree: Option<usize>,
    ) -> Result<MitigatedResult, ErrorMitigationError> {
        let zne = ZNEMitigation::new(self.circuit.clone(), config.fold_levels.clone());

        debug_assert_eq!(
            record.folded_circuits.len(),
            record.noisy_expectations.len()
        );
        debug_assert_eq!(record.noise_factors, zne.noise_factors().to_vec());
        debug_assert_eq!(record.noise_factors.len(), record.noisy_expectations.len());
        let _ = (&record.gate_set, record.shots);
        let degree = match method {
            ExtrapolateMethod::Polynomial => {
                degree.unwrap_or_else(|| record.noisy_expectations.len().saturating_sub(1).min(1))
            }
            ExtrapolateMethod::Exponential => degree.unwrap_or(0),
        };
        let expectation = zne.extrapolate(&record.noisy_expectations, method, degree)?;

        Ok(MitigatedResult {
            expectation,
            variance: None,
        })
    }

    fn get_virtual_distillation_mitigated(
        &self,
        record: &VirtualDistillationRunRecord,
    ) -> Result<MitigatedResult, ErrorMitigationError> {
        debug_assert!(record.copy_swap_circuit.width() >= self.circuit.width());
        let _ = (record.shots_numerator, record.shots_denominator);
        let (expectation, variance) = VirtualDistillation::mitigate_from_statistics(
            record.numerator.0,
            record.numerator.1,
            record.denominator.0,
            record.denominator.1,
        )?;

        Ok(MitigatedResult {
            expectation,
            variance: Some(variance),
        })
    }
}
