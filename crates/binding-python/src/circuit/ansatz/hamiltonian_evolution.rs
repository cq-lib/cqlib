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

//! Python bindings for the Hamiltonian-to-Circuit ansatz.
//!
//! Exposes [`PyEvolutionStrategy`], [`PyEvolutionInfo`], and
//! [`PyPauliEvolutionAnsatz`] to Python as `cqlib.circuit.ansatz`.

use cqlib_core::circuit::ansatz::hamiltonian_evolution::{
    EvolutionInfo, EvolutionStrategy, PauliEvolutionAnsatz,
};
use cqlib_core::circuit::ansatz::traits::Ansatz;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::circuit::circuit_impl::PyCircuit;
use crate::qis::evolution::PyTrotterMode;
use crate::qis::hamiltonian::PyHamiltonian;

/// Controls how a Hamiltonian is compiled into a quantum circuit.
///
/// An `EvolutionStrategy` is created via one of the three static factory methods:
///
/// - :meth:`exact` — exact single-pass evolution; only valid when all
///   Hamiltonian terms mutually commute.
/// - :meth:`auto` — automatically selects exact or first-order Trotter.
/// - :meth:`trotter` — explicit Trotter-Suzuki decomposition with a chosen mode
///   and number of steps.
///
/// Examples:
///     >>> from cqlib.circuit.ansatz import EvolutionStrategy
///     >>> from cqlib.qis import TrotterMode
///     >>> s1 = EvolutionStrategy.exact()
///     >>> s2 = EvolutionStrategy.auto(steps=10)
///     >>> s3 = EvolutionStrategy.trotter(TrotterMode.second_order(), steps=5)
#[pyclass(
    name = "EvolutionStrategy",
    module = "cqlib.circuit.ansatz",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyEvolutionStrategy {
    pub(crate) inner: EvolutionStrategy,
}

impl From<EvolutionStrategy> for PyEvolutionStrategy {
    fn from(inner: EvolutionStrategy) -> Self {
        Self { inner }
    }
}

