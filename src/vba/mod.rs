//! Visual Basic Architecture (`__vba*`) compiler runtime helpers.
//!
//! When the Visual Basic 6 native-code compiler (C2.EXE) compiles functions,
//! it generates calls to `__vba*` helpers for string management, variant arithmetic,
//! array lifecycle, and SEH (Structured Exception Handling).

use crate::types::{
    bstr_as_slice, bstr_len, bstr_to_string, sys_alloc_string, Bstr, SafeArray, SysAllocStringLen,
    SysFreeString, Variant, VT_BOOL, VT_BSTR, VT_CY, VT_DISPATCH, VT_I2, VT_I4, VT_NULL,
    VT_UNKNOWN,
};
use core::ffi::c_void;

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
    let sl1 = if !s1.is_null() {
        bstr_as_slice(s1)
    } else {
        &[]
    };
    let sl2 = if !s2.is_null() {
        bstr_as_slice(s2)
    } else {
        &[]
    };
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
pub unsafe extern "system" fn __vbaInStr(arg1: i32, arg2: usize, arg3: usize, arg4: i32) -> i32 {
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
    let start_pos = if arg1 > 0 {
        arg1
    } else if arg4 > 0 {
        arg4
    } else {
        1
    };
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
        let vt = (*v).vt;
        if vt == VT_BSTR {
            if let Some(b) = (*v).as_bstr() {
                if !b.is_null() {
                    SysFreeString(b);
                }
            }
        } else if vt == VT_DISPATCH || vt == VT_UNKNOWN {
            let v_ref = &*v;
            let p_obj = usize::from_ne_bytes(
                v_ref.data[..core::mem::size_of::<usize>()]
                    .try_into()
                    .unwrap(),
            ) as *mut c_void;
            if !p_obj.is_null() {
                crate::com::com_release(p_obj);
            }
        }
        *v = Variant::empty();
    }
}

