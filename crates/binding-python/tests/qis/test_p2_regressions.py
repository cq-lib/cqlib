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

import itertools
import numpy as np
import pytest
from cqlib.circuit import (
    Circuit,
    Qubit,
    StandardGate,
    Instruction,
    Parameter,
    circuit_to_matrix,
)
from cqlib.device import NoiseModel, SingleQubitNoise
from cqlib.qis import (
    Statevector,
    DensityMatrix,
    DensityMatrixNoise,
    Hamiltonian,
    PauliString,
    entropy,
)
from cqlib.error_mitigation.zne import ZNEMitigation


@pytest.mark.parametrize(
    "constructor", [Statevector.from_state, DensityMatrix.from_state]
)
@pytest.mark.parametrize(
    "bad", [float("nan"), float("inf"), -float("inf"), np.finfo(float).max]
)
def test_nonfinite_norm_rejected(constructor, bad):
    for amp in [complex(bad, 0), complex(0, bad)]:
        with pytest.raises(ValueError, match="normalized"):
            constructor(1, [amp, 0j])
    assert constructor(0, [1j]).probabilities() == pytest.approx([1])
    assert constructor(1, [0.6, 0.8j]).probabilities() == pytest.approx([0.36, 0.64])


@pytest.mark.parametrize("bad", [float("nan"), float("inf"), -float("inf")])
def test_nonfinite_density_matrix_rejected(bad):
    for pos in range(4):
        for value in [complex(bad, 0), complex(0, bad)]:
            data = np.eye(2, dtype=complex).flatten() / 2
            data[pos] = value
            with pytest.raises(ValueError):
                DensityMatrix.from_density_matrix(1, data)


@pytest.mark.parametrize("label", ["iZ", "-iI", "iX"])
def test_real_expectations_reject_nonhermitian(label):
    p = PauliString.from_str(label)
    h = Hamiltonian(1)
    h.add_term(p, 1)
    measurements = [(p, {"0": 1.0})]
    for observable in [p, h]:
        for state in [Statevector(1), DensityMatrix(1), DensityMatrixNoise(1)]:
            with pytest.raises(ValueError, match="Hermitian"):
                state.expectation(observable)
        with pytest.raises(ValueError, match="Hermitian"):
            observable.expectation_probs(measurements)
    with pytest.raises(ValueError, match="Hermitian"):
        p.expectation({"0": 1.0})


def test_merged_hermitian_and_small_coefficients():
    for terms, expected in [
        ([("iZ", -1j)], 1),
        ([("Z", 1 + 2j), ("Z", -2j)], 1),
        ([("Z", 1e-11)], 1e-11),
    ]:
        h = Hamiltonian(1)
        for label, coeff in terms:
            h.add_term(PauliString.from_str(label), coeff)
        assert h.expectation_statevector(Statevector(1)) == pytest.approx(
            expected, abs=1e-14
        )
        assert h.expectation_density_matrix(DensityMatrix(1)) == pytest.approx(
            expected, abs=1e-14
        )
        assert h.expectation_probs(
            [(PauliString.from_str("Z"), {"0": 1.0})]
        ) == pytest.approx(expected, abs=1e-14)


def test_pauli_expectations_against_independent_dense_matrices():
    rng = np.random.default_rng(7451)
    matrices = {
        "I": np.eye(2),
        "X": np.array([[0, 1], [1, 0]]),
        "Y": np.array([[0, -1j], [1j, 0]]),
        "Z": np.diag([1, -1]),
    }
    for n in [1, 2, 3]:
        psi = rng.normal(size=2**n) + 1j * rng.normal(size=2**n)
        psi /= np.linalg.norm(psi)
        sv = Statevector.from_state(n, psi)
        rho = 0.7 * np.outer(psi, psi.conj()) + 0.3 * np.eye(2**n) / 2**n
        dm = DensityMatrix.from_density_matrix(n, rho.flatten())
        for labels in itertools.product("IXYZ", repeat=n):
            matrix = np.array([[1]])
            for label in labels:
                matrix = np.kron(matrix, matrices[label])
            for sign in [1, -1]:
                p = PauliString.from_str(("-" if sign == -1 else "") + "".join(labels))
                assert sv.expectation(p) == pytest.approx(
                    sign * np.vdot(psi, matrix @ psi).real, abs=1e-12
                )
                assert dm.expectation(p) == pytest.approx(
                    sign * np.trace(rho @ matrix).real, abs=1e-12
                )


@pytest.mark.parametrize("p", [0, 0.25, 1])
def test_identity_noise_sparse_labels(p):
    model = NoiseModel()
    model.add_single_qubit_error(StandardGate.I, 5, SingleQubitNoise.bit_flip(p))
    for count in [1, 2, 3]:
        c = Circuit([Qubit(12), Qubit(5)])
        for _ in range(count):
            c.i(Qubit(5))
        flipped = (1 - (1 - 2 * p) ** count) / 2
        assert DensityMatrixNoise.from_circuit(
            c, model
        ).probabilities() == pytest.approx([1 - flipped, 0, flipped, 0])


