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

import concurrent.futures
import copy

import numpy as np
import pytest

from cqlib.circuit import Circuit, Instruction, StandardGate, UnitaryGate
from cqlib.compile import CompilerConfigError, compile
from cqlib.compile.transform.decompose import (
    UnitaryDecomposeConfig,
    decompose_unitaries,
    decompose_unitaries_with_rule_stats,
)
from cqlib.compile.transform.decompose.block_zxz import (
    BlockZxzDecomposeOptions,
    decompose_block_zxz,
    decompose_block_zxz_with_options,
)
from cqlib.device import Device


def random_unitary(dim: int, seed: int) -> np.ndarray:
    rng = np.random.default_rng(seed)
    shape = (dim, dim)
    z = rng.normal(size=shape) + 1j * rng.normal(size=shape)
    return np.linalg.qr(z)[0] * np.exp(0.321j)


@pytest.mark.parametrize(
    "matrix", [np.roll(np.eye(8), 1, axis=0), random_unitary(32, 13)]
)
def test_block_zxz_preserves_global_phase(matrix: np.ndarray) -> None:
    n = (len(matrix) - 1).bit_length()
    source = Circuit(n)
    source.append_unitary_gate(
        UnitaryGate("phase", n).with_matrix(matrix), list(reversed(range(n)))
    )
    for circuit in (decompose_block_zxz(matrix), decompose_unitaries(source).circuit):
        actual = circuit.to_matrix()
        phase = np.angle(np.vdot(matrix, actual))
        np.testing.assert_allclose(
            actual,
            matrix,
            rtol=0,
            atol=1e-10,
            err_msg=f"Relative phase {phase:.6g} rad",
        )


def test_explicit_options_and_strided_input() -> None:
    matrix = random_unitary(8, 7)[::-1, ::-1]
    for options, expected_cx in [(None, 19), (BlockZxzDecomposeOptions.none(), 24)]:
        c = decompose_block_zxz_with_options(matrix, options=options)
        assert sum(op.instruction.name == "CX" for op in c.operations) == expected_cx
        np.testing.assert_allclose(c.to_matrix(), matrix, rtol=0, atol=1e-10)


@pytest.mark.parametrize(
    "matrix",
    [
        np.zeros((2, 3)),
        np.eye(3),
        np.zeros((8, 8)),
        [[np.nan, 0], [0, 1]],
        [[1, 0], [0, np.inf]],
    ],
)
def test_invalid_matrix(matrix: object) -> None:
    with pytest.raises(CompilerConfigError):
        decompose_block_zxz(matrix)


@pytest.mark.parametrize("atol", [-1, np.nan, np.inf])
def test_invalid_tolerance(atol: float) -> None:
    with pytest.raises(CompilerConfigError):
        decompose_block_zxz_with_options(np.eye(8), atol)


def test_block_zxz_through_device_workflow() -> None:
    matrix = random_unitary(8, 7)
    body = Circuit(3)
    body.append_unitary_gate(
        UnitaryGate("asymmetric", 3).with_matrix(matrix), [2, 0, 1]
    )
    source = Circuit(3)
    source.append_circuit_gate(body.to_gate("nested"), [0, 1, 2])
    device = Device.bidirectional_line("block-zxz", 3)
    device.native_gates = [
        Instruction.from_standard_gate(g) for g in (StandardGate.U, StandardGate.CX)
    ]
    result = compile(source, device=device, seed=17)
    assert result.step_changed("decompose.definitions")
    assert result.step_changed("decompose.unitary")
    assert result.step("route.sabre") is not None
    assert {op.instruction.name for op in result.circuit.operations} == {"U", "CX"}
    metadata = result.device_metadata
    assert metadata is not None

    # Routing permutes the input columns and output rows independently.
    def physical_indices(layout):
        return [
            sum(((i >> j) & 1) << layout.get_physical(j).id for j in range(3))
            for i in range(8)
        ]

    indices = np.ix_(
        physical_indices(metadata.final_layout),
        physical_indices(metadata.initial_layout),
    )
    np.testing.assert_allclose(
        result.circuit.to_matrix()[indices], source.to_matrix(), rtol=0, atol=1e-10
    )


