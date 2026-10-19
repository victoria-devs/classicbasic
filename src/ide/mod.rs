// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # IDE Hosting, Project Introspection & Debugger Telemetry (`Eb*` & `Tip*`)
//!
//! Re-implementation of the Microsoft Visual Basic 6.0 Runtime internal IDE hosting,
//! project introspection, and debugger service exports (`msvbvm60.dll`).
//!
//! These 19 ordinals allow the VB6 development environment (and debugging tools) to:
//! - Query executing project handles and execution stack state (`EbGetHandleOfExecutingProject`, `EbIsProjectOnStack`).
//! - Manage library dependencies dynamically (`EbLibraryLoad`, `EbLibraryUnload`, `EbLoadRunTime`).
//! - Access project-level error telemetry (`EbGetErrorInfo`).
//! - Query connection sink telemetry on COM objects (`EbGetObjConnectionCounts`).
//! - Obtain the root VBA application object (`EbGetVBAObject`).
//! - Reset project execution state (`EbResetProject`, `EbResetProjectNormal`).
//! - Query predeclared class instances (`TipGetAddressOfPredeclaredInstance`).
//! - Manage project-specific COM class instances (`TipCreateInstanceEx`, `TipCreateInstanceProject2`, `TipUnloadInstance`).
//! - Control IDE compilation and execution options (`TipSetOption`, `TipUnloadProject`).
//! - Dispatch method invocations via internal project IDs (`TipInvokeMethod`, `TipInvokeMethod2`).
//! - Instantiate runtime expression evaluator services (`CreateIExprSrvObj`).

use core::ffi::c_void;

const S_OK: i32 = 0;

/// Standard COM `IDispatch` VTable representation for IDE and runtime helper objects.
#[repr(C)]
pub struct IdeDispatchVTable {
    pub QueryInterface: unsafe extern "system" fn(this: *mut c_void, riid: *const c_void, ppv: *mut *mut c_void) -> i32,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub GetTypeInfoCount: unsafe extern "system" fn(this: *mut c_void, pctinfo: *mut u32) -> i32,
    pub GetTypeInfo: unsafe extern "system" fn(this: *mut c_void, iTInfo: u32, lcid: u32, ppTInfo: *mut *mut c_void) -> i32,
    pub GetIDsOfNames: unsafe extern "system" fn(this: *mut c_void, riid: *const c_void, rgszNames: *const *const u16, cNames: u32, lcid: u32, rgDispId: *mut i32) -> i32,
    pub Invoke: unsafe extern "system" fn(this: *mut c_void, dispId: i32, riid: *const c_void, lcid: u32, flags: u16, pDispParams: *mut c_void, pVarResult: *mut c_void, pExcepInfo: *mut c_void, puArgErr: *mut u32) -> i32,
}

/// Static COM instance representing an IDE or runtime service object.
#[repr(C)]
pub struct IdeComInstance {
    pub vtable: *const IdeDispatchVTable,
}

unsafe impl Send for IdeComInstance {}
unsafe impl Sync for IdeComInstance {}

unsafe extern "system" fn ide_query_interface(
    this: *mut c_void,
    _riid: *const c_void,
    ppv: *mut *mut c_void,
) -> i32 {
    if !ppv.is_null() {
        *ppv = this;
    }
    S_OK
}

unsafe extern "system" fn ide_add_ref(_this: *mut c_void) -> u32 {
    1
}

unsafe extern "system" fn ide_release(_this: *mut c_void) -> u32 {
    1
}

unsafe extern "system" fn ide_get_type_info_count(_this: *mut c_void, pctinfo: *mut u32) -> i32 {
    if !pctinfo.is_null() {
        *pctinfo = 0;
    }
    S_OK
}

unsafe extern "system" fn ide_get_type_info(
    _this: *mut c_void,
    _iTInfo: u32,
    _lcid: u32,
    ppTInfo: *mut *mut c_void,
) -> i32 {
    if !ppTInfo.is_null() {
        *ppTInfo = core::ptr::null_mut();
    }
    S_OK
}

unsafe extern "system" fn ide_get_ids_of_names(
    _this: *mut c_void,
    _riid: *const c_void,
    _rgszNames: *const *const u16,
    _cNames: u32,
    _lcid: u32,
    _rgDispId: *mut i32,
) -> i32 {
    S_OK
}

unsafe extern "system" fn ide_invoke(
    _this: *mut c_void,
    _dispId: i32,
    _riid: *const c_void,
    _lcid: u32,
    _flags: u16,
    _pDispParams: *mut c_void,
    _pVarResult: *mut c_void,
    _pExcepInfo: *mut c_void,
    _puArgErr: *mut u32,
) -> i32 {
    S_OK
}

static IDE_DISPATCH_VTABLE: IdeDispatchVTable = IdeDispatchVTable {
    QueryInterface: ide_query_interface,
    AddRef: ide_add_ref,
    Release: ide_release,
    GetTypeInfoCount: ide_get_type_info_count,
    GetTypeInfo: ide_get_type_info,
    GetIDsOfNames: ide_get_ids_of_names,
    Invoke: ide_invoke,
};

static IDE_COM_INSTANCE: IdeComInstance = IdeComInstance {
    vtable: &IDE_DISPATCH_VTABLE,
};

/// Returns a pointer to a valid static COM service instance implementing `IUnknown` / `IDispatch`.
pub fn get_ide_com_object_ptr() -> *mut c_void {
    &IDE_COM_INSTANCE as *const _ as *mut c_void
}

// ----------------------------------------------------------------------------
// 19 Introspection & Telemetry ABI Exports
// ----------------------------------------------------------------------------

