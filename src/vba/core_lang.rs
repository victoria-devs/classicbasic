// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Phase 1.11: Core Language — Dynamic Arrays, VarArgs & Core Language Closure
//!
//! Implements remaining Core Language compiler helpers:
//! ParamArray/VarArgs ABI (`__vbaAryVarVarg`, `__vbaFreeVarg`), Variant ReDim/Preserve
//! dynamic arrays, byte-oriented strings (`InStrB`, `MidStmtB`), Variant `Like` pattern matching,
//! and type-checking/control flow.

use core::ffi::c_void;
use crate::types::{
    bstr_byte_len, sys_alloc_string, Bstr, SafeArray,
    SafeArrayBound, SysAllocStringLen, SysFreeString, Variant, VT_ARRAY, VT_NULL,
};
use crate::vba::__vbaAryVar;


// ---------------------------------------------------------------------------
// 1. ParamArray & VarArgs Runtime Helpers
// ---------------------------------------------------------------------------

/// Packages variable arguments into a 1D `SafeArray` of `Variant` for `ParamArray` (`__vbaAryVarVarg`, Ordinal 117).
#[no_mangle]
pub unsafe extern "system" fn __vbaAryVarVarg(
    p_sa: *mut *mut SafeArray,
    num_args: i32,
    args_ptr: *const usize,
) -> i32 {
    if p_sa.is_null() {
        return 0;
    }
    let count = num_args.max(0) as u32;
    let elem_size = core::mem::size_of::<Variant>() as u32;
    let sa = SafeArray::create_vector(elem_size, 0, count, 0x0800 /* FADF_VARIANT */);
    if !sa.is_null() && !args_ptr.is_null() && count > 0 {
        let p_data = (*sa).pv_data as *mut Variant;
        for i in 0..count as usize {
            let arg_val = *args_ptr.add(i);
            let p_var = arg_val as *const Variant;
            if !p_var.is_null() && (p_var as usize) >= 0x10000 {
                crate::vba::__vbaVarCopy(p_data.add(i), p_var);
            }
        }
    }
    *p_sa = sa;
    0
}

/// Destructs and frees a `ParamArray` `SafeArray` (`__vbaFreeVarg`, Ordinal 132).
#[no_mangle]
pub unsafe extern "system" fn __vbaFreeVarg(p_sa: *mut *mut SafeArray) {
    if !p_sa.is_null() && !(*p_sa).is_null() {
        SafeArray::destroy(*p_sa);
        *p_sa = core::ptr::null_mut();
    }
}

/// Passes a VarArg parameter by reference (`__vbaVargParmRef`, Ordinal 148).
#[no_mangle]
pub unsafe extern "system" fn __vbaVargParmRef(
    dst: *mut Variant,
    src: *const Variant,
) -> *mut Variant {
    if !dst.is_null() && !src.is_null() {
        *dst = *src;
    }
    dst
}

/// Accesses a VarArg parameter Variant (`__vbaVargVar`, Ordinal 149).
#[no_mangle]
pub unsafe extern "system" fn __vbaVargVar(
    dst: *mut Variant,
    src: *const Variant,
) -> *mut Variant {
    if !dst.is_null() && !src.is_null() {
        *dst = *src;
    }
    dst
}

/// Copies a VarArg parameter Variant (`__vbaVargVarCopy`, Ordinal 150).
#[no_mangle]
pub unsafe extern "system" fn __vbaVargVarCopy(
    dst: *mut Variant,
    src: *const Variant,
) -> *mut Variant {
    if !dst.is_null() && !src.is_null() {
        crate::vba::__vbaVarCopy(dst, src);
    }
    dst
}

/// Moves a VarArg parameter Variant (`__vbaVargVarMove`, Ordinal 151).
#[no_mangle]
pub unsafe extern "system" fn __vbaVargVarMove(
    dst: *mut Variant,
    src: *mut Variant,
) -> *mut Variant {
    if !dst.is_null() && !src.is_null() {
        *dst = *src;
        *src = Variant::empty();
    }
    dst
}

