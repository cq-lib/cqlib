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

"""Tests for circuit ansatz Python bindings."""

import copy

from cqlib.circuit import Circuit, StandardGate
from cqlib.circuit.ansatz import (
    AngleEncoding,
    BasicEntanglerLayers,
    BasisEncoding,
    EntanglementTopology,
    EvolutionStrategy,
    IQPFeatureMap,
    PauliEvolutionAnsatz,
    PauliFeatureMap,
    QAOAAnsatz,
    StronglyEntanglingLayers,
    TwoLocal,
    ZFeatureMap,
    ZZFeatureMap,
    efficient_su2,
    pauli_feature_map,
    real_amplitudes,
    zz_feature_map,
)
from cqlib.qis import Hamiltonian, PauliString, TrotterMode


def commuting_hamiltonian() -> Hamiltonian:
    hamiltonian = Hamiltonian(2)
    hamiltonian.add_term(PauliString.from_str("ZZ"), 0.5)
    hamiltonian.add_term(PauliString.from_str("IZ"), 0.25)
    return hamiltonian


def assert_copy_protocol(value):
    copied = copy.copy(value)
    deep_copied = copy.deepcopy(value)

    assert type(copied) is type(value)
    assert type(deep_copied) is type(value)
    assert repr(copied) == repr(value)
    assert repr(deep_copied) == repr(value)


def assert_builds_symbolic_circuit(ansatz, prefix: str, expected_symbols: set[str]):
    circuit = ansatz.build_circuit(prefix)

    assert isinstance(circuit, Circuit)
    assert circuit.num_qubits == ansatz.num_qubits()
    assert len(circuit) > 0
    assert set(circuit.symbols) == expected_symbols


def test_ansatz_exports_match_runtime_all():
    import cqlib.circuit.ansatz as ansatz

    assert sorted(ansatz.__all__) == sorted(
        [
            "AngleEncoding",
            "BasicEntanglerLayers",
            "BasisEncoding",
            "EntanglementTopology",
            "EvolutionInfo",
            "EvolutionStrategy",
            "IQPFeatureMap",
            "PauliEvolutionAnsatz",
            "PauliFeatureMap",
            "QAOAAnsatz",
            "StronglyEntanglingLayers",
            "TwoLocal",
            "ZFeatureMap",
            "ZZFeatureMap",
            "efficient_su2",
            "pauli_feature_map",
            "real_amplitudes",
            "zz_feature_map",
        ]
    )


def test_entanglement_topology_copy_and_pairs():
    topology = EntanglementTopology.custom([(0, 2), (1, 2)])

    assert topology.generate_pairs(3) == [(0, 2), (1, 2)]
    assert EntanglementTopology.linear().generate_pairs(3) == [(0, 1), (1, 2)]
    assert EntanglementTopology.circular().generate_pairs(3) == [
        (0, 1),
        (1, 2),
        (2, 0),
    ]
    assert EntanglementTopology.full().generate_pairs(3) == [(0, 1), (0, 2), (1, 2)]
    assert_copy_protocol(topology)


def test_two_local_builders_copy_and_build_circuit():
    ansatz = (
        TwoLocal(3)
        .reps(2)
        .rotation_gates([StandardGate.RY, StandardGate.RZ])
        .entanglement_gate(StandardGate.CZ)
        .entanglement(EntanglementTopology.linear())
    )

    assert ansatz.num_qubits() == 3
    assert ansatz.num_parameters() == 18
    assert_builds_symbolic_circuit(
        ansatz,
        "theta",
        {f"theta_{index}" for index in range(ansatz.num_parameters())},
    )
    assert_copy_protocol(ansatz)


def test_angle_encoding_copy_and_build_circuit():
    encoding = AngleEncoding(3, StandardGate.RX)

    assert encoding.num_qubits() == 3
    assert encoding.num_parameters() == 3
    assert_builds_symbolic_circuit(encoding, "x", {"x_0", "x_1", "x_2"})
    assert_copy_protocol(encoding)


def test_basis_encoding_copy_and_build_circuit():
    encoding = BasisEncoding([True, False, True])
    circuit = encoding.build_circuit("ignored")

    assert isinstance(circuit, Circuit)
    assert encoding.num_qubits() == 3
    assert encoding.num_parameters() == 0
    assert len(circuit) == 2
    assert set(circuit.symbols) == set()
    assert_copy_protocol(encoding)


def test_z_feature_map_copy_and_build_circuit():
    feature_map = ZFeatureMap(3).reps(1)

    assert feature_map.num_qubits() == 3
    assert feature_map.num_parameters() == 3
    assert_builds_symbolic_circuit(feature_map, "x", {"x_0", "x_1", "x_2"})
    assert_copy_protocol(feature_map)


