//! Visual Basic Architecture (`__vba*`) compiler runtime helpers.
//!
//! When the Visual Basic 6 native-code compiler (C2.EXE) compiles functions,
//! it generates calls to `__vba*` helpers for string management, variant arithmetic,
//! array lifecycle, and SEH (Structured Exception Handling).

use core::ffi::c_void;
use crate::types::{
    bstr_as_slice, bstr_len, bstr_to_string, sys_alloc_string, Bstr, SafeArray, SysAllocStringLen,
    SysFreeString, Variant, VT_BOOL, VT_BSTR, VT_I2, VT_I4,
};

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
        if !(*dst).is_null() {
            SysFreeString(*dst);
        }
        if !src.is_null() {
            let slice = bstr_as_slice(src);
            *dst = SysAllocStringLen(slice.as_ptr(), slice.len() as u32);
        } else {
            *dst = core::ptr::null_mut();
        }
        return *dst;
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
    if !dst.is_null() {
        if !(*dst).is_null() {
            SysFreeString(*dst);
        }
        if !src.is_null() {
            *dst = *src;
            *src = core::ptr::null_mut();
        } else {
            *dst = core::ptr::null_mut();
        }
        return *dst;
    }
    core::ptr::null_mut()
}

/// Concatenates two BSTR strings and returns a newly allocated BSTR pointer.
///
/// # Arguments
/// * `s1` - First BSTR string.
/// * `s2` - Second BSTR string to concatenate.
///
/// # Returns
/// The concatenated BSTR pointer.
#[no_mangle]
pub unsafe extern "system" fn __vbaStrCat(s1: Bstr, s2: Bstr) -> Bstr {
    let sl1 = if !s1.is_null() { bstr_as_slice(s1) } else { &[] };
    let sl2 = if !s2.is_null() { bstr_as_slice(s2) } else { &[] };
    let total_len = sl1.len() + sl2.len();
    let mut buf = Vec::with_capacity(total_len);
    buf.extend_from_slice(sl1);
    buf.extend_from_slice(sl2);
    SysAllocStringLen(buf.as_ptr(), total_len as u32)
}

/// Returns the length in characters of a BSTR string.
///
/// # Arguments
/// * `s` - BSTR string pointer.
///
/// # Returns
/// Length in UTF-16 code units.
#[no_mangle]
pub unsafe extern "system" fn __vbaLenBstr(s: Bstr) -> i32 {
    bstr_len(s) as i32
}

/// Converts a 32-bit signed integer to a newly allocated BSTR string.
///
/// # Arguments
/// * `val` - 32-bit integer value.
///
/// # Returns
/// Allocated BSTR string representation.
#[no_mangle]
pub unsafe extern "system" fn __vbaStrI4(val: i32) -> Bstr {
    let s = val.to_string();
    sys_alloc_string(&s)
}

/// Converts a 16-bit signed integer to a newly allocated BSTR.
///
/// # Arguments
/// * `val` - 16-bit integer value.
///
/// # Returns
/// Pointer to the newly allocated BSTR string.
#[no_mangle]
pub unsafe extern "system" fn __vbaStrI2(val: i16) -> Bstr {
    let s = val.to_string();
    sys_alloc_string(&s)
}

/// Copies and coerces a `Variant` structure to a newly allocated BSTR.
///
/// # Arguments
/// * `src` - Pointer to source `Variant`.
///
/// # Returns
/// Pointer to the newly allocated BSTR string representing the variant.
#[no_mangle]
pub unsafe extern "system" fn __vbaStrErrVarCopy(src: *const Variant) -> Bstr {
    if src.is_null() {
        return sys_alloc_string("");
    }
    let s = (*src).to_string_representation();
    sys_alloc_string(&s)
}

