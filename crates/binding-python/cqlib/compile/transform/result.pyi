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

"""Common compiler-transform result type."""

from __future__ import annotations

from cqlib.circuit import Circuit

class TransformResult:
    """Common result returned by circuit-to-circuit compiler transforms."""

    @property
    def circuit(self) -> Circuit:
        """Transformed circuit owned by this result."""
        ...
    @property
    def changed(self) -> bool:
        """Whether the transform changed the compiler IR representation."""
        ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> TransformResult: ...
    def __deepcopy__(self, memo: dict[int, object]) -> TransformResult: ...

__all__ = [
    "TransformResult",
]
