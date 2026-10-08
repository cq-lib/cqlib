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

"""Public API regressions for P1 numerical and simulation bugs."""

import struct

import numpy as np
import pytest
from cqlib.circuit import Circuit, Qubit, StandardGate
from cqlib.device import NoiseModel, SingleQubitNoise
from cqlib.qis import (
    Statevector,
    DensityMatrix,
    DensityMatrixNoise,
    Hamiltonian,
    PauliString,
    TrotterMode,
    entropy,
)


@pytest.mark.parametrize(
    "constructor",
    [Statevector, DensityMatrix, DensityMatrixNoise, DensityMatrix.maximally_mixed],
)
def test_invalid_dimensions_raise_value_error(constructor):
    bits = struct.calcsize("P") * 8
    for n in [bits - 4, bits, 2**bits - 1]:
        with pytest.raises(ValueError, match="dimension"):
            constructor(n)
    for n in [0, 1, 2]:
        assert np.sum(constructor(n).probabilities()) == pytest.approx(1.0)


def test_pauli_probability_boundary():
    noise = SingleQubitNoise.pauli(0.3, 0.6, 0.1)
    assert noise.is_valid()
    assert all(np.isfinite(k).all() for k in noise.to_kraus())
    model = NoiseModel()
    model.add_single_qubit_error(StandardGate.X, 0, noise)
    sim = DensityMatrixNoise(1, model)
    sim.apply_x(0)
    np.testing.assert_allclose(sim.probabilities(), [0.9, 0.1], atol=1e-12)


def test_gate_noise_uses_sparse_labels():
    model = NoiseModel()
    model.add_single_qubit_error(StandardGate.X, 5, SingleQubitNoise.bit_flip(1.0))
    circuit = Circuit([Qubit(5)])
    circuit.x(Qubit(5))
    sim = DensityMatrixNoise.from_circuit(circuit, model)
    np.testing.assert_allclose(sim.probabilities(), [1.0, 0.0], atol=1e-12)
    sim.apply_x(0)
    np.testing.assert_allclose(sim.probabilities(), [1.0, 0.0], atol=1e-12)


def test_formation_entropy_roundoff():
    state = np.array(
        [
            -0.5917562523120286 - 0.14345273890626087j,
            0.12739228627789673 + 0.3361800930257551j,
            -0.06600502193827436 + 0.35339664207352467j,
            -0.5571108451004554 + 0.2457267928442915j,
        ]
    )
    assert entropy.entanglement_of_formation(
        DensityMatrix.from_state(2, state)
    ) == pytest.approx(1.0, abs=1e-12)


def test_renyi_extreme_orders():
    dm = DensityMatrix.maximally_mixed(1)
    for alpha in [1.0 - 1e-12, 1.0 + 1e-12, 2000.0, float("inf")]:
        assert entropy.renyi_entropy(dm, alpha) == pytest.approx(1.0, abs=1e-10)
    with pytest.raises(ValueError):
        entropy.renyi_entropy(dm, float("nan"))
    small = DensityMatrix.from_density_matrix(
        1, np.diag([1 - 1e-14, 1e-14]).astype(complex).flatten()
    )
    assert entropy.renyi_entropy(small, 0.1) == pytest.approx(
        0.06257881098503021, abs=1e-12
    )


@pytest.mark.parametrize("method", ["to_trotter_circuit", "to_evolution_circuit"])
def test_evolution_retains_small_terms(method):
    h = Hamiltonian(1)
    h.add_term(PauliString.from_str("Z"), 1e-11)
    circuit = getattr(h, method)(1e11, 3, TrotterMode.first_order())
    # Probabilities alone cannot expose this regression: compare the full matrix.
    from cqlib.circuit import circuit_to_matrix

    np.testing.assert_allclose(
        circuit_to_matrix(circuit), np.diag(np.exp([-1j, 1j])), atol=1e-12
    )


@pytest.mark.parametrize("state_type", [Statevector, DensityMatrix, DensityMatrixNoise])
def test_oversized_circuit_raises_value_error(state_type):
    with pytest.raises(ValueError, match="dimension"):
        state_type.from_circuit(Circuit(struct.calcsize("P") * 8))


@pytest.mark.parametrize(
    "constructor",
    [
        Statevector.from_state,
        DensityMatrix.from_state,
        DensityMatrix.from_density_matrix,
    ],
)
def test_oversized_array_constructors_raise_value_error(constructor):
    with pytest.raises(ValueError, match="dimension"):
        constructor(struct.calcsize("P") * 8, [])