/// Searches for a substring within a string (equivalent to VB6 `InStr`).
///
/// # Arguments
/// * `arg1` - Start index or comparison method flag.
/// * `arg2` - Pointer to haystack or needle BSTR.
/// * `arg3` - Pointer to needle or haystack BSTR.
/// * `arg4` - Start index or comparison method flag.
///
/// # Returns
/// 1-based character index of the match, or 0 if not found.
#[no_mangle]
pub unsafe extern "system" fn __vbaInStr(
    arg1: i32,
    arg2: usize,
    arg3: usize,
    arg4: i32,
) -> i32 {
    let b1 = arg2 as Bstr;
    let b2 = arg3 as Bstr;
    let (str_haystack, str_needle) = if bstr_len(b1) >= bstr_len(b2) {
        (b1, b2)
    } else {
        (b2, b1)
    };
    if str_haystack.is_null() || str_needle.is_null() {
        return 0;
    }
    let hay = bstr_as_slice(str_haystack);
    let needle = bstr_as_slice(str_needle);
    if needle.is_empty() {
        return 1;
    }
    let start_pos = if arg1 > 0 { arg1 } else if arg4 > 0 { arg4 } else { 1 };
    let start_idx = (start_pos.max(1) - 1) as usize;
    if start_idx >= hay.len() {
        return 0;
    }
    for i in start_idx..=hay.len().saturating_sub(needle.len()) {
        if &hay[i..i + needle.len()] == needle {
            return (i + 1) as i32;
        }
    }
    0
}

/// Converts an ANSI string buffer to a Unicode BSTR.
///
/// # Arguments
/// * `dst` - Pointer to destination BSTR variable.
/// * `ansi_src` - Pointer to null-terminated ANSI string buffer.
///
/// # Returns
/// The newly allocated Unicode BSTR.
#[no_mangle]
pub unsafe extern "system" fn __vbaStrToUnicode(dst: *mut Bstr, ansi_src: *const u8) -> Bstr {
    if ansi_src.is_null() {
        if !dst.is_null() {
            *dst = core::ptr::null_mut();
        }
        return core::ptr::null_mut();
    }
    let mut len = 0;
    while *ansi_src.add(len) != 0 {
        len += 1;
    }
    let slice = core::slice::from_raw_parts(ansi_src, len);
    let s = String::from_utf8_lossy(slice);
    let b = sys_alloc_string(&s);
    if !dst.is_null() {
        if !(*dst).is_null() {
            SysFreeString(*dst);
        }
        *dst = b;
    }
    b
}

/// Converts a Unicode BSTR to an ANSI string buffer.
///
/// # Arguments
/// * `dst` - Pointer to destination ANSI buffer pointer.
/// * `unicode_src` - Source Unicode BSTR string.
///
/// # Returns
/// Pointer to the newly allocated ANSI character buffer.
#[no_mangle]
pub unsafe extern "system" fn __vbaStrToAnsi(dst: *mut *mut u8, unicode_src: Bstr) -> *mut u8 {
    if unicode_src.is_null() {
        if !dst.is_null() {
            *dst = core::ptr::null_mut();
        }
        return core::ptr::null_mut();
    }
    let s = bstr_to_string(unicode_src);
    let bytes = s.into_bytes();
    let total_len = bytes.len() + 1;
    let layout = std::alloc::Layout::from_size_align(total_len, 1).unwrap();
    let mem = std::alloc::alloc(layout);
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), mem, bytes.len());
    *mem.add(bytes.len()) = 0; // null terminator
    if !dst.is_null() {
        *dst = mem;
    }
    mem
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
    let sl1 = bstr_as_slice(s1);
    let sl2 = bstr_as_slice(s2);
    for (c1, c2) in sl1.iter().zip(sl2.iter()) {
        if c1 != c2 {
            return (*c1 as i32) - (*c2 as i32);
        }
    }
    (sl1.len() as i32) - (sl2.len() as i32)
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
        if !(*s).is_null() {
            SysFreeString(*s);
            *s = core::ptr::null_mut();
        }
    }
}

/// Frees multiple BSTR string variables in a single call.
///
/// # Arguments
/// * `_count` - Number of BSTR pointers in the argument list.
#[no_mangle]
pub unsafe extern "system" fn __vbaFreeStrList(_count: i32) {
    // List free of BSTRs
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
        if (*src).vt == VT_BSTR {
            if let Some(b) = (*src).as_bstr() {
                let s = bstr_to_string(b);
                *dst = Variant::from_bstr(sys_alloc_string(&s));
            }
        }
    }
    dst
}