impl From<PyEvolutionStrategy> for EvolutionStrategy {
    fn from(value: PyEvolutionStrategy) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyEvolutionStrategy {
    /// Returns the ``Exact`` strategy.
    ///
    /// Compiles the Hamiltonian as a single product of Pauli rotations:
    ///
    /// ```text
    /// U(t) = \\prod_k e^{-i c_k t P_k}
    /// ```
    ///
    /// This is **mathematically exact** when all Hamiltonian terms mutually commute.
    /// If any two terms do not commute, :meth:`~PauliEvolutionAnsatz.build_circuit`
    /// will raise ``ValueError``.
    ///
    /// Returns:
    ///     EvolutionStrategy: The exact strategy.
    ///
    /// Raises:
    ///     ValueError: At circuit-build time if any two Hamiltonian terms do not commute.
    #[staticmethod]
    fn exact() -> Self {
        Self {
            inner: EvolutionStrategy::Exact,
        }
    }

    /// Returns the ``Auto`` strategy with a given number of Trotter steps.
    ///
    /// Automatically selects the best method:
    ///
    /// - If **all terms commute**: uses exact single-pass evolution
    ///   (``steps`` is ignored).
    /// - Otherwise: uses first-order Lie-Trotter decomposition with ``steps``
    ///   repetitions (error :math:`O(t^2/n)`).
    ///
    /// Args:
    ///     steps: Number of Trotter steps :math:`n \\geq 1`. Ignored for commuting
    ///         Hamiltonians. Defaults to ``1``.
    ///
    /// Returns:
    ///     EvolutionStrategy: The auto strategy.
    ///
    /// Raises:
    ///     ValueError: At circuit-build time if ``steps < 1``.
    #[staticmethod]
    #[pyo3(signature = (steps = 1))]
    fn auto(steps: usize) -> Self {
        Self {
            inner: EvolutionStrategy::Auto { steps },
        }
    }

    /// Returns an explicit ``Trotter`` strategy.
    ///
    /// Applies the chosen product-formula approximation regardless of whether
    /// the Hamiltonian terms commute.
    ///
    /// Available modes:
    ///
    /// - :meth:`~cqlib.qis.TrotterMode.first_order` — error :math:`O(t^2/n)`
    /// - :meth:`~cqlib.qis.TrotterMode.second_order` — error :math:`O(t^3/n^2)`
    /// - :meth:`~cqlib.qis.TrotterMode.randomized` — randomized term ordering per step
    ///
    /// Args:
    ///     mode: The Trotter decomposition mode.
    ///     steps: Number of Trotter repetitions :math:`n \\geq 1`.
    ///
    /// Returns:
    ///     EvolutionStrategy: The explicit Trotter strategy.
    ///
    /// Raises:
    ///     ValueError: At circuit-build time if ``steps < 1``.
    #[staticmethod]
    fn trotter(mode: PyRef<'_, PyTrotterMode>, steps: usize) -> Self {
        Self {
            inner: EvolutionStrategy::Trotter {
                mode: mode.inner,
                steps,
            },
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            EvolutionStrategy::Exact => "EvolutionStrategy.exact()".to_string(),
            EvolutionStrategy::Auto { steps } => {
                format!("EvolutionStrategy.auto(steps={})", steps)
            }
            EvolutionStrategy::Trotter { mode, steps } => {
                let mode_str = match mode {
                    cqlib_core::qis::evolution::TrotterMode::FirstOrder => {
                        "TrotterMode.first_order()"
                    }
                    cqlib_core::qis::evolution::TrotterMode::SecondOrder => {
                        "TrotterMode.second_order()"
                    }
                    cqlib_core::qis::evolution::TrotterMode::Randomized(_) => {
                        "TrotterMode.randomized(...)"
                    }
                };
                format!("EvolutionStrategy.trotter({}, steps={})", mode_str, steps)
            }
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __eq__(&self, other: &PyEvolutionStrategy) -> bool {
        self.inner == other.inner
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Read-only metadata describing how a :class:`PauliEvolutionAnsatz` compiles.
///
/// Returned by :meth:`PauliEvolutionAnsatz.evolution_info`. This is a **cheap**
/// introspection call — it does not build the circuit.
///
/// Attributes:
///     is_exact (bool): ``True`` iff the decomposition is mathematically exact.
///         This holds whenever all Hamiltonian terms mutually commute and the
///         selected strategy emits a mathematically exact decomposition.
///         Explicit :meth:`EvolutionStrategy.trotter` can therefore still report
///         ``True`` here when applied to a commuting Hamiltonian.
///     steps (int): Effective number of decomposition repetitions emitted into the
///         circuit. This is ``1`` for single-pass exact evolution, but remains the
///         configured value for explicit :meth:`EvolutionStrategy.trotter`, even
///         when the result is mathematically exact because all terms commute.
///     trotter_mode (TrotterMode | None): The Trotter mode in use, or ``None`` only
///         when the single-pass exact path is selected.
///     all_terms_commute (bool): ``True`` iff all Hamiltonian terms mutually commute.
///     num_terms (int): Number of Pauli terms in the (simplified) Hamiltonian.
///
/// Note:
///     If the strategy is :meth:`EvolutionStrategy.exact` but the Hamiltonian is
///     non-commuting, ``is_exact`` will be ``False`` and
///     :meth:`~PauliEvolutionAnsatz.build_circuit` will fail.
///
/// Examples:
///     >>> info = ansatz.evolution_info()
///     >>> if info.is_exact:
///     ...     print("Mathematically exact decomposition")
///     ... else:
///     ...     print(f"Trotter with {info.steps} steps, mode={info.trotter_mode}")
#[pyclass(
    name = "EvolutionInfo",
    module = "cqlib.circuit.ansatz",
    get_all,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyEvolutionInfo {
    /// ``True`` iff the decomposition is mathematically exact.
    pub is_exact: bool,
    /// Effective number of emitted decomposition repetitions.
    pub steps: usize,
    /// The Trotter mode in use, or ``None`` for the single-pass exact path.
    pub trotter_mode: Option<PyTrotterMode>,
    /// ``True`` iff all Hamiltonian terms mutually commute.
    pub all_terms_commute: bool,
    /// Number of Pauli terms in the (simplified) Hamiltonian.
    pub num_terms: usize,
}

impl From<EvolutionInfo> for PyEvolutionInfo {
    fn from(info: EvolutionInfo) -> Self {
        Self {
            is_exact: info.is_exact,
            steps: info.steps,
            trotter_mode: info.trotter_mode.map(PyTrotterMode::from),
            all_terms_commute: info.all_terms_commute,
            num_terms: info.num_terms,
        }
    }
}

#[pymethods]
impl PyEvolutionInfo {
    fn __repr__(&self) -> String {
        let mode_str = match &self.trotter_mode {
            None => "None".to_string(),
            Some(m) => match m.inner {
                cqlib_core::qis::evolution::TrotterMode::FirstOrder => {
                    "TrotterMode.FirstOrder".to_string()
                }
                cqlib_core::qis::evolution::TrotterMode::SecondOrder => {
                    "TrotterMode.SecondOrder".to_string()
                }
                cqlib_core::qis::evolution::TrotterMode::Randomized(seed) => {
                    format!("TrotterMode.Randomized(seed={})", seed)
                }
            },
        };
        format!(
            "EvolutionInfo(is_exact={}, steps={}, trotter_mode={}, \
             all_terms_commute={}, num_terms={})",
            self.is_exact, self.steps, mode_str, self.all_terms_commute, self.num_terms,
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Compiles a Hamiltonian into a parameterized time-evolution circuit.
///
/// Implements (or approximates) the unitary:
///
/// ```text
/// U(t) = e^{-iHt}, \\quad H = \\sum_k c_k P_k
/// ```
///
/// where `t` is a single symbolic `cqlib.circuit.Parameter`.
///
/// **Exact evolution** is possible when all terms commute:
///
/// ```text
/// e^{-iHt} = \\prod_k e^{-i c_k t P_k}
/// ```
///
/// **Approximate evolution** uses product-formula methods when terms do not commute.
///
/// # Angle Convention
///
/// The underlying Pauli evolution gate implements
/// `e^{-i\\theta/2 \\cdot P}`. To realize `e^{-i c t P}`, the angle
/// passed is `\\theta = 2ct`. This is the same convention as
/// `cqlib.qis.Hamiltonian.to_trotter_circuit` and
/// `cqlib.qis.Hamiltonian.to_evolution_circuit`.
///
/// # Builder Methods
///
/// Builder methods return a **new** ``PauliEvolutionAnsatz`` (immutable builder pattern).
///
/// Examples:
/// ```python
/// >>> from cqlib.circuit.ansatz import PauliEvolutionAnsatz, EvolutionStrategy
/// >>> from cqlib.qis import Hamiltonian, PauliString, TrotterMode
/// >>> h = Hamiltonian(2)
/// >>> h.add_term(PauliString.from_str("ZZ"), 0.5)
/// >>> h.add_term(PauliString.from_str("ZI"), 0.3)
/// >>> ansatz = PauliEvolutionAnsatz(h)   # Auto strategy (commuting → exact)
/// >>> circuit = ansatz.build_circuit("evo")
/// >>> # circuit has one symbolic parameter "evo_t"
/// >>> ansatz.num_parameters()
/// 1
///
/// >>> # Non-commuting Hamiltonian with Suzuki-2 decomposition
/// >>> h2 = Hamiltonian(1)
/// >>> h2.add_term(PauliString.from_str("X"), 1.0)
/// >>> h2.add_term(PauliString.from_str("Z"), 1.0)
/// >>> ansatz2 = (PauliEvolutionAnsatz(h2)
/// ...     .with_strategy(EvolutionStrategy.trotter(TrotterMode.second_order(), steps=10))
/// ...     .with_time_param_name("tau"))
/// >>> circuit2 = ansatz2.build_circuit("ignored")
/// >>> # circuit2 has one symbolic parameter "tau"
/// ```
#[pyclass(
    name = "PauliEvolutionAnsatz",
    module = "cqlib.circuit.ansatz",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPauliEvolutionAnsatz {
    pub(crate) inner: PauliEvolutionAnsatz,
}

impl From<PauliEvolutionAnsatz> for PyPauliEvolutionAnsatz {
    fn from(inner: PauliEvolutionAnsatz) -> Self {
        Self { inner }
    }
}

impl From<PyPauliEvolutionAnsatz> for PauliEvolutionAnsatz {
    fn from(value: PyPauliEvolutionAnsatz) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyPauliEvolutionAnsatz {
    /// Creates a new ``PauliEvolutionAnsatz`` from a Hamiltonian.
    ///
    /// The Hamiltonian is automatically **simplified** (phases absorbed, duplicate
    /// terms merged, near-zero terms removed) before any further processing.
    ///
    /// Default strategy is :meth:`EvolutionStrategy.auto` with ``steps=1``.
    ///
    /// Args:
    ///     hamiltonian: The Hamiltonian :math:`H = \\sum_k c_k P_k` to evolve.
    ///         Must be Hermitian (real coefficients after simplification).
    ///
    /// Raises:
    ///     ValueError: If the Hamiltonian is empty after simplification.
    ///     ValueError: If any coefficient has a non-zero imaginary part
    ///         (non-Hermitian Hamiltonian).
    #[new]
    fn new(hamiltonian: PyRef<'_, PyHamiltonian>) -> PyResult<Self> {
        PauliEvolutionAnsatz::new(hamiltonian.inner.clone())
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Sets the compilation strategy.
    ///
    /// Args:
    ///     strategy: The :class:`EvolutionStrategy` to use. See
    ///         :meth:`EvolutionStrategy.exact`, :meth:`EvolutionStrategy.auto`, and
    ///         :meth:`EvolutionStrategy.trotter`.
    ///
    /// Returns:
    ///     PauliEvolutionAnsatz: A new ansatz with the updated strategy.
    ///
    /// Examples:
    ///     >>> ansatz = ansatz.with_strategy(EvolutionStrategy.trotter(
    ///     ...     TrotterMode.second_order(), steps=20))
    fn with_strategy(&self, strategy: PyRef<'_, PyEvolutionStrategy>) -> Self {
        Self {
            inner: self.inner.clone().with_strategy(strategy.inner.clone()),
        }
    }

    /// Overrides the name of the time parameter in the built circuit.
    ///
    /// By default the parameter is named ``"{prefix}_t"`` where ``prefix`` is
    /// the argument passed to :meth:`build_circuit`. Setting an explicit name is
    /// useful when composing multiple ansatze that must share a common time
    /// parameter.
    ///
    /// Args:
    ///     name: Explicit parameter name (e.g. ``"tau"``). Pass an empty string
    ///         ``""`` to restore the default prefix-derived name.
    ///
    /// Returns:
    ///     PauliEvolutionAnsatz: A new ansatz with the updated parameter name.
    ///
    /// Examples:
    ///     >>> ansatz = ansatz.with_time_param_name("tau")
    ///     >>> circuit = ansatz.build_circuit("ignored")
    ///     >>> circuit.symbols  # ["tau"]
    fn with_time_param_name(&self, name: &str) -> Self {
        Self {
            inner: self.inner.clone().with_time_param_name(name),
        }
    }

    /// Returns metadata about the compiled evolution.
    ///
    /// This is a **cheap** introspection call that does **not** build the circuit.
    /// Use it to inspect commutativity, effective step count, and whether the
    /// decomposition is exact before committing to :meth:`build_circuit`.
    ///
    /// Returns:
    ///     EvolutionInfo: Metadata describing the current configuration.
    ///
    /// Examples:
    ///     >>> info = ansatz.evolution_info()
    ///     >>> info.is_exact
    ///     True
    ///     >>> info.all_terms_commute
    ///     True
    fn evolution_info(&self) -> PyEvolutionInfo {
        PyEvolutionInfo::from(self.inner.evolution_info())
    }

    /// Validates the ansatz configuration without building the circuit.
    ///
    /// Checks:
    ///
    /// - Hamiltonian is non-empty.
    /// - Hamiltonian is Hermitian (real coefficients after simplification).
    /// - For :meth:`EvolutionStrategy.exact`: all terms must commute.
    /// - For :meth:`EvolutionStrategy.auto` / :meth:`EvolutionStrategy.trotter`:
    ///   ``steps >= 1``.
    ///
    /// Raises:
    ///     ValueError: If any validation check fails (with a descriptive message).
    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Builds the parameterized time-evolution circuit.
    ///
    /// The output circuit contains exactly **one symbolic parameter**: the evolution
    /// time :math:`t`. Parameter naming:
    ///
    /// - If :meth:`with_time_param_name` was called with a non-empty string, that
    ///   name is used exactly (``prefix`` is ignored for the time parameter).
    /// - Otherwise the parameter is named ``"{prefix}_t"``.
    ///
    /// # Angle Convention
    ///
    /// Each Pauli term `c_k P_k` is realized as a rotation with angle
    /// `\\theta_k = 2 c_k t` per Trotter step (or per circuit for exact
    /// evolution). This follows the underlying convention
    /// `e^{-i\\theta/2 \\cdot P}`.
    ///
    /// Args:
    ///     prefix: Prefix for the time parameter name when no explicit name is set.
    ///
    /// Returns:
    ///     Circuit: A parameterized quantum circuit with one symbolic parameter.
    ///
    /// Raises:
    ///     ValueError: If :meth:`validate` fails (e.g. non-commuting terms with
    ///         ``Exact`` strategy, or ``steps < 1``).
    fn build_circuit(&self, prefix: &str) -> PyResult<PyCircuit> {
        self.inner
            .build_circuit(prefix)
            .map(|c| PyCircuit { inner: c })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Returns the number of symbolic parameters in the built circuit.
    ///
    /// Always ``1`` — the evolution time :math:`t`.
    fn num_parameters(&self) -> usize {
        self.inner.num_parameters()
    }

    /// Returns the number of qubits (= number of qubits of the Hamiltonian).
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    fn __repr__(&self) -> String {
        let info = self.inner.evolution_info();
        format!(
            "PauliEvolutionAnsatz(num_qubits={}, num_terms={}, is_exact={}, steps={})",
            self.inner.num_qubits(),
            info.num_terms,
            info.is_exact,
            info.steps,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}
