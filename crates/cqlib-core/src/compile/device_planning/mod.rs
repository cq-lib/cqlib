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

//! Shared exact-qargs device planning used by routing and final lowering.

mod catalog;
pub(crate) mod cost;
mod planner;
mod registry;
mod session;
mod state;
mod swap_equivalence;
mod templates;

pub(crate) use catalog::NativePlanCatalog;
pub(crate) use cost::{
    CalibrationEstimator, DevicePhysicalCost, NativePlanCost, NativePlanLeaf, NativePlanSummary,
};
pub(crate) use planner::{DevicePlanner, DevicePlannerError, PlanChoice, PlanId, PlanTemplate};
pub(crate) use session::{
    DevicePlanSnapshot, DevicePlanningSession, NativePlanAvailability, SelectedNativePlan,
};
pub(crate) use state::DeviceGateState;
pub(crate) use templates::DirectionTemplate;

#[cfg(test)]
#[path = "device_planning_test.rs"]
mod device_planning_test;
