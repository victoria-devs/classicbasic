// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Phase 1.10: Core Language — Variant Bitwise, Relational Testing & Vector Assignments
//!
//! Implements 25 variant bitwise, logical, conditional relational (`__vbaVarTst*`),
//! text-mode comparison (`__vbaVarText*`), and variable/vector assignment runtime helpers.

use core::ffi::c_void;
use crate::types::{
    Variant, SafeArray, VT_BOOL, VT_NULL,
};
use crate::vba::__vbaAryVar;


// ---------------------------------------------------------------------------
// 1. Bitwise and Logical Operations on Variants
// ---------------------------------------------------------------------------

/// Performs bitwise/logical OR on two Variants (`__vbaVarOr`, Ordinal 480).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarOr(
    dst: *mut Variant,
    left: *const Variant,
    right: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let l = &*left;
    let r = &*right;
    if l.vt == VT_NULL && r.vt == VT_NULL {
        *dst = Variant::null();
        return dst;
    }
    if l.vt == VT_BOOL && r.vt == VT_BOOL {
        let bl = l.as_bool().unwrap_or(false);
        let br = r.as_bool().unwrap_or(false);
        *dst = Variant::from_bool(bl || br);
        return dst;
    }
    let li = l.as_i64().unwrap_or(0);
    let ri = r.as_i64().unwrap_or(0);
    *dst = Variant::from_i32((li | ri) as i32);
    dst
}

/// Performs bitwise/logical XOR on two Variants (`__vbaVarXor`, Ordinal 509).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarXor(
    dst: *mut Variant,
    left: *const Variant,
    right: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let l = &*left;
    let r = &*right;
    if l.vt == VT_NULL || r.vt == VT_NULL {
        *dst = Variant::null();
        return dst;
    }
    if l.vt == VT_BOOL && r.vt == VT_BOOL {
        let bl = l.as_bool().unwrap_or(false);
        let br = r.as_bool().unwrap_or(false);
        *dst = Variant::from_bool(bl ^ br);
        return dst;
    }
    let li = l.as_i64().unwrap_or(0);
    let ri = r.as_i64().unwrap_or(0);
    *dst = Variant::from_i32((li ^ ri) as i32);
    dst
}

/// Performs logical/bitwise Equivalence on two Variants (`__vbaVarEqv`, Ordinal 461).
/// In Visual Basic, `A Eqv B` is `Not (A Xor B)`.
#[no_mangle]
pub unsafe extern "system" fn __vbaVarEqv(
    dst: *mut Variant,
    left: *const Variant,
    right: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let l = &*left;
    let r = &*right;
    if l.vt == VT_NULL || r.vt == VT_NULL {
        *dst = Variant::null();
        return dst;
    }
    if l.vt == VT_BOOL && r.vt == VT_BOOL {
        let bl = l.as_bool().unwrap_or(false);
        let br = r.as_bool().unwrap_or(false);
        *dst = Variant::from_bool(!(bl ^ br));
        return dst;
    }
    let li = l.as_i64().unwrap_or(0);
    let ri = r.as_i64().unwrap_or(0);
    *dst = Variant::from_i32(!(li ^ ri) as i32);
    dst
}

/// Performs logical/bitwise Implication on two Variants (`__vbaVarImp`, Ordinal 467).
/// In Visual Basic, `A Imp B` is `(Not A) Or B`.
#[no_mangle]
pub unsafe extern "system" fn __vbaVarImp(
    dst: *mut Variant,
    left: *const Variant,
    right: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let l = &*left;
    let r = &*right;
    if l.vt == VT_NULL && r.vt == VT_NULL {
        *dst = Variant::null();
        return dst;
    }
    if l.vt == VT_BOOL && r.vt == VT_BOOL {
        let bl = l.as_bool().unwrap_or(false);
        let br = r.as_bool().unwrap_or(false);
        *dst = Variant::from_bool((!bl) || br);
        return dst;
    }
    let li = l.as_i64().unwrap_or(0);
    let ri = r.as_i64().unwrap_or(0);
    *dst = Variant::from_i32(((!li) | ri) as i32);
    dst
}

