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

"""Tests for visualization Python bindings."""

from __future__ import annotations

from dataclasses import dataclass
from functools import cache
import hashlib
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import zlib

import pytest
import resvg_py

import cqlib
import cqlib._native as cqlib_native
from cqlib import Circuit
from cqlib.device import ExecutionResult
from cqlib.qis import DensityMatrix, Statevector
import cqlib.visualization as vis

_PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
_FIGURE_ROOT = Path(__file__).resolve().parent / "figure"
_FONT_URL = "https://cdn.jsdelivr.net/npm/pdfjs-dist@5.4.624/standard_fonts/LiberationSans-Regular.ttf"
_FONT_SHA256 = "f8ace1f892b2bd9dc1792ba7f097fa7588f84fed48321480e04de5390828221f"


@cache
def _visual_font() -> Path:
    """Use the same verified font as the Rust tests, including installed-wheel tests."""
    override = os.environ.get("CQLIB_VISUAL_FONT")
    cache_root = Path(os.environ.get("CARGO_TARGET_DIR", tempfile.gettempdir()))
    path = (
        Path(override)
        if override
        else cache_root / "cqlib-visual-fonts" / "LiberationSans-Regular.ttf"
    )
    if path.is_file() and hashlib.sha256(path.read_bytes()).hexdigest() == _FONT_SHA256:
        return path
    assert not override, f"{path} is missing or has the wrong SHA-256"
    result = subprocess.run(
        [
            "curl",
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--max-time",
            "45",
            "--retry",
            "1",
            _FONT_URL,
        ],
        capture_output=True,
        check=False,
        timeout=100,
    )
    assert (
        result.returncode == 0
        and hashlib.sha256(result.stdout).hexdigest() == _FONT_SHA256
    ), (
        f"visual-test font download failed or SHA-256 mismatch: {result.stderr.decode(errors='replace')}; "
        "set CQLIB_VISUAL_FONT to a verified font for offline tests"
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(result.stdout)
    return path


@dataclass(frozen=True)
class _RgbImage:
    width: int
    height: int
    data: bytes


@dataclass(frozen=True)
class _VisualCasePaths:
    actual_svg: Path
    actual_png: Path
    exported_png: Path
    reference_png: Path
    diff_png: Path


def _bell_circuit():
    """Create a small Bell-state circuit for visualization tests."""
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)
    circuit.measure(0)
    circuit.measure(1)
    return circuit


def _folded_circuit():
    """Create a circuit that exercises circuit figure options."""
    circuit = Circuit(3)
    circuit.h(0)
    circuit.rx(1, 0.25)
    circuit.cx(0, 2)
    circuit.barrier([0, 1, 2])
    circuit.swap(1, 2)
    circuit.measure(0)
    circuit.measure(1)
    circuit.measure(2)
    return circuit


def _execution_result(counts: dict[str, int], num_qubits: int):
    """Create a completed execution result with non-empty counts."""
    return ExecutionResult.from_counts(
        "vis-test",
        list(range(num_qubits)),
        sum(counts.values()),
        num_qubits,
        counts,
    )


def _zero_statevector():
    """Create a one-qubit |0> statevector."""
    return Statevector(1)


def _plus_statevector():
    """Create a one-qubit |+> statevector."""
    state = Statevector(1)
    state.apply_h(0)
    return state


def _plus_density_matrix():
    """Create a one-qubit |+><+| density matrix."""
    state = DensityMatrix(1)
    state.apply_h(0)
    return state


def _reverse_bits_density_matrix():
    """Create a two-qubit diagonal density matrix with asymmetric basis weights."""
    return DensityMatrix.from_density_matrix(
        2,
        [
            0.05 + 0j,
            0.0 + 0j,
            0.0 + 0j,
            0.0 + 0j,
            0.0 + 0j,
            0.15 + 0j,
            0.0 + 0j,
            0.0 + 0j,
            0.0 + 0j,
            0.0 + 0j,
            0.75 + 0j,
            0.0 + 0j,
            0.0 + 0j,
            0.0 + 0j,
            0.0 + 0j,
            0.05 + 0j,
        ],
    )


