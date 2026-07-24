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
//
// Modified from the original work.

//! Logical-to-physical qubit layout management for circuit routing.
//!
//! [`Layout`] tracks where circuit logical qubits are placed on a quantum
//! device. Physical qubits that do not carry a logical qubit remain vacant and
//! may be used by routing or by a later compiler step that explicitly activates
//! an auxiliary logical qubit.
//!
//! Layout does not allocate auxiliary qubits. Algorithm auxiliary qubits are
//! logical circuit resources and must be managed by the compiler resource
//! manager before they are bound to physical qubits.
//!
//! # Example
//!
//! ```
//! use cqlib_core::device::{Layout, LogicalQubit, PhysicalQubit};
//!
//! let logical = vec![LogicalQubit::new(0), LogicalQubit::new(1)];
//! let physical = vec![
//!     PhysicalQubit::new(100),
//!     PhysicalQubit::new(101),
//!     PhysicalQubit::new(102),
//! ];
//!
//! let mut layout = Layout::new(logical, physical, None).unwrap();
//! assert_eq!(layout.num_vacant_physical(), 1);
//!
//! layout
//!     .bind(LogicalQubit::new(2), PhysicalQubit::new(102))
//!     .unwrap();
//! assert_eq!(layout.num_vacant_physical(), 0);
//! ```

use crate::device::error::LayoutError;
use crate::device::{LogicalQubit, PhysicalQubit};
use std::collections::{BTreeMap, BTreeSet};

/// Maps circuit logical qubits to physical qubits on a quantum device.
///
/// A layout owns the set of physical qubits available to a placement or
/// routing step. Every logical qubit present in the layout has exactly one
/// physical mapping. A physical qubit may be vacant.
///
/// Auxiliary qubit ownership and lifetime are intentionally outside this type.
/// From the layout perspective, an auxiliary logical qubit behaves like any
/// other logical qubit after it is explicitly [`Layout::bind`]ed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Layout {
    /// Physical qubits available to this layout.
    physical_qubits: BTreeSet<PhysicalQubit>,
    /// Mapping from circuit logical qubits to device physical qubits.
    l2p: BTreeMap<LogicalQubit, PhysicalQubit>,
    /// Reverse mapping from device physical qubits to circuit logical qubits.
    p2l: BTreeMap<PhysicalQubit, LogicalQubit>,
}

impl Layout {
    /// Creates a layout and maps each supplied logical qubit to a physical qubit.
    ///
    /// Entries in `init_map` are applied first. Remaining logical qubits are
    /// mapped to remaining physical qubits in the order supplied by `logical`
    /// and `physical`. Extra physical qubits remain vacant.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError`] if:
    /// - there are more logical qubits than physical qubits;
    /// - the logical or physical qubit list contains duplicate entries;
    /// - `init_map` references an undeclared logical or physical qubit;
    /// - `init_map` maps multiple logical qubits to one physical qubit.
    ///
    /// # Example
    ///
    /// ```
    /// use cqlib_core::device::{Layout, LogicalQubit, PhysicalQubit};
    /// use std::collections::BTreeMap;
    ///
    /// let logical = vec![LogicalQubit::new(0), LogicalQubit::new(1)];
    /// let physical = vec![PhysicalQubit::new(100), PhysicalQubit::new(101)];
    ///
    /// let mut init = BTreeMap::new();
    /// init.insert(LogicalQubit::new(0), PhysicalQubit::new(101));
    ///
    /// let layout = Layout::new(logical, physical, Some(init)).unwrap();
    /// assert_eq!(
    ///     layout.get_physical(LogicalQubit::new(0)),
    ///     Some(PhysicalQubit::new(101)),
    /// );
    /// ```
    pub fn new(
        logical: Vec<LogicalQubit>,
        physical: Vec<PhysicalQubit>,
        init_map: Option<BTreeMap<LogicalQubit, PhysicalQubit>>,
    ) -> Result<Self, LayoutError> {
        if logical.len() > physical.len() {
            return Err(LayoutError::TooManyLogicalQubits {
                logical: logical.len(),
                physical: physical.len(),
            });
        }

        let mut logical_qubits = BTreeSet::new();
        for logical in logical.iter().copied() {
            if !logical_qubits.insert(logical) {
                return Err(LayoutError::DuplicateLogicalQubit(logical));
            }
        }

        let mut physical_qubits = BTreeSet::new();
        for physical in physical.iter().copied() {
            if !physical_qubits.insert(physical) {
                return Err(LayoutError::DuplicatePhysicalQubit(physical));
            }
        }

        let mut layout = Self {
            physical_qubits,
            l2p: BTreeMap::new(),
            p2l: BTreeMap::new(),
        };

        if let Some(init_map) = init_map {
            for (logical, physical) in init_map {
                if !logical_qubits.contains(&logical) {
                    return Err(LayoutError::InvalidLogicalQubit(logical));
                }
                if !layout.physical_qubits.contains(&physical) {
                    return Err(LayoutError::InvalidPhysicalQubit(physical));
                }
                if layout.p2l.contains_key(&physical) {
                    return Err(LayoutError::PhysicalQubitAlreadyOccupied(physical));
                }

                layout.l2p.insert(logical, physical);
                layout.p2l.insert(physical, logical);
            }
        }

        let vacant_physical: Vec<_> = physical
            .into_iter()
            .filter(|physical| !layout.p2l.contains_key(physical))
            .collect();
        let unmapped_logical: Vec<_> = logical
            .into_iter()
            .filter(|logical| !layout.l2p.contains_key(logical))
            .collect();

        for (logical, physical) in unmapped_logical.into_iter().zip(vacant_physical) {
            debug_assert!(
                !layout.p2l.contains_key(&physical),
                "vacant physical qubit became occupied before initial mapping completed"
            );
            layout.l2p.insert(logical, physical);
            layout.p2l.insert(physical, logical);
        }

        debug_assert_eq!(
            layout.l2p.len(),
            logical_qubits.len(),
            "logical qubit count was validated against physical capacity"
        );
        debug_assert_eq!(
            layout.l2p.len(),
            layout.p2l.len(),
            "each logical mapping must have one reverse mapping"
        );
        Ok(layout)
    }