def test_nearly_scalar_spectrum() -> None:
    # Sixteen phases occupy an interval of 0.015 radians. The 1e-10
    # matrix budget is seven orders below their nearest-neighbor spacing.
    basis = random_unitary(16, 27)
    phases = 0.37 + 0.001 * np.arange(16)
    matrix = (basis * np.exp(1j * phases)) @ basis.conj().T
    np.testing.assert_allclose(
        decompose_block_zxz(matrix).to_matrix(), matrix, rtol=0, atol=1e-10
    )


def test_decompose_package_reexports_direct_entry_points() -> None:
    from cqlib.compile.transform import decompose

    assert decompose.decompose_block_zxz is decompose_block_zxz
    assert (
        decompose.decompose_block_zxz_with_options is decompose_block_zxz_with_options
    )
    assert decompose.BlockZxzDecomposeOptions is BlockZxzDecomposeOptions


def test_nearly_degenerate_two_qubit_leaf() -> None:
    fourier = np.fft.fft(np.eye(4)) / 2
    spectrum = np.exp(1e-8j * np.array([0, 1, 3, 7]))
    unitary = ((fourier * spectrum) @ fourier.conj().T)[[0, 1, 3, 2]]
    np.testing.assert_allclose(
        decompose_block_zxz(unitary).to_matrix(), unitary, atol=1e-10, rtol=0
    )


def test_options_value_semantics() -> None:
    default_a = BlockZxzDecomposeOptions()
    default_b = BlockZxzDecomposeOptions(opt_a1=True, opt_a2=True, opt_a2_force=False)
    none_opts = BlockZxzDecomposeOptions.none()

    assert default_a == default_b
    assert hash(default_a) == hash(default_b)
    assert default_a != none_opts
    assert len({default_a, default_b, none_opts}) == 2
    assert (
        repr(default_a)
        == "BlockZxzDecomposeOptions(opt_a1=True, opt_a2=True, opt_a2_force=False)"
    )
    assert (
        repr(none_opts)
        == "BlockZxzDecomposeOptions(opt_a1=False, opt_a2=False, opt_a2_force=False)"
    )
    assert copy.copy(default_a) == default_a
    assert copy.deepcopy(default_a) == default_a

    with pytest.raises(AttributeError):
        default_a.opt_a1 = False  # type: ignore[misc]
    with pytest.raises(TypeError):
        BlockZxzDecomposeOptions(True, True, False)  # type: ignore[misc]


def test_array_input_layouts_dtypes_and_invalid_shapes() -> None:
    u = random_unitary(8, 29)
    read_only = u.copy()
    read_only.setflags(write=False)
    padded = np.zeros((16, 16), dtype=np.complex128)
    padded[::2, ::2] = u

    for candidate in (
        np.asfortranarray(u),
        u.T,
        padded[::2, ::2],
        read_only,
        u.tolist(),
        np.eye(8, dtype=np.float64),
        np.eye(8, dtype=np.int64),
        np.eye(8, dtype=np.complex64),
    ):
        expected = np.asarray(candidate, dtype=np.complex128)
        np.testing.assert_allclose(
            decompose_block_zxz(candidate).to_matrix(), expected, rtol=0, atol=1e-10
        )

    for bad_type in (1.0, np.ones(8), np.zeros((2, 2, 2)), None):
        with pytest.raises(TypeError):
            decompose_block_zxz(bad_type)  # type: ignore[arg-type]
    for bad_dim in (np.empty((0, 0)), np.eye(1), np.eye(6)):
        with pytest.raises(CompilerConfigError):
            decompose_block_zxz(bad_dim)


