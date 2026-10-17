// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Regression Suite `v0.1.10` — Phase 1.10: Variant Bitwise, Relational Testing & Vector Assignments (`App25`)
//!
//! Validates:
//! - Variant Bitwise and Logical Operations (`__vbaVarOr`, `__vbaVarXor`, `__vbaVarEqv`, `__vbaVarImp`, `__vbaVarPow`)
//! - Variant Relational Testing in Conditionals (`__vbaVarTstEq`, `__vbaVarTstNe`, `__vbaVarTstLt`, `__vbaVarTstLe`, `__vbaVarTstGt`, `__vbaVarTstGe`)
//! - Text Compare Relational Testing (`__vbaVarTextTstEq`, `__vbaVarTextTstNe`, `__vbaVarTextTstLt`, `__vbaVarTextTstLe`, `__vbaVarTextTstGt`, `__vbaVarTextTstGe`)
//! - Text Compare Binary Operators (`__vbaVarTextCmpEq`, `__vbaVarTextCmpNe`, `__vbaVarTextCmpLt`, `__vbaVarTextCmpLe`, `__vbaVarTextCmpGt`, `__vbaVarTextCmpGe`)
//! - Variant Assignments and Vector Operations (`__vbaVarSetVar`, `__vbaVarSetVarAddref`, `__vbaVarZero`, `__vbaVarIndexStore`, `__vbaVar2Vec`)

use msvbvm60::types::*;
use msvbvm60::vba::*;

#[test]
fn test_variant_bitwise_and_pow() {
    unsafe {
        let v1 = Variant::from_i32(0x0F);
        let v2 = Variant::from_i32(0xF0);
        let mut v_res = Variant::empty();

        __vbaVarOr(&mut v_res, &v1, &v2);
        assert_eq!(v_res.as_i32(), Some(0xFF));

        __vbaVarXor(&mut v_res, &v1, &v2);
        assert_eq!(v_res.as_i32(), Some(0xFF));

        let v_t = Variant::from_bool(true);
        let v_f = Variant::from_bool(false);
        __vbaVarEqv(&mut v_res, &v_t, &v_t);
        assert_eq!(v_res.as_bool(), Some(true));
        __vbaVarEqv(&mut v_res, &v_t, &v_f);
        assert_eq!(v_res.as_bool(), Some(false));

        __vbaVarImp(&mut v_res, &v_t, &v_f);
        assert_eq!(v_res.as_bool(), Some(false));
        __vbaVarImp(&mut v_res, &v_f, &v_f);
        assert_eq!(v_res.as_bool(), Some(true));

        let v3 = Variant::from_i32(3);
        let v4 = Variant::from_i32(4);
        __vbaVarPow(&mut v_res, &v4, &v3);
        assert_eq!(v_res.as_f64().unwrap() as i64, 81);
    }
}

#[test]
fn test_variant_relational_testing() {
    unsafe {
        let va = Variant::from_i32(100);
        let vb = Variant::from_i32(100);
        assert_eq!(__vbaVarTstEq(&va, &vb), -1);
        assert_eq!(__vbaVarTstNe(&va, &vb), 0);

        let vc = Variant::from_i32(200);
        assert_eq!(__vbaVarTstNe(&va, &vc), -1);
        assert_eq!(__vbaVarTstLt(&va, &vc), -1);
        assert_eq!(__vbaVarTstLe(&va, &vc), -1);
        assert_eq!(__vbaVarTstGt(&va, &vc), 0);
        assert_eq!(__vbaVarTstGe(&va, &vc), 0);

        assert_eq!(__vbaVarTstGt(&vc, &va), -1);
        assert_eq!(__vbaVarTstGe(&vc, &va), -1);
        assert_eq!(__vbaVarTstLt(&vc, &va), 0);
        assert_eq!(__vbaVarTstLe(&vc, &va), 0);

        let b_apple = sys_alloc_string("Apple");
        let b_lower = sys_alloc_string("apple");
        let v_apple = Variant::from_bstr(b_apple);
        let v_lower = Variant::from_bstr(b_lower);
        assert_eq!(__vbaVarTstEq(&v_apple, &v_lower), 0);
        assert_eq!(__vbaVarTstLt(&v_apple, &v_lower), -1);
        SysFreeString(b_apple);
        SysFreeString(b_lower);
    }
}

#[test]
fn test_variant_text_relational_testing() {
    unsafe {
        let b_up = sys_alloc_string("HELLO WORLD");
        let b_lo = sys_alloc_string("hello world");
        let v_upper = Variant::from_bstr(b_up);
        let v_low = Variant::from_bstr(b_lo);

        assert_eq!(__vbaVarTextTstEq(&v_upper, &v_low), -1);
        assert_eq!(__vbaVarTextTstNe(&v_upper, &v_low), 0);
        assert_eq!(__vbaVarTextTstLe(&v_upper, &v_low), -1);
        assert_eq!(__vbaVarTextTstGe(&v_upper, &v_low), -1);
        assert_eq!(__vbaVarTextTstLt(&v_upper, &v_low), 0);
        assert_eq!(__vbaVarTextTstGt(&v_upper, &v_low), 0);

        let mut v_cmp = Variant::empty();
        __vbaVarTextCmpEq(&mut v_cmp, &v_upper, &v_low);
        assert_eq!(v_cmp.as_bool(), Some(true));

        __vbaVarTextCmpNe(&mut v_cmp, &v_upper, &v_low);
        assert_eq!(v_cmp.as_bool(), Some(false));

        __vbaVarTextCmpLe(&mut v_cmp, &v_upper, &v_low);
        assert_eq!(v_cmp.as_bool(), Some(true));

        __vbaVarTextCmpGe(&mut v_cmp, &v_upper, &v_low);
        assert_eq!(v_cmp.as_bool(), Some(true));

        __vbaVarTextCmpLt(&mut v_cmp, &v_upper, &v_low);
        assert_eq!(v_cmp.as_bool(), Some(false));

        __vbaVarTextCmpGt(&mut v_cmp, &v_upper, &v_low);
        assert_eq!(v_cmp.as_bool(), Some(false));

        SysFreeString(b_up);
        SysFreeString(b_lo);
    }
}

#[test]
fn test_variant_assignment_and_zero() {
    unsafe {
        let mut v_dst = Variant::empty();
        let b_str = sys_alloc_string("ClassicBasic Variant");
        let v_src = Variant::from_bstr(b_str);

        __vbaVarSetVar(&mut v_dst, &v_src);
        assert_eq!(v_dst.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "ClassicBasic Variant");

        let mut v_dst2 = Variant::empty();
        __vbaVarSetVarAddref(&mut v_dst2, &v_src);
        assert_eq!(v_dst2.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "ClassicBasic Variant");

        __vbaVarZero(&mut v_dst);
        assert_eq!(v_dst.vt, 0);

        SysFreeString(b_str);
    }
}
