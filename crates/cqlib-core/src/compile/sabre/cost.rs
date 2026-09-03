// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Compatibility boundary for native-plan costs used by SABRE routing.

#[cfg(test)]
pub(crate) use crate::compile::device_planning::cost::{
    CalibrationEstimator, RobustDurationKey, RobustErrorKey,
};
pub(crate) use crate::compile::device_planning::cost::{MetricAvailability, NativePlanCost};

#[cfg(test)]
use crate::compile::device_planning::cost::{negative_log_success, quantiles};
#[cfg(test)]
use std::cmp::Ordering;
#[cfg(test)]
use std::collections::HashMap;

#[cfg(test)]
#[path = "cost_test.rs"]
mod cost_test;