def test_adversarial_demux_spectra_and_loose_atol() -> None:
    v4 = random_unitary(4, 31)
    l1 = random_unitary(4, 32)
    l2 = random_unitary(4, 33)
    for gap in (1.5e-8, 2e-8, 5e-8, 1e-7):
        phases = np.array(
            [np.pi / 2, -np.pi / 2 + gap, np.pi / 4, -3 * np.pi / 4 + gap]
        )
        c_mat = (v4 * np.exp(-1j * phases)) @ v4.conj().T
        uy = 1j * c_mat
        s, cs = np.sin(0.63), np.cos(0.63)
        dense = np.block([[cs * l1, s * (l1 @ uy)], [-s * l2, cs * (l2 @ uy)]])
        np.testing.assert_allclose(
            decompose_block_zxz(dense, atol=1e-10).to_matrix(),
            dense,
            rtol=0,
            atol=1e-10,
        )

    for dim, seed in ((2, 41), (4, 42), (8, 43), (16, 44)):
        exact = random_unitary(dim, seed)
        noisy = exact * (1.0 + 2e-7)
        actual = decompose_block_zxz(noisy, atol=1e-5).to_matrix()
        np.testing.assert_allclose(actual, noisy, rtol=0, atol=1e-5)


def test_overlapping_permuted_cache_and_cz_device_compile() -> None:
    u3 = random_unitary(8, 51)
    gate3 = UnitaryGate("u3", 3).with_matrix(u3)
    qc = Circuit(5)
    for qargs in ([0, 1, 2], [1, 2, 0], [2, 0, 1], [4, 1, 3]):
        qc.append_unitary_gate(gate3, qargs)
    res, stats = decompose_unitaries_with_rule_stats(qc)
    assert (stats.hits, stats.misses, stats.inserts) == (3, 1, 1)
    np.testing.assert_allclose(
        res.circuit.to_matrix(), qc.to_matrix(), rtol=0, atol=1e-9
    )

    u4 = random_unitary(16, 52)
    src = Circuit(4)
    src.append_unitary_gate(UnitaryGate("u4", 4).with_matrix(u4), [2, 0, 3, 1])
    device = Device.bidirectional_line("cz-4q", 4)
    device.native_gates = [
        Instruction.from_standard_gate(g)
        for g in (StandardGate.RZ, StandardGate.X2P, StandardGate.CZ)
    ]
    compiled = compile(src, device=device, seed=19)
    assert {op.instruction.name for op in compiled.circuit.operations}.issubset(
        {"RZ", "X2P", "CZ"}
    )
    metadata = compiled.device_metadata
    assert metadata is not None

    def physical_indices(layout):
        return [
            sum(((i >> j) & 1) << layout.get_physical(j).id for j in range(4))
            for i in range(16)
        ]

    indices = np.ix_(
        physical_indices(metadata.final_layout),
        physical_indices(metadata.initial_layout),
    )
    np.testing.assert_allclose(
        compiled.circuit.to_matrix()[indices], src.to_matrix(), rtol=0, atol=1e-9
    )


def test_concurrent_block_zxz_synthesis() -> None:
    def run_one(seed: int) -> float:
        u = random_unitary(16, seed)
        c = decompose_block_zxz(u)
        return float(np.max(np.abs(c.to_matrix() - u)))

    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
        errors = list(executor.map(run_one, range(60, 68)))
    assert max(errors) <= 1e-10


def test_unitary_decompose_config_block_zxz_options() -> None:
    default_cfg = UnitaryDecomposeConfig()
    assert default_cfg.block_zxz_options == BlockZxzDecomposeOptions()
    assert "block_zxz_options" not in repr(default_cfg)

    unopt_cfg = UnitaryDecomposeConfig(
        block_zxz_options=BlockZxzDecomposeOptions.none()
    )
    assert unopt_cfg.block_zxz_options == BlockZxzDecomposeOptions.none()
    assert unopt_cfg != default_cfg
    rebuilt = eval(
        repr(unopt_cfg),
        {
            "UnitaryDecomposeConfig": UnitaryDecomposeConfig,
            "BlockZxzDecomposeOptions": BlockZxzDecomposeOptions,
        },
    )
    assert rebuilt == unopt_cfg

    u3 = random_unitary(8, 7)
    qc = Circuit(3)
    qc.append_unitary_gate(UnitaryGate("u3", 3).with_matrix(u3), [2, 1, 0])
    opt_circuit = decompose_unitaries(qc, default_cfg).circuit
    unopt_circuit = decompose_unitaries(qc, unopt_cfg).circuit
    assert sum(op.instruction.name == "CX" for op in opt_circuit.operations) == 19
    assert sum(op.instruction.name == "CX" for op in unopt_circuit.operations) == 24
    np.testing.assert_allclose(opt_circuit.to_matrix(), u3, rtol=0, atol=1e-10)
    np.testing.assert_allclose(unopt_circuit.to_matrix(), u3, rtol=0, atol=1e-10)


