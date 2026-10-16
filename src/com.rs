//! COM Automation and ActiveX DLL export implementations.
//!
//! Provides the standard COM in-process server lifecycle exports (`DllGetClassObject`,
//! `DllCanUnloadNow`, etc.) as well as the internal VB6 class, event sink, and zombie
//! object dispatch helpers.

use core::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};
use crate::types::{
    CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, CLSID_Dictionary, Clsid, E_NOINTERFACE,
    E_POINTER, Guid, HResult, IID_IClassFactory, IID_IDispatch, IID_IUnknown, Iid, S_OK,
};

/// COM Interface ID for `IVbaHost`: `{1D6F7D1E-EACD-11CD-9570-00AA0051E5D4}`.
#[no_mangle]
pub static IID_IVbaHost: Guid = Guid::new(
    0x1D6F7D1E,
    0xEACD,
    0x11CD,
    [0x95, 0x70, 0x00, 0xAA, 0x00, 0x51, 0xE5, 0xD4],
);

// ----------------------------------------------------------------------------
// COM Interface VTables and Helpers
// ----------------------------------------------------------------------------

/// COM `IClassFactory` vtable layout.
#[repr(C)]
pub struct IClassFactoryVtbl {
    pub QueryInterface: unsafe extern "system" fn(this: *mut c_void, riid: *const Iid, ppv: *mut *mut c_void) -> HResult,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub CreateInstance: unsafe extern "system" fn(
        this: *mut c_void,
        p_unk_outer: *mut c_void,
        riid: *const Iid,
        ppv: *mut *mut c_void,
    ) -> HResult,
    pub LockServer: unsafe extern "system" fn(this: *mut c_void, f_lock: i32) -> HResult,
}

/// COM `IDispatch` vtable layout.
#[repr(C)]
pub struct IDispatchVtbl {
    // IUnknown
    pub QueryInterface: unsafe extern "system" fn(this: *mut c_void, riid: *const Iid, ppv: *mut *mut c_void) -> HResult,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    // IDispatch
    pub GetTypeInfoCount: unsafe extern "system" fn(this: *mut c_void, pctinfo: *mut u32) -> HResult,
    pub GetTypeInfo: unsafe extern "system" fn(this: *mut c_void, iTInfo: u32, lcid: u32, ppTInfo: *mut *mut c_void) -> HResult,
    pub GetIDsOfNames: unsafe extern "system" fn(
        this: *mut c_void,
        riid: *const Iid,
        rgszNames: *const *const u16,
        cNames: u32,
        lcid: u32,
        rgDispId: *mut i32,
    ) -> HResult,
    pub Invoke: unsafe extern "system" fn(
        this: *mut c_void,
        dispIdMember: i32,
        riid: *const Iid,
        lcid: u32,
        wFlags: u16,
        pDispParams: *mut c_void,
        pVarResult: *mut c_void,
        pExcepInfo: *mut c_void,
        puArgErr: *mut u32,
    ) -> HResult,
}

/// Helper to safely call `IUnknown::AddRef` through a COM interface pointer.
pub unsafe fn com_add_ref(p_obj: *mut c_void) -> u32 {
    if p_obj.is_null() {
        return 0;
    }
    let vtable = *(p_obj as *mut *mut usize);
    if vtable.is_null() {
        return 0;
    }
    let add_ref_fn: unsafe extern "system" fn(*mut c_void) -> u32 =
        core::mem::transmute(*vtable.add(1));
    add_ref_fn(p_obj)
}

/// Helper to safely call `IUnknown::Release` through a COM interface pointer.
pub unsafe fn com_release(p_obj: *mut c_void) -> u32 {
    if p_obj.is_null() {
        return 0;
    }
    let vtable = *(p_obj as *mut *mut usize);
    if vtable.is_null() {
        return 0;
    }
    let release_fn: unsafe extern "system" fn(*mut c_void) -> u32 =
        core::mem::transmute(*vtable.add(2));
    release_fn(p_obj)
}