/// Clears a `Variant` structure, releasing any nested BSTR and resetting tag to `VT_EMPTY`.
///
/// # Arguments
/// * `v` - Pointer to the `Variant` to clear.
#[no_mangle]
pub unsafe extern "system" fn __vbaFreeVar(v: *mut Variant) {
    if !v.is_null() {
        if (*v).vt == VT_BSTR {
            if let Some(b) = (*v).as_bstr() {
                if !b.is_null() {
                    SysFreeString(b);
                }
            }
        }
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

/// Dynamically adds two `Variant` values, performing numeric addition or string concatenation.
///
/// # Arguments
/// * `dst` - Pointer to destination `Variant`.
/// * `right` - Right operand `Variant`.
/// * `left` - Left operand `Variant`.
///
/// # Returns
/// Pointer to destination `Variant`.
#[no_mangle]
pub unsafe extern "system" fn __vbaVarAdd(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    if (*left).vt != VT_BSTR && (*right).vt != VT_BSTR {
        if let (Some(l), Some(r)) = ((*left).as_i32(), (*right).as_i32()) {
            *dst = Variant::from_i32(l.wrapping_add(r));
            return dst;
        }
    }
    if let (Some(fl), Some(fr)) = ((*left).as_f64(), (*right).as_f64()) {
        let sum = fl + fr;
        if sum.fract() == 0.0 && sum.abs() <= i32::MAX as f64 && sum.abs() >= i32::MIN as f64 {
            *dst = Variant::from_i32(sum as i32);
        } else {
            *dst = Variant::from_f64(sum);
        }
    } else {
        let sl = (*left).to_string_representation();
        let sr = (*right).to_string_representation();
        let combined = format!("{}{}", sl, sr);
        *dst = Variant::from_bstr(sys_alloc_string(&combined));
    }
    dst
}

/// Dynamically subtracts two `Variant` values (`left - right`).
///
/// # Arguments
/// * `dst` - Pointer to destination `Variant`.
/// * `right` - Right operand `Variant`.
/// * `left` - Left operand `Variant`.
///
/// # Returns
/// Pointer to destination `Variant`.
#[no_mangle]
pub unsafe extern "system" fn __vbaVarSub(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    if (*left).vt != VT_BSTR && (*right).vt != VT_BSTR {
        if let (Some(l), Some(r)) = ((*left).as_i32(), (*right).as_i32()) {
            *dst = Variant::from_i32(l.wrapping_sub(r));
            return dst;
        }
    }
    if let (Some(fl), Some(fr)) = ((*left).as_f64(), (*right).as_f64()) {
        let diff = fl - fr;
        if diff.fract() == 0.0 && diff.abs() <= i32::MAX as f64 && diff.abs() >= i32::MIN as f64 {
            *dst = Variant::from_i32(diff as i32);
        } else {
            *dst = Variant::from_f64(diff);
        }
    } else {
        *dst = Variant::empty();
    }
    dst
}

/// Dynamically multiplies two `Variant` values (`left * right`).
///
/// # Arguments
/// * `dst` - Pointer to destination `Variant`.
/// * `right` - Right operand `Variant`.
/// * `left` - Left operand `Variant`.
///
/// # Returns
/// Pointer to destination `Variant`.
#[no_mangle]
pub unsafe extern "system" fn __vbaVarMul(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    if (*left).vt != VT_BSTR && (*right).vt != VT_BSTR {
        if let (Some(l), Some(r)) = ((*left).as_i32(), (*right).as_i32()) {
            *dst = Variant::from_i32(l.wrapping_mul(r));
            return dst;
        }
    }
    if let (Some(fl), Some(fr)) = ((*left).as_f64(), (*right).as_f64()) {
        let prod = fl * fr;
        if prod.fract() == 0.0 && prod.abs() <= i32::MAX as f64 && prod.abs() >= i32::MIN as f64 {
            *dst = Variant::from_i32(prod as i32);
        } else {
            *dst = Variant::from_f64(prod);
        }
    } else {
        *dst = Variant::empty();
    }
    dst
}

/// Dynamically performs floating-point division on two `Variant` values (`left / right`).
///
/// # Arguments
/// * `dst` - Pointer to destination `Variant`.
/// * `right` - Right operand `Variant`.
/// * `left` - Left operand `Variant`.
///
/// # Returns
/// Pointer to destination `Variant` (`VT_R8`).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarDiv(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    if let (Some(fl), Some(fr)) = ((*left).as_f64(), (*right).as_f64()) {
        let res = if fr != 0.0 { fl / fr } else { 0.0 };
        *dst = Variant::from_f64(res);
    } else {
        *dst = Variant::empty();
    }
    dst
}

