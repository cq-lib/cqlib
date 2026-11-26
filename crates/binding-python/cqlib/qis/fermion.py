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

"""Structured fermionic algebra and fermion-to-qubit mappings.

Modes are numbered from zero. A product A * B applies B first. Jordan-Wigner
maps mode p to qubit p, with a Z chain on modes below p; qubit 0 is the least
significant bit. Core imposes no chemical or spin-orbital ordering.

The excitation generator is G = i(T - T†), where targets supply creators in
input order and sources supply annihilators in reverse input order. Evolving
the mapped generator implements exp(-i theta G) = exp(theta (T - T†)).
"""

from __future__ import annotations

from typing import Protocol, Sequence, runtime_checkable

from .._native import qis as _qis_module
from .hamiltonian import Hamiltonian

FermionicLadder = _qis_module.fermion.FermionicLadder
FermionicOp = _qis_module.fermion.FermionicOp
JordanWignerMapper = _qis_module.fermion.JordanWignerMapper


@runtime_checkable
class FermionToQubitMapper(Protocol):
    """Structural contract for operator mappings, including non-Hermitian inputs.

    Implementations need not inherit this protocol. Runtime checks establish
    member presence only; callers must still honor signatures and semantics.
    """

    def map_operator(self, op: FermionicOp) -> Hamiltonian: ...
    def num_qubits(self, num_modes: int) -> int: ...


@runtime_checkable
class OccupationStateMapper(Protocol):
    """Separate contract for computational-basis occupation encodings.

    occupation[p] denotes mode p and returned bits[p] addresses qubit p.
    Not every operator mapping has a computational-basis occupation encoding.
    """

    def map_occupation(self, occupation: Sequence[int]) -> list[bool]: ...


__all__ = [
    "FermionicLadder",
    "FermionicOp",
    "FermionToQubitMapper",
    "OccupationStateMapper",
    "JordanWignerMapper",
]
