// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Phase 1.9: Core Language — Numeric Intrinsics & Primitive Coercions
//!
//! Implements 40 typed numeric math, Abs/Sgn, Currency coercion, and floating-point/Date
//! conversion compiler helpers for `msvbvm60.dll`.

use crate::err::set_vb_error;
use crate::types::Variant;
use crate::vba::__vbaErrorOverflow;

// ---------------------------------------------------------------------------
// 1. Signed and Absolute Value Helpers
// ---------------------------------------------------------------------------

/// Computes the absolute value of a 16-bit signed integer (`__vbaI2Abs`, Ordinal 133).
#[no_mangle]
pub unsafe extern "system" fn __vbaI2Abs(val: i16) -> i16 {
    if val == i16::MIN {
        __vbaErrorOverflow();
    }
    val.abs()
}

/// Computes the absolute value of a 32-bit signed integer (`__vbaI4Abs`, Ordinal 136).
#[no_mangle]
pub unsafe extern "system" fn __vbaI4Abs(val: i32) -> i32 {
    if val == i32::MIN {
        __vbaErrorOverflow();
    }
    val.abs()
}

/// Computes the absolute value of a 64-bit Currency fixed-point integer (`__vbaCyAbs`, Ordinal 185).
#[no_mangle]
pub unsafe extern "system" fn __vbaCyAbs(val: i64) -> i64 {
    if val == i64::MIN {
        __vbaErrorOverflow();
    }
    val.abs()
}

