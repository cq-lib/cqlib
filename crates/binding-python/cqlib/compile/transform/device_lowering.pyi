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

from __future__ import annotations

from cqlib.circuit import Circuit
from cqlib.device import Device
from .result import TransformResult

class DeviceLowerer:
    """Lower a routed circuit to exact ordered native device capabilities."""

    def __init__(self, device: Device) -> None: ...
    @property
    def device(self) -> Device: ...
    def run(self, circuit: Circuit) -> TransformResult:
        """Return a native circuit without modifying ``circuit``.

        Raises:
            CompilerConfigError: If high-level operations were not decomposed.
            CompilerTransformError: If no exact lowering plan exists.
            CompilerInternalError: If planning violates an internal invariant.
            CircuitError: If circuit rebuilding fails.
        """
        ...
    def __copy__(self) -> DeviceLowerer: ...
    def __deepcopy__(self, memo: dict[int, object]) -> DeviceLowerer: ...
    def __repr__(self) -> str: ...

__all__ = [
    "DeviceLowerer",
]
