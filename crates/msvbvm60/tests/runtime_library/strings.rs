// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Runtime Library: String Engine & File I/O
//!
//! Intrinsic VB6 string manipulation (`rtc*`, `__vbaStr*`), channel-based
//! sequential/binary file I/O, format masks, and pattern routines.

use msvbvm60::rtc::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;



use std::sync::Mutex;
static FILE_TEST_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_rtc_free_file_allocation() {
    let _guard = FILE_TEST_MUTEX.lock().unwrap();
    unsafe {
        let v_range = Variant::empty();
        let ch = rtcFreeFile(&v_range) as i32;
        assert!(ch >= 1 && ch <= 511);
    }
}

#[test]
fn test_vba_file_open_output_print_and_close() {
    let _guard = FILE_TEST_MUTEX.lock().unwrap();
    unsafe {
        let path = sys_alloc_string("test_io_output.tmp");
        let v_range = Variant::empty();
        let ch = rtcFreeFile(&v_range) as i32;

        assert_eq!(__vbaFileOpen(0x02, -1, ch, path), 0); // Open For Output
        let line = sys_alloc_string("ClassicBasic Line 1");
        assert_eq!(__vbaPrintFile(core::ptr::null(), ch, line), 0);
        assert_eq!(__vbaFileClose(ch), 0);

        let content = std::fs::read_to_string("test_io_output.tmp").unwrap();
        assert_eq!(content, "ClassicBasic Line 1\r\n");

        rtcKillFiles(path);
        SysFreeString(path);
        SysFreeString(line);
    }
}

#[test]
fn test_vba_file_open_append_and_file_len() {
    let _guard = FILE_TEST_MUTEX.lock().unwrap();
    unsafe {
        let path = sys_alloc_string("test_io_append.tmp");
        std::fs::write("test_io_append.tmp", "Header\r\n").unwrap();

        let v_range = Variant::empty();
        let ch = rtcFreeFile(&v_range) as i32;
        assert_eq!(__vbaFileOpen(0x08, -1, ch, path), 0); // Open For Append

        let app_line = sys_alloc_string("Footer");
        assert_eq!(__vbaPrintFile(core::ptr::null(), ch, app_line), 0);
        assert_eq!(__vbaFileClose(ch), 0);

        let len = rtcFileLen(path);
        assert_eq!(len, 16); // "Header\r\nFooter\r\n" = 8 + 8 = 16

        rtcKillFiles(path);
        SysFreeString(path);
        SysFreeString(app_line);
    }
}

#[test]
fn test_vba_file_open_input_line_input_and_eof() {
    let _guard = FILE_TEST_MUTEX.lock().unwrap();
    unsafe {
        let path = sys_alloc_string("test_io_input.tmp");
        std::fs::write("test_io_input.tmp", "Row 1\r\nRow 2\r\n").unwrap();

        let v_range = Variant::empty();
        let ch = rtcFreeFile(&v_range) as i32;
        assert_eq!(__vbaFileOpen(0x01, -1, ch, path), 0); // Open For Input
        assert_eq!(rtcEndOfFile(ch), 0);

        let mut s1: Bstr = core::ptr::null_mut();
        __vbaLineInputStr(&mut s1, ch);
        assert_eq!(bstr_to_string(s1), "Row 1");

        let mut s2: Bstr = core::ptr::null_mut();
        __vbaLineInputStr(&mut s2, ch);
        assert_eq!(bstr_to_string(s2), "Row 2");

        assert_eq!(rtcEndOfFile(ch), -1); // True in VB6 is -1
        assert_eq!(__vbaFileClose(ch), 0);

        rtcKillFiles(path);
        SysFreeString(path);
        SysFreeString(s1);
        SysFreeString(s2);
    }
}

#[test]
fn test_vba_file_binary_put_seek_and_get() {
    let _guard = FILE_TEST_MUTEX.lock().unwrap();
    unsafe {
        let path = sys_alloc_string("test_io_binary.tmp");
        let v_range = Variant::empty();
        let ch = rtcFreeFile(&v_range) as i32;

        assert_eq!(__vbaFileOpen(0x20, -1, ch, path), 0); // Open For Binary

        let write_val: i32 = 0x11223344;
        assert_eq!(
            __vbaPut4(
                4,
                &write_val as *const i32 as *const core::ffi::c_void,
                1,
                ch
            ),
            0
        );

        assert_eq!(__vbaFileSeek(1, ch), 0);
        assert_eq!(rtcFileSeek(ch), 1);

        let mut read_val: i32 = 0;
        assert_eq!(
            __vbaGet4(
                4,
                &mut read_val as *mut i32 as *mut core::ffi::c_void,
                1,
                ch
            ),
            0
        );
        assert_eq!(read_val, 0x11223344);

        assert_eq!(__vbaFileClose(ch), 0);
        rtcKillFiles(path);
        SysFreeString(path);
    }
}

