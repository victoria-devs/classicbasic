// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # IDE Hosting, Project Introspection & Debugger Telemetry Tests (Phase 2.16)
//!
//! Validates the clean-room implementation of all 19 final MSVBVM60 exports:
//! - Project handles and execution stack state (`EbGetHandleOfExecutingProject`, `EbIsProjectOnStack`)
//! - Runtime initialization & dynamic library management (`EbLoadRunTime`, `EbLibraryLoad`, `EbLibraryUnload`)
//! - Project execution reset (`EbResetProject`, `EbResetProjectNormal`)
//! - Error telemetry integration (`EbGetErrorInfo`)
//! - Connection sink telemetry (`EbGetObjConnectionCounts`)
//! - Root VBA application object (`EbGetVBAObject`)
//! - Predeclared class instances (`TipGetAddressOfPredeclaredInstance`)
//! - Project-level COM instantiation (`TipCreateInstanceEx`, `TipCreateInstanceProject2`, `TipUnloadInstance`)
//! - IDE compiler options (`TipSetOption`, `TipUnloadProject`)
//! - Member method dispatch via disp-id (`TipInvokeMethod`, `TipInvokeMethod2`)
//! - Debugger Expression Service Factory (`CreateIExprSrvObj`)
//! - Resolver mapping by both Ordinal and Exported Name

use core::ffi::c_void;
use msvbvm60::ide::*;
use msvbvm60::pe::resolver::resolve_symbol;

#[test]
fn test_project_handle_and_stack_telemetry() {
    unsafe {
        let mut hproj = 0usize;
        let hr = EbGetHandleOfExecutingProject(&mut hproj);
        assert_eq!(hr, 0, "EbGetHandleOfExecutingProject must return S_OK");
        assert_ne!(hproj, 0, "Executing project handle must be non-zero");

        let on_stack = EbIsProjectOnStack(hproj);
        assert_eq!(on_stack, 1, "Executing project must be on the call stack");

        // Null pointer handling should succeed gracefully
        assert_eq!(EbGetHandleOfExecutingProject(core::ptr::null_mut()), 0);
    }
}

#[test]
fn test_predeclared_class_instance_resolution() {
    unsafe {
        let mut p_inst: *mut c_void = core::ptr::null_mut();
        let hr = TipGetAddressOfPredeclaredInstance(0x1234, &mut p_inst);
        assert_eq!(hr, 0);
        assert!(!p_inst.is_null());

        // Validate COM VTable on returned instance
        let vtable_ptr = *(p_inst as *const *const IdeDispatchVTable);
        assert!(!vtable_ptr.is_null());

        // QueryInterface
        let mut ppv: *mut c_void = core::ptr::null_mut();
        assert_eq!(((*vtable_ptr).QueryInterface)(p_inst, core::ptr::null(), &mut ppv), 0);
        assert_eq!(ppv, p_inst);

        // AddRef & Release
        assert_eq!(((*vtable_ptr).AddRef)(p_inst), 1);
        assert_eq!(((*vtable_ptr).Release)(p_inst), 1);
    }
}

#[test]
fn test_connection_counts_and_telemetry() {
    unsafe {
        let mut count: u32 = 999;
        let hr = EbGetObjConnectionCounts(core::ptr::null_mut(), &mut count);
        assert_eq!(hr, 0);
        assert_eq!(count, 0);

        // Null pointer check
        assert_eq!(EbGetObjConnectionCounts(core::ptr::null_mut(), core::ptr::null_mut()), 0);
    }
}

#[test]
fn test_expression_service_and_vba_root_object() {
    unsafe {
        let mut p_expr: *mut c_void = core::ptr::null_mut();
        let hr = CreateIExprSrvObj(core::ptr::null_mut(), core::ptr::null(), &mut p_expr);
        assert_eq!(hr, 0);
        assert!(!p_expr.is_null());

        let mut p_vba: *mut c_void = core::ptr::null_mut();
        let hr = EbGetVBAObject(&mut p_vba);
        assert_eq!(hr, 0);
        assert!(!p_vba.is_null());
    }
}

#[test]
fn test_tip_project_options_and_instance_lifecycle() {
    unsafe {
        assert_eq!(TipSetOption(1, 100), 0);
        assert_eq!(TipUnloadProject(0x1000), 0);

        let mut obj1: *mut c_void = core::ptr::null_mut();
        assert_eq!(TipCreateInstanceEx(0, 0, &mut obj1), 0);
        assert!(!obj1.is_null());

        let mut obj2: *mut c_void = core::ptr::null_mut();
        assert_eq!(TipCreateInstanceProject2(0, core::ptr::null(), &mut obj2), 0);
        assert!(!obj2.is_null());

        // Invoke method
        assert_eq!(TipInvokeMethod(obj1, 10), 0);
        assert_eq!(TipInvokeMethod2(obj2, 20, 0), 0);

        // Unload instance
        assert_eq!(TipUnloadInstance(obj1), 0);
        assert_eq!(TipUnloadInstance(obj2), 0);
    }
}

#[test]
fn test_library_management_error_info_and_project_reset() {
    unsafe {
        assert_eq!(EbLoadRunTime(), 0);
        assert_eq!(EbLibraryLoad(0), 0);
        assert_eq!(EbLibraryUnload(0), 0);

        // Error info tracking
        msvbvm60::err::set_vb_error(53, Some("File not found test"), None);
        let mut err_num = 0i32;
        assert_eq!(EbGetErrorInfo(&mut err_num, core::ptr::null_mut()), 0);
        assert_eq!(err_num, 53);

        // Project reset clears error state
        assert_eq!(EbResetProjectNormal(), 0);
        assert_eq!(msvbvm60::err::get_vb_error_number(), 0);

        msvbvm60::err::set_vb_error(11, None, None);
        assert_eq!(EbResetProject(), 0);
        assert_eq!(msvbvm60::err::get_vb_error_number(), 0);
    }
}

#[test]
fn test_resolver_mappings_all_19_ordinals() {
    let ordinals = [
        (195, "TipGetAddressOfPredeclaredInstance"),
        (220, "TipSetOption"),
        (227, "TipUnloadProject"),
        (230, "TipCreateInstanceProject2"),
        (231, "EbResetProject"),
        (232, "EbGetHandleOfExecutingProject"),
        (265, "EbResetProjectNormal"),
        (266, "TipUnloadInstance"),
        (268, "EbLibraryLoad"),
        (269, "EbLibraryUnload"),
        (271, "EbLoadRunTime"),
        (284, "EbGetErrorInfo"),
        (298, "EbIsProjectOnStack"),
        (299, "TipCreateInstanceEx"),
        (999, "TipInvokeMethod2"),
        (1016, "TipInvokeMethod"),
        (1025, "EbGetObjConnectionCounts"),
        (2000, "CreateIExprSrvObj"),
        (2010, "EbGetVBAObject"),
    ];

    for (ord, name) in ordinals {
        let addr_by_ord = resolve_symbol("msvbvm60.dll", None, Some(ord));
        assert_ne!(addr_by_ord, 0, "Ordinal {} must resolve to a valid function address", ord);

        let addr_by_name = resolve_symbol("msvbvm60.dll", Some(name), None);
        assert_ne!(addr_by_name, 0, "Symbol {} must resolve to a valid function address", name);

        assert_eq!(
            addr_by_ord, addr_by_name,
            "Ordinal {} and Symbol {} must resolve to the exact same function entry point",
            ord, name
        );
    }
}