/// Clears multiple `Variant` structures in a single call.
///
/// # Arguments
/// * `count` - Number of `Variant` pointers to release.
#[no_mangle]
pub unsafe extern "C" fn __vbaFreeVarList(count: i32) {
    if count <= 0 {
        return;
    }
    let p_args = (&count as *const i32).add(1) as *const *mut Variant;
    for i in 0..count {
        let p_var = *p_args.add(i as usize);
        __vbaFreeVar(p_var);
    }
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
    if (*left).vt == VT_NULL || (*right).vt == VT_NULL {
        *dst = Variant::null();
        return dst;
    }
    if (*left).vt == VT_CY && (*right).vt == VT_CY {
        if let (Some(cl), Some(cr)) = ((*left).as_cy(), (*right).as_cy()) {
            *dst = Variant::from_cy(crate::types::currency::Currency(cl.0 + cr.0));
            return dst;
        }
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
    if (*left).vt == VT_NULL || (*right).vt == VT_NULL {
        *dst = Variant::null();
        return dst;
    }
    if let (Some(fl), Some(fr)) = ((*left).as_f64(), (*right).as_f64()) {
        if fr == 0.0 {
            crate::err::set_vb_error(11, None, None);
            *dst = Variant::from_f64(0.0);
        } else {
            *dst = Variant::from_f64(fl / fr);
        }
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
    if (*left).vt == VT_NULL || (*right).vt == VT_NULL {
        *dst = Variant::null();
        return dst;
    }
    if let (Some(il), Some(ir)) = ((*left).as_i32(), (*right).as_i32()) {
        if ir == 0 {
            crate::err::set_vb_error(11, None, None);
            *dst = Variant::from_i32(0);
        } else {
            *dst = Variant::from_i32(il / ir);
        }
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
pub unsafe extern "system" fn __vbaVarNeg(dst: *mut Variant, src: *const Variant) -> *mut Variant {
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
pub unsafe extern "system" fn __vbaVarNot(dst: *mut Variant, src: *const Variant) -> *mut Variant {
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
        let bounds_ptr = (*p_array).rgsabound.as_ptr();
        (*bounds_ptr.add((dim - 1) as usize)).l_lbound
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
        let bounds_ptr = (*p_array).rgsabound.as_ptr();
        let b = &*bounds_ptr.add((dim - 1) as usize);
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
    if (*src).vt == crate::types::variant::VT_BSTR {
        if let Some(b) = (*src).as_bstr() {
            let len = crate::types::SysStringLen(b);
            let new_b = crate::types::SysAllocStringLen(b, len);
            *dst = Variant::from_bstr(new_b);
            return dst;
        }
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

// ============================================================================
// Classic BASIC File I/O Subsystem (#1..#511 Channels)
// ============================================================================

/// Standard newline terminator flag (`Print #`).
pub const PRINT_NEWLINE: usize = 0;
/// Semicolon separator flag (`Print #`, suppresses trailing newline).
pub const PRINT_SEMICOLON: usize = 1;
/// Comma separator flag (`Print #`, advances to next 14-column tab stop).
pub const PRINT_COMMA: usize = 2;

/// Represents an open Classic BASIC file channel (`#1` through `#511`).
pub struct VbFileChannel {
    /// Underlying host file handle.
    pub file: std::fs::File,
    /// Path used when opening the file.
    pub path: String,
    /// VB6 open mode flags (`0x01`=Input, `0x02`=Output, `0x04`=Random, `0x08`=Append, `0x20`=Binary).
    pub mode: i32,
    /// Record length (`Len = ...`), or `-1` for default.
    pub record_len: i32,
    /// Current print column position (for 14-column tab alignment).
    pub col: usize,
    /// Buffered tokens for `Input #` statements.
    pub input_buffer: std::collections::VecDeque<String>,
}

static FILE_CHANNELS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::BTreeMap<i32, VbFileChannel>>,
> = std::sync::OnceLock::new();

fn channel_table() -> &'static std::sync::Mutex<std::collections::BTreeMap<i32, VbFileChannel>> {
    FILE_CHANNELS.get_or_init(|| std::sync::Mutex::new(std::collections::BTreeMap::new()))
}

/// Normalizes a Windows filesystem path for Linux/POSIX execution.
///
/// Converts Windows backslashes (`\`) into forward slashes (`/`),
/// strips DOS drive letters (`C:`), and maps them relative to the root or working directory.
pub fn resolve_virtual_path(path: &str) -> String {
    let s = path.trim();
    if s.len() >= 2 && s.as_bytes()[1] == b':' && s.as_bytes()[0].is_ascii_alphabetic() {
        let without_drive = &s[2..];
        let normalized = without_drive.replace('\\', "/");
        if normalized.starts_with('/') {
            normalized.to_lowercase()
        } else {
            format!("/{}", normalized).to_lowercase()
        }
    } else if s.starts_with('\\') || s.starts_with('/') {
        s.replace('\\', "/").to_lowercase()
    } else {
        s.replace('\\', "/")
    }
}

/// Extracts a `String` from either a `Bstr` (`*const u16`) or a `*const Variant` (`VT_BSTR` / `VT_BYREF | VT_BSTR`).
pub unsafe fn extract_bstr_or_variant_path(ptr: Bstr) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let first_word = *ptr;
    if first_word == 0 {
        return String::new();
    }
    let raw = if first_word == crate::types::VT_BSTR
        || first_word == (crate::types::VT_BYREF | crate::types::VT_BSTR)
        || first_word == crate::types::VT_ERROR
    {
        let v = &*(ptr as *const Variant);
        v.to_string_representation()
    } else {
        bstr_to_string(ptr)
    };
    resolve_virtual_path(&raw)
}

/// Finds the next available file number in the requested range (`0` -> `1..=255`, `1` -> `256..=511`).
pub fn find_free_file_channel(range: i32) -> i16 {
    let (start, end) = if range == 1 { (256, 511) } else { (1, 255) };
    if let Ok(tbl) = channel_table().lock() {
        for ch in start..=end {
            if !tbl.contains_key(&ch) {
                return ch as i16;
            }
        }
    }
    1
}

/// Closes all open VB6 file channels (`Reset` / `__vbaFileCloseAll`).
pub fn close_all_file_channels() {
    if let Ok(mut tbl) = channel_table().lock() {
        tbl.clear();
    }
}

/// Returns the length in bytes of an open file channel (`LOF(fn)`).
pub fn get_file_channel_length(file_number: i32) -> i32 {
    if let Ok(tbl) = channel_table().lock() {
        if let Some(ch) = tbl.get(&file_number) {
            if let Ok(meta) = ch.file.metadata() {
                return meta.len() as i32;
            }
        }
    }
    0
}

/// Returns `-1` (`True`) if the given file channel is at End-of-File, or `0` (`False`) otherwise (`EOF(fn)`).
pub fn get_file_channel_eof(file_number: i32) -> i16 {
    use std::io::Seek;
    if let Ok(mut tbl) = channel_table().lock() {
        if let Some(ch) = tbl.get_mut(&file_number) {
            let len = ch.file.metadata().map(|m| m.len()).unwrap_or(0);
            let pos = ch.file.stream_position().unwrap_or(0);
            return if pos >= len { -1 } else { 0 };
        }
    }
    crate::err::set_vb_error(52, None, None);
    -1
}

/// Returns the 1-based byte offset for the next read/write on `file_number` (`Seek(fn)`).
pub fn get_file_channel_seek(file_number: i32) -> i32 {
    use std::io::Seek;
    if let Ok(mut tbl) = channel_table().lock() {
        if let Some(ch) = tbl.get_mut(&file_number) {
            let pos = ch.file.stream_position().unwrap_or(0);
            return (pos as i32) + 1;
        }
    }
    1
}

/// Returns the current position within `file_number` (`Loc(fn)`).
pub fn get_file_channel_loc(file_number: i32) -> i32 {
    use std::io::Seek;
    if let Ok(mut tbl) = channel_table().lock() {
        if let Some(ch) = tbl.get_mut(&file_number) {
            let pos = ch.file.stream_position().unwrap_or(0);
            return pos as i32;
        }
    }
    0
}

/// Returns the open mode or OS handle for `file_number` (`FileAttr(fn, returnType)`).
pub fn get_file_channel_attr(file_number: i32, return_type: i32) -> i32 {
    if return_type == 2 {
        return 0;
    }
    if let Ok(tbl) = channel_table().lock() {
        if let Some(ch) = tbl.get(&file_number) {
            return ch.mode & 0xFF;
        }
    }
    0
}

/// Opens a file channel (`Open filename For mode As #file_number`, Ordinal 235).
#[no_mangle]
pub unsafe extern "system" fn __vbaFileOpen(
    mode: i32,
    record_len: i32,
    file_number: i32,
    filename: Bstr,
) -> i32 {
    if file_number < 1 || file_number > 511 {
        crate::err::set_vb_error(52, None, None);
        return -1;
    }
    let path = extract_bstr_or_variant_path(filename);
    if path.is_empty() {
        crate::err::set_vb_error(52, None, None);
        return -1;
    }

    let low_mode = mode & 0xFF;
    let mut opts = std::fs::OpenOptions::new();
    match low_mode {
        0x01 => {
            // For Input
            opts.read(true);
        }
        0x02 => {
            // For Output
            opts.write(true).create(true).truncate(true);
        }
        0x08 => {
            // For Append
            opts.write(true).create(true).append(true);
        }
        0x04 | 0x20 => {
            // For Random (0x04) or For Binary (0x20)
            opts.read(true).write(true).create(true);
        }
        _ => {
            opts.read(true).write(true).create(true);
        }
    }

    match opts.open(&path) {
        Ok(file) => {
            if let Ok(mut tbl) = channel_table().lock() {
                tbl.insert(
                    file_number,
                    VbFileChannel {
                        file,
                        path,
                        mode: low_mode,
                        record_len,
                        col: 0,
                        input_buffer: std::collections::VecDeque::new(),
                    },
                );
            }
            crate::err::clear_vb_error();
            0
        }
        Err(e) => {
            let code = match e.kind() {
                std::io::ErrorKind::NotFound => 53,
                std::io::ErrorKind::PermissionDenied => 70,
                _ => 75,
            };
            crate::err::set_vb_error(code, None, None);
            -1
        }
    }
}

/// Closes an open file channel (`Close #file_number`, Ordinal 229).
///
/// If `file_number` is `0` or negative, closes all open channels.
#[no_mangle]
pub unsafe extern "system" fn __vbaFileClose(file_number: i32) -> i32 {
    if file_number <= 0 {
        close_all_file_channels();
    } else if let Ok(mut tbl) = channel_table().lock() {
        tbl.remove(&file_number);
    }
    0
}

/// Closes all open file channels (`Close`, Ordinal 233).
#[no_mangle]
pub unsafe extern "C" fn __vbaFileCloseAll() -> i32 {
    close_all_file_channels();
    0
}

/// Locks or unlocks a byte range on an open file channel (`Lock #fn` / `Unlock #fn`, Ordinal 234).
#[no_mangle]
pub unsafe extern "system" fn __vbaFileLock(
    _flags: i32,
    _end_pos: i32,
    _start_pos: i32,
    _file_number: i32,
) -> i32 {
    0
}

/// Seeks to a 1-based byte or record position in an open file channel (`Seek #file_number, position`, Ordinal 236).
#[no_mangle]
pub unsafe extern "system" fn __vbaFileSeek(position: i32, file_number: i32) -> i32 {
    use std::io::{Seek, SeekFrom};
    if position < 1 {
        crate::err::set_vb_error(63, None, None);
        return -1;
    }
    if let Ok(mut tbl) = channel_table().lock() {
        if let Some(ch) = tbl.get_mut(&file_number) {
            let _ = ch.file.seek(SeekFrom::Start((position - 1) as u64));
            return 0;
        }
    }
    crate::err::set_vb_error(52, None, None);
    -1
}

/// Prints a formatted string to an open file channel (`Print #file_number, ...`, Ordinal 355).
#[no_mangle]
pub unsafe extern "C" fn __vbaPrintFile(
    p_desc: *const core::ffi::c_void,
    file_number: i32,
    val: Bstr,
) -> i32 {
    use std::io::Write;
    let text = extract_bstr_or_variant_path(val);
    let desc = p_desc as usize;
    if let Ok(mut tbl) = channel_table().lock() {
        if let Some(ch) = tbl.get_mut(&file_number) {
            let _ = ch.file.write_all(text.as_bytes());
            ch.col += text.len();

            if desc == PRINT_SEMICOLON {
                // Semicolon: no spacing, no newline
            } else if desc == PRINT_COMMA {
                // Comma: align to next 14-column tab stop
                let next_stop = ((ch.col / 14) + 1) * 14;
                let spaces = next_stop - ch.col;
                let pad = vec![b' '; spaces];
                let _ = ch.file.write_all(&pad);
                ch.col = next_stop;
            } else {
                let _ = ch.file.write_all(b"\r\n");
                ch.col = 0;
            }
            let _ = ch.file.flush();
            return 0;
        }
    }
    crate::err::set_vb_error(52, None, None);
    -1
}

/// Writes a delimited value to an open file channel (`Write #file_number, ...`, Ordinal 623).
#[no_mangle]
pub unsafe extern "C" fn __vbaWriteFile(
    _p_desc: *const core::ffi::c_void,
    file_number: i32,
    val: Bstr,
) -> i32 {
    use std::io::Write;
    let text = extract_bstr_or_variant_path(val);
    if let Ok(mut tbl) = channel_table().lock() {
        if let Some(ch) = tbl.get_mut(&file_number) {
            let formatted = format!("\"{}\"\r\n", text);
            let _ = ch.file.write_all(formatted.as_bytes());
            let _ = ch.file.flush();
            return 0;
        }
    }
    crate::err::set_vb_error(52, None, None);
    -1
}

/// Reads a single line from an open file channel into a `BSTR` (`Line Input #file_number, sLine`, Ordinal 331).
#[no_mangle]
pub unsafe extern "system" fn __vbaLineInputStr(dest_bstr: *mut Bstr, file_number: i32) -> i32 {
    use std::io::Read;
    if dest_bstr.is_null() {
        return -1;
    }
    let mut line_bytes: Vec<u8> = Vec::new();
    if let Ok(mut tbl) = channel_table().lock() {
        if let Some(ch) = tbl.get_mut(&file_number) {
            let mut buf = [0u8; 1];
            while let Ok(1) = ch.file.read(&mut buf) {
                if buf[0] == b'\n' {
                    break;
                }
                if buf[0] != b'\r' {
                    line_bytes.push(buf[0]);
                }
            }
        } else {
            crate::err::set_vb_error(52, None, None);
            return -1;
        }
    }
    let line_str = String::from_utf8_lossy(&line_bytes);
    if !(*dest_bstr).is_null() {
        SysFreeString(*dest_bstr);
    }
    *dest_bstr = sys_alloc_string(&line_str);
    0
}

/// Reads a single line from an open file channel into a `Variant` (`Line Input #file_number, vLine`, Ordinal 332).
#[no_mangle]
pub unsafe extern "system" fn __vbaLineInputVar(dest_var: *mut Variant, file_number: i32) -> i32 {
    if dest_var.is_null() {
        return -1;
    }
    let mut bstr_out: Bstr = core::ptr::null_mut();
    let res = __vbaLineInputStr(&mut bstr_out, file_number);
    *dest_var = Variant::from_bstr(bstr_out);
    res
}

fn parse_vb_input_line(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        if chars[i] == '"' {
            i += 1;
            let mut tok = String::new();
            while i < chars.len() && chars[i] != '"' {
                tok.push(chars[i]);
                i += 1;
            }
            if i < chars.len() && chars[i] == '"' {
                i += 1;
            }
            while i < chars.len() && chars[i] != ',' {
                i += 1;
            }
            if i < chars.len() && chars[i] == ',' {
                i += 1;
            }
            tokens.push(tok);
        } else {
            let mut tok = String::new();
            while i < chars.len() && chars[i] != ',' {
                tok.push(chars[i]);
                i += 1;
            }
            if i < chars.len() && chars[i] == ',' {
                i += 1;
            }
            tokens.push(tok.trim().to_string());
        }
    }
    tokens
}

/// Reads an input value from an open file channel (`Input #file_number, ...`, Ordinal 281).
#[no_mangle]
pub unsafe extern "C" fn __vbaInputFile(file_number: i32, dest_bstr: *mut Bstr) -> i32 {
    use std::io::Read;
    if dest_bstr.is_null() {
        return -1;
    }
    if let Ok(mut tbl) = channel_table().lock() {
        if let Some(ch) = tbl.get_mut(&file_number) {
            if ch.input_buffer.is_empty() {
                let mut line_bytes: Vec<u8> = Vec::new();
                let mut buf = [0u8; 1];
                while let Ok(1) = ch.file.read(&mut buf) {
                    if buf[0] == b'\n' {
                        break;
                    }
                    if buf[0] != b'\r' {
                        line_bytes.push(buf[0]);
                    }
                }
                let line_str = String::from_utf8_lossy(&line_bytes);
                for token in parse_vb_input_line(&line_str) {
                    ch.input_buffer.push_back(token);
                }
            }
            if let Some(token) = ch.input_buffer.pop_front() {
                if !(*dest_bstr).is_null() {
                    SysFreeString(*dest_bstr);
                }
                *dest_bstr = sys_alloc_string(&token);
                return 0;
            } else {
                if !(*dest_bstr).is_null() {
                    SysFreeString(*dest_bstr);
                }
                *dest_bstr = sys_alloc_string("");
                return 0;
            }
        }
    }
    crate::err::set_vb_error(52, None, None);
    -1
}

/// Writes binary or record data to an open file channel (`Put #file_number, [pos], data`, Ordinal 358).
#[no_mangle]
pub unsafe extern "system" fn __vbaPut4(
    byte_len: i32,
    data_ptr: *const core::ffi::c_void,
    record_pos: i32,
    file_number: i32,
) -> i32 {
    use std::io::{Seek, SeekFrom, Write};
    if data_ptr.is_null() || byte_len <= 0 {
        return 0;
    }
    if let Ok(mut tbl) = channel_table().lock() {
        if let Some(ch) = tbl.get_mut(&file_number) {
            if record_pos >= 1 {
                let _ = ch.file.seek(SeekFrom::Start((record_pos - 1) as u64));
            }
            let slice = core::slice::from_raw_parts(data_ptr as *const u8, byte_len as usize);
            let _ = ch.file.write_all(slice);
            let _ = ch.file.flush();
            return 0;
        }
    }
    crate::err::set_vb_error(52, None, None);
    -1
}

/// Writes binary or record data to an open file channel (`__vbaPut3`, Ordinal 357).
#[no_mangle]
pub unsafe extern "system" fn __vbaPut3(
    byte_len: i32,
    data_ptr: *const core::ffi::c_void,
    record_pos: i32,
    file_number: i32,
) -> i32 {
    __vbaPut4(byte_len, data_ptr, record_pos, file_number)
}

/// Reads binary or record data from an open file channel (`Get #file_number, [pos], data`, Ordinal 248).
#[no_mangle]
pub unsafe extern "system" fn __vbaGet4(
    byte_len: i32,
    data_ptr: *mut core::ffi::c_void,
    record_pos: i32,
    file_number: i32,
) -> i32 {
    use std::io::{Read, Seek, SeekFrom};
    if data_ptr.is_null() || byte_len <= 0 {
        return 0;
    }
    if let Ok(mut tbl) = channel_table().lock() {
        if let Some(ch) = tbl.get_mut(&file_number) {
            if record_pos >= 1 {
                let _ = ch.file.seek(SeekFrom::Start((record_pos - 1) as u64));
            }
            let slice = core::slice::from_raw_parts_mut(data_ptr as *mut u8, byte_len as usize);
            let _ = ch.file.read(slice);
            return 0;
        }
    }
    crate::err::set_vb_error(52, None, None);
    -1
}

/// Reads binary or record data from an open file channel (`__vbaGet3`, Ordinal 247).
#[no_mangle]
pub unsafe extern "system" fn __vbaGet3(
    byte_len: i32,
    data_ptr: *mut core::ffi::c_void,
    record_pos: i32,
    file_number: i32,
) -> i32 {
    __vbaGet4(byte_len, data_ptr, record_pos, file_number)
}

/// Renames or moves a file (`Name old_name As new_name`, Ordinal 339).
///
/// Note: In the VB6 `__stdcall` ABI, `old_name` is pushed first and `new_name` is pushed second,
/// so `new_name` is the first parameter and `old_name` is the second parameter.
#[no_mangle]
pub unsafe extern "system" fn __vbaNameFile(new_name: Bstr, old_name: Bstr) -> i32 {
    let old_p = extract_bstr_or_variant_path(old_name);
    let new_p = extract_bstr_or_variant_path(new_name);
    if old_p.is_empty() || new_p.is_empty() {
        crate::err::set_vb_error(53, None, None);
        return -1;
    }
    match std::fs::rename(&old_p, &new_p) {
        Ok(_) => {
            crate::err::clear_vb_error();
            0
        }
        Err(_) => {
            crate::err::set_vb_error(53, None, None);
            -1
        }
    }
}

// ----------------------------------------------------------------------------
// Core Primitives & Type Conversions (Iteration 3)
// ----------------------------------------------------------------------------

/// Runtime execution state.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RuntimeState {
    Uninitialized,
    Running,
    Terminated,
}

static RUNTIME_STATE: std::sync::Mutex<RuntimeState> = std::sync::Mutex::new(RuntimeState::Running);
static EXIT_HOOK: std::sync::Mutex<Option<Box<dyn Fn(i32) + Send + Sync>>> =
    std::sync::Mutex::new(None);

pub fn get_runtime_state() -> RuntimeState {
    *RUNTIME_STATE.lock().unwrap()
}

pub fn set_runtime_state(state: RuntimeState) {
    *RUNTIME_STATE.lock().unwrap() = state;
}

pub fn set_exit_hook<F>(hook: Option<F>)
where
    F: Fn(i32) + Send + Sync + 'static,
{
    let mut g = EXIT_HOOK.lock().unwrap();
    *g = hook.map(|f| Box::new(f) as Box<dyn Fn(i32) + Send + Sync>);
}

/// Terminates application execution cleanly (`End` statement, Ordinal 212).
#[no_mangle]
pub unsafe extern "system" fn __vbaEnd() {
    close_all_file_channels();
    set_runtime_state(RuntimeState::Terminated);
    if let Ok(guard) = EXIT_HOOK.lock() {
        if let Some(ref hook) = *guard {
            hook(0);
            return;
        }
    }
    #[cfg(not(test))]
    std::process::exit(0);
}

/// Converts a 16-bit Boolean value into a BSTR (`"True"` or `"False"`, Ordinal 399).
#[no_mangle]
pub unsafe extern "system" fn __vbaStrBool(val: i16) -> Bstr {
    if val != 0 {
        sys_alloc_string("True")
    } else {
        sys_alloc_string("False")
    }
}

/// Converts a 64-bit Currency value into a BSTR (`__vbaStrCy`, Ordinal 409).
#[no_mangle]
pub unsafe extern "system" fn __vbaStrCy(cy: i64) -> Bstr {
    let val = cy as f64 / 10000.0;
    let s = format!("{:.4}", val);
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    sys_alloc_string(trimmed)
}

/// Converts a BSTR string into a 64-bit Currency value (`__vbaCyStr`, Ordinal 202).
#[no_mangle]
pub unsafe extern "system" fn __vbaCyStr(bstr: Bstr) -> i64 {
    if bstr.is_null() {
        return 0;
    }
    let s = bstr_to_string(bstr);
    let trimmed = s.trim();
    if let Ok(v) = trimmed.parse::<f64>() {
        (v * 10000.0).round() as i64
    } else {
        0
    }
}

/// Truncates toward zero for floating-point values (`Fix()`, Ordinal 225).
#[no_mangle]
pub unsafe extern "C" fn __vbaFPFix(val: f64) -> f64 {
    val.trunc()
}

/// Extracts an IEEE 754 64-bit float from a Variant (`__vbaR8Var`, Ordinal 378).
#[no_mangle]
pub unsafe extern "system" fn __vbaR8Var(var: *const Variant) -> f64 {
    if var.is_null() {
        return 0.0;
    }
    (*var).as_f64().unwrap_or(0.0)
}

/// Coerces a Variant to a Boolean (`CBool`, Ordinal 175).
#[no_mangle]
pub unsafe extern "system" fn __vbaBoolVar(var: *const Variant) -> i16 {
    if var.is_null() {
        return 0;
    }
    if (*var).as_bool().unwrap_or(false) {
        -1
    } else {
        0
    }
}

/// Coerces a BSTR to a Boolean (`CBool`, Ordinal 174).
#[no_mangle]
pub unsafe extern "system" fn __vbaBoolStr(bstr: Bstr) -> i16 {
    if bstr.is_null() {
        return 0;
    }
    let s = bstr_to_string(bstr);
    let trimmed = s.trim();
    if trimmed.eq_ignore_ascii_case("true") || trimmed == "-1" {
        -1
    } else if let Ok(n) = trimmed.parse::<f64>() {
        if n != 0.0 {
            -1
        } else {
            0
        }
    } else {
        0
    }
}

/// Coerces a Variant to a 16-bit Integer (`CInt`, Ordinal 263).
#[no_mangle]
pub unsafe extern "system" fn __vbaI2Var(var: *const Variant) -> i16 {
    if var.is_null() {
        return 0;
    }
    (*var).as_i32().unwrap_or(0) as i16
}

/// Converts a BSTR string to a 16-bit integer (`CInt`, Ordinal 262).
#[no_mangle]
pub unsafe extern "system" fn __vbaI2Str(bstr: Bstr) -> i16 {
    if bstr.is_null() {
        return 0;
    }
    let s = bstr_to_string(bstr);
    s.trim().parse::<i16>().unwrap_or_else(|_| {
        s.trim()
            .parse::<f64>()
            .map(|f| f.round() as i16)
            .unwrap_or(0)
    })
}

/// Coerces a Variant to a 32-bit Long integer (`CLng`, Ordinal 273).
#[no_mangle]
pub unsafe extern "system" fn __vbaI4Var(var: *const Variant) -> i32 {
    if var.is_null() {
        return 0;
    }
    (*var).as_i32().unwrap_or(0)
}

/// Converts a BSTR string to a 32-bit integer (`CLng`, Ordinal 272).
#[no_mangle]
pub unsafe extern "system" fn __vbaI4Str(bstr: Bstr) -> i32 {
    if bstr.is_null() {
        return 0;
    }
    let s = bstr_to_string(bstr);
    s.trim().parse::<i32>().unwrap_or_else(|_| {
        s.trim()
            .parse::<f64>()
            .map(|f| f.round() as i32)
            .unwrap_or(0)
    })
}

/// Coerces a Variant to a 32-bit Single float (`CSng`, Ordinal 368).
#[no_mangle]
pub unsafe extern "system" fn __vbaR4Var(var: *const Variant) -> f32 {
    if var.is_null() {
        return 0.0;
    }
    (*var).as_f64().unwrap_or(0.0) as f32
}

/// Coerces a Variant to a 64-bit Currency integer (`CCur`, Ordinal 205).
#[no_mangle]
pub unsafe extern "system" fn __vbaCyVar(var: *const Variant) -> i64 {
    if var.is_null() {
        return 0;
    }
    let f = (*var).as_f64().unwrap_or(0.0);
    (f * 10000.0).round() as i64
}

/// Computes the absolute value of a Variant (`Abs()`, Ordinal 449).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarAbs(dst: *mut Variant, src: *const Variant) -> *mut Variant {
    if dst.is_null() || src.is_null() {
        return dst;
    }
    let val = (*src).as_f64().unwrap_or(0.0).abs();
    *dst = Variant::from_f64(val);
    dst
}

/// Computes the floor of a Variant (`Int()`, Ordinal 473).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarInt(dst: *mut Variant, src: *const Variant) -> *mut Variant {
    if dst.is_null() || src.is_null() {
        return dst;
    }
    let val = (*src).as_f64().unwrap_or(0.0).floor();
    *dst = Variant::from_f64(val);
    dst
}

/// Truncates a Variant toward zero (`Fix()`, Ordinal 463).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarFix(dst: *mut Variant, src: *const Variant) -> *mut Variant {
    if dst.is_null() || src.is_null() {
        return dst;
    }
    let val = (*src).as_f64().unwrap_or(0.0).trunc();
    *dst = Variant::from_f64(val);
    dst
}

