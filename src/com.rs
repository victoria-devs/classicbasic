//! COM Automation and ActiveX DLL export implementations.
//!
//! Provides the standard COM in-process server lifecycle exports (`DllGetClassObject`,
//! `DllCanUnloadNow`, etc.) as well as the internal VB6 class, event sink, and zombie
//! object dispatch helpers.

use core::ffi::c_void;
use crate::types::{CLASS_E_CLASSNOTAVAILABLE, Clsid, E_POINTER, Guid, HResult, Iid, S_OK};

/// COM Interface ID for `IVbaHost`: {1D6F7D1E-EACD-11CD-9570-00AA0051E5D4}
#[no_mangle]
pub static IID_IVbaHost: Guid = Guid::new(
    0x1D6F7D1E,
    0xEACD,
    0x11CD,
    [0x95, 0x70, 0x00, 0xAA, 0x00, 0x51, 0xE5, 0xD4],
);

// ----------------------------------------------------------------------------
// COM In-Process Server Lifecycle
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "system" fn DllCanUnloadNow() -> HResult {
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    _rclsid: *const Clsid,
    _riid: *const Iid,
    ppv: *mut *mut c_void,
) -> HResult {
    if ppv.is_null() {
        return E_POINTER;
    }
    *ppv = core::ptr::null_mut();
    CLASS_E_CLASSNOTAVAILABLE
}

#[no_mangle]
pub unsafe extern "system" fn DllRegisterServer() -> HResult {
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn DllUnregisterServer() -> HResult {
    S_OK
}

#[repr(C)]
pub struct DllFunctionCallInfo {
    pub lp_dll_name: *const i8,
    pub lp_function_name: *const i8,
    pub dw_flags: u32,
    pub lp_fn_cache: *mut usize,
}

#[no_mangle]
pub unsafe extern "system" fn DllFunctionCall(info: *mut DllFunctionCallInfo) -> usize {
    if info.is_null() {
        return 0;
    }
    let info = &*info;
    if info.lp_dll_name.is_null() || info.lp_function_name.is_null() {
        return 0;
    }
    let dll_name = core::ffi::CStr::from_ptr(info.lp_dll_name).to_string_lossy();
    let func_name = core::ffi::CStr::from_ptr(info.lp_function_name).to_string_lossy();
    let pfn = crate::pe::resolver::resolve_symbol(&dll_name, Some(&func_name), None);
    if !info.lp_fn_cache.is_null() {
        *info.lp_fn_cache = pfn;
    }
    pfn
}

#[no_mangle]
pub unsafe extern "system" fn DLLGetDocumentation() -> HResult {
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn UserDllMain(
    _hinst_dll: *mut c_void,
    _fdw_reason: u32,
    _lp_reserved: *mut c_void,
) -> i32 {
    1
}

#[no_mangle]
pub unsafe extern "system" fn VBDllCanUnloadNow() -> HResult {
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn VBDllGetClassObject(
    rclsid: *const Clsid,
    riid: *const Iid,
    ppv: *mut *mut c_void,
) -> HResult {
    DllGetClassObject(rclsid, riid, ppv)
}

#[no_mangle]
pub unsafe extern "system" fn VBDllRegisterServer() -> HResult {
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn VBDllUnRegisterServer() -> HResult {
    S_OK
}

// ----------------------------------------------------------------------------
// BASIC_CLASS Dispatch Implementations
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "system" fn BASIC_CLASS_AddRef(_this: *mut c_void) -> u32 {
    1
}

#[no_mangle]
pub unsafe extern "system" fn BASIC_CLASS_Release(_this: *mut c_void) -> u32 {
    1
}

#[no_mangle]
pub unsafe extern "system" fn BASIC_CLASS_QueryInterface(
    _this: *mut c_void,
    _riid: *const Iid,
    ppv: *mut *mut c_void,
) -> HResult {
    if ppv.is_null() {
        return E_POINTER;
    }
    *ppv = core::ptr::null_mut();
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn BASIC_CLASS_GetIDsOfNames() -> HResult {
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn BASIC_CLASS_Invoke() -> HResult {
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn BASIC_DISPINTERFACE_GetTICount() -> HResult {
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn BASIC_DISPINTERFACE_GetTypeInfo() -> HResult {
    S_OK
}

// ----------------------------------------------------------------------------
// EVENT_SINK Dispatch Implementations
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "system" fn EVENT_SINK_AddRef(_this: *mut c_void) -> u32 {
    1
}

#[no_mangle]
pub unsafe extern "system" fn EVENT_SINK_Release(_this: *mut c_void) -> u32 {
    1
}

#[no_mangle]
pub unsafe extern "system" fn EVENT_SINK_QueryInterface(
    _this: *mut c_void,
    _riid: *const Iid,
    ppv: *mut *mut c_void,
) -> HResult {
    if ppv.is_null() {
        return E_POINTER;
    }
    *ppv = core::ptr::null_mut();
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn EVENT_SINK_GetIDsOfNames() -> HResult {
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn EVENT_SINK_Invoke() -> HResult {
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn EVENT_SINK2_AddRef(_this: *mut c_void) -> u32 {
    1
}

#[no_mangle]
pub unsafe extern "system" fn EVENT_SINK2_Release(_this: *mut c_void) -> u32 {
    1
}

// ----------------------------------------------------------------------------
// Zombie Dispatch Implementations (Dead COM references)
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "system" fn Zombie_AddRef(_this: *mut c_void) -> u32 {
    1
}

#[no_mangle]
pub unsafe extern "system" fn Zombie_Release(_this: *mut c_void) -> u32 {
    1
}

#[no_mangle]
pub unsafe extern "system" fn Zombie_QueryInterface(
    _this: *mut c_void,
    _riid: *const Iid,
    ppv: *mut *mut c_void,
) -> HResult {
    if ppv.is_null() {
        return E_POINTER;
    }
    *ppv = core::ptr::null_mut();
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn Zombie_GetIDsOfNames() -> HResult {
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn Zombie_GetTypeInfo() -> HResult {
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn Zombie_GetTypeInfoCount() -> HResult {
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn Zombie_Invoke() -> HResult {
    S_OK
}
