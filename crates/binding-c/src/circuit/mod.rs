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

//! Minimal C ABI for circuit construction and symbolic parameters.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod gates_single;
pub mod gates_three;
pub mod gates_two;
pub mod instructions;
pub mod lifecycle;
pub mod parameter;
pub mod properties;

use cqlib_core::circuit::{Circuit, Parameter, ParameterValue, Qubit};
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;

pub struct CCircuit {
    pub inner: Circuit,
}

pub struct CParameter {
    pub inner: Parameter,
}

pub(crate) fn parse_bindings(bindings: *const c_char) -> Option<HashMap<String, f64>> {
    if bindings.is_null() {
        return None;
    }

    let c_str = unsafe { CStr::from_ptr(bindings) };
    let text = c_str.to_str().ok()?;
    let mut map = HashMap::new();
    for pair in text.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let (name, value) = pair.split_once(':')?;
        let value = value.trim().parse::<f64>().ok()?;
        if !value.is_finite() {
            return None;
        }
        map.insert(name.trim().to_string(), value);
    }
    Some(map)
}

pub(crate) fn binding_refs(bindings: &HashMap<String, f64>) -> HashMap<&str, f64> {
    bindings
        .iter()
        .map(|(name, value)| (name.as_str(), *value))
        .collect()
}

pub(crate) fn check_qubit(circuit: &Circuit, qubit: u32) -> Result<Qubit, i32> {
    if qubit as usize >= circuit.num_qubits() {
        return Err(-2);
    }
    Ok(Qubit::new(qubit))
}

pub(crate) fn apply_single(
    ptr: *mut CCircuit,
    qubit: u32,
    apply: impl FnOnce(&mut Circuit, Qubit) -> Result<(), cqlib_core::circuit::CircuitError>,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let qubit = match check_qubit(&wrapper.inner, qubit) {
        Ok(qubit) => qubit,
        Err(code) => return code,
    };
    apply(&mut wrapper.inner, qubit).map_or(-3, |_| 0)
}

pub(crate) fn apply_two(
    ptr: *mut CCircuit,
    first: u32,
    second: u32,
    apply: impl FnOnce(&mut Circuit, Qubit, Qubit) -> Result<(), cqlib_core::circuit::CircuitError>,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let first = match check_qubit(&wrapper.inner, first) {
        Ok(qubit) => qubit,
        Err(code) => return code,
    };
    let second = match check_qubit(&wrapper.inner, second) {
        Ok(qubit) => qubit,
        Err(code) => return code,
    };
    apply(&mut wrapper.inner, first, second).map_or(-3, |_| 0)
}

pub(crate) fn apply_param(
    ptr: *mut CCircuit,
    qubit: u32,
    param_ptr: *const CParameter,
    apply: impl FnOnce(
        &mut Circuit,
        Qubit,
        ParameterValue,
    ) -> Result<(), cqlib_core::circuit::CircuitError>,
) -> i32 {
    if ptr.is_null() || param_ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let qubit = match check_qubit(&wrapper.inner, qubit) {
        Ok(qubit) => qubit,
        Err(code) => return code,
    };
    let param = unsafe { &(*param_ptr).inner };
    apply(
        &mut wrapper.inner,
        qubit,
        ParameterValue::Param(param.clone()),
    )
    .map_or(-3, |_| 0)
}

/// Apply a single-qubit gate with two symbolic parameters (e.g. `rxy`).
pub(crate) fn apply_two_params(
    ptr: *mut CCircuit,
    qubit: u32,
    p1: *const CParameter,
    p2: *const CParameter,
    apply: impl FnOnce(
        &mut Circuit,
        Qubit,
        ParameterValue,
        ParameterValue,
    ) -> Result<(), cqlib_core::circuit::CircuitError>,
) -> i32 {
    if ptr.is_null() || p1.is_null() || p2.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let qubit = match check_qubit(&wrapper.inner, qubit) {
        Ok(qubit) => qubit,
        Err(code) => return code,
    };
    let p1 = unsafe { &(*p1).inner };
    let p2 = unsafe { &(*p2).inner };
    apply(
        &mut wrapper.inner,
        qubit,
        ParameterValue::Param(p1.clone()),
        ParameterValue::Param(p2.clone()),
    )
    .map_or(-3, |_| 0)
}

