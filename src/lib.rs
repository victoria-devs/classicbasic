//! # MSVBVM60 - Visual Basic 6.0 Native Runtime Clone in Rust
//!
//! Re-implementation and reverse-engineering study clone of Microsoft's `MSVBVM60.DLL`
//! (Visual Basic Virtual Machine 6.0).
//!
//! ## Overview of MSVBVM60.DLL
//!
//! `MSVBVM60.DLL` is the runtime execution engine for all software built with Microsoft
//! Visual Basic 6.0 (released in 1998, code named "Thunder"). Whether a VB6 program was compiled
//! to Native Code (x86 machine instructions via an optimizing backend based on MSVC C2) or P-Code
//! (interpreted bytecode), it depends directly on this DLL.
//!
//! ### Key Responsibilities:
//! 1. **Application Lifecycle (`ThunRTMain`)**:
//!    - Native VB6 binaries have a minimal entry point that pushes the address of a `VBHeader`
//!      structure and invokes `ThunRTMain` ("Thunder Runtime Main", ordinal 100).
//!    - `ThunRTMain` validates the `"VB5!"` magic bytes, initializes COM/OLE apartments,
//!      registers SEH exception handlers (`__vbaExceptHandler`), loads forms/modules,
//!      invokes `Sub Main` or creates the initial Form, and manages the Win32 message pump.
//!
//! 2. **P-Code Interpreter Engines (`ProcCallEngine`, `MethCallEngine`)**:
//!    - For applications compiled in P-Code mode, execution is interpreted instruction-by-instruction
//!      by these engines.
//!
//! 3. **Type System and Object Model**:
//!    - **BSTR**: Length-prefixed (32-bit byte count at offset -4), UTF-16, null-terminated COM strings.
//!    - **VARIANT**: 16-byte tagged unions (`VT_I4`, `VT_BSTR`, `VT_DISPATCH`, etc.) enabling VB6 dynamic typing.
//!    - **SAFEARRAY**: Multi-dimensional bounded array descriptors.
//!    - **Currency (CY)**: 64-bit integer fixed-point with 4 decimal places.
//!    - **COM Interfaces**: `BASIC_CLASS_*`, `EVENT_SINK_*`, and `Zombie_*` implement `IUnknown` and `IDispatch`.
//!
//! 4. **Memory Manipulation (`GetMem*`, `PutMem*`, `VarPtr`)**:
//!    - Since VB6 lacks native pointer syntax, programmers historically used undocumented runtime
//!      exports like `VarPtr` (which passes back a variable's address) and `GetMem4`/`PutMem4`
//!      for direct memory manipulation and high-performance Win32 API interop.
//!
//! 5. **VBA Standard Library (`rtc*`)**:
//!    - Functions like `MsgBox` (`rtcMsgBox`), `DoEvents` (`rtcDoEvents`), `RGB` (`rtcRgb`),
//!      `InStr` (`rtcInStr`), `Trim` (`rtcTrimBstr`), and `Shell` (`rtcShell`).
//!
//! 6. **Compiler Runtime Helpers (`__vba*`)**:
//!    - String allocation/copying/concatenation (`__vbaStrCopy`, `__vbaStrCat`, `__vbaFreeStr`).
//!    - Variant math and conversions (`__vbaVarAdd`, `__vbaI4Str`, `__vbaVarMove`).
//!    - Array allocation and bounds checking (`__vbaAryConstruct`, `__vbaAryLock`).
//!    - Error handling (`__vbaOnError`, `__vbaExceptHandler`).
//!
//! 7. **Calling Conventions**:
//!    - VB6 on x86 utilizes `__stdcall` (`extern "system"` in Rust) for the vast majority of exports.
//!    - Specific CRT math routines (`_CI*`, `_adj_fdiv_*`, `_allmul`) use `extern "C"`.
//!
//! 8. **Export Ordinals (`msvbvm60.def`)**:
//!    - Windows DLLs often link by ordinal. This implementation includes a complete `.def` file
//!      covering all 635 exports ranging from ordinal 100 (`ThunRTMain`) to 2010 (`EbGetVBAObject`).
//!
//! ## Trademark & Legal Notice
//!
//! Microsoft®, Windows®, Visual Basic®, Visual Studio®, MSVB, and MSVBVM60 are either
//! registered trademarks or trademarks of Microsoft Corporation in the United States,
//! Canada, and/or other countries. All other trademarks, trade names, or company names
//! referenced herein are the property of their respective owners.
//!
//! ClassicBasic is an independent open-source clean-room reimplementation created strictly
//! for educational study and software interoperability under Section 30.61 and Section 29
//! of the *Copyright Act of Canada* (R.S.C., 1985, c. C-42). It is not affiliated with,
//! endorsed by, sponsored by, or associated in any way with Microsoft Corporation.

