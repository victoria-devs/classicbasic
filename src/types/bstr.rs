//! COM Automation BSTR representation.
//!
//! A BSTR is a pointer to the first UTF-16 code unit of a string.
//! The 4 bytes directly preceding the pointer contain the byte length
//! of the string (excluding the null terminator). The string is also
//! terminated by a null UTF-16 character (0x0000).

pub type Bstr = *mut u16;

/// Returns the byte length of a BSTR by inspecting the 4 bytes preceding the pointer.
pub unsafe fn bstr_byte_len(s: Bstr) -> u32 {
    if s.is_null() {
        return 0;
    }
    let len_ptr = (s as *const u8).sub(4) as *const u32;
    *len_ptr
}

/// Returns the number of UTF-16 code units in the BSTR.
pub unsafe fn bstr_len(s: Bstr) -> usize {
    (bstr_byte_len(s) / 2) as usize
}

/// Converts a BSTR to a Rust slice of u16 code units.
pub unsafe fn bstr_as_slice<'a>(s: Bstr) -> &'a [u16] {
    if s.is_null() {
        return &[];
    }
    let len = bstr_len(s);
    core::slice::from_raw_parts(s, len)
}
