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

from .._native import visualization as _visualization_module
from ..qis import DensityMatrix, Statevector

FigureSize = tuple[float, float]
"""Width/height in inches for figure-rendering functions."""

QuantumState = Statevector | DensityMatrix
"""State types accepted by state-visualization functions."""


class _InlineSvg(str):
    """String SVG wrapper with rich display support for notebook frontends."""

    def _repr_svg_(self) -> str:
        return str(self)


draw_text = _visualization_module.draw_text


def draw_figure(
    circuit,
    *,
    fold=None,
    initial_state=False,
    reverse_bits=False,
    show_params=True,
    decompose_circuit_gates=False,
    output_path=None,
):
    """Render a circuit as SVG.

    In notebook frontends, the return value displays inline when used as the
    last expression in a cell.
    """
    svg = _visualization_module.draw_figure(
        circuit,
        fold=fold,
        initial_state=initial_state,
        reverse_bits=reverse_bits,
        show_params=show_params,
        decompose_circuit_gates=decompose_circuit_gates,
        output_path=output_path,
    )
    return _InlineSvg(svg)


def plot_histogram(
    result,
    *,
    figsize=None,
    color=None,
    number_to_keep=None,
    sort="asc",
    target_string=None,
    legend=None,
    bar_labels=True,
    title=None,
    output_path=None,
):
    """Render execution-result counts as an SVG histogram."""
    svg = _visualization_module.plot_histogram(
        result,
        figsize=figsize,
        color=color,
        number_to_keep=number_to_keep,
        sort=sort,
        target_string=target_string,
        legend=legend,
        bar_labels=bar_labels,
        title=title,
        output_path=output_path,
    )
    return _InlineSvg(svg)


def plot_distribution(
    result,
    *,
    figsize=None,
    color=None,
    number_to_keep=None,
    sort="asc",
    target_string=None,
    legend=None,
    bar_labels=True,
    title=None,
    output_path=None,
):
    """Render execution-result counts as a normalized SVG distribution."""
    svg = _visualization_module.plot_distribution(
        result,
        figsize=figsize,
        color=color,
        number_to_keep=number_to_keep,
        sort=sort,
        target_string=target_string,
        legend=legend,
        bar_labels=bar_labels,
        title=title,
        output_path=output_path,
    )
    return _InlineSvg(svg)


def plot_bloch_vector(
    vector,
    *,
    title=None,
    color=None,
    alpha=1.0,
    reverse_bits=False,
    figsize=None,
    output_path=None,
):
    """Render one Bloch vector as SVG."""
    svg = _visualization_module.plot_bloch_vector(
        vector,
        title=title,
        color=color,
        alpha=alpha,
        reverse_bits=reverse_bits,
        figsize=figsize,
        output_path=output_path,
    )
    return _InlineSvg(svg)


def plot_bloch_multivector(
    state,
    *,
    title=None,
    color=None,
    alpha=1.0,
    reverse_bits=False,
    figsize=None,
    output_path=None,
):
    """Render one reduced Bloch vector per qubit as SVG."""
    svg = _visualization_module.plot_bloch_multivector(
        state,
        title=title,
        color=color,
        alpha=alpha,
        reverse_bits=reverse_bits,
        figsize=figsize,
        output_path=output_path,
    )
    return _InlineSvg(svg)


def plot_state_city(
    state,
    *,
    title=None,
    color=None,
    alpha=1.0,
    reverse_bits=False,
    figsize=None,
    output_path=None,
):
    """Render real and imaginary density-matrix components as SVG."""
    svg = _visualization_module.plot_state_city(
        state,
        title=title,
        color=color,
        alpha=alpha,
        reverse_bits=reverse_bits,
        figsize=figsize,
        output_path=output_path,
    )
    return _InlineSvg(svg)


def plot_state_paulivec(
    state,
    *,
    title=None,
    color=None,
    alpha=1.0,
    reverse_bits=False,
    figsize=None,
    output_path=None,
):
    """Render Pauli-basis expectation values as SVG."""
    svg = _visualization_module.plot_state_paulivec(
        state,
        title=title,
        color=color,
        alpha=alpha,
        reverse_bits=reverse_bits,
        figsize=figsize,
        output_path=output_path,
    )
    return _InlineSvg(svg)


__all__ = [
    "FigureSize",
    "QuantumState",
    "draw_text",
    "draw_figure",
    "plot_histogram",
    "plot_distribution",
    "plot_bloch_vector",
    "plot_bloch_multivector",
    "plot_state_city",
    "plot_state_paulivec",
]
