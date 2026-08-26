# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

"""Pre-layout virtual-permutation elision."""

from __future__ import annotations

from typing import Literal

from cqlib.circuit import Circuit

class VirtualPermutationElisionResult:
    @property
    def circuit(self) -> Circuit: ...
    @property
    def changed(self) -> bool: ...
    @property
    def status(self) -> Literal["unchanged", "changed", "skipped_control_flow"]: ...
    @property
    def elided_swap_count(self) -> int: ...
    @property
    def virtual_permutation(self) -> dict[int, int]: ...
    def __repr__(self) -> str: ...
    def __copy__(self) -> VirtualPermutationElisionResult: ...
    def __deepcopy__(
        self, memo: dict[int, object]
    ) -> VirtualPermutationElisionResult: ...

def elide_virtual_permutations(
    circuit: Circuit,
) -> VirtualPermutationElisionResult: ...

__all__ = ["VirtualPermutationElisionResult", "elide_virtual_permutations"]
