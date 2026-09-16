# Copyright China Telecom Quantum Group 2026
# SPDX-License-Identifier: Apache-2.0

"""Deprecated base-exception import; identity is preserved."""

from .circuit import CqlibError
from ._compat.deprecation import warn as _warn

_warn("cqlib.exceptions", "cqlib.circuit")
__all__ = ["CqlibError"]
