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

from typing import List, Tuple, Union, final, Sequence, Dict

import numpy as np

from cqlib.circuit import Circuit

from . import TrotterMode
from .pauli import PauliString
from .state import Statevector, DensityMatrix

# Type alias for coefficient types (float, int, complex, or tuple)
Coefficient = Union[float, int, complex, Tuple[float, float]]

@final
class Hamiltonian:
    """A quantum Hamiltonian represented as a sum of Pauli strings.

    A `Hamiltonian` is essentially a sparse representation of a $2^N \times 2^N$
    matrix, expressed as $H = \sum_k c_k P_k$, where $c_k$ is a complex coefficient
    and $P_k$ is an $N$-qubit Pauli string.

    This is commonly used for defining system energies, observables for expectation
    value calculations, and operators for time evolution.

    Examples:
        >>> from cqlib.qis import Hamiltonian, PauliString
        >>> # Create a 2-qubit Hamiltonian
        >>> h = Hamiltonian(2)
        >>> # Add terms: H = 0.5 * ZZ + 0.3 * XX
        >>> h.add_term(PauliString.from_str("ZZ"), 0.5)
        >>> h.add_term(PauliString.from_str("XX"), 0.3)
        >>> # Simplify to merge duplicate terms
        >>> h.simplify()
    """

    def __new__(cls, num_qubits: int) -> "Hamiltonian":
        """Creates a new empty Hamiltonian.

        The resulting Hamiltonian represents the zero operator for the given
        number of qubits.

        Args:
            num_qubits: The number of qubits this operator acts on.

        Examples:
            >>> h = Hamiltonian(3)  # 3-qubit Hamiltonian
        """
        ...

    @staticmethod
    def from_pauli(pauli: PauliString) -> "Hamiltonian":
        """Creates a Hamiltonian from a single Pauli string with a coefficient of 1.0.

        Args:
            pauli: The Pauli string to wrap into a Hamiltonian.

        Returns:
            A new Hamiltonian representing H = 1.0 * P.

        Examples:
            >>> from cqlib.qis import Hamiltonian, PauliString
            >>> h = Hamiltonian.from_pauli(PauliString.from_str("ZZ"))
        """
        ...

    @staticmethod
    def from_list(terms: Sequence[Tuple[PauliString, Coefficient]]) -> "Hamiltonian":
        """Creates a Hamiltonian from a list of (PauliString, coefficient) tuples.

        Args:
            terms: A list of tuples, each containing a PauliString and a coefficient.
                   Coefficients can be float, int, complex, or tuple (real, imag).

        Returns:
            A new Hamiltonian instance.

        Raises:
            ValueError: If not all Pauli strings have the same number of qubits.

        Examples:
            >>> from cqlib.qis import Hamiltonian, PauliString
            >>> terms = [
            ...     (PauliString.from_str("ZZ"), 0.5),
            ...     (PauliString.from_str("XX"), (0.0, 0.3)),  # complex 0.3j
            ... ]
            >>> h = Hamiltonian.from_list(terms)
        """
        ...

    def add_term(self, op: PauliString, coeff: Coefficient) -> None:
        """Adds a new Pauli string term with a given coefficient to the Hamiltonian.

        Args:
            op: The Pauli string operator to add.
            coeff: The coefficient for this term. Can be float, int, complex, or tuple (real, imag).

        Returns:
            None

        Raises:
            ValueError: If the number of qubits in the operator does not match the Hamiltonian.

        Examples:
            >>> from cqlib.qis import Hamiltonian, PauliString
            >>> h = Hamiltonian(2)
            >>> h.add_term(PauliString.from_str("ZZ"), 0.5)
            >>> h.add_term(PauliString.from_str("XX"), (0.0, 0.3))  # complex 0.3j
        """
        ...

    def simplify(self) -> None:
        """Simplifies the Hamiltonian by combining terms with the same Pauli string.

        This method performs two optimizations:
        1. **Phase Normalization**: Absorbs any internal phases from the PauliString
           into the complex coefficient.
        2. **Term Aggregation**: Groups terms with identical Pauli strings and sums
           their coefficients. Terms with near-zero coefficients are removed.

        This is important for optimizing performance before quantum simulations.

        Examples:
            >>> from cqlib.qis import Hamiltonian, PauliString
            >>> h = Hamiltonian(2)
            >>> h.add_term(PauliString.from_str("ZZ"), 0.5)
            >>> h.add_term(PauliString.from_str("ZZ"), 0.3)  # duplicate
            >>> h.simplify()  # Now H = 0.8 * ZZ
        """
        ...

    def scale(self, factor: Coefficient) -> None:
        """Scales all terms in the Hamiltonian by a complex factor.

        Args:
            factor: The scaling factor. Can be float, int, complex, or tuple (real, imag).

        Examples:
            >>> from cqlib.qis import Hamiltonian, PauliString
            >>> h = Hamiltonian(2)
            >>> h.add_term(PauliString.from_str("ZZ"), 1.0)
            >>> h.scale(2.0)  # H = 2.0 * ZZ
        """
        ...

    @property
    def num_qubits(self) -> int:
        """Returns the number of qubits this Hamiltonian acts on."""
        ...

    @property
    def terms(self) -> List[Tuple[PauliString, complex]]:
        """Returns the list of terms as (PauliString, complex) tuples.

        Returns:
            A list of tuples, each containing a PauliString and its complex coefficient.
        """
        ...

    @property
    def num_terms(self) -> int:
        """Returns the number of terms in the Hamiltonian.

        This is the length of the terms list before simplification.
        """
        ...

    def all_terms_commute(self) -> bool:
        """Returns ``True`` if all Pauli terms mutually commute."""
        ...

    def to_matrix(self) -> np.ndarray:
        """Computes the dense matrix representation as a NumPy array.

        Evaluates H = Σ_k c_k P_k by summing each term's Pauli string matrix
        (in little-endian qubit order, including Pauli phases) multiplied by
        its coefficient. An empty Hamiltonian yields the zero matrix.

        Memory usage is O(4^N); intended for small-system analysis and verification.

        Returns:
            A NumPy array of shape (2^num_qubits, 2^num_qubits) with complex128 dtype.

        Raises:
            ValueError: If a term's qubit count does not match the Hamiltonian's
                (defensive; the public constructors already reject such terms).
        """
        ...

    def expectation_statevector(self, sv: Statevector) -> float:
        """Computes the expectation value for a statevector.

        Args:
            sv: The statevector.

        Returns:
            The real expectation value.
        """
        ...

    def expectation_density_matrix(self, dm: DensityMatrix) -> float:
        """Computes the expectation value for a density matrix.

        Args:
            dm: The density matrix.

        Returns:
            The real expectation value.
        """
        ...

    def expectation_probs(
        self, measurements: Sequence[Tuple[PauliString, Dict[str, float]]]
    ) -> float:
        """Computes the expectation value from measurement probabilities.

        Args:
            measurements: A sequence of tuples containing the measurement basis
                (as a PauliString) and a map from state strings to their observed probabilities.

        Returns:
            The real expectation value.
        """
        ...

    def __add__(self, other: "Hamiltonian") -> "Hamiltonian":
        """Adds two Hamiltonians together.

        Note: This performs a simple lazy concatenation of the term lists.
        It does not automatically merge identical terms. Call `simplify()` after
        addition to optimize the result.

        Args:
            other: The Hamiltonian to add.

        Returns:
            A new Hamiltonian containing all terms from both.

        Raises:
            ValueError: If the Hamiltonians have different numbers of qubits.

        Examples:
            >>> from cqlib.qis import Hamiltonian, PauliString
            >>> h1 = Hamiltonian(2)
            >>> h1.add_term(PauliString.from_str("ZZ"), 0.5)
            >>> h2 = Hamiltonian(2)
            >>> h2.add_term(PauliString.from_str("XX"), 0.3)
            >>> h3 = h1 + h2  # Contains both ZZ and XX terms
        """
        ...

    def __iadd__(self, other: "Hamiltonian") -> "Hamiltonian":
        """In-place addition of another Hamiltonian.

        Args:
            other: The Hamiltonian to add.

        Returns:
            self

        Raises:
            ValueError: If the Hamiltonians have different numbers of qubits.
        """
        ...

    def __eq__(self, other: object) -> bool:
        """Checks if two Hamiltonians are equal."""
        ...

    def __str__(self) -> str:
        """Returns a string representation of the Hamiltonian."""
        ...

    def __repr__(self) -> str:
        """Returns a detailed string representation of the Hamiltonian."""
        ...

    def to_trotter_circuit(self, time: float, steps: int, mode: TrotterMode) -> Circuit:
        """Converts the Hamiltonian evolution e^(-iHt) into a quantum circuit
        using Trotter-Suzuki decomposition.

        Args:
            time: The total evolution time t.
            steps: The number of Trotter steps (n).
            mode: The decomposition mode (e.g., FirstOrder, SecondOrder).

        Returns:
            A quantum circuit implementing the approximate time evolution.

        Raises:
            ValueError: If the Hamiltonian has no terms or decomposition fails.

        Examples:
            >>> from cqlib.qis import Hamiltonian, PauliString, TrotterMode
            >>> h = Hamiltonian(2)
            >>> h.add_term(PauliString.from_str("ZZ"), 0.5)
            >>> h.add_term(PauliString.from_str("XX"), 0.3)
            >>> circuit = h.to_trotter_circuit(1.0, 10, TrotterMode.first_order())
        """
        ...

    def to_evolution_circuit(self, time: float, steps: int, mode: TrotterMode) -> Circuit:
        """Converts the Hamiltonian evolution e^(-iHt) into a quantum circuit.

        Commuting terms use an exact single-pass decomposition. Non-commuting
        terms fall back to the selected Trotter-Suzuki approximation.

        Args:
            time: The total evolution time.
            steps: Trotter steps used for non-commuting terms; must be positive.
            mode: Trotter decomposition mode used for non-commuting terms.

        Raises:
            ValueError: If the Hamiltonian is empty, non-Hermitian, or ``steps`` is zero.
        """
        ...

    def variance_statevector(self, sv: Statevector) -> float:
        """Computes the variance for a statevector.

        Raises:
            ValueError: If the Hamiltonian is not Hermitian or qubit counts differ.
        """
        ...

    def copy(self) -> "Hamiltonian":
        """Returns a copy of this Hamiltonian.

        Returns:
            A new Hamiltonian instance with the same terms.
        """
        ...