/// Apply a single-qubit gate with three symbolic parameters (e.g. `u`).
pub(crate) fn apply_three_params(
    ptr: *mut CCircuit,
    qubit: u32,
    p1: *const CParameter,
    p2: *const CParameter,
    p3: *const CParameter,
    apply: impl FnOnce(
        &mut Circuit,
        Qubit,
        ParameterValue,
        ParameterValue,
        ParameterValue,
    ) -> Result<(), cqlib_core::circuit::CircuitError>,
) -> i32 {
    if ptr.is_null() || p1.is_null() || p2.is_null() || p3.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let qubit = match check_qubit(&wrapper.inner, qubit) {
        Ok(qubit) => qubit,
        Err(code) => return code,
    };
    let p1 = unsafe { &(*p1).inner };
    let p2 = unsafe { &(*p2).inner };
    let p3 = unsafe { &(*p3).inner };
    apply(
        &mut wrapper.inner,
        qubit,
        ParameterValue::Param(p1.clone()),
        ParameterValue::Param(p2.clone()),
        ParameterValue::Param(p3.clone()),
    )
    .map_or(-3, |_| 0)
}

/// Apply a two-qubit gate with one symbolic parameter (e.g. `rxx_param`).
pub(crate) fn apply_two_qubit_param(
    ptr: *mut CCircuit,
    first: u32,
    second: u32,
    param_ptr: *const CParameter,
    apply: impl FnOnce(
        &mut Circuit,
        Qubit,
        Qubit,
        ParameterValue,
    ) -> Result<(), cqlib_core::circuit::CircuitError>,
) -> i32 {
    if ptr.is_null() || param_ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let first = match check_qubit(&wrapper.inner, first) {
        Ok(qubit) => qubit,
        Err(code) => return code,
    };
    let second = match check_qubit(&wrapper.inner, second) {
        Ok(qubit) => qubit,
        Err(code) => return code,
    };
    let param = unsafe { &(*param_ptr).inner };
    apply(
        &mut wrapper.inner,
        first,
        second,
        ParameterValue::Param(param.clone()),
    )
    .map_or(-3, |_| 0)
}

/// Apply a two-qubit gate with two symbolic parameters (e.g. `fsim_param`).
pub(crate) fn apply_two_qubit_two_params(
    ptr: *mut CCircuit,
    first: u32,
    second: u32,
    p1: *const CParameter,
    p2: *const CParameter,
    apply: impl FnOnce(
        &mut Circuit,
        Qubit,
        Qubit,
        ParameterValue,
        ParameterValue,
    ) -> Result<(), cqlib_core::circuit::CircuitError>,
) -> i32 {
    if ptr.is_null() || p1.is_null() || p2.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let first = match check_qubit(&wrapper.inner, first) {
        Ok(qubit) => qubit,
        Err(code) => return code,
    };
    let second = match check_qubit(&wrapper.inner, second) {
        Ok(qubit) => qubit,
        Err(code) => return code,
    };
    let p1 = unsafe { &(*p1).inner };
    let p2 = unsafe { &(*p2).inner };
    apply(
        &mut wrapper.inner,
        first,
        second,
        ParameterValue::Param(p1.clone()),
        ParameterValue::Param(p2.clone()),
    )
    .map_or(-3, |_| 0)
}

/// Apply a three-qubit gate with no parameters (e.g. `ccx`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_three(
    ptr: *mut CCircuit,
    first: u32,
    second: u32,
    third: u32,
    apply: impl FnOnce(
        &mut Circuit,
        Qubit,
        Qubit,
        Qubit,
    ) -> Result<(), cqlib_core::circuit::CircuitError>,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let wrapper = unsafe { &mut *ptr };
    let first = match check_qubit(&wrapper.inner, first) {
        Ok(qubit) => qubit,
        Err(code) => return code,
    };
    let second = match check_qubit(&wrapper.inner, second) {
        Ok(qubit) => qubit,
        Err(code) => return code,
    };
    let third = match check_qubit(&wrapper.inner, third) {
        Ok(qubit) => qubit,
        Err(code) => return code,
    };
    apply(&mut wrapper.inner, first, second, third).map_or(-3, |_| 0)
}

pub use gates_single::*;
pub use gates_three::*;
pub use gates_two::*;
pub use instructions::*;
pub use lifecycle::*;
pub use parameter::*;
pub use properties::*;