/// Dynamically performs integer division on two `Variant` values (`left \ right`).
///
/// # Arguments
/// * `dst` - Pointer to destination `Variant`.
/// * `right` - Right operand `Variant`.
/// * `left` - Left operand `Variant`.
///
/// # Returns
/// Pointer to destination `Variant` (`VT_I4`).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarIdiv(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    if let (Some(il), Some(ir)) = ((*left).as_i32(), (*right).as_i32()) {
        let res = if ir != 0 { il / ir } else { 0 };
        *dst = Variant::from_i32(res);
    } else {
        *dst = Variant::empty();
    }
    dst
}

/// Dynamically calculates the modulus of two `Variant` values (`left Mod right`).
///
/// # Arguments
/// * `dst` - Pointer to destination `Variant`.
/// * `right` - Right operand `Variant`.
/// * `left` - Left operand `Variant`.
///
/// # Returns
/// Pointer to destination `Variant` (`VT_I4`).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarMod(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    if let (Some(il), Some(ir)) = ((*left).as_i32(), (*right).as_i32()) {
        let res = if ir != 0 { il % ir } else { 0 };
        *dst = Variant::from_i32(res);
    } else {
        *dst = Variant::empty();
    }
    dst
}

/// Dynamically concatenates two `Variant` values as strings (`left & right`).
///
/// # Arguments
/// * `dst` - Pointer to destination `Variant`.
/// * `right` - Right operand `Variant`.
/// * `left` - Left operand `Variant`.
///
/// # Returns
/// Pointer to destination `Variant` (`VT_BSTR`).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarCat(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let sl = (*left).to_string_representation();
    let sr = (*right).to_string_representation();
    let combined = format!("{}{}", sl, sr);
    *dst = Variant::from_bstr(sys_alloc_string(&combined));
    dst
}

/// Dynamically negates a `Variant` value (`-src`).
///
/// # Arguments
/// * `dst` - Pointer to destination `Variant`.
/// * `src` - Source `Variant` operand.
///
/// # Returns
/// Pointer to destination `Variant`.
#[no_mangle]
pub unsafe extern "system" fn __vbaVarNeg(
    dst: *mut Variant,
    src: *const Variant,
) -> *mut Variant {
    if dst.is_null() || src.is_null() {
        return dst;
    }
    if (*src).vt == VT_I2 || (*src).vt == VT_I4 {
        if let Some(i) = (*src).as_i32() {
            *dst = Variant::from_i32(-i);
            return dst;
        }
    }
    if let Some(f) = (*src).as_f64() {
        if f.fract() == 0.0 && f.abs() <= i32::MAX as f64 {
            *dst = Variant::from_i32(-(f as i32));
        } else {
            *dst = Variant::from_f64(-f);
        }
    } else {
        *dst = Variant::empty();
    }
    dst
}

/// Dynamically performs logical or bitwise NOT on a `Variant` value (`Not src`).
///
/// # Arguments
/// * `dst` - Pointer to destination `Variant`.
/// * `src` - Source `Variant` operand.
///
/// # Returns
/// Pointer to destination `Variant`.
#[no_mangle]
pub unsafe extern "system" fn __vbaVarNot(
    dst: *mut Variant,
    src: *const Variant,
) -> *mut Variant {
    if dst.is_null() || src.is_null() {
        return dst;
    }
    if (*src).vt == VT_BOOL {
        let b = (*src).as_bool().unwrap_or(false);
        *dst = Variant::from_bool(!b);
    } else if let Some(i) = (*src).as_i32() {
        *dst = Variant::from_i32(!i);
    } else {
        *dst = Variant::empty();
    }
    dst
}

