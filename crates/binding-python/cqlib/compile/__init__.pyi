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

from . import commutation as commutation
from . import knowledge as knowledge
from . import resource as resource
from . import sabre as sabre
from . import transform as transform
from .compiler import CompileConfig as CompileConfig
from .compiler import CompileMode as CompileMode
from .compiler import CompileResult as CompileResult
from .compiler import CompileTarget as CompileTarget
from .compiler import CompilerWorkflow as CompilerWorkflow
from .compiler import DeviceCompilationMetadata as DeviceCompilationMetadata
from .compiler import DeviceCompileTarget as DeviceCompileTarget
from .compiler import WorkflowStepReport as WorkflowStepReport
from .compiler import compile as compile
from cqlib.circuit import CqlibError

class CompilerError(CqlibError):
    """Base class for compiler pipeline failures."""

    ...

class CompilerConfigError(CompilerError):
    """The compiler input or configuration is invalid."""

    ...

class CompilerTransformError(CompilerError):
    """A compiler transform could not complete its operation."""

    ...

class CompilerInternalError(CompilerError):
    """The compiler violated an internal invariant."""

    ...

__all__ = [
    "commutation",
    "knowledge",
    "resource",
    "sabre",
    "transform",
    "CompileMode",
    "DeviceCompileTarget",
    "CompileTarget",
    "CompileConfig",
    "WorkflowStepReport",
    "DeviceCompilationMetadata",
    "CompileResult",
    "CompilerWorkflow",
    "CompilerError",
    "CompilerConfigError",
    "CompilerTransformError",
    "CompilerInternalError",
    "compile",
]
