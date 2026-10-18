// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Runtime Library: Primitive Coercions & Loop Mechanics
//!
//! Numeric type conversions, overflow trapping (Err 6), signed coercions,
//! and typed/dynamic loop iterators (For Next, Gosub, On Go).

use msvbvm60::err::*;
use msvbvm60::rtc::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;



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

#[test]
fn test_instrrev_with_negative_one_starts_from_end_of_string() {
    unsafe {
        // GIVEN string "SYSTEM.CONFIG.INI"
        let str_haystack = sys_alloc_string("SYSTEM.CONFIG.INI");
        let str_dot = sys_alloc_string(".");

        // WHEN InStrRev("SYSTEM.CONFIG.INI", ".", -1) is evaluated
        let pos = rtcInStrRev(str_haystack, str_dot, -1, 0);

        // THEN the returned 1-based index is 14 (identifying the final dot)
        assert_eq!(pos, 14);

        SysFreeString(str_haystack);
        SysFreeString(str_dot);
    }
}

#[test]
fn test_val_stops_at_first_non_digit_and_parses_hex_prefixes() {
    unsafe {
        // GIVEN inputs "  104.55 USD", "&HFF", and "12 34"
        let s_usd = sys_alloc_string("  104.55 USD");
        let s_hex = sys_alloc_string("&HFF");
        let s_spaces = sys_alloc_string("12 34");

        // WHEN Val() is evaluated
        // THEN "  104.55 USD" returns 104.55
        let val_usd = rtcR8ValFromBstr(s_usd);
        assert!((val_usd - 104.55).abs() < 1e-6);

        // AND "&HFF" returns 255
        let val_hex = rtcR8ValFromBstr(s_hex);
        assert_eq!(val_hex, 255.0);

        // AND "12 34" returns 1234 (spaces stripped internally)
        let val_spaces = rtcR8ValFromBstr(s_spaces);
        assert_eq!(val_spaces, 1234.0);

        SysFreeString(s_usd);
        SysFreeString(s_hex);
        SysFreeString(s_spaces);
    }
}

#[test]
fn test_format_numeric_zero_and_hash_padding() {
    unsafe {
        // GIVEN a numeric input 42
        let v42 = Variant::from_i32(42);
        let b_zeros = sys_alloc_string("00000");
        let mask_zeros = Variant::from_bstr(b_zeros);
        let res1 = rtcBstrFromFormatVar(&v42, &mask_zeros, 0, 0);
        assert_eq!(bstr_to_string(res1), "00042");
        SysFreeString(res1);
        SysFreeString(b_zeros);

        // WHEN Format$(123.456, "#,##0.00") is evaluated
        let v_float = Variant::from_f64(123.456);
        let b_comma = sys_alloc_string("#,##0.00");
        let mask_comma = Variant::from_bstr(b_comma);
        let res2 = rtcBstrFromFormatVar(&v_float, &mask_comma, 0, 0);
        assert_eq!(bstr_to_string(res2), "123.46");
        SysFreeString(res2);
        SysFreeString(b_comma);

        // WHEN Format$(0, "#") is evaluated
        let v_zero = Variant::from_i32(0);
        let b_hash = sys_alloc_string("#");
        let mask_hash = Variant::from_bstr(b_hash);
        let res3 = rtcBstrFromFormatVar(&v_zero, &mask_hash, 0, 0);
        assert_eq!(bstr_to_string(res3), "");
        SysFreeString(res3);
        SysFreeString(b_hash);
    }
}

#[test]
fn test_format_datetime_standard_masks() {
    unsafe {
        // GIVEN a serialized date representing 2026-10-15 14:30:05
        // (46310 days since Dec 30 1899 + (14*3600 + 30*60 + 5)/86400 = 46310.60422453703)
        let date_ole = 46310.60422453703f64;
        let mut v_date = Variant::from_f64(date_ole);
        v_date.vt = msvbvm60::types::VT_DATE;

        // WHEN Format$(date, "yyyy-mm-dd hh:nn:ss") is evaluated
        let b_mask1 = sys_alloc_string("yyyy-mm-dd hh:nn:ss");
        let mask1 = Variant::from_bstr(b_mask1);
        let res1 = rtcBstrFromFormatVar(&v_date, &mask1, 0, 0);
        assert_eq!(bstr_to_string(res1), "2026-10-15 14:30:05");
        SysFreeString(res1);
        SysFreeString(b_mask1);

        // WHEN Format$(date, "dd/mm/yyyy") is evaluated
        let b_mask2 = sys_alloc_string("dd/mm/yyyy");
        let mask2 = Variant::from_bstr(b_mask2);
        let res2 = rtcBstrFromFormatVar(&v_date, &mask2, 0, 0);
        assert_eq!(bstr_to_string(res2), "15/10/2026");
        SysFreeString(res2);
        SysFreeString(b_mask2);
    }
}




