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

from typing import Dict, List, Union, final, Optional
import numpy as np

from cqlib.circuit import Circuit, Measurement
from cqlib.circuit.gates.standard import StandardGate
from cqlib.device import ExecutionResult, Outcome
from cqlib.qis import Hamiltonian, PauliString

@final
class DensityMatrix:
    """Quantum density matrix representing mixed or pure quantum states.

    A density matrix describes the statistical state of an N-qubit quantum system.
    Unlike a statevector which can only represent pure states, a density matrix
    can represent mixed states (ensembles of pure states).

    # Memory Layout
    The data uses contiguous memory layout representing a flattened 2^N x 2^N matrix.
    To optimize simulation performance, the simulator employs a 2N-qubit isomorphism:
    - The matrix is treated as a statevector of 2N qubits.
    - The "ket" side (Left U) acts on the upper N qubits (indices N to 2N-1).
    - The "bra" side (Right U†) acts on the lower N qubits (indices 0 to N-1).

    # Example
    ```python
    from cqlib.qis.state import DensityMatrix

    # Create a 1-qubit density matrix in state |0⟩⟨0|
    dm = DensityMatrix(1)

    # Apply Hadamard gate -> |+⟩⟨+|
    dm.apply_h(0)

    # Probabilities should be 0.5 for both |0⟩ and |1⟩
    probs = dm.probabilities()
    print(probs)  # [0.5, 0.5]
    ```
    """

    def __new__(cls, num_qubits: int) -> "DensityMatrix":
        """Creates a new density matrix initialized to the pure state |0...0⟩⟨0...0|.

        Args:
            num_qubits: Number of qubits in the system

        Returns:
            A new DensityMatrix instance in the ground state

        Examples:
            >>> dm = DensityMatrix(2)  # |00⟩⟨00| state
        """
        ...

    @staticmethod
    def from_state(
        num_qubits: int, initial_state: Union[np.ndarray, List[complex]]
    ) -> "DensityMatrix":
        """Creates a density matrix from an initial statevector (pure state).

        Internally computes the outer product ρ = |ψ⟩⟨ψ|.

        Args:
            num_qubits: Number of qubits in the system
            initial_state: NumPy array of 2^N complex amplitudes, or a list of complex numbers

        Returns:
            A new DensityMatrix instance

        Raises:
            ValueError: If the state length doesn't match 2^num_qubits or state is not normalized

        Examples:
            >>> import numpy as np
            >>> # Create |+⟩ = (|0⟩ + |1⟩)/√2
            >>> amps = np.array([1/np.sqrt(2), 1/np.sqrt(2)], dtype=complex)
            >>> dm = DensityMatrix.from_state(1, amps)
        """
        ...

    @staticmethod
    def from_density_matrix(
        num_qubits: int, dm_state: Union[np.ndarray, List[complex]]
    ) -> "DensityMatrix":
        """Creates a density matrix directly from a flattened 2^N x 2^N matrix.

        Args:
            num_qubits: Number of qubits in the system
            dm_state: NumPy array of 4^N complex values representing the density matrix

        Returns:
            A new DensityMatrix instance

        Raises:
            ValueError: If the matrix length doesn't match 4^num_qubits or trace is not 1

        Examples:
            >>> import numpy as np
            >>> # Create |0⟩⟨0| density matrix for 1 qubit
            >>> dm_flat = np.array([1, 0, 0, 0], dtype=complex)
            >>> dm = DensityMatrix.from_density_matrix(1, dm_flat)
        """
        ...

    @staticmethod
    def from_circuit(circuit: Circuit) -> "DensityMatrix":
        """Creates a density matrix by simulating a quantum circuit.

        Executes the circuit gates sequentially to evolve the initial |0...0⟩⟨0...0| state.

        Args:
            circuit: The quantum circuit to simulate

        Returns:
            A new DensityMatrix instance after circuit execution

        Raises:
            ValueError: If the circuit contains unsupported operations

        Examples:
            >>> from cqlib import Circuit
            >>> from cqlib.qis.state import DensityMatrix
            >>> circuit = Circuit(2)
            >>> circuit.h(0)
            >>> circuit.cx(0, 1)
            >>> dm = DensityMatrix.from_circuit(circuit)
        """
        ...

    @staticmethod
    def maximally_mixed(num_qubits: int) -> "DensityMatrix":
        """Creates the maximally mixed state I / 2^N.

        The result is a diagonal matrix with entries 1/2^N on the diagonal,
        representing a uniform classical mixture over all computational basis
        states. For zero qubits this yields the 1x1 matrix [1].

        Args:
            num_qubits: Number of qubits in the system

        Returns:
            A new DensityMatrix instance in the maximally mixed state

        Examples:
            >>> from cqlib.qis.state import DensityMatrix
            >>> dm = DensityMatrix.maximally_mixed(1)
            >>> dm.probabilities()
            [0.5, 0.5]
        """
        ...

    def apply_circuit(self, circuit: Circuit) -> None:
        """Applies a circuit to this density matrix in place."""
        ...

    @property
    def num_qubits(self) -> int:
        """Returns the number of qubits in the density matrix."""
        ...

    @property
    def data(self) -> np.ndarray:
        """Returns the density matrix data as a 2D NumPy array.

        Returns:
            A 2D NumPy array of complex numbers with shape (2^num_qubits, 2^num_qubits).
        """
        ...

    def probabilities(self) -> List[float]:
        """Returns the measurement probabilities for all computational basis states.

        Extracts the diagonal elements of the density matrix, which represent
        the probabilities P(|i⟩) = ρ_ii.

        Returns:
            A list of probabilities (floats) with length 2^num_qubits.
        """
        ...

    def trace(self) -> float:
        """Computes the trace of the density matrix.

        For any valid physical state, the trace must equal 1.0.

        Returns:
            The trace (sum of diagonal elements) as a real number.
        """
        ...

    def apply_standard_gate(
        self,
        gate: StandardGate,
        qubits: List[int],
        params: Optional[List[float]] = None,
    ) -> None:
        """Applies a standard gate to the density matrix.

        Args:
            gate: The standard gate to apply.
            qubits: List of target qubit indices.
            params: List of parameters for parameterized gates.
        """
        ...

    def apply_x(self, qubit: int) -> None:
        """Applies the Pauli-X (NOT) gate to the specified qubit."""
        ...

    def apply_y(self, qubit: int) -> None:
        """Applies the Pauli-Y gate to the specified qubit."""
        ...

    def apply_z(self, qubit: int) -> None:
        """Applies the Pauli-Z gate to the specified qubit."""
        ...

    def apply_h(self, qubit: int) -> None:
        """Applies the Hadamard gate to the specified qubit."""
        ...

    def apply_s(self, qubit: int) -> None:
        """Applies the S (phase) gate to the specified qubit."""
        ...

    def apply_sdg(self, qubit: int) -> None:
        """Applies the S† (S-dagger) gate to the specified qubit."""
        ...

    def apply_t(self, qubit: int) -> None:
        """Applies the T gate to the specified qubit."""
        ...

    def apply_tdg(self, qubit: int) -> None:
        """Applies the T† (T-dagger) gate to the specified qubit."""
        ...

    def apply_rx(self, qubit: int, theta: float) -> None:
        """Applies a parameterized RX (X-rotation) gate.

        Args:
            qubit: Target qubit index
            theta: Rotation angle in radians
        """
        ...

    def apply_ry(self, qubit: int, theta: float) -> None:
        """Applies a parameterized RY (Y-rotation) gate.

        Args:
            qubit: Target qubit index
            theta: Rotation angle in radians
        """
        ...

    def apply_rz(self, qubit: int, theta: float) -> None:
        """Applies a parameterized RZ (Z-rotation) gate.

        Args:
            qubit: Target qubit index
            theta: Rotation angle in radians
        """
        ...

    def apply_phase(self, qubit: int, theta: float) -> None:
        """Applies a parameterized phase gate."""
        ...

    def apply_x2p(self, qubit: int) -> None:
        """Applies the √X (X/2 plus) gate to the specified qubit."""
        ...

    def apply_x2m(self, qubit: int) -> None:
        """Applies the √X† (X/2 minus) gate to the specified qubit."""
        ...

    def apply_y2p(self, qubit: int) -> None:
        """Applies the √Y (Y/2 plus) gate to the specified qubit."""
        ...

    def apply_y2m(self, qubit: int) -> None:
        """Applies the √Y† (Y/2 minus) gate to the specified qubit."""
        ...

    def apply_u(self, qubit: int, theta: float, phi: float, lambda_: float) -> None:
        """Applies a general single-qubit U gate.

        The U gate is defined as:
        U(θ, φ, λ) = Rz(φ) Ry(θ) Rz(λ)

        Args:
            qubit: Target qubit index
            theta: Rotation angle θ
            phi: Rotation angle φ
            lambda_: Rotation angle λ
        """
        ...

    def apply_gphase(self, phi: float) -> None:
        """Applies a global phase (has no observable effect on a density matrix)."""
        ...

    def apply_xy(self, qubit: int, theta: float) -> None:
        """Applies the XY gate.

        Args:
            qubit: Target qubit index
            theta: Rotation angle
        """
        ...

    def apply_xy2p(self, qubit: int, theta: float) -> None:
        """Applies the XY2P gate.

        Args:
            qubit: Target qubit index
            theta: Rotation angle
        """
        ...

    def apply_xy2m(self, qubit: int, theta: float) -> None:
        """Applies the XY2M gate.

        Args:
            qubit: Target qubit index
            theta: Rotation angle
        """
        ...

    def apply_rxy(self, qubit: int, theta: float, phi: float) -> None:
        """Applies a parameterized RXY rotation gate.

        Args:
            qubit: Target qubit index
            theta: Rotation angle θ
            phi: Rotation angle φ
        """
        ...

    def apply_swap(self, q0: int, q1: int) -> None:
        """Applies the SWAP gate between two qubits.

        Args:
            q0: First qubit index
            q1: Second qubit index
        """
        ...

    def apply_cx(self, control: int, target: int) -> None:
        """Applies the controlled-X (CNOT) gate.

        Args:
            control: Control qubit index
            target: Target qubit index
        """
        ...

    def apply_cy(self, control: int, target: int) -> None:
        """Applies the controlled-Y gate.

        Args:
            control: Control qubit index
            target: Target qubit index
        """
        ...

    def apply_cz(self, q0: int, q1: int) -> None:
        """Applies the controlled-Z gate.

        Args:
            q0: First qubit index
            q1: Second qubit index
        """
        ...

    def apply_crx(self, control: int, target: int, theta: float) -> None:
        """Applies the controlled-RX gate.

        Args:
            control: Control qubit index
            target: Target qubit index
            theta: Rotation angle in radians
        """
        ...

    def apply_cry(self, control: int, target: int, theta: float) -> None:
        """Applies the controlled-RY gate.

        Args:
            control: Control qubit index
            target: Target qubit index
            theta: Rotation angle in radians
        """
        ...

    def apply_crz(self, control: int, target: int, theta: float) -> None:
        """Applies the controlled-RZ gate.

        Args:
            control: Control qubit index
            target: Target qubit index
            theta: Rotation angle in radians
        """
        ...

    def apply_rxx(self, q0: int, q1: int, theta: float) -> None:
        """Applies the RXX (Ising XX) gate.

        Args:
            q0: First qubit index
            q1: Second qubit index
            theta: Rotation angle
        """
        ...

    def apply_ryy(self, q0: int, q1: int, theta: float) -> None:
        """Applies the RYY (Ising YY) gate.

        Args:
            q0: First qubit index
            q1: Second qubit index
            theta: Rotation angle
        """
        ...

    def apply_rzz(self, q0: int, q1: int, theta: float) -> None:
        """Applies the RZZ (Ising ZZ) gate.

        Args:
            q0: First qubit index
            q1: Second qubit index
            theta: Rotation angle
        """
        ...

    def apply_rzx(self, q0: int, q1: int, theta: float) -> None:
        """Applies the RZX gate.

        Args:
            q0: First qubit index
            q1: Second qubit index
            theta: Rotation angle
        """
        ...

    def apply_ccx(self, c0: int, c1: int, target: int) -> None:
        """Applies the CCX (Toffoli) gate.

        Args:
            c0: First control qubit index
            c1: Second control qubit index
            target: Target qubit index
        """
        ...

    def apply_fsim(self, q0: int, q1: int, theta: float, phi: float) -> None:
        """Applies the Fermionic Simulation (FSIM) gate.

        Args:
            q0: First qubit index
            q1: Second qubit index
            theta: iSWAP angle
            phi: Controlled-phase angle
        """
        ...

    def apply_single_qubit_gate(self, qubit: int, matrix: np.ndarray) -> None:
        """Applies a custom single-qubit gate.

        Args:
            qubit: Target qubit index
            matrix: 2x2 complex matrix as a NumPy array

        Raises:
            ValueError: If matrix is not 2x2
        """
        ...

    def apply_double_qubits_gate(self, q0: int, q1: int, matrix: np.ndarray) -> None:
        """Applies a custom two-qubit gate.

        Args:
            q0: First qubit index
            q1: Second qubit index
            matrix: 4x4 complex matrix as a NumPy array

        Raises:
            ValueError: If matrix is not 4x4
        """
        ...

    def apply_unitary_gate(self, qubits: List[int], matrix: np.ndarray) -> None:
        """Applies an arbitrary n-qubit unitary gate.

        The evolution is given by ρ → U ρ U†.

        Args:
            qubits: List of qubit indices the gate acts on
            matrix: The unitary matrix as a 2^n x 2^n NumPy array

        Raises:
            ValueError: If the matrix dimensions don't match qubit count
        """
        ...

    def apply_kraus(self, qubits: List[int], ops: List[np.ndarray]) -> None:
        """Applies a general quantum channel specified by Kraus operators.

        The evolution of the density matrix is given by ρ → Σ_k K_k ρ K_k†,
        where Σ_k K_k† K_k = I for a trace-preserving channel.

        Args:
            qubits: List of qubit indices the channel acts upon
            ops: A list of Kraus operators, where each operator is a flattened NumPy array

        Raises:
            ValueError: If the Kraus operators are invalid

        Examples:
            >>> import numpy as np
            >>> from cqlib.qis.state import DensityMatrix
            >>> # Depolarizing channel with p=0.1
            >>> p = 0.1
            >>> K0 = np.sqrt(1 - p) * np.eye(2, dtype=complex)
            >>> K1 = np.sqrt(p/3) * np.array([[0, 1], [1, 0]], dtype=complex)
            >>> K2 = np.sqrt(p/3) * np.array([[0, -1j], [1j, 0]], dtype=complex)
            >>> K3 = np.sqrt(p/3) * np.array([[1, 0], [0, -1]], dtype=complex)
            >>> dm = DensityMatrix(1)
            >>> dm.apply_kraus([0], [K0.flatten(), K1.flatten(), K2.flatten(), K3.flatten()])
        """
        ...

    def partial_trace(self, keep: List[int]) -> "DensityMatrix":
        """Computes the partial trace over a set of qubits.

        Reduces the N-qubit system to a smaller subsystem containing only the specified qubits
        by tracing out all other qubits.

        Args:
            keep: List of qubit indices to keep in the resulting reduced density matrix

        Returns:
            A new DensityMatrix representing the subsystem

        Raises:
            IndexError: If any qubit index is out of bounds.
            ValueError: If qubit indices contain duplicates.
        """
        ...

    def expectation(self, observable: Union[Hamiltonian, PauliString]) -> float:
        """Computes the expectation value of an observable.

        Calculates Tr(ρ * O) for the current density matrix ρ and a given observable O.

        Args:
            observable: The observable (Hamiltonian or PauliString)

        Returns:
            The expectation value as a real number

        Raises:
            ValueError: If the qubit counts don't match or the observable type is invalid
        """
        ...

    def __repr__(self) -> str:
        """Returns a string representation of the density matrix."""
        ...

    def copy(self) -> "DensityMatrix":
        """Returns a copy of this density matrix.

        Returns:
            A new DensityMatrix instance with the same data.
        """
        ...

    def is_hermitian(self, tol: float = 1e-10) -> bool:
        """Checks if the density matrix is Hermitian (self-adjoint) within a tolerance.

        A valid density matrix must satisfy ρ = ρ†, i.e., ρ_ij = ρ_ji*.

        Args:
            tol: Tolerance for floating-point comparison (default: 1e-10)

        Returns:
            True if the matrix is Hermitian within the specified tolerance.

        Examples:
            >>> from cqlib.qis import DensityMatrix
            >>> dm = DensityMatrix(1)
            >>> dm.apply_h(0)
            >>> dm.is_hermitian()
            True
        """
        ...

    def is_positive_semidefinite(self, tol: float = 1e-10) -> bool:
        """Checks if the density matrix is positive semidefinite.

        Computes eigenvalues via a self-adjoint eigenvalue decomposition and
        checks that every eigenvalue satisfies λᵢ ≥ -tol. For Hermitian matrices
        this check is definitive; non-Hermitian inputs (or matrices containing
        NaN/Inf) return ``False``.

        Args:
            tol: Tolerance for floating-point comparison (default: 1e-10)

        Returns:
            True if the matrix satisfies the positive semidefinite condition.

        Examples:
            >>> from cqlib.qis import DensityMatrix
            >>> dm = DensityMatrix(1)
            >>> dm.is_positive_semidefinite()
            True
        """
        ...

    def validate_physical(self, tol: float = 1e-10) -> None:
        """Validates all physical constraints of the density matrix.

        Checks:
        1. Hermiticity: ρ = ρ†
        2. Positive semidefiniteness: All eigenvalues >= 0
        3. Unit trace: Tr(ρ) = 1

        Args:
            tol: Tolerance for floating-point comparisons (default: 1e-10)

        Raises:
            ValueError: If any physical constraint is violated (not Hermitian,
                       not positive semidefinite, or trace not equal to 1).

        Examples:
            >>> from cqlib.qis import DensityMatrix
            >>> dm = DensityMatrix(1)
            >>> dm.apply_h(0)
            >>> dm.validate_physical()  # Should pass for valid states
        """
        ...

    def reset(self, qubit: int) -> None:
        """Resets the specified qubit to the |0⟩ state by measuring and flipping if 1.

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def measure(self, qubit: int) -> bool:
        """Measures one qubit and collapses the state."""
        ...

    def measure_all(self) -> Outcome:
        """Measures all qubits and collapses the state."""
        ...

    def sample_shots(self, shots: int) -> List[Outcome]:
        """Samples measurement outcomes without mutating this state."""
        ...

    def sample(self, measurement: Measurement, shots: int) -> ExecutionResult:
        """Samples measurement outcomes according to a circuit measurement receipt."""
        ...

    def probs(self, measurement: Measurement) -> Dict[Outcome, float]:
        """Returns marginal probabilities according to a circuit measurement receipt."""
        ...
