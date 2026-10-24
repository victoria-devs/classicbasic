// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Complete VB6 runtime standard library closure (`rtc*` and `rt*`).
//!
//! Implements all 73 remaining runtime library functions:
//! - Variant String overloads: Left, Right, Mid, Trim, LTrim, RTrim, LCase, UCase,
//!   Space, String, InStr, Len, StrComp, Byte/Char Asc/Chr routines.
//! - Error & Inspection: CVErr, IsError, IsMissing, Erl, Format on Variant.
//! - Date/Time & Calendar: Date/Time statements and Variant getters.
//! - File, Directory & Environment: CurDir, Environ, FileDateTime, ChDrive, Input$.
//! - Colors, GUI & Legacy Shims: QBColor, MacID, InputBox, AppActivate, SendKeys, IMEStatus.

use crate::types::{
    bstr_as_slice, bstr_byte_len, bstr_len, bstr_to_string, sys_alloc_string,
    sys_alloc_string_byte_len, Bstr, SysAllocStringLen, Variant,
    VT_CY, VT_DATE, VT_EMPTY, VT_ERROR, VT_I2, VT_I4,
};
use core::sync::atomic::{AtomicI32, Ordering};

static CURRENT_CALENDAR: AtomicI32 = AtomicI32::new(0); // 0 = Gregorian, 1 = Hijri

/// Helper to coerce any Variant reference into an owned Rust String.
unsafe fn variant_to_string(v: &Variant) -> String {
    if let Some(b) = v.as_bstr() {
        bstr_to_string(b)
    } else if let Some(i) = v.as_i32() {
        i.to_string()
    } else if let Some(f) = v.as_f64() {
        f.to_string()
    } else if let Some(b) = v.as_bool() {
        if b {
            "True".to_string()
        } else {
            "False".to_string()
        }
    } else {
        String::new()
    }
}

// ----------------------------------------------------------------------------
// 1. Variant String Overloads & Byte/Char Helpers
// ----------------------------------------------------------------------------

/// Implementation of VB6 `Left(Variant, Length)` returning Variant (`rtcLeftVar`, Ordinal 513).
#[no_mangle]
pub unsafe extern "system" fn rtcLeftVar(
    dst: *mut Variant,
    str_var: *const Variant,
    len_var: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if str_var.is_null() {
        *dst = Variant::empty();
        return dst;
    }
    let s_orig = variant_to_string(&*str_var);
    let take_len = if !len_var.is_null() {
        (&*len_var).as_i32().unwrap_or(0).max(0) as usize
    } else {
        0
    };
    let u16_chars: Vec<u16> = s_orig.encode_utf16().collect();
    let count = take_len.min(u16_chars.len());
    let b = SysAllocStringLen(u16_chars.as_ptr(), count as u32);
    *dst = Variant::from_bstr(b);
    dst
}

/// Implementation of VB6 `Right(Variant, Length)` returning Variant (`rtcRightVar`, Ordinal 515).
#[no_mangle]
pub unsafe extern "system" fn rtcRightVar(
    dst: *mut Variant,
    str_var: *const Variant,
    len_var: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if str_var.is_null() {
        *dst = Variant::empty();
        return dst;
    }
    let s_orig = variant_to_string(&*str_var);
    let take_len = if !len_var.is_null() {
        (&*len_var).as_i32().unwrap_or(0).max(0) as usize
    } else {
        0
    };
    let u16_chars: Vec<u16> = s_orig.encode_utf16().collect();
    let count = take_len.min(u16_chars.len());
    let start_idx = u16_chars.len() - count;
    let b = SysAllocStringLen(u16_chars[start_idx..].as_ptr(), count as u32);
    *dst = Variant::from_bstr(b);
    dst
}

/// Implementation of VB6 `LCase(Variant)` returning Variant (`rtcLowerCaseVar`, Ordinal 518).
#[no_mangle]
pub unsafe extern "system" fn rtcLowerCaseVar(
    dst: *mut Variant,
    str_var: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if str_var.is_null() {
        *dst = Variant::empty();
        return dst;
    }
    let s = variant_to_string(&*str_var).to_lowercase();
    *dst = Variant::from_bstr(sys_alloc_string(&s));
    dst
}

/// Implementation of VB6 `UCase(Variant)` returning Variant (`rtcUpperCaseVar`, Ordinal 528).
#[no_mangle]
pub unsafe extern "system" fn rtcUpperCaseVar(
    dst: *mut Variant,
    str_var: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if str_var.is_null() {
        *dst = Variant::empty();
        return dst;
    }
    let s = variant_to_string(&*str_var).to_uppercase();
    *dst = Variant::from_bstr(sys_alloc_string(&s));
    dst
}

/// Implementation of VB6 `Trim(Variant)` returning Variant (`rtcTrimVar`, Ordinal 520).
#[no_mangle]
pub unsafe extern "system" fn rtcTrimVar(
    dst: *mut Variant,
    str_var: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if str_var.is_null() {
        *dst = Variant::empty();
        return dst;
    }
    let s = variant_to_string(&*str_var);
    let trimmed = s.trim();
    *dst = Variant::from_bstr(sys_alloc_string(trimmed));
    dst
}

