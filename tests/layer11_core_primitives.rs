//! Layer 11: Core Primitives & Extended Runtime Standard Library Tests
//!
//! Granular unit tests (one function, one test) for the built-in VB6 core primitives,
//! inspection functions, control structures, and coercions corresponding to App10-core-primitives.

use msvbvm60::*;

#[test]
fn test_rtc_type_name() {
    unsafe {
        let v_empty = Variant::empty();
        let s_empty = rtcTypeName(&v_empty);
        assert_eq!(bstr_to_string(s_empty), "Empty");
        SysFreeString(s_empty);

        let v_null = Variant::null();
        let s_null = rtcTypeName(&v_null);
        assert_eq!(bstr_to_string(s_null), "Null");
        SysFreeString(s_null);

        let v_i2 = Variant::from_i16(10);
        let s_i2 = rtcTypeName(&v_i2);
        assert_eq!(bstr_to_string(s_i2), "Integer");
        SysFreeString(s_i2);

        let v_i4 = Variant::from_i32(100);
        let s_i4 = rtcTypeName(&v_i4);
        assert_eq!(bstr_to_string(s_i4), "Long");
        SysFreeString(s_i4);

        let s_val = sys_alloc_string("Text");
        let v_str = Variant::from_bstr(s_val);
        let s_str = rtcTypeName(&v_str);
        assert_eq!(bstr_to_string(s_str), "String");
        SysFreeString(s_str);
        SysFreeString(s_val);
    }
}

#[test]
fn test_rtc_is_empty() {
    unsafe {
        let v_empty = Variant::empty();
        assert_eq!(rtcIsEmpty(&v_empty), -1);

        let v_not_empty = Variant::from_i32(42);
        assert_eq!(rtcIsEmpty(&v_not_empty), 0);
    }
}

#[test]
fn test_rtc_is_null() {
    unsafe {
        let v_null = Variant::null();
        assert_eq!(rtcIsNull(&v_null), -1);

        let v_empty = Variant::empty();
        assert_eq!(rtcIsNull(&v_empty), 0);
    }
}

#[test]
fn test_rtc_is_array() {
    unsafe {
        let sa = SafeArray::create_vector(2, 1, 2, FADF_HAVEVARTYPE);
        let mut v_arr = Variant::empty();
        v_arr.vt = VT_ARRAY | VT_I2;
        let ptr_bytes = (sa as usize).to_ne_bytes();
        v_arr.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);

        assert_eq!(rtcIsArray(&v_arr), -1);

        let v_single = Variant::from_i32(123);
        assert_eq!(rtcIsArray(&v_single), 0);

        SafeArray::destroy(sa);
    }
}

#[test]
fn test_rtc_is_numeric() {
    unsafe {
        let v_num = Variant::from_i32(1234);
        assert_eq!(rtcIsNumeric(&v_num), -1);

        let s_num = sys_alloc_string("  42.5  ");
        let v_str_num = Variant::from_bstr(s_num);
        assert_eq!(rtcIsNumeric(&v_str_num), -1);

        let s_not_num = sys_alloc_string("HelloWorld");
        let v_str_not = Variant::from_bstr(s_not_num);
        assert_eq!(rtcIsNumeric(&v_str_not), 0);

        SysFreeString(s_num);
        SysFreeString(s_not_num);
    }
}

#[test]
fn test_rtc_immediate_if() {
    unsafe {
        let mut dst = Variant::empty();
        let cond_t = Variant::from_bool(true);
        let cond_f = Variant::from_bool(false);
        let s_yes = sys_alloc_string("YES");
        let s_no = sys_alloc_string("NO");
        let v_yes = Variant::from_bstr(s_yes);
        let v_no = Variant::from_bstr(s_no);

        rtcImmediateIf(&mut dst, &cond_t, &v_yes, &v_no);
        assert_eq!(bstr_to_string(dst.as_bstr().unwrap()), "YES");

        rtcImmediateIf(&mut dst, &cond_f, &v_yes, &v_no);
        assert_eq!(bstr_to_string(dst.as_bstr().unwrap()), "NO");

        SysFreeString(s_yes);
        SysFreeString(s_no);
    }
}

