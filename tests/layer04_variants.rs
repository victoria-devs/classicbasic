//! Layer 04: Variant System & Dynamic Type Coercion Tests
//!
//! Granular tests for Variant creation, arithmetic operations (+, -, *, /, \, Mod),
//! concatenation (&), bitwise/logical operations (Not, And, Or, Xor),
//! comparison operators, and type tagging corresponding to App04-variants.

use msvbvm60::*;

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