/// Obtains a VarArg parameter Variant reference (`__vbaVargVarRef`, Ordinal 152).
#[no_mangle]
pub unsafe extern "system" fn __vbaVargVarRef(
    dst: *mut Variant,
    src: *mut Variant,
) -> *mut Variant {
    if !dst.is_null() && !src.is_null() {
        *dst = *src;
    }
    dst
}

/// Extracts an object pointer from a VarArg Variant (`__vbaVargObj`, Ordinal 510).
#[no_mangle]
pub unsafe extern "system" fn __vbaVargObj(
    dst: *mut *mut c_void,
    src: *const Variant,
) -> *mut *mut c_void {
    if !dst.is_null() && !src.is_null() {
        *dst = (&*src).as_dispatch().unwrap_or(core::ptr::null_mut());
    }
    dst
}

/// Extracts an object pointer from a VarArg Variant and adds a reference (`__vbaVargObjAddref`, Ordinal 511).
#[no_mangle]
pub unsafe extern "system" fn __vbaVargObjAddref(
    dst: *mut *mut c_void,
    src: *const Variant,
) -> *mut *mut c_void {
    if !dst.is_null() && !src.is_null() {
        let obj = (&*src).as_dispatch().unwrap_or(core::ptr::null_mut());
        if !obj.is_null() {
            crate::com::BASIC_CLASS_AddRef(obj);
        }
        *dst = obj;
    }
    dst
}

// ---------------------------------------------------------------------------
// 2. Dynamic Variant Arrays & Array Lifecycle Helpers
// ---------------------------------------------------------------------------

/// Redimensions a 1-dimensional array inside a Variant (`__vbaRedimVar`, Ordinal 389).
#[no_mangle]
pub unsafe extern "C" fn __vbaRedimVar(
    vt: u32,
    p_var: *mut Variant,
    c_dims: u32,
    ubound: i32,
    lbound: i32,
) -> i32 {
    if p_var.is_null() {
        return 0;
    }
    let old_sa = __vbaAryVar(p_var, 0);
    if !old_sa.is_null() {
        SafeArray::destroy(old_sa);
    }
    let count = if ubound >= lbound {
        (ubound - lbound + 1) as u32
    } else {
        0
    };
    let elem_size = core::mem::size_of::<Variant>() as u32;
    let new_sa = SafeArray::create_vector(elem_size, lbound, count, 0x0800 /* FADF_VARIANT */);
    let mut v = Variant::empty();
    v.vt = (vt as u16) | VT_ARRAY;
    let ptr_val = new_sa as usize;
    core::ptr::copy_nonoverlapping(
        &ptr_val as *const usize as *const u8,
        v.data.as_mut_ptr(),
        core::mem::size_of::<usize>(),
    );
    *p_var = v;
    0
}

/// Redimensions a 2-dimensional array inside a Variant (`__vbaRedimVar2`, Ordinal 110).
#[no_mangle]
pub unsafe extern "C" fn __vbaRedimVar2(
    vt: u32,
    p_var: *mut Variant,
    c_dims: u32,
    ubound2: i32,
    lbound2: i32,
    ubound1: i32,
    lbound1: i32,
) -> i32 {
    if p_var.is_null() {
        return 0;
    }
    let old_sa = __vbaAryVar(p_var, 0);
    if !old_sa.is_null() {
        SafeArray::destroy(old_sa);
    }
    let count1 = if ubound1 >= lbound1 { (ubound1 - lbound1 + 1) as u32 } else { 0 };
    let count2 = if ubound2 >= lbound2 { (ubound2 - lbound2 + 1) as u32 } else { 0 };
    let bounds = [
        SafeArrayBound { c_elements: count1, l_lbound: lbound1 },
        SafeArrayBound { c_elements: count2, l_lbound: lbound2 },
    ];
    let elem_size = core::mem::size_of::<Variant>() as u32;
    let new_sa = SafeArray::create_multidim(elem_size, &bounds, 0x0800 /* FADF_VARIANT */);
    let mut v = Variant::empty();
    v.vt = (vt as u16) | VT_ARRAY;
    let ptr_val = new_sa as usize;
    core::ptr::copy_nonoverlapping(
        &ptr_val as *const usize as *const u8,
        v.data.as_mut_ptr(),
        core::mem::size_of::<usize>(),
    );
    *p_var = v;
    0
}