/// Computes the signum of a 16-bit integer (`__vbaI2Sgn`, Ordinal 135).
#[no_mangle]
pub unsafe extern "system" fn __vbaI2Sgn(val: i16) -> i16 {
    match val.cmp(&0) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Computes the signum of a 32-bit integer (`__vbaI4Sgn`, Ordinal 137).
#[no_mangle]
pub unsafe extern "system" fn __vbaI4Sgn(val: i32) -> i16 {
    match val.cmp(&0) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Computes the signum of an 8-bit unsigned integer (`__vbaUI1Sgn`, Ordinal 142).
#[no_mangle]
pub unsafe extern "system" fn __vbaUI1Sgn(val: u8) -> i16 {
    if val == 0 {
        0
    } else {
        1
    }
}

/// Computes the signum of a 64-bit Currency value (`__vbaCySgn`, Ordinal 201).
#[no_mangle]
pub unsafe extern "system" fn __vbaCySgn(val: i64) -> i16 {
    match val.cmp(&0) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Computes the signum of a 32-bit float (`__vbaR4Sgn`, Ordinal 366).
#[no_mangle]
pub unsafe extern "system" fn __vbaR4Sgn(val: f32) -> i16 {
    if val < 0.0 {
        -1
    } else if val > 0.0 {
        1
    } else {
        0
    }
}

/// Computes the signum of a 64-bit float (`__vbaR8Sgn`, Ordinal 376).
#[no_mangle]
pub unsafe extern "system" fn __vbaR8Sgn(val: f64) -> i16 {
    if val < 0.0 {
        -1
    } else if val > 0.0 {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// 2. Currency Math and Coercion Helpers
// ---------------------------------------------------------------------------

/// Truncates a Currency value towards zero (`__vbaCyFix`, Ordinal 191).
#[no_mangle]
pub unsafe extern "system" fn __vbaCyFix(val: i64) -> i64 {
    (val / 10_000) * 10_000
}

/// Floors a Currency value towards negative infinity (`__vbaCyInt`, Ordinal 197).
#[no_mangle]
pub unsafe extern "system" fn __vbaCyInt(val: i64) -> i64 {
    if val < 0 && val % 10_000 != 0 {
        ((val / 10_000) - 1) * 10_000
    } else {
        (val / 10_000) * 10_000
    }
}

/// Subtracts two Currency values with overflow checking (`__vbaCySub`, Ordinal 203).
#[no_mangle]
pub unsafe extern "system" fn __vbaCySub(left: i64, right: i64) -> i64 {
    match left.checked_sub(right) {
        Some(res) => res,
        None => {
            __vbaErrorOverflow();
            0
        }
    }
}

/// Converts a 16-bit integer to Currency (`__vbaCyI2`, Ordinal 194).
#[no_mangle]
pub unsafe extern "system" fn __vbaCyI2(val: i16) -> i64 {
    (val as i64) * 10_000
}

/// Converts a 32-bit integer to Currency (`__vbaCyI4`, Ordinal 196).
#[no_mangle]
pub unsafe extern "system" fn __vbaCyI4(val: i32) -> i64 {
    match (val as i64).checked_mul(10_000) {
        Some(res) => res,
        None => {
            __vbaErrorOverflow();
            0
        }
    }
}

/// Converts an 8-bit unsigned integer to Currency (`__vbaCyUI1`, Ordinal 204).
#[no_mangle]
pub unsafe extern "system" fn __vbaCyUI1(val: u8) -> i64 {
    (val as i64) * 10_000
}

/// Helper for Banker's Rounding of a Currency value to an integer.
fn round_currency_to_i64(val: i64) -> i64 {
    let quot = val / 10_000;
    let rem = val % 10_000;
    let rem_abs = rem.abs();

    if rem_abs > 5_000 {
        if val > 0 {
            quot + 1
        } else {
            quot - 1
        }
    } else if rem_abs == 5_000 {
        // Round to even
        if quot % 2 != 0 {
            if val > 0 {
                quot + 1
            } else {
                quot - 1
            }
        } else {
            quot
        }
    } else {
        quot
    }
}

/// Converts Currency to a 16-bit integer (`__vbaI2Cy`, Ordinal 259).
#[no_mangle]
pub unsafe extern "system" fn __vbaI2Cy(val: i64) -> i16 {
    let rounded = round_currency_to_i64(val);
    if !(i16::MIN as i64..=i16::MAX as i64).contains(&rounded) {
        __vbaErrorOverflow();
    }
    rounded as i16
}

/// Converts Currency to a 32-bit integer (`__vbaI4Cy`, Ordinal 264).
#[no_mangle]
pub unsafe extern "system" fn __vbaI4Cy(val: i64) -> i32 {
    let rounded = round_currency_to_i64(val);
    if !(i32::MIN as i64..=i32::MAX as i64).contains(&rounded) {
        __vbaErrorOverflow();
    }
    rounded as i32
}

/// Converts Currency to a 32-bit float (`__vbaR4Cy`, Ordinal 363).
#[no_mangle]
pub unsafe extern "system" fn __vbaR4Cy(val: i64) -> f32 {
    (val as f64 / 10_000.0) as f32
}

/// Converts Currency to an 8-bit unsigned integer (`__vbaUI1Cy`, Ordinal 439).
#[no_mangle]
pub unsafe extern "system" fn __vbaUI1Cy(val: i64) -> u8 {
    let rounded = round_currency_to_i64(val);
    if !(0..=255).contains(&rounded) {
        __vbaErrorOverflow();
    }
    rounded as u8
}

/// Compares two Currency values (`__vbaFpCmpCy`, Ordinal 122).
/// Returns -1 if left < right, 0 if left == right, 1 if left > right.
#[no_mangle]
pub unsafe extern "system" fn __vbaFpCmpCy(left: i64, right: i64) -> i32 {
    match left.cmp(&right) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

// ---------------------------------------------------------------------------
// 3. Floating Point & Date Conversions
// ---------------------------------------------------------------------------

/// Converts a Date (stored as f64) to a 32-bit float (`__vbaDateR4`, Ordinal 206).
#[no_mangle]
pub unsafe extern "system" fn __vbaDateR4(val: f64) -> f32 {
    val as f32
}

/// Converts a Date to a 64-bit float (`__vbaDateR8`, Ordinal 207).
#[no_mangle]
pub unsafe extern "system" fn __vbaDateR8(val: f64) -> f64 {
    val
}

/// Converts a 32-bit float to a 64-bit float (`__vbaFpCDblR4`, Ordinal 118).
#[no_mangle]
pub unsafe extern "system" fn __vbaFpCDblR4(val: f32) -> f64 {
    val as f64
}

/// Returns a 64-bit float unchanged (`__vbaFpCDblR8`, Ordinal 119).
#[no_mangle]
pub unsafe extern "system" fn __vbaFpCDblR8(val: f64) -> f64 {
    val
}

/// Returns a 32-bit float unchanged (`__vbaFpCSngR4`, Ordinal 120).
#[no_mangle]
pub unsafe extern "system" fn __vbaFpCSngR4(val: f32) -> f32 {
    val
}

/// Converts a 64-bit float to a 32-bit float (`__vbaFpCSngR8`, Ordinal 121).
#[no_mangle]
pub unsafe extern "system" fn __vbaFpCSngR8(val: f64) -> f32 {
    val as f32
}

/// Banker's rounds a 64-bit float to an 8-bit unsigned integer (`__vbaFpUI1`, Ordinal 128).
#[no_mangle]
pub unsafe extern "system" fn __vbaFpUI1(val: f64) -> u8 {
    let rounded = val.round();
    if !(0.0..=255.0).contains(&rounded) || val.is_nan() {
        __vbaErrorOverflow();
    }
    // Check half-way tie breaking to even
    let floor = val.floor();
    let diff = (val - floor).abs();
    if (diff - 0.5).abs() < 1e-9 {
        if (floor as i64) % 2 == 0 {
            floor as u8
        } else {
            (floor + 1.0) as u8
        }
    } else {
        rounded as u8
    }
}

/// Exponentiation of two 64-bit floats (`__vbaPowerR8`, Ordinal 354).
#[no_mangle]
pub unsafe extern "system" fn __vbaPowerR8(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

/// Fix (truncate towards zero) a 64-bit float to a 16-bit integer (`__vbaR8FixI2`, Ordinal 371).
#[no_mangle]
pub unsafe extern "system" fn __vbaR8FixI2(val: f64) -> i16 {
    let trunc = val.trunc();
    if !(i16::MIN as f64..=i16::MAX as f64).contains(&trunc) || val.is_nan() {
        __vbaErrorOverflow();
    }
    trunc as i16
}

/// Fix (truncate towards zero) a 64-bit float to a 32-bit integer (`__vbaR8FixI4`, Ordinal 372).
#[no_mangle]
pub unsafe extern "system" fn __vbaR8FixI4(val: f64) -> i32 {
    let trunc = val.trunc();
    if !(i32::MIN as f64..=i32::MAX as f64).contains(&trunc) || val.is_nan() {
        __vbaErrorOverflow();
    }
    trunc as i32
}

/// Int (floor towards negative infinity) a 64-bit float to a 16-bit integer (`__vbaR8IntI2`, Ordinal 374).
#[no_mangle]
pub unsafe extern "system" fn __vbaR8IntI2(val: f64) -> i16 {
    let floor = val.floor();
    if !(i16::MIN as f64..=i16::MAX as f64).contains(&floor) || val.is_nan() {
        __vbaErrorOverflow();
    }
    floor as i16
}

/// Int (floor towards negative infinity) a 64-bit float to a 32-bit integer (`__vbaR8IntI4`, Ordinal 375).
#[no_mangle]
pub unsafe extern "system" fn __vbaR8IntI4(val: f64) -> i32 {
    let floor = val.floor();
    if !(i32::MIN as f64..=i32::MAX as f64).contains(&floor) || val.is_nan() {
        __vbaErrorOverflow();
    }
    floor as i32
}

// ---------------------------------------------------------------------------
// 4. Primitive & Error Variant Coercions
// ---------------------------------------------------------------------------

/// Converts a 32-bit integer to an 8-bit unsigned integer with overflow checking (`__vbaUI1I4`, Ordinal 141).
#[no_mangle]
pub unsafe extern "system" fn __vbaUI1I4(val: i32) -> u8 {
    if !(0..=255).contains(&val) {
        __vbaErrorOverflow();
    }
    val as u8
}

/// Converts a Variant to an 8-bit unsigned integer (`__vbaUI1Var`, Ordinal 444).
#[no_mangle]
pub unsafe extern "system" fn __vbaUI1Var(val: *const Variant) -> u8 {
    if val.is_null() {
        return 0;
    }
    let v = &*val;
    if let Some(n) = v.as_i64() {
        if !(0..=255).contains(&n) {
            __vbaErrorOverflow();
        }
        n as u8
    } else if let Some(f) = v.as_f64() {
        __vbaFpUI1(f)
    } else {
        set_vb_error(13, None, None); // Type mismatch
        0
    }
}

/// Converts a Variant to a Date Variant (`__vbaVarDateVar`, Ordinal 459).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarDateVar(dst: *mut Variant, src: *const Variant) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if src.is_null() {
        *dst = Variant::empty();
        return dst;
    }
    let s = &*src;
    let dt_val = if let Some(d) = s.as_date() {
        d
    } else if let Some(f) = s.as_f64() {
        f
    } else if let Some(n) = s.as_i32() {
        n as f64
    } else {
        set_vb_error(13, None, None); // Type Mismatch
        0.0
    };
    *dst = Variant::date(dt_val);
    dst
}

/// Coerces an Error Variant to Boolean (`__vbaBoolErrVar`, Ordinal 109).
/// Throws Type Mismatch (Error 13) unless trapped by On Error Resume Next.
#[no_mangle]
pub unsafe extern "system" fn __vbaBoolErrVar(val: *const Variant) -> i16 {
    set_vb_error(13, None, None); // Type Mismatch
    let _ = val;
    0
}

/// Coerces an Error Variant to Currency (`__vbaCyErrVar`, Ordinal 189).
#[no_mangle]
pub unsafe extern "system" fn __vbaCyErrVar(val: *const Variant) -> i64 {
    set_vb_error(13, None, None); // Type Mismatch
    let _ = val;
    0
}

/// Coerces an Error Variant to a 16-bit integer (`__vbaI2ErrVar`, Ordinal 260).
#[no_mangle]
pub unsafe extern "system" fn __vbaI2ErrVar(val: *const Variant) -> i16 {
    set_vb_error(13, None, None); // Type Mismatch
    let _ = val;
    0
}

/// Coerces an Error Variant to a 32-bit float (`__vbaR4ErrVar`, Ordinal 364).
#[no_mangle]
pub unsafe extern "system" fn __vbaR4ErrVar(val: *const Variant) -> f32 {
    set_vb_error(13, None, None); // Type Mismatch
    let _ = val;
    0.0
}

/// Coerces an Error Variant to an 8-bit unsigned integer (`__vbaUI1ErrVar`, Ordinal 442).
#[no_mangle]
pub unsafe extern "system" fn __vbaUI1ErrVar(val: *const Variant) -> u8 {
    set_vb_error(13, None, None); // Type Mismatch
    let _ = val;
    0
}
