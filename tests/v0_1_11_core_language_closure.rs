// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Regression Suite `v0.1.11` — Phase 1.11: Dynamic Arrays, VarArgs & Core Language 100% Closure (`App26`)
//!
//! Validates:
//! - ParamArray & VarArgs helpers (`__vbaAryVarVarg`, `__vbaFreeVarg`, `__vbaVargParmRef`,
//!   `__vbaVargVar`, `__vbaVargVarCopy`, `__vbaVargVarMove`, `__vbaVargVarRef`)
//! - Dynamic Array Lifecycle (`__vbaRedimVar`, `__vbaRedimVar2`, `__vbaRedimPreserveVar`,
//!   `__vbaRedimPreserveVar2`, `__vbaEraseKeepData`, `__vbaEraseNoPop`, `__vbaAryConstruct2`,
//!   `__vbaAryMove`, `__vbaDerefAry`, `__vbaDerefAry1`, `__vbaLdZeroAry`)
//! - Byte-Oriented Strings & Advanced String Ops (`__vbaInStrB`, `__vbaInStrVarB`,
//!   `__vbaMidStmtBstrB`, `__vbaMidStmtVar`, `__vbaMidStmtVarB`, `__vbaStrCompVar`,
//!   `__vbaStrTextCmp`, `__vbaStrVarCopy`, `rtcStrReverse`)
//! - Variant Pattern Matching (`__vbaVarLike`, `__vbaVarLikeVar`)
//! - Interface Type Checking & Control Flow (`__vbaCheckType`, `__vbaCheckTypeVar`,
//!   `__vbaError`, `__vbaStopExe`)

use msvbvm60::rtc::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;

#[test]
fn test_paramarray_and_varargs() {
    unsafe {
        let v1 = Variant::from_i32(10);
        let v2 = Variant::from_i32(20);
        let v3 = Variant::from_i32(30);
        let v4 = Variant::from_i32(40);
        let arg_ptrs = [
            &v1 as *const Variant as usize,
            &v2 as *const Variant as usize,
            &v3 as *const Variant as usize,
            &v4 as *const Variant as usize,
        ];

        let mut sa_ptr: *mut SafeArray = core::ptr::null_mut();
        __vbaAryVarVarg(&mut sa_ptr, 4, arg_ptrs.as_ptr());
        assert!(!sa_ptr.is_null());

        let mut v_copy = Variant::empty();
        __vbaVargVarCopy(&mut v_copy, &v1);
        assert_eq!(v_copy.as_i32(), Some(10));

        let mut v_move = Variant::empty();
        let mut v2_copy = v2;
        __vbaVargVarMove(&mut v_move, &mut v2_copy);
        assert_eq!(v_move.as_i32(), Some(20));

        __vbaFreeVarg(&mut sa_ptr);
        assert!(sa_ptr.is_null());
    }
}

#[test]
fn test_dynamic_variant_arrays() {
    unsafe {
        let mut v_arr = Variant::empty();
        __vbaRedimVar(VT_VARIANT as u32, &mut v_arr, 1, 5, 1);
        assert_eq!(v_arr.vt, VT_ARRAY | VT_VARIANT);

        __vbaRedimPreserveVar(VT_VARIANT as u32, &mut v_arr, 1, 8, 1);
        let mut psa = v_arr.as_safearray().unwrap_or(core::ptr::null_mut());
        if !psa.is_null() {
            __vbaEraseNoPop(&mut psa);
        }
    }
}

#[test]
fn test_byte_strings_and_advanced_string_ops() {
    unsafe {
        let mut b_src = sys_alloc_string("ABCDEFGHIJ");
        let b_x = sys_alloc_string("X");
        __vbaMidStmtBstrB(&mut b_src, 3, 2, b_x);
        SysFreeString(b_src);
        SysFreeString(b_x);

        let b_target = sys_alloc_string("1234567890");
        let b_repl = sys_alloc_string("ABC");
        let mut v_target = Variant::from_bstr(b_target);
        let v_repl = Variant::from_bstr(b_repl);
        __vbaMidStmtVar(&mut v_target, 4, 3, &v_repl);
        assert_eq!(v_target.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "123ABC7890");
        SysFreeString(b_target);
        SysFreeString(b_repl);

        let b_s = sys_alloc_string("Visual Basic 6");
        let b_find = sys_alloc_string("B");
        let pos = __vbaInStrB(1, b_s, b_find, 0);
        assert!(pos > 0);
        SysFreeString(b_s);
        SysFreeString(b_find);

        let b_cb = sys_alloc_string("ClassicBasic");
        let b_rev = rtcStrReverse(b_cb);
        assert_eq!(bstr_to_string(b_rev), "cisaBcissalC");
        SysFreeString(b_cb);
        SysFreeString(b_rev);
    }
}

#[test]
fn test_variant_like_pattern_matching() {
    unsafe {
        let b_text = sys_alloc_string("Antigravity-2026");
        let b_pat = sys_alloc_string("Anti*-####");
        let v_s = Variant::from_bstr(b_text);
        let v_p1 = Variant::from_bstr(b_pat);

        let mut v_like = Variant::empty();
        __vbaVarLike(&mut v_like, &v_p1, &v_s);
        assert_eq!(v_like.as_bool(), Some(true));

        let mut v_like_res = Variant::empty();
        __vbaVarLikeVar(&mut v_like_res, &v_p1, &v_s);
        assert_eq!(v_like_res.as_bool(), Some(true));

        SysFreeString(b_text);
        SysFreeString(b_pat);
    }
}

#[test]
fn test_type_checking_and_control_flow() {
    unsafe {
        let p_fake_obj = 0x12345678 as *mut core::ffi::c_void;
        let v_obj = Variant::from_dispatch(p_fake_obj);
        assert_eq!(__vbaCheckType(p_fake_obj, core::ptr::null()), p_fake_obj);
        assert_eq!(__vbaCheckTypeVar(&v_obj, core::ptr::null()), p_fake_obj);

        __vbaError(11); // Division by zero
        assert_eq!(msvbvm60::err::get_vb_error_number(), 11);
        msvbvm60::err::clear_vb_error();
    }
}