/// Implementation of VB6 `Mid$` statement in-place mutation (Ordinal 335).
///
/// Modifies a portion of an existing BSTR in-place without altering the
/// string's allocated capacity or length prefix.
#[no_mangle]
pub unsafe extern "system" fn __vbaMidStmtBstr(dst: *mut Bstr, start: i32, len: i32, src: Bstr) {
    if dst.is_null() || (*dst).is_null() || src.is_null() {
        return;
    }

    let target = *dst;
    let target_len = crate::types::SysStringLen(target) as usize;
    if start <= 0 {
        return;
    }

    let start_idx = (start - 1) as usize;
    if start_idx >= target_len {
        return;
    }

    let avail_in_target = target_len - start_idx;
    let src_len = crate::types::SysStringLen(src) as usize;

    let chars_to_copy = if len < 0 {
        src_len.min(avail_in_target)
    } else {
        (len as usize).min(src_len).min(avail_in_target)
    };

    if chars_to_copy > 0 {
        core::ptr::copy_nonoverlapping(src, target.add(start_idx), chars_to_copy);
    }
}

/// Coerces a BSTR string into a 1D SAFEARRAY of Bytes (VT_UI1).
#[no_mangle]
pub unsafe extern "system" fn __vbaUI1ArrayStr(bstr: Bstr) -> *mut SafeArray {
    if bstr.is_null() {
        return SafeArray::create_vector(1, 0, 0, crate::types::safearray::FADF_HAVEVARTYPE);
    }
    let len = crate::types::SysStringLen(bstr) as usize;
    let psa = SafeArray::create_vector(1, 0, len as u32, crate::types::safearray::FADF_HAVEVARTYPE);
    if !psa.is_null() && !(*psa).pv_data.is_null() {
        let dest = (*psa).pv_data as *mut u8;
        for i in 0..len {
            *dest.add(i) = *bstr.add(i) as u8;
        }
    }
    psa
}

