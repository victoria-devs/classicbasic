// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Regression Suite `v0.1.8` — Phase 1.8: Standard Library Complete Closure (`rtc*`) (`App23`)
//!
//! Validates 100% completion of all remaining `rtc*` and `rt*` symbols in `msvbvm60.dll`:
//! - Variant string slicing, casing, and trimming (`rtcLeftVar`, `rtcRightVar`, `rtcMidVar`, etc.)
//! - Byte and Char string primitives (`rtcByteValueBstr`, `rtcBstrFromByte`, `rtcCharValueBstr`, etc.)
//! - Error handling and inspection (`rtcCVErrFromVar`, `rtcIsError`, `rtcGetErl`, `rt*FromErrVar`, etc.)
//! - Date and Time statements and calendar (`rtcSetDateVar`, `rtcGetDateVar`, `rtcGetCurrentCalendar`, etc.)
//! - Format, hex, octal, and variant conversions (`rtcHexVarFromVar`, `rtcOctVarFromVar`, `rtcVarFromFormatVar`, etc.)
//! - Filesystem and Environment inspection (`rtcCurrentDir`, `rtcEnvironVar`, `rtcFileDateTime`, etc.)
//! - GUI, colors, and OS platform shims (`rtcQBColor`, `rtcInputBox`, `rtcSendKeys`, `rtcMacId`, etc.)
//! - Compiler runtime helpers (`__vbaStrVarVal`, `__vbaVarErrI4`, `__vbaLenVar`, `__vbaStrComp`, `__vbaInStrVar`, `__vbaStrUI1`)

use msvbvm60::rtc::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;

#[test]
fn test_variant_string_slice_and_casing() {
    unsafe {
        let v_source = Variant::from_bstr(sys_alloc_string("  ClassicBasic VB6  "));

        let mut v_left = Variant::empty();
        let v_len7 = Variant::from_i32(7);
        rtcLeftVar(&mut v_left, &v_source, &v_len7);
        assert_eq!(v_left.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "  Class");

        let mut v_right = Variant::empty();
        let v_len5 = Variant::from_i32(5);
        rtcRightVar(&mut v_right, &v_source, &v_len5);
        assert_eq!(v_right.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "VB6  ");

        let mut v_mid = Variant::empty();
        let v_start3 = Variant::from_i32(3);
        let v_len12 = Variant::from_i32(12);
        rtcMidVar(&mut v_mid, &v_source, &v_start3, &v_len12);
        assert_eq!(v_mid.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "ClassicBasic");

        let b_mid = sys_alloc_string("HelloWorld");
        let b_mid_res = rtcMidBstr(b_mid, 6, &Variant::from_i32(5));
        assert_eq!(bstr_to_string(b_mid_res), "World");
        SysFreeString(b_mid);
        SysFreeString(b_mid_res);

        let mut v_ltrim = Variant::empty();
        rtcLeftTrimVar(&mut v_ltrim, &v_source);
        assert_eq!(v_ltrim.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "ClassicBasic VB6  ");

        let mut v_rtrim = Variant::empty();
        rtcRightTrimVar(&mut v_rtrim, &v_source);
        assert_eq!(v_rtrim.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "  ClassicBasic VB6");

        let mut v_trim = Variant::empty();
        rtcTrimVar(&mut v_trim, &v_source);
        assert_eq!(v_trim.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "ClassicBasic VB6");

        let b_sp = sys_alloc_string("   ABC");
        let b_lt = rtcLeftTrimBstr(b_sp);
        assert_eq!(bstr_to_string(b_lt), "ABC");
        SysFreeString(b_sp);
        SysFreeString(b_lt);

        let b_sp2 = sys_alloc_string("XYZ   ");
        let b_rt = rtcRightTrimBstr(b_sp2);
        assert_eq!(bstr_to_string(b_rt), "XYZ");
        SysFreeString(b_sp2);
        SysFreeString(b_rt);

        let mut v_lcase = Variant::empty();
        rtcLowerCaseVar(&mut v_lcase, &v_source);
        assert!(v_lcase.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default().contains("classicbasic"));
    }
}

