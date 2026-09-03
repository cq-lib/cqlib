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

"""Compiler pipeline bindings.

Use :func:`compile` for the standard end-to-end workflow. The package also
exposes lower-level transform, routing, resource, and knowledge modules for
callers that need to assemble or inspect individual compiler stages.

Example::

    from cqlib.circuit import Circuit
    from cqlib.compile import compile

    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)

    result = compile(circuit)
    assert result.circuit is not circuit
"""

from . import commutation as commutation
from . import knowledge as knowledge
from . import resource as resource
from . import sabre as sabre
from . import transform as transform
from .compiler import (
    CompileConfig,
    CompileMode,
    CompileResult,
    CompileTarget,
    CompilerWorkflow,
    DeviceCompilationMetadata,
    DeviceCompileTarget,
    WorkflowStepReport,
    compile,
)
from .._native import compile as _compile_module

CompilerError = _compile_module.CompilerError
CompilerConfigError = _compile_module.CompilerConfigError
CompilerTransformError = _compile_module.CompilerTransformError
CompilerInternalError = _compile_module.CompilerInternalError

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