    /// Creates a layout from `(logical, physical)` qubit ID pairs.
    ///
    /// Logical qubits are exactly the logical IDs that appear in
    /// `logical_physical`. Physical qubits are `0..physical_count`; any
    /// physical qubit not referenced by a pair remains vacant.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError`] if a logical ID appears more than once, a
    /// physical ID appears more than once, or any physical ID is outside
    /// `0..physical_count`.
    ///
    /// # Example
    ///
    /// ```
    /// use cqlib_core::device::{Layout, LogicalQubit, PhysicalQubit};
    ///
    /// let layout = Layout::from_pairs(&[(0, 2), (1, 0)], 4).unwrap();
    ///
    /// assert_eq!(
    ///     layout.get_physical(LogicalQubit::new(0)),
    ///     Some(PhysicalQubit::new(2)),
    /// );
    /// assert_eq!(layout.num_vacant_physical(), 2);
    /// ```
    pub fn from_pairs(
        logical_physical: &[(u32, u32)],
        physical_count: u32,
    ) -> Result<Self, LayoutError> {
        let logical = logical_physical
            .iter()
            .map(|&(logical, _)| LogicalQubit::new(logical))
            .collect::<Vec<_>>();
        let physical = (0..physical_count)
            .map(PhysicalQubit::new)
            .collect::<Vec<_>>();
        let init_map = logical_physical
            .iter()
            .map(|&(logical, physical)| (LogicalQubit::new(logical), PhysicalQubit::new(physical)))
            .collect::<BTreeMap<_, _>>();
        Self::new(logical, physical, Some(init_map))
    }

    /// Returns the number of mapped logical qubits.
    pub fn num_logical(&self) -> usize {
        self.l2p.len()
    }

    /// Returns the number of physical qubits available to the layout.
    pub fn num_physical(&self) -> usize {
        self.physical_qubits.len()
    }

    /// Returns the number of physical qubits that do not carry a logical qubit.
    pub fn num_vacant_physical(&self) -> usize {
        self.physical_qubits.len() - self.p2l.len()
    }

    /// Returns the physical qubit carrying `logical`, if it is bound.
    pub fn get_physical(&self, logical: LogicalQubit) -> Option<PhysicalQubit> {
        self.l2p.get(&logical).copied()
    }

    /// Returns the logical qubit carried by `physical`, if the position is occupied.
    pub fn get_logical(&self, physical: PhysicalQubit) -> Option<LogicalQubit> {
        self.p2l.get(&physical).copied()
    }

