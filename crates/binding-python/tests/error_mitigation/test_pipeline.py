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

import copy
import math

import pytest

import cqlib.error_mitigation as em
from cqlib.circuit import Circuit
from cqlib.error_mitigation.unified import Estimator as UnifiedEstimator
from cqlib.error_mitigation.virtual_distillation import Estimator as VdEstimator
from cqlib.error_mitigation.zne import Estimator as ZneEstimator
from cqlib.qis import Hamiltonian, PauliString


def single_qubit_z_hamiltonian() -> Hamiltonian:
    return Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])


def single_x_circuit() -> Circuit:
    circuit = Circuit(1)
    circuit.x(0)
    return circuit


def test_error_mitigation_public_exports_and_estimator_aliases() -> None:
    assert em.ZNEMitigation.__module__ == "cqlib.error_mitigation"
    assert em.ErrorMitigation.__module__ == "cqlib.error_mitigation"
    assert em.Estimator is UnifiedEstimator
    assert ZneEstimator == UnifiedEstimator
    assert VdEstimator == UnifiedEstimator
    assert "Estimator" in em.__all__
    assert "ZNEMitigation" in em.__all__
    assert "ErrorMitigation" in em.__all__


def test_error_mitigation_value_objects_compare_by_value() -> None:
    assert em.ZneConfig([0, 1]) == em.ZneConfig([0, 1])
    assert em.ZneConfig([0, 1]) != em.ZneConfig([0, 2])
    assert em.ZneConfig([0]).__eq__(object()) is NotImplemented

    assert em.VirtualDistillationConfig(2) == em.VirtualDistillationConfig(2)
    assert em.VirtualDistillationConfig(2) != em.VirtualDistillationConfig(3)
    assert em.VirtualDistillationConfig(2).__eq__(object()) is NotImplemented

    assert em.MitigationMethod.zne(em.ZneConfig([0])) == em.MitigationMethod.zne(
        em.ZneConfig([0])
    )
    assert em.MitigationMethod.virtual_distillation(
        em.VirtualDistillationConfig(2)
    ) == em.MitigationMethod.virtual_distillation(em.VirtualDistillationConfig(2))
    assert em.MitigationMethod.zne(em.ZneConfig([0])).__eq__(object()) is NotImplemented

    assert em.RunArgs.zne(shots=128) == em.RunArgs.zne(shots=128)
    assert em.RunArgs.virtual_distillation(3, 2) == em.RunArgs.virtual_distillation(
        3, 2
    )
    assert em.RunArgs.zne(shots=128).__eq__(object()) is NotImplemented

    assert em.ProcessArgs.zne(
        em.ExtrapolateMethod.polynomial(), 1
    ) == em.ProcessArgs.zne(em.ExtrapolateMethod.polynomial(), 1)
    assert (
        em.ProcessArgs.virtual_distillation() == em.ProcessArgs.virtual_distillation()
    )
    assert (
        em.ProcessArgs.zne(em.ExtrapolateMethod.polynomial(), 1).__eq__(object())
        is NotImplemented
    )


def test_zne_fold_run_and_extrapolate_flow() -> None:
    circuit = single_x_circuit()
    hamiltonian = single_qubit_z_hamiltonian()
    zne = em.ZNEMitigation(circuit, [0, 1, 2])

    assert zne.fold_levels == [0, 1, 2]
    assert zne.noise_factors == [1, 3, 5]

    folded = zne.fold_circuits()
    assert len(folded) == 3
    assert [len(folded_circuit.operations) for folded_circuit in folded] == [1, 3, 5]
    assert len(circuit.operations) == 1

    calls: list[tuple[int, int | None, bool]] = []

    def estimator(
        folded_circuit: Circuit,
        observable: Hamiltonian | None,
        shots: int | None,
    ) -> tuple[float, float]:
        calls.append((len(folded_circuit.operations), shots, observable is hamiltonian))
        return (0.5 * len(folded_circuit.operations), 0.0)

    noisy = zne.run_em_sequence_with_shots(None, hamiltonian, 256, estimator)

    assert noisy == [0.5, 1.5, 2.5]
    assert calls == [(1, 256, False), (3, 256, False), (5, 256, False)]
    assert zne.extrapolate(
        noisy, em.ExtrapolateMethod.polynomial(), 1
    ) == pytest.approx(0.0)
    assert zne.poly_extrapolate(noisy, 1) == pytest.approx(0.0)
    assert zne.exp_extrapolate(
        [math.exp(-factor) for factor in zne.noise_factors]
    ) == pytest.approx(1.0)