/// Implementation of VB6 `LTrim$(String)` (`rtcLeftTrimBstr`, Ordinal 521).
#[no_mangle]
pub unsafe extern "system" fn rtcLeftTrimBstr(bstr: Bstr) -> Bstr {
    if bstr.is_null() {
        return sys_alloc_string("");
    }
    let s = bstr_to_string(bstr);
    let trimmed = s.trim_start();
    sys_alloc_string(trimmed)
}

/// Implementation of VB6 `LTrim(Variant)` returning Variant (`rtcLeftTrimVar`, Ordinal 522).
#[no_mangle]
pub unsafe extern "system" fn rtcLeftTrimVar(
    dst: *mut Variant,
    str_var: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if str_var.is_null() {
        *dst = Variant::empty();
        return dst;
    }
    let s = variant_to_string(&*str_var);
    let trimmed = s.trim_start();
    *dst = Variant::from_bstr(sys_alloc_string(trimmed));
    dst
}

/// Implementation of VB6 `RTrim$(String)` (`rtcRightTrimBstr`, Ordinal 523).
#[no_mangle]
pub unsafe extern "system" fn rtcRightTrimBstr(bstr: Bstr) -> Bstr {
    if bstr.is_null() {
        return sys_alloc_string("");
    }
    let s = bstr_to_string(bstr);
    let trimmed = s.trim_end();
    sys_alloc_string(trimmed)
}

/// Implementation of VB6 `RTrim(Variant)` returning Variant (`rtcRightTrimVar`, Ordinal 524).
#[no_mangle]
pub unsafe extern "system" fn rtcRightTrimVar(
    dst: *mut Variant,
    str_var: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if str_var.is_null() {
        *dst = Variant::empty();
        return dst;
    }
    let s = variant_to_string(&*str_var);
    let trimmed = s.trim_end();
    *dst = Variant::from_bstr(sys_alloc_string(trimmed));
    dst
}

/// Implementation of VB6 `Space(Variant)` returning Variant (`rtcSpaceVar`, Ordinal 526).
#[no_mangle]
pub unsafe extern "system" fn rtcSpaceVar(
    dst: *mut Variant,
    count_var: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let count = if !count_var.is_null() {
        (&*count_var).as_i32().unwrap_or(0).max(0) as usize
    } else {
        0
    };
    let s = " ".repeat(count);
    *dst = Variant::from_bstr(sys_alloc_string(&s));
    dst
}

/// Implementation of VB6 `String(number, character)` returning Variant (`rtcStringVar`, Ordinal 607).
#[no_mangle]
pub unsafe extern "system" fn rtcStringVar(
    dst: *mut Variant,
    number: *const Variant,
    character: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let count = if !number.is_null() {
        (&*number).as_i32().unwrap_or(0).max(0) as usize
    } else {
        0
    };
    let char_str = if !character.is_null() {
        variant_to_string(&*character)
    } else {
        String::new()
    };
    let ch = char_str.chars().next().unwrap_or(' ');
    let mut s = String::with_capacity(count);
    for _ in 0..count {
        s.push(ch);
    }
    *dst = Variant::from_bstr(sys_alloc_string(&s));
    dst
}

/// Implementation of VB6 `rtcVarBstrFromAnsi` (Ordinal 608).
#[no_mangle]
pub unsafe extern "system" fn rtcVarBstrFromAnsi(
    dst: *mut Variant,
    src_var: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let s = if !src_var.is_null() {
        variant_to_string(&*src_var)
    } else {
        String::new()
    };
    *dst = Variant::from_bstr(sys_alloc_string(&s));
    dst
}

/// Implementation of VB6 `CStr(Variant)` (`rtcVarStrFromVar`, Ordinal 613).
#[no_mangle]
pub unsafe extern "system" fn rtcVarStrFromVar(
    dst: *mut Variant,
    src: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let s = if !src.is_null() {
        variant_to_string(&*src)
    } else {
        String::new()
    };
    *dst = Variant::from_bstr(sys_alloc_string(&s));
    dst
}

/// Implementation of VB6 `Left(Variant, Length)` with integer length (`rtcLeftCharVar`, Ordinal 617).
#[no_mangle]
pub unsafe extern "system" fn rtcLeftCharVar(
    dst: *mut Variant,
    str_var: *const Variant,
    len: i32,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if str_var.is_null() {
        *dst = Variant::empty();
        return dst;
    }
    let s_orig = variant_to_string(&*str_var);
    let u16_chars: Vec<u16> = s_orig.encode_utf16().collect();
    let count = (len.max(0) as usize).min(u16_chars.len());
    let b = SysAllocStringLen(u16_chars.as_ptr(), count as u32);
    *dst = Variant::from_bstr(b);
    dst
}

/// Implementation of VB6 `Right(Variant, Length)` with integer length (`rtcRightCharVar`, Ordinal 619).
#[no_mangle]
pub unsafe extern "system" fn rtcRightCharVar(
    dst: *mut Variant,
    str_var: *const Variant,
    len: i32,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if str_var.is_null() {
        *dst = Variant::empty();
        return dst;
    }
    let s_orig = variant_to_string(&*str_var);
    let u16_chars: Vec<u16> = s_orig.encode_utf16().collect();
    let count = (len.max(0) as usize).min(u16_chars.len());
    let start_idx = u16_chars.len().saturating_sub(count);
    let b = SysAllocStringLen(u16_chars[start_idx..].as_ptr(), count as u32);
    *dst = Variant::from_bstr(b);
    dst
}

