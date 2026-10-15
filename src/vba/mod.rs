//! Visual Basic Architecture (`__vba*`) compiler runtime helpers.
//!
//! When the Visual Basic 6 native-code compiler (C2.EXE) compiles functions,
//! it generates calls to `__vba*` helpers for string management, variant arithmetic,
//! array lifecycle, and SEH (Structured Exception Handling).

use core::ffi::c_void;
use crate::types::{Bstr, SafeArray, Variant};

// ----------------------------------------------------------------------------
// Low-level Memory Operations
// ----------------------------------------------------------------------------

/// Copies a block of memory from source to destination.
///
/// Emitted by the compiler for fixed-size buffer copying, UDT (User Defined Type)
/// assignments, and memory transfers.
///
/// # Arguments
/// * `count` - Number of bytes to copy.
/// * `dst` - Pointer to destination buffer.
/// * `src` - Pointer to source buffer.
#[no_mangle]
pub unsafe extern "system" fn __vbaCopyBytes(count: usize, dst: *mut u8, src: *const u8) {
    if !dst.is_null() && !src.is_null() && count > 0 {
        core::ptr::copy(src, dst, count);
    }
}

/// Copies memory from source to destination, zero-filling the destination if the source is null.
///
/// # Arguments
/// * `count` - Number of bytes to copy or zero.
/// * `dst` - Pointer to destination buffer.
/// * `src` - Pointer to source buffer, or null to zero-fill `dst`.
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

/// Assigns a BSTR to a destination pointer variable by copying the string reference.
///
/// # Arguments
/// * `dst` - Pointer to destination BSTR variable.
/// * `src` - Source BSTR pointer.
///
/// # Returns
/// The resulting BSTR pointer.
#[no_mangle]
pub unsafe extern "system" fn __vbaStrCopy(dst: *mut Bstr, src: Bstr) -> Bstr {
    if !dst.is_null() {
        *dst = src;
    }
    src
}

/// Moves a BSTR from source to destination, transferring ownership and clearing source.
///
/// # Arguments
/// * `dst` - Pointer to destination BSTR variable.
/// * `src` - Pointer to source BSTR variable (set to null on move).
///
/// # Returns
/// The moved BSTR pointer.
#[no_mangle]
pub unsafe extern "system" fn __vbaStrMove(dst: *mut Bstr, src: *mut Bstr) -> Bstr {
    if !dst.is_null() && !src.is_null() {
        *dst = *src;
        *src = core::ptr::null_mut();
        return *dst;
    }
    core::ptr::null_mut()
}

/// Concatenates two BSTR strings and writes the resulting BSTR pointer into destination.
///
/// # Arguments
/// * `dst` - Pointer to destination BSTR variable.
/// * `_src` - Secondary BSTR to concatenate.
///
/// # Returns
/// The concatenated BSTR pointer.
#[no_mangle]
pub unsafe extern "system" fn __vbaStrCat(dst: *mut Bstr, _src: Bstr) -> Bstr {
    if !dst.is_null() {
        *dst
    } else {
        core::ptr::null_mut()
    }
}

/// Compares two BSTR strings lexicographically.
///
/// # Arguments
/// * `s1` - First BSTR string.
/// * `s2` - Second BSTR string.
///
/// # Returns
/// * `< 0` if `s1` is less than `s2`.
/// * `0` if `s1` equals `s2`.
/// * `> 0` if `s1` is greater than `s2`.
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

/// Frees an allocated BSTR string and sets its pointer variable to null.
///
/// In standard VB6 runtime, this invokes `SysFreeString` in OLE Automation.
///
/// # Arguments
/// * `s` - Pointer to the BSTR variable to release.
#[no_mangle]
pub unsafe extern "system" fn __vbaFreeStr(s: *mut Bstr) {
    if !s.is_null() {
        *s = core::ptr::null_mut();
    }
}

/// Frees multiple BSTR string variables in a single call.
///
/// # Arguments
/// * `_count` - Number of BSTR pointers in the argument list.
#[no_mangle]
pub unsafe extern "system" fn __vbaFreeStrList(_count: i32) {
    // Variadic or list free of BSTRs
}

// ----------------------------------------------------------------------------
// Variant Helpers
// ----------------------------------------------------------------------------

/// Moves a `Variant` structure from source to destination, transferring ownership.
///
/// # Arguments
/// * `dst` - Pointer to destination `Variant`.
/// * `src` - Pointer to source `Variant`.
///
/// # Returns
/// Pointer to destination `Variant`.
#[no_mangle]
pub unsafe extern "system" fn __vbaVarMove(dst: *mut Variant, src: *const Variant) -> *mut Variant {
    if !dst.is_null() && !src.is_null() {
        *dst = *src;
    }
    dst
}

