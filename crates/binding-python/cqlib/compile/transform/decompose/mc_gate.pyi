# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

"""Exact multi-controlled gate synthesis primitives.

Every function returns a self-contained list of :class:`ValueOperation`
objects and leaves resource selection to the caller. ``*_n_clean`` and
``*_clean_*`` functions require the supplied ancillas to enter in ``|0>`` and
restore them to ``|0>``. Dirty-ancilla functions accept arbitrary input states
and restore those states exactly. Extra ancillas are ignored only where the
corresponding core algorithm documents that behavior.

All functions raise ``ValueError`` for unsupported gates, duplicate qubits,
or insufficient resources. Functions accepting ``ParamLike`` additionally
raise ``ParameterError`` for non-finite numeric values, and normal Python
``TypeError`` for incompatible argument types.
"""

from __future__ import annotations

from cqlib.circuit import Parameter, Qubit, StandardGate, ValueOperation

QubitLike = int | Qubit
QubitList = list[int] | list[Qubit]
ParamLike = float | Parameter

class Su2RotationAxis:
    """Axis of a multi-controlled special-unitary rotation."""

    @staticmethod
    def x() -> Su2RotationAxis: ...
    @staticmethod
    def y() -> Su2RotationAxis: ...
    @staticmethod
    def z() -> Su2RotationAxis: ...
    @property
    def rotation_gate(self) -> StandardGate: ...
    @property
    def controlled_rotation_gate(self) -> StandardGate: ...
    def __copy__(self) -> Su2RotationAxis: ...
    def __deepcopy__(self, memo: dict[int, object]) -> Su2RotationAxis: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

def decompose_mcx_small(
    controls: QubitList, target: QubitLike
) -> list[ValueOperation]: ...
def decompose_mcx_no_aux(
    controls: QubitList, target: QubitLike
) -> list[ValueOperation]: ...
def decompose_mcx_n_clean(
    controls: QubitList, target: QubitLike, clean_ancillas: QubitList
) -> list[ValueOperation]: ...
def decompose_mcx_n_dirty(
    controls: QubitList, target: QubitLike, dirty_ancillas: QubitList
) -> list[ValueOperation]: ...
def decompose_mcx_1_clean_b95(
    controls: QubitList, target: QubitLike, clean_ancilla: QubitLike
) -> list[ValueOperation]: ...
def decompose_mcx_1_clean_kg24(
    controls: QubitList, target: QubitLike, clean_ancilla: QubitLike
) -> list[ValueOperation]: ...
def decompose_mcx_1_dirty(
    controls: QubitList, target: QubitLike, dirty_ancilla: QubitLike
) -> list[ValueOperation]: ...
def decompose_mcx_2_clean(
    controls: QubitList, target: QubitLike, clean_ancillas: QubitList
) -> list[ValueOperation]: ...
def decompose_mcx_2_dirty(
    controls: QubitList, target: QubitLike, dirty_ancillas: QubitList
) -> list[ValueOperation]: ...
def decompose_mc_su2_no_aux(
    axis: Su2RotationAxis, theta: ParamLike, controls: QubitList, target: QubitLike
) -> list[ValueOperation]: ...
def decompose_mc_su2_n_clean(
    axis: Su2RotationAxis,
    theta: ParamLike,
    controls: QubitList,
    target: QubitLike,
    clean_ancillas: QubitList,
) -> list[ValueOperation]: ...
def decompose_rotation_no_aux(
    rotation: StandardGate, theta: ParamLike, controls: QubitList, target: QubitLike
) -> list[ValueOperation]: ...
def decompose_rotation_n_clean(
    rotation: StandardGate,
    theta: ParamLike,
    controls: QubitList,
    target: QubitLike,
    clean_ancillas: QubitList,
) -> list[ValueOperation]: ...
def decompose_pauli_small(
    pauli: StandardGate, controls: QubitList, target: QubitLike
) -> list[ValueOperation]: ...
def decompose_pauli_no_aux(
    pauli: StandardGate, controls: QubitList, target: QubitLike
) -> list[ValueOperation]: ...
def decompose_pauli_n_clean(
    pauli: StandardGate,
    controls: QubitList,
    target: QubitLike,
    clean_ancillas: QubitList,
) -> list[ValueOperation]: ...
def decompose_pauli_n_dirty(
    pauli: StandardGate,
    controls: QubitList,
    target: QubitLike,
    dirty_ancillas: QubitList,
) -> list[ValueOperation]: ...
def decompose_pauli_1_clean_b95(
    pauli: StandardGate,
    controls: QubitList,
    target: QubitLike,
    clean_ancilla: QubitLike,
) -> list[ValueOperation]: ...
def decompose_pauli_1_clean_kg24(
    pauli: StandardGate,
    controls: QubitList,
    target: QubitLike,
    clean_ancilla: QubitLike,
) -> list[ValueOperation]: ...
def decompose_pauli_1_dirty(
    pauli: StandardGate,
    controls: QubitList,
    target: QubitLike,
    dirty_ancilla: QubitLike,
) -> list[ValueOperation]: ...
def decompose_pauli_2_clean(
    pauli: StandardGate,
    controls: QubitList,
    target: QubitLike,
    clean_ancillas: QubitList,
) -> list[ValueOperation]: ...
def decompose_pauli_2_dirty(
    pauli: StandardGate,
    controls: QubitList,
    target: QubitLike,
    dirty_ancillas: QubitList,
) -> list[ValueOperation]: ...
def decompose_mc_rzz_no_aux(
    theta: ParamLike, controls: QubitList, first: QubitLike, second: QubitLike
) -> list[ValueOperation]: ...
def decompose_mc_rzz_n_clean(
    theta: ParamLike,
    controls: QubitList,
    first: QubitLike,
    second: QubitLike,
    clean_ancillas: QubitList,
) -> list[ValueOperation]: ...
def decompose_pauli_rotation_no_aux(
    rotation: StandardGate,
    theta: ParamLike,
    controls: QubitList,
    first: QubitLike,
    second: QubitLike,
) -> list[ValueOperation]: ...
def decompose_pauli_rotation_n_clean(
    rotation: StandardGate,
    theta: ParamLike,
    controls: QubitList,
    first: QubitLike,
    second: QubitLike,
    clean_ancillas: QubitList,
) -> list[ValueOperation]: ...
def decompose_phase_no_aux(
    phase: StandardGate, theta: ParamLike | None, controls: QubitList, target: QubitLike
) -> list[ValueOperation]: ...
def decompose_phase_n_clean(
    phase: StandardGate,
    theta: ParamLike | None,
    controls: QubitList,
    target: QubitLike,
    clean_ancillas: QubitList,
) -> list[ValueOperation]: ...
def decompose_qcis_no_aux(
    gate: StandardGate, params: list[ParamLike], controls: QubitList, target: QubitLike
) -> list[ValueOperation]: ...
def decompose_qcis_n_clean(
    gate: StandardGate,
    params: list[ParamLike],
    controls: QubitList,
    target: QubitLike,
    clean_ancillas: QubitList,
) -> list[ValueOperation]: ...
def decompose_hadamard_no_aux(
    controls: QubitList, target: QubitLike
) -> list[ValueOperation]: ...
def decompose_hadamard_n_clean(
    controls: QubitList, target: QubitLike, clean_ancillas: QubitList
) -> list[ValueOperation]: ...
def decompose_swap_no_aux(
    controls: QubitList, first: QubitLike, second: QubitLike
) -> list[ValueOperation]: ...
def decompose_swap_n_clean(
    controls: QubitList, first: QubitLike, second: QubitLike, clean_ancillas: QubitList
) -> list[ValueOperation]: ...
def decompose_fsim_no_aux(
    params: list[ParamLike], controls: QubitList, first: QubitLike, second: QubitLike
) -> list[ValueOperation]: ...
def decompose_fsim_n_clean(
    params: list[ParamLike],
    controls: QubitList,
    first: QubitLike,
    second: QubitLike,
    clean_ancillas: QubitList,
) -> list[ValueOperation]: ...
def decompose_unitary_no_aux(
    theta: ParamLike,
    phi: ParamLike,
    lambda_: ParamLike,
    controls: QubitList,
    target: QubitLike,
) -> list[ValueOperation]: ...
def decompose_unitary_n_clean(
    theta: ParamLike,
    phi: ParamLike,
    lambda_: ParamLike,
    controls: QubitList,
    target: QubitLike,
    clean_ancillas: QubitList,
) -> list[ValueOperation]: ...