def _assert_in_order(text: str, needles: list[str]) -> None:
    offset = 0
    for needle in needles:
        found = text.find(needle, offset)
        assert found >= 0, needle
        offset = found + len(needle)


def _visual_threshold():
    return float(os.environ.get("CQLIB_VISUAL_THRESHOLD", "0.995"))


def _visual_case_paths(filename: str) -> _VisualCasePaths:
    references_dir = _FIGURE_ROOT / "references"
    diffs_dir = _FIGURE_ROOT / "diffs"

    _FIGURE_ROOT.mkdir(parents=True, exist_ok=True)
    references_dir.mkdir(parents=True, exist_ok=True)
    diffs_dir.mkdir(parents=True, exist_ok=True)

    return _VisualCasePaths(
        actual_svg=_FIGURE_ROOT / filename.replace(".png", ".svg"),
        actual_png=_FIGURE_ROOT / filename,
        exported_png=_FIGURE_ROOT / filename.replace(".png", "_export.png"),
        reference_png=references_dir / filename,
        diff_png=diffs_dir / f"diff_{filename}",
    )


def _png_chunks(path: Path):
    raw = path.read_bytes()
    if not raw.startswith(_PNG_SIGNATURE):
        raise AssertionError(f"not a PNG file: {path}")

    offset = len(_PNG_SIGNATURE)
    while offset < len(raw):
        if offset + 8 > len(raw):
            raise AssertionError(f"truncated PNG chunk header: {path}")
        length = struct.unpack(">I", raw[offset : offset + 4])[0]
        kind = raw[offset + 4 : offset + 8]
        offset += 8
        data = raw[offset : offset + length]
        offset += length + 4
        yield kind, data
        if kind == b"IEND":
            break


def _paeth_predictor(left: int, up: int, upper_left: int) -> int:
    p = left + up - upper_left
    pa = abs(p - left)
    pb = abs(p - up)
    pc = abs(p - upper_left)
    if pa <= pb and pa <= pc:
        return left
    if pb <= pc:
        return up
    return upper_left