#[test]
fn test_rtc_choose() {
    unsafe {
        let s_first = sys_alloc_string("First");
        let s_second = sys_alloc_string("Second");
        let s_third = sys_alloc_string("Third");
        let choices_items = [
            Variant::from_bstr(s_first),
            Variant::from_bstr(s_second),
            Variant::from_bstr(s_third),
        ];

        let sa = SafeArray::create_vector(
            core::mem::size_of::<Variant>() as u32,
            1,
            3,
            FADF_VARIANT | FADF_HAVEVARTYPE,
        );
        core::ptr::copy_nonoverlapping(choices_items.as_ptr(), (*sa).pv_data as *mut Variant, 3);
        let mut v_choices = Variant::empty();
        rtcArray(&mut v_choices, sa as *const core::ffi::c_void);

        let mut res = Variant::empty();
        rtcChoose(&mut res, 2.0, &v_choices);
        assert_eq!(bstr_to_string(res.as_bstr().unwrap()), "Second");

        rtcChoose(&mut res, 1.0, &v_choices);
        assert_eq!(bstr_to_string(res.as_bstr().unwrap()), "First");

        rtcChoose(&mut res, 99.0, &v_choices);
        assert_eq!(res.vt, VT_NULL);

        SysFreeString(s_first);
        SysFreeString(s_second);
        SysFreeString(s_third);
        SafeArray::destroy(sa);
    }
}

#[test]
fn test_rtc_command_bstr() {
    unsafe {
        let cmd = rtcCommandBstr();
        assert!(!cmd.is_null());
        SysFreeString(cmd);
    }
}

#[test]
fn test_rtc_ansi_value_bstr() {
    unsafe {
        let s = sys_alloc_string("ABC");
        assert_eq!(rtcAnsiValueBstr(s), 65);
        SysFreeString(s);
    }
}

#[test]
fn test_rtc_bstr_from_ansi() {
    unsafe {
        let s = rtcBstrFromAnsi(65);
        assert_eq!(bstr_to_string(s), "A");
        SysFreeString(s);
    }
}

#[test]
fn test_rtc_hex_bstr_from_var() {
    unsafe {
        let v = Variant::from_i32(255);
        let s = rtcHexBstrFromVar(&v);
        assert_eq!(bstr_to_string(s), "FF");
        SysFreeString(s);
    }
}

#[test]
fn test_rtc_oct_bstr_from_var() {
    unsafe {
        let v = Variant::from_i32(64);
        let s = rtcOctBstrFromVar(&v);
        assert_eq!(bstr_to_string(s), "100");
        SysFreeString(s);
    }
}

#[test]
fn test_rtc_r8_val_from_bstr() {
    unsafe {
        let s = sys_alloc_string("42.50 px");
        let val = rtcR8ValFromBstr(s);
        assert_eq!(val, 42.5);
        SysFreeString(s);
    }
}

#[test]
fn test_rtc_str_from_var() {
    unsafe {
        let v_pos = Variant::from_i32(42);
        let s_pos = rtcStrFromVar(&v_pos);
        assert_eq!(bstr_to_string(s_pos), " 42");
        SysFreeString(s_pos);

        let v_neg = Variant::from_i32(-42);
        let s_neg = rtcStrFromVar(&v_neg);
        assert_eq!(bstr_to_string(s_neg), "-42");
        SysFreeString(s_neg);
    }
}

#[test]
fn test_rtc_lower_case_bstr() {
    unsafe {
        let s = sys_alloc_string("CLASSIC BASIC");
        let lower = rtcLowerCaseBstr(s);
        assert_eq!(bstr_to_string(lower), "classic basic");
        SysFreeString(s);
        SysFreeString(lower);
    }
}

#[test]
fn test_rtc_replace() {
    unsafe {
        let s_haystack = sys_alloc_string("foo bar foo");
        let s_find = sys_alloc_string("foo");
        let s_repl = sys_alloc_string("baz");
        let res = rtcReplace(s_haystack, s_find, s_repl, 1, -1, 0);
        assert_eq!(bstr_to_string(res), "baz bar baz");
        SysFreeString(s_haystack);
        SysFreeString(s_find);
        SysFreeString(s_repl);
        SysFreeString(res);
    }
}

#[test]
fn test_rtc_in_str_rev() {
    unsafe {
        let s_path = sys_alloc_string("alpha/beta/gamma");
        let s_slash = sys_alloc_string("/");
        assert_eq!(rtcInStrRev(s_path, s_slash, -1, 0), 11);
        SysFreeString(s_path);
        SysFreeString(s_slash);
    }
}