#![allow(non_snake_case)]
#![allow(unused_variables)]

pub mod com;
pub mod err;
pub mod math;
pub mod mem;
pub mod rtc;
pub mod stubs;
pub mod thun_rt;
pub mod types;
pub mod vba;
#[cfg(feature = "std")]
pub mod pe;

use core::ffi::c_void;

// Re-export primary modules for convenient usage as a library
pub use com::*;
pub use err::*;
pub use math::*;
pub use mem::*;
pub use rtc::*;
pub use thun_rt::*;
pub use types::*;
pub use vba::*;
#[cfg(feature = "std")]
pub use pe::*;

// ----------------------------------------------------------------------------
// Windows DLL Entry Point
// ----------------------------------------------------------------------------

const DLL_PROCESS_DETACH: u32 = 0;
const DLL_PROCESS_ATTACH: u32 = 1;
const DLL_THREAD_ATTACH: u32 = 2;
const DLL_THREAD_DETACH: u32 = 3;

/// Dynamic-link library (DLL) entry point.
///
/// Called by the operating system loader when processes or threads
/// attach to or detach from `MSVBVM60.DLL`.
///
/// # Arguments
/// * `_hinst_dll` - Handle to the DLL module.
/// * `fdw_reason` - Reason code (`DLL_PROCESS_ATTACH`, `DLL_THREAD_ATTACH`, etc.).
/// * `_lp_reserved` - Reserved pointer for loader context.
///
/// # Returns
/// `1` (`TRUE`) indicating successful initialization.
#[no_mangle]
pub unsafe extern "system" fn DllMain(
    _hinst_dll: *mut c_void,
    fdw_reason: u32,
    _lp_reserved: *mut c_void,
) -> i32 {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            #[cfg(feature = "std")]
            #[cfg(feature = "trace_calls")]
            eprintln!("[msvbvm60] DLL_PROCESS_ATTACH: Visual Basic 6.0 Runtime initialized");
        }
        DLL_THREAD_ATTACH => {}
        DLL_THREAD_DETACH => {}
        DLL_PROCESS_DETACH => {
            #[cfg(feature = "std")]
            #[cfg(feature = "trace_calls")]
            eprintln!("[msvbvm60] DLL_PROCESS_DETACH: Visual Basic 6.0 Runtime uninitialized");
        }
        _ => {}
    }
    1 // TRUE (success)
}

