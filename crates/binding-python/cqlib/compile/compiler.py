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

"""Public compiler entry point."""

from .._native import compile as _compile_module

CompileMode = _compile_module.CompileMode
DeviceCompileTarget = _compile_module.DeviceCompileTarget
CompileTarget = _compile_module.CompileTarget
CompileConfig = _compile_module.CompileConfig
WorkflowStepReport = _compile_module.WorkflowStepReport
DeviceCompilationMetadata = _compile_module.DeviceCompilationMetadata
CompileResult = _compile_module.CompileResult
CompilerWorkflow = _compile_module.CompilerWorkflow
compile = _compile_module.compile

__all__ = [
    "CompileMode",
    "DeviceCompileTarget",
    "CompileTarget",
    "CompileConfig",
    "WorkflowStepReport",
    "DeviceCompilationMetadata",
    "CompileResult",
    "CompilerWorkflow",
    "compile",
]