def _unfilter_png_rows(
    payload: bytes, width: int, height: int, channels: int
) -> list[bytes]:
    row_len = width * channels
    rows: list[bytes] = []
    offset = 0
    previous = bytes(row_len)

    for _ in range(height):
        filter_type = payload[offset]
        offset += 1
        raw = payload[offset : offset + row_len]
        offset += row_len
        row = bytearray(row_len)

        for idx, value in enumerate(raw):
            left = row[idx - channels] if idx >= channels else 0
            up = previous[idx]
            upper_left = previous[idx - channels] if idx >= channels else 0
            if filter_type == 0:
                decoded = value
            elif filter_type == 1:
                decoded = value + left
            elif filter_type == 2:
                decoded = value + up
            elif filter_type == 3:
                decoded = value + ((left + up) // 2)
            elif filter_type == 4:
                decoded = value + _paeth_predictor(left, up, upper_left)
            else:
                raise AssertionError(f"unsupported PNG filter type: {filter_type}")
            row[idx] = decoded & 0xFF

        previous = bytes(row)
        rows.append(previous)

    return rows


def _load_png_rgb(path: Path) -> _RgbImage:
    width = height = bit_depth = color_type = interlace = None
    idat_parts: list[bytes] = []

    for kind, data in _png_chunks(path):
        if kind == b"IHDR":
            width, height, bit_depth, color_type, _, _, interlace = struct.unpack(
                ">IIBBBBB", data
            )
        elif kind == b"IDAT":
            idat_parts.append(data)

    if width is None or height is None or bit_depth != 8 or interlace != 0:
        raise AssertionError(f"unsupported PNG format: {path}")
    if color_type not in (2, 6):
        raise AssertionError(f"unsupported PNG color type {color_type}: {path}")

    channels = 4 if color_type == 6 else 3
    rows = _unfilter_png_rows(
        zlib.decompress(b"".join(idat_parts)), width, height, channels
    )
    rgb = bytearray(width * height * 3)

    for y, row in enumerate(rows):
        for x in range(width):
            src = x * channels
            dst = (y * width + x) * 3
            if channels == 4:
                alpha = row[src + 3]
                rgb[dst] = (row[src] * alpha + 255 * (255 - alpha) + 127) // 255
                rgb[dst + 1] = (row[src + 1] * alpha + 255 * (255 - alpha) + 127) // 255
                rgb[dst + 2] = (row[src + 2] * alpha + 255 * (255 - alpha) + 127) // 255
            else:
                rgb[dst : dst + 3] = row[src : src + 3]

    return _RgbImage(width, height, bytes(rgb))


def _pad_rgb_to_canvas(img: _RgbImage, width: int, height: int) -> bytes:
    out = bytearray([255]) * (width * height * 3)
    for y in range(img.height):
        row_len = img.width * 3
        src = y * row_len
        dst = y * width * 3
        out[dst : dst + row_len] = img.data[src : src + row_len]
    return bytes(out)


def _similarity_ratio(actual: bytes, reference: bytes) -> float:
    if len(actual) != len(reference):
        return 0.0
    mse = sum((a - b) * (a - b) for a, b in zip(actual, reference)) / len(actual)
    if mse <= 1e-12:
        return 1.0
    return max(0.0, 1.0 - mse / (255.0 * 255.0))


def _png_chunk(kind: bytes, data: bytes) -> bytes:
    crc = zlib.crc32(kind + data) & 0xFFFFFFFF
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", crc)


def _write_png_rgb(path: Path, width: int, height: int, data: bytes) -> None:
    rows = bytearray()
    row_len = width * 3
    for y in range(height):
        rows.append(0)
        start = y * row_len
        rows.extend(data[start : start + row_len])

    path.write_bytes(
        _PNG_SIGNATURE
        + _png_chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
        + _png_chunk(b"IDAT", zlib.compress(bytes(rows)))
        + _png_chunk(b"IEND", b"")
    )


def _save_diff_png(
    actual: bytes, reference: bytes, width: int, height: int, output_path: Path
) -> None:
    diff = bytearray(len(actual))
    for idx, (left, right) in enumerate(zip(actual, reference)):
        diff[idx] = min(abs(left - right) * 4, 255)
    _write_png_rgb(output_path, width, height, bytes(diff))


def _save_diff_and_similarity(
    actual_png: Path, reference_png: Path, diff_png: Path
) -> float:
    actual = _load_png_rgb(actual_png)
    reference = _load_png_rgb(reference_png)
    width = max(actual.width, reference.width)
    height = max(actual.height, reference.height)
    actual_padded = _pad_rgb_to_canvas(actual, width, height)
    reference_padded = _pad_rgb_to_canvas(reference, width, height)
    ratio = _similarity_ratio(actual_padded, reference_padded)
    _save_diff_png(actual_padded, reference_padded, width, height, diff_png)
    assert (actual.width, actual.height) == (reference.width, reference.height), (
        f"image dimensions changed: actual {actual_png}, reference {reference_png}, diff {diff_png}"
    )
    return ratio


def _assert_visual_match(filename: str, render):
    paths = _visual_case_paths(filename)

    svg = render(str(paths.actual_svg))
    # Exercise the public PNG exporter; compare with a fixed font independently
    # of OS font discovery, just as the Rust visual tests do.
    render(str(paths.exported_png))
    exported = _load_png_rgb(paths.exported_png)
    assert exported.width > 0 and exported.height > 0

    assert isinstance(svg, str)
    assert paths.actual_svg.read_text(encoding="utf-8").startswith("<svg")

    paths.actual_png.write_bytes(
        resvg_py.svg_to_bytes(
            svg_string=str(svg),
            skip_system_fonts=True,
            font_files=[str(_visual_font())],
            font_family="Liberation Sans",
            sans_serif_family="Liberation Sans",
            serif_family="Liberation Sans",
            monospace_family="Liberation Sans",
        )
    )
    assert paths.reference_png.is_file(), (
        f"missing visual reference: {paths.reference_png}"
    )

    ratio = _save_diff_and_similarity(
        paths.actual_png, paths.reference_png, paths.diff_png
    )
    threshold = _visual_threshold()
    assert ratio >= threshold, (
        f"Similarity ratio {ratio:.6f} < {threshold:.6f} for {filename}; "
        f"actual: {paths.actual_png}; reference: {paths.reference_png}; "
        f"diff: {paths.diff_png}; SVG: {paths.actual_svg}; font: Liberation Sans"
    )
    return svg


def test_visualization_public_exports():
    """The visualization package should expose all reviewed plotting functions."""
    expected = {
        "draw_text",
        "draw_figure",
        "plot_histogram",
        "plot_distribution",
        "plot_bloch_vector",
        "plot_bloch_multivector",
        "plot_state_city",
        "plot_state_paulivec",
    }
    assert expected.issubset(set(vis.__all__))


def test_visualization_uses_checkout_editable_binding():
    """The public package and native extension should come from this checkout."""
    binding_pkg = Path(__file__).resolve().parents[2] / "cqlib"

    assert Path(cqlib.__file__).resolve().is_relative_to(binding_pkg)
    assert Path(cqlib_native.__file__).resolve().is_relative_to(binding_pkg)


def test_circuit_visualization_matches_reference_image():
    """Circuit visualization should expose text, SVG, and stable PNG output."""
    circuit = _bell_circuit()

    text = vis.draw_text(circuit)
    assert "H" in text

    svg = _assert_visual_match(
        "bell_default.png",
        lambda output_path: vis.draw_figure(circuit, output_path=output_path),
    )
    assert svg._repr_svg_() == str(svg)
    assert "<svg" in svg


def test_circuit_visualization_option_passthrough_matches_reference_image():
    """Circuit figure options should affect the rendered public API output."""
    circuit = _folded_circuit()
    svg = _assert_visual_match(
        "circuit_fold_reverse_initial.png",
        lambda output_path: vis.draw_figure(
            circuit,
            fold=2,
            initial_state=True,
            reverse_bits=True,
            show_params=False,
            output_path=output_path,
        ),
    )
    assert "q2 |0" in svg
    assert "q0 |0" in svg
    _assert_in_order(
        svg, [">q2 |0&gt;</text>", ">q1 |0&gt;</text>", ">q0 |0&gt;</text>"]
    )
    assert "0.25" not in svg


def test_result_visualization_matches_reference_images():
    """Result visualization should compare raw counts and probabilities."""
    histogram_result = _execution_result({"00": 2, "11": 5}, 2)
    distribution_result = _execution_result({"0": 25, "1": 75}, 1)

    histogram = _assert_visual_match(
        "histogram_counts.png",
        lambda output_path: vis.plot_histogram(
            histogram_result,
            output_path=output_path,
        ),
    )
    assert "<svg" in histogram
    assert "Count" in histogram

    distribution = _assert_visual_match(
        "distribution_probabilities.png",
        lambda output_path: vis.plot_distribution(
            distribution_result,
            output_path=output_path,
        ),
    )
    assert "<svg" in distribution
    assert "Probability" in distribution


def test_result_visualization_option_passthrough_matches_reference_images():
    """Result plot options should be reflected in SVG structure and reference PNGs."""
    histogram_result = _execution_result({"100": 4, "101": 3, "010": 2, "001": 1}, 3)
    histogram = _assert_visual_match(
        "histogram_hamming_sorted.png",
        lambda output_path: vis.plot_histogram(
            histogram_result,
            sort="hamming",
            target_string="100",
            number_to_keep=3,
            color=["#123456"],
            legend=["sim"],
            bar_labels=False,
            title="Hamming order",
            output_path=output_path,
        ),
    )
    assert "Hamming order" in histogram
    assert "#123456" in histogram
    assert ">sim</text>" in histogram
    assert ">rest</text>" in histogram
    assert ">4</text>" not in histogram

    distribution_result = _execution_result({"0": 25, "1": 75}, 1)
    distribution = _assert_visual_match(
        "distribution_custom_options.png",
        lambda output_path: vis.plot_distribution(
            distribution_result,
            figsize=(3.2, 2.4),
            color=["#0f766e"],
            legend=["probability"],
            bar_labels=False,
            title="Distribution options",
            output_path=output_path,
        ),
    )
    assert 'width="320"' in distribution
    assert 'height="240"' in distribution
    assert "#0f766e" in distribution
    assert ">probability</text>" in distribution


def test_state_visualization_matches_reference_images():
    """State visualization should compare every plotting family."""
    zero = _zero_statevector()
    sv = _plus_statevector()

    vector_svg = _assert_visual_match(
        "bloch_vector_z.png",
        lambda output_path: vis.plot_bloch_vector(
            [0.0, 0.0, 1.0],
            output_path=output_path,
        ),
    )
    assert "data-cqlib-bloch-3d" in vector_svg

    bloch_svg = _assert_visual_match(
        "bloch_multivector_plus.png",
        lambda output_path: vis.plot_bloch_multivector(
            sv,
            output_path=output_path,
        ),
    )
    assert "data-cqlib-bloch-3d" in bloch_svg

    state_city = _assert_visual_match(
        "state_city_zero.png",
        lambda output_path: vis.plot_state_city(
            zero,
            output_path=output_path,
        ),
    )
    assert "Re[rho]" in state_city

    paulivec_svg = _assert_visual_match(
        "paulivec_plus.png",
        lambda output_path: vis.plot_state_paulivec(
            sv,
            output_path=output_path,
        ),
    )
    assert "<svg" in paulivec_svg


def test_state_visualization_option_passthrough_matches_reference_images():
    """State plot options should pass through Python wrappers into rendered SVG."""
    density = _reverse_bits_density_matrix()
    state_city = _assert_visual_match(
        "state_city_reverse_bits.png",
        lambda output_path: vis.plot_state_city(
            density,
            reverse_bits=True,
            alpha=0.7,
            figsize=(5.0, 4.0),
            title="Reverse state city",
            output_path=output_path,
        ),
    )
    assert "Reverse state city" in state_city
    assert 'width="500"' in state_city
    assert 'height="400"' in state_city
    assert 'fill-opacity="0.700"' in state_city
    _assert_in_order(
        state_city, [">00</text>", ">10</text>", ">01</text>", ">11</text>"]
    )

    paulivec = _assert_visual_match(
        "paulivec_custom_options.png",
        lambda output_path: vis.plot_state_paulivec(
            _plus_statevector(),
            color=["#00aa00", "#aa0000"],
            title="Pauli colors",
            figsize=(5.0, 3.0),
            output_path=output_path,
        ),
    )
    assert "Pauli colors" in paulivec
    assert 'width="500"' in paulivec
    assert 'height="300"' in paulivec
    assert "#00aa00" in paulivec


def test_state_visualization_accepts_density_matrix():
    """DensityMatrix should use the same state plotting API as Statevector."""
    sv = _plus_statevector()
    dm = _plus_density_matrix()

    assert vis.plot_state_city(sv) == vis.plot_state_city(dm)
    assert vis.plot_state_paulivec(sv) == vis.plot_state_paulivec(dm)


def test_state_visualization_validates_python_inputs():
    """Invalid Python inputs should fail with clear ValueError messages."""
    with pytest.raises(ValueError, match="exactly 3"):
        vis.plot_bloch_vector([0.0, 1.0])

    with pytest.raises(ValueError, match="Statevector or cqlib.qis.DensityMatrix"):
        vis.plot_state_city(object())
