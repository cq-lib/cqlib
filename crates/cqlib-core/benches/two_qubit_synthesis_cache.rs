// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use cqlib_core::circuit::{Circuit, Qubit, StandardGate, UnitaryGate};
use cqlib_core::compile::transform::decompose::unitary::{
    TwoQubitSynthesisTarget, UnitaryDecomposeConfig, decompose_unitaries_with_rule_stats,
};
use cqlib_core::compile::transform::{
    TransformOutcome, TwoQubitBlockResynthesisConfig, resynthesize_two_qubit_blocks,
};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn repeated_two_qubit_unitaries(repetitions: usize) -> Circuit {
    let matrix = StandardGate::FSIM
        .matrix(&[0.23, -0.31])
        .expect("fSim matrix")
        .into_owned();
    let mut circuit = Circuit::new(2);
    for _ in 0..repetitions {
        let gate = UnitaryGate::new("repeated", 2, 0)
            .with_matrix(matrix.clone())
            .expect("valid two-qubit unitary gate");
        circuit
            .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
            .expect("valid unitary operation");
    }
    circuit
}

fn benchmark_repeated_two_qubit_synthesis_cache(criterion: &mut Criterion) {
    let circuit = repeated_two_qubit_unitaries(128);
    criterion.bench_function(
        "2q_unitary_decomposition/repeated_cache_warm_path",
        |bencher| {
            bencher.iter(|| {
                let (_, stats) = decompose_unitaries_with_rule_stats(
                    &circuit,
                    UnitaryDecomposeConfig::default(),
                )
                .expect("repeated 2q unitary decomposition");
                assert_eq!(stats.misses, 1);
                assert_eq!(stats.inserts, 1);
                assert_eq!(stats.hits, 127);
            });
        },
    );

    let repeated = repeated_resynthesis_blocks(128, false);
    let unique = repeated_resynthesis_blocks(128, true);
    let config = resynthesis_config();
    criterion.bench_function("2q_resynthesis/repeated_exact_keys", |bencher| {
        bencher.iter(|| {
            black_box(
                resynthesize_two_qubit_blocks(&repeated, config.clone())
                    .expect("repeated 2q block resynthesis"),
            );
        });
    });
    criterion.bench_function("2q_resynthesis/unique_exact_keys", |bencher| {
        bencher.iter(|| {
            black_box(
                resynthesize_two_qubit_blocks(&unique, config.clone())
                    .expect("unique 2q block resynthesis"),
            );
        });
    });

    // Alternating commuting CX stars defeat maximal-run collection and create
    // many overlapping dependency-DAG blocks. This specifically tracks the
    // selector's conflict-component and optimistic-bound pruning path.
    let overlapping = overlapping_bounded_blocks(128);
    assert!(matches!(
        resynthesize_two_qubit_blocks(&overlapping, config.clone())
            .expect("overlapping bounded 2q block resynthesis setup"),
        TransformOutcome::Changed(_)
    ));
    criterion.bench_function("2q_resynthesis/overlapping_bounded_blocks", |bencher| {
        bencher.iter(|| {
            black_box(
                resynthesize_two_qubit_blocks(&overlapping, config.clone())
                    .expect("overlapping bounded 2q block resynthesis"),
            );
        });
    });
}

fn resynthesis_config() -> TwoQubitBlockResynthesisConfig {
    TwoQubitBlockResynthesisConfig::normal(
        TwoQubitSynthesisTarget::from_standard_gates(
            vec![StandardGate::U, StandardGate::H, StandardGate::RZ],
            vec![StandardGate::CX],
            true,
        )
        .expect("valid benchmark synthesis target"),
    )
}

fn repeated_resynthesis_blocks(blocks: usize, unique: bool) -> Circuit {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    for index in 0..blocks {
        // Barriers create independent collector regions. The repeated workload
        // therefore presents the planner with the same exact block key many
        // times, while the unique workload changes one matrix parameter per
        // region and measures cache-miss overhead.
        let angle = if unique {
            0.1 + index as f64 * 0.001
        } else {
            0.37
        };
        circuit.h(q0).expect("benchmark H");
        circuit.cx(q0, q1).expect("benchmark CX");
        circuit.rz(q1, angle).expect("benchmark RZ");
        circuit.cx(q0, q1).expect("benchmark CX");
        circuit
            .barrier(vec![q0, q1])
            .expect("benchmark block boundary");
    }
    circuit
}

fn overlapping_bounded_blocks(operations: usize) -> Circuit {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    for index in 0..operations {
        let target = if index % 2 == 0 { q1 } else { q2 };
        circuit.cx(q0, target).expect("benchmark commuting CX");
    }
    circuit
}

criterion_group!(benches, benchmark_repeated_two_qubit_synthesis_cache);
criterion_main!(benches);
