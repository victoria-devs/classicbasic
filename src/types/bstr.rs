//! COM Automation BSTR representation and heap allocation engine.
//!
//! A BSTR ("Basic String") is a pointer to the first UTF-16 code unit of a string.
//! The 4 bytes directly preceding the pointer contain the byte length
//! of the string (excluding the null terminator). The string is also
//! terminated by a null UTF-16 character (0x0000).

use std::alloc::{alloc, dealloc, Layout};

/// OLE Automation BSTR pointer type representing a pointer to a null-terminated UTF-16 buffer.
pub type Bstr = *mut u16;

/// Returns the byte length of a BSTR by inspecting the 4 bytes preceding the pointer.
///
/// # Arguments
/// * `s` - BSTR pointer.
///
/// # Safety
/// If non-null, `s` must be a valid BSTR allocated with 4 leading length bytes.
pub unsafe fn bstr_byte_len(s: Bstr) -> u32 {
    if s.is_null() {
        return 0;
    }
    let len_ptr = (s as *const u8).sub(4) as *const u32;
    *len_ptr
}

/// Returns the number of UTF-16 code units in the BSTR.
///
/// # Arguments
/// * `s` - BSTR pointer.
///
/// # Safety
/// If non-null, `s` must be a valid BSTR.
pub unsafe fn bstr_len(s: Bstr) -> usize {
    (bstr_byte_len(s) / 2) as usize
}

/// Converts a BSTR to a Rust slice of u16 code units.
///
/// # Arguments
/// * `s` - BSTR pointer.
///
/// # Safety
/// If non-null, `s` must point to valid memory for the duration of the slice lifetime `'a`.
pub unsafe fn bstr_as_slice<'a>(s: Bstr) -> &'a [u16] {
    if s.is_null() {
        return &[];
    }
    let len = bstr_len(s);
    core::slice::from_raw_parts(s, len)
}

/// Allocates a new BSTR with `len` UTF-16 code units, initialized from `str_in` if non-null.
///
/// # Arguments
/// * `str_in` - Optional pointer to source UTF-16 code units.
/// * `len` - Number of UTF-16 characters to allocate.
///
/// # Returns
/// Pointer to the newly allocated BSTR, or null if allocation fails.
///
/// # Safety
/// `str_in` must be valid for reading `len` code units if non-null.
#[no_mangle]
pub unsafe extern "system" fn SysAllocStringLen(str_in: *const u16, len: u32) -> Bstr {
    let byte_len = len * 2;
    // Layout: 4-byte prefix + byte_len bytes + 2-byte null terminator
    let total_bytes = 4 + (byte_len as usize) + 2;
    let layout = match Layout::from_size_align(total_bytes, 4) {
        Ok(l) => l,
        Err(_) => return core::ptr::null_mut(),
    };

    let mem = alloc(layout);
    if mem.is_null() {
        return core::ptr::null_mut();
    }

    *(mem as *mut u32) = byte_len;
    let bstr_ptr = mem.add(4) as *mut u16;

    if !str_in.is_null() {
        core::ptr::copy_nonoverlapping(str_in, bstr_ptr, len as usize);
    } else {
        core::ptr::write_bytes(bstr_ptr, 0, (len as usize) * 2);
    }
    *bstr_ptr.add(len as usize) = 0; // UTF-16 null terminator

    bstr_ptr
}

/// Allocates a new BSTR from a null-terminated UTF-16 string.
///
/// # Arguments
/// * `str_in` - Pointer to null-terminated UTF-16 string.
///
/// # Returns
/// Pointer to the newly allocated BSTR, or null if `str_in` is null.
///
/// # Safety
/// `str_in` must point to a valid null-terminated sequence of `u16` code units.
#[no_mangle]
pub unsafe extern "system" fn SysAllocString(str_in: *const u16) -> Bstr {
    if str_in.is_null() {
        return core::ptr::null_mut();
    }
    let mut len = 0;
    while *str_in.add(len) != 0 {
        len += 1;
    }
    SysAllocStringLen(str_in, len as u32)
}

/// Deallocates a BSTR string previously allocated via SysAllocString* or runtime helpers.
///
/// # Arguments
/// * `bstr` - Pointer to the BSTR string to release.
///
/// # Safety
/// `bstr` must have been allocated via `SysAllocStringLen` / `SysAllocString`.
#[no_mangle]
pub unsafe extern "system" fn SysFreeString(bstr: Bstr) {
    if bstr.is_null() {
        return;
    }
    let mem = (bstr as *mut u8).sub(4);
    let byte_len = *(mem as *const u32);
    let total_bytes = 4 + (byte_len as usize) + 2;
    if let Ok(layout) = Layout::from_size_align(total_bytes, 4) {
        dealloc(mem, layout);
    }
}

/// Returns the byte length of the BSTR (excluding length prefix and null terminator).
///
/// # Arguments
/// * `bstr` - BSTR string pointer.
///
/// # Safety
/// `bstr` must be valid or null.
#[no_mangle]
pub unsafe extern "system" fn SysStringByteLen(bstr: Bstr) -> u32 {
    bstr_byte_len(bstr)
}

/// Returns the character length of the BSTR in UTF-16 code units.
///
/// # Arguments
/// * `bstr` - BSTR string pointer.
///
/// # Safety
/// `bstr` must be valid or null.
#[no_mangle]
pub unsafe extern "system" fn SysStringLen(bstr: Bstr) -> u32 {
    bstr_len(bstr) as u32
}

/// Helper function to allocate a BSTR from a Rust `&str`.
///
/// # Arguments
/// * `s` - Rust string slice.
///
/// # Returns
/// Pointer to the newly allocated BSTR.
pub fn sys_alloc_string(s: &str) -> Bstr {
    let utf16: Vec<u16> = s.encode_utf16().collect();
    unsafe { SysAllocStringLen(utf16.as_ptr(), utf16.len() as u32) }
}

/// Allocates a BSTR with an exact byte length (`SysAllocStringByteLen` layout).
///
/// # Safety
/// Allocates `4 + bytes.len() + 2` bytes on the heap with a 4-byte length prefix.
pub unsafe fn sys_alloc_string_byte_len(bytes: &[u8]) -> Bstr {
    let byte_len = bytes.len();
    let total_bytes = 4 + byte_len + 2;
    let layout = match Layout::from_size_align(total_bytes, 4) {
        Ok(l) => l,
        Err(_) => return core::ptr::null_mut(),
    };
    let mem = alloc(layout);
    if mem.is_null() {
        return core::ptr::null_mut();
    }
    *(mem as *mut u32) = byte_len as u32;
    let bstr_ptr = mem.add(4);
    if byte_len > 0 {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), bstr_ptr, byte_len);
    }
    *bstr_ptr.add(byte_len) = 0;
    *bstr_ptr.add(byte_len + 1) = 0;
    bstr_ptr as Bstr
}

/// Helper function to convert a BSTR into an owned Rust `String`.
///
/// # Arguments
/// * `s` - BSTR pointer.
///
/// # Safety
/// `s` must be valid or null.
pub unsafe fn bstr_to_string(s: Bstr) -> String {
    if s.is_null() {
        return String::new();
    }
    let slice = bstr_as_slice(s);
    String::from_utf16_lossy(slice)
}

/// Converts a null-terminated UTF-16 pointer into an owned Rust `String`.
///
/// # Safety
/// If non-null, `p` must point to a null-terminated UTF-16 array.
pub unsafe fn utf16_ptr_to_string(p: *const u16) -> String {
    if p.is_null() {
        return String::new();
    }
    let mut len = 0;
    while *p.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(core::slice::from_raw_parts(p, len))
}

