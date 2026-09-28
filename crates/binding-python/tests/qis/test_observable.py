# This code is part of Cqlib.
# Modified to cover Pauli source phases in expectation and variance calculations.
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

import pytest
import math
from itertools import product

import numpy as np
from cqlib import Circuit
from cqlib.qis import PauliString, Hamiltonian, Statevector, DensityMatrix


_PAULI_MATRICES = {
    "I": np.eye(2, dtype=complex),
    "X": np.array([[0, 1], [1, 0]], dtype=complex),
    "Y": np.array([[0, -1j], [1j, 0]], dtype=complex),
    "Z": np.diag([1, -1]).astype(complex),
}
_PAULI_LABELS = [
    "".join(chars) for n in range(1, 4) for chars in product("IXYZ", repeat=n)
] + ["YYYY"]


def _pauli_matrix(label):
    matrix = np.ones((1, 1), dtype=complex)
    for char in label:
        matrix = np.kron(matrix, _PAULI_MATRICES[char])
    return matrix


@pytest.mark.parametrize("sign", [1, -1])
def test_y_expectation_from_circuit(sign):
    circuit = Circuit(1)
    circuit.h(0)
    circuit.rz(0, sign * math.pi / 2)
    sv = Statevector.from_circuit(circuit)
    pauli = PauliString.from_str("Y")
    h = Hamiltonian.from_pauli(pauli)
    amplitudes = sv.data
    reference = np.vdot(amplitudes, _PAULI_MATRICES["Y"] @ amplitudes)
    assert reference == pytest.approx(sign, abs=1e-12)
    for actual in (
        sv.expectation(pauli),
        pauli.expectation_statevector(sv),
        sv.expectation(h),
        h.expectation_statevector(sv),
    ):
        assert actual == pytest.approx(reference.real, abs=1e-12)


@pytest.mark.parametrize("label", _PAULI_LABELS)
@pytest.mark.parametrize("prefix,phase", [("", 1), ("-", -1), ("i", 1j), ("-i", -1j)])
def test_pauli_expectation_phases_match_independent_matrix(label, prefix, phase):
    n = len(label)
    rng = np.random.default_rng(23923 + n)
    amplitudes = rng.normal(size=2**n) + 1j * rng.normal(size=2**n)
    amplitudes /= np.linalg.norm(amplitudes)
    sv = Statevector.from_state(n, amplitudes)
    dm = DensityMatrix.from_state(n, amplitudes)
    pauli = PauliString.from_str(prefix + label)
    reference = np.vdot(amplitudes, phase * _pauli_matrix(label) @ amplitudes)
    # Ensure odd-Y sign errors cannot hide behind a zero expectation.
    assert abs(reference) > 1e-10

    if phase in (1, -1):
        assert abs(reference.imag) < 1e-12
        for actual in (
            sv.expectation(pauli),
            pauli.expectation_statevector(sv),
            pauli.expectation_density_matrix(dm),
        ):
            assert actual == pytest.approx(reference.real, abs=1e-12)

    coefficient = -0.37 / phase
    h = Hamiltonian.from_list([(pauli, coefficient)])
    expected = coefficient * reference
    assert abs(expected.imag) < 1e-12
    for actual in (
        sv.expectation(h),
        h.expectation_statevector(sv),
        h.expectation_density_matrix(dm),
    ):
        assert actual == pytest.approx(expected.real, abs=1e-12)


@pytest.mark.parametrize("other,angle", [("X", math.pi / 4), ("I", math.pi / 2)])
def test_hamiltonian_y_variance_matches_independent_matrix(other, angle):
    amplitudes = np.array([1, np.exp(1j * angle)]) / np.sqrt(2)
    sv = Statevector.from_state(1, amplitudes)
    h = Hamiltonian.from_list(
        [(PauliString.from_str(other), 1), (PauliString.from_str("-iY"), 1j)]
    )
    matrix = _PAULI_MATRICES[other] + _PAULI_MATRICES["Y"]
    applied = matrix @ amplitudes
    mean = np.vdot(amplitudes, applied)
    variance = np.vdot(applied, applied) - mean**2
    assert abs(mean.imag) < 1e-12
    assert variance == pytest.approx(0, abs=1e-12)
    assert h.expectation_statevector(sv) == pytest.approx(mean.real, abs=1e-12)
    assert h.variance_statevector(sv) == pytest.approx(variance.real, abs=1e-12)


def test_mixed_hamiltonian_moments_match_independent_matrix():
    rng = np.random.default_rng(23923)
    amplitudes = rng.normal(size=8) + 1j * rng.normal(size=8)
    amplitudes /= np.linalg.norm(amplitudes)
    sv = Statevector.from_state(3, amplitudes)
    terms = [
        ("XYZ", "", 1, 0.73),
        ("YYY", "-i", -1j, 0.2j),
        ("YIY", "-", -1, -0.41),
        ("ZXI", "i", 1j, -0.19j),
        ("III", "", 1, -0.3),
        ("XYZ", "", 1, -0.11),
    ]
    h = Hamiltonian.from_list(
        [
            (PauliString.from_str(prefix + label), coeff)
            for label, prefix, _, coeff in terms
        ]
    )
    matrix = sum(
        phase * coeff * _pauli_matrix(label) for label, _, phase, coeff in terms
    )
    applied = matrix @ amplitudes
    mean = np.vdot(amplitudes, applied)
    variance = np.vdot(applied, applied) - mean**2
    assert abs(mean.imag) < 1e-12
    assert abs(variance.imag) < 1e-12
    assert h.expectation_statevector(sv) == pytest.approx(mean.real, abs=1e-12)
    assert h.variance_statevector(sv) == pytest.approx(variance.real, abs=1e-12)
    h.simplify()
    assert h.expectation_statevector(sv) == pytest.approx(mean.real, abs=1e-12)
    assert h.variance_statevector(sv) == pytest.approx(variance.real, abs=1e-12)


