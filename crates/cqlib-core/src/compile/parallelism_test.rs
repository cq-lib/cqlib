// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::*;
use rayon::ThreadPoolBuilder;

const THRESHOLDS: ParallelismThresholds = ParallelismThresholds::new(100, 400, 1_600);
const FOUR_GROUP_THRESHOLDS: ParallelismThresholds = ParallelismThresholds::up_to_four(100, 400);

fn chunk_size_with(
    threads: usize,
    tasks: usize,
    work: usize,
    thresholds: ParallelismThresholds,
) -> Option<usize> {
    ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap()
        .install(|| parallel_chunk_size(tasks, work, thresholds))
}

fn chunk_size(threads: usize, tasks: usize, work: usize) -> Option<usize> {
    chunk_size_with(threads, tasks, work, THRESHOLDS)
}

#[test]
fn small_or_single_task_workloads_stay_serial() {
    assert_eq!(chunk_size(8, 8, 99), None);
    assert_eq!(chunk_size(8, 1, 10_000), None);
    assert_eq!(chunk_size(1, 8, 10_000), None);
}

#[test]
fn work_and_pool_size_bound_task_groups() {
    assert_eq!(chunk_size(8, 9, 100), Some(5));
    assert_eq!(chunk_size(8, 9, 400), Some(3));
    assert_eq!(chunk_size(8, 9, 1_600), Some(2));
    assert_eq!(chunk_size(4, 9, 1_600), Some(3));
    assert_eq!(chunk_size(3, 9, 1_600), Some(5));
}

#[test]
fn phase_ceiling_can_keep_large_workloads_at_four_groups() {
    assert_eq!(
        chunk_size_with(8, 16, usize::MAX, FOUR_GROUP_THRESHOLDS),
        Some(4)
    );
}