def test_strided_non_element_multiple_and_unaligned_complex128_arrays() -> None:
    hadamard = np.array([[1, 1], [1, -1]], dtype=np.complex128) / np.sqrt(2)

    # 8-byte aligned complex128 array with 40-byte row stride (not a multiple of 16).
    backing = np.zeros(12, dtype=np.float64)
    strided = np.lib.stride_tricks.as_strided(
        backing.view(np.complex128), shape=(2, 2), strides=(40, 16)
    )
    strided[...] = hadamard
    assert strided.flags.aligned
    assert strided.strides == (40, 16)
    np.testing.assert_allclose(
        decompose_block_zxz(strided).to_matrix(), hadamard, rtol=0, atol=1e-10
    )

    # Unaligned but C-contiguous complex128 view (1-byte offset inside a raw buffer).
    raw = bytearray(128)
    unaligned = np.ndarray(shape=(2, 2), dtype=np.complex128, buffer=raw, offset=1)
    unaligned[...] = hadamard
    assert unaligned.flags.c_contiguous
    assert not unaligned.flags.aligned
    np.testing.assert_allclose(
        decompose_block_zxz(unaligned).to_matrix(), hadamard, rtol=0, atol=1e-10
    )


def test_repeated_weak_xx_interaction_does_not_accumulate_dropped_rotation() -> None:
    from cqlib.compile.transform.decompose import TwoQubitUnitaryDecomposeBasis
    from cqlib.compile.transform.decompose.unitary import synthesize_numeric_2q_unitary

    angle = 9e-11
    u2 = np.zeros((4, 4), dtype=np.complex128)
    for i in range(4):
        u2[i, i] = np.cos(angle)
        u2[i, i ^ 3] = 1j * np.sin(angle)
    for basis in (
        TwoQubitUnitaryDecomposeBasis.cx(),
        TwoQubitUnitaryDecomposeBasis.cy(),
        TwoQubitUnitaryDecomposeBasis.cz(),
    ):
        synth = synthesize_numeric_2q_unitary(u2, 1, 0, basis)
        entanglers = sum(
            op.instruction.name in {"CX", "CY", "CZ"} for op in synth.operations
        )
        assert entanglers == 2

    def weak_3q(theta: float) -> np.ndarray:
        out = np.zeros((8, 8), dtype=np.complex128)
        for i in range(8):
            out[i, i] = np.cos(theta)
            out[i, i ^ 3] = 1j * np.sin(theta)
        return out

    for weak_angle, repeats in ((9e-11, 64), (8e-13, 512)):
        gate = UnitaryGate("weak_xx", 3).with_matrix(weak_3q(weak_angle))
        qc = Circuit(3)
        for _ in range(repeats):
            qc.append_unitary_gate(gate, [2, 1, 0])
        expected = weak_3q(weak_angle * repeats)

        np.testing.assert_allclose(
            decompose_unitaries(qc).circuit.to_matrix(), expected, rtol=0, atol=1e-10
        )
        np.testing.assert_allclose(
            compile(qc).circuit.to_matrix(), expected, rtol=0, atol=1e-10
        )