#[test]
fn test_typed_for_next_checks_integer_long_single_double() {
    unsafe {
        // 1. Integer (i16) For..Next: 1 to 10 step 2
        let mut i2_val: i16 = 1;
        let mut i2_count = 0;
        let mut i2_sum = 0;
        while __vbaI2ForNextCheck(i2_val, 10, 2) != 0 {
            i2_sum += i2_val as i32;
            i2_count += 1;
            i2_val += 2;
        }
        assert_eq!(i2_count, 5);
        assert_eq!(i2_sum, 25); // 1 + 3 + 5 + 7 + 9

        // 2. Long (i32) For..Next negative step: 100 to 50 step -10
        let mut i4_val: i32 = 100;
        let mut i4_count = 0;
        while __vbaI4ForNextCheck(i4_val, 50, -10) != 0 {
            i4_count += 1;
            i4_val -= 10;
        }
        assert_eq!(i4_count, 6); // 100, 90, 80, 70, 60, 50

        // Zero-trip loop
        assert_eq!(__vbaI4ForNextCheck(10, 1, 1), 0);
        assert_eq!(__vbaI4ForNextCheck(1, 10, -1), 0);

        // 3. Single (f32) For..Next: 0.5 to 2.5 step 0.5
        let mut r4_val: f32 = 0.5;
        let mut r4_count = 0;
        while __vbaR4ForNextCheck(r4_val, 2.5, 0.5) != 0 {
            r4_count += 1;
            r4_val += 0.5;
        }
        assert_eq!(r4_count, 5);

        // 4. Double (f64) For..Next: 10.0 to 2.0 step -2.0
        let mut r8_val: f64 = 10.0;
        let mut r8_count = 0;
        while __vbaR8ForNextCheck(r8_val, 2.0, -2.0) != 0 {
            r8_count += 1;
            r8_val -= 2.0;
        }
        assert_eq!(r8_count, 5);
    }
}

#[test]
fn test_currency_for_next_init_and_iteration() {
    unsafe {
        // Currency 10.0000@ to 25.0000@ step 5.0000@
        let mut cy_counter: i64 = 0;
        let cy_start = 100_000i64; // 10.0000
        let cy_end = 250_000i64;   // 25.0000
        let cy_step = 50_000i64;   // 5.0000

        let mut cy_count = 0;
        let mut cy_total = 0i64;

        let should_run = __vbaCyForInit(&mut cy_counter, cy_start, cy_end, cy_step);
        assert_ne!(should_run, 0);
        assert_eq!(cy_counter, cy_start);

        loop {
            cy_total += cy_counter;
            cy_count += 1;
            if __vbaCyForNext(&mut cy_counter, cy_end, cy_step) == 0 {
                break;
            }
        }

        assert_eq!(cy_count, 4);
        assert_eq!(cy_total, 700_000i64); // (10 + 15 + 20 + 25) * 10000

        // Zero-trip Currency loop
        let mut zero_counter: i64 = 0;
        assert_eq!(__vbaCyForInit(&mut zero_counter, 100_000, 10_000, 10_000), 0);
    }
}

#[test]
fn test_variant_for_next_init_and_iteration() {
    unsafe {
        // Variant loop: 1 to 3 Step 0.5
        let mut counter = Variant::empty();
        let mut temp_step = Variant::empty();
        let mut temp_end = Variant::empty();

        let v_start = Variant::from_i32(1);
        let v_end = Variant::from_f64(3.0);
        let v_step = Variant::from_f64(0.5);

        let init_res = __vbaVarForInit(
            &mut counter,
            &mut temp_step,
            &mut temp_end,
            &v_start,
            &v_end,
            &v_step,
        );
        assert_ne!(init_res, 0);
        assert_eq!(counter.as_f64().unwrap(), 1.0);

        let mut count = 0;
        let mut sum = 0.0f64;
        loop {
            sum += counter.as_f64().unwrap();
            count += 1;
            if __vbaVarForNext(&mut counter, &mut temp_step, &mut temp_end) == 0 {
                break;
            }
        }
        assert_eq!(count, 5); // 1.0, 1.5, 2.0, 2.5, 3.0
        assert!((sum - 10.0).abs() < 1e-6);

        // Negative step Variant loop: 10 to 2 Step -2
        let v_neg_start = Variant::from_i32(10);
        let v_neg_end = Variant::from_i32(2);
        let v_neg_step = Variant::from_i32(-2);

        let mut count_neg = 0;
        if __vbaVarForInit(
            &mut counter,
            &mut temp_step,
            &mut temp_end,
            &v_neg_start,
            &v_neg_end,
            &v_neg_step,
        ) != 0
        {
            loop {
                count_neg += 1;
                if __vbaVarForNext(&mut counter, &mut temp_step, &mut temp_end) == 0 {
                    break;
                }
            }
        }
        assert_eq!(count_neg, 5); // 10, 8, 6, 4, 2
    }
}