/// Dynamically compares if `left > right`, storing a `VT_BOOL` in `dst`.
///
/// # Arguments
/// * `dst` - Pointer to destination `Variant`.
/// * `right` - Right operand `Variant`.
/// * `left` - Left operand `Variant`.
///
/// # Returns
/// Pointer to destination `Variant` (`VT_BOOL`).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarCmpGt(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let res = if let (Some(fl), Some(fr)) = ((*left).as_f64(), (*right).as_f64()) {
        fl > fr
    } else {
        (*left).to_string_representation() > (*right).to_string_representation()
    };
    *dst = Variant::from_bool(res);
    dst
}

/// Dynamically compares if `left == right`, storing a `VT_BOOL` in `dst`.
///
/// # Arguments
/// * `dst` - Pointer to destination `Variant`.
/// * `right` - Right operand `Variant`.
/// * `left` - Left operand `Variant`.
///
/// # Returns
/// Pointer to destination `Variant` (`VT_BOOL`).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarCmpEq(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let res = if let (Some(fl), Some(fr)) = ((*left).as_f64(), (*right).as_f64()) {
        fl == fr
    } else {
        (*left).to_string_representation() == (*right).to_string_representation()
    };
    *dst = Variant::from_bool(res);
    dst
}

/// Dynamically compares if `left < right`, storing a `VT_BOOL` in `dst`.
#[no_mangle]
pub unsafe extern "system" fn __vbaVarCmpLt(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let res = if let (Some(fl), Some(fr)) = ((*left).as_f64(), (*right).as_f64()) {
        fl < fr
    } else {
        (*left).to_string_representation() < (*right).to_string_representation()
    };
    *dst = Variant::from_bool(res);
    dst
}

/// Dynamically compares if `left <= right`, storing a `VT_BOOL` in `dst`.
#[no_mangle]
pub unsafe extern "system" fn __vbaVarCmpLe(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let res = if let (Some(fl), Some(fr)) = ((*left).as_f64(), (*right).as_f64()) {
        fl <= fr
    } else {
        (*left).to_string_representation() <= (*right).to_string_representation()
    };
    *dst = Variant::from_bool(res);
    dst
}

/// Dynamically compares if `left >= right`, storing a `VT_BOOL` in `dst`.
#[no_mangle]
pub unsafe extern "system" fn __vbaVarCmpGe(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let res = if let (Some(fl), Some(fr)) = ((*left).as_f64(), (*right).as_f64()) {
        fl >= fr
    } else {
        (*left).to_string_representation() >= (*right).to_string_representation()
    };
    *dst = Variant::from_bool(res);
    dst
}

/// Dynamically compares if `left != right`, storing a `VT_BOOL` in `dst`.
#[no_mangle]
pub unsafe extern "system" fn __vbaVarCmpNe(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let res = if let (Some(fl), Some(fr)) = ((*left).as_f64(), (*right).as_f64()) {
        fl != fr
    } else {
        (*left).to_string_representation() != (*right).to_string_representation()
    };
    *dst = Variant::from_bool(res);
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

// Note: __vbaOnError is implemented in crate::err

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

/// Converts a `Variant` to an 8-byte IEEE Date (`f64`) (`__vbaDateVar`, Ordinal 209).
#[no_mangle]
pub unsafe extern "system" fn __vbaDateVar(p_var: *const Variant) -> f64 {
    if p_var.is_null() {
        return 0.0;
    }
    let v = &*p_var;
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&v.data);
    f64::from_ne_bytes(bytes)
}

/// Converts a 32-bit single-precision float (`f32`) to a `BSTR` (`__vbaStrR4`, Ordinal 422).
#[no_mangle]
pub unsafe extern "system" fn __vbaStrR4(val: f32) -> Bstr {
    let s = if val.fract() == 0.0 {
        format!("{}", val as i64)
    } else {
        format!("{}", val)
    };
    sys_alloc_string(&s)
}

/// Computes floor/integer part of a floating-point number (`__vbaFPInt`, Ordinal 226).
#[no_mangle]
pub unsafe extern "C" fn __vbaFPInt(val: f64) -> f64 {
    val.floor()
}

/// Converts or passes through single-precision float (`__vbaFpR4`, Ordinal 126).
#[no_mangle]
pub unsafe extern "C" fn __vbaFpR4(val: f32) -> f32 {
    val
}

// ----------------------------------------------------------------------------
// Dynamic Array & Vector Operations
// ----------------------------------------------------------------------------

