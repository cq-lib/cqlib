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

from datetime import datetime
from cqlib.circuit import Circuit, Instruction, Qubit, StandardGate, ValueOperation
from .qubit import PhysicalQubit
from .topology import Topology

_PhysicalQubitLike = int | Qubit | PhysicalQubit

class InstructionProp:
    """Calibration data for a quantum gate executed on specific qubits.

    Stores the gate's error rate (infidelity) and optionally its execution
    duration (e.g., gate length in nanoseconds). Used by the compiler for
    noise-aware scheduling and fidelity estimation.

    Key Usage::

        >>> from cqlib.device import InstructionProp
        >>> from cqlib.circuit import Instruction, StandardGate
        >>>
        >>> # H gate with 0.1% error rate
        >>> h = Instruction.from_standard_gate(StandardGate.H)
        >>> prop = InstructionProp(h, error_rate=0.001)
        >>>
        >>> # Optionally set gate duration
        >>> prop.length = 35.0  # 35 ns
        >>>
        >>> prop.instruction    # Instruction(StandardGate.H)
        >>> prop.error_rate     # 0.001
        >>> prop.length         # 35.0
    """

    def __init__(self, instruction: Instruction, error_rate: float) -> None:
        """Creates calibration data for a given instruction.

        Args:
            instruction: The quantum instruction, for example one created by
                ``Instruction.from_standard_gate(StandardGate.H)``.
            error_rate: The error rate (infidelity) in range [0.0, 1.0].

        Raises:
            ValueError: If ``error_rate`` is not finite or outside [0, 1].
        """

    @property
    def instruction(self) -> Instruction:
        """The quantum instruction associated with these properties."""

    @instruction.setter
    def instruction(self, instruction: Instruction) -> None:
        """Replace the instruction.

        Args:
            instruction: The new quantum instruction.
        """

    @property
    def error_rate(self) -> float:
        """The error rate (infidelity) of the instruction, in [0.0, 1.0]."""

    @error_rate.setter
    def error_rate(self, error_rate: float) -> None:
        """Set the error rate.

        Args:
            error_rate: Error rate in [0.0, 1.0].

        Raises:
            ValueError: If ``error_rate`` is not finite or outside [0, 1].
        """

    @property
    def length(self) -> float | None:
        """The duration of the instruction in nanoseconds, or ``None`` if not configured."""

    @length.setter
    def length(self, length: float) -> None:
        """Set the gate duration.

        Args:
            length: Gate length in nanoseconds.

        Raises:
            ValueError: If ``length`` is not finite or is negative.
        """

    def __copy__(self) -> "InstructionProp": ...
    def __deepcopy__(self, memo: dict) -> "InstructionProp": ...


