//! Visual Basic Architecture (`__vba*`) compiler runtime helpers.
//!
//! When Visual Basic 6 native-code compiler (C2.EXE) compiles functions,
//! it generates calls to `__vba*` helpers for string management, variant arithmetic,
//! array lifecycle, and SEH (Structured Exception Handling).

use core::ffi::c_void;
use crate::types::{Bstr, SafeArray, Variant, VT_I4};

// ----------------------------------------------------------------------------
// Low-level Memory Operations
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "system" fn __vbaCopyBytes(count: usize, dst: *mut u8, src: *const u8) {
    if !dst.is_null() && !src.is_null() && count > 0 {
        core::ptr::copy(src, dst, count);
    }
}

#[no_mangle]
pub unsafe extern "system" fn __vbaCopyBytesZero(count: usize, dst: *mut u8, src: *const u8) {
    if !dst.is_null() && count > 0 {
        if !src.is_null() {
            core::ptr::copy(src, dst, count);
        } else {
            core::ptr::write_bytes(dst, 0, count);
        }
    }
}

// ----------------------------------------------------------------------------
// String Helpers (BSTR Management)
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "system" fn __vbaStrCopy(dst: *mut Bstr, src: Bstr) -> Bstr {
    if !dst.is_null() {
        *dst = src;
    }
    src
}

#[no_mangle]
pub unsafe extern "system" fn __vbaStrMove(dst: *mut Bstr, src: *mut Bstr) -> Bstr {
    if !dst.is_null() && !src.is_null() {
        *dst = *src;
        *src = core::ptr::null_mut();
        return *dst;
    }
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "system" fn __vbaStrCat(dst: *mut Bstr, _src: Bstr) -> Bstr {
    if !dst.is_null() {
        *dst
    } else {
        core::ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "system" fn __vbaStrCmp(s1: Bstr, s2: Bstr) -> i32 {
    if s1 == s2 {
        return 0;
    }
    if s1.is_null() {
        return -1;
    }
    if s2.is_null() {
        return 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "system" fn __vbaFreeStr(s: *mut Bstr) {
    if !s.is_null() {
        *s = core::ptr::null_mut();
    }
}

#[no_mangle]
pub unsafe extern "system" fn __vbaFreeStrList(_count: i32) {
    // Variadic or list free of BSTRs
}

// ----------------------------------------------------------------------------
// Variant Helpers
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "system" fn __vbaVarMove(dst: *mut Variant, src: *const Variant) -> *mut Variant {
    if !dst.is_null() && !src.is_null() {
        *dst = *src;
    }
    dst
}

#[no_mangle]
pub unsafe extern "system" fn __vbaVarCopy(dst: *mut Variant, src: *const Variant) -> *mut Variant {
    if !dst.is_null() && !src.is_null() {
        *dst = *src;
    }
    dst
}

#[no_mangle]
pub unsafe extern "system" fn __vbaFreeVar(v: *mut Variant) {
    if !v.is_null() {
        *v = Variant::empty();
    }
}

#[no_mangle]
pub unsafe extern "system" fn __vbaFreeVarList(_count: i32) {
    // Variadic or list free of Variants
}

#[no_mangle]
pub unsafe extern "system" fn __vbaVarAdd(
    dst: *mut Variant,
    left: *const Variant,
    right: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    if let (Some(l), Some(r)) = ((*left).as_i32(), (*right).as_i32()) {
        *dst = Variant::from_i32(l.wrapping_add(r));
    } else {
        *dst = *left;
    }
    dst
}

// ----------------------------------------------------------------------------
// SafeArray Helpers
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "system" fn __vbaAryConstruct(_elems: u32, _desc: *mut c_void) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "system" fn __vbaAryConstruct2(_elems: u32, _desc: *mut c_void) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "system" fn __vbaAryDestruct(_arr: *mut SafeArray) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "system" fn __vbaAryLock(_arr: *mut SafeArray) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "system" fn __vbaAryUnlock(_arr: *mut SafeArray) -> i32 {
    0
}

// ----------------------------------------------------------------------------
// Exception and Stack Management
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "system" fn __vbaOnError(_handler: *mut c_void) {}

#[no_mangle]
pub unsafe extern "system" fn __vbaExceptHandler() -> i32 {
    0 // ExceptionContinueExecution / unhandled
}

#[no_mangle]
pub unsafe extern "system" fn __vbaSetSystemError(_code: i32) {}

#[no_mangle]
pub unsafe extern "C" fn __vbaChkstk() {
    // Stack probe routine used by compiler for large alloca / stack frames
}