def test_near_tolerance_unitary_boundary_decomposes_and_compiles() -> None:
    rng = np.random.default_rng(7)
    base = random_unitary(8, 7)
    scaled = base * (1.0 + 4.5e-11)

    noise = rng.normal(size=(8, 8)) + 1j * rng.normal(size=(8, 8))
    probe = base + noise * 1e-11
    probe_err = float(np.max(np.abs(probe.conj().T @ probe - np.eye(8))))
    anisotropic = base + noise * (1e-11 * (9e-11 / probe_err))

    # Row-scaled QFT has Gram error <= 1e-10 while polar max-entry shift > 1e-10.
    idx = np.arange(8)
    qft = np.exp(2j * np.pi * np.outer(idx, idx) / 8) / np.sqrt(8)
    polar_shift = qft.copy()
    polar_shift[0, :] *= 1.0 + 3.6e-10

    for matrix in (scaled, anisotropic, polar_shift):
        unitarity_err = float(np.max(np.abs(matrix.conj().T @ matrix - np.eye(8))))
        assert 8e-11 < unitarity_err < 1e-10
        u_svd, _, vh_svd = np.linalg.svd(matrix)
        projected = u_svd @ vh_svd

        gate = UnitaryGate("near_tol", 3).with_matrix(matrix)
        np.testing.assert_allclose(
            decompose_block_zxz(matrix, atol=1e-10).to_matrix(),
            projected,
            rtol=0,
            atol=1e-10,
        )
        qc = Circuit(3)
        qc.append_unitary_gate(gate, [2, 1, 0])
        np.testing.assert_allclose(
            decompose_unitaries(qc).circuit.to_matrix(), projected, rtol=0, atol=1e-10
        )
        np.testing.assert_allclose(
            compile(qc).circuit.to_matrix(), projected, rtol=0, atol=1e-10
        )

    # 64x64 coherent rank-1 perturbation below 64*n*eps in max-entry Gram norm.
    n = 64
    gram_noise = 0.94 * 64 * n * np.finfo(float).eps
    p = gram_noise / (np.sqrt(1.0 + n * gram_noise) + 1.0)
    coherent = np.eye(n, dtype=np.complex128) + p * np.ones((n, n), dtype=np.complex128)
    u_svd, _, vh_svd = np.linalg.svd(coherent)
    coherent_proj = u_svd @ vh_svd
    np.testing.assert_allclose(
        decompose_block_zxz(coherent, atol=1e-10).to_matrix(),
        coherent_proj,
        rtol=0,
        atol=1e-10,
    )


def test_nine_qubit_dense_wide_arc_spectrum_decomposes() -> None:
    rng = np.random.default_rng(777)
    z = rng.normal(size=(256, 256)) + 1j * rng.normal(size=(256, 256))
    v, _ = np.linalg.qr(z)
    phases = np.linspace(-1.2, 1.2, 256)
    u = np.block(
        [
            [(v * np.exp(1j * phases)) @ v.conj().T, np.zeros((256, 256))],
            [np.zeros((256, 256)), np.eye(256)],
        ]
    )

    circuit = decompose_block_zxz(u, 1e-10)
    assert circuit.num_qubits == 9
    assert len(circuit.operations) > 0

    psi = rng.normal(size=512) + 1j * rng.normal(size=512)
    psi /= np.linalg.norm(psi)
    state = psi.reshape([2] * 9).copy()
    for op in circuit.operations:
        qs = [q.index for q in op.qubits]
        if op.instruction.name == "CX":
            c, t = 8 - qs[0], 8 - qs[1]
            idx0 = [slice(None)] * 9
            idx1 = [slice(None)] * 9
            idx0[c], idx0[t] = 1, 0
            idx1[c], idx1[t] = 1, 1
            tmp = state[tuple(idx0)].copy()
            state[tuple(idx0)] = state[tuple(idx1)]
            state[tuple(idx1)] = tmp
        else:
            ax = 8 - qs[0]
            state = np.tensordot(op.matrix(), state, axes=([1], [ax]))
            state = np.moveaxis(state, 0, ax)
    phase = float(str(circuit.global_phase))
    actual = state.reshape(-1) * np.exp(1j * phase)
    np.testing.assert_allclose(actual, u @ psi, rtol=0, atol=1e-10)
