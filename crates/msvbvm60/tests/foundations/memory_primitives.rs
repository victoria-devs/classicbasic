// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Foundations: Memory Primitives & Win32 Shims
//!
//! Pointer arithmetic, memory reading/writing (`VarPtr`, `GetMem*`, `PutMem*`),
//! alignment, and Win32 KERNEL32/ADVAPI32/OLEAUT32 system shims.

use core::ffi::c_void;
use msvbvm60::pe::shims::*;
use msvbvm60::*;
use std::ffi::{CStr, CString};

#[test]
fn test_varptr_returns_exact_pointer() {
    let mut val: u32 = 0x12345678;
    let ptr = &mut val as *mut u32 as *mut c_void;
    unsafe {
        assert_eq!(VarPtr(ptr), ptr);
    }
}

#[test]
fn test_getmem4_reads_memory() {
    let val: u32 = 0xCAFEBABE;
    let mut dest: u32 = 0;
    unsafe {
        let res = GetMem4(&val, &mut dest);
        assert_eq!(res, S_OK);
        assert_eq!(dest, 0xCAFEBABE);
    }
}

#[test]
fn test_putmem4_writes_memory() {
    let mut val: u32 = 0;
    unsafe {
        let res = PutMem4(&mut val, 0x12345678);
        assert_eq!(res, S_OK);
        assert_eq!(val, 0x12345678);
    }
}

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

#[test]
fn test_advapi32_registry_shims_and_resolver() {
    use msvbvm60::pe::resolver::resolve_symbol;

    let p_open = resolve_symbol("advapi32.dll", Some("RegOpenKeyExA"), None);
    let p_set = resolve_symbol("advapi32.dll", Some("RegSetValueExA"), None);
    let p_query = resolve_symbol("advapi32.dll", Some("RegQueryValueExA"), None);
    let p_close = resolve_symbol("advapi32.dll", Some("RegCloseKey"), None);
    let p_user = resolve_symbol("advapi32.dll", Some("GetUserNameA"), None);

    assert_ne!(p_open, 0);
    assert_ne!(p_set, 0);
    assert_ne!(p_query, 0);
    assert_ne!(p_close, 0);
    assert_ne!(p_user, 0);

    unsafe {
        let key_name = CString::new("Software\\ClassicBasic\\TestKey").unwrap();
        let val_name = CString::new("ConfigValue").unwrap();
        let val_data = CString::new("42").unwrap();

        let mut hkey: usize = 0;
        let res_open = advapi32_RegOpenKeyExA(0x80000002, key_name.as_ptr(), 0, 0, &mut hkey);
        assert_eq!(res_open, 0); // ERROR_SUCCESS
        assert_ne!(hkey, 0);

        let res_set = advapi32_RegSetValueExA(
            hkey,
            val_name.as_ptr(),
            0,
            1, // REG_SZ
            val_data.as_ptr() as *const u8,
            val_data.as_bytes().len() as u32,
        );
        assert_eq!(res_set, 0);

        let mut buf = [0u8; 64];
        let mut buf_len = buf.len() as u32;
        let mut val_type = 0u32;
        let res_query = advapi32_RegQueryValueExA(
            hkey,
            val_name.as_ptr(),
            core::ptr::null_mut(),
            &mut val_type,
            buf.as_mut_ptr(),
            &mut buf_len,
        );
        assert_eq!(res_query, 0);
        assert_eq!(val_type, 1);
        let read_val = CStr::from_ptr(buf.as_ptr() as *const i8).to_str().unwrap();
        assert_eq!(read_val, "42");

        let res_close = advapi32_RegCloseKey(hkey);
        assert_eq!(res_close, 0);

        let mut user_buf = [0i8; 128];
        let mut user_len = user_buf.len() as u32;
        let res_user = advapi32_GetUserNameA(user_buf.as_mut_ptr(), &mut user_len);
        assert_eq!(res_user, 1);
        assert!(user_len > 1);
    }
}

#[test]
fn test_oleaut32_variant_and_safearray_shims_and_resolver() {
    use msvbvm60::pe::resolver::resolve_symbol;
    use msvbvm60::types::Variant;

    let p_var_init = resolve_symbol("oleaut32.dll", Some("VariantInit"), None);
    let p_sa_create = resolve_symbol("oleaut32.dll", Some("SafeArrayCreateVector"), None);
    let p_sys_alloc = resolve_symbol("oleaut32.dll", Some("SysAllocString"), None);
    assert_ne!(p_var_init, 0);
    assert_ne!(p_sa_create, 0);
    assert_ne!(p_sys_alloc, 0);

    let p_ord_sys_alloc = resolve_symbol("oleaut32.dll", None, Some(2));
    let p_ord_var_init = resolve_symbol("oleaut32.dll", None, Some(8));
    let p_ord_sa_create = resolve_symbol("oleaut32.dll", None, Some(15));
    assert_eq!(p_ord_sys_alloc, p_sys_alloc);
    assert_eq!(p_ord_var_init, p_var_init);
    assert_eq!(p_ord_sa_create, p_sa_create);

    unsafe {
        let mut v = Variant::from_i32(12345);
        oleaut32_VariantInit(&mut v);
        assert_eq!(v.vt, 0);

        let psa = oleaut32_SafeArrayCreateVector(3, 0, 10);
        assert!(!psa.is_null());
        assert_eq!(oleaut32_SafeArrayGetDim(psa), 1);

        let mut lbound = 0i32;
        let mut ubound = 0i32;
        assert_eq!(oleaut32_SafeArrayGetLBound(psa, 1, &mut lbound), 0);
        assert_eq!(oleaut32_SafeArrayGetUBound(psa, 1, &mut ubound), 0);
        assert_eq!(lbound, 0);
        assert_eq!(ubound, 9);

        let mut data_ptr: *mut core::ffi::c_void = core::ptr::null_mut();
        assert_eq!(oleaut32_SafeArrayAccessData(psa, &mut data_ptr), 0);
        assert!(!data_ptr.is_null());
        let slice = core::slice::from_raw_parts_mut(data_ptr as *mut i32, 10);
        slice[0] = 777;
        assert_eq!(slice[0], 777);

        assert_eq!(oleaut32_SafeArrayUnaccessData(psa), 0);
        assert_eq!(oleaut32_SafeArrayDestroy(psa), 0);
    }
}