#[test]
fn test_rtc_file_copy_and_name_file() {
    let _guard = FILE_TEST_MUTEX.lock().unwrap();
    unsafe {
        let src = sys_alloc_string("test_copy_src.tmp");
        let dst = sys_alloc_string("test_copy_dst.tmp");
        let ren = sys_alloc_string("test_copy_renamed.tmp");

        std::fs::write("test_copy_src.tmp", "Copy Content").unwrap();

        assert_eq!(rtcFileCopy(src, dst), 0);
        assert!(std::path::Path::new("test_copy_dst.tmp").exists());

        assert_eq!(__vbaNameFile(ren, dst), 0); // Name dst As ren
        assert!(!std::path::Path::new("test_copy_dst.tmp").exists());
        assert!(std::path::Path::new("test_copy_renamed.tmp").exists());

        rtcKillFiles(src);
        rtcKillFiles(ren);

        SysFreeString(src);
        SysFreeString(dst);
        SysFreeString(ren);
    }
}

#[test]
fn test_rtc_dir_wildcard_matching() {
    let _guard = FILE_TEST_MUTEX.lock().unwrap();
    unsafe {
        let pattern = sys_alloc_string("test_dir_pattern_*.tmp");
        std::fs::write("test_dir_pattern_abc.tmp", "abc").unwrap();

        let v_pat = Variant::from_bstr(pattern);
        let found = rtcDir(&v_pat, 0);
        assert_eq!(bstr_to_string(found), "test_dir_pattern_abc.tmp");

        let kill_target = sys_alloc_string("test_dir_pattern_abc.tmp");
        rtcKillFiles(kill_target);

        SysFreeString(pattern);
        SysFreeString(found);
        SysFreeString(kill_target);
    }
}

#[test]
fn test_rtc_file_reset_closes_all() {
    let _guard = FILE_TEST_MUTEX.lock().unwrap();
    unsafe {
        let path = sys_alloc_string("test_io_reset.tmp");
        let v_range = Variant::empty();
        let ch = rtcFreeFile(&v_range) as i32;
        assert_eq!(__vbaFileOpen(0x02, -1, ch, path), 0);

        rtcFileReset(); // Closes all open channels

        rtcKillFiles(path);
        SysFreeString(path);
    }
}

#[test]
fn test_freefile_allocates_lowest_available_virtual_channel() {
    let _guard = FILE_TEST_MUTEX.lock().unwrap();
    unsafe {
        let path1 = sys_alloc_string("test_ff_1.tmp");
        let path2 = sys_alloc_string("test_ff_2.tmp");
        let v_range = Variant::empty();

        // GIVEN channels #1 and #2 are open
        assert_eq!(__vbaFileOpen(0x02, -1, 1, path1), 0);
        assert_eq!(__vbaFileOpen(0x02, -1, 2, path2), 0);

        // WHEN FreeFile() is queried
        let ch = rtcFreeFile(&v_range) as i32;
        // THEN it returns 3
        assert_eq!(ch, 3);

        // WHEN channel #1 is closed and FreeFile() is queried again
        assert_eq!(__vbaFileClose(1), 0);
        let reclaimed = rtcFreeFile(&v_range) as i32;
        // THEN it reclaims and returns channel 1
        assert_eq!(reclaimed, 1);

        // Cleanup
        __vbaFileClose(2);
        rtcKillFiles(path1);
        rtcKillFiles(path2);
        SysFreeString(path1);
        SysFreeString(path2);
    }
}

#[test]
fn test_print_hash_formats_with_tab_and_semicolon_spacing() {
    let _guard = FILE_TEST_MUTEX.lock().unwrap();
    unsafe {
        let path = sys_alloc_string("test_print_tab.tmp");
        let v_range = Variant::empty();
        let ch = rtcFreeFile(&v_range) as i32;
        assert_eq!(__vbaFileOpen(0x02, -1, ch, path), 0);

        // GIVEN an open sequential file on channel #1
        // WHEN executing Print #1, "A"; "B", "C"
        let str_a = sys_alloc_string("A");
        let str_b = sys_alloc_string("B");
        let str_c = sys_alloc_string("C");

        // "A"; -> PRINT_SEMICOLON
        assert_eq!(__vbaPrintFile(PRINT_SEMICOLON as *const _, ch, str_a), 0);
        // "B", -> PRINT_COMMA
        assert_eq!(__vbaPrintFile(PRINT_COMMA as *const _, ch, str_b), 0);
        // "C" -> PRINT_NEWLINE
        assert_eq!(__vbaPrintFile(PRINT_NEWLINE as *const _, ch, str_c), 0);

        assert_eq!(__vbaFileClose(ch), 0);

        let content = std::fs::read_to_string("test_print_tab.tmp").unwrap();
        // THEN "A" and "B" are concatenated without spaces (semicolon behavior)
        // AND "C" is aligned to the next 14-column tab stop (comma behavior)
        // AND a standard CRLF (\r\n) is appended at the end
        assert_eq!(content, "AB            C\r\n");

        rtcKillFiles(path);
        SysFreeString(path);
        SysFreeString(str_a);
        SysFreeString(str_b);
        SysFreeString(str_c);
    }
}