/// Implementation of VB6 `Mid(String, Start, Length)` returning BSTR (`rtcMidBstr`, Ordinal 628).
#[no_mangle]
pub unsafe extern "system" fn rtcMidBstr(
    str_val: Bstr,
    start: i32,
    len: *const Variant,
) -> Bstr {
    if str_val.is_null() || start <= 0 {
        return sys_alloc_string("");
    }
    let s = bstr_as_slice(str_val);
    let start_idx = (start - 1) as usize;
    if start_idx >= s.len() {
        return sys_alloc_string("");
    }
    let max_avail = s.len() - start_idx;
    let take_count = if !len.is_null() && (&*len).vt != VT_EMPTY {
        ((&*len).as_i32().unwrap_or(0).max(0) as usize).min(max_avail)
    } else {
        max_avail
    };
    SysAllocStringLen(s[start_idx..].as_ptr(), take_count as u32)
}

/// Implementation of VB6 `Mid(Variant, Start, Length)` returning Variant (`rtcMidVar`, Ordinal 629).
#[no_mangle]
pub unsafe extern "system" fn rtcMidVar(
    dst: *mut Variant,
    str_var: *const Variant,
    start_var: *const Variant,
    len_var: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if str_var.is_null() {
        *dst = Variant::empty();
        return dst;
    }
    let s_orig = variant_to_string(&*str_var);
    let start = if !start_var.is_null() {
        (&*start_var).as_i32().unwrap_or(1)
    } else {
        1
    };
    if start <= 0 {
        *dst = Variant::from_bstr(sys_alloc_string(""));
        return dst;
    }
    let u16_chars: Vec<u16> = s_orig.encode_utf16().collect();
    let start_idx = (start - 1) as usize;
    if start_idx >= u16_chars.len() {
        *dst = Variant::from_bstr(sys_alloc_string(""));
        return dst;
    }
    let max_avail = u16_chars.len() - start_idx;
    let take_count = if !len_var.is_null() && (&*len_var).vt != VT_EMPTY {
        ((&*len_var).as_i32().unwrap_or(0).max(0) as usize).min(max_avail)
    } else {
        max_avail
    };
    let b = SysAllocStringLen(u16_chars[start_idx..].as_ptr(), take_count as u32);
    *dst = Variant::from_bstr(b);
    dst
}

/// Implementation of VB6 `Mid(Variant, Start, Length)` with integer start (`rtcMidCharVar`, Ordinal 632).
#[no_mangle]
pub unsafe extern "system" fn rtcMidCharVar(
    dst: *mut Variant,
    str_var: *const Variant,
    start: i32,
    len: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if str_var.is_null() || start <= 0 {
        *dst = Variant::from_bstr(sys_alloc_string(""));
        return dst;
    }
    let s_orig = variant_to_string(&*str_var);
    let u16_chars: Vec<u16> = s_orig.encode_utf16().collect();
    let start_idx = (start - 1) as usize;
    if start_idx >= u16_chars.len() {
        *dst = Variant::from_bstr(sys_alloc_string(""));
        return dst;
    }
    let max_avail = u16_chars.len() - start_idx;
    let take_count = if !len.is_null() && (&*len).vt != VT_EMPTY {
        ((&*len).as_i32().unwrap_or(0).max(0) as usize).min(max_avail)
    } else {
        max_avail
    };
    let b = SysAllocStringLen(u16_chars[start_idx..].as_ptr(), take_count as u32);
    *dst = Variant::from_bstr(b);
    dst
}

/// Implementation of VB6 `InStr([start,] string1, string2[, compare])` returning Variant (`rtcInStr`, Ordinal 630).
#[no_mangle]
pub unsafe extern "system" fn rtcInStr(
    start: *const Variant,
    str1: *const Variant,
    str2: *const Variant,
    compare: *const Variant,
) -> Variant {
    let start_pos = if !start.is_null() {
        (&*start).as_i32().unwrap_or(1).max(1)
    } else {
        1
    };
    let s1 = if !str1.is_null() {
        variant_to_string(&*str1)
    } else {
        String::new()
    };
    let s2 = if !str2.is_null() {
        variant_to_string(&*str2)
    } else {
        String::new()
    };
    if s2.is_empty() {
        return Variant::from_i32(start_pos);
    }
    let u1: Vec<u16> = s1.encode_utf16().collect();
    let u2: Vec<u16> = s2.encode_utf16().collect();
    let start_idx = (start_pos - 1) as usize;
    if start_idx >= u1.len() {
        return Variant::from_i32(0);
    }
    let text_mode = if !compare.is_null() {
        (&*compare).as_i32().unwrap_or(0) == 1
    } else {
        false
    };
    let eq = |a: u16, b: u16| {
        if text_mode {
            let ca = char::from_u32(a as u32)
                .unwrap_or('\0')
                .to_lowercase()
                .next()
                .unwrap_or('\0') as u32;
            let cb = char::from_u32(b as u32)
                .unwrap_or('\0')
                .to_lowercase()
                .next()
                .unwrap_or('\0') as u32;
            ca == cb
        } else {
            a == b
        }
    };
    for i in start_idx..=u1.len().saturating_sub(u2.len()) {
        if u1[i..i + u2.len()]
            .iter()
            .zip(u2.iter())
            .all(|(&a, &b)| eq(a, b))
        {
            return Variant::from_i32((i + 1) as i32);
        }
    }
    Variant::from_i32(0)
}

