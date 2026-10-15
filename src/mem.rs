//! Visual Basic 6.0 Memory Access and Pointer Functions.
//!
//! Visual Basic 6.0 does not natively expose pointer arithmetic or dereferencing in the language syntax.
//! Instead, developers and the runtime use these low-level memory functions (`VarPtr`, `GetMem*`, `PutMem*`, `SetMem*`)
//! to manipulate raw memory, pass buffers to the Windows API, and optimize structure passing.

use core::ffi::c_void;
use crate::types::{E_POINTER, HResult, S_OK, Variant};

/// Returns the raw memory address passed as an argument.
///
/// In Visual Basic 6.0, `VarPtr(Variable)` returns the 32-bit virtual memory address
/// of any variable or data structure, allowing low-level interop with Windows APIs.
///
/// # Arguments
/// * `ptr` - Pointer to any variable or memory buffer.
///
/// # Returns
/// Returns the exact pointer passed into the function.
#[no_mangle]
pub unsafe extern "system" fn VarPtr(ptr: *mut c_void) -> *mut c_void {
    ptr
}

// ----------------------------------------------------------------------------
// GetMem Functions (Read from raw address into destination)
// ----------------------------------------------------------------------------

/// Reads a 1-byte value (`u8` / VB6 `Byte`) from the source address into destination.
///
/// # Arguments
/// * `src` - Pointer to the source 8-bit memory location.
/// * `dst` - Pointer to the destination buffer receiving the value.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if either `src` or `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn GetMem1(src: *const u8, dst: *mut u8) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

/// Reads a 2-byte value (`u16` / VB6 `Integer` or `Boolean`) from source into destination.
///
/// # Arguments
/// * `src` - Pointer to the source 16-bit memory location.
/// * `dst` - Pointer to the destination buffer receiving the value.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if either `src` or `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn GetMem2(src: *const u16, dst: *mut u16) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

/// Reads a 4-byte value (`u32` / VB6 `Long` or 32-bit pointer) from source into destination.
///
/// # Arguments
/// * `src` - Pointer to the source 32-bit memory location.
/// * `dst` - Pointer to the destination buffer receiving the value.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if either `src` or `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn GetMem4(src: *const u32, dst: *mut u32) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

/// Reads an 8-byte value (`u64` / VB6 `Double`, `Date`, or `Currency`) from source into destination.
///
/// # Arguments
/// * `src` - Pointer to the source 64-bit memory location.
/// * `dst` - Pointer to the destination buffer receiving the value.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if either `src` or `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn GetMem8(src: *const u64, dst: *mut u64) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

/// Reads a BSTR string pointer from source into destination.
///
/// # Arguments
/// * `src` - Pointer to a BSTR pointer variable.
/// * `dst` - Destination pointer variable receiving the BSTR pointer.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if either `src` or `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn GetMemStr(src: *const *mut u16, dst: *mut *mut u16) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

/// Reads a 16-byte `Variant` structure from source into destination.
///
/// # Arguments
/// * `src` - Pointer to the source `Variant`.
/// * `dst` - Destination pointer receiving the copy of the `Variant`.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if either `src` or `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn GetMemVar(src: *const Variant, dst: *mut Variant) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

/// Reads a COM object interface pointer (`IUnknown*` / `IDispatch*`) from source into destination.
///
/// # Arguments
/// * `src` - Pointer to the source COM interface pointer.
/// * `dst` - Destination pointer receiving the interface pointer.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if either `src` or `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn GetMemObj(src: *const *mut c_void, dst: *mut *mut c_void) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

/// Reads a newly created COM object pointer from source into destination.
///
/// # Arguments
/// * `src` - Pointer to the source COM interface pointer.
/// * `dst` - Destination pointer receiving the interface pointer.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if either `src` or `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn GetMemNewObj(src: *const *mut c_void, dst: *mut *mut c_void) -> HResult {
    if src.is_null() || dst.is_null() {
        return E_POINTER;
    }
    *dst = *src;
    S_OK
}

/// Reads an event sink interface pointer from source into destination.
///
/// # Arguments
/// * `src` - Pointer to the source event sink interface pointer.
/// * `dst` - Destination pointer receiving the interface pointer.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if either `src` or `dst` is null.
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

/// Writes a 1-byte value (`u8` / VB6 `Byte`) directly into the destination address.
///
/// # Arguments
/// * `dst` - Pointer to the destination 8-bit memory location.
/// * `val` - 8-bit byte value to write.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn PutMem1(dst: *mut u8, val: u8) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

