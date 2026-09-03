# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
#
# Any modifications or derivative works of this code must retain this
# copyright notice, and modified files need to carry a notice indicating
# that they have been altered from the originals.

"""Type stubs for the Hamiltonian-to-Circuit ansatz module.

Provides :class:`PauliEvolutionAnsatz`, which compiles a :class:`~cqlib.qis.Hamiltonian`
into a parameterized quantum circuit implementing (or approximating) the time-evolution
operator :math:`U(t) = e^{-iHt}`, where :math:`t` is a symbolic
:class:`~cqlib.circuit.Parameter`.
"""

from __future__ import annotations

from typing import Optional, final

from ...circuit import Circuit
from ...qis import Hamiltonian
from ...qis.evolution import TrotterMode

@final
class EvolutionStrategy:
    """Controls how a Hamiltonian is compiled into a quantum circuit.

    An ``EvolutionStrategy`` is immutable and created via one of the three
    static factory methods below.  Use it as a builder argument to
    :meth:`PauliEvolutionAnsatz.with_strategy`.

    Variants
    --------
    - :meth:`exact` — single-pass product of Pauli rotations (only valid for
      mutually commuting Hamiltonians).
    - :meth:`auto` — auto-selects exact or first-order Trotter based on
      commutativity.
    - :meth:`trotter` — explicit Trotter-Suzuki decomposition.

    Examples:
        >>> from cqlib.circuit.ansatz import EvolutionStrategy
        >>> from cqlib.qis import TrotterMode
        >>> s1 = EvolutionStrategy.exact()
        >>> s2 = EvolutionStrategy.auto(steps=10)
        >>> s3 = EvolutionStrategy.trotter(TrotterMode.second_order(), steps=5)
    """

    @staticmethod
    def exact() -> "EvolutionStrategy":
        """Returns the ``Exact`` strategy.

        Compiles the Hamiltonian as a single product of Pauli rotations:

        .. math::

            U(t) = \\prod_k e^{-i c_k t P_k}

        This decomposition is **mathematically exact** when all Hamiltonian terms
        mutually commute (:math:`[P_j, P_k] = 0` for all :math:`j, k`).  If any
        two terms do not commute, :meth:`PauliEvolutionAnsatz.build_circuit` will
        raise ``ValueError``.

        Returns:
            EvolutionStrategy: The exact strategy.

        Raises:
            ValueError: At circuit-build time if any two terms do not commute.
        """
        ...

    @staticmethod
    def auto(steps: int = 1) -> "EvolutionStrategy":
        """Returns the ``Auto`` strategy.

        Automatically selects the most appropriate method:

        - **All terms commute**: uses exact single-pass evolution — the ``steps``
          argument is ignored and no approximation error is introduced.
        - **Any two terms do not commute**: uses first-order Lie-Trotter with
          ``steps`` repetitions.

          .. math::

              U(t) \\approx \\left[\\prod_k e^{-i c_k (t/n) P_k}\\right]^n,
              \\quad \\text{error } O\\!\\left(\\frac{t^2}{n}\\right)

        Use :meth:`PauliEvolutionAnsatz.evolution_info` to check which path was
        selected before building the circuit.

        Args:
            steps: Number of Trotter steps :math:`n \\geq 1` used when terms do
                not commute.  Defaults to ``1``.

        Returns:
            EvolutionStrategy: The auto strategy.

        Raises:
            ValueError: At circuit-build time if ``steps < 1``.
        """
        ...

    @staticmethod
    def trotter(mode: TrotterMode, steps: int) -> "EvolutionStrategy":
        """Returns an explicit ``Trotter`` strategy.

        Applies the chosen product-formula approximation regardless of whether
        the Hamiltonian terms commute.

        Available decomposition modes:

        - :meth:`~cqlib.qis.TrotterMode.first_order` — first-order Lie-Trotter:

          .. math::

              U(t) \\approx \\left[\\prod_k e^{-i c_k (t/n) P_k}\\right]^n,
              \\quad \\text{error } O\\!\\left(\\frac{t^2}{n}\\right)

        - :meth:`~cqlib.qis.TrotterMode.second_order` — second-order Suzuki
          (symmetric / Strang splitting):

          .. math::

              U(t) \\approx \\left[
                \\prod_k e^{-i c_k (t/2n) P_k} \\cdot
                \\prod_k^{\\leftarrow} e^{-i c_k (t/2n) P_k}
              \\right]^n,
              \\quad \\text{error } O\\!\\left(\\frac{t^3}{n^2}\\right)

        - :meth:`~cqlib.qis.TrotterMode.randomized` — first-order Trotter with a
          uniformly random term ordering per step (seeded for reproducibility).
          This is a **randomized product formula**, *not* qDrift.

        Args:
            mode: The Trotter decomposition mode (a :class:`~cqlib.qis.TrotterMode`).
            steps: Number of Trotter repetitions :math:`n \\geq 1`.

        Returns:
            EvolutionStrategy: The explicit Trotter strategy.

        Raises:
            ValueError: At circuit-build time if ``steps < 1``.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> "EvolutionStrategy": ...
    def __deepcopy__(self, memo: dict) -> "EvolutionStrategy": ...

@final
class EvolutionInfo:
    """Read-only metadata describing how a :class:`PauliEvolutionAnsatz` compiles.

    Returned by :meth:`PauliEvolutionAnsatz.evolution_info`.  This is a **cheap**
    introspection call — it uses a cached commutativity result computed at
    construction time and does **not** build the circuit.

    All attributes are read-only.

    Attributes:
        is_exact (bool): ``True`` iff the decomposition is mathematically exact.
            This holds whenever all Hamiltonian terms mutually commute and the
            selected strategy emits a mathematically exact decomposition.
            Explicit :meth:`EvolutionStrategy.trotter` can therefore still
            report ``True`` here when applied to a commuting Hamiltonian.

            .. note::

                If the strategy is :meth:`EvolutionStrategy.exact` but the
                Hamiltonian has non-commuting terms, ``is_exact`` is ``False``
                and :meth:`~PauliEvolutionAnsatz.build_circuit` will raise
                ``ValueError``.

        steps (int): The **effective** number of decomposition repetitions
            emitted into the circuit.  This is ``1`` for single-pass exact
            evolution. For explicit :meth:`EvolutionStrategy.trotter`, this
            remains the configured value even when the result is mathematically
            exact because all terms commute.

        trotter_mode (TrotterMode | None): The active Trotter mode, or ``None``
            only when the single-pass exact path is selected.

        all_terms_commute (bool): ``True`` iff all Hamiltonian terms mutually
            commute.  Cached from construction and reused without re-computation.

        num_terms (int): Number of Pauli terms in the simplified Hamiltonian.

    Examples:
        >>> info = ansatz.evolution_info()
        >>> if info.is_exact:
        ...     print("Mathematically exact decomposition")
        ... else:
        ...     print(f"Approximate: mode={info.trotter_mode}, steps={info.steps}")
        ...     # error for first-order  ~ O(t^2 / steps)
        ...     # error for second-order ~ O(t^3 / steps^2)
    """

    @property
    def is_exact(self) -> bool:
        """``True`` iff the decomposition is mathematically exact."""
        ...

    @property
    def steps(self) -> int:
        """Effective number of emitted decomposition repetitions."""
        ...

    @property
    def trotter_mode(self) -> Optional[TrotterMode]:
        """The Trotter mode in use, or ``None`` for the single-pass exact path."""
        ...

    @property
    def all_terms_commute(self) -> bool:
        """``True`` iff all Hamiltonian terms mutually commute."""
        ...

    @property
    def num_terms(self) -> int:
        """Number of Pauli terms in the simplified Hamiltonian."""
        ...

    def __repr__(self) -> str: ...
    def __copy__(self) -> "EvolutionInfo": ...
    def __deepcopy__(self, memo: dict) -> "EvolutionInfo": ...

@final
class PauliEvolutionAnsatz:
    """Compiles a Hamiltonian into a parameterized time-evolution circuit.

    This ansatz implements (or approximates) the unitary

    .. math::

        U(t) = e^{-iHt}, \\quad H = \\sum_k c_k P_k

    where :math:`t` is a single symbolic :class:`~cqlib.circuit.Parameter`.
    The Hamiltonian must be **Hermitian** (real coefficients after simplification).

    Exact Evolution
    ---------------
    When all Pauli terms :math:`P_k` mutually commute:

    .. math::

        e^{-iHt} = \\prod_k e^{-i c_k t P_k}

    Each factor is an exact Pauli rotation with no approximation error.

    Approximate Evolution (Trotter-Suzuki)
    ---------------------------------------
    When terms do not commute, product-formula approximations are used.

    **First-order Lie-Trotter** (:meth:`~cqlib.qis.TrotterMode.first_order`):

    .. math::

        e^{-iHt} \\approx \\left[\\prod_k e^{-i c_k (t/n) P_k}\\right]^n,
        \\quad \\text{error } O\\!\\left(\\frac{t^2}{n}\\right)

    **Second-order Suzuki** (:meth:`~cqlib.qis.TrotterMode.second_order`):

    .. math::

        e^{-iHt} \\approx \\left[
          \\prod_k e^{-i c_k (t/2n) P_k} \\cdot
          \\prod_k^{\\leftarrow} e^{-i c_k (t/2n) P_k}
        \\right]^n,
        \\quad \\text{error } O\\!\\left(\\frac{t^3}{n^2}\\right)

    Angle Convention
    ----------------
    The underlying :meth:`~cqlib.circuit.Circuit.pauli_evolution` gate implements
    :math:`e^{-i\\theta/2 \\cdot P}`.  To realize :math:`e^{-i c t P}`, the
    angle passed internally is :math:`\\theta = 2ct`.  This is the same convention
    used in :class:`~cqlib.circuit.ansatz.QAOAAnsatz`.

    Builder Pattern
    ---------------
    All builder methods return a **new** ``PauliEvolutionAnsatz`` (the original is
    unchanged).  This makes it safe to branch from a base configuration::

        base = PauliEvolutionAnsatz(h)
        exact_version  = base.with_strategy(EvolutionStrategy.exact())
        trotter_version = base.with_strategy(
            EvolutionStrategy.trotter(TrotterMode.second_order(), steps=20))

    Identity Terms
    --------------
    An all-identity Pauli string :math:`P_k = I^{\\otimes n}` contributes only a
    global phase :math:`e^{-i c_k t}`.  This is handled correctly by the underlying
    :meth:`~cqlib.circuit.Circuit.pauli_evolution` call and is not an observable
    quantity.

    Examples:
        **Commuting Hamiltonian (exact evolution):**

        >>> from cqlib.circuit.ansatz import PauliEvolutionAnsatz, EvolutionStrategy
        >>> from cqlib.qis import Hamiltonian
        >>> h = Hamiltonian(2)
        >>> h.add_term(PauliString.from_str("ZZ"), 0.5)
        >>> h.add_term(PauliString.from_str("ZI"), 0.3)
        >>> ansatz = PauliEvolutionAnsatz(h)
        >>> info = ansatz.evolution_info()
        >>> info.is_exact, info.all_terms_commute
        (True, True)
        >>> circuit = ansatz.build_circuit("evo")
        >>> circuit.symbols   # ['evo_t']

        **Non-commuting Hamiltonian (Trotter approximation):**

        >>> h2 = Hamiltonian(1)
        >>> h2.add_term(PauliString.from_str("X"), 1.0)
        >>> h2.add_term(PauliString.from_str("Z"), 1.0)
        >>> from cqlib.qis import TrotterMode
        >>> ansatz2 = (PauliEvolutionAnsatz(h2)
        ...     .with_strategy(EvolutionStrategy.trotter(TrotterMode.second_order(), steps=10))
        ...     .with_time_param_name("tau"))
        >>> circuit2 = ansatz2.build_circuit("ignored")
        >>> circuit2.symbols   # ['tau']
        >>> ansatz2.num_parameters()
        1
    """

    def __init__(self, hamiltonian: Hamiltonian) -> None:
        """Creates a new ``PauliEvolutionAnsatz``.

        The Hamiltonian is automatically **simplified** before processing:
        Pauli phases are absorbed into coefficients, duplicate terms are merged,
        and near-zero terms are removed.  The default strategy is
        :meth:`EvolutionStrategy.auto` with ``steps=1``.

        Args:
            hamiltonian: The Hamiltonian :math:`H = \\sum_k c_k P_k`.
                Must be Hermitian (real coefficients after simplification).

        Raises:
            ValueError: If the Hamiltonian is empty after simplification.
            ValueError: If any coefficient has a non-zero imaginary part after
                simplification (non-Hermitian Hamiltonian).
        """
        ...

    def with_strategy(self, strategy: EvolutionStrategy) -> "PauliEvolutionAnsatz":
        """Sets the compilation strategy.

        Args:
            strategy: The :class:`EvolutionStrategy` to use.

        Returns:
            PauliEvolutionAnsatz: A new ansatz with the updated strategy.

        Examples:
            >>> from cqlib.qis import TrotterMode
            >>> ansatz = ansatz.with_strategy(
            ...     EvolutionStrategy.trotter(TrotterMode.second_order(), steps=20))
        """
        ...

    def with_time_param_name(self, name: str) -> "PauliEvolutionAnsatz":
        """Overrides the name of the time parameter in the built circuit.

        By default the time parameter is named ``"{prefix}_t"`` where ``prefix``
        is the argument passed to :meth:`build_circuit`.  Setting an explicit name
        is useful when composing multiple ansatze that must share a common time
        parameter or when a predictable name is required for external tooling.

        Args:
            name: Explicit parameter name (e.g. ``"tau"``).  Pass an empty string
                ``""`` to restore the default prefix-derived name ``"{prefix}_t"``.

        Returns:
            PauliEvolutionAnsatz: A new ansatz with the updated parameter name.

        Examples:
            >>> ansatz = ansatz.with_time_param_name("tau")
            >>> circuit = ansatz.build_circuit("ignored")
            >>> circuit.symbols   # ['tau']
        """
        ...

    def evolution_info(self) -> EvolutionInfo:
        """Returns metadata about the compiled evolution.

        This is a **cheap** introspection call — it reads a cached commutativity
        result computed at construction time without building the circuit.

        Returns:
            EvolutionInfo: Metadata for the current configuration including
            :attr:`~EvolutionInfo.is_exact`, :attr:`~EvolutionInfo.steps`,
            :attr:`~EvolutionInfo.trotter_mode`, :attr:`~EvolutionInfo.all_terms_commute`,
            and :attr:`~EvolutionInfo.num_terms`.

        Examples:
            >>> info = ansatz.evolution_info()
            >>> if not info.is_exact:
            ...     print(f"Trotter error ≈ O(t² / {info.steps})")
        """
        ...

    def validate(self) -> None:
        """Validates the ansatz configuration without building the circuit.

        Performs the following checks:

        1. The Hamiltonian is non-empty.
        2. All coefficients are real (Hermitian check).
        3. For :meth:`EvolutionStrategy.exact`: all terms must mutually commute.
        4. For :meth:`EvolutionStrategy.auto` / :meth:`EvolutionStrategy.trotter`:
           ``steps >= 1``.

        Raises:
            ValueError: If any check fails, with a descriptive error message.
        """
        ...

    def build_circuit(self, prefix: str) -> Circuit:
        """Builds the parameterized time-evolution circuit.

        The resulting circuit contains exactly **one** symbolic parameter: the
        evolution time :math:`t`.

        Parameter Naming
        ~~~~~~~~~~~~~~~~
        - If :meth:`with_time_param_name` was called with a non-empty string, that
          name is used exactly (``prefix`` is ignored for the time parameter).
        - Otherwise the parameter is named ``"{prefix}_t"``.

        Angle Convention
        ~~~~~~~~~~~~~~~~
        Each Pauli term :math:`c_k P_k` contributes a rotation angle

        .. math::

            \\theta_k = 2 c_k \\cdot \\frac{t}{n}

        per Trotter step (or :math:`\\theta_k = 2 c_k t` for exact evolution).
        This realizes :math:`e^{-i c_k t/n \\cdot P_k}` via the underlying gate
        :math:`e^{-i\\theta/2 \\cdot P}`.

        Args:
            prefix: String prefix for the time parameter when no explicit name is
                set (e.g. ``"evo"`` → parameter name ``"evo_t"``).

        Returns:
            Circuit: A parameterized quantum circuit with exactly one symbolic
            parameter.

        Raises:
            ValueError: If :meth:`validate` fails (e.g. non-commuting terms with
                ``Exact`` strategy, or ``steps < 1``).
        """
        ...

    def num_parameters(self) -> int:
        """Returns the number of symbolic parameters in the built circuit.

        Always ``1`` — the evolution time :math:`t`.

        Returns:
            int: Always ``1``.
        """
        ...

    def num_qubits(self) -> int:
        """Returns the number of qubits (equals the Hamiltonian's qubit count).

        Returns:
            int: Number of qubits.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> "PauliEvolutionAnsatz": ...
    def __deepcopy__(self, memo: dict) -> "PauliEvolutionAnsatz": ...
