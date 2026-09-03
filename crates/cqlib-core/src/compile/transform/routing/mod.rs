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

//! Device-aware routing transforms.
//!
//! Routing transforms rebuild a logical [`Circuit`](crate::circuit::Circuit)
//! into a physical circuit that satisfies a target
//! [`Device`](crate::device::Device) topology. A routing algorithm selects or
//! consumes a logical-to-physical layout and inserts SWAP operations when
//! needed.
//!
//! This module is an algorithm boundary, not a compiler workflow. It does not
//! choose when routing should run, lower operations to a target basis, or mutate
//! device topology facts. Concrete algorithms live in submodules such as
//! [`sabre`].
//!
//! # Entry Points
//!
//! - [`route_sabre`] selects an initial layout with SABRE layout refinement,
//!   then routes the circuit from that layout.
//! - [`route_with_layout`] routes from a caller-supplied initial layout and
//!   skips automatic layout selection.
//!
//! The public compiler workflow uses [`route_with_layout`] when
//! [`DeviceCompileTarget::initial_layout`](crate::compile::DeviceCompileTarget::initial_layout)
//! is set, otherwise it uses [`route_sabre`]. Routed circuits use physical
//! qubit identifiers and guarantee undirected physical adjacency for routed
//! two-qubit operations. Target-basis translation and exact directed device
//! lowering remain separate compiler stages.
//!
//! # Example
//!
//! ```
//! use cqlib_core::circuit::{Circuit, Qubit};
//! use cqlib_core::compile::sabre::SabreConfig;
//! use cqlib_core::compile::transform::LayoutObjective;
//! use cqlib_core::compile::transform::routing::route_sabre;
//! use cqlib_core::device::{Device, PhysicalQubit, Topology};
//! use std::collections::HashSet;
//!
//! let physical = vec![
//!     PhysicalQubit::new(0),
//!     PhysicalQubit::new(1),
//!     PhysicalQubit::new(2),
//! ];
//! let topology = Topology::new(
//!     physical.clone(),
//!     vec![
//!         (PhysicalQubit::new(0), PhysicalQubit::new(1), "cx".to_string()),
//!         (PhysicalQubit::new(1), PhysicalQubit::new(2), "cx".to_string()),
//!     ],
//! )?;
//! let device = Device::new(
//!     "line3",
//!     physical.iter().copied().collect::<HashSet<_>>(),
//!     topology,
//! )?;
//!
//! let mut circuit = Circuit::new(3);
//! circuit.cx(Qubit::new(0), Qubit::new(2))?;
//!
//! let routed = route_sabre(
//!     &circuit,
//!     &device,
//!     &LayoutObjective::topology_only(),
//!     &SabreConfig::default(),
//! )?;
//!
//! assert_all_two_qubit_ops_are_local(routed.circuit());
//! # fn assert_all_two_qubit_ops_are_local(_: &Circuit) {}
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod sabre;

pub use sabre::{RoutedCircuit, SabreRouteResult, route_sabre, route_with_layout};
pub(crate) use sabre::{
    SabreParetoRouteCandidate, prepare_sabre_pareto_routes_with_session_on_physical,
    route_sabre_tracked_on_topology, route_sabre_tracked_with_session_on_physical,
    route_with_layout_tracked_on_topology, route_with_layout_tracked_with_session_on_physical,
};