/// Character/BSTR based `InStr` (`rtcInStrChar`, Ordinal 633).
#[no_mangle]
pub unsafe extern "system" fn rtcInStrChar(
    start: *const Variant,
    str1: Bstr,
    str2: Bstr,
    compare: *const Variant,
) -> i32 {
    let start_pos = if !start.is_null() {
        (&*start).as_i32().unwrap_or(1).max(1)
    } else {
        1
    };
    let comp = if !compare.is_null() {
        (&*compare).as_i32().unwrap_or(0)
    } else {
        0
    };
    crate::vba::__vbaInStr(start_pos, str1 as usize, str2 as usize, comp)
}

/// Implementation of VB6 `StrComp(string1, string2[, compare])` (`rtcCompareBstr`, Ordinal 649).
#[no_mangle]
pub unsafe extern "system" fn rtcCompareBstr(str1: Bstr, str2: Bstr, compare: i32) -> i32 {
    let s1 = if str1.is_null() {
        String::new()
    } else {
        bstr_to_string(str1)
    };
    let s2 = if str2.is_null() {
        String::new()
    } else {
        bstr_to_string(str2)
    };
    if compare == 1 {
        // Text compare (case-insensitive)
        let s1_lower = s1.to_lowercase();
        let s2_lower = s2.to_lowercase();
        match s1_lower.cmp(&s2_lower) {
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Equal => 0,
            core::cmp::Ordering::Greater => 1,
        }
    } else {
        // Binary compare (case-sensitive Unicode codepoints)
        match s1.cmp(&s2) {
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Equal => 0,
            core::cmp::Ordering::Greater => 1,
        }
    }
}

/// Implementation of VB6 `LenB(Variant)` (`rtcLenCharVar`, Ordinal 653).
#[no_mangle]
pub unsafe extern "system" fn rtcLenCharVar(var: *const Variant) -> i32 {
    if var.is_null() {
        return 0;
    }
    let v = &*var;
    if let Some(b) = v.as_bstr() {
        bstr_byte_len(b) as i32
    } else {
        let s = variant_to_string(v);
        (s.encode_utf16().count() * 2) as i32
    }
}

/// Implementation of VB6 `Len(Variant)` (`rtcLenVar`, Ordinal 654).
#[no_mangle]
pub unsafe extern "system" fn rtcLenVar(var: *const Variant) -> i32 {
    if var.is_null() {
        return 0;
    }
    let v = &*var;
    if let Some(b) = v.as_bstr() {
        bstr_len(b) as i32
    } else {
        let s = variant_to_string(v);
        s.chars().count() as i32
    }
}

/// Implementation of VB6 `AscB(string)` (`rtcByteValueBstr`, Ordinal 693).
#[no_mangle]
pub unsafe extern "system" fn rtcByteValueBstr(bstr: Bstr) -> u8 {
    if bstr.is_null() || bstr_byte_len(bstr) == 0 {
        return 0;
    }
    *(bstr as *const u8)
}

/// Implementation of VB6 `ChrB(byte)` (`rtcBstrFromByte`, Ordinal 694).
#[no_mangle]
pub unsafe extern "system" fn rtcBstrFromByte(byte_val: u8) -> Bstr {
    sys_alloc_string_byte_len(&[byte_val])
}

/// Implementation of VB6 `ChrB(byte)` returning Variant (`rtcVarBstrFromByte`, Ordinal 695).
#[no_mangle]
pub unsafe extern "system" fn rtcVarBstrFromByte(dst: *mut Variant, byte_val: u8) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let b = sys_alloc_string_byte_len(&[byte_val]);
    *dst = Variant::from_bstr(b);
    dst
}

/// Implementation of VB6 `Asc(string)` / `AscW(string)` (`rtcCharValueBstr`, Ordinal 696).
#[no_mangle]
pub unsafe extern "system" fn rtcCharValueBstr(bstr: Bstr) -> i16 {
    if bstr.is_null() || bstr_len(bstr) == 0 {
        return 0;
    }
    *(bstr as *const u16) as i16
}

/// Implementation of VB6 `Chr(charcode)` / `ChrW(charcode)` (`rtcBstrFromChar`, Ordinal 697).
#[no_mangle]
pub unsafe extern "system" fn rtcBstrFromChar(ch: i16) -> Bstr {
    let u = ch as u16;
    SysAllocStringLen(&u, 1)
}

/// Implementation of VB6 `Chr(charcode)` returning Variant (`rtcVarBstrFromChar`, Ordinal 698).
#[no_mangle]
pub unsafe extern "system" fn rtcVarBstrFromChar(dst: *mut Variant, ch: i16) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let u = ch as u16;
    let b = SysAllocStringLen(&u, 1);
    *dst = Variant::from_bstr(b);
    dst
}

// ----------------------------------------------------------------------------
// 2. Error Inspection & Coercion Routines
// ----------------------------------------------------------------------------