#[test]
fn test_variant_space_string_instr_len() {
    unsafe {
        let mut v_spc = Variant::empty();
        let v_c4 = Variant::from_i32(4);
        rtcSpaceVar(&mut v_spc, &v_c4);
        assert_eq!(v_spc.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "    ");

        let mut v_str = Variant::empty();
        let v_c5 = Variant::from_i32(5);
        let v_char_x = Variant::from_bstr(sys_alloc_string("X"));
        rtcStringVar(&mut v_str, &v_c5, &v_char_x);
        assert_eq!(v_str.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "XXXXX");

        let v_src = Variant::from_bstr(sys_alloc_string("AlphaBetaGamma"));
        let v_target = Variant::from_bstr(sys_alloc_string("Beta"));
        let v_st = Variant::from_i32(1);
        let v_comp = Variant::from_i32(0);
        let res_instr = rtcInStr(&v_st, &v_src, &v_target, &v_comp);
        assert_eq!(res_instr.as_i32().unwrap_or(0), 6);

        let b_src = sys_alloc_string("AlphaBetaGamma");
        let b_tgt = sys_alloc_string("Beta");
        let res_char = rtcInStrChar(&v_st, b_src, b_tgt, &v_comp);
        assert_eq!(res_char, 6);
        SysFreeString(b_src);
        SysFreeString(b_tgt);

        let b_cmp1 = sys_alloc_string("hello");
        let b_cmp2 = sys_alloc_string("HELLO");
        assert!(rtcCompareBstr(b_cmp1, b_cmp2, 0) > 0);
        assert_eq!(rtcCompareBstr(b_cmp1, b_cmp2, 1), 0);
        SysFreeString(b_cmp1);
        SysFreeString(b_cmp2);

        assert_eq!(rtcLenVar(&v_src), 14);
        assert_eq!(rtcLenCharVar(&v_src), 28);

        let mut v_left_char = Variant::empty();
        rtcLeftCharVar(&mut v_left_char, &v_src, 5);
        assert_eq!(v_left_char.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "Alpha");

        let mut v_right_char = Variant::empty();
        rtcRightCharVar(&mut v_right_char, &v_src, 5);
        assert_eq!(v_right_char.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "Gamma");

        let mut v_mid_char = Variant::empty();
        rtcMidCharVar(&mut v_mid_char, &v_src, 6, &Variant::from_i32(4));
        assert_eq!(v_mid_char.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "Beta");
    }
}

#[test]
fn test_byte_and_char_conversions() {
    unsafe {
        let b_a = sys_alloc_string("A");
        assert_eq!(rtcByteValueBstr(b_a), 65);
        assert_eq!(rtcCharValueBstr(b_a), 65);
        SysFreeString(b_a);

        let b_b = rtcBstrFromByte(66);
        assert_eq!(rtcByteValueBstr(b_b), 66);
        SysFreeString(b_b);

        let b_c = rtcBstrFromChar(67);
        assert_eq!(bstr_to_string(b_c), "C");
        SysFreeString(b_c);

        let mut v_b = Variant::empty();
        rtcVarBstrFromByte(&mut v_b, 68);
        let b_ptr = v_b.as_bstr().unwrap();
        assert_eq!(rtcByteValueBstr(b_ptr), 68);
        SysFreeString(b_ptr);

        let mut v_c = Variant::empty();
        rtcVarBstrFromChar(&mut v_c, 69);
        assert_eq!(v_c.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "E");

        let mut v_ansi = Variant::empty();
        let v_txt = Variant::from_bstr(sys_alloc_string("Hello"));
        rtcVarBstrFromAnsi(&mut v_ansi, &v_txt);
        assert_eq!(v_ansi.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "Hello");

        let mut v_str_res = Variant::empty();
        rtcVarStrFromVar(&mut v_str_res, &v_txt);
        assert_eq!(v_str_res.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "Hello");
    }
}