// ----------------------------------------------------------------------------
// Unit Tests (verifying foundational data types and memory logic)
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varptr_passthrough() {
        let mut x: u32 = 0x12345678;
        let ptr = &mut x as *mut u32 as *mut c_void;
        unsafe {
            assert_eq!(VarPtr(ptr), ptr);
        }
    }

    #[test]
    fn test_getmem_putmem() {
        let mut val: u32 = 0xCAFEBABE;
        let mut dest: u32 = 0;

        unsafe {
            let res = GetMem4(&val, &mut dest);
            assert_eq!(res, S_OK);
            assert_eq!(dest, 0xCAFEBABE);

            let res_put = PutMem4(&mut val, 0x12345678);
            assert_eq!(res_put, S_OK);
            assert_eq!(val, 0x12345678);
        }
    }

    #[test]
    fn test_currency_conversion() {
        let cy = Currency::from_f64(123.4567);
        assert_eq!(cy.0, 1234567);
        let back = cy.to_f64();
        assert!((back - 123.4567).abs() < 1e-4);
    }

    #[test]
    fn test_variant_integer() {
        let v = Variant::from_i32(42);
        assert_eq!(v.vt, VT_I4);
        assert_eq!(v.as_i32(), Some(42));
    }

    #[test]
    fn test_rgb_macro() {
        unsafe {
            // In Windows COLORREF: RGB(255, 0, 128) -> 0x008000FF
            let color = rtcRgb(255, 0, 128);
            assert_eq!(color as u32, 0x008000FF);
        }
    }

    #[test]
    fn test_vbheader_magic() {
        let mut hdr = VBHeader {
            sz_magic: *b"VB5!",
            runtime_build_version: 0,
            sz_language_dll: [0; 14],
            sz_sec_lang_dll: [0; 14],
            runtime_revision: 0,
            lcid: 0x409,
            sec_lcid: 0,
            lp_sub_main: 0,
            lp_project_data: 0,
            mdl_int_pool_cache: 0,
            mdl_int_pool_cache2: 0,
            dw_thread_flags: 0,
            dw_thread_count: 1,
            w_forms_count: 1,
            w_external_component_count: 0,
            dw_thunk_count: 0,
            lp_gui_table: 0,
            lp_ide_data: 0,
            lp_project_dependencies: 0,
            lp_import_table: 0,
            lp_export_table: 0,
        };
        assert!(hdr.is_valid());

        hdr.sz_magic = *b"TEST";
        assert!(!hdr.is_valid());
    }

    #[test]
    fn test_bstr_lifecycle() {
        unsafe {
            let s = sys_alloc_string("Hello ClassicBasic");
            assert_eq!(bstr_len(s), 18);
            assert_eq!(bstr_byte_len(s), 36);
            assert_eq!(bstr_to_string(s), "Hello ClassicBasic");
            SysFreeString(s);
        }
    }

    #[test]
    fn test_bstr_concat_and_len() {
        unsafe {
            let s1 = sys_alloc_string("Classic");
            let s2 = sys_alloc_string(" ");
            let s3 = sys_alloc_string("Basic");
            let s4 = sys_alloc_string(" v6.0");

            let c1 = __vbaStrCat(s1, s2);
            let c2 = __vbaStrCat(c1, s3);
            let full = __vbaStrCat(c2, s4);

            assert_eq!(bstr_to_string(full), "Classic Basic v6.0");
            assert_eq!(__vbaLenBstr(full), 18);

            SysFreeString(s1);
            SysFreeString(s2);
            SysFreeString(s3);
            SysFreeString(s4);
            SysFreeString(c1);
            SysFreeString(c2);
            SysFreeString(full);
        }
    }

    #[test]
    fn test_bstr_mid_left_right() {
        unsafe {
            let full = sys_alloc_string("Classic Basic v6.0");

            // Test Mid: Mid$("Classic Basic v6.0", 9, 5) -> "Basic"
            let v_len = Variant::from_i32(5);
            let mid = rtcMidCharBstr(full, 9, &v_len as *const Variant);
            assert_eq!(bstr_to_string(mid), "Basic");

            // Test Left: Left$("Classic Basic v6.0", 7) -> "Classic"
            let left = rtcLeftCharBstr(full, 7);
            assert_eq!(bstr_to_string(left), "Classic");

            // Test Right: Right$("Classic Basic v6.0", 4) -> "v6.0"
            let right = rtcRightCharBstr(full, 4);
            assert_eq!(bstr_to_string(right), "v6.0");

            SysFreeString(mid);
            SysFreeString(left);
            SysFreeString(right);
            SysFreeString(full);
        }
    }

    #[test]
    fn test_bstr_instr_and_ucase_and_stri4() {
        unsafe {
            let full = sys_alloc_string("Classic Basic v6.0");
            let needle = sys_alloc_string("Basic");

            // InStr(1, full, needle, 0) -> 9
            let pos = __vbaInStr(1, full as usize, needle as usize, 0);
            assert_eq!(pos, 9);

            // UCase$("Basic") -> "BASIC"
            let upper = rtcUpperCaseBstr(needle);
            assert_eq!(bstr_to_string(upper), "BASIC");

            // Str$(123) -> "123"
            let num = __vbaStrI4(123);
            assert_eq!(bstr_to_string(num), "123");

            SysFreeString(upper);
            SysFreeString(num);
            SysFreeString(needle);
            SysFreeString(full);
        }
    }

    #[test]
    fn test_bstr_copy_and_move() {
        unsafe {
            let mut dst: Bstr = core::ptr::null_mut();
            let src = sys_alloc_string("TestString");

            __vbaStrCopy(&mut dst, src);
            assert_eq!(bstr_to_string(dst), "TestString");

            let mut moved_dst: Bstr = core::ptr::null_mut();
            __vbaStrMove(&mut moved_dst, &mut dst);
            assert_eq!(bstr_to_string(moved_dst), "TestString");
            assert!(dst.is_null()); // source is cleared on move

            SysFreeString(src);
            SysFreeString(moved_dst);
        }
    }

    #[test]
    fn test_variant_coercion_arithmetic() {
        unsafe {
            // [Test 1] VarAdd & VarSub
            let v_40 = Variant::from_i32(40);
            let v_2 = Variant::from_i32(2);
            let mut res_add = Variant::empty();
            let mut res_sub = Variant::empty();

            __vbaVarAdd(&mut res_add, &v_2, &v_40);
            __vbaVarSub(&mut res_sub, &v_2, &v_40);

            assert_eq!(res_add.as_i32(), Some(42));
            assert_eq!(res_sub.as_i32(), Some(38));

            // [Test 2] VarMul, VarDiv, VarIdiv, VarMod
            let v_25 = Variant::from_i32(25);
            let v_4 = Variant::from_i32(4);
            let mut res_mul = Variant::empty();
            let mut res_div = Variant::empty();
            let mut res_idiv = Variant::empty();
            let mut res_mod = Variant::empty();

            __vbaVarMul(&mut res_mul, &v_4, &v_25);
            __vbaVarDiv(&mut res_div, &v_4, &v_25);
            __vbaVarIdiv(&mut res_idiv, &v_4, &v_25);
            __vbaVarMod(&mut res_mod, &v_4, &v_25);

            assert_eq!(res_mul.as_i32(), Some(100));
            assert_eq!(res_div.as_f64(), Some(6.25));
            assert_eq!(res_div.to_string_representation(), "6.25");
            assert_eq!(res_idiv.as_i32(), Some(6));
            assert_eq!(res_mod.as_i32(), Some(1));
        }
    }

    #[test]
    fn test_variant_coercion_strings() {
        unsafe {
            // [Test 3] Dynamic coercion ("100" + 25 -> 125) & VarCat ("100" & 25 -> "10025")
            let s_100 = sys_alloc_string("100");
            let v_s = Variant::from_bstr(s_100);
            let v_n = Variant::from_i32(25);

            let mut res_add = Variant::empty();
            let mut res_cat = Variant::empty();

            __vbaVarAdd(&mut res_add, &v_n, &v_s);
            __vbaVarCat(&mut res_cat, &v_n, &v_s);

            let b_coercion = __vbaStrErrVarCopy(&res_add);
            let b_cat = __vbaStrErrVarCopy(&res_cat);

            assert_eq!(bstr_to_string(b_coercion), "125");
            assert_eq!(bstr_to_string(b_cat), "10025");

            SysFreeString(s_100);
            SysFreeString(b_coercion);
            SysFreeString(b_cat);
        }
    }

    #[test]
    fn test_variant_unary_and_relational() {
        unsafe {
            // [Test 4] VarNeg & VarNot
            let v_42 = Variant::from_i32(42);
            let mut res_neg = Variant::empty();
            __vbaVarNeg(&mut res_neg, &v_42);
            assert_eq!(res_neg.as_i32(), Some(-42));
            assert_eq!(res_neg.to_string_representation(), "-42");

            let v_true = Variant::from_bool(true);
            let mut res_not = Variant::empty();
            __vbaVarNot(&mut res_not, &v_true);
            assert_eq!(res_not.as_bool(), Some(false));
            assert_eq!(res_not.to_string_representation(), "False");

            // [Test 5] VarTst (50 > 30, 50 = 30)
            let v_50 = Variant::from_i32(50);
            let v_30 = Variant::from_i32(30);
            let mut res_gt = Variant::empty();
            let mut res_eq = Variant::empty();

            __vbaVarCmpGt(&mut res_gt, &v_30, &v_50);
            __vbaVarCmpEq(&mut res_eq, &v_30, &v_50);

            assert_eq!(res_gt.as_bool(), Some(true));
            assert_eq!(res_gt.to_string_representation(), "True");
            assert_eq!(res_eq.as_bool(), Some(false));
            assert_eq!(res_eq.to_string_representation(), "False");

            // [Test 6] VarType & __vbaStrI2
            let v_i10 = Variant::from_i16(10);
            let s_hello = sys_alloc_string("Hello");
            let v_str = Variant::from_bstr(s_hello);
            let v_b = Variant::from_bool(true);

            assert_eq!(rtcVarType(&v_i10), VT_I2);
            assert_eq!(rtcVarType(&v_str), VT_BSTR);
            assert_eq!(rtcVarType(&v_b), VT_BOOL);

            let b_vt1 = __vbaStrI2(rtcVarType(&v_i10) as i16);
            let b_vt2 = __vbaStrI2(rtcVarType(&v_str) as i16);
            let b_vt3 = __vbaStrI2(rtcVarType(&v_b) as i16);

            assert_eq!(bstr_to_string(b_vt1), "2");
            assert_eq!(bstr_to_string(b_vt2), "8");
            assert_eq!(bstr_to_string(b_vt3), "11");

            SysFreeString(s_hello);
            SysFreeString(b_vt1);
            SysFreeString(b_vt2);
            SysFreeString(b_vt3);
        }
    }

    #[test]
    fn test_win32_file_io_shims() {
        unsafe {
            use crate::pe::shims::*;
            use std::ffi::CString;

            let test_path = CString::new("test_unit_fileio.tmp").unwrap();
            let payload = b"Win32 File I/O Shim Test Data";

            // 1. CreateFileA (CREATE_ALWAYS)
            let h_write = win32_CreateFileA(
                test_path.as_ptr(),
                0x40000000, // GENERIC_WRITE
                0,
                core::ptr::null_mut(),
                2, // CREATE_ALWAYS
                0x80, // FILE_ATTRIBUTE_NORMAL
                0,
            );
            assert_ne!(h_write, 0xFFFFFFFF);
            assert_ne!(h_write, !0usize);

            // 2. WriteFile
            let mut bytes_written: u32 = 0;
            let write_res = win32_WriteFile(
                h_write,
                payload.as_ptr(),
                payload.len() as u32,
                &mut bytes_written,
                core::ptr::null_mut(),
            );
            assert_eq!(write_res, 1);
            assert_eq!(bytes_written as usize, payload.len());

            // 3. CloseHandle (Write)
            let close_res = win32_CloseHandle(h_write);
            assert_eq!(close_res, 1);

            // 4. CreateFileA (OPEN_EXISTING)
            let h_read = win32_CreateFileA(
                test_path.as_ptr(),
                0x80000000, // GENERIC_READ
                1, // FILE_SHARE_READ
                core::ptr::null_mut(),
                3, // OPEN_EXISTING
                0x80,
                0,
            );
            assert_ne!(h_read, 0xFFFFFFFF);

            // 5. GetFileSize
            let mut size_high: u32 = 0;
            let file_size = win32_GetFileSize(h_read, &mut size_high);
            assert_eq!(file_size as usize, payload.len());
            assert_eq!(size_high, 0);

            // 6. ReadFile
            let mut read_buf = vec![0u8; file_size as usize];
            let mut bytes_read: u32 = 0;
            let read_res = win32_ReadFile(
                h_read,
                read_buf.as_mut_ptr(),
                file_size,
                &mut bytes_read,
                core::ptr::null_mut(),
            );
            assert_eq!(read_res, 1);
            assert_eq!(bytes_read as usize, payload.len());
            assert_eq!(&read_buf[..], payload);

            // 7. CloseHandle (Read)
            let close_read = win32_CloseHandle(h_read);
            assert_eq!(close_read, 1);

            // Cleanup
            let _ = std::fs::remove_file("test_unit_fileio.tmp");

            // 8. Open Non-existent file
            let bad_path = CString::new("this_file_does_not_exist_12345.tmp").unwrap();
            let h_bad = win32_CreateFileA(
                bad_path.as_ptr(),
                0x80000000,
                0,
                core::ptr::null_mut(),
                3, // OPEN_EXISTING
                0x80,
                0,
            );
            assert_eq!(h_bad, 0xFFFFFFFF);
            assert_eq!(win32_GetLastError(), 2); // ERROR_FILE_NOT_FOUND
        }
    }

    #[test]
    fn test_error_handling_and_err_object() {
        use crate::err::*;
        use crate::types::{bstr_to_string, SysFreeString, Variant};

        unsafe {
            // 1. Test On Error modes
            __vbaOnError((-1isize) as *mut _);
            assert_eq!(get_vb_error_handler(), -1);

            __vbaOnError(0 as *mut _);
            assert_eq!(get_vb_error_handler(), 0);

            __vbaOnError(100 as *mut _);
            assert_eq!(get_vb_error_handler(), 1);

            // 2. Test Error Setting and Standard Descriptions
            set_vb_error(11, None, None);
            assert_eq!(get_vb_error_number(), 11);
            assert_eq!(get_vb_error_description(), "Division by zero");

            // 3. Test Err.Clear
            clear_vb_error();
            assert_eq!(get_vb_error_number(), 0);
            assert_eq!(get_vb_error_description(), "");

            // 4. Test COM ErrObject VTable
            let err_ptr = rtcErrObj();
            assert!(!err_ptr.is_null());
            let err_inst = &*(err_ptr as *const ErrObjectInstance);
            let vtable = &*err_inst.vtable;

            // Test QueryInterface / AddRef / Release
            let mut pv: *mut core::ffi::c_void = core::ptr::null_mut();
            let hr_qi = (vtable.QueryInterface)(err_ptr, core::ptr::null(), &mut pv);
            assert_eq!(hr_qi, 0);
            assert_eq!(pv, err_ptr);
            assert_eq!((vtable.AddRef)(err_ptr), 1);
            assert_eq!((vtable.Release)(err_ptr), 1);

            // Test get_Number / put_Number
            let mut num: i32 = 0;
            (vtable.put_Number)(err_ptr, 13);
            (vtable.get_Number)(err_ptr, &mut num);
            assert_eq!(num, 13);
            assert_eq!(get_vb_error_description(), "Type mismatch");

            // Test Raise via COM
            let b_source_raw = crate::types::sys_alloc_string("ClassicBasicTest");
            let b_desc_raw = crate::types::sys_alloc_string("Test Custom Description");
            let v_source = Variant::from_bstr(b_source_raw);
            let v_desc = Variant::from_bstr(b_desc_raw);
            (vtable.Raise)(
                err_ptr,
                1001,
                &v_source,
                &v_desc,
                core::ptr::null(),
                core::ptr::null(),
            );
            SysFreeString(b_source_raw);
            SysFreeString(b_desc_raw);
            assert_eq!(get_vb_error_number(), 1001);
            assert_eq!(get_vb_error_source(), "ClassicBasicTest");
            assert_eq!(get_vb_error_description(), "Test Custom Description");

            let mut b_src = core::ptr::null_mut();
            let mut b_desc = core::ptr::null_mut();
            (vtable.get_Source)(err_ptr, &mut b_src);
            (vtable.get_Description)(err_ptr, &mut b_desc);
            assert_eq!(bstr_to_string(b_src), "ClassicBasicTest");
            assert_eq!(bstr_to_string(b_desc), "Test Custom Description");
            SysFreeString(b_src);
            SysFreeString(b_desc);

            // Test Clear via COM
            (vtable.Clear)(err_ptr);
            assert_eq!(get_vb_error_number(), 0);

            // 5. Test compiler helpers
            __vbaResume(0);
            __vbaExitProc();
            let mut obj_slot: *mut core::ffi::c_void = core::ptr::null_mut();
            let ret_obj = __vbaObjSet(&mut obj_slot, err_ptr);
            assert_eq!(ret_obj, err_ptr);
            assert_eq!(obj_slot, err_ptr);
            __vbaFreeObj(&mut obj_slot);
            assert!(obj_slot.is_null());
            assert_eq!(__vbaHresultCheckObj(0, core::ptr::null_mut(), 0, core::ptr::null_mut()), 0);
        }
    }

    #[test]
    fn test_env_datetime_filesystem() {
        use crate::rtc::*;
        use crate::types::{bstr_to_string, sys_alloc_string, SysFreeString, Variant, VT_DATE};
        use crate::vba::*;

        unsafe {
            // 1. Environ$
            let var_name = sys_alloc_string("PATH");
            let env_val = rtcEnvironBstr(var_name);
            let env_str = bstr_to_string(env_val);
            assert!(!env_str.is_empty());
            SysFreeString(var_name);
            SysFreeString(env_val);

            // 2. CurDir$
            let cur_val = rtcCurrentDirBstr(core::ptr::null_mut());
            let cur_str = bstr_to_string(cur_val);
            assert!(!cur_str.is_empty());
            SysFreeString(cur_val);

            // 3. Date$ & Time$
            let d_val = rtcGetDateBstr();
            let t_val = rtcGetTimeBstr();
            let d_str = bstr_to_string(d_val);
            let t_str = bstr_to_string(t_val);
            assert_eq!(d_str.len(), 10);
            assert_eq!(t_str.len(), 8);
            SysFreeString(d_val);
            SysFreeString(t_val);

            // 4. Timer
            let timer_val = rtcGetTimer();
            assert!(timer_val >= 0.0 && timer_val <= 86400.0);

            // 5. Now and __vbaDateVar
            let mut v_now = Variant::empty();
            rtcGetPresentDate(&mut v_now);
            assert_eq!(v_now.vt, VT_DATE);
            let date_f64 = __vbaDateVar(&v_now);
            assert!(date_f64 > 25569.0); // Later than 1970-01-01

            // 6. MkDir & RmDir
            let dir_name = sys_alloc_string("classicbasic_unit_test_dir");
            rtcMakeDir(dir_name);
            assert_eq!(crate::err::get_vb_error_number(), 0);
            assert!(std::path::Path::new("classicbasic_unit_test_dir").exists());

            rtcRemoveDir(dir_name);
            assert_eq!(crate::err::get_vb_error_number(), 0);
            assert!(!std::path::Path::new("classicbasic_unit_test_dir").exists());
            SysFreeString(dir_name);

            // 7. Math / String helpers
            let s_r4 = __vbaStrR4(42.5);
            assert_eq!(bstr_to_string(s_r4), "42.5");
            SysFreeString(s_r4);

            assert_eq!(__vbaFPInt(42.9), 42.0);
            assert_eq!(__vbaFpR4(10.5), 10.5);
        }
    }

    #[test]
    fn test_dynamic_arrays_and_safearray() {
        unsafe {
            use crate::rtc::*;
            use crate::vba::*;

            // 1. Dynamic ReDim and Bounds
            let mut p_sa: *mut SafeArray = core::ptr::null_mut();
            __vbaRedim(0x80, 4, &mut p_sa, 3, 1, 4, 0);
            assert!(!p_sa.is_null());
            assert_eq!(__vbaLbound(p_sa, 1), 0);
            assert_eq!(__vbaUbound(p_sa, 1), 4);

            let slice = core::slice::from_raw_parts_mut((*p_sa).pv_data as *mut i32, 5);
            slice[0] = 10;
            slice[1] = 20;
            slice[2] = 30;
            slice[3] = 40;
            slice[4] = 50;
            assert_eq!(slice.iter().sum::<i32>(), 150);

            // 2. ReDim Preserve
            __vbaRedimPreserve(0x80, 4, &mut p_sa, 3, 1, 6, 0);
            assert_eq!(__vbaUbound(p_sa, 1), 6);
            let slice2 = core::slice::from_raw_parts_mut((*p_sa).pv_data as *mut i32, 7);
            assert_eq!(slice2[0], 10);
            assert_eq!(slice2[4], 50);
            slice2[5] = 60;
            slice2[6] = 70;
            assert_eq!(slice2[5], 60);
            assert_eq!(slice2[6], 70);

            // 3. Split and Join
            let raw_text = sys_alloc_string("classic,basic,linux,rust");
            let comma_bstr = sys_alloc_string(",");
            let comma_var = Variant::from_bstr(comma_bstr);
            let mut split_res = Variant::empty();
            rtcSplit(&mut split_res, raw_text, &comma_var, -1, 0);

            let p_split_sa = __vbaAryVar(&split_res, 0);
            assert!(!p_split_sa.is_null());
            assert_eq!(__vbaUbound(p_split_sa, 1) - __vbaLbound(p_split_sa, 1) + 1, 4);

            let dash_bstr = sys_alloc_string("-");
            let dash_var = Variant::from_bstr(dash_bstr);
            let joined_bstr = rtcJoin(&split_res, &dash_var);
            assert_eq!(bstr_to_string(joined_bstr), "classic-basic-linux-rust");

            // 4. Filter
            let mut filter_res = Variant::empty();
            let filter_target = sys_alloc_string("linux");
            rtcFilter(&mut filter_res, &split_res, filter_target, -1, 0);
            let p_filt_sa = __vbaAryVar(&filter_res, 0);
            assert_eq!(__vbaUbound(p_filt_sa, 1), 0);
            let filt_slice = core::slice::from_raw_parts((*p_filt_sa).pv_data as *const Bstr, 1);
            assert_eq!(bstr_to_string(filt_slice[0]), "linux");

            // 5. Variant Array()
            let arr_items = [Variant::from_i32(100), Variant::from_i32(200), Variant::from_i32(300)];
            let sa_var = SafeArray::create_vector(
                core::mem::size_of::<Variant>() as u32,
                0,
                3,
                FADF_VARIANT | FADF_HAVEVARTYPE,
            );
            core::ptr::copy_nonoverlapping(arr_items.as_ptr(), (*sa_var).pv_data as *mut Variant, 3);
            let mut v_list = Variant::empty();
            rtcArray(&mut v_list, sa_var as *const core::ffi::c_void);

            let v_sa = __vbaAryVar(&v_list, 0);
            assert_eq!(__vbaLbound(v_sa, 1), 0);
            assert_eq!(__vbaUbound(v_sa, 1), 2);
            let mut item1 = Variant::empty();
            __vbaVarIndexLoad(&mut item1, &v_list, 1, 1);
            assert_eq!(item1.as_i32(), Some(200));

            // 6. VarAnd, VarDup, BoolVarNull
            let mut and_res = Variant::empty();
            let v1 = Variant::from_i32(0xFF00);
            let v2 = Variant::from_i32(0x0FF0);
            __vbaVarAnd(&mut and_res, &v1, &v2);
            assert_eq!(and_res.as_i32(), Some(0x0F00));

            let mut dup_res = Variant::empty();
            __vbaVarDup(&mut dup_res, &v1);
            assert_eq!(dup_res.as_i32(), Some(0xFF00));

            assert_eq!(__vbaBoolVarNull(&Variant::from_bool(true)), -1);
            assert_eq!(__vbaBoolVarNull(&Variant::from_bool(false)), 0);

            // 7. Erase
            __vbaErase(&mut p_sa, 0);
            assert!(p_sa.is_null());

            // Cleanup
            SysFreeString(raw_text);
            SysFreeString(comma_bstr);
            SysFreeString(dash_bstr);
            SysFreeString(filter_target);
            SysFreeString(joined_bstr);
        }
    }
}
