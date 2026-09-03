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

"""Non-unitary circuit directives.

Directives represent operations that carry instructions to the
backend or compiler but do not have a unitary matrix representation.
They are classified into three categories:

- **Barrier**: Prevents gate reordering across the listed qubits during
  optimisation passes.  Useful for timing-sensitive sequences or when
  the backend must respect a particular gate order for hardware reasons.
- **Measure**: Indicates a mid-circuit projective measurement in the
  computational basis.  On hardware this collapses the qubit; in
  simulation it projects the state vector.
- **Reset**: Forces a qubit into the :math:`|0\\rangle` state regardless
  of its current state, discarding coherence.

Directives are typically created via the static factories on the
:class:`Directive` class.  They are added to a circuit through the
generic :meth:`Circuit.append_gate <cqlib.circuit.Circuit.append_gate>`
method or the convenience helpers (e.g. :meth:`Circuit.barrier
<cqlib.circuit.Circuit.barrier>`, :meth:`Circuit.measure
<cqlib.circuit.Circuit.measure>`).

Example::

    from cqlib.circuit.gates import Directive

    barrier = Directive.barrier()
    measure = Directive.measure()
    reset = Directive.reset()
"""

from __future__ import annotations

from typing_extensions import final

@final
class Directive:
    """A non-unitary instruction in a quantum circuit.

    Not a quantum gate — has no unitary matrix representation.  Use the
    static factory methods to obtain instances; the class cannot be
    instantiated directly.
    """

    @staticmethod
    def barrier() -> Directive:
        """A barrier that prevents gate reordering across the selected qubits.

        The compiler and backend must not reorder operations across a
        barrier.  Commonly inserted between gates that have a known
        hardware inter-dependency that the compiler cannot infer.

        Example::

            barrier = Directive.barrier()
        """
        ...

    @staticmethod
    def measure() -> Directive:
        """A projective measurement in the computational basis.

        Collapses the qubit to :math:`|0\\rangle` or :math:`|1\\rangle`
        with probability given by the squared amplitude.  On a simulator
        this projects the state vector; on hardware it produces a
        classical readout.

        Example::

            measure = Directive.measure()
        """
        ...

    @staticmethod
    def reset() -> Directive:
        """A reset that forces the qubit into the :math:`|0\\rangle` state.

        Discards any stored quantum information on the target qubit.
        After a reset the qubit behaves as if freshly initialised.

        Example::

            reset = Directive.reset()
        """
        ...

    def name(self) -> str:
        """Return the directive name: ``"barrier"``, ``"measure"``, or ``"reset"``."""
        ...

    def is_barrier(self) -> bool:
        """Return ``True`` if this is a barrier."""
        ...

    def is_measure(self) -> bool:
        """Return ``True`` if this is a measure directive."""
        ...

    def is_reset(self) -> bool:
        """Return ``True`` if this is a reset directive."""
        ...

    def inverse(self) -> Directive | None:
        """Return the inverse directive, or ``None`` if it has none.

        A barrier is its own inverse.  Measurement and reset are
        irreversible and return ``None``.
        """
        ...

    def __repr__(self) -> str: ...
    def __copy__(self) -> Directive: ...
    def __deepcopy__(self, memo: dict) -> Directive: ...
    def __str__(self) -> str: ...
