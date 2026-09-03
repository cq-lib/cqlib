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

"""Frozen circuits and circuit-defined composite gates.

Building reusable custom gates
-------------------------------

:class:`FrozenCircuit` captures an immutable circuit definition.  Once frozen,
it can be wrapped by a :class:`CircuitGate` and reused across multiple circuits
as a single composite gate::

    from cqlib import Circuit, Qubit
    from cqlib.circuit.gates import FrozenCircuit, CircuitGate

    # Build a 2-qubit Bell-pair sub-circuit
    bell = Circuit(2)
    bell.h(0)
    bell.cx(0, 1)

    # Freeze and wrap as a reusable gate
    frozen = FrozenCircuit(
        qubits=[Qubit(0), Qubit(1)],
        operations=bell.operations(),
    )
    bell_gate = CircuitGate("Bell", frozen)

    # Use it in a larger circuit
    big = Circuit(4)
    big.append_circuit_gate(bell_gate, [0, 1])
    big.append_circuit_gate(bell_gate, [2, 3])

When the sub-circuit carries symbolic parameters, bind them positionally
via ``params`` on :meth:`~cqlib.circuit.Circuit.append_circuit_gate`.

Typical lifecycle
-----------------

1. Build a circuit with :class:`~cqlib.circuit.Circuit`.
2. Freeze it with :class:`FrozenCircuit`, optionally via
   :meth:`~cqlib.circuit.Circuit.to_gate(name)` which returns a ready-made
   :class:`CircuitGate`.
3. Apply the gate to other circuits via
   :meth:`~cqlib.circuit.Circuit.append_circuit_gate`.

Classes
-------

- :class:`FrozenCircuit` — immutable, validated snapshot of a circuit's
  :class:`~cqlib.circuit.ValueOperation` list and qubit layout.
- :class:`CircuitGate` — named, reusable composite gate backed by a frozen
  circuit definition.
"""

from ..bit import Qubit
from ..classical import ClassicalType
from ..operation import ValueOperation

class FrozenCircuit:
    """Immutable circuit snapshot suitable for use inside a gate definition.

    Created from construction-IR operations.  Once built, the internal
    operation sequence cannot mutate, guaranteeing that every
    :class:`CircuitGate` that references this definition observes the same
    behaviour.

    Use :class:`FrozenCircuit` directly when you need to keep the raw
    circuit around for inspection or custom gate logic.  For the common
    case, :meth:`Circuit.to_gate(name) <cqlib.circuit.Circuit.to_gate>`
    returns a ready-made :class:`CircuitGate` without exposing this type.
    """

    def __init__(
        self,
        qubits: list[Qubit],
        operations: list[ValueOperation],
        classical_vars: list[ClassicalType] | None = ...,
        classical_values: list[ClassicalType] | None = ...,
    ) -> None:
        """Create a frozen circuit from construction-IR parts.

        Args:
            qubits: Qubits in storage order.
            operations: Serialised :class:`~cqlib.circuit.ValueOperation`
                entries.
            classical_vars: Types of mutable classical variables, if any.
            classical_values: Types of immutable classical values produced
                by measurement, if any.

        Raises:
            CircuitError: If the circuit IR does not pass validation.
        """
        ...

    @property
    def qubits(self) -> list[Qubit]:
        """Qubits in storage order."""
        ...

    @property
    def num_operations(self) -> int:
        """Number of stored operations."""
        ...

    @property
    def operations(self) -> list[ValueOperation]:
        """Self-contained operations with circuit parameters resolved.

        Each returned :class:`~cqlib.circuit.ValueOperation` carries
        its own :class:`~cqlib.circuit.Parameter` values, not
        circuit-local parameter-table indices.
        """
        ...

    @property
    def symbols(self) -> list[str]:
        """Interned symbol names in stable circuit insertion order.

        This registry may contain symbols no longer referenced by executable
        IR. Use :attr:`used_symbols` for live dependencies.
        """
        ...

    @property
    def used_symbols(self) -> list[str]:
        """Symbols actually referenced by the frozen executable IR."""
        ...

    def __copy__(self) -> FrozenCircuit: ...
    def __deepcopy__(self, memo: dict) -> FrozenCircuit: ...
    def __repr__(self) -> str: ...

class CircuitGate:
    """Composite gate defined by an immutable :class:`FrozenCircuit`.

    Args:
        name: A descriptive name for the gate (appears in circuit
            visualisations and serialisation).
        circuit: The frozen circuit definition backing this gate.
    """

    def __init__(
        self,
        name: str,
        circuit: FrozenCircuit,
        signature_params: list[str] | None = ...,
    ) -> None:
        """Create a named gate from a frozen circuit.

        If ``signature_params`` is omitted, the positional signature is
        inferred from the backing circuit's live symbols. An explicit
        signature may contain declared but unused parameters.

        The backing circuit may contain directives (e.g. measure or
        reset); they are accepted here and only rejected later, when a
        unitary matrix or the inverse is requested.

        Raises:
            CircuitError: If a live symbol is undeclared, or if the
                explicit signature contains duplicate names.
        """
        ...

    @staticmethod
    def with_signature(
        name: str, circuit: FrozenCircuit, signature_params: list[str]
    ) -> CircuitGate:
        """Create a gate with an explicit positional parameter signature."""
        ...

    @property
    def name(self) -> str:
        """The gate name."""
        ...

    @property
    def num_qubits(self) -> int:
        """Number of qubits used by the definition."""
        ...

    @property
    def num_params(self) -> int:
        """Number of positional symbolic parameters accepted by this gate."""
        ...

    @property
    def signature_params(self) -> list[str]:
        """Positional parameter names accepted when invoking this gate."""
        ...

    @property
    def used_symbols(self) -> list[str]:
        """Symbols actually referenced by the backing circuit."""
        ...

    @property
    def symbols(self) -> list[str]:
        """Backward-compatible alias for :attr:`used_symbols`.

        This is not the positional call signature; use
        :attr:`signature_params` for that purpose.
        """
        ...

    @property
    def circuit(self) -> FrozenCircuit:
        """The immutable circuit definition."""
        ...

    def inverse(self) -> CircuitGate:
        """Return the inverse circuit gate.

        The inverse reverses every operation in the underlying circuit and
        preserves the same parameter slots.  If any operation is not
        invertible a ``CircuitError`` is raised.

        Raises:
            CircuitError: When the underlying circuit contains irreversible
                operations.
        """
        ...

    def __eq__(self, other: object) -> bool:
        """Equality compares logical identity (name and circuit definition).

        Two gates with the same name but different frozen circuits are not
        considered equal.
        """
        ...

    def __copy__(self) -> CircuitGate: ...
    def __deepcopy__(self, memo: dict) -> CircuitGate: ...
    def __repr__(self) -> str: ...
