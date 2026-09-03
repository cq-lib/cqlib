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

"""Quantum circuit container and builder.

.. autoclass:: Circuit
    :noindex:

Type aliases for gate arguments
-------------------------------

.. data:: QubitLike
    :annotation: = int | Qubit

    Single-qubit gate argument (integer index or Qubit object).

.. data:: QubitInput
    :annotation: = int | list[int] | list[Qubit]

    Circuit constructor argument: a qubit count, a list of indices, or Qubits.

.. data:: QubitList
    :annotation: = list[int] | list[Qubit]

    Multi-qubit operation argument: list of indices or Qubit objects.

.. data:: ParamLike
    :annotation: = float | Parameter

    Gate parameter argument: a concrete float or symbolic Parameter.
"""

import numpy as np
from collections.abc import Callable
from numpy.typing import NDArray
from .bit import Qubit
from .classical import CircuitId, ClassicalType, ClassicalVar, Measurement
from .classical_expr import ClassicalExpr
from .control_flow import ClassicalControlOp
from .dag import CircuitDag
from .gates import CircuitGate, MCGate, StandardGate, UnitaryGate
from .operation import ValueOperation
from .parameter import Parameter
from .symbolic_matrix import SymbolicMatrix

QubitLike = int | Qubit
QubitInput = int | list[int] | list[Qubit]
QubitList = list[int] | list[Qubit]
ParamLike = float | Parameter

class _SwitchBuilder:
    """Temporary case collector passed only to :meth:`Circuit.switch` callbacks.

    This object is valid **only** inside the callback handed to
    :meth:`Circuit.switch`.  Methods that refer to scoped circuit
    operations (:meth:`value`, :meth:`default`) operate on a temporary
    transaction within the parent circuit: all operations discarded if
    the callback raises an exception.

    Do not hold a reference to this object beyond the callback lifetime.
    """

    def value(self, value: int, body: Callable[[Circuit], None]) -> None:
        """Register one exact-integer switch case.

        Args:
            value: The unsigned integer value to match.
            body: A callback that receives the scoped circuit and builds
                the case's operations.  Built atomically — a callback
                exception rolls back this case.
        """
        ...

    def default(self, body: Callable[[Circuit], None]) -> None:
        """Register the optional default (fallback) case.

        Must be called at most once per switch.  If not called, the
        switch has no fallback (may trap on unhandled values).

        Args:
            body: A callback that receives the scoped circuit and builds
                the default branch.  Operates atomically with the same
                rollback semantics as :meth:`value`.
        """
        ...

