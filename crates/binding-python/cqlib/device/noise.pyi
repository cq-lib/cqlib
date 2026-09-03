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

import numpy as np
from cqlib.circuit import StandardGate, Qubit
from cqlib.qis import Pauli
from .qubit import PhysicalQubit

_PhysicalQubitLike = int | Qubit | PhysicalQubit

class SingleQubitNoise:
    """
    Single-qubit quantum noise channel.

    Represents various types of single-qubit noise that can occur in quantum
    systems, including bit-flip, phase-flip, depolarizing, and amplitude/phase
    damping channels.

    # Example

    ```python
    from cqlib.device import SingleQubitNoise
    import numpy as np

    # Create depolarizing noise with 0.1% error probability
    noise = SingleQubitNoise.depolarizing(p=0.001)

    # Get Kraus operators for simulation
    kraus_ops = noise.to_kraus()  # List of 2x2 NumPy arrays

    # Validate noise parameters
    assert noise.is_valid()  # True if probabilities are in [0, 1]
    ```
    """

    @staticmethod
    def bit_flip(p: float) -> "SingleQubitNoise":
        """
        Creates a bit-flip noise channel.
        Kraus operators: E₀ = √(1-p) I, E₁ = √p X

        Args:
            p: Bit-flip probability in range [0.0, 1.0]

        Raises:
            ValueError: If ``p`` is not finite or outside [0, 1].
        """
        ...

    @staticmethod
    def phase_flip(p: float) -> "SingleQubitNoise":
        """
        Creates a phase-flip noise channel.
        Kraus operators: E₀ = √(1-p) I, E₁ = √p Z

        Args:
            p: Phase-flip probability in range [0.0, 1.0]

        Raises:
            ValueError: If ``p`` is not finite or outside [0, 1].
        """
        ...

    @staticmethod
    def pauli(px: float, py: float, pz: float) -> "SingleQubitNoise":
        """
        Creates a general Pauli noise channel.
        Kraus operators include √(1-px-py-pz) I, √px X, √py Y, √pz Z.

        Args:
            px: Probability of X error
            py: Probability of Y error
            pz: Probability of Z error

        Raises:
            ValueError: If a probability is not finite or outside [0, 1], or
                if ``px + py + pz`` exceeds 1.
        """
        ...

    @staticmethod
    def depolarizing(p: float) -> "SingleQubitNoise":
        """
        Creates a depolarizing noise channel.
        With probability p, applies a random Pauli error (X, Y, or Z).

        Args:
            p: Total depolarization probability in range [0.0, 1.0]

        Raises:
            ValueError: If ``p`` is not finite or outside [0, 1].
        """
        ...

    @staticmethod
    def amplitude_damping(gamma: float) -> "SingleQubitNoise":
        """
        Creates an amplitude damping channel.
        Models energy relaxation (T1) where excited states decay to ground state.

        Args:
            gamma: Damping parameter in range [0.0, 1.0]

        Raises:
            ValueError: If ``gamma`` is not finite or outside [0, 1].
        """
        ...

    @staticmethod
    def phase_damping(lambda_: float) -> "SingleQubitNoise":
        """
        Creates a phase damping channel.
        Models pure dephasing (T2) without energy relaxation.

        Args:
            lambda_: Phase damping parameter in range [0.0, 1.0]

        Raises:
            ValueError: If ``lambda_`` is not finite or outside [0, 1].
        """
        ...

    def is_valid(self) -> bool:
        """
        Validates that noise parameters are physically valid.
        Returns `True` if all probabilities are in valid ranges.
        """
        ...

    def to_kraus(self) -> list[np.ndarray]:
        """
        Returns the Kraus operators as NumPy arrays.

        Returns:
            list[np.ndarray]: List of 2x2 complex NumPy arrays representing the Kraus operators.
        """
        ...

    def __copy__(self) -> "SingleQubitNoise": ...
    def __deepcopy__(self, memo: dict) -> "SingleQubitNoise": ...

