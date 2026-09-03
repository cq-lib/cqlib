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

from collections.abc import Sequence
from typing import TypeAlias

from ..circuit import Circuit
from ..device import ExecutionResult
from ..qis import DensityMatrix, Statevector

FigureSize: TypeAlias = tuple[float, float]
QuantumState: TypeAlias = Statevector | DensityMatrix

def draw_text(
    circuit: Circuit,
    *,
    line_width: int | None = None,
    initial_state: bool = False,
    reverse_bits: bool = False,
    show_params: bool = True,
    decompose_circuit_gates: bool = False,
) -> str:
    """Render a circuit as unicode text."""
    ...

def draw_figure(
    circuit: Circuit,
    *,
    fold: int | None = None,
    initial_state: bool = False,
    reverse_bits: bool = False,
    show_params: bool = True,
    decompose_circuit_gates: bool = False,
    output_path: str | None = None,
) -> str:
    """Render a circuit as SVG string.

    The runtime object also supports inline SVG display in notebook frontends.
    """
    ...

def plot_histogram(
    result: ExecutionResult,
    *,
    figsize: FigureSize | None = None,
    color: Sequence[str] | None = None,
    number_to_keep: int | None = None,
    sort: str = "asc",
    target_string: str | None = None,
    legend: Sequence[str] | None = None,
    bar_labels: bool = True,
    title: str | None = None,
    output_path: str | None = None,
) -> str:
    """Render execution-result counts as an SVG histogram."""
    ...

def plot_distribution(
    result: ExecutionResult,
    *,
    figsize: FigureSize | None = None,
    color: Sequence[str] | None = None,
    number_to_keep: int | None = None,
    sort: str = "asc",
    target_string: str | None = None,
    legend: Sequence[str] | None = None,
    bar_labels: bool = True,
    title: str | None = None,
    output_path: str | None = None,
) -> str:
    """Render execution-result counts as a normalized SVG distribution."""
    ...

def plot_bloch_vector(
    vector: Sequence[float],
    *,
    title: str | None = None,
    color: Sequence[str] | None = None,
    alpha: float = 1.0,
    reverse_bits: bool = False,
    figsize: FigureSize | None = None,
    output_path: str | None = None,
) -> str:
    """Render one Bloch vector as SVG."""
    ...

def plot_bloch_multivector(
    state: QuantumState,
    *,
    title: str | None = None,
    color: Sequence[str] | None = None,
    alpha: float = 1.0,
    reverse_bits: bool = False,
    figsize: FigureSize | None = None,
    output_path: str | None = None,
) -> str:
    """Render one reduced Bloch vector per qubit as SVG."""
    ...

def plot_state_city(
    state: QuantumState,
    *,
    title: str | None = None,
    color: Sequence[str] | None = None,
    alpha: float = 1.0,
    reverse_bits: bool = False,
    figsize: FigureSize | None = None,
    output_path: str | None = None,
) -> str:
    """Render real and imaginary density-matrix components as SVG."""
    ...

def plot_state_paulivec(
    state: QuantumState,
    *,
    title: str | None = None,
    color: Sequence[str] | None = None,
    alpha: float = 1.0,
    reverse_bits: bool = False,
    figsize: FigureSize | None = None,
    output_path: str | None = None,
) -> str:
    """Render Pauli-basis expectation values as SVG."""
    ...
