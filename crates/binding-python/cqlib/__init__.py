# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2025-2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
#
# Any modifications or derivative works of this code must retain this
# copyright notice, and modified files need to carry a notice indicating
# that they have been altered from the originals.

from importlib.metadata import PackageNotFoundError
from importlib.metadata import version as _distribution_version

from .circuit import (
    Circuit,
    CircuitDag,
    CircuitError,
    CircuitGate,
    CqlibError,
    DagControlFlow,
    DagSwitchCase,
    DagWire,
    Directive,
    FrozenCircuit,
    Instruction,
    MCGate,
    Parameter,
    ParameterError,
    Qubit,
    QubitError,
    StandardGate,
    UnitaryGate,
)
from .compile import (
    CompileConfig,
    CompileMode,
    CompileResult,
    CompilerConfigError,
    CompilerError,
    CompilerInternalError,
    CompilerTransformError,
    CompilerWorkflow,
)
from .compile import compile as compile_circuit
from .device import (
    Device,
    ExecutionResult,
    Layout,
    NoiseModel,
    Outcome,
    ReadoutError,
    SingleQubitNoise,
    Status,
    Topology,
    TwoQubitNoise,
)
from .qis import (
    DensityMatrix,
    DensityMatrixNoise,
    Hamiltonian,
    Pauli,
    PauliString,
    Phase,
    Statevector,
    TrotterMode,
)

try:
    __version__ = _distribution_version("cqlib")
except PackageNotFoundError:
    __version__ = "0+unknown"

__all__ = [
    "__version__",
    "Circuit",
    "CircuitDag",
    "CircuitError",
    "CircuitGate",
    "CqlibError",
    "DagControlFlow",
    "DagSwitchCase",
    "DagWire",
    "Directive",
    "FrozenCircuit",
    "Instruction",
    "MCGate",
    "Parameter",
    "ParameterError",
    "Qubit",
    "QubitError",
    "StandardGate",
    "UnitaryGate",
    "CompileConfig",
    "CompileMode",
    "CompileResult",
    "CompilerConfigError",
    "CompilerError",
    "CompilerInternalError",
    "CompilerTransformError",
    "CompilerWorkflow",
    "compile_circuit",
    "Device",
    "ExecutionResult",
    "Layout",
    "NoiseModel",
    "Outcome",
    "ReadoutError",
    "SingleQubitNoise",
    "Status",
    "Topology",
    "TwoQubitNoise",
    "DensityMatrix",
    "DensityMatrixNoise",
    "Hamiltonian",
    "Pauli",
    "PauliString",
    "Phase",
    "Statevector",
    "TrotterMode",
]