#[test]
fn test_rtc_sqr() {
    unsafe {
        assert_eq!(rtcSqr(144.0), 12.0);
        assert_eq!(rtcSqr(0.0), 0.0);
    }
}

#[test]
fn test_rtc_round() {
    unsafe {
        let mut dst = Variant::empty();
        let val = Variant::from_f64(3.14159);
        rtcRound(&mut dst, &val, 2);
        assert_eq!(dst.as_f64().unwrap(), 3.14);
    }
}

#[test]
fn test_vba_var_int() {
    unsafe {
        let mut dst = Variant::empty();
        let v_neg = Variant::from_f64(-4.8);
        __vbaVarInt(&mut dst, &v_neg);
        assert_eq!(dst.as_i32().unwrap(), -5);
    }
}

#[test]
fn test_vba_var_fix() {
    unsafe {
        let mut dst = Variant::empty();
        let v_neg = Variant::from_f64(-4.8);
        __vbaVarFix(&mut dst, &v_neg);
        assert_eq!(dst.as_i32().unwrap(), -4);
    }
}

#[test]
fn test_vba_var_abs() {
    unsafe {
        let mut dst = Variant::empty();
        let v_neg = Variant::from_f64(-123.45);
        __vbaVarAbs(&mut dst, &v_neg);
        assert_eq!(dst.as_f64().unwrap(), 123.45);
    }
}

#[test]
fn test_vba_fp_fix() {
    unsafe {
        assert_eq!(__vbaFPFix(-4.8), -4.0);
        assert_eq!(__vbaFPFix(4.8), 4.0);
    }
}

#[test]
fn test_vba_str_bool() {
    unsafe {
        let s_true = __vbaStrBool(-1);
        assert_eq!(bstr_to_string(s_true), "True");
        SysFreeString(s_true);

        let s_false = __vbaStrBool(0);
        assert_eq!(bstr_to_string(s_false), "False");
        SysFreeString(s_false);
    }
}

#[test]
fn test_vba_bool_var() {
    unsafe {
        let v_true = Variant::from_bool(true);
        assert_eq!(__vbaBoolVar(&v_true), -1);

        let v_false = Variant::from_bool(false);
        assert_eq!(__vbaBoolVar(&v_false), 0);
    }
}

#[test]
fn test_vba_bool_str() {
    unsafe {
        let s_true = sys_alloc_string("True");
        assert_eq!(__vbaBoolStr(s_true), -1);
        SysFreeString(s_true);

        let s_false = sys_alloc_string("False");
        assert_eq!(__vbaBoolStr(s_false), 0);
        SysFreeString(s_false);
    }
}

#[test]
fn test_vba_cy_str_and_str_cy() {
    unsafe {
        let s_in = sys_alloc_string("99.95");
        let cy = __vbaCyStr(s_in);
        assert_eq!(cy, 999500);

        let s_out = __vbaStrCy(cy);
        assert_eq!(bstr_to_string(s_out), "99.95");

        SysFreeString(s_in);
        SysFreeString(s_out);
    }
}

#[test]
fn test_vba_cy_var() {
    unsafe {
        let v = Variant::from_f64(10.5);
        assert_eq!(__vbaCyVar(&v), 105000);
    }
}

#[test]
fn test_vba_i2_var() {
    unsafe {
        let v = Variant::from_i32(42);
        assert_eq!(__vbaI2Var(&v), 42);
    }
}

#[test]
fn test_vba_i4_var() {
    unsafe {
        let v = Variant::from_i32(100000);
        assert_eq!(__vbaI4Var(&v), 100000);
    }
}

#[test]
fn test_vba_i2_str() {
    unsafe {
        let s = sys_alloc_string("123");
        assert_eq!(__vbaI2Str(s), 123);
        SysFreeString(s);
    }
}

#[test]
fn test_vba_i4_str() {
    unsafe {
        let s = sys_alloc_string("456789");
        assert_eq!(__vbaI4Str(s), 456789);
        SysFreeString(s);
    }
}

#[test]
fn test_vba_r4_var() {
    unsafe {
        let v = Variant::from_f32(3.14);
        assert!((__vbaR4Var(&v) - 3.14).abs() < 1e-5);
    }
}

#[test]
fn test_vba_r8_var() {
    unsafe {
        let v = Variant::from_f64(2.718281828);
        assert!((__vbaR8Var(&v) - 2.718281828).abs() < 1e-7);
    }
}

#[test]
fn test_vba_end() {
    unsafe {
        __vbaEnd();
    }
}