#[test]
fn test_error_and_inspection() {
    unsafe {
        let mut v_err = Variant::empty();
        let v_code = Variant::from_i32(11); // Division by zero
        rtcCVErrFromVar(&mut v_err, &v_code);
        assert_eq!(rtcIsError(&v_err), -1);

        let v_ok = Variant::from_i32(42);
        assert_eq!(rtcIsError(&v_ok), 0);

        let v_missing = Variant {
            vt: VT_ERROR,
            w_reserved1: 0,
            w_reserved2: 0,
            w_reserved3: 0,
            data: [0x04, 0x00, 0x02, 0x80, 0, 0, 0, 0],
        };
        assert_eq!(rtcIsMissing(&v_missing), -1);
        assert_eq!(rtcIsMissing(&v_ok), 0);

        assert_eq!(rtcGetErl(), 0);

        let b_err = rtcBstrFromError(11);
        assert!(!bstr_to_string(b_err).is_empty());
        SysFreeString(b_err);

        let mut v_from_err = Variant::empty();
        rtcVarFromError(&mut v_from_err, 11);
        assert_eq!(v_from_err.vt, VT_BSTR);

        assert_eq!(rtI4FromErrVar(&v_err), 11);
        assert_eq!(rtI2FromErrVar(&v_err), 11);
        assert_eq!(rtUI1FromErrVar(&v_err), 11);
        assert_eq!(rtBoolFromErrVar(&v_err), -1);
        assert_eq!(rtCyFromErrVar(&v_err), 110000);
        assert!((rtR4FromErrVar(&v_err) - 11.0).abs() < 1e-4);
        assert!((rtR8FromErrVar(&v_err) - 11.0).abs() < 1e-6);

        let b_str_err = rtBstrFromErrVar(&v_err);
        assert_eq!(bstr_to_string(b_str_err), "Error 11");
        SysFreeString(b_str_err);
    }
}

#[test]
fn test_date_time_statements_and_calendar() {
    unsafe {
        let mut v_d = Variant::empty();
        rtcGetDateVar(&mut v_d);
        assert_eq!(v_d.vt, VT_DATE);
        assert!(v_d.as_f64().unwrap_or(0.0) > 0.0);

        let mut v_t = Variant::empty();
        rtcGetTimeVar(&mut v_t);
        assert_eq!(v_t.vt, VT_DATE);
        assert!(v_t.as_f64().unwrap_or(0.0) >= 0.0);

        let v_d_str = Variant::from_bstr(sys_alloc_string("2026-10-17"));
        let d_val = rtcDateFromVar(&v_d_str);
        assert!(d_val > 0.0);

        let mut v_date_ret = Variant::empty();
        rtcVarDateFromVar(&mut v_date_ret, &v_d_str);
        assert_eq!(v_date_ret.vt, VT_DATE);

        let v_new_date = Variant::from_bstr(sys_alloc_string("2026-01-01"));
        rtcSetDateVar(&v_new_date);
        let b_date = sys_alloc_string("2026-01-01");
        rtcSetDateBstr(b_date);
        SysFreeString(b_date);

        let v_new_time = Variant::from_bstr(sys_alloc_string("12:00:00"));
        rtcSetTimeVar(&v_new_time);
        let b_time = sys_alloc_string("12:00:00");
        rtcSetTimeBstr(b_time);
        SysFreeString(b_time);

        assert_eq!(rtcGetCurrentCalendar(), 0);
        rtcSetCurrentCalendar(1);
        assert_eq!(rtcGetCurrentCalendar(), 1);
        rtcSetCurrentCalendar(0);
    }
}

#[test]
fn test_format_hex_oct_and_conversions() {
    unsafe {
        let mut v_hex = Variant::empty();
        let v_val = Variant::from_i32(255);
        rtcHexVarFromVar(&mut v_hex, &v_val);
        assert_eq!(v_hex.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "FF");

        let mut v_oct = Variant::empty();
        let v_val64 = Variant::from_i32(64);
        rtcOctVarFromVar(&mut v_oct, &v_val64);
        assert_eq!(v_oct.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "100");

        let mut v_cloned = Variant::empty();
        rtcVarFromVar(&mut v_cloned, &v_val);
        assert_eq!(v_cloned.as_i32().unwrap_or(0), 255);

        let mut v_fmt = Variant::empty();
        let v_num = Variant::from_f64(1234.5);
        let v_pat = Variant::from_bstr(sys_alloc_string("#,##0.00"));
        rtcVarFromFormatVar(&mut v_fmt, &v_num, &v_pat, core::ptr::null(), core::ptr::null());
        assert_eq!(v_fmt.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "1,234.50");
    }
}

