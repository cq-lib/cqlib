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

//! Internal workload-aware scheduling for compiler searches.
//!
//! The compiler shares Rayon's process-wide pool. Search phases use this
//! module only to bound how many independent task groups they expose to that
//! pool; they never construct a pool during compilation. Keeping task groups
//! deterministic also keeps worker-local allocation and reduction behavior
//! stable across executions.

/// Work-unit boundaries and ceiling for exposing parallel task groups.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ParallelismThresholds {
    two: usize,
    four: usize,
    eight: Option<usize>,
}

impl ParallelismThresholds {
    pub(crate) const fn new(two: usize, four: usize, eight: usize) -> Self {
        assert!(two <= four);
        assert!(four <= eight);
        Self {
            two,
            four,
            eight: Some(eight),
        }
    }

    /// Creates thresholds for a phase whose measured scaling stops at four
    /// independent groups.
    pub(crate) const fn up_to_four(two: usize, four: usize) -> Self {
        assert!(two <= four);
        Self {
            two,
            four,
            eight: None,
        }
    }
}

/// Returns the deterministic contiguous chunk size for a search workload.
///
/// `task_count` counts independent, quality-equivalent search tasks and
/// `work_units` is a phase-local estimate. The returned chunk size exposes at
/// most 2, 4, or 8 groups, bounded by both the current Rayon pool and the
/// number of independent tasks. `None` selects the direct serial path.
pub(crate) fn parallel_chunk_size(
    task_count: usize,
    work_units: usize,
    thresholds: ParallelismThresholds,
) -> Option<usize> {
    let available = rayon::current_num_threads().min(task_count).min(8);
    if available < 2 || work_units < thresholds.two {
        return None;
    }

    let desired = if available >= 8 && thresholds.eight.is_some_and(|eight| work_units >= eight) {
        8
    } else if available >= 4 && work_units >= thresholds.four {
        4
    } else {
        2
    };
    Some(task_count.div_ceil(desired.min(available)))
}

#[cfg(test)]
#[path = "./parallelism_test.rs"]
mod parallelism_test;
