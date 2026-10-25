// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Types & Automation: Currency & OLE Date
//!
//! 64-bit 10,000 fixed-point Currency arithmetic, Banker's rounding,
//! IEEE float serial dates, and the 1899-12-30 epoch.

use msvbvm60::rtc::extended::{civil_ymd_to_serial_days, serial_days_to_civil_ymd};
use msvbvm60::types::currency::Currency;
use msvbvm60::types::*;
use msvbvm60::*;

#[test]
fn test_currency_from_f64_scaled_integer() {
    let cy = Currency::from_f64(123.4567);
    assert_eq!(cy.0, 1234567);
}

#[test]
fn test_currency_to_f64_roundtrip() {
    let cy = Currency::from_f64(9876.5432);
    let back = cy.to_f64();
    assert!((back - 9876.5432).abs() < 1e-4);
}

#[test]
fn test_rgb_macro_colorref_format() {
    unsafe {
        let color = rtcRgb(255, 0, 128);
        assert_eq!(color as u32, 0x008000FF);
    }
}

#[test]
fn test_vba_str_i2_conversion() {
    unsafe {
        let b = __vbaStrI2(1234);
        assert_eq!(bstr_to_string(b), "1234");
        SysFreeString(b);
    }
}

#[test]
fn test_vba_str_i4_conversion() {
    unsafe {
        let b = __vbaStrI4(987654321);
        assert_eq!(bstr_to_string(b), "987654321");
        SysFreeString(b);
    }
}

#[test]
fn test_vba_str_r4_conversion() {
    unsafe {
        let b = __vbaStrR4(42.5);
        assert_eq!(bstr_to_string(b), "42.5");
        SysFreeString(b);
    }
}

#[test]
fn test_vba_fp_int_truncation() {
    unsafe {
        assert_eq!(__vbaFPInt(42.9), 42.0);
        assert_eq!(__vbaFPInt(-42.9), -43.0);
    }
}

#[test]
fn test_vba_fp_r4_passthrough() {
    unsafe {
        assert_eq!(__vbaFpR4(10.5), 10.5);
    }
}

#[test]
fn test_currency_fixed_point_scaling_and_arithmetic_precision() {
    let cy_a = Currency::from_scaled(199900); // 19.99
    let cy_b = Currency::from_scaled(100); // 0.01

    let var_a = Variant::from_cy(cy_a);
    let var_b = Variant::from_cy(cy_b);

    let mut res = Variant::empty();
    unsafe {
        __vbaVarAdd(&mut res, &var_b, &var_a);
    }

    assert_eq!(res.vt, VT_CY);
    let cy_res = res.as_cy().expect("Result must be Currency");
    assert_eq!(
        cy_res.0, 200000,
        "Internal integer must equal exactly 200000 (20.0000)"
    );

    let str_repr = res.to_string_representation();
    assert_eq!(
        str_repr, "20",
        "Formatting to String must yield '20' without floating-point drift"
    );
}

#[test]
fn test_currency_intermediate_precision_and_bankers_rounding() {
    // Currency is 64-bit signed int scaled by 10,000.
    // E.g., $12.3456 = 123456 in raw integer form.
    let c1 = Currency::from_raw(123456); // 12.3456
    let c2 = Currency::from_raw(20000);  // 2.0000

    // Multiply: 12.3456 * 2.0000 = 24.6912
    let c_mul = c1.checked_mul(c2).expect("Multiplication overflow");
    assert_eq!(c_mul.as_raw(), 246912);

    // Exact tie banker's rounding test (Round to even):
    // 0.0025 -> 0.002 (even), 0.0035 -> 0.004 (even)
    let c_tie1 = Currency::from_raw(25); // 0.0025
    let c_tie2 = Currency::from_raw(35); // 0.0035
    assert_eq!(c_tie1.round_to_decimals(3).as_raw(), 20); // rounds down to 2
    assert_eq!(c_tie2.round_to_decimals(3).as_raw(), 40); // rounds up to 4

    // Extreme Boundary: Multiplication near 64-bit saturation
    let c_max = Currency::from_raw(i64::MAX);
    let c_two = Currency::from_raw(20000); // 2.0000
    assert!(c_max.checked_mul(c_two).is_none(), "Must detect Currency overflow");
}

#[test]
fn test_date_serial_epoch_and_negative_day_triangulation() {
    // Test 1: Day 0.0 = December 30, 1899
    let (y0, m0, d0) = serial_days_to_civil_ymd(0.0);
    assert_eq!((y0, m0, d0), (1899, 12, 30));

    // Test 2: Day 1.0 = December 31, 1899
    let (y1, m1, d1) = serial_days_to_civil_ymd(1.0);
    assert_eq!((y1, m1, d1), (1899, 12, 31));

    // Test 3: Day 2.0 = January 1, 1900
    let (y2, m2, d2) = serial_days_to_civil_ymd(2.0);
    assert_eq!((y2, m2, d2), (1900, 1, 1));

    // Test 4: Negative day: Day -1.0 = December 29, 1899
    let (yn1, mn1, dn1) = serial_days_to_civil_ymd(-1.0);
    assert_eq!((yn1, mn1, dn1), (1899, 12, 29));

    // Test 5: Modern Date: Day 45678.0 = 2025-01-21
    let modern_serial = civil_ymd_to_serial_days(2025, 1, 21);
    let (ym, mm, dm) = serial_days_to_civil_ymd(modern_serial);
    assert_eq!((ym, mm, dm), (2025, 1, 21));
}