/// Computes exponentiation on two Variants (`__vbaVarPow`, Ordinal 481).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarPow(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let l_f = (&*left).as_f64().unwrap_or(0.0);
    let r_f = (&*right).as_f64().unwrap_or(0.0);
    let res = l_f.powf(r_f);
    if res.fract() == 0.0 && res >= (i32::MIN as f64) && res <= (i32::MAX as f64) {
        *dst = Variant::from_i32(res as i32);
    } else {
        *dst = Variant::from_f64(res);
    }
    dst
}

// ---------------------------------------------------------------------------
// 2. Conditional Relational Testing Helpers (__vbaVarTst*)
// ---------------------------------------------------------------------------

/// Compares two variants for equality in conditionals (`__vbaVarTstEq`, Ordinal 503).
/// Returns -1 (True) or 0 (False).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTstEq(left: *const Variant, right: *const Variant) -> i16 {
    if left.is_null() || right.is_null() {
        return 0;
    }
    let l = &*left;
    let r = &*right;
    if l.vt == VT_NULL || r.vt == VT_NULL {
        return 0;
    }
    if let (Some(fl), Some(fr)) = (l.as_f64(), r.as_f64()) {
        if fl == fr { -1 } else { 0 }
    } else {
        if l.to_string_representation() == r.to_string_representation() { -1 } else { 0 }
    }
}

/// Compares two variants for inequality in conditionals (`__vbaVarTstNe`, Ordinal 508).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTstNe(left: *const Variant, right: *const Variant) -> i16 {
    if left.is_null() || right.is_null() {
        return 0;
    }
    let l = &*left;
    let r = &*right;
    if l.vt == VT_NULL || r.vt == VT_NULL {
        return 0;
    }
    if let (Some(fl), Some(fr)) = (l.as_f64(), r.as_f64()) {
        if fl != fr { -1 } else { 0 }
    } else {
        if l.to_string_representation() != r.to_string_representation() { -1 } else { 0 }
    }
}

/// Compares if left < right in conditionals (`__vbaVarTstLt`, Ordinal 507).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTstLt(left: *const Variant, right: *const Variant) -> i16 {
    if left.is_null() || right.is_null() {
        return 0;
    }
    let l = &*left;
    let r = &*right;
    if l.vt == VT_NULL || r.vt == VT_NULL {
        return 0;
    }
    if let (Some(fl), Some(fr)) = (l.as_f64(), r.as_f64()) {
        if fl < fr { -1 } else { 0 }
    } else {
        if l.to_string_representation() < r.to_string_representation() { -1 } else { 0 }
    }
}

/// Compares if left <= right in conditionals (`__vbaVarTstLe`, Ordinal 506).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTstLe(left: *const Variant, right: *const Variant) -> i16 {
    if left.is_null() || right.is_null() {
        return 0;
    }
    let l = &*left;
    let r = &*right;
    if l.vt == VT_NULL || r.vt == VT_NULL {
        return 0;
    }
    if let (Some(fl), Some(fr)) = (l.as_f64(), r.as_f64()) {
        if fl <= fr { -1 } else { 0 }
    } else {
        if l.to_string_representation() <= r.to_string_representation() { -1 } else { 0 }
    }
}

/// Compares if left > right in conditionals (`__vbaVarTstGt`, Ordinal 505).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTstGt(left: *const Variant, right: *const Variant) -> i16 {
    if left.is_null() || right.is_null() {
        return 0;
    }
    let l = &*left;
    let r = &*right;
    if l.vt == VT_NULL || r.vt == VT_NULL {
        return 0;
    }
    if let (Some(fl), Some(fr)) = (l.as_f64(), r.as_f64()) {
        if fl > fr { -1 } else { 0 }
    } else {
        if l.to_string_representation() > r.to_string_representation() { -1 } else { 0 }
    }
}

/// Compares if left >= right in conditionals (`__vbaVarTstGe`, Ordinal 504).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTstGe(left: *const Variant, right: *const Variant) -> i16 {
    if left.is_null() || right.is_null() {
        return 0;
    }
    let l = &*left;
    let r = &*right;
    if l.vt == VT_NULL || r.vt == VT_NULL {
        return 0;
    }
    if let (Some(fl), Some(fr)) = (l.as_f64(), r.as_f64()) {
        if fl >= fr { -1 } else { 0 }
    } else {
        if l.to_string_representation() >= r.to_string_representation() { -1 } else { 0 }
    }
}

// ---------------------------------------------------------------------------
// 3. Option Compare Text Relational Testing Helpers (__vbaVarTextTst*)
// ---------------------------------------------------------------------------