/// Implementation of VB6 `IsError(expression)` (`rtcIsError`, Ordinal 559).
#[no_mangle]
pub unsafe extern "system" fn rtcIsError(var: *const Variant) -> i16 {
    if var.is_null() {
        return 0;
    }
    if (&*var).vt == VT_ERROR {
        -1
    } else {
        0
    }
}

/// Implementation of VB6 `IsMissing(argument)` (`rtcIsMissing`, Ordinal 592).
#[no_mangle]
pub unsafe extern "system" fn rtcIsMissing(var: *const Variant) -> i16 {
    if var.is_null() {
        return -1;
    }
    let v = &*var;
    if v.vt == VT_EMPTY {
        return -1;
    }
    if v.vt == VT_ERROR {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&v.data[..4]);
        if u32::from_ne_bytes(bytes) == 0x80020004 {
            return -1;
        }
    }
    0
}

/// Implementation of VB6 `Erl` line number function (`rtcGetErl`, Ordinal 605).
#[no_mangle]
pub unsafe extern "system" fn rtcGetErl() -> i32 {
    0
}

/// Implementation of VB6 `CVErr(errornumber)` (`rtcCVErrFromVar`, Ordinal 643).
#[no_mangle]
pub unsafe extern "system" fn rtcCVErrFromVar(
    dst: *mut Variant,
    err_code: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let code = if !err_code.is_null() {
        (&*err_code).as_i32().unwrap_or(0)
    } else {
        0
    };
    let mut v = Variant::empty();
    v.vt = VT_ERROR;
    v.data[..4].copy_from_slice(&code.to_ne_bytes());
    *dst = v;
    dst
}

/// Implementation of VB6 `Error$(errornumber)` (`rtcBstrFromError`, Ordinal 651).
#[no_mangle]
pub unsafe extern "system" fn rtcBstrFromError(err_code: i32) -> Bstr {
    let desc = match err_code {
        5 => "Invalid procedure call or argument",
        6 => "Overflow",
        7 => "Out of memory",
        9 => "Subscript out of range",
        10 => "This array is fixed or temporarily locked",
        11 => "Division by zero",
        13 => "Type mismatch",
        14 => "Out of string space",
        53 => "File not found",
        58 => "File already exists",
        429 => "ActiveX component can't create object",
        438 => "Object doesn't support this property or method",
        _ => "Application-defined or object-defined error",
    };
    sys_alloc_string(desc)
}

/// Implementation of VB6 `Error(errornumber)` returning Variant (`rtcVarFromError`, Ordinal 652).
#[no_mangle]
pub unsafe extern "system" fn rtcVarFromError(dst: *mut Variant, err_code: i32) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let b = rtcBstrFromError(err_code);
    *dst = Variant::from_bstr(b);
    dst
}

unsafe fn extract_err_code(v: &Variant) -> i32 {
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&v.data[..4]);
    i32::from_ne_bytes(bytes)
}

/// Coerces Variant to BSTR handling VT_ERROR (`rtBstrFromErrVar`, Ordinal 634).
#[no_mangle]
pub unsafe extern "system" fn rtBstrFromErrVar(var: *const Variant) -> Bstr {
    if var.is_null() {
        return sys_alloc_string("");
    }
    let v = &*var;
    if v.vt == VT_ERROR {
        let code = extract_err_code(v);
        sys_alloc_string(&format!("Error {}", code))
    } else {
        let s = variant_to_string(v);
        sys_alloc_string(&s)
    }
}

/// Coerces Variant to Boolean handling VT_ERROR (`rtBoolFromErrVar`, Ordinal 635).
#[no_mangle]
pub unsafe extern "system" fn rtBoolFromErrVar(var: *const Variant) -> i16 {
    if var.is_null() {
        return 0;
    }
    let v = &*var;
    if extract_err_code(v) != 0 {
        -1
    } else {
        0
    }
}

/// Coerces Variant to Currency handling VT_ERROR (`rtCyFromErrVar`, Ordinal 636).
#[no_mangle]
pub unsafe extern "system" fn rtCyFromErrVar(var: *const Variant) -> i64 {
    if var.is_null() {
        return 0;
    }
    let v = &*var;
    if v.vt == VT_CY {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&v.data[..8]);
        i64::from_ne_bytes(bytes)
    } else {
        let code = extract_err_code(v) as i64;
        code * 10000
    }
}

/// Coerces Variant to Integer (i16) handling VT_ERROR (`rtI2FromErrVar`, Ordinal 637).
#[no_mangle]
pub unsafe extern "system" fn rtI2FromErrVar(var: *const Variant) -> i16 {
    if var.is_null() {
        return 0;
    }
    extract_err_code(&*var) as i16
}

/// Coerces Variant to Long (i32) handling VT_ERROR (`rtI4FromErrVar`, Ordinal 638).
#[no_mangle]
pub unsafe extern "system" fn rtI4FromErrVar(var: *const Variant) -> i32 {
    if var.is_null() {
        return 0;
    }
    extract_err_code(&*var)
}

/// Coerces Variant to Single (f32) handling VT_ERROR (`rtR4FromErrVar`, Ordinal 639).
#[no_mangle]
pub unsafe extern "system" fn rtR4FromErrVar(var: *const Variant) -> f32 {
    if var.is_null() {
        return 0.0;
    }
    extract_err_code(&*var) as f32
}