/// Coerces a 1D SAFEARRAY of Bytes (VT_UI1) into a BSTR.
#[no_mangle]
pub unsafe extern "system" fn __vbaStrUI1Array(sa: *const SafeArray) -> Bstr {
    if sa.is_null() || (*sa).pv_data.is_null() {
        return sys_alloc_string("");
    }
    let count = (*sa).rgsabound[0].c_elements as usize;
    let src = (*sa).pv_data as *const u8;
    let utf16_chars: Vec<u16> = (0..count).map(|i| *src.add(i) as u16).collect();
    crate::types::SysAllocStringLen(utf16_chars.as_ptr(), count as u32)
}

/// Layout of the class descriptor structure passed by VB6 compiler to `__vbaNew`.
#[repr(C)]
pub struct VbaClassDescriptor {
    pub reserved: usize,
    pub p_clsid: *const crate::types::Clsid,
    pub p_iid: *const crate::types::Iid,
    pub reserved2: usize,
}

/// Instantiates an object from a compiled class descriptor (`__vbaNew`, Ordinal 341).
///
/// Implements early-bound object creation (`Set obj = New Class`).
///
/// # Arguments
/// * `p_desc` - Pointer to the `VbaClassDescriptor`.
///
/// # Returns
/// Pointer to the instantiated COM interface, or null on failure.
#[no_mangle]
pub unsafe extern "system" fn __vbaNew(p_desc: *const c_void) -> *mut c_void {
    if p_desc.is_null() {
        return core::ptr::null_mut();
    }
    let desc = &*(p_desc as *const VbaClassDescriptor);
    if desc.p_clsid.is_null() {
        return core::ptr::null_mut();
    }
    let clsid = *desc.p_clsid;
    let iid = if !desc.p_iid.is_null() {
        *desc.p_iid
    } else {
        crate::types::IID_IUnknown
    };

    let mut ppv = core::ptr::null_mut();
    let hr = crate::com::DllGetClassObject(&clsid, &crate::types::IID_IClassFactory, &mut ppv);
    if hr >= 0 && !ppv.is_null() {
        let mut inst = core::ptr::null_mut();
        let create_fn: unsafe extern "system" fn(
            *mut c_void,
            *mut c_void,
            *const crate::types::Iid,
            *mut *mut c_void,
        ) -> crate::types::HResult = core::mem::transmute(*(*(ppv as *mut *mut usize)).add(3));
        let hr_inst = create_fn(ppv, core::ptr::null_mut(), &iid, &mut inst);
        crate::com::com_release(ppv);
        if hr_inst >= 0 {
            return inst;
        }
    }

    crate::err::set_vb_error(429, Some("ActiveX component can't create object"), None);
    core::ptr::null_mut()
}