class TwoQubitNoise:
    """
    Two-qubit quantum noise channel.

    Represents noise affecting pairs of qubits, including depolarizing noise
    and correlated Pauli errors.

    # Example

    ```python
    from cqlib.device import SingleQubitNoise, TwoQubitNoise

    # Depolarizing noise with 1% total error probability
    noise = TwoQubitNoise.depolarizing(p=0.01)

    # Independent noise on each qubit
    q0_noise = SingleQubitNoise.depolarizing(0.001)
    q1_noise = SingleQubitNoise.depolarizing(0.001)
    independent = TwoQubitNoise.independent(q0_noise, q1_noise)
    ```
    """

    @staticmethod
    def depolarizing(p: float) -> "TwoQubitNoise":
        """
        Creates a two-qubit depolarizing noise channel.
        With probability p, applies a random Pauli error from the 15 non-identity
        Pauli operators.

        Args:
            p: Total depolarization probability in range [0.0, 1.0]

        Raises:
            ValueError: If ``p`` is not finite or outside [0, 1].
        """
        ...

    @staticmethod
    def independent(
        q0_noise: SingleQubitNoise, q1_noise: SingleQubitNoise
    ) -> "TwoQubitNoise":
        """
        Creates independent single-qubit noise on both qubits.

        Args:
            q0_noise: Noise channel for the first qubit
            q1_noise: Noise channel for the second qubit
        """
        ...

    @staticmethod
    def correlated_pauli(op_q0: Pauli, op_q1: Pauli, p: float) -> "TwoQubitNoise":
        """
        Creates correlated Pauli noise on two qubits.

        Args:
            op_q0: Pauli operator for the first qubit.
            op_q1: Pauli operator for the second qubit.
            p: Correlation probability in range [0.0, 1.0].

        Raises:
            ValueError: If ``p`` is not finite or outside [0, 1].
        """
        ...

    def is_valid(self) -> bool:
        """Validates that noise parameters are physically valid."""
        ...

    def to_kraus(self) -> list[np.ndarray]:
        """
        Returns the Kraus operators as NumPy arrays.

        Returns:
            list[np.ndarray]: List of 4x4 complex NumPy arrays representing the Kraus operators.
        """
        ...

    @property
    def kind(self) -> str:
        """Returns the noise channel type."""
        ...

    def __copy__(self) -> "TwoQubitNoise": ...
    def __deepcopy__(self, memo: dict) -> "TwoQubitNoise": ...

class ReadoutError:
    """
    Asymmetric readout error model.

    Represents measurement errors where the probabilities of false 0 and false 1
    may differ.

    # State Discrimination

    - `p_0_given_1`: Probability of measuring 0 when state was |1⟩ (false negative)
    - `p_1_given_0`: Probability of measuring 1 when state was |0⟩ (false positive)
    """

    def __init__(self, p_0_given_1: float, p_1_given_0: float) -> None:
        """
        Creates a new readout error model.

        Args:
            p_0_given_1: Probability of measuring 0 given state was prepared in 1
            p_1_given_0: Probability of measuring 1 given state was prepared in 0

        Raises:
            ValueError: If either probability is not finite or outside [0, 1].
        """
        ...

    @property
    def p_0_given_1(self) -> float:
        """Returns P(meas 0 | prep 1), the false-negative probability."""
        ...

    @property
    def p_1_given_0(self) -> float:
        """Returns P(meas 1 | prep 0), the false-positive probability."""
        ...

    def is_valid(self) -> bool:
        """Returns `True` if both probabilities are in [0.0, 1.0]."""
        ...

    def __copy__(self) -> "ReadoutError": ...
    def __deepcopy__(self, memo: dict) -> "ReadoutError": ...