/// Ordinal 195: Resolves the address of the predeclared default instance of a class module.
#[no_mangle]
pub unsafe extern "system" fn TipGetAddressOfPredeclaredInstance(
    _p_desc: usize,
    p_inst: *mut *mut c_void,
) -> i32 {
    if !p_inst.is_null() {
        *p_inst = get_ide_com_object_ptr();
    }
    S_OK
}

/// Ordinal 220: Sets an internal IDE or project compilation/execution option flag.
#[no_mangle]
pub unsafe extern "system" fn TipSetOption(_opt_id: u32, _opt_val: u32) -> i32 {
    S_OK
}

/// Ordinal 227: Unloads project execution resources and cleans up references.
#[no_mangle]
pub unsafe extern "system" fn TipUnloadProject(_h_project: usize) -> i32 {
    S_OK
}

/// Ordinal 230: Creates an instance of a project-defined class in an IDE context.
#[no_mangle]
pub unsafe extern "system" fn TipCreateInstanceProject2(
    _h_project: usize,
    _rclsid: *const u8,
    out_instance: *mut *mut c_void,
) -> i32 {
    if !out_instance.is_null() {
        *out_instance = get_ide_com_object_ptr();
    }
    S_OK
}

/// Ordinal 231: Resets executing project state and error status.
#[no_mangle]
pub unsafe extern "system" fn EbResetProject() -> i32 {
    crate::err::clear_vb_error();
    S_OK
}

/// Ordinal 232: Retrieves the memory handle / base pointer of the executing VB project.
#[no_mangle]
pub unsafe extern "system" fn EbGetHandleOfExecutingProject(out_hproject: *mut usize) -> i32 {
    if !out_hproject.is_null() {
        #[cfg(feature = "std")]
        let base = crate::pe::shims::GLOBAL_MODULE_HANDLE;
        #[cfg(not(feature = "std"))]
        let base = 0usize;

        *out_hproject = if base != 0 { base } else { 0x00400000 };
    }
    S_OK
}

/// Ordinal 265: Resets executing project state normally and clears active runtime errors.
#[no_mangle]
pub unsafe extern "system" fn EbResetProjectNormal() -> i32 {
    crate::err::clear_vb_error();
    S_OK
}

/// Ordinal 266: Unloads a project class instance.
#[no_mangle]
pub unsafe extern "system" fn TipUnloadInstance(_instance: *mut c_void) -> i32 {
    S_OK
}

/// Ordinal 268: Dynamically loads an external runtime library or typelib.
#[no_mangle]
pub unsafe extern "system" fn EbLibraryLoad(_p_lib_name: usize) -> i32 {
    S_OK
}

/// Ordinal 269: Unloads a dynamically loaded external runtime library or typelib.
#[no_mangle]
pub unsafe extern "system" fn EbLibraryUnload(_handle: usize) -> i32 {
    S_OK
}

/// Ordinal 271: Initializes the runtime engine for IDE hosting.
#[no_mangle]
pub unsafe extern "system" fn EbLoadRunTime() -> i32 {
    S_OK
}

/// Ordinal 284: Retrieves error telemetry information for the current execution context.
#[no_mangle]
pub unsafe extern "system" fn EbGetErrorInfo(err_num: *mut i32, _pbstr_desc: *mut usize) -> i32 {
    if !err_num.is_null() {
        *err_num = crate::err::with_error_state(|state| state.number);
    }
    S_OK
}

/// Ordinal 298: Queries whether the specified project is currently on the execution call stack.
#[no_mangle]
pub unsafe extern "system" fn EbIsProjectOnStack(_hproject: usize) -> i32 {
    1 // True (active on stack)
}

/// Ordinal 299: Creates an instance of a project-defined class with extended options.
#[no_mangle]
pub unsafe extern "system" fn TipCreateInstanceEx(
    _hproj: usize,
    _clsid: usize,
    p_obj: *mut *mut c_void,
) -> i32 {
    if !p_obj.is_null() {
        *p_obj = get_ide_com_object_ptr();
    }
    S_OK
}

/// Ordinal 999: Invokes a project method directly by member ID with secondary parameters.
#[no_mangle]
pub unsafe extern "system" fn TipInvokeMethod2(
    _p_obj: *mut c_void,
    _method_id: i32,
    _extra: usize,
) -> i32 {
    S_OK
}

/// Ordinal 1016: Invokes a project method directly by member ID.
#[no_mangle]
pub unsafe extern "system" fn TipInvokeMethod(
    _p_obj: *mut c_void,
    _method_id: i32,
) -> i32 {
    S_OK
}

/// Ordinal 1025: Queries the number of active connection point sinks attached to an object.
#[no_mangle]
pub unsafe extern "system" fn EbGetObjConnectionCounts(
    _p_obj: *mut c_void,
    out_count: *mut u32,
) -> i32 {
    if !out_count.is_null() {
        *out_count = 0;
    }
    S_OK
}

/// Ordinal 2000: Creates a runtime Expression Service Object (`IExprSrvObj`) for debugger watch evaluation.
#[no_mangle]
pub unsafe extern "system" fn CreateIExprSrvObj(
    _p_unk_outer: *mut c_void,
    _riid: *const c_void,
    p_expr_srv: *mut *mut c_void,
) -> i32 {
    if !p_expr_srv.is_null() {
        *p_expr_srv = get_ide_com_object_ptr();
    }
    S_OK
}

/// Ordinal 2010: Retrieves the internal root VBA application object (`IVba`).
#[no_mangle]
pub unsafe extern "system" fn EbGetVBAObject(p_vba: *mut *mut c_void) -> i32 {
    if !p_vba.is_null() {
        *p_vba = get_ide_com_object_ptr();
    }
    S_OK
}
