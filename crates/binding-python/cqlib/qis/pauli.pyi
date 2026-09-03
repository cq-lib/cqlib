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

from typing import Dict, List, Tuple, Sequence, final
import numpy as np

from .state import Statevector, DensityMatrix

@final
class Phase:
    """Phase factor in the Pauli group, isomorphic to Z4 (cyclic group of order 4)."""

    def __new__(cls, val: int) -> "Phase":
        """Creates a Phase from an integer (mod 4)."""
        ...

    @staticmethod
    def plus() -> "Phase":
        """Returns the +1 phase."""
        ...

    @staticmethod
    def i() -> "Phase":
        """Returns the +i phase."""
        ...

    @staticmethod
    def minus() -> "Phase":
        """Returns the -1 phase."""
        ...

    @staticmethod
    def minus_i() -> "Phase":
        """Returns the -i phase."""
        ...

    def to_complex(self) -> complex:
        """Converts the phase to a Python complex number."""
        ...

    @property
    def exponent(self) -> int:
        """Returns the phase as an integer exponent (0-3)."""
        ...

    def __add__(self, other: "Phase") -> "Phase":
        """Adds two phases (multiplication in the group)."""
        ...

    def __mul__(self, other: "Phase") -> "Phase":
        """Multiplies two phases (same as addition in Z4)."""
        ...

    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

@final
class Pauli:
    """Single-qubit Pauli operators (I, X, Y, Z)."""

    @staticmethod
    def x() -> "Pauli":
        """Returns the X Pauli operator."""
        ...

    @staticmethod
    def y() -> "Pauli":
        """Returns the Y Pauli operator."""
        ...

    @staticmethod
    def z() -> "Pauli":
        """Returns the Z Pauli operator."""
        ...

    @staticmethod
    def i() -> "Pauli":
        """Returns the Identity operator."""
        ...

    def to_symplectic(self) -> Tuple[int, int]:
        """Returns the symplectic representation (x, z) as a tuple.

        The symplectic encoding maps Pauli operators to binary pairs:
        - I = (0, 0)
        - X = (1, 0)
        - Y = (1, 1)
        - Z = (0, 1)
        """
        ...

    def to_matrix(self) -> np.ndarray:
        """Returns the 2x2 complex matrix representation as a NumPy array."""
        ...

    def mul_with_phase(self, other: "Pauli") -> Tuple["Pauli", Phase]:
        """Multiplies two Pauli operators, returning the result and phase factor."""
        ...

    def __mul__(self, other: "Pauli") -> "Pauli":
        """Multiplication operator (without explicit phase tracking)."""
        ...

    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