/// Redimensions an array variable (`__vbaRedim`, Ordinal 385).
#[no_mangle]
pub unsafe extern "C" fn __vbaRedim(
    f_features: u32,
    elem_size: u32,
    pp_array: *mut *mut SafeArray,
    _vt: u32,
    _c_dims: u32,
    ubound: i32,
    lbound: i32,
) -> i32 {
    if pp_array.is_null() {
        return 0;
    }
    if !(*pp_array).is_null() {
        SafeArray::destroy(*pp_array);
        *pp_array = core::ptr::null_mut();
    }
    let count = if ubound >= lbound {
        (ubound - lbound + 1) as u32
    } else {
        0
    };
    *pp_array = SafeArray::create_vector(elem_size, lbound, count, f_features as u16);
    0
}

/// Redimensions an array variable, preserving existing contents (`__vbaRedimPreserve`, Ordinal 386).
#[no_mangle]
pub unsafe extern "C" fn __vbaRedimPreserve(
    f_features: u32,
    elem_size: u32,
    pp_array: *mut *mut SafeArray,
    _vt: u32,
    _c_dims: u32,
    ubound: i32,
    lbound: i32,
) -> i32 {
    if pp_array.is_null() {
        return 0;
    }
    if (*pp_array).is_null() {
        let count = if ubound >= lbound {
            (ubound - lbound + 1) as u32
        } else {
            0
        };
        *pp_array = SafeArray::create_vector(elem_size, lbound, count, f_features as u16);
    } else {
        SafeArray::redim_preserve(*pp_array, ubound, lbound);
    }
    0
}

/// Returns the lower bound for the given dimension of an array (`__vbaLbound`, Ordinal 325).
#[no_mangle]
pub unsafe extern "system" fn __vbaLbound(p_array: *const SafeArray, dim: i32) -> i32 {
    if p_array.is_null() {
        return 0;
    }
    if dim >= 1 && dim <= (*p_array).c_dims as i32 {
        (*p_array).rgsabound[(dim - 1) as usize].l_lbound
    } else {
        0
    }
}

/// Returns the upper bound for the given dimension of an array (`__vbaUbound`, Ordinal 445).
#[no_mangle]
pub unsafe extern "system" fn __vbaUbound(p_array: *const SafeArray, dim: i32) -> i32 {
    if p_array.is_null() {
        return 0;
    }
    if dim >= 1 && dim <= (*p_array).c_dims as i32 {
        let b = &(*p_array).rgsabound[(dim - 1) as usize];
        b.l_lbound + (b.c_elements as i32) - 1
    } else {
        0
    }
}

/// Erases an array, releasing its memory (`__vbaErase`, Ordinal 213).
#[no_mangle]
pub unsafe extern "system" fn __vbaErase(pp_array: *mut *mut SafeArray, _flags: i32) -> i32 {
    if !pp_array.is_null() && !(*pp_array).is_null() {
        SafeArray::destroy(*pp_array);
        *pp_array = core::ptr::null_mut();
    }
    0
}

/// Copies an array structure (`__vbaAryCopy`, Ordinal 166).
#[no_mangle]
pub unsafe extern "system" fn __vbaAryCopy(
    pp_dst: *mut *mut SafeArray,
    pp_src: *const *const SafeArray,
) -> i32 {
    if pp_dst.is_null() {
        return 0;
    }
    if !(*pp_dst).is_null() {
        SafeArray::destroy(*pp_dst);
        *pp_dst = core::ptr::null_mut();
    }
    if pp_src.is_null() || (*pp_src).is_null() {
        return 0;
    }
    let src = &**pp_src;
    let new_sa = SafeArray::create_vector(
        src.cb_elements,
        src.rgsabound[0].l_lbound,
        src.rgsabound[0].c_elements,
        src.f_features,
    );
    if !new_sa.is_null() && !src.pv_data.is_null() {
        let bytes = (src.rgsabound[0].c_elements as usize) * (src.cb_elements as usize);
        core::ptr::copy_nonoverlapping(src.pv_data, (*new_sa).pv_data, bytes);
    }
    *pp_dst = new_sa;
    0
}