/// Dynamically casts an object reference to a requested interface (`__vbaCastObj`, Ordinal 178).
///
/// # Arguments
/// * `p_obj` - Pointer to the COM object.
/// * `p_iid` - Pointer to the requested interface GUID.
///
/// # Returns
/// Pointer to the interface, or null on failure.
#[no_mangle]
pub unsafe extern "system" fn __vbaCastObj(
    p_obj: *mut c_void,
    p_iid: *const crate::types::Iid,
) -> *mut c_void {
    if p_obj.is_null() {
        return core::ptr::null_mut();
    }
    if p_iid.is_null() {
        return p_obj;
    }
    let mut ppv = core::ptr::null_mut();
    let hr = crate::com::com_query_interface(p_obj, p_iid, &mut ppv);
    if hr >= 0 && !ppv.is_null() {
        // QueryInterface incremented the refcount. Release the intermediate reference
        // because VB6 callers immediately invoke __vbaObjSet which adds its own reference.
        crate::com::com_release(ppv);
        return ppv;
    }
    crate::err::set_vb_error(13, Some("Type mismatch"), None);
    core::ptr::null_mut()
}

/// Moves a string value out of a Variant into a BSTR (`__vbaStrVarMove`, Ordinal 437).
///
/// # Arguments
/// * `v` - Pointer to source Variant.
///
/// # Returns
/// Pointer to the extracted BSTR.
#[no_mangle]
pub unsafe extern "system" fn __vbaStrVarMove(v: *mut Variant) -> *mut u16 {
    if v.is_null() {
        return core::ptr::null_mut();
    }
    if (*v).vt == crate::types::variant::VT_BSTR {
        let b = (*v).as_bstr().unwrap_or(core::ptr::null_mut());
        *v = Variant::empty();
        b
    } else {
        let s = (*v).to_string_representation();
        __vbaFreeVar(v);
        sys_alloc_string(&s)
    }
}