class Circuit:
    """Mutable quantum circuit with gate, parameter, and dynamic-control support.

    ``Circuit`` is the primary interface for building quantum programs.  It
    accepts flexible qubit arguments — integers or :class:`Qubit` objects::

        c = Circuit(2)        # 2-qubit circuit
        c.h(0)                # Hadamard on qubit 0
        c.cx(0, 1)            # CNOT with control=0, target=1
        c.rx(0, 0.5)          # RX(0.5) on qubit 0

    Parameterized circuits use :class:`Parameter` for gate angles::

        from cqlib import Parameter
        theta = Parameter("theta")
        c = Circuit(1)
        c.rx(0, theta)
        bound = c.assign_parameters({"theta": 3.14})

    Dynamic circuits use :meth:`var`, :meth:`measure`, and
    :meth:`append_control` for mid-circuit classical logic.

    ``Circuit`` instances are copyable. Methods returning ``Circuit`` (e.g.
    :meth:`inverse`, :meth:`decompose`, :meth:`assign_parameters`) return plain
    ``Circuit`` objects.
    """

    def __init__(self, qubits: QubitInput) -> None:
        """Create a circuit.

        Args:
            qubits: An integer count (``Circuit(3)`` creates qubits 0,1,2),
                a list of integer indices (``Circuit([0, 2, 4])``), or a list
                of :class:`Qubit` objects.

        Raises:
            CircuitError: If the qubit specification is invalid.
        """
        ...
    @staticmethod
    def from_operations(
        qubits: list[Qubit],
        operations: list[ValueOperation],
        classical_vars: list[ClassicalType] | None = ...,
        classical_values: list[ClassicalType] | None = ...,
    ) -> Circuit:
        """Build a circuit from self-contained construction-IR operations.

        This is the low-level entry point for programmatic circuit construction
        and deserialization.
        """
        ...
    @property
    def id(self) -> CircuitId:
        """The :class:`CircuitId` owning this circuit's classical handles."""
        ...
    @property
    def num_qubits(self) -> int:
        """Number of qubits in the circuit."""
        ...
    @property
    def width(self) -> int:
        """Alias for :attr:`num_qubits`."""
        ...
    @property
    def qubits(self) -> list[Qubit]:
        """Qubits in insertion order."""
        ...
    @property
    def parameters(self) -> list[Parameter]:
        """Interned parameters in insertion order."""
        ...
    @property
    def symbols(self) -> list[str]:
        """Interned symbol names in stable insertion order.

        This registry may retain a symbol after all executable references to
        it have been removed. Use :attr:`used_symbols` for live dependencies.
        """
        ...
    @property
    def used_symbols(self) -> list[str]:
        """Symbol names actually referenced by executable circuit IR.

        Global phase, top-level operations, and nested control-flow bodies are
        included. Ordering follows the stable interned registry order.
        """
        ...
    def uses_symbol(self, symbol: str) -> bool:
        """Whether executable circuit IR currently references ``symbol``.

        An interned symbol that is no longer referenced returns ``False``.
        """
        ...
    @property
    def global_phase(self) -> Parameter:
        """The circuit's global phase as a :class:`Parameter`."""
        ...
    @property
    def classical_vars(self) -> list[ClassicalType]:
        """Types of allocated classical variables."""
        ...
    @property
    def classical_values(self) -> list[ClassicalType]:
        """Types of immutable classical values (from measurements)."""
        ...
    @property
    def operations(self) -> list[ValueOperation]:
        """All operations in insertion order."""
        ...
    def dag(self) -> CircuitDag:
        """Build the operation dependency DAG analysis view."""
        ...
    def depth(self, recurse: bool = False) -> int:
        """Return the circuit depth (longest ASAP path over qubit wires).

        Every instruction node contributes one layer on the qubits it touches.
        Barriers serialize their listed qubits (an empty-qubit barrier is
        global). ``CircuitGate`` counts as depth 1 (opaque; call :meth:`decompose`
        first to recurse).

        Args:
            recurse: If ``False`` (default), raise :class:`CircuitError` when the
                circuit contains any control-flow operation (``if``/``while``/
                ``for``/``switch``/``break``/``continue``) at any depth. If
                ``True``, return an estimated depth: ``if``/``switch`` take the
                max branch, ``while`` counts its body once, ``for`` with a
                statically-known unsigned-literal range is fully unrolled (else
                counted once).

        Raises:
            CircuitError: If ``recurse=False`` and the circuit contains control
                flow.
        """
        ...
    def set_global_phase(self, phase: ParamLike) -> None:
        """Replace the circuit global phase."""
        ...
    def add_qubits(self, qubits: QubitList) -> None:
        """Add qubits while preserving existing circuit data."""
        ...
    def append(self, operation: ValueOperation) -> None:
        """Append any self-contained construction-IR operation."""
        ...
    def append_control(self, control: ClassicalControlOp) -> None:
        """Append a classical control-flow operation.

        Build the control op with :class:`ClassicalControlOp` static methods
        (``if_``, ``while_``, ``for_uint``, ``switch``, etc.) and use this
        method to add it to the circuit.
        """
        ...
    def if_(self, condition: ClassicalExpr, body: Callable[[Circuit], None]) -> None:
        """Append an ``if`` whose body is built by ``body``.

        The callback receives this circuit in a temporary branch scope. Its
        operations and classical values are committed atomically. A callback
        exception or circuit validation error rolls back the complete body.
        """
        ...
    def if_else(
        self,
        condition: ClassicalExpr,
        then_body: Callable[[Circuit], None],
        else_body: Callable[[Circuit], None],
    ) -> None:
        """Append an ``if``/``else`` built by two atomic callbacks."""
        ...
    def while_(self, condition: ClassicalExpr, body: Callable[[Circuit], None]) -> None:
        """Append a runtime ``while`` loop built by an atomic callback."""
        ...
    def for_uint(
        self,
        var: ClassicalVar,
        start: ClassicalExpr,
        stop: ClassicalExpr,
        step: ClassicalExpr,
        body: Callable[[Circuit, ClassicalExpr], None],
    ) -> None:
        """Append a half-open unsigned runtime loop.

        ``body`` receives the scoped circuit and an expression reading ``var``.
        ``start``, ``stop``, and ``step`` must have the same UInt type as ``var``.
        """
        ...
    def switch(
        self, target: ClassicalExpr, build: Callable[[_SwitchBuilder], None]
    ) -> None:
        """Append an exact-value UInt switch built atomically by ``build``."""
        ...
    def break_loop(self) -> None:
        """Exit the nearest enclosing loop or switch callback body."""
        ...
    def continue_loop(self) -> None:
        """Continue the nearest enclosing loop callback body."""
        ...
    def operation(self, index: int) -> ValueOperation:
        """Return one operation by index with circuit-local parameters resolved.

        Raises:
            IndexError: If ``index`` is out of bounds.
        """
        ...
    def remove_operation(self, index: int) -> ValueOperation:
        """Remove one top-level operation and return it.

        Args:
            index: Original operation index before deletion.

        Raises:
            IndexError: If ``index`` is out of bounds.
            CircuitError: If deletion would violate classical-value/control-flow
                invariants.
        """
        ...
    def remove_operations(self, indices: list[int]) -> list[ValueOperation]:
        """Remove multiple top-level operations and return them.

        ``indices`` are interpreted against the original operation list before
        deletion. Duplicate indices are ignored, and removed operations are
        returned in ascending original-index order.

        Raises:
            IndexError: If any index is out of bounds.
            CircuitError: If deletion would violate classical-value/control-flow
                invariants. On failure the circuit is unchanged.
        """
        ...
    def append_gate(
        self, gate: StandardGate, qubits: QubitList, label: str | None = ...
    ) -> None:
        """Append a standard gate with any bound parameters already applied.

        Args:
            gate: A :class:`StandardGate` (call it with parameters to bind them).
            qubits: Target qubits, in the order the gate expects.
            label: Optional diagnostic label attached to the operation.
        """
        ...
    def append_mc_gate(
        self, gate: MCGate, qubits: QubitList, label: str | None = ...
    ) -> None:
        """Append a multi-controlled gate.

        Args:
            gate: A :class:`MCGate` (controls followed by its base gate's targets).
            qubits: Control qubits then target qubits, matching ``gate.num_ctrl_qubits``.
            label: Optional diagnostic label attached to the operation.
        """
        ...
    def append_unitary_gate(
        self, gate: UnitaryGate, qubits: QubitList, params: list[ParamLike] | None = ...
    ) -> None:
        """Append a user-defined unitary gate.

        Args:
            gate: A :class:`UnitaryGate` (numeric, symbolic, or circuit-backed).
            qubits: Target qubits in the order the gate's matrix acts on.
            params: Concrete parameters for a parametric unitary; ``None`` keeps symbols.
        """
        ...
    def append_circuit_gate(
        self, gate: CircuitGate, qubits: QubitList, params: list[ParamLike] | None = ...
    ) -> None:
        """Append a sub-circuit wrapped as a gate.

        Args:
            gate: A :class:`CircuitGate` built from a :class:`FrozenCircuit`.
            qubits: Target qubits mapped positionally onto the sub-circuit's qubits.
            params: Parameter bindings for the sub-circuit's free symbols.
        """
        ...
    def i(self, qubit: QubitLike) -> None:
        """Append an identity (no-op) gate."""
        ...
    def h(self, qubit: QubitLike) -> None:
        """Append a Hadamard gate."""
        ...
    def x(self, qubit: QubitLike) -> None:
        """Append a Pauli-X (NOT) gate."""
        ...
    def y(self, qubit: QubitLike) -> None:
        """Append a Pauli-Y gate."""
        ...
    def z(self, qubit: QubitLike) -> None:
        """Append a Pauli-Z gate."""
        ...
    def x2p(self, qubit: QubitLike) -> None:
        """Append an X^{+1/2} gate (rotation about X by +pi/2)."""
        ...
    def x2m(self, qubit: QubitLike) -> None:
        """Append an X^{-1/2} gate (rotation about X by -pi/2)."""
        ...
    def y2p(self, qubit: QubitLike) -> None:
        """Append a Y^{+1/2} gate (rotation about Y by +pi/2)."""
        ...
    def y2m(self, qubit: QubitLike) -> None:
        """Append a Y^{-1/2} gate (rotation about Y by -pi/2)."""
        ...
    def xy(self, qubit: QubitLike, theta: ParamLike) -> None:
        """Append an XY interaction gate parameterized by ``theta``.

        Args:
            qubit: Target qubit.
            theta: Interaction angle (float or :class:`Parameter`).
        """
        ...
    def xy2p(self, qubit: QubitLike, theta: ParamLike) -> None:
        """Append a +half-pi XY gate (XY interaction at +pi/2).

        Args:
            qubit: Target qubit.
            theta: Additional interaction angle (float or :class:`Parameter`).
        """
        ...
    def xy2m(self, qubit: QubitLike, theta: ParamLike) -> None:
        """Append a -half-pi XY gate (XY interaction at -pi/2).

        Args:
            qubit: Target qubit.
            theta: Additional interaction angle (float or :class:`Parameter`).
        """
        ...
    def s(self, qubit: QubitLike) -> None:
        """Append an S gate (phase gate, |1> -> i|1>)."""
        ...
    def sdg(self, qubit: QubitLike) -> None:
        """Append an S-dagger gate (inverse of S)."""
        ...
    def t(self, qubit: QubitLike) -> None:
        """Append a T gate (pi/4 phase gate)."""
        ...
    def tdg(self, qubit: QubitLike) -> None:
        """Append a T-dagger gate (inverse of T)."""
        ...
    def rx(self, qubit: QubitLike, theta: ParamLike) -> None:
        """Append an RX rotation gate.

        Args:
            qubit: Target qubit.
            theta: Rotation angle (float or :class:`Parameter`).
        """
        ...
    def ry(self, qubit: QubitLike, theta: ParamLike) -> None:
        """Append an RY rotation gate.

        Args:
            qubit: Target qubit.
            theta: Rotation angle (float or :class:`Parameter`).
        """
        ...
    def rz(self, qubit: QubitLike, theta: ParamLike) -> None:
        """Append an RZ rotation gate.

        Args:
            qubit: Target qubit.
            theta: Rotation angle (float or :class:`Parameter`).
        """
        ...
    def phase(self, qubit: QubitLike, lambda_: ParamLike) -> None:
        """Append a phase gate P(lambda) (|1> -> e^{i*lambda}|1>).

        Args:
            qubit: Target qubit.
            lambda_: Phase angle (float or :class:`Parameter`).
        """
        ...
    def u(
        self, qubit: QubitLike, theta: ParamLike, phi: ParamLike, lambda_: ParamLike
    ) -> None:
        """Append a general single-qubit unitary U(theta, phi, lambda).

        Args:
            qubit: Target qubit.
            theta: Rotation angle about the Bloch axis.
            phi: Azimuthal phase angle.
            lambda_: Terminal phase angle.
        """
        ...
    def cx(self, control: QubitLike, target: QubitLike) -> None:
        """Append a controlled-X (CNOT) gate."""
        ...
    def cy(self, control: QubitLike, target: QubitLike) -> None:
        """Append a controlled-Y gate."""
        ...
    def cz(self, control: QubitLike, target: QubitLike) -> None:
        """Append a controlled-Z gate."""
        ...
    def swap(self, a: QubitLike, b: QubitLike) -> None:
        """Append a SWAP gate exchanging the states of two qubits."""
        ...
    def ccx(self, control1: QubitLike, control2: QubitLike, target: QubitLike) -> None:
        """Append a doubly-controlled-X (Toffoli) gate."""
        ...
    def rxx(self, a: QubitLike, b: QubitLike, theta: ParamLike) -> None:
        """Append an RXX(theta) gate (rotation about XX).

        Args:
            a: First qubit.
            b: Second qubit.
            theta: Rotation angle (float or :class:`Parameter`).
        """
        ...
    def ryy(self, a: QubitLike, b: QubitLike, theta: ParamLike) -> None:
        """Append an RYY(theta) gate (rotation about YY).

        Args:
            a: First qubit.
            b: Second qubit.
            theta: Rotation angle (float or :class:`Parameter`).
        """
        ...
    def rzz(self, a: QubitLike, b: QubitLike, theta: ParamLike) -> None:
        """Append an RZZ(theta) gate (rotation about ZZ).

        Args:
            a: First qubit.
            b: Second qubit.
            theta: Rotation angle (float or :class:`Parameter`).
        """
        ...
    def rzx(self, a: QubitLike, b: QubitLike, theta: ParamLike) -> None:
        """Append an RZX(theta) gate (rotation about ZX).

        Args:
            a: First qubit (Z operand).
            b: Second qubit (X operand).
            theta: Rotation angle (float or :class:`Parameter`).
        """
        ...
    def crx(self, control: QubitLike, target: QubitLike, theta: ParamLike) -> None:
        """Append a controlled-RX(theta) gate.

        Args:
            control: Control qubit.
            target: Target qubit.
            theta: Rotation angle (float or :class:`Parameter`).
        """
        ...
    def cry(self, control: QubitLike, target: QubitLike, theta: ParamLike) -> None:
        """Append a controlled-RY(theta) gate.

        Args:
            control: Control qubit.
            target: Target qubit.
            theta: Rotation angle (float or :class:`Parameter`).
        """
        ...
    def crz(self, control: QubitLike, target: QubitLike, theta: ParamLike) -> None:
        """Append a controlled-RZ(theta) gate.

        Args:
            control: Control qubit.
            target: Target qubit.
            theta: Rotation angle (float or :class:`Parameter`).
        """
        ...
    def fsim(
        self, a: QubitLike, b: QubitLike, theta: ParamLike, phi: ParamLike
    ) -> None:
        """Append an fSim(theta, phi) gate.

        Args:
            a: First qubit.
            b: Second qubit.
            theta: Swap angle (float or :class:`Parameter`).
            phi: Controlled-phase angle (float or :class:`Parameter`).
        """
        ...
    def rxy(self, qubit: QubitLike, theta: ParamLike, phi: ParamLike) -> None:
        """Append an RXY(theta, phi) gate whose rotation axis is set by ``phi``.

        Args:
            qubit: Target qubit.
            theta: Rotation amount (float or :class:`Parameter`).
            phi: Rotation-axis angle in the XY plane (float or :class:`Parameter`).
        """
        ...
    def barrier(self, qubits: QubitList) -> None:
        """Insert a barrier preventing gate reordering across the listed qubits.

        Args:
            qubits: Qubits the barrier spans.
        """
        ...
    def reset(self, qubit: QubitLike) -> None:
        """Append a reset directive, forcing the qubit to |0>.

        Args:
            qubit: Qubit to reset.
        """
        ...
    def delay(self, qubit: QubitLike, duration: ParamLike) -> None:
        """Append a delay (idle) of the given duration on a qubit.

        Args:
            qubit: Qubit held idle.
            duration: Idle duration (float or :class:`Parameter`).
        """
        ...
    def var(self, ty: ClassicalType) -> ClassicalVar:
        """Allocate a mutable classical variable of the given type."""
        ...
    def store(self, target: ClassicalVar, value: ClassicalExpr) -> None:
        """Store a classical expression into a variable."""
        ...
    def measure(self, qubit: QubitLike) -> Measurement:
        """Measure a single qubit and return a :class:`Measurement` receipt."""
        ...
    def measure_bits(self, qubits: QubitList) -> Measurement:
        """Measure multiple qubits and return a :class:`Measurement` receipt."""
        ...
    def measure_into(self, qubit: QubitLike, target: ClassicalVar) -> Measurement:
        """Measure a qubit and store the result into an existing variable."""
        ...
    def measure_bits_into(self, qubits: QubitList, target: ClassicalVar) -> Measurement:
        """Measure multiple qubits and store results into an existing variable."""
        ...
    def inverse(self) -> Circuit:
        """Return the inverse circuit (every operation reversed).

        Raises:
            CircuitError: If any operation is not invertible.
        """
        ...
    def decompose(self) -> Circuit:
        """Recursively expand circuit-defined gates."""
        ...
    def to_gate(self, name: str) -> CircuitGate:
        """Convert this circuit into a reusable :class:`CircuitGate`."""
        ...
    def assign_parameters(self, bindings: dict[str, float] | None = ...) -> Circuit:
        """Return a new circuit with symbolic parameters numerically bound.

        Args:
            bindings: Mapping from parameter name to numeric value.

        Returns:
            A new :class:`Circuit` with all symbols resolved.

        Raises:
            ParameterError: If any binding value is non-finite (NaN, Inf).
        """
        ...
    def compose(self, other: Circuit, qubits: QubitList | None = ...) -> None:
        """Append another circuit, optionally remapping its qubits."""
        ...
    def to_matrix(self, qubits_order: list[int] | None = ...) -> NDArray[np.complex128]:
        """Compute the dense numeric unitary matrix.

        Args:
            qubits_order: Optional custom qubit ordering (default: natural order).

        Returns:
            A 2D NumPy array (dtype=complex128) of shape (2ⁿ, 2ⁿ).

        Raises:
            CircuitError: If the circuit contains non-unitary operations.
        """
        ...
    def to_symbolic_matrix(
        self, qubits_order: list[int] | None = ...
    ) -> SymbolicMatrix:
        """Compute a dense unitary matrix preserving symbolic parameters.

        Args:
            qubits_order: Optional custom qubit ordering.

        Returns:
            A :class:`SymbolicMatrix` with :class:`Parameter` entries.
        """
        ...
    def validate(self) -> None:
        """Validate classical ownership, dominance, and control-flow invariants.

        Raises:
            CircuitError: If validation fails.
        """
        ...
    def __len__(self) -> int: ...
    def __getitem__(self, index: int) -> ValueOperation:
        """Return an operation by index. Negative indices are supported.

        Raises:
            IndexError: If ``index`` is out of bounds.
        """
        ...
    def __copy__(self) -> Circuit: ...
    def __deepcopy__(self, memo: dict) -> Circuit: ...
    def __repr__(self) -> str: ...