#[test]
fn test_for_each_safearray_and_exit_each_ary_lock_management() {
    unsafe {
        let bstr_size = core::mem::size_of::<Bstr>() as u32;
        let psa = SafeArray::create_vector(bstr_size, 1, 4, FADF_HAVEVARTYPE | FADF_BSTR);
        assert!(!psa.is_null());

        let words = ["First", "Second", "Third", "Fourth"];
        for (idx, w) in words.iter().enumerate() {
            let bstr = sys_alloc_string(w);
            let slice = core::slice::from_raw_parts_mut((*psa).pv_data as *mut Bstr, 4);
            slice[idx] = bstr;
        }

        // Full iteration
        let mut enum_state = VbAryEnumState::default();
        let mut item = Variant::empty();
        let mut items = Vec::new();

        let has_items = __vbaForEachAry(
            8, // VT_BSTR
            &mut enum_state as *mut _ as *mut core::ffi::c_void,
            core::ptr::null_mut(),
            &mut item,
            psa,
        );
        assert_ne!(has_items, 0);
        assert_eq!((*psa).c_locks, 1); // Array locked during traversal

        loop {
            items.push(item.to_string_representation());
            if __vbaNextEachAry(
                8,
                core::ptr::null_mut(),
                &mut item,
                &mut enum_state as *mut _ as *mut core::ffi::c_void,
            ) == 0
            {
                break;
            }
        }

        assert_eq!(items, vec!["First", "Second", "Third", "Fourth"]);
        assert_eq!((*psa).c_locks, 0); // Lock cleanly released

        // Early exit iteration using __vbaExitEachAry
        let mut exit_state = VbAryEnumState::default();
        let mut item_exit = Variant::empty();

        let init = __vbaForEachAry(
            8,
            &mut exit_state as *mut _ as *mut core::ffi::c_void,
            core::ptr::null_mut(),
            &mut item_exit,
            psa,
        );
        assert_ne!(init, 0);
        assert_eq!((*psa).c_locks, 1);

        // Early Exit after 1st element
        __vbaExitEachAry(&mut exit_state as *mut _ as *mut core::ffi::c_void);
        assert_eq!((*psa).c_locks, 0); // Lock freed on early exit

        SafeArray::destroy(psa);
    }
}

#[test]
fn test_for_each_variant_wrapping_array_and_exit_each_var() {
    unsafe {
        let psa = SafeArray::create_vector(4, 0, 3, FADF_HAVEVARTYPE);
        let slice = core::slice::from_raw_parts_mut((*psa).pv_data as *mut i32, 3);
        for idx in 0..3 {
            slice[idx] = (idx as i32 + 1) * 100;
        }

        let mut v_arr = Variant::empty();
        v_arr.vt = VT_ARRAY | VT_I4;
        v_arr.set_data_ptr(psa as usize);

        let mut vs_state = [0usize; 4];
        let mut item = Variant::empty();
        let mut sum = 0;

        let init = __vbaForEachVar(
            vs_state.as_mut_ptr() as *mut core::ffi::c_void,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            &mut item,
            &mut v_arr,
        );
        assert_ne!(init, 0);
        assert_eq!((*psa).c_locks, 1);

        loop {
            sum += item.as_i32().unwrap();
            if __vbaNextEachVar(
                vs_state.as_mut_ptr() as *mut core::ffi::c_void,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut item,
            ) == 0
            {
                break;
            }
        }
        assert_eq!(sum, 600); // 100 + 200 + 300
        assert_eq!((*psa).c_locks, 0);

        // Test early exit on Variant array
        let mut es_state = [0usize; 4];
        let mut e_item = Variant::empty();
        __vbaForEachVar(
            es_state.as_mut_ptr() as *mut core::ffi::c_void,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            &mut e_item,
            &mut v_arr,
        );
        assert_eq!((*psa).c_locks, 1);

        __vbaExitEachVar(
            es_state.as_mut_ptr() as *mut core::ffi::c_void,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        );
        assert_eq!((*psa).c_locks, 0);

        SafeArray::destroy(psa);
    }
}