class OperationKey:
    """
    Key for looking up noise parameters in a noise model.

    Uniquely identifies a gate operation on specific qubits for noise lookup.
    """

    @staticmethod
    def new_single(gate: StandardGate, q0: _PhysicalQubitLike) -> "OperationKey":
        """
        Creates a key for a single-qubit operation.

        Args:
            gate: The quantum gate
            q0: The target qubit

        """
        ...

    @staticmethod
    def new_double(
        gate: StandardGate, q0: _PhysicalQubitLike, q1: _PhysicalQubitLike
    ) -> "OperationKey":
        """
        Creates a key for a two-qubit operation.

        Args:
            gate: The quantum gate
            q0: First qubit (typically control)
            q1: Second qubit (typically target)

        Raises:
            ValueError: If the qubits are duplicated.
        """
        ...

    @staticmethod
    def new_triple(
        gate: StandardGate,
        q0: _PhysicalQubitLike,
        q1: _PhysicalQubitLike,
        q2: _PhysicalQubitLike,
    ) -> "OperationKey":
        """
        Creates a key for a three-qubit operation.

        Args:
            gate: The quantum gate
            q0: First qubit
            q1: Second qubit
            q2: Third qubit

        Raises:
            ValueError: If any qubits are duplicated.
        """
        ...

    @property
    def qubits(self) -> list[int]:
        """Returns the qubit indices involved in this operation."""
        ...

    @property
    def gate(self) -> StandardGate:
        """Returns the gate type.

        .. note::

           ``OperationKey`` only stores the gate type, not parameters.
           Parametric gates (e.g., ``RX``, ``RY``, ``U``) will have
           their parameters filled with zeros.
        """
        ...

    def __hash__(self) -> int: ...
    def __eq__(self, value: object) -> bool: ...
    def __copy__(self) -> "OperationKey": ...
    def __deepcopy__(self, memo: dict) -> "OperationKey": ...

class NoiseModel:
    """
    Complete noise model for a quantum device.

    Aggregates all noise sources: readout errors, single-qubit gate errors,
    and two-qubit gate errors. Used by noise-aware compilers and simulators.

    # Example

    ```python
    from cqlib.device import NoiseModel, SingleQubitNoise, TwoQubitNoise
    from cqlib.circuit import StandardGate

    model = NoiseModel()

    # Add noise to all H gates on qubit 0
    model.add_single_qubit_error(
        StandardGate.H, 0,
        SingleQubitNoise.depolarizing(0.001)
    )

    # Add noise to CX gates between qubits 0 and 1
    model.add_two_qubit_error(
        StandardGate.CX, 0, 1,
        TwoQubitNoise.depolarizing(0.01)
    )
    ```
    """

    def __init__(self) -> None:
        """Creates an empty noise model."""
        ...

    def add_readout_error(self, qubit: _PhysicalQubitLike, error: ReadoutError) -> None:
        """
        Adds a readout error for a specific qubit.

        Args:
            qubit: The qubit index
            error: The readout error model
        """
        ...

    def add_single_qubit_error(
        self, gate: StandardGate, qubit: _PhysicalQubitLike, noise: SingleQubitNoise
    ) -> None:
        """
        Adds single-qubit noise to a gate on a specific qubit.

        Args:
            gate: The quantum gate
            qubit: The target qubit
            noise: The noise channel
        """
        ...

    def add_two_qubit_error(
        self,
        gate: StandardGate,
        q0: _PhysicalQubitLike,
        q1: _PhysicalQubitLike,
        noise: TwoQubitNoise,
    ) -> None:
        """
        Adds two-qubit noise to a gate on specific qubits.

        Args:
            gate: The quantum gate
            q0: First qubit (typically control)
            q1: Second qubit (typically target)
            noise: The noise channel
        """
        ...

    def get_readout_error(self, qubit: _PhysicalQubitLike) -> ReadoutError | None:
        """Returns the readout error for a qubit, if any."""
        ...

    def get_single_qubit_errors(
        self, key: OperationKey
    ) -> list[SingleQubitNoise] | None:
        """Returns all single-qubit noise channels for an operation."""
        ...

    def get_two_qubit_errors(self, key: OperationKey) -> list[TwoQubitNoise] | None:
        """Returns all two-qubit noise channels for an operation."""
        ...

    def __copy__(self) -> "NoiseModel": ...
    def __deepcopy__(self, memo: dict) -> "NoiseModel": ...