    /// Returns an iterator over mapped logical qubits.
    pub fn logical_qubits(&self) -> impl Iterator<Item = LogicalQubit> + '_ {
        self.l2p.keys().copied()
    }

    /// Returns an iterator over physical qubits available to the layout.
    pub fn physical_qubits(&self) -> impl Iterator<Item = PhysicalQubit> + '_ {
        self.physical_qubits.iter().copied()
    }

    /// Returns an iterator over vacant physical qubits.
    pub fn vacant_physical_qubits(&self) -> impl Iterator<Item = PhysicalQubit> + '_ {
        self.physical_qubits
            .iter()
            .copied()
            .filter(|physical| !self.p2l.contains_key(physical))
    }

    /// Returns whether `physical` belongs to the layout and is vacant.
    pub fn is_physical_vacant(&self, physical: PhysicalQubit) -> bool {
        self.physical_qubits.contains(&physical) && !self.p2l.contains_key(&physical)
    }

    /// Returns the logical-to-physical qubit mapping.
    pub fn l2p_map(&self) -> &BTreeMap<LogicalQubit, PhysicalQubit> {
        &self.l2p
    }

    /// Returns the physical-to-logical qubit mapping.
    pub fn p2l_map(&self) -> &BTreeMap<PhysicalQubit, LogicalQubit> {
        &self.p2l
    }

    /// Binds an unmapped logical qubit to a vacant physical qubit.
    ///
    /// This operation may introduce a new logical qubit to the layout. The
    /// caller is responsible for ensuring that the logical qubit exists in the
    /// circuit and is registered with the compiler resource manager when
    /// required.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::InvalidPhysicalQubit`] if `physical` does not
    /// belong to the layout. Returns [`LayoutError::LogicalQubitAlreadyBound`]
    /// or [`LayoutError::PhysicalQubitAlreadyOccupied`] if either qubit already
    /// participates in a mapping.
    pub fn bind(
        &mut self,
        logical: LogicalQubit,
        physical: PhysicalQubit,
    ) -> Result<(), LayoutError> {
        if !self.physical_qubits.contains(&physical) {
            return Err(LayoutError::InvalidPhysicalQubit(physical));
        }
        if self.l2p.contains_key(&logical) {
            return Err(LayoutError::LogicalQubitAlreadyBound(logical));
        }
        if self.p2l.contains_key(&physical) {
            return Err(LayoutError::PhysicalQubitAlreadyOccupied(physical));
        }

        self.l2p.insert(logical, physical);
        self.p2l.insert(physical, logical);
        Ok(())
    }

    /// Removes the mapping for `logical` and returns the released physical qubit.
    pub fn unbind(&mut self, logical: LogicalQubit) -> Result<PhysicalQubit, LayoutError> {
        let physical = self
            .l2p
            .remove(&logical)
            .ok_or(LayoutError::LogicalQubitNotBound(logical))?;
        self.p2l.remove(&physical);
        Ok(physical)
    }

    /// Swaps the logical qubits carried by two physical qubits.
    ///
    /// Either physical qubit may be vacant. Swapping an occupied qubit with a
    /// vacant qubit moves the logical qubit to the vacant position.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::InvalidPhysicalQubit`] if either physical qubit
    /// does not belong to the layout.
    pub fn swap_physical(
        &mut self,
        phys_a: PhysicalQubit,
        phys_b: PhysicalQubit,
    ) -> Result<(), LayoutError> {
        if !self.physical_qubits.contains(&phys_a) {
            return Err(LayoutError::InvalidPhysicalQubit(phys_a));
        }
        if !self.physical_qubits.contains(&phys_b) {
            return Err(LayoutError::InvalidPhysicalQubit(phys_b));
        }
        if phys_a == phys_b {
            return Ok(());
        }

        let logical_a = self.p2l.remove(&phys_a);
        let logical_b = self.p2l.remove(&phys_b);

        if let Some(logical) = logical_a {
            self.l2p.insert(logical, phys_b);
            self.p2l.insert(phys_b, logical);
        }
        if let Some(logical) = logical_b {
            self.l2p.insert(logical, phys_a);
            self.p2l.insert(phys_a, logical);
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "./layout_test.rs"]
mod layout_test;
