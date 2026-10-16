//! Layer 09: Classic BASIC File I/O & Channel Virtualization Tests
//!
//! Granular tests for FreeFile, Open (Output/Append/Input/Binary), Print #,
//! Line Input #, Write #, Input #, Put #, Get #, Seek, LOF, EOF, FileCopy,
//! Name As, Dir$, FileReset, and Kill corresponding to App09-classic-fileio.

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