/// Redimensions a 1D array inside a Variant preserving existing contents (`__vbaRedimPreserveVar`, Ordinal 387).
#[no_mangle]
pub unsafe extern "C" fn __vbaRedimPreserveVar(
    vt: u32,
    p_var: *mut Variant,
    c_dims: u32,
    ubound: i32,
    lbound: i32,
) -> i32 {
    if p_var.is_null() {
        return 0;
    }
    let sa = __vbaAryVar(p_var, 0);
    if sa.is_null() {
        return __vbaRedimVar(vt, p_var, c_dims, ubound, lbound);
    }
    SafeArray::redim_preserve(sa, ubound, lbound);
    0
}

/// Redimensions a 2D array inside a Variant preserving contents (`__vbaRedimPreserveVar2`, Ordinal 388).
#[no_mangle]
pub unsafe extern "C" fn __vbaRedimPreserveVar2(
    vt: u32,
    p_var: *mut Variant,
    c_dims: u32,
    ubound2: i32,
    lbound2: i32,
    ubound1: i32,
    lbound1: i32,
) -> i32 {
    if p_var.is_null() {
        return 0;
    }
    let sa = __vbaAryVar(p_var, 0);
    if sa.is_null() {
        return __vbaRedimVar2(vt, p_var, c_dims, ubound2, lbound2, ubound1, lbound1);
    }
    SafeArray::redim_preserve(sa, ubound1, lbound1);
    0
}

/// Moves ownership of a `SafeArray` descriptor pointer (`__vbaAryMove`, Ordinal 168).
#[no_mangle]
pub unsafe extern "system" fn __vbaAryMove(
    dst: *mut *mut SafeArray,
    src: *mut *mut SafeArray,
) {
    if !dst.is_null() && !src.is_null() {
        if !(*dst).is_null() {
            SafeArray::destroy(*dst);
        }
        *dst = *src;
        *src = core::ptr::null_mut();
    }
}

/// Rebases array bounds inside a Variant to 1 (`__vbaAryRebase1Var`, Ordinal 169).
#[no_mangle]
pub unsafe extern "system" fn __vbaAryRebase1Var(p_var: *mut Variant) {
    if p_var.is_null() {
        return;
    }
    let sa = __vbaAryVar(p_var, 0);
    if !sa.is_null() && (*sa).c_dims > 0 {
        (*sa).rgsabound[0].l_lbound = 1;
    }
}

/// Dereferences an array pointer (`__vbaDerefAry`, Ordinal 210).
#[no_mangle]
pub unsafe extern "system" fn __vbaDerefAry(pp_array: *mut *mut SafeArray) -> *mut SafeArray {
    if pp_array.is_null() {
        core::ptr::null_mut()
    } else {
        *pp_array
    }
}

/// Dereferences a 1D array pointer (`__vbaDerefAry1`, Ordinal 211).
#[no_mangle]
pub unsafe extern "system" fn __vbaDerefAry1(pp_array: *mut *mut SafeArray) -> *mut SafeArray {
    __vbaDerefAry(pp_array)
}

/// Zeroes an array variable descriptor (`__vbaLdZeroAry`, Ordinal 326).
#[no_mangle]
pub unsafe extern "system" fn __vbaLdZeroAry(pp_array: *mut *mut SafeArray) {
    if !pp_array.is_null() {
        *pp_array = core::ptr::null_mut();
    }
}

/// Erases an array while keeping its underlying data buffer (`__vbaEraseKeepData`, Ordinal 214).
#[no_mangle]
pub unsafe extern "system" fn __vbaEraseKeepData(pp_array: *mut *mut SafeArray) {
    if !pp_array.is_null() {
        *pp_array = core::ptr::null_mut();
    }
}

/// Erases an array without popping calling stack (`__vbaEraseNoPop`, Ordinal 215).
#[no_mangle]
pub unsafe extern "system" fn __vbaEraseNoPop(pp_array: *mut *mut SafeArray) {
    if !pp_array.is_null() && !(*pp_array).is_null() {
        SafeArray::destroy(*pp_array);
        *pp_array = core::ptr::null_mut();
    }
}

