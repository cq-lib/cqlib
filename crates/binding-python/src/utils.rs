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

use pyo3::prelude::*;
use pyo3::types::PyString;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub(crate) fn hash_value<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn python_string_literal(value: &str) -> String {
    Python::attach(|py| {
        PyString::new(py, value)
            .repr()
            .expect("str.__repr__ must succeed")
            .to_string_lossy()
            .into_owned()
    })
}

/// Formats a bool as a Python `True`/`False` literal for use in reprs.
pub(crate) fn python_bool(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}
