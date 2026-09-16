# Copyright China Telecom Quantum Group 2026
# SPDX-License-Identifier: Apache-2.0

"""Deprecated factories returning native StandardGate instances.

I(t), RXY positional adaptation and legacy gate class inheritance are outside
this compatibility subset. Labels belong to ValueOperation / append_gate.
"""

from __future__ import annotations

from ...circuit import Parameter, StandardGate
from ..._compat.deprecation import warn as _warn


def _gate(name, native_name, params, label):
    if label not in (None, ""):
        raise TypeError(
            "gate labels are unsupported; use append_gate(gate, qubits, label=...) instead"
        )
    _warn(f"cqlib.circuits.gates.{name}()", f"StandardGate.{native_name}")
    return getattr(StandardGate, native_name)(*params)


def H(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.H."""
    return _gate("H", "H", (), label)


def X(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.X."""
    return _gate("X", "X", (), label)


def Y(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.Y."""
    return _gate("Y", "Y", (), label)


def Z(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.Z."""
    return _gate("Z", "Z", (), label)


def S(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.S."""
    return _gate("S", "S", (), label)


def SD(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.SDG."""
    return _gate("SD", "SDG", (), label)


def T(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.T."""
    return _gate("T", "T", (), label)


def TD(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.TDG."""
    return _gate("TD", "TDG", (), label)


def CX(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.CX."""
    return _gate("CX", "CX", (), label)


def CY(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.CY."""
    return _gate("CY", "CY", (), label)


def CZ(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.CZ."""
    return _gate("CZ", "CZ", (), label)


def CCX(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.CCX."""
    return _gate("CCX", "CCX", (), label)


def CNOT(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.CX."""
    return _gate("CNOT", "CX", (), label)


def CCNOT(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.CCX."""
    return _gate("CCNOT", "CCX", (), label)


def SWAP(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.SWAP."""
    return _gate("SWAP", "SWAP", (), label)


def X2P(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.X2P."""
    return _gate("X2P", "X2P", (), label)


def X2M(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.X2M."""
    return _gate("X2M", "X2M", (), label)


def Y2P(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.Y2P."""
    return _gate("Y2P", "Y2P", (), label)


def Y2M(label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.Y2M."""
    return _gate("Y2M", "Y2M", (), label)


def RX(theta: float | Parameter, label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.RX."""
    return _gate("RX", "RX", (theta,), label)


def RY(theta: float | Parameter, label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.RY."""
    return _gate("RY", "RY", (theta,), label)


def RZ(theta: float | Parameter, label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.RZ."""
    return _gate("RZ", "RZ", (theta,), label)


def CRX(theta: float | Parameter, label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.CRX."""
    return _gate("CRX", "CRX", (theta,), label)


def CRY(theta: float | Parameter, label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.CRY."""
    return _gate("CRY", "CRY", (theta,), label)


def CRZ(theta: float | Parameter, label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.CRZ."""
    return _gate("CRZ", "CRZ", (theta,), label)


def XY(theta: float | Parameter, label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.XY."""
    return _gate("XY", "XY", (theta,), label)


def XY2P(theta: float | Parameter, label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.XY2P."""
    return _gate("XY2P", "XY2P", (theta,), label)


def XY2M(theta: float | Parameter, label: str | None = None) -> StandardGate:
    """Deprecated: use StandardGate.XY2M."""
    return _gate("XY2M", "XY2M", (theta,), label)


def U(
    theta: float | Parameter,
    phi: float | Parameter,
    lam: float | Parameter,
    label: str | None = None,
) -> StandardGate:
    """Deprecated: use StandardGate.U."""
    return _gate("U", "U", (theta, phi, lam), label)


def FSIM(
    theta: float | Parameter, phi: float | Parameter, label: str | None = None
) -> StandardGate:
    """Deprecated: use StandardGate.FSIM."""
    return _gate("FSIM", "FSIM", (theta, phi), label)


def RXY(
    theta: float | Parameter, phi: float | Parameter, label: str | None = None
) -> StandardGate:
    """Deprecated factory. Uses the current native (theta, phi) order."""
    return _gate("RXY", "RXY", (theta, phi), label)


__all__ = [
    "RXY",
    "H",
    "X",
    "Y",
    "Z",
    "S",
    "SD",
    "T",
    "TD",
    "CX",
    "CY",
    "CZ",
    "CCX",
    "CNOT",
    "CCNOT",
    "SWAP",
    "X2P",
    "X2M",
    "Y2P",
    "Y2M",
    "RX",
    "RY",
    "RZ",
    "CRX",
    "CRY",
    "CRZ",
    "XY",
    "XY2P",
    "XY2M",
    "U",
    "FSIM",
]
