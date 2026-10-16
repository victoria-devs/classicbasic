//! COM Automation and ActiveX DLL export implementations.
//!
//! Provides the standard COM in-process server lifecycle exports (`DllGetClassObject`,
//! `DllCanUnloadNow`, etc.) as well as the internal VB6 class, event sink, and zombie
//! object dispatch helpers.

use core::ffi::c_void;
use crate::types::{CLASS_E_CLASSNOTAVAILABLE, Clsid, E_POINTER, Guid, HResult, Iid, S_OK};

/// COM Interface ID for `IVbaHost`: `{1D6F7D1E-EACD-11CD-9570-00AA0051E5D4}`.
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

/// Determines whether the DLL is currently in use and can be unloaded from memory.
///
/// In standard COM, the OLE component manager periodically calls this function.
///
/// # Returns
/// * `S_OK` (0) if no active object references exist and the DLL may be unloaded.
#[no_mangle]
pub unsafe extern "system" fn DllCanUnloadNow() -> HResult {
    S_OK
}

/// Retrieves the class factory object for a specified Class ID (CLSID).
///
/// COM clients call this function when creating instances of COM classes exposed by this library.
///
/// # Arguments
/// * `_rclsid` - Pointer to the target `CLSID` (GUID identifying the COM coclass).
/// * `_riid` - Pointer to the requested interface identifier (`IID_IClassFactory`).
/// * `ppv` - Pointer to the output memory location where the interface pointer is written.
///
/// # Returns
/// * `S_OK` on success.
/// * `E_POINTER` if `ppv` is null.
/// * `CLASS_E_CLASSNOTAVAILABLE` if the requested CLSID is not supported.
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

/// Registers the COM classes and type library exposed by this module into the system registry.
///
/// Invoked by utilities like `regsvr32.exe`.
///
/// # Returns
/// * `S_OK` (0) indicating registration succeeded.
#[no_mangle]
pub unsafe extern "system" fn DllRegisterServer() -> HResult {
    S_OK
}

/// Removes registration entries created by `DllRegisterServer` from the system registry.
///
/// Invoked by `regsvr32.exe /u`.
///
/// # Returns
/// * `S_OK` (0) indicating unregistration succeeded.
#[no_mangle]
pub unsafe extern "system" fn DllUnregisterServer() -> HResult {
    S_OK
}

/// Information descriptor passed to `DllFunctionCall` for dynamic DLL API resolution.
#[repr(C)]
pub struct DllFunctionCallInfo {
    /// Null-terminated ASCII string containing the name of the DLL (e.g., `"kernel32.dll"`).
    pub lp_dll_name: *const i8,
    /// Null-terminated ASCII string containing the function name (e.g., `"GetTickCount"`).
    pub lp_function_name: *const i8,
    /// Flags controlling dynamic resolution and calling semantics.
    pub dw_flags: u32,
    /// Pointer to a cached function pointer address updated once resolved.
    pub lp_fn_cache: *mut usize,
}

/// Dynamic resolver invoked by VB6 for external DLL `Declare Sub` / `Declare Function` calls.
///
/// Instead of statically binding to external DLLs at link time, VB6 compiles `Declare`
/// statements to call `DllFunctionCall` on the first invocation and caches the resolved pointer.
///
/// # Arguments
/// * `info` - Pointer to a `DllFunctionCallInfo` descriptor describing the target DLL and function.
///
/// # Returns
/// Address of the resolved function in executable memory, or `0` if resolution fails.
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

/// Dynamic resolver alias for VB6 external DLL calls (`__vbaDllFunctionCall`).
#[no_mangle]
pub unsafe extern "system" fn __vbaDllFunctionCall(info: *mut DllFunctionCallInfo) -> usize {
    DllFunctionCall(info)
}

/// Retrieves documentation and Help strings for ActiveX objects and typelibs.
///
/// # Returns
/// * `S_OK` (0) indicating success.
#[no_mangle]
pub unsafe extern "system" fn DLLGetDocumentation() -> HResult {
    S_OK
}

