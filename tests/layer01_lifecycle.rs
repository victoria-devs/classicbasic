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

#[test]
fn test_vba_end_flushes_and_closes_all_active_virtual_file_channels() {
    unsafe {
        // GIVEN channels #1, #2, and #3 are actively open with unwritten pending buffer data
        let p1 = sys_alloc_string("test_end_chan1.txt");
        let p2 = sys_alloc_string("test_end_chan2.txt");
        let p3 = sys_alloc_string("test_end_chan3.txt");

        assert_eq!(__vbaFileOpen(0x02, -1, 1, p1), 0);
        assert_eq!(__vbaFileOpen(0x02, -1, 2, p2), 0);
        assert_eq!(__vbaFileOpen(0x02, -1, 3, p3), 0);

        let text1 = sys_alloc_string("Channel 1 pending data");
        assert_eq!(__vbaPrintFile(PRINT_NEWLINE as *const _, 1, text1), 0);
        SysFreeString(text1);

        set_runtime_state(RuntimeState::Running);
        set_exit_hook(Some(|_code: i32| {}));

        // WHEN __vbaEnd (or rtcEnd) is invoked
        __vbaEnd();

        // Reset hook
        set_exit_hook(None::<fn(i32)>);

        // THEN write buffers for all virtual channels are flushed to disk
        // AND all OS file handles are closed cleanly (preventing descriptor leaks)
        let path1 = std::path::Path::new("test_end_chan1.txt");
        assert!(path1.exists());
        let content1 = std::fs::read_to_string(path1).unwrap();
        assert!(content1.contains("Channel 1 pending data"));

        let eof_res = rtcEndOfFile(1);
        assert_eq!(eof_res, -1);
        assert_eq!(msvbvm60::err::get_vb_error_number(), 52);

        // AND internal runtime state transitions to Terminated
        assert_eq!(get_runtime_state(), RuntimeState::Terminated);

        // Cleanup
        rtcKillFiles(p1);
        rtcKillFiles(p2);
        rtcKillFiles(p3);
        SysFreeString(p1);
        SysFreeString(p2);
        SysFreeString(p3);
    }
}

#[test]
fn test_vba_end_exits_process_with_zero_status_code() {
    // GIVEN a running VB6 execution context
    // WHEN the program executes the End statement
    // THEN execution stops immediately without falling through to adjacent instructions
    // AND the exit code returned to the host environment is 0
    let runner = env!("CARGO_BIN_EXE_vb6-run");
    let binary_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("artifacts")
        .join("App10-core-primitives-native.exe");

    let output = std::process::Command::new(runner)
        .arg(&binary_path)
        .output()
        .expect("Failed to execute runner");

    assert_eq!(output.status.code(), Some(0));
}
