# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

"""Numeric two-qubit block resynthesis transforms."""

from __future__ import annotations

from typing import Literal

from cqlib.circuit import Circuit, Instruction
from cqlib.compile.commutation import CommutationConfig
from cqlib.device import Device
from .decompose import TwoQubitUnitaryDecomposeBasis
from .result import TransformResult

class TwoQubitBlockResynthesisConfig:
    """Configuration for bounded numeric two-qubit block resynthesis."""

    def __init__(
        self,
        *,
        two_qubit_basis: TwoQubitUnitaryDecomposeBasis | None = None,
        target_basis: list[Instruction | str] | None = None,
        enhanced: bool = False,
        max_block_ops: int | None = None,
        max_crossed_ops: int | None = None,
        max_scan_span: int | None = None,
        skip_labeled_ops: bool = True,
        recurse_control_flow: bool = True,
        commutation: CommutationConfig | None = None,
    ) -> None: ...
    @property
    def two_qubit_basis(self) -> TwoQubitUnitaryDecomposeBasis | None:
        """Basis used for synthesized two-qubit interaction gates.

        ``None`` when the config uses the unconstrained core default target.
        """
        ...
    @property
    def target_basis(self) -> list[Instruction] | None:
        """Explicit target basis used for two-qubit synthesis candidate choice.

        ``None`` when the config uses the unconstrained core default target or
        the legacy ``two_qubit_basis`` selection. Mutually exclusive with
        ``two_qubit_basis``.
        """
        ...
    @property
    def max_block_ops(self) -> int: ...
    @property
    def max_crossed_ops(self) -> int: ...
    @property
    def max_scan_span(self) -> int: ...
    @property
    def skip_labeled_ops(self) -> bool: ...
    @property
    def recurse_control_flow(self) -> bool: ...
    @property
    def commutation(self) -> CommutationConfig: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> TwoQubitBlockResynthesisConfig: ...
    def __deepcopy__(
        self, memo: dict[int, object]
    ) -> TwoQubitBlockResynthesisConfig: ...

class ResynthesizeTwoQubitBlocks:
    """Reusable numeric two-qubit block resynthesis transformer."""

    def __init__(
        self, config: TwoQubitBlockResynthesisConfig | None = None
    ) -> None: ...
    @property
    def config(self) -> TwoQubitBlockResynthesisConfig: ...
    def run(self, circuit: Circuit) -> TransformResult: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> ResynthesizeTwoQubitBlocks: ...
    def __deepcopy__(self, memo: dict[int, object]) -> ResynthesizeTwoQubitBlocks: ...

def resynthesize_two_qubit_blocks(
    circuit: Circuit,
    config: TwoQubitBlockResynthesisConfig | None = None,
) -> TransformResult:
    """Resynthesize strictly improving fixed numeric two-qubit blocks."""
    ...

def resynthesize_two_qubit_blocks_for_device(
    circuit: Circuit,
    device: Device,
    *,
    placement: Literal["pre_layout_envelope", "exact_physical"] = "pre_layout_envelope",
    config: TwoQubitBlockResynthesisConfig | None = None,
) -> TransformResult:
    """Resynthesize using the device-aware workflow cost policy.

    ``pre_layout_envelope`` treats circuit qubits as logical and requires a
    candidate to remain suitable over possible placements. ``exact_physical``
    requires an already routed physical circuit.
    """
    ...

__all__ = [
    "TwoQubitBlockResynthesisConfig",
    "ResynthesizeTwoQubitBlocks",
    "resynthesize_two_qubit_blocks",
    "resynthesize_two_qubit_blocks_for_device",
]
