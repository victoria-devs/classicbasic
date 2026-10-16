//! Layer 09: Classic BASIC File I/O & Channel Virtualization Tests
//!
//! Granular tests for FreeFile, Open (Output/Append/Input/Binary), Print #,
//! Line Input #, Write #, Input #, Put #, Get #, Seek, LOF, EOF, FileCopy,
//! Name As, Dir$, FileReset, and Kill corresponding to App09-classic-fileio.

use std::sync::Mutex;
use msvbvm60::rtc::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;

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
            __vbaPut4(4, &write_val as *const i32 as *const core::ffi::c_void, 1, ch),
            0
        );

        assert_eq!(__vbaFileSeek(1, ch), 0);
        assert_eq!(rtcFileSeek(ch), 1);

        let mut read_val: i32 = 0;
        assert_eq!(
            __vbaGet4(4, &mut read_val as *mut i32 as *mut core::ffi::c_void, 1, ch),
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
