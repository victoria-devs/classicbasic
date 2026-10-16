//! Layer 05: Win32 API Shims & KERNEL32 Emulation Tests
//!
//! Granular tests for POSIX-backed Win32 file handles, WriteFile, ReadFile,
//! GetFileSize, CloseHandle, GetLastError, and standard handles corresponding to App05-win32-shims.

use msvbvm60::pe::shims::*;
use std::ffi::CString;

#[test]
fn test_win32_create_file_write_and_close() {
    unsafe {
        let test_path = CString::new("test_shim_write.tmp").unwrap();
        let h_file = win32_CreateFileA(
            test_path.as_ptr(),
            0x40000000, // GENERIC_WRITE
            0,
            core::ptr::null_mut(),
            2,    // CREATE_ALWAYS
            0x80, // FILE_ATTRIBUTE_NORMAL
            0,
        );
        assert_ne!(h_file, 0xFFFFFFFF);
        assert_ne!(h_file, !0usize);

        let payload = b"Win32 Shim Payload 123";
        let mut bytes_written: u32 = 0;
        let res = win32_WriteFile(
            h_file,
            payload.as_ptr(),
            payload.len() as u32,
            &mut bytes_written,
            core::ptr::null_mut(),
        );
        assert_eq!(res, 1);
        assert_eq!(bytes_written as usize, payload.len());

        let close_res = win32_CloseHandle(h_file);
        assert_eq!(close_res, 1);

        let _ = std::fs::remove_file("test_shim_write.tmp");
    }
}

#[test]
fn test_win32_open_read_and_get_file_size() {
    unsafe {
        let test_path = CString::new("test_shim_read.tmp").unwrap();
        std::fs::write("test_shim_read.tmp", b"Testing ReadFile").unwrap();

        let h_file = win32_CreateFileA(
            test_path.as_ptr(),
            0x80000000, // GENERIC_READ
            1,          // FILE_SHARE_READ
            core::ptr::null_mut(),
            3, // OPEN_EXISTING
            0x80,
            0,
        );
        assert_ne!(h_file, 0xFFFFFFFF);

        let mut size_high: u32 = 0;
        let size = win32_GetFileSize(h_file, &mut size_high);
        assert_eq!(size, 16);
        assert_eq!(size_high, 0);

        let mut buf = vec![0u8; 16];
        let mut bytes_read: u32 = 0;
        let res = win32_ReadFile(
            h_file,
            buf.as_mut_ptr(),
            16,
            &mut bytes_read,
            core::ptr::null_mut(),
        );
        assert_eq!(res, 1);
        assert_eq!(bytes_read, 16);
        assert_eq!(&buf[..], b"Testing ReadFile");

        assert_eq!(win32_CloseHandle(h_file), 1);
        let _ = std::fs::remove_file("test_shim_read.tmp");
    }
}

#[test]
fn test_win32_get_last_error_on_missing_file() {
    unsafe {
        let bad_path = CString::new("this_file_does_not_exist_98765.tmp").unwrap();
        let h_file = win32_CreateFileA(
            bad_path.as_ptr(),
            0x80000000,
            0,
            core::ptr::null_mut(),
            3, // OPEN_EXISTING
            0x80,
            0,
        );
        assert_eq!(h_file, 0xFFFFFFFF);
        assert_eq!(win32_GetLastError(), 2); // ERROR_FILE_NOT_FOUND
    }
}

#[test]
fn test_win32_get_std_handle() {
    unsafe {
        assert_ne!(win32_GetStdHandle(-11), 0); // STD_OUTPUT_HANDLE (-11)
        assert_ne!(win32_GetStdHandle(-12), 0); // STD_ERROR_HANDLE (-12)
    }
}
