//! Visual Basic 6.0 Memory Access and Pointer Functions.
//!
//! Visual Basic 6.0 does not natively expose pointer arithmetic or dereferencing in the language syntax.
//! Instead, developers and the runtime use these low-level memory functions (`VarPtr`, `GetMem*`, `PutMem*`, `SetMem*`)
//! to manipulate raw memory, pass buffers to the Windows API, and optimize structure passing.

use core::ffi::c_void;
use crate::types::{E_POINTER, HResult, S_OK, Variant};

/// Returns the pointer passed as argument.
///
/// In VB6, `VarPtr(Variable)` returns the 32-bit memory address of any variable.
/// Calling convention: `__stdcall` (`extern "system"`).
#[no_mangle]
pub unsafe extern "system" fn VarPtr(ptr: *mut c_void) -> *mut c_void {
    ptr
}

// ----------------------------------------------------------------------------
// GetMem Functions (Read from raw address into destination)
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "system" fn GetMem1(src: *const u8, dst: *mut u8) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn GetMem2(src: *const u16, dst: *mut u16) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn GetMem4(src: *const u32, dst: *mut u32) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn GetMem8(src: *const u64, dst: *mut u64) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn GetMemStr(src: *const *mut u16, dst: *mut *mut u16) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn GetMemVar(src: *const Variant, dst: *mut Variant) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn GetMemObj(src: *const *mut c_void, dst: *mut *mut c_void) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn GetMemNewObj(src: *const *mut c_void, dst: *mut *mut c_void) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn GetMemEvent(src: *const *mut c_void, dst: *mut *mut c_void) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

// ----------------------------------------------------------------------------
// PutMem Functions (Write value into destination address)
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "system" fn PutMem1(dst: *mut u8, val: u8) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn PutMem2(dst: *mut u16, val: u16) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn PutMem4(dst: *mut u32, val: u32) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn PutMem8(dst: *mut u64, val: u64) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn PutMemStr(dst: *mut *mut u16, val: *mut u16) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn PutMemVar(dst: *mut Variant, val: *const Variant) -> HResult {
    if dst.is_null() || val.is_null() {
        return E_POINTER;
    }
    *dst = *val;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn PutMemObj(dst: *mut *mut c_void, val: *mut c_void) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn PutMemNewObj(dst: *mut *mut c_void, val: *mut c_void) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn PutMemEvent(dst: *mut *mut c_void, val: *mut c_void) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

// ----------------------------------------------------------------------------
// SetMem Functions (Reference-counted write for COM objects and Variants)
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "system" fn SetMemVar(dst: *mut Variant, val: *const Variant) -> HResult {
    if dst.is_null() || val.is_null() {
        return E_POINTER;
    }
    *dst = *val;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn SetMemObj(dst: *mut *mut c_void, val: *mut c_void) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn SetMemNewObj(dst: *mut *mut c_void, val: *mut c_void) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn SetMemEvent(dst: *mut *mut c_void, val: *mut c_void) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}
