// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Types & Automation: VARIANT Automation Engine
//!
//! Automation VARIANT type tags, VT_BYREF multi-level pointer dereferencing,
//! relational comparison, bitwise algebra, and type coercion tables.

use msvbvm60::rtc::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;



#[test]
fn test_variant_constructors_and_tagging() {
    let v_i32 = Variant::from_i32(42);
    assert_eq!(v_i32.vt, VT_I4);
    assert_eq!(v_i32.as_i32(), Some(42));

    let v_i16 = Variant::from_i16(12);
    assert_eq!(v_i16.vt, VT_I2);

    let v_bool = Variant::from_bool(true);
    assert_eq!(v_bool.vt, VT_BOOL);
    assert_eq!(v_bool.as_bool(), Some(true));

    let v_f64 = Variant::from_f64(3.1415);
    assert_eq!(v_f64.vt, VT_R8);
    assert_eq!(v_f64.as_f64(), Some(3.1415));
}

#[test]
fn test_vba_var_add_numeric() {
    unsafe {
        let v1 = Variant::from_i32(40);
        let v2 = Variant::from_i32(2);
        let mut res = Variant::empty();
        __vbaVarAdd(&mut res, &v2, &v1);
        assert_eq!(res.as_i32(), Some(42));
    }
}

#[test]
fn test_vba_var_add_coercion_string_plus_numeric() {
    unsafe {
        let s_100 = sys_alloc_string("100");
        let v_s = Variant::from_bstr(s_100);
        let v_n = Variant::from_i32(25);
        let mut res = Variant::empty();
        __vbaVarAdd(&mut res, &v_n, &v_s);
        assert_eq!(res.as_i32(), Some(125));
        SysFreeString(s_100);
    }
}

#[test]
fn test_vba_var_sub_numeric() {
    unsafe {
        let v1 = Variant::from_i32(40);
        let v2 = Variant::from_i32(2);
        let mut res = Variant::empty();
        __vbaVarSub(&mut res, &v2, &v1);
        assert_eq!(res.as_i32(), Some(38));
    }
}

#[test]
fn test_vba_var_mul_numeric() {
    unsafe {
        let v1 = Variant::from_i32(25);
        let v2 = Variant::from_i32(4);
        let mut res = Variant::empty();
        __vbaVarMul(&mut res, &v2, &v1);
        assert_eq!(res.as_i32(), Some(100));
    }
}

#[test]
fn test_vba_var_div_floating_point() {
    unsafe {
        let v1 = Variant::from_i32(25);
        let v2 = Variant::from_i32(4);
        let mut res = Variant::empty();
        __vbaVarDiv(&mut res, &v2, &v1);
        assert_eq!(res.as_f64(), Some(6.25));
    }
}

#[test]
fn test_vba_var_idiv_integer_division() {
    unsafe {
        let v1 = Variant::from_i32(25);
        let v2 = Variant::from_i32(4);
        let mut res = Variant::empty();
        __vbaVarIdiv(&mut res, &v2, &v1);
        assert_eq!(res.as_i32(), Some(6));
    }
}

#[test]
fn test_vba_var_mod_modulo() {
    unsafe {
        let v1 = Variant::from_i32(25);
        let v2 = Variant::from_i32(4);
        let mut res = Variant::empty();
        __vbaVarMod(&mut res, &v2, &v1);
        assert_eq!(res.as_i32(), Some(1));
    }
}

#[test]
fn test_vba_var_cat_concatenation() {
    unsafe {
        let s_hello = sys_alloc_string("100");
        let v_s = Variant::from_bstr(s_hello);
        let v_n = Variant::from_i32(25);
        let mut res = Variant::empty();
        __vbaVarCat(&mut res, &v_n, &v_s);
        let b = __vbaStrErrVarCopy(&res);
        assert_eq!(bstr_to_string(b), "10025");
        SysFreeString(s_hello);
        SysFreeString(b);
    }
}

#[test]
fn test_vba_var_neg_unary_minus() {
    unsafe {
        let v = Variant::from_i32(42);
        let mut res = Variant::empty();
        __vbaVarNeg(&mut res, &v);
        assert_eq!(res.as_i32(), Some(-42));
    }
}

#[test]
fn test_vba_var_not_logical_negation() {
    unsafe {
        let v_true = Variant::from_bool(true);
        let mut res = Variant::empty();
        __vbaVarNot(&mut res, &v_true);
        assert_eq!(res.as_bool(), Some(false));
    }
}

#[test]
fn test_vba_var_and_bitwise() {
    unsafe {
        let v1 = Variant::from_i32(0xFF00);
        let v2 = Variant::from_i32(0x0FF0);
        let mut res = Variant::empty();
        __vbaVarAnd(&mut res, &v1, &v2);
        assert_eq!(res.as_i32(), Some(0x0F00));
    }
}