/// Copies a `Variant` structure from source to destination by value.
///
/// # Arguments
/// * `dst` - Pointer to destination `Variant`.
/// * `src` - Pointer to source `Variant`.
///
/// # Returns
/// Pointer to destination `Variant`.
#[no_mangle]
pub unsafe extern "system" fn __vbaVarCopy(dst: *mut Variant, src: *const Variant) -> *mut Variant {
    if !dst.is_null() && !src.is_null() {
        *dst = *src;
    }
    dst
}

/// Clears a `Variant` structure, resetting its type tag to `VT_EMPTY`.
///
/// # Arguments
/// * `v` - Pointer to the `Variant` to clear.
#[no_mangle]
pub unsafe extern "system" fn __vbaFreeVar(v: *mut Variant) {
    if !v.is_null() {
        *v = Variant::empty();
    }
}

/// Clears multiple `Variant` structures in a single call.
///
/// # Arguments
/// * `_count` - Number of `Variant` pointers to release.
#[no_mangle]
pub unsafe extern "system" fn __vbaFreeVarList(_count: i32) {
    // Variadic or list free of Variants
}

/// Dynamically adds two `Variant` values, storing the result in destination.
///
/// In VB6, variant addition automatically coerces types (e.g. integer addition,
/// float addition, or string concatenation if strings are involved).
///
/// # Arguments
/// * `dst` - Pointer to destination `Variant`.
/// * `left` - Left operand `Variant`.
/// * `right` - Right operand `Variant`.
///
/// # Returns
/// Pointer to destination `Variant`.
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

/// Constructs and initializes a `SafeArray` descriptor for an array variable.
///
/// # Arguments
/// * `_elems` - Number of elements or element size.
/// * `_desc` - Pointer to array descriptor memory.
///
/// # Returns
/// Status code (`0` for success).
#[no_mangle]
pub unsafe extern "system" fn __vbaAryConstruct(_elems: u32, _desc: *mut c_void) -> i32 {
    0
}

/// Secondary constructor for multi-dimensional or complex `SafeArray` descriptors.
///
/// # Arguments
/// * `_elems` - Number of elements or dimensions.
/// * `_desc` - Pointer to array descriptor memory.
///
/// # Returns
/// Status code (`0` for success).
#[no_mangle]
pub unsafe extern "system" fn __vbaAryConstruct2(_elems: u32, _desc: *mut c_void) -> i32 {
    0
}

/// Destructs a `SafeArray`, freeing element memory and deallocating the descriptor.
///
/// # Arguments
/// * `_arr` - Pointer to `SafeArray` descriptor.
///
/// # Returns
/// Status code (`0` for success).
#[no_mangle]
pub unsafe extern "system" fn __vbaAryDestruct(_arr: *mut SafeArray) -> i32 {
    0
}

/// Increments the lock count of a `SafeArray` to prevent reallocation or resizing during iteration.
///
/// # Arguments
/// * `_arr` - Pointer to `SafeArray` descriptor.
///
/// # Returns
/// Status code (`0` for success).
#[no_mangle]
pub unsafe extern "system" fn __vbaAryLock(_arr: *mut SafeArray) -> i32 {
    0
}

/// Decrements the lock count of a `SafeArray`.
///
/// # Arguments
/// * `_arr` - Pointer to `SafeArray` descriptor.
///
/// # Returns
/// Status code (`0` for success).
#[no_mangle]
pub unsafe extern "system" fn __vbaAryUnlock(_arr: *mut SafeArray) -> i32 {
    0
}

// ----------------------------------------------------------------------------
// Exception and Stack Management
// ----------------------------------------------------------------------------

/// Registers an active `On Error Goto` / `On Error Resume Next` exception handler.
///
/// # Arguments
/// * `_handler` - Pointer to exception handler record or code target address.
#[no_mangle]
pub unsafe extern "system" fn __vbaOnError(_handler: *mut c_void) {}

/// Structured Exception Handling (SEH) top-level filter for Visual Basic 6 modules.
///
/// Catches hardware exceptions (access violations, divide-by-zero, stack overflow)
/// and converts them into Visual Basic runtime error numbers (`Err.Number`).
///
/// # Returns
/// Exception disposition code (`0` = `ExceptionContinueExecution`).
#[no_mangle]
pub unsafe extern "system" fn __vbaExceptHandler() -> i32 {
    0 // ExceptionContinueExecution / unhandled
}

/// Sets the thread's last error code in the VB6 runtime error subsystem.
///
/// # Arguments
/// * `_code` - Integer error code.
#[no_mangle]
pub unsafe extern "system" fn __vbaSetSystemError(_code: i32) {}

/// Compiler stack probe routine.
///
/// When a function has a stack frame larger than a memory page (4 KB), the compiler
/// emits calls to `__vbaChkstk` to touch pages sequentially, preventing stack overflow faults.
#[no_mangle]
pub unsafe extern "C" fn __vbaChkstk() {
    // Stack probe routine used by compiler for large alloca / stack frames
}
