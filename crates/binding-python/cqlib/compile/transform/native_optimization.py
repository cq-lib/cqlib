# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

"""Exact-physical native fixed-point optimization."""

from ..._native import compile as _compile_module

_native_optimization_module = _compile_module.transform.native_optimization

NativeOptimizationSummary = _native_optimization_module.NativeOptimizationSummary
NativeOptimizationResult = _native_optimization_module.NativeOptimizationResult
NativeQualityPolicy = _native_optimization_module.NativeQualityPolicy
NativeOptimizer = _native_optimization_module.NativeOptimizer

__all__ = [
    "NativeOptimizationSummary",
    "NativeOptimizationResult",
    "NativeQualityPolicy",
    "NativeOptimizer",
]
