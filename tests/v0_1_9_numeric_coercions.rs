// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Regression Suite `v0.1.9` — Phase 1.9: Numeric Intrinsics & Primitive Coercions (`App24`)
//!
//! Validates:
//! - Signed and Absolute value intrinsics (`__vbaI2Abs`, `__vbaI4Abs`, `__vbaCyAbs`,
//!   `__vbaI2Sgn`, `__vbaI4Sgn`, `__vbaUI1Sgn`, `__vbaCySgn`, `__vbaR4Sgn`, `__vbaR8Sgn`)
//! - Currency arithmetic and conversions (`__vbaCyFix`, `__vbaCyInt`, `__vbaCySub`,
//!   `__vbaCyI2`, `__vbaCyI4`, `__vbaCyUI1`, `__vbaI2Cy`, `__vbaI4Cy`, `__vbaR4Cy`,
//!   `__vbaUI1Cy`, `__vbaFpCmpCy`)
//! - Floating-point widening, narrowing, power, and date conversions (`__vbaDateR4`,
//!   `__vbaDateR8`, `__vbaFpCDblR4`, `__vbaFpCDblR8`, `__vbaFpCSngR4`, `__vbaFpCSngR8`,
//!   `__vbaFpUI1`, `__vbaPowerR8`, `__vbaR8FixI2`, `__vbaR8FixI4`, `__vbaR8IntI2`,
//!   `__vbaR8IntI4`)
//! - Primitive and Error Variant conversions (`__vbaBoolErrVar`, `__vbaCyErrVar`,
//!   `__vbaI2ErrVar`, `__vbaR4ErrVar`, `__vbaUI1ErrVar`, `__vbaUI1I4`, `__vbaUI1Var`,
//!   `__vbaVarDateVar`)

use msvbvm60::types::*;
use msvbvm60::vba::*;

#[test]
fn test_signed_and_absolute_value_intrinsics() {
    unsafe {
        // Abs
        assert_eq!(__vbaI2Abs(-1234), 1234);
        assert_eq!(__vbaI2Abs(1234), 1234);
        assert_eq!(__vbaI4Abs(-987654321), 987654321);
        assert_eq!(__vbaI4Abs(987654321), 987654321);
        assert_eq!(__vbaCyAbs(-12345678), 12345678);
        assert_eq!(__vbaCyAbs(12345678), 12345678);

        // Sgn
        assert_eq!(__vbaI2Sgn(-42), -1);
        assert_eq!(__vbaI2Sgn(0), 0);
        assert_eq!(__vbaI2Sgn(42), 1);

        assert_eq!(__vbaI4Sgn(-100000), -1);
        assert_eq!(__vbaI4Sgn(0), 0);
        assert_eq!(__vbaI4Sgn(100000), 1);

        assert_eq!(__vbaUI1Sgn(15), 1);
        assert_eq!(__vbaUI1Sgn(0), 0);

        assert_eq!(__vbaCySgn(-502500), -1);
        assert_eq!(__vbaCySgn(0), 0);
        assert_eq!(__vbaCySgn(502500), 1);

        assert_eq!(__vbaR4Sgn(-3.14), -1);
        assert_eq!(__vbaR4Sgn(0.0), 0);
        assert_eq!(__vbaR4Sgn(3.14), 1);

        assert_eq!(__vbaR8Sgn(-2.71828), -1);
        assert_eq!(__vbaR8Sgn(0.0), 0);
        assert_eq!(__vbaR8Sgn(2.71828), 1);
    }
}

#[test]
fn test_currency_math_and_conversions() {
    unsafe {
        // CyFix & CyInt
        assert_eq!(__vbaCyFix(-1234567), -1230000);
        assert_eq!(__vbaCyInt(-1234567), -1240000);
        assert_eq!(__vbaCyFix(1234567), 1230000);
        assert_eq!(__vbaCyInt(1234567), 1230000);

        // CySub
        assert_eq!(__vbaCySub(5005000, 1252500), 3752500);

        // Coercions to Currency
        assert_eq!(__vbaCyI2(1234), 12340000);
        assert_eq!(__vbaCyI4(987654), 9876540000);
        assert_eq!(__vbaCyUI1(250), 2500000);

        // Coercions from Currency
        assert_eq!(__vbaI2Cy(45678900), 4568);
        assert_eq!(__vbaI4Cy(45678900), 4568);
        assert!((__vbaR4Cy(45678900) - 4567.89).abs() < 0.01);
        assert_eq!(__vbaUI1Cy(2004000), 200);

        // Currency Comparisons
        assert_eq!(__vbaFpCmpCy(1005000, 1005000), 0);
        assert_eq!(__vbaFpCmpCy(1005000, 2007500), -1);
        assert_eq!(__vbaFpCmpCy(2007500, 1005000), 1);
    }
}

#[test]
fn test_floating_point_and_date_conversions() {
    unsafe {
        assert_eq!(__vbaDateR8(36526.5), 36526.5);
        assert_eq!(__vbaDateR4(36526.5), 36526.5);

        assert!((__vbaFpCDblR4(12.34) - 12.34).abs() < 0.01);
        assert_eq!(__vbaFpCDblR8(12345.6789), 12345.6789);
        assert!((__vbaFpCSngR4(12.34) - 12.34).abs() < 0.001);
        assert!((__vbaFpCSngR8(987.654321) - 987.654).abs() < 0.01);
        assert_eq!(__vbaFpUI1(254.6), 255);

        assert_eq!(__vbaPowerR8(2.0, 10.0), 1024.0);
        assert_eq!(__vbaPowerR8(9.0, 0.5), 3.0);

        assert_eq!(__vbaR8FixI2(-15.75), -15);
        assert_eq!(__vbaR8IntI2(-15.75), -16);
        assert_eq!(__vbaR8FixI4(-123456.78), -123456);
        assert_eq!(__vbaR8IntI4(-123456.78), -123457);
    }
}

#[test]
fn test_error_variant_and_type_coercions() {
    unsafe {
        assert_eq!(__vbaUI1I4(127), 127);
        let v210 = Variant::from_i32(210);
        assert_eq!(__vbaUI1Var(&v210), 210);

        let b_dt = sys_alloc_string("2026-10-17 14:00:00");
        let v_dt_str = Variant::from_bstr(b_dt);
        let mut v_date_res = Variant::empty();
        __vbaVarDateVar(&mut v_date_res, &v_dt_str);
        assert_eq!(v_date_res.vt, VT_DATE);
        SysFreeString(b_dt);

        let v_err = Variant::cverr(5);
        __vbaBoolErrVar(&v_err);
        __vbaCyErrVar(&v_err);
        __vbaI2ErrVar(&v_err);
        __vbaR4ErrVar(&v_err);
        __vbaUI1ErrVar(&v_err);
    }
}