class QubitProp:
    """Physical properties of an individual qubit.

    Includes coherence metrics (T1 relaxation, T2 dephasing), operational
    frequency, measurement error rates, and a list of native single-qubit
    instructions with their calibrated fidelities and durations.

    Key Usage::

        >>> from cqlib.device import QubitProp
        >>>
        >>> # Create with 1% readout error
        >>> prop = QubitProp(readout_error=0.01)
        >>>
        >>> # Set coherence times (microseconds)
        >>> prop.t1 = 50.0
        >>> prop.t2 = 30.0
        >>>
        >>> # Set qubit frequency (GHz)
        >>> prop.frequency = 5.2
        >>>
        >>> # Set measurement discrimination errors
        >>> prop.prob_meas0_prep1 = 0.02  # P(meas 0 | prep 1)
        >>> prop.prob_meas1_prep0 = 0.01  # P(meas 1 | prep 0)
    """

    def __init__(self, readout_error: float) -> None:
        """Creates qubit properties with a base readout error.

        Args:
            readout_error: Base readout error rate in [0.0, 1.0].

        Raises:
            ValueError: If ``readout_error`` is not finite or outside [0, 1].
        """

    @property
    def readout_error(self) -> float:
        """Base readout error rate."""

    @property
    def prob_meas0_prep1(self) -> float | None:
        """Probability of measuring 0 given state was prepared in 1 (false negative)."""

    @prob_meas0_prep1.setter
    def prob_meas0_prep1(self, prob: float) -> None:
        """Set P(meas 0 | prep 1).

        Args:
            prob: Probability in [0.0, 1.0].

        Raises:
            ValueError: If ``prob`` is not finite or outside [0, 1].
        """

    @property
    def prob_meas1_prep0(self) -> float | None:
        """Probability of measuring 1 given state was prepared in 0 (false positive)."""

    @prob_meas1_prep0.setter
    def prob_meas1_prep0(self, prob: float) -> None:
        """Set P(meas 1 | prep 0).

        Args:
            prob: Probability in [0.0, 1.0].

        Raises:
            ValueError: If ``prob`` is not finite or outside [0, 1].
        """

    @property
    def t1(self) -> float | None:
        """T1 relaxation time in microseconds, or ``None`` if not set."""

    @t1.setter
    def t1(self, t1: float) -> None:
        """Set T1 relaxation time.

        Args:
            t1: T1 time in microseconds.

        Raises:
            ValueError: If ``t1`` is not finite or is not positive.
        """

    @property
    def t2(self) -> float | None:
        """T2 dephasing time in microseconds, or ``None`` if not set."""

    @t2.setter
    def t2(self, t2: float) -> None:
        """Set T2 dephasing time.

        Args:
            t2: T2 time in microseconds.

        Raises:
            ValueError: If ``t2`` is not finite or is not positive.
        """

    @property
    def frequency(self) -> float | None:
        """Qubit transition frequency in GHz, or ``None`` if not set."""

    @frequency.setter
    def frequency(self, frequency: float) -> None:
        """Set qubit frequency.

        Args:
            frequency: Transition frequency in GHz.

        Raises:
            ValueError: If ``frequency`` is not finite or is not positive.
        """

    @property
    def native_instructions(self) -> list[InstructionProp]:
        """List of native instructions supported on this qubit with their calibration."""

    def add_native_instruction(self, prop: InstructionProp) -> None:
        """Appends a native instruction to this qubit's calibration list.

        Use this to register per-qubit calibration data for each supported gate.

        Args:
            prop: The instruction calibration data to add.

        Raises:
            ValueError: If the instruction is not a single-qubit standard gate
                or its calibration values are invalid.
        """

    def __copy__(self) -> "QubitProp": ...
    def __deepcopy__(self, memo: dict) -> "QubitProp": ...


class EdgeProp:
    """Properties of a coupling edge between two qubits.

    Tracks the native multi-qubit instructions (e.g., CX, CZ) supported
    across a specific physical connection, including their directional
    error rates and execution times.

    Key Usage::

        >>> from cqlib.device import EdgeProp, InstructionProp
        >>> from cqlib.circuit import StandardGate
        >>>
        >>> edge = EdgeProp()
        >>> cx = Instruction.from_standard_gate(StandardGate.CX)
        >>> cx_prop = InstructionProp(cx, error_rate=0.005)
        >>> cx_prop.length = 200.0  # 200 ns
        >>> edge.add_native_instruction(cx_prop)
    """

    def __init__(self) -> None:
        """Creates empty edge properties."""

    @property
    def native_instructions(self) -> list[InstructionProp]:
        """List of multi-qubit instructions supported on this edge."""

    def add_native_instruction(self, prop: InstructionProp) -> None:
        """Appends a native instruction to this edge's calibration list.

        Args:
            prop: The instruction calibration data (appended to existing list).

        Raises:
            ValueError: If the instruction is not a two-qubit standard gate
                or its calibration values are invalid.
        """

    def __copy__(self) -> "EdgeProp": ...
    def __deepcopy__(self, memo: dict) -> "EdgeProp": ...


