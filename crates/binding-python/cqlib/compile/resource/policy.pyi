# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

from __future__ import annotations

class ResourcePolicy:
    """Permissions governing ancillary resources used by the compiler.

    Policy is distinct from device capacity. It permits the compiler to create
    a bounded number of clean logical ancillas before layout and optionally to
    borrow input qubits under the stronger dirty-restoration contract.

    The long ``max_pre_layout_clean_ancillas`` keyword is intentionally explicit
    because it names when the compiler may create clean ancillas. Prefer binding
    the policy to a local variable when passing it into several compile calls.

    Args:
        max_pre_layout_clean_ancillas: Maximum number of clean logical qubits
            the compiler may create. Released qubits remain in this total and
            can be reused.
        allow_dirty_borrowing: Whether input qubits may satisfy dirty requests.
            This never makes input qubits eligible for clean-zero requests.

    Example::

        policy = ResourcePolicy(
            max_pre_layout_clean_ancillas=2,
            allow_dirty_borrowing=True,
        )
    """

    def __init__(
        self,
        *,
        max_pre_layout_clean_ancillas: int = 0,
        allow_dirty_borrowing: bool = False,
    ) -> None: ...
    @property
    def max_pre_layout_clean_ancillas(self) -> int:
        """Maximum permitted pre-layout clean logical ancillas."""
        ...
    @property
    def allow_dirty_borrowing(self) -> bool:
        """Whether input qubits may be borrowed and restored exactly."""
        ...
    def __copy__(self) -> ResourcePolicy: ...
    def __deepcopy__(self, memo: dict) -> ResourcePolicy: ...
    def __eq__(self, other: object) -> bool: ...

class ResourceLimits:
    """Hard bounds on the complete logical circuit.

    Args:
        max_total_qubits: Maximum total logical qubits, including input and
            compiler-created qubits. ``None`` disables this manager-level bound.

    ``cqlib.compile.compile`` derives this bound from its target device, so
    users normally construct limits only for a standalone ``ResourceManager``.
    """

    def __init__(self, *, max_total_qubits: int | None = None) -> None: ...
    @property
    def max_total_qubits(self) -> int | None:
        """Maximum total logical qubits, or ``None`` when unrestricted."""
        ...
    def __copy__(self) -> ResourceLimits: ...
    def __deepcopy__(self, memo: dict) -> ResourceLimits: ...
    def __eq__(self, other: object) -> bool: ...

__all__ = [
    "ResourcePolicy",
    "ResourceLimits",
]