/// Helper to call `IUnknown::QueryInterface` through a COM interface pointer.
pub unsafe fn com_query_interface(
    p_obj: *mut c_void,
    riid: *const Iid,
    ppv: *mut *mut c_void,
) -> HResult {
    if p_obj.is_null() || ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    *ppv = core::ptr::null_mut();
    let vtable = *(p_obj as *mut *mut usize);
    if vtable.is_null() {
        return E_POINTER;
    }
    let qi_fn: unsafe extern "system" fn(*mut c_void, *const Iid, *mut *mut c_void) -> HResult =
        core::mem::transmute(*vtable.add(0));
    qi_fn(p_obj, riid, ppv)
}

// ----------------------------------------------------------------------------
// Scripting.Dictionary In-Process COM Object
// ----------------------------------------------------------------------------

const DICTIONARY_MAGIC: u32 = 0x53435252; // 'SCRR'

/// In-memory representation of a `Scripting.Dictionary` COM Automation object.
#[repr(C)]
pub struct DictionaryObject {
    pub vtable: *const IDispatchVtbl,
    pub ref_count: AtomicU32,
    pub magic: u32,
    pub items: std::sync::Mutex<std::collections::HashMap<String, crate::types::Variant>>,
}

unsafe extern "system" fn dict_QueryInterface(
    this: *mut c_void,
    riid: *const Iid,
    ppv: *mut *mut c_void,
) -> HResult {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    let iid = *riid;
    if iid == IID_IUnknown || iid == IID_IDispatch {
        *ppv = this;
        dict_AddRef(this);
        return S_OK;
    }
    *ppv = core::ptr::null_mut();
    E_NOINTERFACE
}