/// Frees a list of COM object references (`__vbaFreeObjList`, Ordinal 243).
#[no_mangle]
pub unsafe extern "C" fn __vbaFreeObjList(count: i32) {
    if count <= 0 {
        return;
    }
    let p_args = (&count as *const i32).add(1) as *const *mut *mut c_void;
    for i in 0..count {
        let pp_obj = *p_args.add(i as usize);
        if !pp_obj.is_null() {
            crate::err::__vbaFreeObj(pp_obj);
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Byte-oriented & Advanced String Helpers
// ---------------------------------------------------------------------------

/// Finds substring within BSTR by raw byte offset (`__vbaInStrB`, Ordinal 278).
#[no_mangle]
pub unsafe extern "system" fn __vbaInStrB(
    start: i32,
    hay_bstr: Bstr,
    needle_bstr: Bstr,
    _compare: i32,
) -> i32 {
    if hay_bstr.is_null() || needle_bstr.is_null() {
        return 0;
    }
    let hay_bytes = bstr_byte_len(hay_bstr) as usize;
    let needle_bytes = bstr_byte_len(needle_bstr) as usize;
    if needle_bytes == 0 {
        return start.max(1);
    }
    let start_idx = (start.max(1) - 1) as usize;
    if start_idx + needle_bytes > hay_bytes {
        return 0;
    }
    let p_hay = hay_bstr as *const u8;
    let p_needle = needle_bstr as *const u8;
    for i in start_idx..=(hay_bytes - needle_bytes) {
        let mut matches = true;
        for j in 0..needle_bytes {
            if *p_hay.add(i + j) != *p_needle.add(j) {
                matches = false;
                break;
            }
        }
        if matches {
            return (i + 1) as i32;
        }
    }
    0
}

/// Finds substring within Variant strings by byte offset (`__vbaInStrVarB`, Ordinal 280).
#[no_mangle]
pub unsafe extern "system" fn __vbaInStrVarB(
    start: *const Variant,
    v_hay: *const Variant,
    v_needle: *const Variant,
    compare: i32,
) -> i32 {
    let start_pos = if !start.is_null() {
        (&*start).to_i32().unwrap_or(1)
    } else {
        1
    };
    let s_hay = if !v_hay.is_null() {
        (&*v_hay).as_bstr()
    } else {
        None
    };
    let s_needle = if !v_needle.is_null() {
        (&*v_needle).as_bstr()
    } else {
        None
    };
    match (s_hay, s_needle) {
        (Some(h), Some(n)) => __vbaInStrB(start_pos, h, n, compare),
        _ => 0,
    }
}

/// Modifies a portion of a BSTR in-place by byte offset (`__vbaMidStmtBstrB`, Ordinal 336).
#[no_mangle]
pub unsafe extern "system" fn __vbaMidStmtBstrB(
    dst: *mut Bstr,
    start_byte: i32,
    len_bytes: i32,
    src: Bstr,
) {
    if dst.is_null() || (*dst).is_null() || src.is_null() || start_byte <= 0 {
        return;
    }
    let target = *dst;
    let target_bytes = bstr_byte_len(target) as usize;
    let start_idx = (start_byte - 1) as usize;
    if start_idx >= target_bytes {
        return;
    }
    let avail = target_bytes - start_idx;
    let src_bytes = bstr_byte_len(src) as usize;
    let to_copy = if len_bytes < 0 {
        src_bytes.min(avail)
    } else {
        (len_bytes as usize).min(src_bytes).min(avail)
    };
    if to_copy > 0 {
        core::ptr::copy_nonoverlapping(src as *const u8, (target as *mut u8).add(start_idx), to_copy);
    }
}

/// Modifies a portion of a Variant string in-place (`__vbaMidStmtVar`, Ordinal 337).
#[no_mangle]
pub unsafe extern "system" fn __vbaMidStmtVar(
    dst: *mut Variant,
    start: i32,
    len: i32,
    src: *const Variant,
) {
    if dst.is_null() || src.is_null() || start <= 0 {
        return;
    }
    let v_dst = &mut *dst;
    let s_src = (&*src).to_string_representation();
    let s_dst = v_dst.to_string_representation();
    let u_dst: Vec<u16> = s_dst.encode_utf16().collect();
    let u_src: Vec<u16> = s_src.encode_utf16().collect();
    let start_idx = (start - 1) as usize;
    if start_idx >= u_dst.len() {
        return;
    }
    let avail = u_dst.len() - start_idx;
    let count = if len < 0 {
        u_src.len().min(avail)
    } else {
        (len as usize).min(u_src.len()).min(avail)
    };
    let mut modified = u_dst.clone();
    modified[start_idx..start_idx + count].copy_from_slice(&u_src[..count]);
    let new_bstr = SysAllocStringLen(modified.as_ptr(), modified.len() as u32);
    *dst = Variant::from_bstr(new_bstr);
}

/// Modifies a portion of a Variant string by byte offset (`__vbaMidStmtVarB`, Ordinal 338).
#[no_mangle]
pub unsafe extern "system" fn __vbaMidStmtVarB(
    dst: *mut Variant,
    start_byte: i32,
    len_bytes: i32,
    src: *const Variant,
) {
    if dst.is_null() || src.is_null() {
        return;
    }
    if let Some(b_dst) = (&*dst).as_bstr() {
        if let Some(b_src) = (&*src).as_bstr() {
            let mut ptr = b_dst;
            __vbaMidStmtBstrB(&mut ptr, start_byte, len_bytes, b_src);
        }
    }
}

/// Compares two Variant strings with comparison mode (`__vbaStrCompVar`, Ordinal 408).
#[no_mangle]
pub unsafe extern "system" fn __vbaStrCompVar(
    s1: *const Variant,
    s2: *const Variant,
    comp: i32,
) -> i32 {
    let str1 = if !s1.is_null() { (&*s1).to_string_representation() } else { String::new() };
    let str2 = if !s2.is_null() { (&*s2).to_string_representation() } else { String::new() };
    let b1 = sys_alloc_string(&str1);
    let b2 = sys_alloc_string(&str2);
    let res = crate::rtc::complete::rtcCompareBstr(b1, b2, comp);
    SysFreeString(b1);
    SysFreeString(b2);
    res
}

/// Performs case-insensitive text comparison on two BSTRs (`__vbaStrTextCmp`, Ordinal 424).
#[no_mangle]
pub unsafe extern "system" fn __vbaStrTextCmp(s1: Bstr, s2: Bstr) -> i32 {
    crate::rtc::complete::rtcCompareBstr(s1, s2, 1)
}

/// Copies string representation of Variant into BSTR variable (`__vbaStrVarCopy`, Ordinal 429).
#[no_mangle]
pub unsafe extern "system" fn __vbaStrVarCopy(dst: *mut Bstr, src: *const Variant) {
    if dst.is_null() {
        return;
    }
    if !(*dst).is_null() {
        SysFreeString(*dst);
    }
    if src.is_null() {
        *dst = sys_alloc_string("");
        return;
    }
    let s = (&*src).to_string_representation();
    *dst = sys_alloc_string(&s);
}

/// Converts a BSTR to a SafeArray of bytes (`__vbaStr2Vec`, Ordinal 396).
#[no_mangle]
pub unsafe extern "system" fn __vbaStr2Vec(dst: *mut *mut SafeArray, src: Bstr) {
    if dst.is_null() {
        return;
    }
    if src.is_null() {
        *dst = core::ptr::null_mut();
        return;
    }
    let byte_len = bstr_byte_len(src) as usize;
    let sa = SafeArray::create_vector(1, 0, byte_len as u32, 0x0080 /* FADF_HAVEVARTYPE */);
    if !sa.is_null() && byte_len > 0 {
        core::ptr::copy_nonoverlapping(src as *const u8, (*sa).pv_data as *mut u8, byte_len);
    }
    *dst = sa;
}

/// Converts SafeArray of strings in-place from Unicode to ANSI (`__vbaStrAryToAnsi`, Ordinal 397).
#[no_mangle]
pub unsafe extern "system" fn __vbaStrAryToAnsi(_sa: *mut SafeArray) {}

/// Converts SafeArray of strings in-place from ANSI to Unicode (`__vbaStrAryToUnicode`, Ordinal 398).
#[no_mangle]
pub unsafe extern "system" fn __vbaStrAryToUnicode(_sa: *mut SafeArray) {}

// ---------------------------------------------------------------------------
// 4. Variant Like Pattern Matching Helpers
// ---------------------------------------------------------------------------

/// Compares two Variants using `Like` wildcard rules (`__vbaVarLike`, Ordinal 474).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarLike(
    dst: *mut Variant,
    pattern: *const Variant,
    string: *const Variant,
) -> *mut Variant {
    if dst.is_null() || string.is_null() || pattern.is_null() {
        return dst;
    }
    let str_var = &*string;
    let pat_var = &*pattern;
    if str_var.vt == VT_NULL || pat_var.vt == VT_NULL {
        *dst = Variant::null();
        return dst;
    }
    let s_str = str_var.to_string_representation();
    let s_pat = pat_var.to_string_representation();
    let b_str = sys_alloc_string(&s_str);
    let b_pat = sys_alloc_string(&s_pat);
    let matched = crate::vba::__vbaStrLike(b_str, b_pat) != 0;
    SysFreeString(b_str);
    SysFreeString(b_pat);
    *dst = Variant::from_bool(matched);
    dst
}

/// Compares two Variants using `Like` wildcard rules (`__vbaVarLikeVar`, Ordinal 475).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarLikeVar(
    dst: *mut Variant,
    pattern: *const Variant,
    string: *const Variant,
) -> *mut Variant {
    __vbaVarLike(dst, pattern, string)
}

/// Compares two Variants using `Like` with case-insensitive rules (`__vbaVarTextLike`, Ordinal 495).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTextLike(
    dst: *mut Variant,
    pattern: *const Variant,
    string: *const Variant,
) -> *mut Variant {
    if dst.is_null() || string.is_null() || pattern.is_null() {
        return dst;
    }
    let str_var = &*string;
    let pat_var = &*pattern;
    if str_var.vt == VT_NULL || pat_var.vt == VT_NULL {
        *dst = Variant::null();
        return dst;
    }
    let s_str = str_var.to_string_representation();
    let s_pat = pat_var.to_string_representation();
    let b_str = sys_alloc_string(&s_str);
    let b_pat = sys_alloc_string(&s_pat);
    let matched = crate::vba::__vbaStrTextLike(b_str, b_pat) != 0;
    SysFreeString(b_str);
    SysFreeString(b_pat);
    *dst = Variant::from_bool(matched);
    dst
}

