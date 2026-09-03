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

"""Gate definitions and factory constants.

The ``cqlib.circuit.gates`` submodule provides gate type classes
(:class:`StandardGate`, :class:`UnitaryGate`, :class:`MCGate`,
:class:`CircuitGate`, :class:`Directive`, :class:`FrozenCircuit`) and
36 pre-built standard-gate singleton constants (``H``, ``X``, ``CX``,
``RX``, ``RZ``, etc.) for use with
:meth:`Circuit.append_gate <cqlib.circuit.Circuit.append_gate>`
and :meth:`Circuit.append_unitary_gate
<cqlib.circuit.Circuit.append_unitary_gate>`.

See ``cqlib/circuit/gates/__init__.pyi`` for the complete API reference.
"""

from ..._native.circuit import gate as _gate_module

StandardGate = _gate_module.StandardGate
UnitaryGate = _gate_module.UnitaryGate
MCGate = _gate_module.MCGate
CircuitGate = _gate_module.CircuitGate
FrozenCircuit = _gate_module.FrozenCircuit
Directive = _gate_module.Directive

I = StandardGate.I
H = StandardGate.H
X = StandardGate.X
Y = StandardGate.Y
Z = StandardGate.Z
S = StandardGate.S
SDG = StandardGate.SDG
T = StandardGate.T
TDG = StandardGate.TDG

# --- Parametric Rotation Gates ---
RX = StandardGate.RX
RY = StandardGate.RY
RZ = StandardGate.RZ
U = StandardGate.U
Phase = StandardGate.Phase
GPhase = StandardGate.GPhase

# --- Two Qubit Gates ---
CX = StandardGate.CX
CY = StandardGate.CY
CZ = StandardGate.CZ
SWAP = StandardGate.SWAP
RXX = StandardGate.RXX
RYY = StandardGate.RYY
RZZ = StandardGate.RZZ
RZX = StandardGate.RZX
RXY = StandardGate.RXY
FSIM = StandardGate.FSIM

# --- Multi-Controlled Gates ---
CCX = StandardGate.CCX

# --- Controlled Rotation Gates ---
CRX = StandardGate.CRX
CRY = StandardGate.CRY
CRZ = StandardGate.CRZ

# --- Other Gates ---
XY = StandardGate.XY
X2P = StandardGate.X2P
X2M = StandardGate.X2M
XY2P = StandardGate.XY2P
XY2M = StandardGate.XY2M
Y2P = StandardGate.Y2P
Y2M = StandardGate.Y2M

__all__ = [
    "StandardGate",
    "UnitaryGate",
    "MCGate",
    "CircuitGate",
    "FrozenCircuit",
    "Directive",
    # Single Qubit
    "I",
    "H",
    "X",
    "Y",
    "Z",
    "S",
    "SDG",
    "T",
    "TDG",
    # Parametric
    "RX",
    "RY",
    "RZ",
    "U",
    "Phase",
    "GPhase",
    # Two Qubit
    "CX",
    "CY",
    "CZ",
    "SWAP",
    "RXX",
    "RYY",
    "RZZ",
    "RZX",
    "RXY",
    "FSIM",
    # Multi-Controlled
    "CCX",
    # Controlled Rotation
    "CRX",
    "CRY",
    "CRZ",
    # Other
    "XY",
    "X2P",
    "X2M",
    "XY2P",
    "XY2M",
    "Y2P",
    "Y2M",
]
