# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

"""Pre-layout virtual-permutation elision."""

from ..._native import compile as _compile_module

_virtual_permutation_module = _compile_module.transform.virtual_permutation

VirtualPermutationElisionResult = (
    _virtual_permutation_module.VirtualPermutationElisionResult
)
elide_virtual_permutations = _virtual_permutation_module.elide_virtual_permutations

__all__ = ["VirtualPermutationElisionResult", "elide_virtual_permutations"]