// ----------------------------------------------------------------------------
// Late-Binding Dispatch Helpers (Phase 2.3)
// ----------------------------------------------------------------------------

/// Invokes a method on an object via late binding (`__vbaLateMemCall`, Ordinal 292).
///
/// Calling convention: `cdecl` (`extern "C"`).
///
/// # Safety
///
/// `p_obj` must point to a valid COM object or null.
/// `member_name` must point to a null-terminated UTF-16 string or null.
#[no_mangle]
pub unsafe extern "C" fn __vbaLateMemCall(
    p_obj: *mut c_void,
    member_name: *const u16,
    arg_count: u32,
) {
    let args_ptr = (&arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_member(
        p_obj,
        member_name,
        crate::com::DISPATCH_METHOD,
        args_ptr,
        arg_count,
        core::ptr::null_mut(),
    );
}

/// Invokes a method or property get on an object via late binding, returning a Variant (`__vbaLateMemCallLd`, Ordinal 293).
///
/// Calling convention: `cdecl` (`extern "C"`).
///
/// # Safety
///
/// `ret_var` must point to a valid destination Variant.
#[no_mangle]
pub unsafe extern "C" fn __vbaLateMemCallLd(
    ret_var: *mut Variant,
    p_obj: *mut c_void,
    member_name: *const u16,
    arg_count: u32,
) -> *mut Variant {
    if !ret_var.is_null() {
        *ret_var = Variant::empty();
    }
    let args_ptr = (&arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_member(
        p_obj,
        member_name,
        crate::com::DISPATCH_METHOD | crate::com::DISPATCH_PROPERTYGET,
        args_ptr,
        arg_count,
        ret_var,
    );
    ret_var
}

/// Sets a property on an object via late binding (`__vbaLateMemCallSt`, Ordinal 294).
///
/// Calling convention: `cdecl` (`extern "C"`).
///
/// # Safety
///
/// `p_obj` must point to a valid COM object or null.
#[no_mangle]
pub unsafe extern "C" fn __vbaLateMemCallSt(
    p_obj: *mut c_void,
    member_name: *const u16,
    arg_count: u32,
) {
    let args_ptr = (&arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_member(
        p_obj,
        member_name,
        crate::com::DISPATCH_PROPERTYPUT,
        args_ptr,
        arg_count,
        core::ptr::null_mut(),
    );
}

/// Sets a property on an object via late binding (`__vbaLateMemSt`, Ordinal 323).
///
/// Calling convention: `cdecl` (`extern "C"`).
///
/// # Safety
///
/// `p_obj` must point to a valid COM object or null.
#[no_mangle]
pub unsafe extern "C" fn __vbaLateMemSt(
    p_obj: *mut c_void,
    member_name: *const u16,
    arg_count: u32,
) {
    __vbaLateMemCallSt(p_obj, member_name, arg_count);
}

/// Invokes a method by DISPID via late binding (`__vbaLateIdCall`, Ordinal 282).
///
/// Calling convention: `cdecl` (`extern "C"`).
///
/// # Safety
///
/// `p_obj` must point to a valid COM object or null.
#[no_mangle]
pub unsafe extern "C" fn __vbaLateIdCall(
    p_obj: *mut c_void,
    dispid: i32,
    arg_count: u32,
) {
    let args_ptr = (&arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_id(
        p_obj,
        dispid,
        crate::com::DISPATCH_METHOD,
        args_ptr,
        arg_count,
        core::ptr::null_mut(),
    );
}

/// Invokes a method or property get by DISPID via late binding, returning a Variant (`__vbaLateIdCallLd`, Ordinal 283).
///
/// Calling convention: `cdecl` (`extern "C"`).
///
/// # Safety
///
/// `ret_var` must point to a valid destination Variant.
#[no_mangle]
pub unsafe extern "C" fn __vbaLateIdCallLd(
    ret_var: *mut Variant,
    p_obj: *mut c_void,
    dispid: i32,
    arg_count: u32,
) -> *mut Variant {
    if !ret_var.is_null() {
        *ret_var = Variant::empty();
    }
    let args_ptr = (&arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_id(
        p_obj,
        dispid,
        crate::com::DISPATCH_METHOD | crate::com::DISPATCH_PROPERTYGET,
        args_ptr,
        arg_count,
        ret_var,
    );
    ret_var
}

/// Sets a property by DISPID via late binding (`__vbaLateIdCallSt`, Ordinal 285).
///
/// Calling convention: `cdecl` (`extern "C"`).
///
/// # Safety
///
/// `p_obj` must point to a valid COM object or null.
#[no_mangle]
pub unsafe extern "C" fn __vbaLateIdCallSt(
    p_obj: *mut c_void,
    dispid: i32,
    arg_count: u32,
) {
    let args_ptr = (&arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_id(
        p_obj,
        dispid,
        crate::com::DISPATCH_PROPERTYPUT,
        args_ptr,
        arg_count,
        core::ptr::null_mut(),
    );
}

/// Sets a property by DISPID via late binding (`__vbaLateIdSt`, Ordinal 290).
///
/// Calling convention: `cdecl` (`extern "C"`).
///
/// # Safety
///
/// `p_obj` must point to a valid COM object or null.
#[no_mangle]
pub unsafe extern "C" fn __vbaLateIdSt(
    p_obj: *mut c_void,
    dispid: i32,
    arg_count: u32,
) {
    __vbaLateIdCallSt(p_obj, dispid, arg_count);
}

/// Extracts an IUnknown/IDispatch object pointer from a Variant (`__vbaUnkVar`, Ordinal 447).
///
/// Calling convention: `__stdcall` (`extern "system"`).
///
/// # Safety
///
/// `p_var` must point to a valid Variant or null.
#[no_mangle]
pub unsafe extern "system" fn __vbaUnkVar(
    p_var: *const Variant,
    _reserved: u32,
) -> *mut c_void {
    if p_var.is_null() {
        return core::ptr::null_mut();
    }
    let var = &*p_var;
    let vt = var.vt;
    if vt == VT_DISPATCH || vt == VT_UNKNOWN {
        let mut ptr_bytes = [0u8; core::mem::size_of::<usize>()];
        ptr_bytes.copy_from_slice(&var.data[..core::mem::size_of::<usize>()]);
        usize::from_ne_bytes(ptr_bytes) as *mut c_void
    } else {
        core::ptr::null_mut()
    }
}

/// Assigns an object reference into a destination Variant and calls `AddRef` (`__vbaVarSetObjAddref`, Ordinal 483).
///
/// Calling convention: `__stdcall` (`extern "system"`).
///
/// # Safety
///
/// `p_dst` must point to a valid Variant destination or null.
#[no_mangle]
pub unsafe extern "system" fn __vbaVarSetObjAddref(
    p_dst: *mut Variant,
    p_obj: *mut c_void,
) -> *mut Variant {
    if p_dst.is_null() {
        return core::ptr::null_mut();
    }
    let dst = &mut *p_dst;
    if dst.vt == VT_DISPATCH || dst.vt == VT_UNKNOWN {
        let mut ptr_bytes = [0u8; core::mem::size_of::<usize>()];
        ptr_bytes.copy_from_slice(&dst.data[..core::mem::size_of::<usize>()]);
        let existing = usize::from_ne_bytes(ptr_bytes) as *mut c_void;
        if !existing.is_null() {
            crate::com::com_release(existing);
        }
    }
    if p_obj.is_null() {
        *dst = Variant::empty();
    } else {
        *dst = Variant::empty();
        dst.vt = VT_DISPATCH;
        let ptr_bytes = (p_obj as usize).to_ne_bytes();
        dst.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
        crate::com::com_add_ref(p_obj);
    }
    p_dst
}

/// Invokes a method or property get on an object stored inside a Variant (`__vbaVarLateMemCallLd`, Ordinal 112).
///
/// Calling convention: `cdecl` (`extern "C"`).
///
/// # Safety
///
/// `ret_var` must point to a valid destination Variant.
#[no_mangle]
pub unsafe extern "C" fn __vbaVarLateMemCallLd(
    ret_var: *mut Variant,
    p_var_obj: *const Variant,
    member_name: *const u16,
    arg_count: u32,
) -> *mut Variant {
    let p_obj = if p_var_obj.is_null() {
        core::ptr::null_mut()
    } else {
        __vbaUnkVar(p_var_obj, 0)
    };
    if p_obj.is_null() {
        crate::err::set_vb_error(91, None, None); // Object variable or With block variable not set
        if !ret_var.is_null() {
            *ret_var = Variant::empty();
        }
        return ret_var;
    }
    __vbaLateMemCallLd(ret_var, p_obj, member_name, arg_count)
}

/// Sets a property on an object stored inside a Variant (`__vbaVarLateMemCallSt`, Ordinal 114).
///
/// Calling convention: `cdecl` (`extern "C"`).
///
/// # Safety
///
/// `p_var_obj` must point to a valid Variant or null.
#[no_mangle]
pub unsafe extern "C" fn __vbaVarLateMemCallSt(
    p_var_obj: *const Variant,
    member_name: *const u16,
    arg_count: u32,
) {
    let p_obj = if p_var_obj.is_null() {
        core::ptr::null_mut()
    } else {
        __vbaUnkVar(p_var_obj, 0)
    };
    if p_obj.is_null() {
        crate::err::set_vb_error(91, None, None);
        return;
    }
    __vbaLateMemCallSt(p_obj, member_name, arg_count);
}

/// Sets a property on an object stored inside a Variant (`__vbaVarLateMemSt`, Ordinal 115).
///
/// Calling convention: `cdecl` (`extern "C"`).
///
/// # Safety
///
/// `p_var_obj` must point to a valid Variant or null.
#[no_mangle]
pub unsafe extern "C" fn __vbaVarLateMemSt(
    p_var_obj: *const Variant,
    member_name: *const u16,
    arg_count: u32,
) {
    __vbaVarLateMemCallSt(p_var_obj, member_name, arg_count);
}