/// Writes a 2-byte value (`u16` / VB6 `Integer`) directly into the destination address.
///
/// # Arguments
/// * `dst` - Pointer to the destination 16-bit memory location.
/// * `val` - 16-bit integer value to write.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn PutMem2(dst: *mut u16, val: u16) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

/// Writes a 4-byte value (`u32` / VB6 `Long` or 32-bit pointer) directly into destination.
///
/// # Arguments
/// * `dst` - Pointer to the destination 32-bit memory location.
/// * `val` - 32-bit value to write.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn PutMem4(dst: *mut u32, val: u32) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

/// Writes an 8-byte value (`u64` / VB6 `Double`, `Date`, or `Currency`) directly into destination.
///
/// # Arguments
/// * `dst` - Pointer to the destination 64-bit memory location.
/// * `val` - 64-bit value to write.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn PutMem8(dst: *mut u64, val: u64) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

/// Writes a BSTR string pointer into destination without copying the string buffer.
///
/// # Arguments
/// * `dst` - Destination pointer variable receiving the BSTR pointer.
/// * `val` - BSTR pointer to store.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn PutMemStr(dst: *mut *mut u16, val: *mut u16) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

/// Writes a `Variant` structure from `val` into destination `dst` by value copy.
///
/// # Arguments
/// * `dst` - Pointer to the destination `Variant`.
/// * `val` - Pointer to the source `Variant` to copy.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if either `dst` or `val` is null.
#[no_mangle]
pub unsafe extern "system" fn PutMemVar(dst: *mut Variant, val: *const Variant) -> HResult {
    if dst.is_null() || val.is_null() {
        return E_POINTER;
    }
    *dst = *val;
    S_OK
}

/// Writes a COM interface pointer into destination `dst`.
///
/// # Arguments
/// * `dst` - Destination pointer variable receiving the interface pointer.
/// * `val` - COM interface pointer to store.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn PutMemObj(dst: *mut *mut c_void, val: *mut c_void) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

/// Writes a new COM object pointer into destination `dst`.
///
/// # Arguments
/// * `dst` - Destination pointer variable receiving the interface pointer.
/// * `val` - Newly instantiated COM interface pointer.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn PutMemNewObj(dst: *mut *mut c_void, val: *mut c_void) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

/// Writes an event sink interface pointer into destination `dst`.
///
/// # Arguments
/// * `dst` - Destination pointer variable receiving the event sink pointer.
/// * `val` - Event sink interface pointer to store.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if `dst` is null.
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

/// Sets a `Variant` reference into destination `dst` adhering to VB6 `Set` semantics.
///
/// In Visual Basic 6.0, `Set x = y` increments COM reference counts when object types
/// are assigned within variants.
///
/// # Arguments
/// * `dst` - Pointer to destination `Variant`.
/// * `val` - Pointer to source `Variant`.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if either pointer is null.
#[no_mangle]
pub unsafe extern "system" fn SetMemVar(dst: *mut Variant, val: *const Variant) -> HResult {
    if dst.is_null() || val.is_null() {
        return E_POINTER;
    }
    *dst = *val;
    S_OK
}

/// Sets a COM object pointer into destination `dst` adhering to VB6 `Set` semantics.
///
/// # Arguments
/// * `dst` - Pointer to destination COM object pointer variable.
/// * `val` - COM interface pointer being assigned.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn SetMemObj(dst: *mut *mut c_void, val: *mut c_void) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

/// Sets a newly created COM object pointer into destination `dst`.
///
/// # Arguments
/// * `dst` - Pointer to destination COM object pointer variable.
/// * `val` - New COM interface pointer being assigned.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn SetMemNewObj(dst: *mut *mut c_void, val: *mut c_void) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}

/// Sets an event sink interface pointer into destination `dst`.
///
/// # Arguments
/// * `dst` - Pointer to destination event sink pointer variable.
/// * `val` - Event sink interface pointer being assigned.
///
/// # Returns
/// * `S_OK` (0) on success.
/// * `E_POINTER` if `dst` is null.
#[no_mangle]
pub unsafe extern "system" fn SetMemEvent(dst: *mut *mut c_void, val: *mut c_void) -> HResult {
    if dst.is_null() {
        return E_POINTER;
    }
    *dst = val;
    S_OK
}
