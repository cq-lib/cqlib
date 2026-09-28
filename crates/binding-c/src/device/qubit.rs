// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating that
// they have been altered from the originals.

//! C ABI for the strongly typed device-facing qubit identifiers.
//!
//! Circuit operations use [`CQubit`] as logical wire identifiers.
//! Device-facing code distinguishes those logical identifiers from physical
//! hardware positions: [`CLogicalQubit`] marks circuit wires and
//! [`CPhysicalQubit`] marks hardware positions. The two types share the same
//! numeric representation but never convert into each other directly; both
//! convert to and from [`CQubit`].

use crate::circuit::qubit::CQubit;
use cqlib_core::device::{LogicalQubit, PhysicalQubit};
use std::ffi::CString;
use std::os::raw::c_char;

/// Logical qubit identifier marking a circuit wire; a value handle passed by
/// value. Compare and hash via the numeric ID.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CLogicalQubit {
    /// Numeric identifier of the logical qubit.
    pub id: u32,
}

/// Physical qubit identifier marking a hardware position; a value handle
/// passed by value. Compare and hash via the numeric ID.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CPhysicalQubit {
    /// Numeric identifier of the physical qubit.
    pub id: u32,
}

impl From<LogicalQubit> for CLogicalQubit {
    fn from(qubit: LogicalQubit) -> Self {
        Self { id: qubit.id() }
    }
}

impl From<CLogicalQubit> for LogicalQubit {
    fn from(qubit: CLogicalQubit) -> Self {
        LogicalQubit::new(qubit.id)
    }
}

impl From<PhysicalQubit> for CPhysicalQubit {
    fn from(qubit: PhysicalQubit) -> Self {
        Self { id: qubit.id() }
    }
}

impl From<CPhysicalQubit> for PhysicalQubit {
    fn from(qubit: CPhysicalQubit) -> Self {
        PhysicalQubit::new(qubit.id)
    }
}

/// Creates a logical qubit identifier from its numeric ID.
#[unsafe(no_mangle)]
pub extern "C" fn logical_qubit_new(id: u32) -> CLogicalQubit {
    CLogicalQubit { id }
}

/// Wraps a circuit qubit as a logical qubit identifier.
#[unsafe(no_mangle)]
pub extern "C" fn logical_qubit_from_qubit(qubit: CQubit) -> CLogicalQubit {
    CLogicalQubit {
        id: LogicalQubit::from_qubit(qubit.into()).id(),
    }
}

/// Returns the underlying circuit qubit identifier.
#[unsafe(no_mangle)]
pub extern "C" fn logical_qubit_qubit(qubit: CLogicalQubit) -> CQubit {
    CQubit::from(LogicalQubit::from(qubit).qubit())
}

/// Returns the numeric identifier of the logical qubit.
#[unsafe(no_mangle)]
pub extern "C" fn logical_qubit_id(qubit: CLogicalQubit) -> u32 {
    LogicalQubit::from(qubit).id()
}

/// Returns 1 when both logical qubits carry the same numeric identifier,
/// else 0.
#[unsafe(no_mangle)]
pub extern "C" fn logical_qubit_equal(a: CLogicalQubit, b: CLogicalQubit) -> i32 {
    LogicalQubit::from(a).eq(&LogicalQubit::from(b)) as i32
}

/// Compares two logical qubits by numeric identifier: -1 when `a < b`, 0
/// when equal, 1 when `a > b`.
#[unsafe(no_mangle)]
pub extern "C" fn logical_qubit_compare(a: CLogicalQubit, b: CLogicalQubit) -> i32 {
    match LogicalQubit::from(a).cmp(&LogicalQubit::from(b)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Formats the logical qubit identifier as `L<id>` (e.g. `L3`); free with
/// `cqlib_string_free`. Returns NULL on allocation failure.
#[unsafe(no_mangle)]
pub extern "C" fn logical_qubit_to_string(qubit: CLogicalQubit) -> *mut c_char {
    CString::new(format!("{}", LogicalQubit::from(qubit)))
        .map_or(std::ptr::null_mut(), |cs| cs.into_raw())
}

/// Creates a physical qubit identifier from its numeric ID.
#[unsafe(no_mangle)]
pub extern "C" fn physical_qubit_new(id: u32) -> CPhysicalQubit {
    CPhysicalQubit { id }
}

/// Wraps a circuit qubit as a physical hardware ID.
#[unsafe(no_mangle)]
pub extern "C" fn physical_qubit_from_qubit(qubit: CQubit) -> CPhysicalQubit {
    CPhysicalQubit {
        id: PhysicalQubit::from_qubit(qubit.into()).id(),
    }
}

/// Returns the underlying circuit qubit identifier.
#[unsafe(no_mangle)]
pub extern "C" fn physical_qubit_qubit(qubit: CPhysicalQubit) -> CQubit {
    CQubit::from(PhysicalQubit::from(qubit).qubit())
}

/// Returns the numeric identifier of the physical qubit.
#[unsafe(no_mangle)]
pub extern "C" fn physical_qubit_id(qubit: CPhysicalQubit) -> u32 {
    PhysicalQubit::from(qubit).id()
}

/// Returns 1 when both physical qubits carry the same numeric identifier,
/// else 0.
#[unsafe(no_mangle)]
pub extern "C" fn physical_qubit_equal(a: CPhysicalQubit, b: CPhysicalQubit) -> i32 {
    PhysicalQubit::from(a).eq(&PhysicalQubit::from(b)) as i32
}

/// Compares two physical qubits by numeric identifier: -1 when `a < b`, 0
/// when equal, 1 when `a > b`.
#[unsafe(no_mangle)]
pub extern "C" fn physical_qubit_compare(a: CPhysicalQubit, b: CPhysicalQubit) -> i32 {
    match PhysicalQubit::from(a).cmp(&PhysicalQubit::from(b)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Formats the physical qubit identifier as `P<id>` (e.g. `P11`); free with
/// `cqlib_string_free`. Returns NULL on allocation failure.
#[unsafe(no_mangle)]
pub extern "C" fn physical_qubit_to_string(qubit: CPhysicalQubit) -> *mut c_char {
    CString::new(format!("{}", PhysicalQubit::from(qubit)))
        .map_or(std::ptr::null_mut(), |cs| cs.into_raw())
}
