//! Math Intrinsics, CRT Floating Point helpers, and Pentium FDIV workarounds.
//!
//! Visual Basic 6 executables compiled with MSVC backends include calls to
//! MSVC CRT math intrinsics (`_CI*`) and the historic Intel Pentium FDIV bug
//! software workaround routines (`_adj_fdiv_*`).

// ----------------------------------------------------------------------------
// CRT Math Intrinsics
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn _CIatan() {}

#[no_mangle]
pub unsafe extern "C" fn _CIcos() {}

#[no_mangle]
pub unsafe extern "C" fn _CIexp() {}

#[no_mangle]
pub unsafe extern "C" fn _CIlog() {}

#[no_mangle]
pub unsafe extern "C" fn _CIsin() {}

#[no_mangle]
pub unsafe extern "C" fn _CIsqrt() {}

#[no_mangle]
pub unsafe extern "C" fn _CItan() {}

// ----------------------------------------------------------------------------
// 64-bit Integer Multiplication Runtime Helper
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn _allmul(a: i64, b: i64) -> i64 {
    a.wrapping_mul(b)
}

// ----------------------------------------------------------------------------
// Pentium FDIV Bug Workarounds (_adj_*)
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn _adj_fdiv_m16i() {}

#[no_mangle]
pub unsafe extern "C" fn _adj_fdiv_m32() {}

#[no_mangle]
pub unsafe extern "C" fn _adj_fdiv_m32i() {}

#[no_mangle]
pub unsafe extern "C" fn _adj_fdiv_m64() {}

#[no_mangle]
pub unsafe extern "C" fn _adj_fdiv_r() {}

#[no_mangle]
pub unsafe extern "C" fn _adj_fdivr_m16i() {}

#[no_mangle]
pub unsafe extern "C" fn _adj_fdivr_m32() {}

#[no_mangle]
pub unsafe extern "C" fn _adj_fdivr_m32i() {}

#[no_mangle]
pub unsafe extern "C" fn _adj_fdivr_m64() {}

#[no_mangle]
pub unsafe extern "C" fn _adj_fpatan() {}

#[no_mangle]
pub unsafe extern "C" fn _adj_fprem() {}

#[no_mangle]
pub unsafe extern "C" fn _adj_fprem1() {}

#[no_mangle]
pub unsafe extern "C" fn _adj_fptan() {}