@final
class PauliString:
    """Multi-qubit Pauli string operator in symplectic representation."""

    def __new__(cls, num_qubits: int) -> "PauliString":
        """Creates a new identity Pauli string with the specified number of qubits."""
        ...

    @staticmethod
    def from_str(s: str) -> "PauliString":
        """Creates a PauliString from a string representation.

        The format is: `[+|-][i|j]<pauli operators>` where pauli operators are I, X, Y, or Z.
        Qubits are in reverse order: the first character corresponds to the highest qubit index.

        Args:
            s: String representation like "XZI", "-iXYZ", "+ZII"

        Raises:
            ValueError: If the string format is invalid
        """
        ...

    def set_pauli(self, idx: int, pauli: Pauli) -> None:
        """Sets the Pauli operator at the specified qubit index.

        Args:
            idx: Qubit index (0 to num_qubits-1)
            pauli: The Pauli operator to set

        Raises:
            IndexError: If idx >= num_qubits
        """
        ...

    def get_pauli(self, idx: int) -> Pauli:
        """Gets the Pauli operator at the specified qubit index.

        Args:
            idx: Qubit index (0 to num_qubits-1)

        Returns:
            The Pauli operator at the specified index

        Raises:
            IndexError: If idx >= num_qubits
        """
        ...

    @property
    def num_qubits(self) -> int:
        """Returns the number of qubits in the Pauli string."""
        ...

    @property
    def phase(self) -> Phase:
        """Returns the global phase factor."""
        ...

    @phase.setter
    def phase(self, phase: Phase) -> None:
        """Sets the global phase factor."""
        ...

    @property
    def x_bits(self) -> List[bool]:
        """Returns the X-component bit vector as a list of booleans."""
        ...

    @property
    def z_bits(self) -> List[bool]:
        """Returns the Z-component bit vector as a list of booleans."""
        ...

    @property
    def x_mask(self) -> int:
        """Returns the X-component as an integer mask."""
        ...

    @property
    def z_mask(self) -> int:
        """Returns the Z-component as an integer mask."""
        ...

    def y_phase(self) -> complex:
        """Computes the phase factor contributed by Y operators.

        Y = iXZ, so n Y operators contribute i^n phase.

        Returns:
            A Python complex number (1, i, -1, or -i)
        """
        ...

    def commutes_with(self, other: "PauliString") -> bool:
        """Checks if this Pauli string commutes with another.

        Two Pauli strings commute if their symplectic inner product is 0 (mod 2).

        Raises:
            ValueError: If Pauli strings have different number of qubits
        """
        ...

    def to_matrix(self) -> np.ndarray:
        """Returns the dense matrix representation as a NumPy array.

        The matrix is the tensor product of the single-qubit Pauli operators
        in little-endian order: qubit 0 is the least-significant tensor factor,
        i.e. the string expands as P_{N-1} ⊗ ... ⊗ P_0. The global phase is included.

        Memory usage is O(4^N); intended for small-system analysis and verification.

        Returns:
            A NumPy array of shape (2^num_qubits, 2^num_qubits) with complex128 dtype.
        """
        ...

    def support(self) -> List[int]:
        """Returns the qubit indices where the Pauli is not identity, in ascending order.

        Returns:
            A sorted list of qubit indices supporting a non-identity Pauli (X, Y, or Z).
        """
        ...

    def __len__(self) -> int:
        """Returns the number of qubits (the number of single-qubit Pauli operators)."""
        ...

    def __iter__(self) -> "PauliStringIter":
        """Returns an iterator over single-qubit Pauli operators in ascending qubit index.

        Identity operators are included. Iteration uses snapshot semantics: modifications
        to the PauliString after creating the iterator do not affect the iteration.

        Equivalent to calling `get_pauli(i)` for i in range(num_qubits), but lazy.
        """
        ...

    def __getitem__(self, index: int) -> Pauli:
        """Returns the Pauli operator at the given qubit index.

        Supports negative indexing: index -1 refers to the highest qubit index.

        Args:
            index: Qubit index (negative values count from the end)

        Raises:
            IndexError: If index is out of bounds
        """
        ...

    def expectation(self, probs: Dict[str, float]) -> float:
        """Computes the expectation value given a probability distribution.

        This calculates ⟨P⟩ = Σ_s p(s) ⟨s|P|s⟩, where p(s) is the probability of basis state |s⟩.

        Important: The state keys use little-endian convention: the rightmost character
        corresponds to qubit 0. For example, "01" means qubit 0 = 1, qubit 1 = 0.

        If this Pauli string contains X or Y operators (non-diagonal), the expectation
        value is 0 for any probability distribution over computational basis states.

        Args:
            probs: A dict mapping state strings (e.g., "00", "01") to their probabilities.
                   The string uses little-endian: index 0 (leftmost) is qubit n-1.

        Returns:
            The expectation value as a float.

        Raises:
            ValueError: If state string length doesn't match num_qubits or contains invalid chars.
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
        self, measurements: Sequence[Tuple["PauliString", Dict[str, float]]]
    ) -> float:
        """Computes the expectation value from measurement probabilities.

        Args:
            measurements: A sequence of tuples containing the measurement basis
                (as a PauliString) and a map from state strings to their observed probabilities.

        Returns:
            The real expectation value.
        """
        ...

    def variance_statevector(self, sv: Statevector) -> float:
        """Computes the variance for a statevector.

        Raises:
            ValueError: If the Pauli string is not Hermitian or qubit counts differ.
        """
        ...

    def __mul__(self, other: "PauliString") -> "PauliString":
        """Returns a new Pauli string that is the product of this and another."""
        ...

    def __imul__(self, other: "PauliString") -> "PauliString":
        """In-place multiplication with another Pauli string."""
        ...

    def __eq__(self, other: object) -> bool: ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
    def copy(self) -> "PauliString": ...

@final
class PauliStringIter:
    """Lazy iterator over the single-qubit Pauli operators of a PauliString.

    Created by `PauliString.__iter__`. Yields `Pauli` objects in ascending
    qubit index order (identities included), raising StopIteration when exhausted.

    Iteration uses snapshot semantics: modifications to the parent PauliString
    after creating the iterator do not affect the iteration.
    """

    def __next__(self) -> Pauli:
        """Returns the next Pauli operator, or raises StopIteration when exhausted."""
        ...

    def __iter__(self) -> "PauliStringIter":
        """Returns the iterator itself (iterators are iterable)."""
        ...

    def __length_hint__(self) -> int:
        """Returns the number of remaining items."""
        ...

    def __repr__(self) -> str: ...
