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

from typing import Dict, List, Optional, Union, final
import numpy as np

from cqlib.circuit import Circuit, Measurement
from cqlib.circuit.gates.standard import StandardGate
from cqlib.device import ExecutionResult, NoiseModel, Outcome
from cqlib.qis import Hamiltonian, PauliString

@final
class DensityMatrixNoise:
    """A density matrix quantum simulator with noise modeling capabilities.

    This simulator wraps the `DensityMatrix` kernel and automatically applies
    Kraus operator noise after each quantum gate based on a configurable
    `NoiseModel`. It supports both interactive gate-by-gate simulation and
    batch circuit execution.

    # Example
    ```python
    from cqlib.qis.state import DensityMatrixNoise
    from cqlib.device import NoiseModel, SingleQubitNoise
    from cqlib.circuit import StandardGate

    # Create noise model with bit-flip noise on X gates
    noise_model = NoiseModel()
    noise = SingleQubitNoise.bit_flip(p=0.01)
    noise_model.add_single_qubit_error(StandardGate.X, 0, noise)

    # Create simulator and apply noisy gate
    sim = DensityMatrixNoise(1, noise_model)
    sim.apply_x(0)

    # Get probabilities (P(|1⟩) ~ 0.99 due to 1% bit-flip noise)
    probs = sim.probabilities()
    ```
    """

    def __new__(
        cls, num_qubits: int, noise_model: Optional[NoiseModel] = None
    ) -> "DensityMatrixNoise":
        """Creates a new simulator with the specified number of qubits and optional noise model.

        Args:
            num_qubits: The number of qubits in the quantum system
            noise_model: Optional NoiseModel defining gate and readout errors

        Returns:
            A new DensityMatrixNoise instance

        Examples:
            >>> from cqlib.qis.state import DensityMatrixNoise
            >>> # Simulator without noise (ideal simulation)
            >>> sim = DensityMatrixNoise(3)
            >>> # Simulator with noise model
            >>> from cqlib.device import NoiseModel
            >>> sim = DensityMatrixNoise(2, NoiseModel())
        """
        ...

    @staticmethod
    def from_circuit(
        circuit: Circuit,
        noise_model: Optional[NoiseModel] = None,
    ) -> "DensityMatrixNoise":
        """Simulates a circuit, applying noise after each operation.

        The circuit is decomposed into basis gates before execution. Noise is
        applied according to the noise model immediately following each gate.

        Args:
            circuit: The quantum circuit to simulate
            noise_model: Optional NoiseModel for noise injection

        Returns:
            A new DensityMatrixNoise instance after circuit execution

        Raises:
            ValueError: If the circuit contains unsupported operations
        """
        ...

    def apply_circuit(self, circuit: Circuit) -> None:
        """Applies a circuit to this simulator in place."""
        ...

    @property
    def num_qubits(self) -> int:
        """Returns the number of qubits in the simulator."""
        ...

    @property
    def state(self) -> np.ndarray:
        """Returns the underlying density matrix state as a 2D NumPy array.

        Returns:
            A 2D NumPy array of complex numbers with shape (2^num_qubits, 2^num_qubits).
        """
        ...

    def probabilities(self) -> List[float]:
        """Returns the ideal measurement probabilities without readout noise."""
        ...

    def probabilities_with_readout(self, qubits: List[int]) -> List[float]:
        """Computes measurement probabilities with readout error modeling.

        Args:
            qubits: Indices of qubits to measure

        Returns:
            A vector of probabilities for all 2^n computational basis states.
        """
        ...

    def apply_standard_gate_noise(
        self,
        gate: StandardGate,
        qubits: List[int],
        params: Optional[List[float]] = None,
    ) -> None:
        """Applies a standard gate to the density matrix, automatically applying any associated noise.

        Args:
            gate: The standard gate to apply.
            qubits: List of target qubit indices.
            params: List of parameters for parameterized gates.
        """
        ...

    def apply_x(self, q: int) -> None:
        """Applies the Pauli-X gate with optional noise."""
        ...

    def apply_y(self, q: int) -> None:
        """Applies the Pauli-Y gate with optional noise."""
        ...

    def apply_z(self, q: int) -> None:
        """Applies the Pauli-Z gate with optional noise."""
        ...

    def apply_h(self, q: int) -> None:
        """Applies the Hadamard gate with optional noise."""
        ...

    def apply_s(self, q: int) -> None:
        """Applies the S gate with optional noise."""
        ...

    def apply_sdg(self, q: int) -> None:
        """Applies the S dagger gate with optional noise."""
        ...

    def apply_t(self, q: int) -> None:
        """Applies the T gate with optional noise."""
        ...

    def apply_tdg(self, q: int) -> None:
        """Applies the T dagger gate with optional noise."""
        ...

    def apply_rx(self, q: int, theta: float) -> None:
        """Applies a rotation around the X-axis with optional noise."""
        ...

    def apply_ry(self, q: int, theta: float) -> None:
        """Applies a rotation around the Y-axis with optional noise."""
        ...

    def apply_rz(self, q: int, theta: float) -> None:
        """Applies a rotation around the Z-axis with optional noise."""
        ...

    def apply_phase(self, q: int, theta: float) -> None:
        """Applies the phase gate with optional noise."""
        ...

    def apply_gphase(self, phi: float) -> None:
        """Applies the global phase gate with optional noise."""
        ...

    def apply_x2p(self, q: int) -> None:
        """Applies the X2P gate with optional noise."""
        ...

    def apply_x2m(self, q: int) -> None:
        """Applies the X2M gate with optional noise."""
        ...

    def apply_y2p(self, q: int) -> None:
        """Applies the Y2P gate with optional noise."""
        ...

    def apply_y2m(self, q: int) -> None:
        """Applies the Y2M gate with optional noise."""
        ...

    def apply_rxy(self, q: int, theta: float, phi: float) -> None:
        """Applies an arbitrary rotation on the Bloch sphere with optional noise."""
        ...

    def apply_xy(self, q: int, theta: float) -> None:
        """Applies the XY gate with optional noise."""
        ...

    def apply_xy2p(self, q: int, theta: float) -> None:
        """Applies the XY2P gate with optional noise."""
        ...

    def apply_xy2m(self, q: int, theta: float) -> None:
        """Applies the XY2M gate with optional noise."""
        ...

    def apply_u(self, q: int, theta: float, phi: float, lambda_: float) -> None:
        """Applies a general single-qubit U gate with optional noise."""
        ...

    def apply_cx(self, control: int, target: int) -> None:
        """Applies the Controlled-X gate with optional noise."""
        ...

    def apply_cy(self, control: int, target: int) -> None:
        """Applies the Controlled-Y gate with optional noise."""
        ...

    def apply_cz(self, q0: int, q1: int) -> None:
        """Applies the Controlled-Z gate with optional noise."""
        ...

    def apply_swap(self, q0: int, q1: int) -> None:
        """Applies the SWAP gate with optional noise."""
        ...

    def apply_rxx(self, q0: int, q1: int, theta: float) -> None:
        """Applies the RXX gate with optional noise."""
        ...

    def apply_ryy(self, q0: int, q1: int, theta: float) -> None:
        """Applies the RYY gate with optional noise."""
        ...

    def apply_rzz(self, q0: int, q1: int, theta: float) -> None:
        """Applies the RZZ gate with optional noise."""
        ...

    def apply_rzx(self, q0: int, q1: int, theta: float) -> None:
        """Applies the RZX gate with optional noise."""
        ...

    def apply_crx(self, control: int, target: int, theta: float) -> None:
        """Applies the Controlled-RX gate with optional noise."""
        ...

    def apply_cry(self, control: int, target: int, theta: float) -> None:
        """Applies the Controlled-RY gate with optional noise."""
        ...

    def apply_crz(self, control: int, target: int, theta: float) -> None:
        """Applies the Controlled-RZ gate with optional noise."""
        ...

    def apply_fsim(self, q0: int, q1: int, theta: float, phi: float) -> None:
        """Applies the fSim gate with optional noise."""
        ...

    def apply_ccx(self, c1: int, c2: int, t: int) -> None:
        """Applies the Toffoli gate with optional noise."""
        ...

    def apply_unitary_gate(self, qubits: List[int], matrix: np.ndarray) -> None:
        """Applies an arbitrary unitary gate to the state.

        Note: No noise is applied for generic unitary gates.

        Args:
            qubits: Qubit indices the gate acts on
            matrix: Unitary matrix as a 2D NumPy array

        Raises:
            ValueError: If the matrix dimensions don't match qubit count
        """
        ...

    def expectation(self, observable: Union[Hamiltonian, PauliString]) -> float:
        """Computes the expectation value of an observable.

        Args:
            observable: The observable (Hamiltonian or PauliString)

        Returns:
            The expectation value as a real number

        Raises:
            ValueError: If the qubit counts don't match or the observable type is invalid
        """
        ...

    def __repr__(self) -> str:
        """Returns a string representation of the simulator."""
        ...

    def copy(self) -> "DensityMatrixNoise":
        """Returns a copy of this simulator."""
        ...

    def measure(self, qubit: int) -> bool:
        """Measures one qubit and collapses the state."""
        ...

    def measure_all(self) -> Outcome:
        """Measures all qubits and collapses the state."""
        ...

    def sample_shots(self, shots: int) -> List[Outcome]:
        """Samples measurement outcomes with readout noise, without mutating this state."""
        ...

    def sample(self, measurement: Measurement, shots: int) -> ExecutionResult:
        """Samples measurement outcomes according to a circuit measurement receipt."""
        ...

    def probs(self, measurement: Measurement) -> Dict[Outcome, float]:
        """Returns ideal marginal probabilities according to a circuit measurement receipt."""
        ...

    def probs_with_readout(self, measurement: Measurement) -> Dict[Outcome, float]:
        """Returns marginal probabilities with readout noise applied."""
        ...