def test_iqp_feature_map_copy_and_build_circuit():
    feature_map = IQPFeatureMap(3).reps(1).entanglement(EntanglementTopology.linear())

    assert feature_map.num_qubits() == 3
    assert feature_map.num_parameters() == 3
    assert_builds_symbolic_circuit(feature_map, "x", {"x_0", "x_1", "x_2"})
    assert_copy_protocol(feature_map)


def test_zz_feature_map_copy_and_build_circuit():
    feature_map = ZZFeatureMap(3).reps(1).entanglement(EntanglementTopology.linear())

    assert feature_map.num_qubits() == 3
    assert feature_map.num_parameters() == 3
    assert_builds_symbolic_circuit(feature_map, "x", {"x_0", "x_1", "x_2"})
    assert_copy_protocol(feature_map)


def test_pauli_feature_map_copy_and_build_circuit():
    feature_map = (
        PauliFeatureMap(3)
        .reps(1)
        .paulis([PauliString.from_str("Z"), PauliString.from_str("ZZ")])
        .entanglement(EntanglementTopology.full())
        .parameter_prefix("data")
    )

    assert feature_map.num_qubits() == 3
    assert feature_map.num_parameters() == 3
    assert_builds_symbolic_circuit(
        feature_map, "ignored", {"ignored_0", "ignored_1", "ignored_2"}
    )
    assert_copy_protocol(feature_map)


def test_basic_entangler_layers_copy_and_build_circuit():
    ansatz = (
        BasicEntanglerLayers(3)
        .reps(2)
        .rotation_gate(StandardGate.RY)
        .entanglement_gate(StandardGate.CZ)
    )

    assert ansatz.num_qubits() == 3
    assert ansatz.num_parameters() == 6
    assert_builds_symbolic_circuit(
        ansatz,
        "theta",
        {f"theta_{index}" for index in range(ansatz.num_parameters())},
    )
    assert_copy_protocol(ansatz)


def test_strongly_entangling_layers_copy_and_build_circuit():
    ansatz = (
        StronglyEntanglingLayers(3)
        .reps(2)
        .ranges([1, 2])
        .entanglement_gate(StandardGate.CX)
    )

    assert ansatz.num_qubits() == 3
    assert ansatz.num_parameters() == 18
    assert_builds_symbolic_circuit(
        ansatz,
        "theta",
        {f"theta_{index}" for index in range(ansatz.num_parameters())},
    )
    assert_copy_protocol(ansatz)


def test_qaoa_copy_and_build_circuit():
    ansatz = QAOAAnsatz(commuting_hamiltonian()).reps(2)

    assert ansatz.num_qubits() == 2
    assert ansatz.num_parameters() == 4
    assert_builds_symbolic_circuit(
        ansatz,
        "p",
        {"p_gamma_0", "p_beta_0", "p_gamma_1", "p_beta_1"},
    )
    assert_copy_protocol(ansatz)


def test_qaoa_custom_mixer_and_initial_state():
    mixer = Hamiltonian(2)
    mixer.add_term(PauliString.from_str("XI"), 1.0)
    mixer.add_term(PauliString.from_str("IX"), 1.0)

    initial = Circuit(2)
    initial.h(0)
    initial.h(1)

    ansatz = QAOAAnsatz(commuting_hamiltonian()).mixer(mixer).initial_state(initial)
    assert ansatz.num_qubits() == 2
    assert ansatz.num_parameters() == 2
    assert_builds_symbolic_circuit(ansatz, "qaoa", {"qaoa_gamma_0", "qaoa_beta_0"})


def test_pauli_evolution_ansatz_copy_and_strategy_info():
    ansatz = (
        PauliEvolutionAnsatz(commuting_hamiltonian())
        .with_strategy(EvolutionStrategy.trotter(TrotterMode.second_order(), 3))
        .with_time_param_name("tau")
    )
    info = ansatz.evolution_info()

    assert info.all_terms_commute is True
    assert info.num_terms == 2
    assert info.steps == 3
    assert info.trotter_mode == TrotterMode.second_order()
    assert_copy_protocol(info)
    assert_copy_protocol(ansatz)
    assert_copy_protocol(EvolutionStrategy.auto())
    assert_builds_symbolic_circuit(ansatz, "ignored", {"tau"})


def test_ansatz_facades_build_expected_types():
    topology = EntanglementTopology.linear()

    real = real_amplitudes(3, 2, topology)
    efficient = efficient_su2(2, 1, topology)
    zz = zz_feature_map(3, 1, topology)
    pauli = pauli_feature_map(
        3,
        1,
        [PauliString.from_str("Z"), PauliString.from_str("ZZ")],
        topology,
    )

    assert isinstance(real, TwoLocal)
    assert real.num_parameters() == 9
    assert isinstance(efficient, TwoLocal)
    assert efficient.num_parameters() == 8
    assert isinstance(zz, ZZFeatureMap)
    assert zz.num_parameters() == 3
    assert isinstance(pauli, PauliFeatureMap)
    assert pauli.num_parameters() == 3