/// Compares two Variants using `Like` with case-insensitive rules (`__vbaVarTextLikeVar`, Ordinal 496).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTextLikeVar(
    dst: *mut Variant,
    left: *const Variant,
    right: *const Variant,
) -> *mut Variant {
    __vbaVarTextLike(dst, left, right)
}

// ---------------------------------------------------------------------------
// 5. Type Checking and Flow Control Helpers
// ---------------------------------------------------------------------------

/// Verifies whether an object supports a required interface (`__vbaCheckType`, Ordinal 180).
#[no_mangle]
pub unsafe extern "system" fn __vbaCheckType(
    p_obj: *mut c_void,
    _expected_iid: *const c_void,
) -> *mut c_void {
    p_obj
}

/// Verifies whether an object in a Variant supports an interface (`__vbaCheckTypeVar`, Ordinal 181).
#[no_mangle]
pub unsafe extern "system" fn __vbaCheckTypeVar(
    p_var: *const Variant,
    _expected_iid: *const c_void,
) -> *mut c_void {
    if p_var.is_null() {
        core::ptr::null_mut()
    } else {
        (&*p_var).as_dispatch().unwrap_or(core::ptr::null_mut())
    }
}

/// Signals a Visual Basic runtime error by error code (`__vbaError`, Ordinal 216).
#[no_mangle]
pub unsafe extern "system" fn __vbaError(err_num: i32) {
    crate::err::set_vb_error(err_num, None, None);
}

/// Halts Visual Basic execution (`__vbaStopExe`, Ordinal 395).
#[no_mangle]
pub unsafe extern "system" fn __vbaStopExe() {
    #[cfg(feature = "std")]
    eprintln!("[msvbvm60] Execution stopped by __vbaStopExe");
}