/// Extracts `SafeArray` pointer from a `Variant` (`__vbaAryVar`, Ordinal 173).
#[no_mangle]
pub unsafe extern "system" fn __vbaAryVar(p_var: *const Variant, _vt: u32) -> *mut SafeArray {
    if p_var.is_null() {
        return core::ptr::null_mut();
    }
    let v = &*p_var;
    let mut ptr_val = 0usize;
    core::ptr::copy_nonoverlapping(
        v.data.as_ptr(),
        &mut ptr_val as *mut usize as *mut u8,
        core::mem::size_of::<usize>(),
    );
    if (v.vt & 0x4000) != 0 {
        // VT_BYREF
        let p_ref = ptr_val as *const usize;
        if !p_ref.is_null() {
            return *p_ref as *mut SafeArray;
        }
    }
    ptr_val as *mut SafeArray
}

/// Obtains pointer to SafeArray pointer inside a Variant (`__vbaRefVarAry`, Ordinal 390).
#[no_mangle]
pub unsafe extern "system" fn __vbaRefVarAry(p_var: *mut Variant) -> *mut *mut SafeArray {
    if p_var.is_null() {
        return core::ptr::null_mut();
    }
    let v = &mut *p_var;
    if (v.vt & 0x4000) != 0 {
        // VT_BYREF
        let mut ptr_val = 0usize;
        core::ptr::copy_nonoverlapping(
            v.data.as_ptr(),
            &mut ptr_val as *mut usize as *mut u8,
            core::mem::size_of::<usize>(),
        );
        ptr_val as *mut *mut SafeArray
    } else {
        v.data.as_mut_ptr() as *mut *mut SafeArray
    }
}

/// Loads an indexed element from an array in a Variant (`__vbaVarIndexLoad`, Ordinal 468).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarIndexLoad(
    dst: *mut Variant,
    p_arr_var: *const Variant,
    _num_indices: i32,
    idx1: i32,
) -> *mut Variant {
    if dst.is_null() || p_arr_var.is_null() {
        return dst;
    }
    let sa = __vbaAryVar(p_arr_var, 0);
    if sa.is_null() || (*sa).pv_data.is_null() {
        return dst;
    }
    let lbound = (*sa).rgsabound[0].l_lbound;
    let count = (*sa).rgsabound[0].c_elements as i32;
    if idx1 < lbound || idx1 >= lbound + count {
        return dst;
    }
    let offset = (idx1 - lbound) as usize;
    let elem_size = (*sa).cb_elements as usize;
    let elem_ptr = ((*sa).pv_data as *const u8).add(offset * elem_size);
    if elem_size == core::mem::size_of::<Variant>() {
        *dst = *(elem_ptr as *const Variant);
    } else if elem_size == 4 {
        let val = *(elem_ptr as *const i32);
        *dst = Variant::from_i32(val);
    }
    dst
}

/// Performs bitwise AND on two variants (`__vbaVarAnd`, Ordinal 451).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarAnd(
    dst: *mut Variant,
    left: *const Variant,
    right: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let l_val = (*left).as_i32().unwrap_or(0);
    let r_val = (*right).as_i32().unwrap_or(0);
    *dst = Variant::from_i32(l_val & r_val);
    dst
}

/// Duplicates a Variant (`__vbaVarDup`, Ordinal 144).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarDup(dst: *mut Variant, src: *const Variant) -> *mut Variant {
    if dst.is_null() || src.is_null() {
        return dst;
    }
    *dst = *src;
    dst
}

/// Tests boolean truthiness of a Variant (`__vbaBoolVarNull`, Ordinal 176).
#[no_mangle]
pub unsafe extern "system" fn __vbaBoolVarNull(p_var: *const Variant) -> i16 {
    if p_var.is_null() {
        return 0;
    }
    if (*p_var).as_bool().unwrap_or(false) {
        -1
    } else {
        0
    }
}

/// Generates subscript out of range error (`__vbaGenerateBoundsError`, Ordinal 246).
#[no_mangle]
pub unsafe extern "system" fn __vbaGenerateBoundsError() {
    crate::err::set_vb_error(9, None, None);
}

/// Generates overflow error (`__vbaErrorOverflow`, Ordinal 217).
#[no_mangle]
pub unsafe extern "system" fn __vbaErrorOverflow() {
    crate::err::set_vb_error(6, None, None);
}
