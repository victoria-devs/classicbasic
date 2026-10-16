//! Layer 03: Math, Numeric Types, and Conversions Tests
//!
//! Granular tests for Currency (fixed-point), RGB color encoding,
//! floating-point helpers, and numeric-to-string formatters corresponding to App03-math-types.

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
        // In Windows COLORREF: RGB(red, green, blue) -> red | (green << 8) | (blue << 16)
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