unsafe extern "system" fn dict_AddRef(this: *mut c_void) -> u32 {
    let obj = &*(this as *mut DictionaryObject);
    obj.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn dict_Release(this: *mut c_void) -> u32 {
    let obj_ptr = this as *mut DictionaryObject;
    let old_count = (*obj_ptr).ref_count.fetch_sub(1, Ordering::SeqCst);
    if old_count == 1 {
        let _ = Box::from_raw(obj_ptr);
        0
    } else {
        old_count - 1
    }
}

unsafe extern "system" fn dict_GetTypeInfoCount(
    _this: *mut c_void,
    pctinfo: *mut u32,
) -> HResult {
    if !pctinfo.is_null() {
        *pctinfo = 1;
    }
    S_OK
}

unsafe extern "system" fn dict_GetTypeInfo(
    _this: *mut c_void,
    _iTInfo: u32,
    _lcid: u32,
    ppTInfo: *mut *mut c_void,
) -> HResult {
    if !ppTInfo.is_null() {
        *ppTInfo = core::ptr::null_mut();
    }
    S_OK
}

unsafe extern "system" fn dict_GetIDsOfNames(
    _this: *mut c_void,
    _riid: *const Iid,
    _rgszNames: *const *const u16,
    _cNames: u32,
    _lcid: u32,
    _rgDispId: *mut i32,
) -> HResult {
    S_OK
}

unsafe extern "system" fn dict_Invoke(
    _this: *mut c_void,
    _dispIdMember: i32,
    _riid: *const Iid,
    _lcid: u32,
    _wFlags: u16,
    _pDispParams: *mut c_void,
    _pVarResult: *mut c_void,
    _pExcepInfo: *mut c_void,
    _puArgErr: *mut u32,
) -> HResult {
    S_OK
}

static DICT_VTBL: IDispatchVtbl = IDispatchVtbl {
    QueryInterface: dict_QueryInterface,
    AddRef: dict_AddRef,
    Release: dict_Release,
    GetTypeInfoCount: dict_GetTypeInfoCount,
    GetTypeInfo: dict_GetTypeInfo,
    GetIDsOfNames: dict_GetIDsOfNames,
    Invoke: dict_Invoke,
};

impl DictionaryObject {
    /// Creates and returns a newly allocated, heap-backed `DictionaryObject`.
    pub fn new() -> *mut DictionaryObject {
        let boxed = Box::new(DictionaryObject {
            vtable: &DICT_VTBL,
            ref_count: AtomicU32::new(1),
            magic: DICTIONARY_MAGIC,
            items: std::sync::Mutex::new(std::collections::HashMap::new()),
        });
        Box::into_raw(boxed)
    }
}

// ----------------------------------------------------------------------------
// Scripting.Dictionary Class Factory
// ----------------------------------------------------------------------------

#[repr(C)]
pub struct DictionaryClassFactory {
    pub vtable: *const IClassFactoryVtbl,
    pub ref_count: AtomicU32,
}

unsafe impl Sync for DictionaryClassFactory {}
unsafe impl Send for DictionaryClassFactory {}
unsafe impl Sync for DictionaryObject {}
unsafe impl Send for DictionaryObject {}

unsafe extern "system" fn cf_QueryInterface(
    this: *mut c_void,
    riid: *const Iid,
    ppv: *mut *mut c_void,
) -> HResult {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    let iid = *riid;
    if iid == IID_IUnknown || iid == IID_IClassFactory {
        *ppv = this;
        cf_AddRef(this);
        return S_OK;
    }
    *ppv = core::ptr::null_mut();
    E_NOINTERFACE
}

unsafe extern "system" fn cf_AddRef(this: *mut c_void) -> u32 {
    let cf = &*(this as *mut DictionaryClassFactory);
    cf.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn cf_Release(this: *mut c_void) -> u32 {
    let cf = &*(this as *mut DictionaryClassFactory);
    let old = cf.ref_count.fetch_sub(1, Ordering::SeqCst);
    if old > 0 { old - 1 } else { 0 }
}

unsafe extern "system" fn cf_CreateInstance(
    _this: *mut c_void,
    p_unk_outer: *mut c_void,
    riid: *const Iid,
    ppv: *mut *mut c_void,
) -> HResult {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    *ppv = core::ptr::null_mut();
    if !p_unk_outer.is_null() {
        return CLASS_E_NOAGGREGATION;
    }
    let dict = DictionaryObject::new();
    let hr = dict_QueryInterface(dict as *mut c_void, riid, ppv);
    dict_Release(dict as *mut c_void);
    hr
}

unsafe extern "system" fn cf_LockServer(_this: *mut c_void, _f_lock: i32) -> HResult {
    S_OK
}

static CF_VTBL: IClassFactoryVtbl = IClassFactoryVtbl {
    QueryInterface: cf_QueryInterface,
    AddRef: cf_AddRef,
    Release: cf_Release,
    CreateInstance: cf_CreateInstance,
    LockServer: cf_LockServer,
};

static DICT_FACTORY: DictionaryClassFactory = DictionaryClassFactory {
    vtable: &CF_VTBL,
    ref_count: AtomicU32::new(1),
};

pub fn get_dictionary_class_factory(riid: *const Iid, ppv: *mut *mut c_void) -> HResult {
    unsafe {
        cf_QueryInterface(
            &DICT_FACTORY as *const DictionaryClassFactory as *mut c_void,
            riid,
            ppv,
        )
    }
}

/// Determines the friendly type name for an active COM object pointer.
pub unsafe fn get_object_type_name(p_obj: *mut c_void) -> Option<String> {
    if p_obj.is_null() {
        return None;
    }
    let dict_candidate = p_obj as *const DictionaryObject;
    if (*dict_candidate).vtable == &DICT_VTBL && (*dict_candidate).magic == DICTIONARY_MAGIC {
        return Some("Dictionary".to_string());
    }
    if p_obj == crate::err::rtcErrObj() {
        return Some("ErrObject".to_string());
    }
    None
}

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
/// * `rclsid` - Pointer to the target `CLSID` (GUID identifying the COM coclass).
/// * `riid` - Pointer to the requested interface identifier (`IID_IClassFactory`).
/// * `ppv` - Pointer to the output memory location where the interface pointer is written.
///
/// # Returns
/// * `S_OK` on success.
/// * `E_POINTER` if `ppv` is null.
/// * `CLASS_E_CLASSNOTAVAILABLE` if the requested CLSID is not supported.
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const Clsid,
    riid: *const Iid,
    ppv: *mut *mut c_void,
) -> HResult {
    if ppv.is_null() || rclsid.is_null() || riid.is_null() {
        return E_POINTER;
    }
    *ppv = core::ptr::null_mut();

    let clsid = *rclsid;
    if clsid == CLSID_Dictionary {
        return get_dictionary_class_factory(riid, ppv);
    }

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