@pytest.mark.parametrize("symbolic", [False, True])
@pytest.mark.parametrize("with_gates", [False, True])
def test_zne_preserves_phase_and_parameter_binding(symbolic, with_gates):
    c = Circuit([Qubit(5)])
    c.set_global_phase(Parameter("phi") if symbolic else 0.37)
    if with_gates:
        c.h(Qubit(5))
        c.t(Qubit(5))
    expected = circuit_to_matrix(c.assign_parameters({"phi": 0.37}))
    for selection in [None, [], [Instruction.from_standard_gate(StandardGate.H)]]:
        for folded in ZNEMitigation(c, [0, 1, 2]).fold_circuits(selection):
            np.testing.assert_allclose(
                circuit_to_matrix(folded.assign_parameters({"phi": 0.37})),
                expected,
                atol=1e-12,
            )


def test_renyi_known_nondiagonal_spectrum():
    # Rotated mixed state: eigenvalues stay known without a reference eigensolver.
    spectrum = np.array([0.8, 0.2])
    u = np.array([[1, 1j], [1j, 1]]) / np.sqrt(2)
    rho = u @ np.diag(spectrum) @ u.conj().T
    dm = DensityMatrix.from_density_matrix(1, rho.flatten())
    shannon = -np.sum(spectrum * np.log2(spectrum))
    for alpha in [0.1, 0.5, 1 - 1e-12, 1, 1 + 1e-12, 2, 20, float("inf")]:
        if abs(alpha - 1) < 1e-8:
            expected = shannon
        elif np.isinf(alpha):
            expected = -np.log2(max(spectrum))
        else:
            expected = np.log2(np.sum(spectrum**alpha)) / (1 - alpha)
        assert entropy.renyi_entropy(dm, alpha) == pytest.approx(expected, abs=1e-10)


@pytest.mark.parametrize("bad", [complex(float("nan"), 0), complex(0, float("inf"))])
def test_nonfinite_observable_coefficients(bad):
    h = Hamiltonian(1)
    h.add_term(PauliString.from_str("Z"), bad)
    for call in [
        lambda: h.expectation_statevector(Statevector(1)),
        lambda: h.expectation_density_matrix(DensityMatrix(1)),
        lambda: h.expectation_probs([(PauliString.from_str("Z"), {"0": 1.0})]),
    ]:
        with pytest.raises(ValueError, match="finite"):
            call()


@pytest.mark.parametrize("alpha", [0.01, 0.1, 0.5, 1, 2, float("inf")])
def test_renyi_nondiagonal_pure_state_is_zero(alpha):
    psi = np.array([1, 2, 3, 4], dtype=complex) / np.sqrt(30)
    dm = DensityMatrix.from_state(2, psi)
    assert entropy.renyi_entropy(dm, alpha) == pytest.approx(0, abs=1e-10)


@pytest.mark.parametrize("alpha", [0.01, 0.1, 0.5, 1, 2, float("inf")])
def test_renyi_rank_deficient_spectrum_is_invariant_under_rotation(alpha):
    spectrum = np.array([0.8, 0.2, 0, 0])
    indices = np.arange(4)
    u = np.exp(2j * np.pi * np.outer(indices, indices) / 4) / 2
    expected = (
        -np.sum(spectrum[:2] * np.log2(spectrum[:2]))
        if alpha == 1
        else -np.log2(0.8)
        if np.isinf(alpha)
        else np.log2(np.sum(spectrum[:2] ** alpha)) / (1 - alpha)
    )
    for rho in [np.diag(spectrum), u @ np.diag(spectrum) @ u.conj().T]:
        dm = DensityMatrix.from_density_matrix(2, rho.astype(complex).flatten())
        assert entropy.renyi_entropy(dm, alpha) == pytest.approx(expected, abs=1e-10)


@pytest.mark.parametrize("bad", [float("nan"), float("inf"), -float("inf")])
def test_variance_rejects_nonfinite_coefficients(bad):
    for coeff in [complex(bad, 0), complex(0, bad)]:
        h = Hamiltonian(1)
        h.add_term(PauliString.from_str("Z"), coeff)
        with pytest.raises(ValueError, match="finite"):
            h.variance_statevector(Statevector(1))


def test_variance_validates_merged_coefficients_and_preserves_small_terms():
    h = Hamiltonian(1)
    for _ in range(2):
        h.add_term(PauliString.from_str("Z"), np.finfo(float).max)
    with pytest.raises(ValueError, match="finite"):
        h.variance_statevector(Statevector(1))
    for terms, expected in [
        ([("iX", -1j)], 1),
        ([("X", 1 + 2j), ("X", -2j)], 1),
        ([("X", 1e-11)], 1e-22),
    ]:
        h = Hamiltonian(1)
        for label, coeff in terms:
            h.add_term(PauliString.from_str(label), coeff)
        assert h.variance_statevector(Statevector(1)) == pytest.approx(
            expected, rel=1e-12, abs=1e-34
        )
    invalid = Hamiltonian.from_pauli(PauliString.from_str("iZ"))
    with pytest.raises(ValueError, match="Hermitian"):
        invalid.variance_statevector(Statevector(1))
    with pytest.raises(ValueError, match="Qubit count mismatch"):
        invalid.variance_statevector(Statevector(2))