def test_virtual_distillation_copy_swap_and_run_paths() -> None:
    circuit = Circuit(1)
    hamiltonian = single_qubit_z_hamiltonian()
    vd = em.VirtualDistillation(circuit, 2)

    copy_swap = vd.build_copy_swap_circuit()
    assert copy_swap.width == 2
    assert [
        operation.instruction.instruction.name for operation in copy_swap.operations
    ] == ["SWAP"]

    calls: list[tuple[int, int | None, bool]] = []

    def estimator(
        run_circuit: Circuit,
        observable: Hamiltonian | None,
        shots: int | None,
    ) -> tuple[float, float]:
        calls.append((run_circuit.width, shots, observable is None))
        if observable is None:
            return (2.0, 1.0)
        return (1.5, 0.25)

    assert vd.run_numerator_circuit(hamiltonian, 3, estimator) == (1.5, 0.25)
    assert vd.run_denominator_circuit(2, estimator) == (2.0, 1.0)
    mitigated = vd.run_vd(hamiltonian, 3, 2, estimator)

    assert mitigated[0] == pytest.approx(0.75)
    assert mitigated[1] == pytest.approx(0.203125)
    assert calls == [
        (2, 3, False),
        (2, 2, True),
        (2, 3, False),
        (2, 2, True),
    ]


def test_unified_error_mitigation_zne_pipeline_and_state_errors() -> None:
    circuit = single_x_circuit()
    hamiltonian = single_qubit_z_hamiltonian()
    mitigation = em.ErrorMitigation(
        circuit,
        em.MitigationMethod.zne(em.ZneConfig([0, 1, 2])),
    )

    with pytest.raises(em.ErrorMitigationError, match="run\\(\\) must be completed"):
        mitigation.get_mitigated(
            em.ProcessArgs.zne(em.ExtrapolateMethod.polynomial(), degree=1)
        )

    def estimator(
        run_circuit: Circuit,
        observable: Hamiltonian | None,
        shots: int | None,
    ) -> tuple[float, float]:
        assert observable is not None
        assert shots == 128
        return (0.5 * len(run_circuit.operations), 0.0)

    mitigation.run(hamiltonian, em.RunArgs.zne(shots=128), estimator)

    with pytest.raises(em.ErrorMitigationError, match="already been completed"):
        mitigation.run(hamiltonian, em.RunArgs.zne(shots=128), estimator)

    result = mitigation.get_mitigated(
        em.ProcessArgs.zne(em.ExtrapolateMethod.polynomial(), degree=1)
    )
    assert result.expectation == pytest.approx(0.0)
    assert result.variance is None
    assert result == copy.copy(result)

    with pytest.raises(em.ErrorMitigationError, match="already been completed"):
        mitigation.get_mitigated(
            em.ProcessArgs.zne(em.ExtrapolateMethod.polynomial(), degree=1)
        )


def test_unified_error_mitigation_virtual_distillation_pipeline() -> None:
    circuit = Circuit(1)
    hamiltonian = single_qubit_z_hamiltonian()
    mitigation = em.ErrorMitigation(
        circuit,
        em.MitigationMethod.virtual_distillation(em.VirtualDistillationConfig(2)),
    )

    def estimator(
        run_circuit: Circuit,
        observable: Hamiltonian | None,
        shots: int | None,
    ) -> tuple[float, float]:
        assert run_circuit.width == 2
        if observable is None:
            assert shots == 2
            return (2.0, 1.0)
        assert shots == 3
        return (1.5, 0.25)

    mitigation.run(hamiltonian, em.RunArgs.virtual_distillation(3, 2), estimator)
    result = mitigation.get_mitigated(em.ProcessArgs.virtual_distillation())

    assert result.expectation == pytest.approx(0.75)
    assert result.variance == pytest.approx(0.203125)


def test_error_mitigation_rejects_invalid_inputs_and_propagates_estimator_errors() -> (
    None
):
    circuit = Circuit(1)
    hamiltonian = single_qubit_z_hamiltonian()
    zne = em.ZNEMitigation(circuit, [0])

    with pytest.raises(TypeError, match="estimator must be callable"):
        zne.run_em_sequence_with_shots(None, hamiltonian, None, object())

    def raising_estimator(
        _circuit: Circuit,
        _observable: Hamiltonian | None,
        _shots: int | None,
    ) -> tuple[float, float]:
        raise RuntimeError("backend failed")

    with pytest.raises(RuntimeError, match="backend failed"):
        zne.run_em_sequence_with_shots(None, hamiltonian, None, raising_estimator)

    def invalid_result_estimator(
        _circuit: Circuit,
        _observable: Hamiltonian | None,
        _shots: int | None,
    ) -> tuple[float, float]:
        return (1.0,)  # type: ignore[return-value]

    with pytest.raises((TypeError, ValueError)):
        zne.run_em_sequence_with_shots(
            None, hamiltonian, None, invalid_result_estimator
        )

    with pytest.raises(em.ErrorMitigationError, match="at least 2 copies"):
        em.VirtualDistillation(circuit, 1)

    with pytest.raises(em.ErrorMitigationError, match="at least 2 copies"):
        em.ErrorMitigation(
            circuit,
            em.MitigationMethod.virtual_distillation(em.VirtualDistillationConfig(1)),
        )

    with pytest.raises(em.ErrorMitigationError, match="non-negative"):
        em.ErrorMitigation(circuit, em.MitigationMethod.zne(em.ZneConfig([0, -1])))
