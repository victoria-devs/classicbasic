//! Math Intrinsics, CRT Floating Point helpers, and Pentium FDIV workarounds.
//!
//! Visual Basic 6 executables compiled with MSVC backends include calls to
//! MSVC CRT math intrinsics (`_CI*`) and the historic Intel Pentium FDIV bug
//! software workaround routines (`_adj_fdiv_*`).

// ----------------------------------------------------------------------------
// CRT Math Intrinsics (x87 FPU Register Operations)
// ----------------------------------------------------------------------------

/// CRT intrinsic computing the arc tangent of the value in the x87 FPU top-of-stack register `st(0)`.
#[no_mangle]
pub unsafe extern "C" fn _CIatan() {}

/// CRT intrinsic computing the cosine of the value in the x87 FPU top-of-stack register `st(0)`.
#[no_mangle]
pub unsafe extern "C" fn _CIcos() {}

/// CRT intrinsic computing the natural exponential (`e^x`) of the value in `st(0)`.
#[no_mangle]
pub unsafe extern "C" fn _CIexp() {}

/// CRT intrinsic computing the natural logarithm (`ln(x)`) of the value in `st(0)`.
#[no_mangle]
pub unsafe extern "C" fn _CIlog() {}

/// CRT intrinsic computing the sine of the value in the x87 FPU top-of-stack register `st(0)`.
#[no_mangle]
pub unsafe extern "C" fn _CIsin() {}

/// CRT intrinsic computing the square root of the value in the x87 FPU top-of-stack register `st(0)`.
#[no_mangle]
pub unsafe extern "C" fn _CIsqrt() {}

/// CRT intrinsic computing the tangent of the value in the x87 FPU top-of-stack register `st(0)`.
#[no_mangle]
pub unsafe extern "C" fn _CItan() {}

// ----------------------------------------------------------------------------
// 64-bit Integer Multiplication Runtime Helper
// ----------------------------------------------------------------------------

/// 64-bit integer multiplication runtime helper emitted by MSVC backends for 32-bit x86 targets.
///
/// Multiplies two 64-bit integers with wrapping semantics.
///
/// # Arguments
/// * `a` - First 64-bit operand.
/// * `b` - Second 64-bit operand.
///
/// # Returns
/// The 64-bit result of `a * b` with wrapping on overflow.
#[no_mangle]
pub unsafe extern "C" fn _allmul(a: i64, b: i64) -> i64 {
    a.wrapping_mul(b)
}

// ----------------------------------------------------------------------------
// Pentium FDIV Bug Workarounds (_adj_*)
// ----------------------------------------------------------------------------

/// Workaround helper for Intel Pentium FDIV bug: 16-bit integer divisor memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdiv_m16i() {}

/// Workaround helper for Intel Pentium FDIV bug: 32-bit single-precision float divisor memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdiv_m32() {}

/// Workaround helper for Intel Pentium FDIV bug: 32-bit integer divisor memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdiv_m32i() {}

/// Workaround helper for Intel Pentium FDIV bug: 64-bit double-precision float divisor memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdiv_m64() {}

/// Workaround helper for Intel Pentium FDIV bug: floating-point division with register operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdiv_r() {}

/// Workaround helper for Intel Pentium FDIV bug: reverse division with 16-bit integer memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdivr_m16i() {}

/// Workaround helper for Intel Pentium FDIV bug: reverse division with 32-bit float memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdivr_m32() {}

/// Workaround helper for Intel Pentium FDIV bug: reverse division with 32-bit integer memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdivr_m32i() {}

/// Workaround helper for Intel Pentium FDIV bug: reverse division with 64-bit double memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdivr_m64() {}

/// Workaround helper for Intel Pentium FDIV bug: partial arc tangent operation.
#[no_mangle]
pub unsafe extern "C" fn _adj_fpatan() {}

/// Workaround helper for Intel Pentium FDIV bug: partial remainder operation.
#[no_mangle]
pub unsafe extern "C" fn _adj_fprem() {}

/// Workaround helper for Intel Pentium FDIV bug: IEEE partial remainder operation.
#[no_mangle]
pub unsafe extern "C" fn _adj_fprem1() {}

/// Workaround helper for Intel Pentium FDIV bug: partial tangent operation.
#[no_mangle]
pub unsafe extern "C" fn _adj_fptan() {}