class Device:
    """Complete hardware description of a quantum device.

    Encapsulates the physical topology, qubit properties, coupling-edge
    properties, and default calibration values needed for noise-aware
    compilation, mapping, routing, and scheduling.

    Per-qubit properties (T1, T2, readout error) take precedence over
    device-wide defaults. Error queries (e.g., :meth:`single_qubit_error`)
    fall back from per-qubit calibration → per-instruction calibration →
    device-wide defaults.

    Key Usage — manual construction::

        >>> from cqlib.device import Device, Topology, QubitProp
        >>> from datetime import datetime, timezone
        >>>
        >>> # 1. Build topology
        >>> topology = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CX")])
        >>>
        >>> # 2. Create device
        >>> device = Device("superconducting_qpu", [0, 1, 2], topology)
        >>>
        >>> # 3. Set device-wide defaults
        >>> device.default_t1 = 100.0   # μs
        >>> device.default_t2 = 50.0    # μs
        >>> device.default_readout_error = 0.02
        >>> device.default_single_qubit_error = 0.001
        >>> device.default_two_qubit_error = 0.01
        >>>
        >>> # 4. Set per-qubit overrides
        >>> prop = QubitProp(readout_error=0.01)
        >>> prop.t1 = 120.0
        >>> device.add_qubit_properties(0, prop)
        >>>
        >>> # 5. Query effective values (fallback to defaults)
        >>> device.get_t1(0)  # 120.0 (specific)
        >>> device.get_t1(1)  # 100.0 (default)

    Key Usage — factory construction::

        >>> from cqlib.device import Device
        >>>
        >>> # Build a 5-qubit directed chain
        >>> dev = Device.line("chain", num_qubits=5)
        >>>
        >>> # Build a 3x3 grid
        >>> dev = Device.grid("grid", rows=3, cols=3)
        >>>
        >>> # Build a ring
        >>> dev = Device.ring("ring", num_qubits=8)
    """

    # ---- Constructor ----

    def __init__(
        self, name: str, qubits: list[_PhysicalQubitLike], topology: Topology
    ) -> None:
        """Creates a device with given qubits and topology.

        Args:
            name: Human-readable device name (e.g., ``"ibm_sherbrooke"``).
            qubits: List of available physical qubits.
            topology: The connectivity graph (:class:`Topology`).

        Raises:
            ValueError: If a qubit in the topology is not in the ``qubits`` list.
        """

    # ---- Factory constructors ----

    @staticmethod
    def line(name: str, num_qubits: int) -> "Device":
        """Create a device with qubits connected as a directed line.

        Qubits ``0..num_qubits-1``, all online. Couplings:
        ``q[i] -> q[i+1]``.

        Args:
            name: Device name.
            num_qubits: Number of qubits.

        Returns:
            A new :class:`Device` with a directed line topology.

        Example::

            >>> dev = Device.line("my_chain", num_qubits=5)
            >>> dev.num_usable_qubits  # 5
        """

    @staticmethod
    def line_from_qubits(
        name: str, physical_qubits: list[_PhysicalQubitLike]
    ) -> "Device":
        """Create a device with given qubits connected as a directed line.

        Couplings follow the supplied order: ``qubits[i] -> qubits[i+1]``.

        Args:
            name: Device name.
            physical_qubits: List of qubit IDs in line order.

        Example::

            >>> dev = Device.line_from_qubits("custom", [100, 101, 102, 103])
            >>> dev.qubits  # [PhysicalQubit(100), PhysicalQubit(101), PhysicalQubit(102), PhysicalQubit(103)]
        """

    @staticmethod
    def bidirectional_line(name: str, num_qubits: int) -> "Device":
        """Create a device with adjacent qubits coupled in both directions.

        Qubits ``0..num_qubits-1``, all online.

        Args:
            name: Device name.
            num_qubits: Number of qubits.
        """

    @staticmethod
    def ring(name: str, num_qubits: int) -> "Device":
        """Create a device with qubits connected as a bidirectional ring.

        For 2+ qubits, each qubit is connected to its successor
        (modulo ``num_qubits``) in both directions.

        Args:
            name: Device name.
            num_qubits: Number of qubits (minimum 2 for non-trivial ring).
        """

    @staticmethod
    def star(name: str, num_qubits: int, center: int) -> "Device":
        """Create a device with qubits connected as a bidirectional star.

        Every non-center qubit is connected to ``center`` in both directions.

        Args:
            name: Device name.
            num_qubits: Total number of qubits.
            center: Center qubit ID (must be in ``0..num_qubits-1``).
        """

    @staticmethod
    def grid(name: str, rows: int, cols: int) -> "Device":
        """Create a device with qubits connected as a bidirectional grid.

        Qubit IDs are row-major order. Horizontal and vertical
        nearest-neighbor couplings are added in both directions.

        Args:
            name: Device name.
            rows: Number of rows.
            cols: Number of columns.
        """

    @staticmethod
    def from_edges(
        name: str, num_qubits: int, edges: list[tuple[int, int]]
    ) -> "Device":
        """Create a device with explicit directed edges.

        Each ``(control, target)`` pair becomes one directed coupling.

        Args:
            name: Device name.
            num_qubits: Number of physical qubits (``0..num_qubits-1``).
            edges: List of ``(control, target)`` pairs.

        Example::

            >>> dev = Device.from_edges("custom", 4, [(0, 1), (1, 2), (2, 3)])
        """

    # ---- Device metadata ----

    @property
    def name(self) -> str:
        """Device name."""

    @property
    def qubits(self) -> list[PhysicalQubit]:
        """All registered physical qubits."""

    @property
    def invalid_qubits(self) -> list[PhysicalQubit]:
        """Invalid (offline / faulty) qubits."""

    @invalid_qubits.setter
    def invalid_qubits(self, qubits: list[_PhysicalQubitLike]) -> None:
        """Set the list of invalid qubits.

        Raises:
            ValueError: If any qubit is not registered with the device.
        """

    @property
    def topology(self) -> Topology:
        """Device connectivity topology."""

    @property
    def native_gates(self) -> list[Instruction]:
        """Device-wide native gates (fallback when per-qubit gates not set)."""

    @native_gates.setter
    def native_gates(self, gates: list[Instruction]) -> None:
        """Set the device-wide native gates.

        Raises:
            ValueError: If any entry is non-standard or acts on more than two qubits.
        """

    @property
    def calibration_time(self) -> datetime | None:
        """Timestamp of last calibration, or ``None``."""

    def set_calibration_time(self, datetime_: datetime) -> None:
        """Set the calibration timestamp with nanosecond precision.

        Args:
            datetime_: The calibration timestamp.

        Raises:
            ValueError: If the timestamp is out of range.
        """

    # ---- Device-wide defaults ----

    @property
    def default_t1(self) -> float | None:
        """Default T1 relaxation time (μs) for qubits without specific data."""

    @default_t1.setter
    def default_t1(self, t1: float) -> None:
        """Set default T1 time (μs).

        Raises:
            ValueError: If ``t1`` is not finite or is not positive.
        """

    @property
    def default_t2(self) -> float | None:
        """Default T2 dephasing time (μs) for qubits without specific data."""

    @default_t2.setter
    def default_t2(self, t2: float) -> None:
        """Set default T2 time (μs).

        Raises:
            ValueError: If ``t2`` is not finite or is not positive.
        """

    @property
    def default_readout_error(self) -> float | None:
        """Default readout error rate for qubits without specific data."""

    @default_readout_error.setter
    def default_readout_error(self, error: float) -> None:
        """Set default readout error.

        Raises:
            ValueError: If ``error`` is not finite or outside [0, 1].
        """

    @property
    def default_single_qubit_error(self) -> float | None:
        """Default error rate for single-qubit gates."""

    @default_single_qubit_error.setter
    def default_single_qubit_error(self, error: float) -> None:
        """Set default single-qubit error.

        Raises:
            ValueError: If ``error`` is not finite or outside [0, 1].
        """

    @property
    def default_two_qubit_error(self) -> float | None:
        """Default error rate for two-qubit gates."""

    @default_two_qubit_error.setter
    def default_two_qubit_error(self, error: float) -> None:
        """Set default two-qubit error.

        Raises:
            ValueError: If ``error`` is not finite or outside [0, 1].
        """

    # ---- Per-qubit / per-edge property management ----

    def add_qubit_properties(
        self, qubit: _PhysicalQubitLike, props: QubitProp
    ) -> None:
        """Add or update properties for a specific qubit.

        Args:
            qubit: Target qubit.
            props: The :class:`QubitProp` data to assign.

        Raises:
            ValueError: If the qubit is not registered or is absent from the topology.

        Calibration can be retained for a registered qubit even while it is
        marked invalid/offline.
        """

    def add_edge_properties(
        self, control: _PhysicalQubitLike, target: _PhysicalQubitLike, props: EdgeProp
    ) -> None:
        """Add or update properties for a specific directed coupling.

        Args:
            control: Source qubit.
            target: Destination qubit.
            props: The :class:`EdgeProp` data to assign.

        Raises:
            ValueError: If the edge is not in the topology.
        """

    def qubit_properties(self, qubit: _PhysicalQubitLike) -> QubitProp | None:
        """Get the properties of a specific qubit.

        Returns ``None`` if no properties have been assigned for this qubit.

        Args:
            qubit: The qubit to query.
        """

    def edge_properties(
        self, control: _PhysicalQubitLike, target: _PhysicalQubitLike
    ) -> EdgeProp | None:
        """Get the properties of a specific directed coupling.

        Returns ``None`` if no properties have been assigned for this edge.

        Args:
            control: Source qubit.
            target: Destination qubit.
        """

    # ---- Effective qubit parameter queries (fallback to defaults) ----

    def get_t1(self, qubit: _PhysicalQubitLike) -> float | None:
        """Get T1 relaxation time for a qubit.

        Falls back to :attr:`default_t1` if the qubit has no specific value.

        Args:
            qubit: The qubit to query.
        """

    def get_t2(self, qubit: _PhysicalQubitLike) -> float | None:
        """Get T2 dephasing time for a qubit.

        Falls back to :attr:`default_t2` if the qubit has no specific value.

        Args:
            qubit: The qubit to query.
        """

    def get_readout_error(self, qubit: _PhysicalQubitLike) -> float | None:
        """Get readout error rate for a qubit.

        Falls back to :attr:`default_readout_error` if the qubit has no specific value.

        Args:
            qubit: The qubit to query.
        """

    # ---- Error rate queries for routing / noise-aware compilation ----

    def single_qubit_error(
        self, qubit: _PhysicalQubitLike, instruction: Instruction
    ) -> float | None:
        """Get the error rate for a given instruction on a single qubit.

        Fallback chain:
        1. Per-qubit native instruction error (from :class:`QubitProp`)
        2. Per-qubit default single-qubit error (from :class:`QubitProp`)
        3. Device-wide :attr:`default_single_qubit_error`

        Returns ``None`` if the qubit is not usable.

        Args:
            qubit: The qubit to query.
            instruction: The instruction whose error rate is requested.
        """

    def two_qubit_error(
        self,
        control: _PhysicalQubitLike,
        target: _PhysicalQubitLike,
        instruction: Instruction,
    ) -> float | None:
        """Get the error rate for a given instruction on a directed coupling.

        Fallback chain:
        1. Per-edge native instruction error (from :class:`EdgeProp`)
        2. Device-wide :attr:`default_two_qubit_error`

        Returns ``None`` if either qubit is unusable or the coupling
        does not exist.

        Args:
            control: Source qubit.
            target: Destination qubit.
            instruction: The instruction whose error rate is requested.
        """

    def edge_error(
        self, control: _PhysicalQubitLike, target: _PhysicalQubitLike
    ) -> float | None:
        """Get the best available two-qubit error on a directed coupling.

        Scans all native instructions on the edge and returns the minimum
        error rate. Useful for routing cost estimation.

        Falls back to :attr:`default_two_qubit_error` if no per-edge
        calibration exists.

        Returns ``None`` if either qubit is unusable or the coupling
        does not exist.

        Args:
            control: Source qubit.
            target: Destination qubit.
        """

    # ---- Qubit usability queries ----

    @property
    def usable_qubits(self) -> list[PhysicalQubit]:
        """All registered physical qubits that are not marked invalid."""

    @property
    def num_usable_qubits(self) -> int:
        """Number of usable (registered and not invalid) physical qubits."""

    def is_usable_qubit(self, qubit: _PhysicalQubitLike) -> bool:
        """Check whether a physical qubit is registered and not invalid.

        Args:
            qubit: The qubit to check.

        Returns:
            ``True`` if the qubit is online and usable.
        """

    def supports_native_instruction(
        self, instruction: Instruction, qargs: list[_PhysicalQubitLike]
    ) -> bool:
        """Return whether an instruction is native on the exact ordered qargs."""

    def validate_operation(self, operation: ValueOperation) -> None:
        """Validate one value-level operation in the physical-qubit ID space.

        Classical control-flow bodies are checked recursively without requiring
        the operation's owning circuit or its classical resource tables.

        Raises:
            ValueError: If the operation cannot execute on this device.
        """

    def validate_circuit(self, circuit: Circuit) -> None:
        """Validate a circuit in the physical-qubit ID space.

        Raises:
            ValueError: If any operation cannot execute on this device.
        """

    def __copy__(self) -> "Device": ...
    def __deepcopy__(self, memo: dict) -> "Device": ...
