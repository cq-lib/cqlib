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
from cqlib.qis import PauliString, Hamiltonian

@final
class Statevector:
    """Quantum statevector representing a pure quantum state.

    A statevector describes the quantum state of an N-qubit system as a vector
    of 2^N complex amplitudes. The state |ψ⟩ = Σᵢ αᵢ|i⟩ is stored with αᵢ
    as the amplitude for basis state |i⟩ (i in binary representation).

    # Memory Layout
    The data uses contiguous memory layout (compatible with C/NumPy),
    where the amplitude at index `i` corresponds to basis state |i⟩ with
    qubit indices mapping to bits from least significant (qubit 0) to most.

    # Example
    ```python
    from cqlib.qis.state import Statevector

    # Create a 2-qubit state in |00⟩
    sv = Statevector(2)

    # Apply gates to create Bell state
    sv.apply_h(0)
    sv.apply_cx(0, 1)

    # Get probabilities
    probs = sv.probabilities()
    print(probs)  # [0.5, 0.0, 0.0, 0.5]
    ```
    """

    def __new__(cls, num_qubits: int) -> "Statevector":
        """Creates a new statevector initialized to the |0...0⟩ state.

        The statevector represents the quantum state as a vector of 2^N complex amplitudes,
        where N is the number of qubits. All amplitudes are initialized to zero except
        the first element (|0...0⟩) which is set to 1.0.

        Args:
            num_qubits: Number of qubits in the system

        Returns:
            A new Statevector instance in the ground state

        Examples:
            >>> sv = Statevector(2)  # |00⟩ state
        """
        ...

    @staticmethod
    def from_state(
        num_qubits: int, initial_state: Union[np.ndarray, List[complex]]
    ) -> "Statevector":
        """Creates a statevector from initial amplitudes.

        Args:
            num_qubits: Number of qubits in the system
            initial_state: NumPy array of 2^N complex amplitudes, or a list of complex numbers

        Returns:
            A new Statevector instance

        Raises:
            ValueError: If the state length doesn't match 2^num_qubits or state is not normalized

        Examples:
            >>> import numpy as np
            >>> # Create |+⟩ state for 1 qubit
            >>> amps = np.array([1/np.sqrt(2), 1/np.sqrt(2)], dtype=complex)
            >>> sv = Statevector.from_state(1, amps)
        """
        ...

    @staticmethod
    def from_circuit(circuit: Circuit) -> "Statevector":
        """Creates a statevector by simulating a quantum circuit.

        Executes the circuit gates sequentially to evolve the initial |0...0⟩ state.

        Args:
            circuit: The quantum circuit to simulate

        Returns:
            A new Statevector instance after circuit execution

        Raises:
            ValueError: If the circuit contains non-unitary operations

        Examples:
            >>> from cqlib import Circuit
            >>> from cqlib.qis.state import Statevector
            >>> circuit = Circuit(2)
            >>> circuit.h(0)
            >>> circuit.cx(0, 1)
            >>> sv = Statevector.from_circuit(circuit)
        """
        ...

    def apply_circuit(self, circuit: Circuit) -> None:
        """Applies a circuit to this statevector in place."""
        ...

    @property
    def num_qubits(self) -> int:
        """Returns the number of qubits in the statevector."""
        ...

    @property
    def data(self) -> np.ndarray:
        """Returns the statevector amplitudes as a NumPy array.

        Returns:
            A 1D NumPy array of complex amplitudes with length 2^num_qubits.
        """
        ...

    def probabilities(self) -> List[float]:
        """Returns the measurement probabilities for all basis states.

        Computes p(i) = |αᵢ|² for each basis state |i⟩.

        Returns:
            A list of probabilities (floats) with length 2^num_qubits.
        """
        ...

    def apply_standard_gate(
        self,
        gate: StandardGate,
        qubits: List[int],
        params: Optional[List[float]] = None,
    ) -> None:
        """Applies a standard gate to the statevector.

        Args:
            gate: The standard gate to apply.
            qubits: List of target qubit indices.
            params: List of parameters for parameterized gates.
        """
        ...

    def apply_pauli_rotation(self, pauli: PauliString, theta: float) -> None:
        """Applies ``exp(-i * theta / 2 * P)`` using a fused native kernel.

        Args:
            pauli: Hermitian Pauli string with the same number of qubits.
            theta: Rotation angle in radians.
        """
        ...

    def apply_x(self, qubit: int) -> None:
        """Applies the Pauli-X (NOT) gate to the specified qubit.

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_y(self, qubit: int) -> None:
        """Applies the Pauli-Y gate to the specified qubit.

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_z(self, qubit: int) -> None:
        """Applies the Pauli-Z gate to the specified qubit.

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_h(self, qubit: int) -> None:
        """Applies the Hadamard gate to the specified qubit.

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_s(self, qubit: int) -> None:
        """Applies the S (phase) gate to the specified qubit.

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_sdg(self, qubit: int) -> None:
        """Applies the S† (S-dagger) gate to the specified qubit.

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_t(self, qubit: int) -> None:
        """Applies the T gate to the specified qubit.

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_tdg(self, qubit: int) -> None:
        """Applies the T† (T-dagger) gate to the specified qubit.

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_rx(self, qubit: int, theta: float) -> None:
        """Applies a parameterized RX (X-rotation) gate.

        Args:
            qubit: Target qubit index
            theta: Rotation angle in radians

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_ry(self, qubit: int, theta: float) -> None:
        """Applies a parameterized RY (Y-rotation) gate.

        Args:
            qubit: Target qubit index
            theta: Rotation angle in radians

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_rz(self, qubit: int, theta: float) -> None:
        """Applies a parameterized RZ (Z-rotation) gate.

        Args:
            qubit: Target qubit index
            theta: Rotation angle in radians

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_phase(self, qubit: int, theta: float) -> None:
        """Applies a parameterized phase gate."""
        ...

    def apply_x2p(self, qubit: int) -> None:
        """Applies the √X (X/2 plus) gate to the specified qubit.

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_x2m(self, qubit: int) -> None:
        """Applies the √X† (X/2 minus) gate to the specified qubit.

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_y2p(self, qubit: int) -> None:
        """Applies the √Y (Y/2 plus) gate to the specified qubit.

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_y2m(self, qubit: int) -> None:
        """Applies the √Y† (Y/2 minus) gate to the specified qubit.

        Raises:
            IndexError: If qubit index is out of bounds.
        """
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

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_gphase(self, phi: float) -> None:
        """Applies a global phase to the statevector."""
        ...

    def apply_swap(self, q0: int, q1: int) -> None:
        """Applies the SWAP gate between two qubits.

        Raises:
            IndexError: If either qubit index is out of bounds.
            ValueError: If q0 and q1 are the same qubit.
        """
        ...

    def apply_cx(self, control: int, target: int) -> None:
        """Applies the controlled-X (CNOT) gate.

        Args:
            control: Control qubit index
            target: Target qubit index

        Raises:
            IndexError: If either qubit index is out of bounds.
            ValueError: If control and target are the same qubit.
        """
        ...

    def apply_cy(self, control: int, target: int) -> None:
        """Applies the controlled-Y gate.

        Args:
            control: Control qubit index
            target: Target qubit index

        Raises:
            IndexError: If either qubit index is out of bounds.
            ValueError: If control and target are the same qubit.
        """
        ...

    def apply_cz(self, q0: int, q1: int) -> None:
        """Applies the controlled-Z gate.

        Args:
            q0: First qubit index
            q1: Second qubit index

        Raises:
            IndexError: If either qubit index is out of bounds.
            ValueError: If q0 and q1 are the same qubit.
        """
        ...

    def apply_crx(self, control: int, target: int, theta: float) -> None:
        """Applies the controlled-RX gate.

        Raises:
            IndexError: If either qubit index is out of bounds.
            ValueError: If control and target are the same qubit.
        """
        ...

    def apply_cry(self, control: int, target: int, theta: float) -> None:
        """Applies the controlled-RY gate.

        Raises:
            IndexError: If either qubit index is out of bounds.
            ValueError: If control and target are the same qubit.
        """
        ...

    def apply_crz(self, control: int, target: int, theta: float) -> None:
        """Applies the controlled-RZ gate.

        Raises:
            IndexError: If either qubit index is out of bounds.
            ValueError: If control and target are the same qubit.
        """
        ...

    def apply_rxx(self, q0: int, q1: int, theta: float) -> None:
        """Applies the RXX (Ising XX) gate.

        Args:
            q0: First qubit index
            q1: Second qubit index
            theta: Rotation angle

        Raises:
            IndexError: If either qubit index is out of bounds.
            ValueError: If q0 and q1 are the same qubit.
        """
        ...

    def apply_ryy(self, q0: int, q1: int, theta: float) -> None:
        """Applies the RYY (Ising YY) gate.

        Raises:
            IndexError: If either qubit index is out of bounds.
            ValueError: If q0 and q1 are the same qubit.
        """
        ...

    def apply_rzz(self, q0: int, q1: int, theta: float) -> None:
        """Applies the RZZ (Ising ZZ) gate.

        Raises:
            IndexError: If either qubit index is out of bounds.
            ValueError: If q0 and q1 are the same qubit.
        """
        ...

    def apply_rzx(self, q0: int, q1: int, theta: float) -> None:
        """Applies the RZX gate.

        Raises:
            IndexError: If either qubit index is out of bounds.
            ValueError: If q0 and q1 are the same qubit.
        """
        ...

    def apply_xy(self, qubit: int, theta: float) -> None:
        """Applies the XY gate.

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_xy2p(self, qubit: int, theta: float) -> None:
        """Applies the XY2P gate: Rz(θ - π/2) Ry(π/2) Rz(π/2 - θ).

        Args:
            qubit: Target qubit index
            theta: Rotation angle

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_xy2m(self, qubit: int, theta: float) -> None:
        """Applies the XY2M gate: Rz(-θ + π/2) Ry(-π/2) Rz(-π/2 + θ).

        Args:
            qubit: Target qubit index
            theta: Rotation angle

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_rxy(self, qubit: int, theta: float, phi: float) -> None:
        """Applies the RXY gate.

        Args:
            qubit: Target qubit index
            theta: Rotation angle
            phi: Rotation axis angle in XY plane

        Raises:
            IndexError: If qubit index is out of bounds.
        """
        ...

    def apply_fsim(self, q0: int, q1: int, theta: float, phi: float) -> None:
        """Applies Fermionic Simulation (fSim) gate.

        Native gate for superconducting qubits. Combines iSWAP and controlled-phase.

        Args:
            q0: First qubit index
            q1: Second qubit index
            theta: iSWAP angle
            phi: Controlled-phase angle

        Raises:
            IndexError: If either qubit index is out of bounds.
            ValueError: If q0 and q1 are the same qubit.
        """
        ...

    def apply_ccx(self, c0: int, c1: int, target: int) -> None:
        """Applies the CCX (Toffoli) gate.

        Args:
            c0: First control qubit index
            c1: Second control qubit index
            target: Target qubit index

        Raises:
            IndexError: If any qubit index is out of bounds.
            ValueError: If any qubits are duplicated.
        """
        ...

    def apply_single_qubit_gate(self, qubit: int, matrix: np.ndarray) -> None:
        """Applies a custom single-qubit gate.

        Args:
            qubit: Target qubit index
            matrix: 2x2 complex matrix as a NumPy array

        Raises:
            IndexError: If qubit index is out of bounds.
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
            IndexError: If either qubit index is out of bounds.
            ValueError: If q0 and q1 are the same qubit or matrix is not 4x4
        """
        ...

    def apply_unitary_gate(self, qubits: List[int], matrix: np.ndarray) -> None:
        """Applies an arbitrary n-qubit unitary gate."""
        ...

    def expectation(self, observable: Union[Hamiltonian, PauliString]) -> float:
        """Computes the expectation value of an observable.

        Calculates ⟨ψ|O|ψ⟩ for the current state |ψ⟩ and a given observable O.

        Args:
            observable: The observable (Hamiltonian or PauliString)

        Returns:
            The expectation value as a real number

        Raises:
            ValueError: If the qubit counts don't match or the observable type is invalid
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

    def __repr__(self) -> str:
        """Returns a string representation of the statevector."""
        ...

    def copy(self) -> "Statevector":
        """Returns a copy of this statevector.

        Returns:
            A new Statevector instance with the same amplitudes.
        """
        ...