/// Coerces Variant to Double (f64) handling VT_ERROR (`rtR8FromErrVar`, Ordinal 640).
#[no_mangle]
pub unsafe extern "system" fn rtR8FromErrVar(var: *const Variant) -> f64 {
    if var.is_null() {
        return 0.0;
    }
    extract_err_code(&*var) as f64
}

/// Coerces Variant to Byte (u8) handling VT_ERROR (`rtUI1FromErrVar`, Ordinal 686).
#[no_mangle]
pub unsafe extern "system" fn rtUI1FromErrVar(var: *const Variant) -> u8 {
    if var.is_null() {
        return 0;
    }
    extract_err_code(&*var) as u8
}

// ----------------------------------------------------------------------------
// 3. Date / Time & System Statements
// ----------------------------------------------------------------------------

/// Sets system date from Variant (`rtcSetDateVar`, Ordinal 548).
#[no_mangle]
pub unsafe extern "system" fn rtcSetDateVar(_var: *const Variant) {
    // Headless / non-root: system clock modification is sandboxed.
}

/// Sets system date from BSTR (`rtcSetDateBstr`, Ordinal 549).
#[no_mangle]
pub unsafe extern "system" fn rtcSetDateBstr(_bstr: Bstr) {
    // Headless sandboxed no-op.
}

/// Sets system time from Variant (`rtcSetTimeVar`, Ordinal 550).
#[no_mangle]
pub unsafe extern "system" fn rtcSetTimeVar(_var: *const Variant) {
    // Headless sandboxed no-op.
}

/// Sets system time from BSTR (`rtcSetTimeBstr`, Ordinal 551).
#[no_mangle]
pub unsafe extern "system" fn rtcSetTimeBstr(_bstr: Bstr) {
    // Headless sandboxed no-op.
}

/// Returns current date as Variant Date (`rtcGetDateVar`, Ordinal 610).
#[no_mangle]
pub unsafe extern "system" fn rtcGetDateVar(dst: *mut Variant) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let now = std::time::SystemTime::now();
    let epoch = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let days_from_1970 = (epoch.as_secs() / 86400) as f64;
    let ole_date = days_from_1970 + 25569.0;
    let mut v = Variant::empty();
    v.vt = VT_DATE;
    v.data[..8].copy_from_slice(&ole_date.to_ne_bytes());
    *dst = v;
    dst
}

/// Returns current time as Variant Date (`rtcGetTimeVar`, Ordinal 612).
#[no_mangle]
pub unsafe extern "system" fn rtcGetTimeVar(dst: *mut Variant) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let now = std::time::SystemTime::now();
    let epoch = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs_of_day = (epoch.as_secs() % 86400) as f64;
    let frac = secs_of_day / 86400.0;
    let mut v = Variant::empty();
    v.vt = VT_DATE;
    v.data[..8].copy_from_slice(&frac.to_ne_bytes());
    *dst = v;
    dst
}

/// Extracts date f64 from Variant (`rtcDateFromVar`, Ordinal 641).
#[no_mangle]
pub unsafe extern "system" fn rtcDateFromVar(var: *const Variant) -> f64 {
    crate::rtc::extended::extract_date_f64(var)
}

/// Coerces Variant to VT_DATE (`rtcVarDateFromVar`, Ordinal 687).
#[no_mangle]
pub unsafe extern "system" fn rtcVarDateFromVar(
    dst: *mut Variant,
    var: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let d = crate::rtc::extended::extract_date_f64(var);
    let mut v = Variant::empty();
    v.vt = VT_DATE;
    v.data[..8].copy_from_slice(&d.to_ne_bytes());
    *dst = v;
    dst
}

/// Sets current calendar type (0 = Gregorian, 1 = Hijri) (`rtcSetCurrentCalendar`, Ordinal 699).
#[no_mangle]
pub unsafe extern "system" fn rtcSetCurrentCalendar(cal: i32) {
    CURRENT_CALENDAR.store(cal, Ordering::SeqCst);
}

/// Gets current calendar type (`rtcGetCurrentCalendar`, Ordinal 700).
#[no_mangle]
pub unsafe extern "system" fn rtcGetCurrentCalendar() -> i32 {
    CURRENT_CALENDAR.load(Ordering::SeqCst)
}

// ----------------------------------------------------------------------------
// 4. Format & Conversions
// ----------------------------------------------------------------------------

/// Implementation of VB6 `Hex(Variant)` returning Variant (`rtcHexVarFromVar`, Ordinal 573).
#[no_mangle]
pub unsafe extern "system" fn rtcHexVarFromVar(
    dst: *mut Variant,
    var: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let val = if !var.is_null() {
        (&*var).as_i32().unwrap_or(0)
    } else {
        0
    };
    let s = format!("{:X}", val);
    *dst = Variant::from_bstr(sys_alloc_string(&s));
    dst
}

/// Implementation of VB6 `Oct(Variant)` returning Variant (`rtcOctVarFromVar`, Ordinal 575).
#[no_mangle]
pub unsafe extern "system" fn rtcOctVarFromVar(
    dst: *mut Variant,
    var: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let val = if !var.is_null() {
        (&*var).as_i32().unwrap_or(0)
    } else {
        0
    };
    let s = format!("{:o}", val);
    *dst = Variant::from_bstr(sys_alloc_string(&s));
    dst
}

