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

from __future__ import annotations

from collections.abc import Sequence
from typing import Literal

from cqlib.circuit import Circuit, Instruction
from cqlib.device import Device, Layout
from .resource import ResourcePolicy

class CompileMode:
    """Optimization effort selected for the compiler workflow."""

    @staticmethod
    def normal() -> CompileMode: ...
    @staticmethod
    def enhanced() -> CompileMode:
        """Use higher optimization effort and validated Pareto route selection for strict devices."""
        ...
    def __copy__(self) -> CompileMode: ...
    def __deepcopy__(self, memo: dict[int, object]) -> CompileMode: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class DeviceCompileTarget:
    """Immutable device-specific compilation inputs.

    The device and optional layout are snapshotted during construction.
    """

    def __init__(
        self,
        device: Device,
        *,
        initial_layout: Layout | None = None,
        seed: int | None = None,
    ) -> None:
        """Snapshot a device, optional initial layout, and routing seed."""
        ...
    @property
    def device(self) -> Device:
        """Return a copy of the target device."""
        ...
    @property
    def initial_layout(self) -> Layout | None:
        """Return a copy of the supplied layout, if any."""
        ...
    @property
    def seed(self) -> int | None:
        """Deterministic seed used only by layout and routing heuristics."""
        ...
    def __copy__(self) -> DeviceCompileTarget: ...
    def __deepcopy__(self, memo: dict[int, object]) -> DeviceCompileTarget: ...
    def __repr__(self) -> str: ...

class CompileTarget:
    """Mutually exclusive logical, basis, or physical-device target.

    Construct targets through the static factories; direct construction is not
    supported.
    """

    @staticmethod
    def logical() -> CompileTarget:
        """Keep compilation in logical-qubit space."""
        ...
    @staticmethod
    def basis(instructions: Sequence[str | Instruction]) -> CompileTarget:
        """Lower to a non-empty explicit standard-instruction basis."""
        ...
    @staticmethod
    def device(
        device: Device,
        *,
        initial_layout: Layout | None = None,
        seed: int | None = None,
    ) -> CompileTarget:
        """Route, lower, and validate for one concrete device."""
        ...
    @staticmethod
    def topology_basis(
        device: Device,
        instructions: Sequence[str | Instruction],
        *,
        initial_layout: Layout | None = None,
        seed: int | None = None,
    ) -> CompileTarget:
        """Route on a device topology and lower to an explicit basis."""
        ...
    @property
    def kind(self) -> Literal["logical", "basis", "device", "topology_basis"]: ...
    @property
    def basis_instructions(self) -> list[Instruction] | None: ...
    @property
    def device_target(self) -> DeviceCompileTarget | None: ...
    def __copy__(self) -> CompileTarget: ...
    def __deepcopy__(self, memo: dict[int, object]) -> CompileTarget: ...
    def __repr__(self) -> str: ...

class CompileConfig:
    """Immutable compiler workflow configuration snapshot.

    ``target=None`` selects :meth:`CompileTarget.logical`.
    """

    def __init__(
        self,
        *,
        mode: CompileMode | Literal["normal", "enhanced"] | None = None,
        target: CompileTarget | None = None,
        resource_policy: ResourcePolicy | None = None,
    ) -> None:
        """Create an immutable configuration snapshot.

        ``None`` values select normal mode, logical compilation, and the
        conservative resource policy respectively.
        """
        ...
    @property
    def mode(self) -> CompileMode: ...
    @property
    def target(self) -> CompileTarget: ...
    @property
    def resource_policy(self) -> ResourcePolicy: ...
    def __copy__(self) -> CompileConfig: ...
    def __deepcopy__(self, memo: dict[int, object]) -> CompileConfig: ...
    def __repr__(self) -> str: ...

class WorkflowStepReport:
    """Per-step execution record produced by a compiler workflow run."""

    @property
    def stage(self) -> str: ...
    @property
    def name(self) -> str: ...
    @property
    def changed(self) -> bool: ...
    @property
    def skipped(self) -> bool: ...
    @property
    def reason(self) -> str | None: ...
    def __copy__(self) -> WorkflowStepReport: ...
    def __deepcopy__(self, memo: dict[int, object]) -> WorkflowStepReport: ...
    def __repr__(self) -> str: ...

class DeviceCompilationMetadata:
    """Initial and final physical layouts from device compilation."""

    @property
    def initial_layout(self) -> Layout:
        """Logical-to-physical layout before routing."""
        ...
    @property
    def final_layout(self) -> Layout:
        """Logical-to-physical layout after routed swaps."""
        ...
    def __copy__(self) -> DeviceCompilationMetadata: ...
    def __deepcopy__(self, memo: dict[int, object]) -> DeviceCompilationMetadata: ...
    def __eq__(self, other: object) -> bool: ...
    def __repr__(self) -> str: ...

class CompileResult:
    """Compiled circuit and ordered workflow diagnostics."""

    @property
    def circuit(self) -> Circuit: ...
    @property
    def changed(self) -> bool: ...
    @property
    def mode(self) -> CompileMode: ...
    @property
    def steps(self) -> list[WorkflowStepReport]:
        """Reports for the retained output path in workflow order.

        Enhanced strict-device runs summarize discarded route branches in the
        final ``select.sabre_pareto_beam`` report.
        """
        ...
    @property
    def device_metadata(self) -> DeviceCompilationMetadata | None:
        """Physical layouts for a device target, otherwise ``None``."""
        ...
    def step(self, name: str) -> WorkflowStepReport | None:
        """Return the first report with ``name``."""
        ...
    def step_changed(self, name: str) -> bool:
        """Return whether a non-skipped report with ``name`` changed the circuit."""
        ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> CompileResult: ...
    def __deepcopy__(self, memo: dict[int, object]) -> CompileResult: ...
    def __repr__(self) -> str: ...

class CompilerWorkflow:
    """Reusable compiler workflow owning an immutable configuration snapshot."""

    def __init__(self, config: CompileConfig | None = None) -> None: ...
    @property
    def config(self) -> CompileConfig: ...
    def run(self, circuit: Circuit) -> CompileResult:
        """Compile without modifying ``circuit``.

        Raises:
            CompilerConfigError: If target configuration is invalid.
            CompilerTransformError: If lowering, routing, or validation fails.
            CompilerInternalError: If an internal invariant is violated.
            CircuitError: If circuit structure validation fails.
        """
        ...

def compile(
    circuit: Circuit,
    *,
    mode: CompileMode | Literal["normal", "enhanced"] | None = None,
    target: CompileTarget | None = None,
    target_basis: Sequence[str | Instruction] | None = None,
    device: Device | None = None,
    initial_layout: Layout | None = None,
    resource_policy: ResourcePolicy | None = None,
    seed: int | None = None,
) -> CompileResult:
    """Compile ``circuit`` for an explicit target or loose constraints.

    ``device`` alone selects strict device-native compilation. Combining
    ``device`` with ``target_basis`` uses the device for capacity, layout, and
    routing while the explicit basis constrains the output; this loose form
    does not guarantee device-native compatibility. ``target`` cannot be
    combined with ``device`` or ``target_basis``. Per-run ``initial_layout``
    and ``seed`` options may accompany an explicit physical target.

    Raises:
        CompilerConfigError: If target configuration is invalid.
        CompilerTransformError: If lowering, routing, or validation fails.
        CompilerInternalError: If an internal invariant is violated.
        CircuitError: If circuit structure validation fails.
    """
    ...

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