__all__ = [
    "Su2RotationAxis",
    "decompose_mcx_small",
    "decompose_mcx_no_aux",
    "decompose_mcx_n_clean",
    "decompose_mcx_n_dirty",
    "decompose_mcx_1_clean_b95",
    "decompose_mcx_1_clean_kg24",
    "decompose_mcx_1_dirty",
    "decompose_mcx_2_clean",
    "decompose_mcx_2_dirty",
    "decompose_mc_su2_no_aux",
    "decompose_mc_su2_n_clean",
    "decompose_rotation_no_aux",
    "decompose_rotation_n_clean",
    "decompose_pauli_small",
    "decompose_pauli_no_aux",
    "decompose_pauli_n_clean",
    "decompose_pauli_n_dirty",
    "decompose_pauli_1_clean_b95",
    "decompose_pauli_1_clean_kg24",
    "decompose_pauli_1_dirty",
    "decompose_pauli_2_clean",
    "decompose_pauli_2_dirty",
    "decompose_mc_rzz_no_aux",
    "decompose_mc_rzz_n_clean",
    "decompose_pauli_rotation_no_aux",
    "decompose_pauli_rotation_n_clean",
    "decompose_phase_no_aux",
    "decompose_phase_n_clean",
    "decompose_qcis_no_aux",
    "decompose_qcis_n_clean",
    "decompose_hadamard_no_aux",
    "decompose_hadamard_n_clean",
    "decompose_swap_no_aux",
    "decompose_swap_n_clean",
    "decompose_fsim_no_aux",
    "decompose_fsim_n_clean",
    "decompose_unitary_no_aux",
    "decompose_unitary_n_clean",
]
