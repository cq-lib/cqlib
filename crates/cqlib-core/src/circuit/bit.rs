// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! # Qubit Module
//!
//! This module defines the fundamental addressing units for quantum circuits.
//!
//! ## Architecture Design
//!
//! This library adopts a **Handle Pattern** (also known as Entity-Component style) for qubit management:
//!
//! - **[`Qubit`]** acts as a lightweight, `Copy`-able logical identifier. It does not hold state
//!   or topological information itself.
//! - State and connectivity are managed by the parent `Circuit` or `DAG`.
//!
//! This design ensures strictly $O(1)$ cloning costs and cache-friendly memory layouts during
//! complex compilation passes.

use std::{fmt, hash::Hash};
use thiserror::Error;

/// Errors returned when converting an integer into a [`Qubit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum QubitError {
    /// The supplied signed integer was negative.
    #[error("qubit index must be non-negative, got {0}")]
    NegativeIndex(i128),
    /// The supplied integer cannot be represented by the internal `u32` ID.
    #[error("qubit index {0} exceeds u32::MAX")]
    IndexTooLarge(u128),
}

/// A lightweight logical qubit identifier.
///
/// Equality compares only the numeric identifier. A `Qubit` does not carry a circuit
/// identity, physical-device location, or state-vector position. The owning circuit and
/// the matrix or simulator qubit order determine those meanings.
///
/// It wraps a `u32` to provide a compact representation. Numeric IDs may be sparse,
/// so callers must not assume that [`Qubit::index`] is a valid position in a circuit's
/// dense qubit storage or a simulator state vector.
///
/// # Examples
///
/// Basic usage:
///
/// ```rust
/// use cqlib_core::circuit::bit::Qubit;
///
/// // Create a logical qubit with numeric identifier 5.
/// let q = Qubit::new(5);
///
/// assert_eq!(q.id(), 5);
/// assert_eq!(q.index(), 5usize);
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[repr(transparent)]
pub struct Qubit(u32);

impl fmt::Display for Qubit {
    /// Formats the qubit identifier.
    ///
    /// The output format is `Q<id>` (e.g., `Q0`, `Q12`).
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Q{}", self.0)
    }
}

impl Qubit {
    /// Creates a logical qubit with the supplied numeric identifier.
    ///
    /// This is the only way for external users to create a Qubit instance,
    /// as the internal field is private to enforce encapsulation.
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the raw underlying identifier as a `u32`.
    ///
    /// Use this when you need the compact 4-byte representation.
    #[inline]
    pub const fn id(&self) -> u32 {
        self.0
    }

    /// Returns the numeric identifier as a `usize`.
    ///
    /// This conversion does not map the qubit to its position in a particular
    /// circuit, matrix, or simulator. Such positions depend on the ordering
    /// maintained by the owning data structure.
    #[inline]
    pub const fn index(&self) -> usize {
        self.0 as usize
    }
}

macro_rules! impl_qubit_from_unsigned {
    ($($t:ty),*) => {
        $(
            impl From<$t> for Qubit {
                fn from(idx: $t) -> Self {
                    Self(idx as u32)
                }
            }
        )*
    };
}
macro_rules! impl_qubit_try_from_signed {
    ($($t:ty),*) => {
        $(
            impl TryFrom<$t> for Qubit {
                type Error = QubitError;

                fn try_from(idx: $t) -> Result<Self, Self::Error> {
                    if idx < 0 {
                        Err(QubitError::NegativeIndex(idx as i128))
                    } else if idx as u128 > u32::MAX as u128 {
                        Err(QubitError::IndexTooLarge(idx as u128))
                    } else {
                        Ok(Self(idx as u32))
                    }
                }
            }
        )*
    };
}

// Implement From for types that cannot overflow u32
impl_qubit_from_unsigned!(u32, u16, u8);
impl_qubit_try_from_signed!(i8, i16, i32, isize);

macro_rules! impl_qubit_try_from_unsigned {
    ($($t:ty),*) => {
        $(
            impl TryFrom<$t> for Qubit {
                type Error = QubitError;

                fn try_from(idx: $t) -> Result<Self, Self::Error> {
                    if idx as u128 > u32::MAX as u128 {
                        Err(QubitError::IndexTooLarge(idx as u128))
                    } else {
                        Ok(Self(idx as u32))
                    }
                }
            }
        )*
    };
}

impl_qubit_try_from_unsigned!(usize);

#[test]
fn test_qubit_creation_and_display() {
    let q0 = Qubit(0);
    let q1 = Qubit(1);

    assert_eq!(q0.id(), 0);
    assert_eq!(q1.id(), 1);
    assert_ne!(q0, q1);

    assert_eq!(format!("{}", q0), "Q0");
    assert_eq!(format!("{}", q1), "Q1");

    let q0_1 = Qubit(0);
    assert_eq!(q0, q0_1);
}

#[test]
fn test_qubit_try_from_usize() {
    // Valid indices should work
    assert!(Qubit::try_from(0usize).is_ok());
    assert!(Qubit::try_from(100usize).is_ok());
    if usize::BITS >= 32 {
        assert!(Qubit::try_from(u32::MAX as usize).is_ok());
    }

    // Overflow on 64-bit systems
    if usize::BITS > 32 {
        let overflow_idx = (u32::MAX as usize) + 1;
        let result = Qubit::try_from(overflow_idx);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            QubitError::IndexTooLarge(overflow_idx as u128)
        );
    }
}

#[test]
fn test_qubit_try_from_isize() {
    // Valid indices should work
    assert!(Qubit::try_from(0isize).is_ok());
    assert!(Qubit::try_from(100isize).is_ok());
    if isize::BITS > 32 {
        assert!(Qubit::try_from(u32::MAX as isize).is_ok());
    } else {
        assert!(Qubit::try_from(isize::MAX).is_ok());
    }

    // Negative index should fail
    let result = Qubit::try_from(-1isize);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), QubitError::NegativeIndex(-1));

    // Overflow on 64-bit systems
    if isize::BITS > 32 {
        let overflow_idx = (u32::MAX as isize) + 1;
        let result = Qubit::try_from(overflow_idx);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            QubitError::IndexTooLarge(overflow_idx as u128)
        );
    }
}

#[test]
fn test_qubit_from_small_types() {
    // These should all work via From (infallible)
    let _: Qubit = 0u8.into();
    let _: Qubit = 100u8.into();
    let _: Qubit = 0u16.into();
    let _: Qubit = 1000u16.into();
    let _: Qubit = 0u32.into();
    let _: Qubit = 100000u32.into();

    assert_eq!(Qubit::try_from(0i8), Ok(Qubit::new(0)));
    assert_eq!(Qubit::try_from(100i8), Ok(Qubit::new(100)));
    assert_eq!(Qubit::try_from(1000i16), Ok(Qubit::new(1000)));
    assert_eq!(Qubit::try_from(100000i32), Ok(Qubit::new(100000)));
    assert_eq!(Qubit::try_from(-1i8), Err(QubitError::NegativeIndex(-1)));
    assert_eq!(Qubit::try_from(-1i16), Err(QubitError::NegativeIndex(-1)));
    assert_eq!(Qubit::try_from(-1i32), Err(QubitError::NegativeIndex(-1)));
}
