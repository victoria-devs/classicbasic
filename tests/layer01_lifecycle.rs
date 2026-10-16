//! Layer 01: Application Lifecycle & Memory Primitives Tests
//!
//! Verifies VarPtr, GetMem/PutMem pointer access, VBHeader integrity,
//! and runtime initialization lifecycle corresponding to App01-hello-world.

use core::ffi::c_void;
use msvbvm60::*;

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
fn test_vbheader_valid_magic() {
    let hdr = VBHeader {
        sz_magic: *b"VB5!",
        runtime_build_version: 0x1234,
        sz_language_dll: [0; 14],
        sz_sec_lang_dll: [0; 14],
        runtime_revision: 1,
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
}

#[test]
fn test_vbheader_invalid_magic() {
    let mut hdr = VBHeader {
        sz_magic: *b"BAD!",
        runtime_build_version: 0,
        sz_language_dll: [0; 14],
        sz_sec_lang_dll: [0; 14],
        runtime_revision: 0,
        lcid: 0,
        sec_lcid: 0,
        lp_sub_main: 0,
        lp_project_data: 0,
        mdl_int_pool_cache: 0,
        mdl_int_pool_cache2: 0,
        dw_thread_flags: 0,
        dw_thread_count: 0,
        w_forms_count: 0,
        w_external_component_count: 0,
        dw_thunk_count: 0,
        lp_gui_table: 0,
        lp_ide_data: 0,
        lp_project_dependencies: 0,
        lp_import_table: 0,
        lp_export_table: 0,
    };
    assert!(!hdr.is_valid());
    hdr.sz_magic = *b"VB5!";
    assert!(hdr.is_valid());
}

#[test]
fn test_dllmain_process_attach_and_detach() {
    unsafe {
        assert_eq!(DllMain(core::ptr::null_mut(), 1, core::ptr::null_mut()), 1);
        assert_eq!(DllMain(core::ptr::null_mut(), 0, core::ptr::null_mut()), 1);
    }
}