/// Clones Variant (`rtcVarFromVar`, Ordinal 642).
#[no_mangle]
pub unsafe extern "system" fn rtcVarFromVar(
    dst: *mut Variant,
    var: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if var.is_null() {
        *dst = Variant::empty();
        return dst;
    }
    *dst = *var;
    dst
}

/// Implementation of VB6 `Format(expression, format)` returning Variant (`rtcVarFromFormatVar`, Ordinal 660).
#[no_mangle]
pub unsafe extern "system" fn rtcVarFromFormatVar(
    dst: *mut Variant,
    val: *const Variant,
    fmt: *const Variant,
    first_day: *const Variant,
    first_week: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let fd = if !first_day.is_null() {
        (&*first_day).as_i32().unwrap_or(0)
    } else {
        0
    };
    let fw = if !first_week.is_null() {
        (&*first_week).as_i32().unwrap_or(0)
    } else {
        0
    };
    let b = crate::rtc::rtcBstrFromFormatVar(
        val as *mut Variant,
        fmt as *mut Variant,
        fd,
        fw,
    );
    *dst = Variant::from_bstr(b);
    dst
}

// ----------------------------------------------------------------------------
// 5. File, Directory & Environment
// ----------------------------------------------------------------------------

/// Implementation of VB6 `ChDrive` (`rtcChangeDrive`, Ordinal 533).
#[no_mangle]
pub unsafe extern "system" fn rtcChangeDrive(_drive: Bstr) {
    // Headless / Linux virtual drive no-op.
}

/// Implementation of VB6 `Input$(number, [#]filenumber)` (`rtcInputCount`, Ordinal 566).
#[no_mangle]
pub unsafe extern "system" fn rtcInputCount(count: i32, channel: i32) -> Bstr {
    use std::io::Read;
    if count <= 0 {
        return sys_alloc_string("");
    }
    if let Ok(mut tbl) = crate::vba::channel_table().lock() {
        if let Some(ch) = tbl.get_mut(&channel) {
            let mut buf = vec![0u8; count as usize];
            let mut total = 0;
            while total < count as usize {
                match ch.file.read(&mut buf[total..]) {
                    Ok(0) => break,
                    Ok(n) => total += n,
                    Err(_) => break,
                }
            }
            buf.truncate(total);
            let s = String::from_utf8_lossy(&buf);
            return sys_alloc_string(&s);
        }
    }
    sys_alloc_string("")
}

/// Implementation of VB6 `Input(number, [#]filenumber)` returning Variant (`rtcInputCountVar`, Ordinal 567).
#[no_mangle]
pub unsafe extern "system" fn rtcInputCountVar(
    dst: *mut Variant,
    count: *const Variant,
    channel: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let n = if !count.is_null() {
        (&*count).as_i32().unwrap_or(0)
    } else {
        0
    };
    let ch = if !channel.is_null() {
        (&*channel).as_i32().unwrap_or(1)
    } else {
        1
    };
    let b = rtcInputCount(n, ch);
    *dst = Variant::from_bstr(b);
    dst
}

/// Implementation of VB6 `FileDateTime(pathname)` (`rtcFileDateTime`, Ordinal 577).
#[no_mangle]
pub unsafe extern "system" fn rtcFileDateTime(path: Bstr) -> Variant {
    let p = if path.is_null() {
        String::new()
    } else {
        bstr_to_string(path)
    };
    let ole_date = if let Ok(metadata) = std::fs::metadata(&p) {
        if let Ok(mtime) = metadata.modified() {
            let dur = mtime
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            (dur.as_secs() as f64 / 86400.0) + 25569.0
        } else {
            45000.0
        }
    } else {
        45000.0
    };
    let mut v = Variant::empty();
    v.vt = VT_DATE;
    v.data[..8].copy_from_slice(&ole_date.to_ne_bytes());
    v
}

/// Implementation of VB6 `InputB$(number, [#]filenumber)` (`rtcInputCharCount`, Ordinal 620).
#[no_mangle]
pub unsafe extern "system" fn rtcInputCharCount(count: i32, channel: i32) -> Bstr {
    use std::io::Read;
    if count <= 0 {
        return sys_alloc_string("");
    }
    if let Ok(mut tbl) = crate::vba::channel_table().lock() {
        if let Some(ch) = tbl.get_mut(&channel) {
            let mut buf = vec![0u8; count as usize];
            let mut total = 0;
            while total < count as usize {
                match ch.file.read(&mut buf[total..]) {
                    Ok(0) => break,
                    Ok(n) => total += n,
                    Err(_) => break,
                }
            }
            buf.truncate(total);
            return sys_alloc_string_byte_len(&buf);
        }
    }
    sys_alloc_string("")
}

/// Implementation of VB6 `InputB(number, [#]filenumber)` returning Variant (`rtcInputCharCountVar`, Ordinal 621).
#[no_mangle]
pub unsafe extern "system" fn rtcInputCharCountVar(
    dst: *mut Variant,
    count: *const Variant,
    channel: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let n = if !count.is_null() {
        (&*count).as_i32().unwrap_or(0)
    } else {
        0
    };
    let ch = if !channel.is_null() {
        (&*channel).as_i32().unwrap_or(1)
    } else {
        1
    };
    let b = rtcInputCharCount(n, ch);
    *dst = Variant::from_bstr(b);
    dst
}

