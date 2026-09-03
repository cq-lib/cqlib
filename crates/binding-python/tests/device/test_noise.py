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

"""Tests for noise model related APIs."""

import pytest

from cqlib.circuit import StandardGate
from cqlib.device import (
    NoiseModel,
    OperationKey,
    ReadoutError,
    SingleQubitNoise,
    TwoQubitNoise,
)
from cqlib.qis import Pauli


class TestNoiseChannels:
    """Tests noise channel construction and basic properties."""

    def test_single_and_two_qubit_noise(self):
        """Noise factories should produce valid channels and kraus matrices."""
        sq = SingleQubitNoise.depolarizing(0.1)
        # SingleQubitNoise doesn't have 'kind' property, check via repr
        assert "depolarizing" in repr(sq)
        assert sq.is_valid() is True
        sq_kraus = sq.to_kraus()
        assert len(sq_kraus) == 4
        assert sq_kraus[0].shape == (2, 2)

        tq = TwoQubitNoise.independent(
            SingleQubitNoise.phase_flip(0.02),
            SingleQubitNoise.bit_flip(0.03),
        )
        assert tq.kind == "independent"
        assert tq.is_valid() is True
        tq_kraus = tq.to_kraus()
        assert len(tq_kraus) == 4
        assert tq_kraus[0].shape == (4, 4)

        cp = TwoQubitNoise.correlated_pauli(Pauli.x(), Pauli.z(), 0.05)
        assert cp.kind == "correlated_pauli"
        # The constructor validates probabilities eagerly.
        with pytest.raises(ValueError):
            TwoQubitNoise.correlated_pauli(Pauli.x(), Pauli.z(), 1.5)


class TestOperationKeyAndNoiseModel:
    """Tests operation key semantics and noise model retrieval APIs."""

    def test_operation_key_hash_equality_and_validation(self):
        """OperationKey equality/hash should match gate+qubit tuple identity."""
        key1 = OperationKey.new_double(StandardGate.CX, 0, 1)
        key2 = OperationKey.new_double(StandardGate.CX, 0, 1)
        key3 = OperationKey.new_double(StandardGate.CX, 1, 0)

        assert key1 == key2
        assert hash(key1) == hash(key2)
        assert key1 != key3
        assert key1.gate == StandardGate.CX
        assert key1.qubits == [0, 1]

        with pytest.raises(ValueError):
            OperationKey.new_double(StandardGate.CX, 0, 0)

    def test_noise_model_add_and_get(self):
        """NoiseModel should store readout/single/two-qubit error channels."""
        nm = NoiseModel()
        ro = ReadoutError(0.1, 0.2)
        assert ro.is_valid() is True
        nm.add_readout_error(0, ro)

        got_ro = nm.get_readout_error(0)
        assert got_ro is not None
        assert got_ro.p_0_given_1 == pytest.approx(0.1)
        assert got_ro.p_1_given_0 == pytest.approx(0.2)

        nm.add_single_qubit_error(StandardGate.X, 0, SingleQubitNoise.bit_flip(0.01))
        skey = OperationKey.new_single(StandardGate.X, 0)
        s_errs = nm.get_single_qubit_errors(skey)
        assert s_errs is not None
        # SingleQubitNoise doesn't have 'kind' property, verify via repr
        assert "bit_flip" in repr(s_errs[0])

        nm.add_two_qubit_error(StandardGate.CX, 0, 1, TwoQubitNoise.depolarizing(0.02))
        tkey = OperationKey.new_double(StandardGate.CX, 0, 1)
        t_errs = nm.get_two_qubit_errors(tkey)
        assert t_errs is not None
        # TwoQubitNoise has 'kind' property
        assert t_errs[0].kind == "depolarizing"

        with pytest.raises(ValueError):
            nm.add_single_qubit_error(StandardGate.X, 0, SingleQubitNoise.bit_flip(1.5))


class TestNoiseEquality:
    """Noise wrappers compare by value; f64 channels stay unhashable."""

    def test_single_qubit_noise_value_equality(self):
        assert SingleQubitNoise.bit_flip(0.1) == SingleQubitNoise.bit_flip(0.1)
        assert SingleQubitNoise.bit_flip(0.1) != SingleQubitNoise.bit_flip(0.2)
        assert SingleQubitNoise.bit_flip(0.1) != SingleQubitNoise.phase_flip(0.1)
        assert SingleQubitNoise.pauli(0.1, 0.2, 0.3) == SingleQubitNoise.pauli(
            0.1, 0.2, 0.3
        )
        assert (SingleQubitNoise.bit_flip(0.1) == 42) is False

    def test_two_qubit_noise_value_equality(self):
        assert TwoQubitNoise.depolarizing(0.02) == TwoQubitNoise.depolarizing(0.02)
        assert TwoQubitNoise.depolarizing(0.02) != TwoQubitNoise.depolarizing(0.03)
        assert TwoQubitNoise.independent(
            SingleQubitNoise.phase_flip(0.02),
            SingleQubitNoise.bit_flip(0.03),
        ) == TwoQubitNoise.independent(
            SingleQubitNoise.phase_flip(0.02),
            SingleQubitNoise.bit_flip(0.03),
        )
        assert TwoQubitNoise.independent(
            SingleQubitNoise.phase_flip(0.02),
            SingleQubitNoise.bit_flip(0.03),
        ) != TwoQubitNoise.independent(
            SingleQubitNoise.bit_flip(0.03),
            SingleQubitNoise.phase_flip(0.02),
        )

    def test_readout_error_value_equality(self):
        assert ReadoutError(0.1, 0.2) == ReadoutError(0.1, 0.2)
        assert ReadoutError(0.1, 0.2) != ReadoutError(0.1, 0.3)
        assert (ReadoutError(0.1, 0.2) == 42) is False

    def test_noise_channels_are_unhashable(self):
        with pytest.raises(TypeError):
            hash(SingleQubitNoise.bit_flip(0.1))
        with pytest.raises(TypeError):
            hash(TwoQubitNoise.depolarizing(0.02))
        with pytest.raises(TypeError):
            hash(ReadoutError(0.1, 0.2))