#[test]
fn test_line_input_preserves_quotes_whereas_input_strips_them() {
    let _guard = FILE_TEST_MUTEX.lock().unwrap();
    unsafe {
        let file_path = "test_quotes_diff.tmp";
        // GIVEN a file containing the line: "John Doe",42,"Engineer"
        std::fs::write(file_path, "\"John Doe\",42,\"Engineer\"\r\n").unwrap();

        let path = sys_alloc_string(file_path);
        let ch = 1;
        assert_eq!(__vbaFileOpen(0x01, -1, ch, path), 0); // Open For Input

        // WHEN Line Input #1, strLine is executed
        let mut line_bstr: Bstr = core::ptr::null_mut();
        assert_eq!(__vbaLineInputStr(&mut line_bstr, ch), 0);
        // THEN strLine contains the literal string: "\"John Doe\",42,\"Engineer\""
        assert_eq!(bstr_to_string(line_bstr), "\"John Doe\",42,\"Engineer\"");
        SysFreeString(line_bstr);
        __vbaFileClose(ch);

        // WHEN resetting and using Input #1, varName, varAge, varJob
        assert_eq!(__vbaFileOpen(0x01, -1, ch, path), 0);
        let mut b_name: Bstr = core::ptr::null_mut();
        let mut b_age: Bstr = core::ptr::null_mut();
        let mut b_job: Bstr = core::ptr::null_mut();

        assert_eq!(__vbaInputFile(ch, &mut b_name), 0);
        assert_eq!(__vbaInputFile(ch, &mut b_age), 0);
        assert_eq!(__vbaInputFile(ch, &mut b_job), 0);

        // THEN varName equals "John Doe" (quotes stripped)
        assert_eq!(bstr_to_string(b_name), "John Doe");
        // AND varAge equals 42
        assert_eq!(bstr_to_string(b_age), "42");
        // AND varJob equals "Engineer"
        assert_eq!(bstr_to_string(b_job), "Engineer");

        SysFreeString(b_name);
        SysFreeString(b_age);
        SysFreeString(b_job);
        __vbaFileClose(ch);

        rtcKillFiles(path);
        SysFreeString(path);
    }
}

#[test]
fn test_binary_put_and_get_seek_pointers_match_byte_offsets() {
    let _guard = FILE_TEST_MUTEX.lock().unwrap();
    unsafe {
        let path = sys_alloc_string("test_bin_offsets.tmp");
        let ch = 1;
        // GIVEN a file opened in Binary mode (0x20)
        assert_eq!(__vbaFileOpen(0x20, -1, ch, path), 0);

        // WHEN Put #1, 1, 0x12345678u32 (4 bytes at position 1)
        let val1: u32 = 0x12345678;
        assert_eq!(__vbaPut4(4, &val1 as *const _ as _, 1, ch), 0);

        // AND Put #1, 5, 0xABCDEFu32 (4 bytes at position 5)
        let val2: u32 = 0x00ABCDEF;
        assert_eq!(__vbaPut4(4, &val2 as *const _ as _, 5, ch), 0);

        // THEN Loc(1) reports position 8
        assert_eq!(rtcFileLocation(ch), 8);
        // AND LOF(1) reports file length 8
        assert_eq!(rtcFileLength(ch), 8);

        // WHEN Get #1, 5, readVar
        let mut read_var: u32 = 0;
        assert_eq!(__vbaGet4(4, &mut read_var as *mut _ as _, 5, ch), 0);
        // THEN readVar recovers exactly 0xABCDEF
        assert_eq!(read_var, 0x00ABCDEF);

        __vbaFileClose(ch);
        rtcKillFiles(path);
        SysFreeString(path);
    }
}

#[test]
fn test_close_without_arguments_closes_all_open_channels() {
    let _guard = FILE_TEST_MUTEX.lock().unwrap();
    unsafe {
        let p1 = sys_alloc_string("test_close_all_1.tmp");
        let p2 = sys_alloc_string("test_close_all_2.tmp");
        let p3 = sys_alloc_string("test_close_all_3.tmp");

        // GIVEN channels #1, #2, and #3 are open
        assert_eq!(__vbaFileOpen(0x02, -1, 1, p1), 0);
        assert_eq!(__vbaFileOpen(0x02, -1, 2, p2), 0);
        assert_eq!(__vbaFileOpen(0x02, -1, 3, p3), 0);

        // WHEN rtcClose is called with channel 0 or no parameters
        assert_eq!(rtcClose(core::ptr::null()), 0);

        // THEN channels #1, #2, and #3 are cleanly flushed and closed
        // AND subsequent EOF(1) raises Runtime Error 52 (Bad file name or number)
        let eof_res = rtcEndOfFile(1);
        assert_eq!(eof_res, -1);
        assert_eq!(msvbvm60::err::get_vb_error_number(), 52);

        rtcKillFiles(p1);
        rtcKillFiles(p2);
        rtcKillFiles(p3);
        SysFreeString(p1);
        SysFreeString(p2);
        SysFreeString(p3);
    }
}




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
        let mut b_out: Bstr = std::ptr::null_mut();
        let b_res = __vbaStrVarVal(&mut b_out, &v_txt);
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