#[test]
fn test_local_gosub_return_and_gosub_free() {
    unsafe {
        let mut frame: usize = 0;

        // Nested GoSub
        __vbaGosub(&mut frame as *mut usize as *mut core::ffi::c_void);
        assert_eq!(frame, 1);

        __vbaGosub(&mut frame as *mut usize as *mut core::ffi::c_void);
        assert_eq!(frame, 2);

        __vbaGosubReturn(&mut frame as *mut usize as *mut core::ffi::c_void);
        assert_eq!(frame, 1);

        __vbaGosubReturn(&mut frame as *mut usize as *mut core::ffi::c_void);
        assert_eq!(frame, 0);

        // GoSubFree on procedure exit with pending GoSub frames
        __vbaGosub(&mut frame as *mut usize as *mut core::ffi::c_void);
        __vbaGosub(&mut frame as *mut usize as *mut core::ffi::c_void);
        assert_eq!(frame, 2);

        __vbaGosubFree(&mut frame as *mut usize as *mut core::ffi::c_void);
        assert_eq!(frame, 0);
    }
}

#[test]
fn test_on_go_check_index_validation() {
    unsafe {
        // Valid indices [0, 255]
        assert_eq!(__vbaOnGoCheck(0), 0);
        assert_eq!(__vbaOnGoCheck(1), 1);
        assert_eq!(__vbaOnGoCheck(5), 5);
        assert_eq!(__vbaOnGoCheck(255), 255);

        // Out of range indices must raise Runtime Error 5 (Invalid procedure call)
        clear_vb_error();
        let _ = __vbaOnGoCheck(-1);
        assert_eq!(get_vb_error_number(), 5);
        clear_vb_error();

        let _ = __vbaOnGoCheck(-100);
        assert_eq!(get_vb_error_number(), 5);
        clear_vb_error();

        let _ = __vbaOnGoCheck(256);
        assert_eq!(get_vb_error_number(), 5);
        clear_vb_error();

        let _ = __vbaOnGoCheck(10000);
        assert_eq!(get_vb_error_number(), 5);
        clear_vb_error();
    }
}

#[test]
fn test_vba_i2i4_conversion_and_vba_r8_err_var() {
    unsafe {
        // __vbaI2I4 within range
        assert_eq!(__vbaI2I4(0), 0);
        assert_eq!(__vbaI2I4(32767), 32767);
        assert_eq!(__vbaI2I4(-32768), -32768);

        // __vbaI2I4 overflow check (Error 6)
        clear_vb_error();
        let _ = __vbaI2I4(32768);
        assert_eq!(get_vb_error_number(), 6);
        clear_vb_error();

        let _ = __vbaI2I4(-32769);
        assert_eq!(get_vb_error_number(), 6);
        clear_vb_error();

        // __vbaR8ErrVar valid
        let v_flt = Variant::from_f64(3.14159);
        let flt_res = __vbaR8ErrVar(&v_flt);
        assert!((flt_res - 3.14159).abs() < 1e-5);

        let v_str = Variant::from_bstr(sys_alloc_string("42.5"));
        let str_res = __vbaR8ErrVar(&v_str);
        assert!((str_res - 42.5).abs() < 1e-5);

        // __vbaR8ErrVar invalid string (Error 13 Type Mismatch)
        clear_vb_error();
        let v_bad_str = Variant::from_bstr(sys_alloc_string("NotANumber"));
        let _ = __vbaR8ErrVar(&v_bad_str);
        assert_eq!(get_vb_error_number(), 13);
        clear_vb_error();
    }
}




