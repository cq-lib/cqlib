# Copyright China Telecom Quantum Group 2026
# SPDX-License-Identifier: Apache-2.0

"""Deprecated QASM2 import path; functions are the native IR functions."""

from ...ir.qasm2 import load, loads, dump, dumps
from ..._compat.deprecation import warn as _warn

_warn("cqlib.utils.qasm2", "cqlib.ir.qasm2")
__all__ = ["load", "loads", "dump", "dumps"]