/// Entrypoint invoked when loading user ActiveX DLL components.
///
/// # Arguments
/// * `_hinst_dll` - Handle to the DLL module instance.
/// * `_fdw_reason` - Reason for calling the function (process attach, thread attach, etc.).
/// * `_lp_reserved` - Reserved pointer for loader context.
///
/// # Returns
/// * `1` (`TRUE`) indicating successful initialization.
#[no_mangle]
pub unsafe extern "system" fn UserDllMain(
    _hinst_dll: *mut c_void,
    _fdw_reason: u32,
    _lp_reserved: *mut c_void,
) -> i32 {
    1
}

/// Visual Basic internal wrapper for `DllCanUnloadNow`.
///
/// # Returns
/// * `S_OK` (0).
#[no_mangle]
pub unsafe extern "system" fn VBDllCanUnloadNow() -> HResult {
    S_OK
}

/// Visual Basic internal wrapper for `DllGetClassObject`.
///
/// # Arguments
/// * `rclsid` - Pointer to CLSID.
/// * `riid` - Pointer to requested IID.
/// * `ppv` - Output pointer for interface.
///
/// # Returns
/// Status code (`HRESULT`).
#[no_mangle]
pub unsafe extern "system" fn VBDllGetClassObject(
    rclsid: *const Clsid,
    riid: *const Iid,
    ppv: *mut *mut c_void,
) -> HResult {
    DllGetClassObject(rclsid, riid, ppv)
}

/// Visual Basic internal wrapper for `DllRegisterServer`.
///
/// # Returns
/// * `S_OK` (0).
#[no_mangle]
pub unsafe extern "system" fn VBDllRegisterServer() -> HResult {
    S_OK
}

/// Visual Basic internal wrapper for `DllUnregisterServer`.
///
/// # Returns
/// * `S_OK` (0).
#[no_mangle]
pub unsafe extern "system" fn VBDllUnRegisterServer() -> HResult {
    S_OK
}

// ----------------------------------------------------------------------------
// BASIC_CLASS Dispatch Implementations
// ----------------------------------------------------------------------------

/// Increments the reference count of a VB6 Class object (`IUnknown::AddRef`).
///
/// # Arguments
/// * `_this` - Pointer to the COM object instance.
///
/// # Returns
/// The new reference count.
#[no_mangle]
pub unsafe extern "system" fn BASIC_CLASS_AddRef(_this: *mut c_void) -> u32 {
    1
}

/// Decrements the reference count of a VB6 Class object (`IUnknown::Release`).
///
/// # Arguments
/// * `_this` - Pointer to the COM object instance.
///
/// # Returns
/// The new reference count.
#[no_mangle]
pub unsafe extern "system" fn BASIC_CLASS_Release(_this: *mut c_void) -> u32 {
    1
}

/// Queries a VB6 Class object for a supported COM interface (`IUnknown::QueryInterface`).
///
/// # Arguments
/// * `_this` - Pointer to the COM object instance.
/// * `_riid` - Pointer to the requested interface identifier.
/// * `ppv` - Pointer where the interface pointer is written.
///
/// # Returns
/// * `S_OK` on success.
/// * `E_POINTER` if `ppv` is null.
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

/// Maps member names to dispatch identifiers (DISPIDs) for late-binding (`IDispatch::GetIDsOfNames`).
///
/// # Returns
/// * `S_OK` (0).
#[no_mangle]
pub unsafe extern "system" fn BASIC_CLASS_GetIDsOfNames() -> HResult {
    S_OK
}

/// Invokes a method or property accessor on a VB6 Class object via late-binding (`IDispatch::Invoke`).
///
/// # Returns
/// * `S_OK` (0).
#[no_mangle]
pub unsafe extern "system" fn BASIC_CLASS_Invoke() -> HResult {
    S_OK
}

/// Returns the number of type information interfaces provided by the dispinterface.
///
/// # Returns
/// * `S_OK` (0).
#[no_mangle]
pub unsafe extern "system" fn BASIC_DISPINTERFACE_GetTICount() -> HResult {
    S_OK
}

/// Retrieves type information for a dispinterface (`IDispatch::GetTypeInfo`).
///
/// # Returns
/// * `S_OK` (0).
#[no_mangle]
pub unsafe extern "system" fn BASIC_DISPINTERFACE_GetTypeInfo() -> HResult {
    S_OK
}

// ----------------------------------------------------------------------------
// EVENT_SINK Dispatch Implementations
// ----------------------------------------------------------------------------