#[test]
fn test_signed_and_absolute_value_intrinsics() {
    unsafe {
        // Abs
        assert_eq!(__vbaI2Abs(-1234), 1234);
        assert_eq!(__vbaI2Abs(1234), 1234);
        assert_eq!(__vbaI4Abs(-987654321), 987654321);
        assert_eq!(__vbaI4Abs(987654321), 987654321);
        assert_eq!(__vbaCyAbs(-12345678), 12345678);
        assert_eq!(__vbaCyAbs(12345678), 12345678);

        // Sgn
        assert_eq!(__vbaI2Sgn(-42), -1);
        assert_eq!(__vbaI2Sgn(0), 0);
        assert_eq!(__vbaI2Sgn(42), 1);

        assert_eq!(__vbaI4Sgn(-100000), -1);
        assert_eq!(__vbaI4Sgn(0), 0);
        assert_eq!(__vbaI4Sgn(100000), 1);

        assert_eq!(__vbaUI1Sgn(15), 1);
        assert_eq!(__vbaUI1Sgn(0), 0);

        assert_eq!(__vbaCySgn(-502500), -1);
        assert_eq!(__vbaCySgn(0), 0);
        assert_eq!(__vbaCySgn(502500), 1);

        assert_eq!(__vbaR4Sgn(-3.14), -1);
        assert_eq!(__vbaR4Sgn(0.0), 0);
        assert_eq!(__vbaR4Sgn(3.14), 1);

        assert_eq!(__vbaR8Sgn(-2.71828), -1);
        assert_eq!(__vbaR8Sgn(0.0), 0);
        assert_eq!(__vbaR8Sgn(2.71828), 1);
    }
}

#[test]
fn test_currency_math_and_conversions() {
    unsafe {
        // CyFix & CyInt
        assert_eq!(__vbaCyFix(-1234567), -1230000);
        assert_eq!(__vbaCyInt(-1234567), -1240000);
        assert_eq!(__vbaCyFix(1234567), 1230000);
        assert_eq!(__vbaCyInt(1234567), 1230000);

        // CySub
        assert_eq!(__vbaCySub(5005000, 1252500), 3752500);

        // Coercions to Currency
        assert_eq!(__vbaCyI2(1234), 12340000);
        assert_eq!(__vbaCyI4(987654), 9876540000);
        assert_eq!(__vbaCyUI1(250), 2500000);

        // Coercions from Currency
        assert_eq!(__vbaI2Cy(45678900), 4568);
        assert_eq!(__vbaI4Cy(45678900), 4568);
        assert!((__vbaR4Cy(45678900) - 4567.89).abs() < 0.01);
        assert_eq!(__vbaUI1Cy(2004000), 200);

        // Currency Comparisons
        assert_eq!(__vbaFpCmpCy(1005000, 1005000), 0);
        assert_eq!(__vbaFpCmpCy(1005000, 2007500), -1);
        assert_eq!(__vbaFpCmpCy(2007500, 1005000), 1);
    }
}

#[test]
fn test_floating_point_and_date_conversions() {
    unsafe {
        assert_eq!(__vbaDateR8(36526.5), 36526.5);
        assert_eq!(__vbaDateR4(36526.5), 36526.5);

        assert!((__vbaFpCDblR4(12.34) - 12.34).abs() < 0.01);
        assert_eq!(__vbaFpCDblR8(12345.6789), 12345.6789);
        assert!((__vbaFpCSngR4(12.34) - 12.34).abs() < 0.001);
        assert!((__vbaFpCSngR8(987.654321) - 987.654).abs() < 0.01);
        assert_eq!(__vbaFpUI1(254.6), 255);

        assert_eq!(__vbaPowerR8(2.0, 10.0), 1024.0);
        assert_eq!(__vbaPowerR8(9.0, 0.5), 3.0);

        assert_eq!(__vbaR8FixI2(-15.75), -15);
        assert_eq!(__vbaR8IntI2(-15.75), -16);
        assert_eq!(__vbaR8FixI4(-123456.78), -123456);
        assert_eq!(__vbaR8IntI4(-123456.78), -123457);
    }
}

#[test]
fn test_error_variant_and_type_coercions() {
    unsafe {
        assert_eq!(__vbaUI1I4(127), 127);
        let v210 = Variant::from_i32(210);
        assert_eq!(__vbaUI1Var(&v210), 210);

        let b_dt = sys_alloc_string("2026-10-17 14:00:00");
        let v_dt_str = Variant::from_bstr(b_dt);
        let mut v_date_res = Variant::empty();
        __vbaVarDateVar(&mut v_date_res, &v_dt_str);
        assert_eq!(v_date_res.vt, VT_DATE);
        SysFreeString(b_dt);

        let v_err = Variant::cverr(5);
        __vbaBoolErrVar(&v_err);
        __vbaCyErrVar(&v_err);
        __vbaI2ErrVar(&v_err);
        __vbaR4ErrVar(&v_err);
        __vbaUI1ErrVar(&v_err);
    }
}