def test_pauli_expectation_statevector():
    # |0⟩ state
    sv = Statevector(1)

    ps_z = PauliString.from_str("Z")
    assert math.isclose(ps_z.expectation_statevector(sv), 1.0)

    ps_x = PauliString.from_str("X")
    assert math.isclose(ps_x.expectation_statevector(sv), 0.0)

    # |+⟩ state
    sv.apply_h(0)
    assert math.isclose(ps_z.expectation_statevector(sv), 0.0)
    assert math.isclose(ps_x.expectation_statevector(sv), 1.0)

    # |-⟩ state
    sv.apply_z(0)
    assert math.isclose(ps_x.expectation_statevector(sv), -1.0)


def test_pauli_expectation_density_matrix():
    dm = DensityMatrix(1)

    ps_z = PauliString.from_str("Z")
    assert math.isclose(ps_z.expectation_density_matrix(dm), 1.0)

    dm.apply_h(0)
    ps_x = PauliString.from_str("X")
    assert math.isclose(ps_x.expectation_density_matrix(dm), 1.0)
    assert math.isclose(ps_z.expectation_density_matrix(dm), 0.0)


def test_pauli_expectation_probs():
    ps_z = PauliString.from_str("Z")

    # 100% |0⟩
    measurements_0 = [(PauliString.from_str("Z"), {"0": 1.0})]
    assert math.isclose(ps_z.expectation_probs(measurements_0), 1.0)

    # 100% |1⟩
    measurements_1 = [(PauliString.from_str("Z"), {"1": 1.0})]
    assert math.isclose(ps_z.expectation_probs(measurements_1), -1.0)

    # 50/50 mix
    measurements_mixed = [(PauliString.from_str("Z"), {"0": 0.5, "1": 0.5})]
    assert math.isclose(ps_z.expectation_probs(measurements_mixed), 0.0)


def test_pauli_variance_statevector():
    ps_z = PauliString.from_str("Z")

    sv_zero = Statevector(1)
    assert math.isclose(ps_z.variance_statevector(sv_zero), 0.0, abs_tol=1e-10)

    sv_plus = Statevector(1)
    sv_plus.apply_h(0)
    assert math.isclose(ps_z.variance_statevector(sv_plus), 1.0, abs_tol=1e-10)

    ps_non_herm = PauliString.from_str("+iZ")
    with pytest.raises(ValueError, match="Hermitian"):
        ps_non_herm.variance_statevector(sv_zero)


def test_hamiltonian_expectation_statevector():
    # Bell state |Φ+⟩ = (|00⟩ + |11⟩)/√2
    sv = Statevector(2)
    sv.apply_h(0)
    sv.apply_cx(0, 1)

    # H = 0.5*ZZ + 0.5*XX
    h = Hamiltonian(2)
    h.add_term(PauliString.from_str("ZZ"), 0.5)
    h.add_term(PauliString.from_str("XX"), 0.5)
    h.simplify()

    # <Φ+|ZZ|Φ+> = 1.0, <Φ+|XX|Φ+> = 1.0, total = 0.5 + 0.5 = 1.0
    assert math.isclose(h.expectation_statevector(sv), 1.0)

    # ZI should be 0.0
    h2 = Hamiltonian(2)
    h2.add_term(PauliString.from_str("ZI"), 1.0)
    assert math.isclose(h2.expectation_statevector(sv), 0.0)


def test_hamiltonian_expectation_density_matrix():
    dm = DensityMatrix(2)
    dm.apply_h(0)
    dm.apply_cx(0, 1)

    h = Hamiltonian(2)
    h.add_term(PauliString.from_str("ZZ"), 0.5)
    h.add_term(PauliString.from_str("XX"), 0.5)
    h.simplify()

    assert math.isclose(h.expectation_density_matrix(dm), 1.0)


def test_hamiltonian_expectation_probs():
    h = Hamiltonian(2)
    h.add_term(PauliString.from_str("ZZ"), 0.5)
    h.add_term(PauliString.from_str("ZI"), 0.3)

    measurements = [(PauliString.from_str("ZZ"), {"00": 0.5, "11": 0.5})]
    # For ZZ: 0.5 * 1 + 0.5 * 1 = 1.0
    # For ZI (derived from ZZ measurements): "00"->Z_0=0(val=1), "11"->Z_0=1(val=-1) -> 0.5*1 + 0.5*(-1) = 0.0
    # Total = 0.5 * 1.0 + 0.3 * 0.0 = 0.5
    assert math.isclose(h.expectation_probs(measurements), 0.5)


def test_expectation_exceptions():
    sv = Statevector(1)
    h = Hamiltonian(2)  # Mismatched qubits

    with pytest.raises(ValueError, match="Qubit count mismatch"):
        h.expectation_statevector(sv)

    ps = PauliString.from_str("ZZ")  # 2 qubits
    with pytest.raises(ValueError, match="Qubit count mismatch"):
        ps.expectation_statevector(sv)

    # Missing compatible basis
    h_x = Hamiltonian(1)
    h_x.add_term(PauliString.from_str("X"), 1.0)
    measurements = [(PauliString.from_str("Z"), {"0": 1.0})]
    with pytest.raises(ValueError, match="No compatible measurement basis"):
        h_x.expectation_probs(measurements)
