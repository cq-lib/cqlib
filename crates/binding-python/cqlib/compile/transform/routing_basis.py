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

"""Public bridge to pre-routing basis legalization."""

from ..._native import compile as _compile_module

_transform_module = _compile_module.transform

LowerToRoutingBasis = _transform_module.LowerToRoutingBasis
lower_to_routing_basis = _transform_module.lower_to_routing_basis

__all__ = [
    "LowerToRoutingBasis",
    "lower_to_routing_basis",
]
