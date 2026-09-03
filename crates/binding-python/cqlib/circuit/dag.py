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

"""Runtime exports for circuit DAG analysis types."""

from .._native import circuit as _circuit_module

DagWire = _circuit_module.DagWire
DagSwitchCase = _circuit_module.DagSwitchCase
DagControlFlow = _circuit_module.DagControlFlow
CircuitDag = _circuit_module.CircuitDag

__all__ = ["DagWire", "DagSwitchCase", "DagControlFlow", "CircuitDag"]