#[test]
fn test_filesystem_and_environment() {
    unsafe {
        let b_c = sys_alloc_string("C");
        rtcChangeDrive(b_c);
        SysFreeString(b_c);

        let mut v_drv = Variant::empty();
        let v_c = Variant::from_bstr(sys_alloc_string("C"));
        rtcCurrentDir(&mut v_drv, &v_c);
        assert!(!v_drv.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default().is_empty());

        let mut v_env = Variant::empty();
        let v_k = Variant::from_bstr(sys_alloc_string("PATH"));
        rtcEnvironVar(&mut v_env, &v_k);
        assert!(!v_env.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default().is_empty());

        assert_eq!(rtcGetHostLCID(), 0x0409);

        let b_dummy = sys_alloc_string("dummy.txt");
        let dt_res = rtcFileDateTime(b_dummy);
        assert_eq!(dt_res.vt, VT_DATE);
        SysFreeString(b_dummy);

        let b_inp = rtcInputCount(1, 10);
        assert_eq!(bstr_to_string(b_inp), "");
        SysFreeString(b_inp);

        let mut v_inp_cnt = Variant::empty();
        let v_cnt1 = Variant::from_i32(1);
        let v_ch10 = Variant::from_i32(10);
        rtcInputCountVar(&mut v_inp_cnt, &v_cnt1, &v_ch10);

        let b_inp_char = rtcInputCharCount(1, 10);
        assert_eq!(bstr_to_string(b_inp_char), "");
        SysFreeString(b_inp_char);

        let mut v_inp_char = Variant::empty();
        rtcInputCharCountVar(&mut v_inp_char, &v_cnt1, &v_ch10);
        assert_eq!(v_inp_char.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "");
    }
}

#[test]
fn test_gui_and_os_shims() {
    unsafe {
        assert_eq!(rtcQBColor(0), 0x000000); // Black
        assert_eq!(rtcQBColor(1), 0x800000); // Blue
        assert_eq!(rtcQBColor(2), 0x008000); // Green
        assert_eq!(rtcQBColor(4), 0x000080); // Red
        assert_eq!(rtcQBColor(15), 0xFFFFFF); // Bright White

        let b_mac = sys_alloc_string("TEXT");
        assert_eq!(rtcMacId(b_mac), 0x54455854);
        SysFreeString(b_mac);

        let v_prompt = Variant::from_bstr(sys_alloc_string("Enter name:"));
        let v_def = Variant::from_bstr(sys_alloc_string("Antigravity"));
        let mut v_box_res = Variant::empty();
        rtcInputBox(&mut v_box_res, &v_prompt, core::ptr::null(), &v_def, core::ptr::null(), core::ptr::null(), core::ptr::null(), core::ptr::null());
        assert_eq!(v_box_res.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "Antigravity");

        let v_app = Variant::from_bstr(sys_alloc_string("Calculator"));
        rtcAppActivate(&v_app, core::ptr::null());

        let b_keys = sys_alloc_string("123{ENTER}");
        rtcSendKeys(b_keys, core::ptr::null());
        SysFreeString(b_keys);

        assert_eq!(rtcIMEStatus(), 0);

        let b_script = sys_alloc_string("beep");
        let b_apple_res = rtcAppleScript(b_script);
        assert_eq!(bstr_to_string(b_apple_res), "");
        SysFreeString(b_script);
        SysFreeString(b_apple_res);
    }
}

#[test]
fn test_compiler_helpers() {
    unsafe {
        let v_txt = Variant::from_bstr(sys_alloc_string("42"));
        let b_res = __vbaStrVarVal(&v_txt);
        assert_eq!(bstr_to_string(b_res), "42");
        SysFreeString(b_res);

        let mut v_err = Variant::empty();
        __vbaVarErrI4(&mut v_err, 9);
        assert_eq!(v_err.vt, VT_ERROR);

        let v_s = Variant::from_bstr(sys_alloc_string("Testing"));
        assert_eq!(__vbaLenVar(&v_s), 7);
        assert_eq!(__vbaLenVarB(&v_s), 14);

        let b1 = sys_alloc_string("abc");
        let b2 = sys_alloc_string("ABC");
        assert_eq!(__vbaStrComp(b1, b2, 0), 1);
        assert_eq!(__vbaStrComp(b1, b2, 1), 0);
        SysFreeString(b1);
        SysFreeString(b2);

        let v_sub = Variant::from_bstr(sys_alloc_string("ing"));
        assert_eq!(__vbaInStrVar(core::ptr::null(), &v_s, &v_sub, 0), 5);

        let b_ui1 = __vbaStrUI1(65);
        assert_eq!(bstr_to_string(b_ui1), "A");
        SysFreeString(b_ui1);
    }
}