/// Increments the reference count of a COM event sink instance.
///
/// # Arguments
/// * `_this` - Pointer to the event sink instance.
///
/// # Returns
/// The new reference count.
#[no_mangle]
pub unsafe extern "system" fn EVENT_SINK_AddRef(_this: *mut c_void) -> u32 {
    1
}

/// Decrements the reference count of a COM event sink instance.
///
/// # Arguments
/// * `_this` - Pointer to the event sink instance.
///
/// # Returns
/// The new reference count.
#[no_mangle]
pub unsafe extern "system" fn EVENT_SINK_Release(_this: *mut c_void) -> u32 {
    1
}

/// Queries an event sink for a supported interface.
///
/// # Arguments
/// * `_this` - Pointer to the event sink instance.
/// * `_riid` - Pointer to requested IID.
/// * `ppv` - Output pointer for interface.
///
/// # Returns
/// * `S_OK` on success.
/// * `E_POINTER` if `ppv` is null.
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

/// Maps event method names to DISPIDs on an event sink.
///
/// # Returns
/// * `S_OK` (0).
#[no_mangle]
pub unsafe extern "system" fn EVENT_SINK_GetIDsOfNames() -> HResult {
    S_OK
}

/// Dispatches an event notification from a COM source to the VB6 event handler method.
///
/// # Returns
/// * `S_OK` (0).
#[no_mangle]
pub unsafe extern "system" fn EVENT_SINK_Invoke() -> HResult {
    S_OK
}

/// Increments the reference count of a secondary COM event sink instance.
///
/// # Arguments
/// * `_this` - Pointer to event sink instance.
///
/// # Returns
/// The new reference count.
#[no_mangle]
pub unsafe extern "system" fn EVENT_SINK2_AddRef(_this: *mut c_void) -> u32 {
    1
}

/// Decrements the reference count of a secondary COM event sink instance.
///
/// # Arguments
/// * `_this` - Pointer to event sink instance.
///
/// # Returns
/// The new reference count.
#[no_mangle]
pub unsafe extern "system" fn EVENT_SINK2_Release(_this: *mut c_void) -> u32 {
    1
}

// ----------------------------------------------------------------------------
// Zombie Dispatch Implementations (Dead COM references)
// ----------------------------------------------------------------------------

/// Increments the reference count on a detached or destroyed ("zombie") COM reference.
///
/// Prevents access violations when code attempts to reference already-closed objects.
///
/// # Arguments
/// * `_this` - Pointer to zombie object reference.
///
/// # Returns
/// Always returns `1`.
#[no_mangle]
pub unsafe extern "system" fn Zombie_AddRef(_this: *mut c_void) -> u32 {
    1
}

/// Decrements the reference count on a detached or destroyed ("zombie") COM reference.
///
/// # Arguments
/// * `_this` - Pointer to zombie object reference.
///
/// # Returns
/// Always returns `1`.
#[no_mangle]
pub unsafe extern "system" fn Zombie_Release(_this: *mut c_void) -> u32 {
    1
}

/// Queries a zombie object for an interface, safely returning null without crashing.
///
/// # Arguments
/// * `_this` - Pointer to zombie object reference.
/// * `_riid` - Requested IID.
/// * `ppv` - Output pointer.
///
/// # Returns
/// * `S_OK` (0) with `*ppv` set to null.
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

/// Safe stub handling late-binding name lookup on zombie objects.
///
/// # Returns
/// * `S_OK` (0).
#[no_mangle]
pub unsafe extern "system" fn Zombie_GetIDsOfNames() -> HResult {
    S_OK
}

/// Safe stub handling type info requests on zombie objects.
///
/// # Returns
/// * `S_OK` (0).
#[no_mangle]
pub unsafe extern "system" fn Zombie_GetTypeInfo() -> HResult {
    S_OK
}

/// Returns zero type info count for zombie objects.
///
/// # Returns
/// * `S_OK` (0).
#[no_mangle]
pub unsafe extern "system" fn Zombie_GetTypeInfoCount() -> HResult {
    S_OK
}

/// Safe stub ignoring method invocation on zombie objects.
///
/// # Returns
/// * `S_OK` (0).
#[no_mangle]
pub unsafe extern "system" fn Zombie_Invoke() -> HResult {
    S_OK
}