fn var_text_compare(l: &Variant, r: &Variant) -> std::cmp::Ordering {
    if let (Some(fl), Some(fr)) = (l.as_f64(), r.as_f64()) {
        fl.partial_cmp(&fr).unwrap_or(std::cmp::Ordering::Equal)
    } else {
        let sl = l.to_string_representation().to_lowercase();
        let sr = r.to_string_representation().to_lowercase();
        sl.cmp(&sr)
    }
}

/// Text compare if left == right in conditionals (`__vbaVarTextTstEq`, Ordinal 497).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTextTstEq(left: *const Variant, right: *const Variant) -> i16 {
    if left.is_null() || right.is_null() {
        return 0;
    }
    if var_text_compare(&*left, &*right) == std::cmp::Ordering::Equal {
        -1
    } else {
        0
    }
}

/// Text compare if left != right in conditionals (`__vbaVarTextTstNe`, Ordinal 502).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTextTstNe(left: *const Variant, right: *const Variant) -> i16 {
    if left.is_null() || right.is_null() {
        return 0;
    }
    if var_text_compare(&*left, &*right) != std::cmp::Ordering::Equal {
        -1
    } else {
        0
    }
}

/// Text compare if left < right in conditionals (`__vbaVarTextTstLt`, Ordinal 501).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTextTstLt(left: *const Variant, right: *const Variant) -> i16 {
    if left.is_null() || right.is_null() {
        return 0;
    }
    if var_text_compare(&*left, &*right) == std::cmp::Ordering::Less {
        -1
    } else {
        0
    }
}

/// Text compare if left <= right in conditionals (`__vbaVarTextTstLe`, Ordinal 500).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTextTstLe(left: *const Variant, right: *const Variant) -> i16 {
    if left.is_null() || right.is_null() {
        return 0;
    }
    let ord = var_text_compare(&*left, &*right);
    if ord == std::cmp::Ordering::Less || ord == std::cmp::Ordering::Equal {
        -1
    } else {
        0
    }
}

/// Text compare if left > right in conditionals (`__vbaVarTextTstGt`, Ordinal 499).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTextTstGt(left: *const Variant, right: *const Variant) -> i16 {
    if left.is_null() || right.is_null() {
        return 0;
    }
    if var_text_compare(&*left, &*right) == std::cmp::Ordering::Greater {
        -1
    } else {
        0
    }
}

/// Text compare if left >= right in conditionals (`__vbaVarTextTstGe`, Ordinal 498).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTextTstGe(left: *const Variant, right: *const Variant) -> i16 {
    if left.is_null() || right.is_null() {
        return 0;
    }
    let ord = var_text_compare(&*left, &*right);
    if ord == std::cmp::Ordering::Greater || ord == std::cmp::Ordering::Equal {
        -1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// 4. Option Compare Text Expressions (__vbaVarTextCmp*)
// ---------------------------------------------------------------------------

/// Text compare expression left == right (`__vbaVarTextCmpEq`, Ordinal 489).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTextCmpEq(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let res = var_text_compare(&*left, &*right) == std::cmp::Ordering::Equal;
    *dst = Variant::from_bool(res);
    dst
}

/// Text compare expression left != right (`__vbaVarTextCmpNe`, Ordinal 494).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTextCmpNe(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let res = var_text_compare(&*left, &*right) != std::cmp::Ordering::Equal;
    *dst = Variant::from_bool(res);
    dst
}

/// Text compare expression left < right (`__vbaVarTextCmpLt`, Ordinal 493).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTextCmpLt(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let res = var_text_compare(&*left, &*right) == std::cmp::Ordering::Less;
    *dst = Variant::from_bool(res);
    dst
}

/// Text compare expression left <= right (`__vbaVarTextCmpLe`, Ordinal 492).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTextCmpLe(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let ord = var_text_compare(&*left, &*right);
    *dst = Variant::from_bool(ord == std::cmp::Ordering::Less || ord == std::cmp::Ordering::Equal);
    dst
}

/// Text compare expression left > right (`__vbaVarTextCmpGt`, Ordinal 491).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTextCmpGt(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let res = var_text_compare(&*left, &*right) == std::cmp::Ordering::Greater;
    *dst = Variant::from_bool(res);
    dst
}

