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

import pytest
import math
from cqlib.qis import PauliString, Hamiltonian, Statevector, DensityMatrix


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
