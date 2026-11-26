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

from typing import List
from .state import DensityMatrix, Statevector

def linear_entropy(dm: DensityMatrix) -> float:
    """Calculates the linear entropy of a quantum state.

    The linear entropy is defined as :math:`S_L(\\rho) = 1 - \\text{Tr}(\\rho^2) = 1 - \\text{Purity}(\\rho)`.
    This is a computationally efficient approximation of the Von Neumann entropy,
    serving as a measure of mixedness: 0 for pure states, approaching :math:`1 - 1/2^N`
    for maximally mixed states.

    Args:
        dm (DensityMatrix): The density matrix representing the quantum state.

    Returns:
        float: The linear entropy value in [0, 1).

    Raises:
        ValueError: If the calculation fails.
    """
    ...

def renyi_entropy(dm: DensityMatrix, alpha: float) -> float:
    """Calculates the Rényi entropy of order alpha.

    The Rényi entropy is defined as :math:`S_\\alpha(\\rho) = \\frac{1}{1-\\alpha} \\log_2(\\text{Tr}(\\rho^\\alpha))`.

    Special cases:
        - When ``alpha = 1``: Returns Von Neumann entropy.
        - When ``alpha = 2``: Collision entropy.
        - When ``alpha = +inf``: Min-entropy.

    Args:
        dm (DensityMatrix): The density matrix representing the quantum state.
        alpha (float): The order parameter. Must be strictly positive.

    Returns:
        float: The Rényi entropy in bits (base-2 logarithm).

    Raises:
        ValueError: If alpha is NaN or <= 0, eigendecomposition fails,
            or no positive spectrum remains.

    Notes:
        Non-diagonal spectra discard eigenvalues at or below
        dimension * machine epsilon * largest eigenvalue as numerical zeros.
        Diagonal spectra retain all positive entries.
    """
    ...

def entanglement_entropy_pure(sv: Statevector, subsys_a: List[int]) -> float:
    """Calculates the entanglement entropy for a bipartite pure state.

    For a pure state of a composite system AB, the entanglement entropy is
    defined as the Von Neumann entropy of the reduced density matrix of subsystem A:
    :math:`E(|\\psi\\rangle) = S(\\rho_A) = -\\text{Tr}(\\rho_A \\log_2 \\rho_A)`.

    Args:
        sv (Statevector): The statevector representing the pure quantum state.
        subsys_a (List[int]): Indices of qubits belonging to subsystem A.
            All other qubits are considered part of subsystem B.

    Returns:
        float: The entanglement entropy in bits.

    Raises:
        ValueError: If `subsys_a` is empty, contains all qubits, has duplicate indices,
            or contains out-of-bounds indices.
    """
    ...

def negativity(dm: DensityMatrix, subsys_a: List[int]) -> float:
    """Calculates the negativity entanglement measure.

    The negativity is based on the Peres-Horodecki criterion (PPT criterion).
    It is computed as the absolute sum of the negative eigenvalues of the
    partially transposed density matrix with respect to subsystem A.

    Args:
        dm (DensityMatrix): The density matrix of the bipartite quantum state.
        subsys_a (List[int]): The qubit indices comprising subsystem A to be transposed.

    Returns:
        float: The negativity value (>= 0). A value of 0 indicates the state is
            separable (for 2x2 and 2x3 systems).

    Raises:
        ValueError: If subsystem indices are invalid or eigendecomposition fails.
    """
    ...

def concurrence(dm: DensityMatrix) -> float:
    """Calculates the concurrence for a 2-qubit quantum state.

    The concurrence is an exact entanglement measure specifically for two-qubit systems.
    It ranges from 0 (separable state) to 1 (maximally entangled state).

    Args:
        dm (DensityMatrix): The density matrix of a 2-qubit quantum state.

    Returns:
        float: The concurrence value in [0, 1].

    Raises:
        ValueError: If the state does not have exactly 2 qubits or calculation fails.
    """
    ...

def entanglement_of_formation(dm: DensityMatrix) -> float:
    """Calculates the entanglement of formation for a 2-qubit state.

    The entanglement of formation quantifies the minimum amount of entanglement
    required to prepare a given mixed state. For 2-qubit systems, it is derived
    directly from the concurrence.

    Args:
        dm (DensityMatrix): The density matrix of a 2-qubit quantum state.

    Returns:
        float: The entanglement of formation in bits.

    Raises:
        ValueError: If the state does not have exactly 2 qubits.
    """
    ...