/// Text compare expression left >= right (`__vbaVarTextCmpGe`, Ordinal 490).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarTextCmpGe(
    dst: *mut Variant,
    right: *const Variant,
    left: *const Variant,
) -> *mut Variant {
    if dst.is_null() || left.is_null() || right.is_null() {
        return dst;
    }
    let ord = var_text_compare(&*left, &*right);
    *dst = Variant::from_bool(ord == std::cmp::Ordering::Greater || ord == std::cmp::Ordering::Equal);
    dst
}

// ---------------------------------------------------------------------------
// 5. Variant Assignment, Zeroing & Array Element Storage Helpers
// ---------------------------------------------------------------------------

/// Assigns a Variant to another Variant (`__vbaVarSetVar`, Ordinal 486).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarSetVar(dst: *mut Variant, src: *const Variant) -> *mut Variant {
    if dst.is_null() || src.is_null() {
        return dst;
    }
    crate::vba::__vbaVarCopy(dst, src);
    dst
}

/// Assigns a Variant to another Variant with AddRef (`__vbaVarSetVarAddref`, Ordinal 487).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarSetVarAddref(dst: *mut Variant, src: *const Variant) -> *mut Variant {
    __vbaVarSetVar(dst, src)
}

/// Assigns an IDispatch object to a Variant (`__vbaVarSetObj`, Ordinal 482).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarSetObj(dst: *mut Variant, obj: *mut c_void) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    *dst = Variant::from_dispatch(obj);
    dst
}

/// Assigns an IUnknown pointer to a Variant (`__vbaVarSetUnk`, Ordinal 484).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarSetUnk(dst: *mut Variant, unk: *mut c_void) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    *dst = Variant::from_unknown(unk);
    dst
}

/// Assigns an IUnknown pointer to a Variant with AddRef (`__vbaVarSetUnkAddref`, Ordinal 485).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarSetUnkAddref(dst: *mut Variant, unk: *mut c_void) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if !unk.is_null() {
        crate::com::BASIC_CLASS_AddRef(unk);
    }
    *dst = Variant::from_unknown(unk);
    dst
}

/// Clears and zeroes a Variant to VT_EMPTY (`__vbaVarZero`, Ordinal 147).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarZero(dst: *mut Variant) {
    if !dst.is_null() {
        crate::vba::__vbaFreeVar(dst);
        *dst = Variant::empty();
    }
}

/// Stores an element value into a Variant array (`__vbaVarIndexStore`, Ordinal 471).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarIndexStore(
    p_arr_var: *mut Variant,
    src_val: *const Variant,
    _num_indices: i32,
    idx1: i32,
) {
    if p_arr_var.is_null() || src_val.is_null() {
        return;
    }
    let sa = __vbaAryVar(p_arr_var, 0);
    if sa.is_null() || (*sa).pv_data.is_null() {
        return;
    }
    let lbound = (*sa).rgsabound[0].l_lbound;
    let count = (*sa).rgsabound[0].c_elements as i32;
    if idx1 < lbound || idx1 >= lbound + count {
        return;
    }
    let offset = (idx1 - lbound) as usize;
    let elem_size = (*sa).cb_elements as usize;
    let elem_ptr = ((*sa).pv_data as *mut u8).add(offset * elem_size);
    if elem_size == core::mem::size_of::<Variant>() {
        crate::vba::__vbaVarCopy(elem_ptr as *mut Variant, src_val);
    } else if elem_size == 4 {
        let v_i32 = (&*src_val).as_i32().unwrap_or(0);
        *(elem_ptr as *mut i32) = v_i32;
    }
}

/// Stores an object reference into a Variant array (`__vbaVarIndexStoreObj`, Ordinal 472).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarIndexStoreObj(
    p_arr_var: *mut Variant,
    obj: *mut c_void,
    num_indices: i32,
    idx1: i32,
) {
    let v = Variant::from_dispatch(obj);
    __vbaVarIndexStore(p_arr_var, &v, num_indices, idx1);
}

/// Extracts SafeArray pointer from a Variant into an array variable (`__vbaVar2Vec`, Ordinal 448).
#[no_mangle]
pub unsafe extern "system" fn __vbaVar2Vec(
    dst_sa: *mut *mut SafeArray,
    src_var: *const Variant,
) {
    if dst_sa.is_null() || src_var.is_null() {
        return;
    }
    let sa = __vbaAryVar(src_var, 0);
    *dst_sa = sa;
}