#[test]
fn test_vba_var_dup_clone() {
    unsafe {
        let v = Variant::from_i32(0xCAFE);
        let mut res = Variant::empty();
        __vbaVarDup(&mut res, &v);
        assert_eq!(res.as_i32(), Some(0xCAFE));
    }
}

#[test]
fn test_vba_var_cmp_gt_and_eq() {
    unsafe {
        let v_50 = Variant::from_i32(50);
        let v_30 = Variant::from_i32(30);
        let mut res_gt = Variant::empty();
        let mut res_eq = Variant::empty();

        __vbaVarCmpGt(&mut res_gt, &v_30, &v_50);
        __vbaVarCmpEq(&mut res_eq, &v_30, &v_50);

        assert_eq!(res_gt.as_bool(), Some(true));
        assert_eq!(res_eq.as_bool(), Some(false));
    }
}

#[test]
fn test_rtc_var_type_inspection() {
    unsafe {
        let v_i16 = Variant::from_i16(10);
        let v_b = Variant::from_bool(true);
        let s = sys_alloc_string("Text");
        let v_s = Variant::from_bstr(s);

        assert_eq!(rtcVarType(&v_i16), VT_I2);
        assert_eq!(rtcVarType(&v_b), VT_BOOL);
        assert_eq!(rtcVarType(&v_s), VT_BSTR);

        SysFreeString(s);
    }
}

#[test]
fn test_vba_bool_var_null_conversion() {
    unsafe {
        let v_true = Variant::from_bool(true);
        let v_false = Variant::from_bool(false);
        assert_eq!(__vbaBoolVarNull(&v_true), -1);
        assert_eq!(__vbaBoolVarNull(&v_false), 0);
    }
}

#[test]
fn test_variant_coercion_rules_distinguish_empty_and_null() {
    let var_empty = Variant::empty();
    let var_null = Variant::null();
    let var_five = Variant::from_i32(5);

    unsafe {
        // IsEmpty(A) is TRUE (-1) and IsEmpty(B) is FALSE (0)
        assert_eq!(rtcIsEmpty(&var_empty), -1);
        assert_eq!(rtcIsEmpty(&var_null), 0);

        // IsNull(A) is FALSE (0) and IsNull(B) is TRUE (-1)
        assert_eq!(rtcIsNull(&var_empty), 0);
        assert_eq!(rtcIsNull(&var_null), -1);

        // __vbaVarAdd(A, Variant(5)) yields Variant(5) (VT_EMPTY coerced to numeric 0)
        let mut res1 = Variant::empty();
        __vbaVarAdd(&mut res1, &var_five, &var_empty);
        assert_eq!(res1.as_i32(), Some(5));

        // __vbaVarAdd(B, Variant(5)) yields VT_NULL (VT_NULL propagates across arithmetic)
        let mut res2 = Variant::empty();
        __vbaVarAdd(&mut res2, &var_five, &var_null);
        assert_eq!(res2.vt, VT_NULL);
    }
}




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

#[test]
fn test_variant_multilevel_byref_dereferencing() {
    let mut base_long: i32 = 12345;

    let v_byref1 = Variant::byref_i32(&mut base_long);
    assert_eq!(v_byref1.vt, VT_BYREF | VT_I4);

    let v_byref2 = Variant::byref_variant(&v_byref1);
    assert_eq!(v_byref2.vt, VT_BYREF | VT_VARIANT);

    let v_byref3 = Variant::byref_variant(&v_byref2);
    assert_eq!(v_byref3.vt, VT_BYREF | VT_VARIANT);

    let deep_val = v_byref3.to_byval();
    assert_eq!(deep_val.vt, VT_I4);
    assert_eq!(deep_val.as_i32(), Some(12345));

    unsafe {
        core::ptr::write(&mut base_long, 99999);
    }
    let mutated_val = v_byref3.to_byval();
    assert_eq!(mutated_val.as_i32(), Some(99999));
}

#[test]
fn test_variant_relational_empty_null_coercion() {
    let v_empty = Variant::empty();
    let v_null = Variant::null();
    let v_zero = Variant::from_i32(0);
    let v_str_zero = Variant::from_bstr(msvbvm60::types::sys_alloc_string("0"));

    assert_eq!(v_empty.to_byval().as_i32(), Some(0));

    let mut v_res = Variant::empty();
    unsafe {
        msvbvm60::vba::__vbaVarTstEq(&v_null, &v_zero);
        msvbvm60::vba::__vbaVarAnd(&mut v_res, &v_null, &Variant::from_bool(true));
        assert_eq!(v_res.vt, VT_NULL, "Null And True must yield Null");
    }

    unsafe {
        msvbvm60::types::SysFreeString(v_str_zero.as_bstr().unwrap());
    }
}