/// Returns host LCID (0x0409 = en-US) (`rtcGetHostLCID`, Ordinal 624).
#[no_mangle]
pub unsafe extern "system" fn rtcGetHostLCID() -> u32 {
    0x0409
}

/// Implementation of VB6 `CurDir(drive)` returning Variant (`rtcCurrentDir`, Ordinal 647).
#[no_mangle]
pub unsafe extern "system" fn rtcCurrentDir(
    dst: *mut Variant,
    drive: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let d = if !drive.is_null() {
        (&*drive).as_bstr().unwrap_or(core::ptr::null_mut())
    } else {
        core::ptr::null_mut()
    };
    let b = crate::rtc::rtcCurrentDirBstr(d);
    *dst = Variant::from_bstr(b);
    dst
}

/// Implementation of VB6 `Environ(expression)` returning Variant (`rtcEnvironVar`, Ordinal 666).
#[no_mangle]
pub unsafe extern "system" fn rtcEnvironVar(
    dst: *mut Variant,
    env: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if env.is_null() {
        *dst = Variant::from_bstr(sys_alloc_string(""));
        return dst;
    }
    let v = &*env;
    let b = if v.vt == VT_I2 || v.vt == VT_I4 {
        let idx = v.as_i32().unwrap_or(1);
        if let Some((k, val)) = std::env::vars().nth((idx.max(1) - 1) as usize) {
            sys_alloc_string(&format!("{}={}", k, val))
        } else {
            sys_alloc_string("")
        }
    } else {
        let name = variant_to_string(v);
        if let Ok(val) = std::env::var(&name) {
            sys_alloc_string(&val)
        } else {
            sys_alloc_string("")
        }
    };
    *dst = Variant::from_bstr(b);
    dst
}

// ----------------------------------------------------------------------------
// 6. Colors, GUI & Legacy Shims
// ----------------------------------------------------------------------------

/// Implementation of VB6 `QBColor(color)` (`rtcQBColor`, Ordinal 589).
///
/// Maps QuickBasic color index (0 to 15) to Win32 COLORREF format (`0x00bbggrr`).
#[no_mangle]
pub unsafe extern "system" fn rtcQBColor(color_index: i32) -> i32 {
    match color_index {
        0 => 0x000000,  // Black
        1 => 0x800000,  // Blue
        2 => 0x008000,  // Green
        3 => 0x808000,  // Cyan
        4 => 0x000080,  // Red
        5 => 0x800080,  // Magenta
        6 => 0x008080,  // Brown
        7 => 0xC0C0C0,  // Light Gray
        8 => 0x808080,  // Dark Gray
        9 => 0xFF0000,  // Bright Blue
        10 => 0x00FF00, // Bright Green
        11 => 0xFFFF00, // Bright Cyan
        12 => 0x0000FF, // Bright Red
        13 => 0xFF00FF, // Bright Magenta
        14 => 0x00FFFF, // Bright Yellow
        15 => 0xFFFFFF, // White
        _ => 0,
    }
}

/// Implementation of VB6 `MacID(string)` (`rtcMacId`, Ordinal 590).
#[no_mangle]
pub unsafe extern "system" fn rtcMacId(str_id: Bstr) -> i32 {
    if str_id.is_null() {
        return 0;
    }
    let s = bstr_to_string(str_id);
    let bytes = s.as_bytes();
    let mut val = 0i32;
    for &b in bytes.iter().take(4) {
        val = (val << 8) | (b as i32);
    }
    val
}

/// Headless implementation of VB6 `InputBox` (`rtcInputBox`, Ordinal 596).
#[no_mangle]
pub unsafe extern "system" fn rtcInputBox(
    dst: *mut Variant,
    _prompt: *const Variant,
    _title: *const Variant,
    default_val: *const Variant,
    _xpos: *const Variant,
    _ypos: *const Variant,
    _helpfile: *const Variant,
    _context: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if !default_val.is_null() && (&*default_val).vt != VT_EMPTY {
        *dst = *default_val;
    } else {
        *dst = Variant::from_bstr(sys_alloc_string(""));
    }
    dst
}

/// Headless implementation of VB6 `AppActivate` (`rtcAppActivate`, Ordinal 597).
#[no_mangle]
pub unsafe extern "system" fn rtcAppActivate(_title: *const Variant, _wait: *const Variant) {
    // Headless no-op
}

/// Headless implementation of VB6 `SendKeys` (`rtcSendKeys`, Ordinal 599).
#[no_mangle]
pub unsafe extern "system" fn rtcSendKeys(_keys: Bstr, _wait: *const Variant) {
    // Headless no-op
}

/// Implementation of VB6 `IMEStatus()` (`rtcIMEStatus`, Ordinal 615).
#[no_mangle]
pub unsafe extern "system" fn rtcIMEStatus() -> i32 {
    0 // 0 = vbIMENoOp / IME closed
}

/// AppleScript compatibility stub (`rtcAppleScript`, Ordinal 627).
#[no_mangle]
pub unsafe extern "system" fn rtcAppleScript(_script: Bstr) -> Bstr {
    sys_alloc_string("")
}
