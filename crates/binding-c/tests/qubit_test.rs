//! Integration tests for the qubit identifier C ABI.

use binding_c::circuit::qubit::{
    CQubit, qubit_compare, qubit_equal, qubit_id, qubit_index, qubit_new, qubit_to_string,
    qubit_try_from_i64, qubit_try_from_u64,
};
use binding_c::cqlib_string_free;
use binding_c::device::qubit::{
    logical_qubit_compare, logical_qubit_equal, logical_qubit_from_qubit, logical_qubit_id,
    logical_qubit_new, logical_qubit_qubit, logical_qubit_to_string, physical_qubit_compare,
    physical_qubit_equal, physical_qubit_from_qubit, physical_qubit_id, physical_qubit_new,
    physical_qubit_qubit, physical_qubit_to_string,
};
use std::ffi::CStr;

#[test]
fn test_qubit_basics() {
    let q = qubit_new(12);
    assert_eq!(q.id, 12);
    assert_eq!(qubit_id(q), 12);
    assert_eq!(qubit_index(q), 12usize);

    assert_eq!(qubit_equal(q, qubit_new(12)), 1);
    assert_eq!(qubit_equal(q, qubit_new(11)), 0);

    assert_eq!(qubit_compare(qubit_new(0), qubit_new(1)), -1);
    assert_eq!(qubit_compare(qubit_new(7), qubit_new(7)), 0);
    assert_eq!(qubit_compare(qubit_new(2), qubit_new(1)), 1);

    let s = qubit_to_string(q);
    assert!(!s.is_null());
    let text = unsafe { CStr::from_ptr(s) };
    assert_eq!(text.to_bytes(), b"Q12");
    cqlib_string_free(s);
}

#[test]
fn test_qubit_checked_conversions() {
    let mut out = CQubit { id: 0 };

    assert_eq!(qubit_try_from_i64(-1, &mut out), -8);
    assert_eq!(qubit_try_from_i64(i64::from(u32::MAX) + 1, &mut out), -8);
    assert_eq!(qubit_try_from_i64(3, &mut out), 0);
    assert_eq!(out.id, 3);

    assert_eq!(qubit_try_from_u64(u64::from(u32::MAX) + 1, &mut out), -8);
    assert_eq!(qubit_try_from_u64(20, &mut out), 0);
    assert_eq!(out.id, 20);

    assert_eq!(qubit_try_from_i64(0, std::ptr::null_mut()), -1);
    assert_eq!(qubit_try_from_u64(0, std::ptr::null_mut()), -1);
}

#[test]
fn test_logical_qubit_roundtrip() {
    let wire = qubit_new(3);
    let logical = logical_qubit_from_qubit(wire);
    assert_eq!(logical.id, 3);
    assert_eq!(logical_qubit_id(logical), 3);

    let back = logical_qubit_qubit(logical);
    assert_eq!(back.id, 3);

    assert_eq!(logical_qubit_equal(logical, logical_qubit_new(3)), 1);
    assert_eq!(logical_qubit_equal(logical, logical_qubit_new(2)), 0);
    assert_eq!(
        logical_qubit_compare(logical_qubit_new(2), logical_qubit_new(0)),
        1
    );

    let s = logical_qubit_to_string(logical);
    let text = unsafe { CStr::from_ptr(s) };
    assert_eq!(text.to_bytes(), b"L3");
    cqlib_string_free(s);
}

#[test]
fn test_physical_qubit_roundtrip() {
    let wire = qubit_new(11);
    let physical = physical_qubit_from_qubit(wire);
    assert_eq!(physical.id, 11);
    assert_eq!(physical_qubit_id(physical), 11);

    let back = physical_qubit_qubit(physical);
    assert_eq!(back.id, 11);

    assert_eq!(physical_qubit_equal(physical, physical_qubit_new(11)), 1);
    assert_eq!(physical_qubit_equal(physical, physical_qubit_new(10)), 0);
    assert_eq!(
        physical_qubit_compare(physical_qubit_new(0), physical_qubit_new(2)),
        -1
    );

    let s = physical_qubit_to_string(physical);
    let text = unsafe { CStr::from_ptr(s) };
    assert_eq!(text.to_bytes(), b"P11");
    cqlib_string_free(s);
}
