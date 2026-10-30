//! COM Automation and ActiveX DLL export implementations.
//!
//! Provides the standard COM in-process server lifecycle exports (`DllGetClassObject`,
//! `DllCanUnloadNow`, etc.) as well as the internal VB6 class, event sink, and zombie
//! object dispatch helpers.

use crate::types::{
    sys_alloc_string, Bstr, CLSID_Dictionary, Clsid, Guid, HResult, IID_IClassFactory,
    IID_IDispatch, IID_IEnumVARIANT, IID_ITypeInfo, IID_IUnknown, IID_NULL, Iid, SysFreeString, Variant,
    CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, DISP_E_BADINDEX, E_INVALIDARG, E_NOINTERFACE, E_NOTIMPL,
    E_POINTER, S_FALSE, S_OK, TYPE_E_ELEMENTNOTFOUND,
};
use core::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

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
    pub QueryInterface: unsafe extern "system" fn(
        this: *mut c_void,
        riid: *const Iid,
        ppv: *mut *mut c_void,
    ) -> HResult,
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
    pub QueryInterface: unsafe extern "system" fn(
        this: *mut c_void,
        riid: *const Iid,
        ppv: *mut *mut c_void,
    ) -> HResult,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    // IDispatch
    pub GetTypeInfoCount:
        unsafe extern "system" fn(this: *mut c_void, pctinfo: *mut u32) -> HResult,
    pub GetTypeInfo: unsafe extern "system" fn(
        this: *mut c_void,
        iTInfo: u32,
        lcid: u32,
        ppTInfo: *mut *mut c_void,
    ) -> HResult,
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

/// COM `IEnumVARIANT` vtable layout.
#[repr(C)]
pub struct IEnumVARIANTVtbl {
    pub QueryInterface:
        unsafe extern "system" fn(this: *mut c_void, riid: *const Iid, ppv: *mut *mut c_void) -> HResult,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Next: unsafe extern "system" fn(
        this: *mut c_void,
        celt: u32,
        rgVar: *mut Variant,
        pceltFetched: *mut u32,
    ) -> HResult,
    pub Skip: unsafe extern "system" fn(this: *mut c_void, celt: u32) -> HResult,
    pub Reset: unsafe extern "system" fn(this: *mut c_void) -> HResult,
    pub Clone: unsafe extern "system" fn(this: *mut c_void, ppEnum: *mut *mut c_void) -> HResult,
}

/// COM `ITypeInfo` vtable layout.
#[repr(C)]
pub struct ITypeInfoVtbl {
    pub QueryInterface:
        unsafe extern "system" fn(this: *mut c_void, riid: *const Iid, ppv: *mut *mut c_void) -> HResult,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub GetTypeAttr: unsafe extern "system" fn(this: *mut c_void, ppTypeAttr: *mut *mut c_void) -> HResult,
    pub GetTypeComp: unsafe extern "system" fn(this: *mut c_void, ppTComp: *mut *mut c_void) -> HResult,
    pub GetFuncDesc:
        unsafe extern "system" fn(this: *mut c_void, index: u32, ppFuncDesc: *mut *mut c_void) -> HResult,
    pub GetVarDesc:
        unsafe extern "system" fn(this: *mut c_void, index: u32, ppVarDesc: *mut *mut c_void) -> HResult,
    pub GetNames: unsafe extern "system" fn(
        this: *mut c_void,
        memid: i32,
        rgBstrNames: *mut Bstr,
        cMaxNames: u32,
        pcNames: *mut u32,
    ) -> HResult,
    pub GetRefTypeOfImplType:
        unsafe extern "system" fn(this: *mut c_void, index: u32, pRefType: *mut u32) -> HResult,
    pub GetImplTypeFlags:
        unsafe extern "system" fn(this: *mut c_void, index: u32, pImplTypeFlags: *mut i32) -> HResult,
    pub GetIDsOfNames: unsafe extern "system" fn(
        this: *mut c_void,
        rgszNames: *const *const u16,
        cNames: u32,
        pMemId: *mut i32,
    ) -> HResult,
    pub Invoke: unsafe extern "system" fn(
        this: *mut c_void,
        pvInstance: *mut c_void,
        memid: i32,
        wFlags: u16,
        pDispParams: *mut c_void,
        pVarResult: *mut Variant,
        pExcepInfo: *mut c_void,
        puArgErr: *mut u32,
    ) -> HResult,
    pub GetDocumentation: unsafe extern "system" fn(
        this: *mut c_void,
        memid: i32,
        pBstrName: *mut Bstr,
        pBstrDocString: *mut Bstr,
        pdwHelpContext: *mut u32,
        pBstrHelpFile: *mut Bstr,
    ) -> HResult,
    pub GetDllEntry: unsafe extern "system" fn(
        this: *mut c_void,
        memid: i32,
        invKind: i32,
        pBstrDllName: *mut Bstr,
        pBstrName: *mut Bstr,
        pwOrdinal: *mut u16,
    ) -> HResult,
    pub GetRefTypeInfo:
        unsafe extern "system" fn(this: *mut c_void, hRefType: u32, ppTInfo: *mut *mut c_void) -> HResult,
    pub AddressOfMember: unsafe extern "system" fn(
        this: *mut c_void,
        memid: i32,
        invKind: i32,
        ppv: *mut *mut c_void,
    ) -> HResult,
    pub CreateInstance: unsafe extern "system" fn(
        this: *mut c_void,
        pUnkOuter: *mut c_void,
        riid: *const Iid,
        ppvObj: *mut *mut c_void,
    ) -> HResult,
    pub GetMops:
        unsafe extern "system" fn(this: *mut c_void, memid: i32, pBstrMops: *mut Bstr) -> HResult,
    pub GetContainingTypeLib:
        unsafe extern "system" fn(this: *mut c_void, ppTLib: *mut *mut c_void, pIndex: *mut u32) -> HResult,
    pub ReleaseTypeAttr: unsafe extern "system" fn(this: *mut c_void, pTypeAttr: *mut c_void),
    pub ReleaseFuncDesc: unsafe extern "system" fn(this: *mut c_void, pFuncDesc: *mut c_void),
    pub ReleaseVarDesc: unsafe extern "system" fn(this: *mut c_void, pVarDesc: *mut c_void),
}

/// Concrete in-memory Self-Describing `ITypeInfo` instance.
#[repr(C)]
pub struct NativeTypeInfo {
    pub vtable: *const ITypeInfoVtbl,
    pub ref_count: AtomicU32,
    pub name: String,
    pub guid: Guid,
    pub members: Vec<String>,
}

impl NativeTypeInfo {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(name: &str, guid: Guid, members: &[&str]) -> *mut c_void {
        let obj = Box::new(Self {
            vtable: &NATIVE_TYPE_INFO_VTBL,
            ref_count: AtomicU32::new(1),
            name: name.to_string(),
            guid,
            members: members.iter().map(|s| s.to_string()).collect(),
        });
        Box::into_raw(obj) as *mut c_void
    }
}

unsafe extern "system" fn ti_QueryInterface(
    this: *mut c_void,
    riid: *const Iid,
    ppv: *mut *mut c_void,
) -> HResult {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    let iid = *riid;
    if iid == IID_IUnknown || iid == IID_ITypeInfo {
        *ppv = this;
        ti_AddRef(this);
        return S_OK;
    }
    *ppv = core::ptr::null_mut();
    E_NOINTERFACE
}

unsafe extern "system" fn ti_AddRef(this: *mut c_void) -> u32 {
    let obj = &*(this as *mut NativeTypeInfo);
    obj.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn ti_Release(this: *mut c_void) -> u32 {
    let obj_ptr = this as *mut NativeTypeInfo;
    let old_count = (*obj_ptr).ref_count.fetch_sub(1, Ordering::SeqCst);
    if old_count == 1 {
        let _ = Box::from_raw(obj_ptr);
        0
    } else {
        old_count - 1
    }
}

unsafe extern "system" fn ti_GetTypeAttr(_this: *mut c_void, ppTypeAttr: *mut *mut c_void) -> HResult {
    if ppTypeAttr.is_null() {
        return E_POINTER;
    }
    *ppTypeAttr = core::ptr::null_mut();
    S_OK
}

unsafe extern "system" fn ti_GetTypeComp(_this: *mut c_void, ppTComp: *mut *mut c_void) -> HResult {
    if !ppTComp.is_null() {
        *ppTComp = core::ptr::null_mut();
    }
    E_NOTIMPL
}

unsafe extern "system" fn ti_GetFuncDesc(_this: *mut c_void, _index: u32, ppFuncDesc: *mut *mut c_void) -> HResult {
    if !ppFuncDesc.is_null() {
        *ppFuncDesc = core::ptr::null_mut();
    }
    TYPE_E_ELEMENTNOTFOUND
}

unsafe extern "system" fn ti_GetVarDesc(_this: *mut c_void, _index: u32, ppVarDesc: *mut *mut c_void) -> HResult {
    if !ppVarDesc.is_null() {
        *ppVarDesc = core::ptr::null_mut();
    }
    TYPE_E_ELEMENTNOTFOUND
}

unsafe extern "system" fn ti_GetNames(
    this: *mut c_void,
    memid: i32,
    rgBstrNames: *mut Bstr,
    cMaxNames: u32,
    pcNames: *mut u32,
) -> HResult {
    if rgBstrNames.is_null() || pcNames.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut NativeTypeInfo);
    if memid == -1 {
        *rgBstrNames = sys_alloc_string(&obj.name);
        *pcNames = 1;
        return S_OK;
    }
    if memid >= 0 && (memid as usize) < obj.members.len() && cMaxNames > 0 {
        *rgBstrNames = sys_alloc_string(&obj.members[memid as usize]);
        *pcNames = 1;
        return S_OK;
    }
    *pcNames = 0;
    TYPE_E_ELEMENTNOTFOUND
}

unsafe extern "system" fn ti_GetRefTypeOfImplType(_this: *mut c_void, _index: u32, _pRefType: *mut u32) -> HResult {
    E_NOTIMPL
}

unsafe extern "system" fn ti_GetImplTypeFlags(_this: *mut c_void, _index: u32, pImplTypeFlags: *mut i32) -> HResult {
    if !pImplTypeFlags.is_null() {
        *pImplTypeFlags = 0;
    }
    S_OK
}

unsafe extern "system" fn ti_GetIDsOfNames(
    this: *mut c_void,
    rgszNames: *const *const u16,
    cNames: u32,
    pMemId: *mut i32,
) -> HResult {
    if this.is_null() || rgszNames.is_null() || pMemId.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut NativeTypeInfo);
    let mut hr = S_OK;
    for i in 0..cNames {
        let name_ptr = *rgszNames.add(i as usize);
        if name_ptr.is_null() {
            *pMemId.add(i as usize) = -1;
            hr = DISP_E_UNKNOWNNAME;
            continue;
        }
        let mut len = 0;
        while *name_ptr.add(len) != 0 {
            len += 1;
        }
        let u16_slice = core::slice::from_raw_parts(name_ptr, len);
        let name_str = String::from_utf16_lossy(u16_slice);
        let mut found = false;
        for (idx, m) in obj.members.iter().enumerate() {
            if m.eq_ignore_ascii_case(&name_str) {
                *pMemId.add(i as usize) = idx as i32;
                found = true;
                break;
            }
        }
        if !found {
            *pMemId.add(i as usize) = -1;
            hr = DISP_E_UNKNOWNNAME;
        }
    }
    hr
}

unsafe extern "system" fn ti_Invoke(
    _this: *mut c_void,
    _pvInstance: *mut c_void,
    _memid: i32,
    _wFlags: u16,
    _pDispParams: *mut c_void,
    _pVarResult: *mut Variant,
    _pExcepInfo: *mut c_void,
    _puArgErr: *mut u32,
) -> HResult {
    E_NOTIMPL
}

unsafe extern "system" fn ti_GetDocumentation(
    this: *mut c_void,
    memid: i32,
    pBstrName: *mut Bstr,
    pBstrDocString: *mut Bstr,
    pdwHelpContext: *mut u32,
    pBstrHelpFile: *mut Bstr,
) -> HResult {
    if this.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut NativeTypeInfo);
    if memid == -1 {
        if !pBstrName.is_null() {
            *pBstrName = sys_alloc_string(&obj.name);
        }
        if !pBstrDocString.is_null() {
            *pBstrDocString = sys_alloc_string("");
        }
        if !pdwHelpContext.is_null() {
            *pdwHelpContext = 0;
        }
        if !pBstrHelpFile.is_null() {
            *pBstrHelpFile = sys_alloc_string("");
        }
        return S_OK;
    }
    if memid >= 0 && (memid as usize) < obj.members.len() {
        if !pBstrName.is_null() {
            *pBstrName = sys_alloc_string(&obj.members[memid as usize]);
        }
        if !pBstrDocString.is_null() {
            *pBstrDocString = sys_alloc_string("");
        }
        if !pdwHelpContext.is_null() {
            *pdwHelpContext = 0;
        }
        if !pBstrHelpFile.is_null() {
            *pBstrHelpFile = sys_alloc_string("");
        }
        return S_OK;
    }
    TYPE_E_ELEMENTNOTFOUND
}

unsafe extern "system" fn ti_GetDllEntry(
    _this: *mut c_void,
    _memid: i32,
    _invKind: i32,
    _pBstrDllName: *mut Bstr,
    _pBstrName: *mut Bstr,
    _pwOrdinal: *mut u16,
) -> HResult {
    E_NOTIMPL
}

unsafe extern "system" fn ti_GetRefTypeInfo(_this: *mut c_void, _hRefType: u32, ppTInfo: *mut *mut c_void) -> HResult {
    if !ppTInfo.is_null() {
        *ppTInfo = core::ptr::null_mut();
    }
    E_NOTIMPL
}

unsafe extern "system" fn ti_AddressOfMember(
    _this: *mut c_void,
    _memid: i32,
    _invKind: i32,
    ppv: *mut *mut c_void,
) -> HResult {
    if !ppv.is_null() {
        *ppv = core::ptr::null_mut();
    }
    E_NOTIMPL
}

unsafe extern "system" fn ti_CreateInstance(
    _this: *mut c_void,
    _pUnkOuter: *mut c_void,
    _riid: *const Iid,
    ppvObj: *mut *mut c_void,
) -> HResult {
    if !ppvObj.is_null() {
        *ppvObj = core::ptr::null_mut();
    }
    E_NOTIMPL
}

unsafe extern "system" fn ti_GetMops(_this: *mut c_void, _memid: i32, pBstrMops: *mut Bstr) -> HResult {
    if !pBstrMops.is_null() {
        *pBstrMops = sys_alloc_string("");
    }
    S_OK
}

unsafe extern "system" fn ti_GetContainingTypeLib(_this: *mut c_void, ppTLib: *mut *mut c_void, pIndex: *mut u32) -> HResult {
    if !ppTLib.is_null() {
        *ppTLib = core::ptr::null_mut();
    }
    if !pIndex.is_null() {
        *pIndex = 0;
    }
    E_NOTIMPL
}

unsafe extern "system" fn ti_ReleaseTypeAttr(_this: *mut c_void, _pTypeAttr: *mut c_void) {}
unsafe extern "system" fn ti_ReleaseFuncDesc(_this: *mut c_void, _pFuncDesc: *mut c_void) {}
unsafe extern "system" fn ti_ReleaseVarDesc(_this: *mut c_void, _pVarDesc: *mut c_void) {}

pub static NATIVE_TYPE_INFO_VTBL: ITypeInfoVtbl = ITypeInfoVtbl {
    QueryInterface: ti_QueryInterface,
    AddRef: ti_AddRef,
    Release: ti_Release,
    GetTypeAttr: ti_GetTypeAttr,
    GetTypeComp: ti_GetTypeComp,
    GetFuncDesc: ti_GetFuncDesc,
    GetVarDesc: ti_GetVarDesc,
    GetNames: ti_GetNames,
    GetRefTypeOfImplType: ti_GetRefTypeOfImplType,
    GetImplTypeFlags: ti_GetImplTypeFlags,
    GetIDsOfNames: ti_GetIDsOfNames,
    Invoke: ti_Invoke,
    GetDocumentation: ti_GetDocumentation,
    GetDllEntry: ti_GetDllEntry,
    GetRefTypeInfo: ti_GetRefTypeInfo,
    AddressOfMember: ti_AddressOfMember,
    CreateInstance: ti_CreateInstance,
    GetMops: ti_GetMops,
    GetContainingTypeLib: ti_GetContainingTypeLib,
    ReleaseTypeAttr: ti_ReleaseTypeAttr,
    ReleaseFuncDesc: ti_ReleaseFuncDesc,
    ReleaseVarDesc: ti_ReleaseVarDesc,
};

// ----------------------------------------------------------------------------
// IDispatch Invocation Constants and Structures
// ----------------------------------------------------------------------------

pub const DISPATCH_METHOD: u16 = 0x1;
pub const DISPATCH_PROPERTYGET: u16 = 0x2;
pub const DISPATCH_PROPERTYPUT: u16 = 0x4;
pub const DISPATCH_PROPERTYPUTREF: u16 = 0x8;

pub const DISPID_VALUE: i32 = 0;
pub const DISPID_UNKNOWN: i32 = -1;
pub const DISPID_PROPERTYPUT: i32 = -3;

pub const DISP_E_UNKNOWNNAME: HResult = -2147352570; // 0x80020006
pub const DISP_E_MEMBERNOTFOUND: HResult = -2147352573; // 0x80020003
pub const DISP_E_PARAMNOTFOUND: HResult = -2147352572; // 0x80020004
pub const DISP_E_TYPEMISMATCH: HResult = -2147352571; // 0x80020005
pub const DISP_E_BADPARAMCOUNT: HResult = -2147352562; // 0x8002000E
pub const DISP_E_EXCEPTION: HResult = -2147352567; // 0x80020009

/// Parameter block passed to `IDispatch::Invoke`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DispParams {
    /// Array of arguments, stored in reverse order per COM standard.
    pub rgvarg: *mut Variant,
    /// Array of DISPIDs for named arguments.
    pub rgdispidNamedArgs: *mut i32,
    /// Count of positional arguments.
    pub cArgs: u32,
    /// Count of named arguments.
    pub cNamedArgs: u32,
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

/// COM `IDictionary` dual-interface vtable layout (Scripting.Dictionary).
#[repr(C)]
pub struct IDictionaryVtbl {
    // IUnknown
    pub QueryInterface: unsafe extern "system" fn(
        this: *mut c_void,
        riid: *const Iid,
        ppv: *mut *mut c_void,
    ) -> HResult,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    // IDispatch
    pub GetTypeInfoCount:
        unsafe extern "system" fn(this: *mut c_void, pctinfo: *mut u32) -> HResult,
    pub GetTypeInfo: unsafe extern "system" fn(
        this: *mut c_void,
        iTInfo: u32,
        lcid: u32,
        ppTInfo: *mut *mut c_void,
    ) -> HResult,
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
    // IDictionary dual interface methods (slots 7..21)
    pub putref_Item: unsafe extern "system" fn(
        this: *mut c_void,
        key: *const crate::types::Variant,
        item: *const crate::types::Variant,
    ) -> HResult,
    pub put_Item: unsafe extern "system" fn(
        this: *mut c_void,
        key: *const crate::types::Variant,
        item: *const crate::types::Variant,
    ) -> HResult,
    pub get_Item: unsafe extern "system" fn(
        this: *mut c_void,
        key: *const crate::types::Variant,
        ret_item: *mut crate::types::Variant,
    ) -> HResult,
    pub Add: unsafe extern "system" fn(
        this: *mut c_void,
        key: *const crate::types::Variant,
        item: *const crate::types::Variant,
    ) -> HResult,
    pub get_Count: unsafe extern "system" fn(this: *mut c_void, ret_count: *mut i32) -> HResult,
    pub Exists: unsafe extern "system" fn(
        this: *mut c_void,
        key: *const crate::types::Variant,
        ret_exists: *mut i16,
    ) -> HResult,
    pub Items: unsafe extern "system" fn(
        this: *mut c_void,
        ret_items: *mut crate::types::Variant,
    ) -> HResult,
    pub put_Key: unsafe extern "system" fn(
        this: *mut c_void,
        key: *const crate::types::Variant,
        new_key: *const crate::types::Variant,
    ) -> HResult,
    pub put_CompareMode: unsafe extern "system" fn(this: *mut c_void, comp_mode: i32) -> HResult,
    pub Remove:
        unsafe extern "system" fn(this: *mut c_void, key: *const crate::types::Variant) -> HResult,
    pub RemoveAll: unsafe extern "system" fn(this: *mut c_void) -> HResult,
    pub Keys: unsafe extern "system" fn(
        this: *mut c_void,
        ret_keys: *mut crate::types::Variant,
    ) -> HResult,
    pub get_CompareMode:
        unsafe extern "system" fn(this: *mut c_void, ret_comp_mode: *mut i32) -> HResult,
    pub _NewEnum:
        unsafe extern "system" fn(this: *mut c_void, ret_enum: *mut *mut c_void) -> HResult,
    pub HashVal: unsafe extern "system" fn(
        this: *mut c_void,
        key: *const crate::types::Variant,
        ret_hash: *mut crate::types::Variant,
    ) -> HResult,
}

/// In-memory representation of a `Scripting.Dictionary` COM Automation object.
#[repr(C)]
pub struct DictionaryObject {
    pub vtable: *const IDictionaryVtbl,
    pub ref_count: AtomicU32,
    pub magic: u32,
    pub items: std::sync::Mutex<std::collections::HashMap<String, crate::types::Variant>>,
    /// Thread ID of the Single-Threaded Apartment (STA) where this object was instantiated.
    pub owner_thread: Option<std::thread::ThreadId>,
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
    if iid == IID_IUnknown || iid == IID_IDispatch || iid == crate::types::IID_IDictionary {
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

unsafe extern "system" fn dict_GetTypeInfoCount(_this: *mut c_void, pctinfo: *mut u32) -> HResult {
    if !pctinfo.is_null() {
        *pctinfo = 1;
    }
    S_OK
}

unsafe extern "system" fn dict_GetTypeInfo(
    _this: *mut c_void,
    iTInfo: u32,
    _lcid: u32,
    ppTInfo: *mut *mut c_void,
) -> HResult {
    if ppTInfo.is_null() {
        return E_POINTER;
    }
    if iTInfo != 0 {
        *ppTInfo = core::ptr::null_mut();
        return DISP_E_BADINDEX;
    }
    *ppTInfo = NativeTypeInfo::new(
        "Dictionary",
        crate::types::IID_IDictionary,
        &[
            "Add",
            "Exists",
            "Items",
            "Keys",
            "Remove",
            "RemoveAll",
            "Count",
            "Item",
            "CompareMode",
        ],
    );
    S_OK
}

unsafe extern "system" fn dict_GetIDsOfNames(
    this: *mut c_void,
    _riid: *const Iid,
    rgszNames: *const *const u16,
    cNames: u32,
    _lcid: u32,
    rgDispId: *mut i32,
) -> HResult {
    if this.is_null() || rgszNames.is_null() || rgDispId.is_null() {
        return E_POINTER;
    }
    let mut hr = S_OK;
    for i in 0..cNames {
        let name_ptr = *rgszNames.add(i as usize);
        if name_ptr.is_null() {
            *rgDispId.add(i as usize) = DISPID_UNKNOWN;
            hr = DISP_E_UNKNOWNNAME;
            continue;
        }
        let mut len = 0;
        while *name_ptr.add(len) != 0 {
            len += 1;
        }
        let name = String::from_utf16_lossy(core::slice::from_raw_parts(name_ptr, len));
        let dispid = match name.to_ascii_lowercase().as_str() {
            "item" => 0,
            "add" => 1,
            "count" => 2,
            "exists" => 3,
            "items" => 4,
            "key" => 5,
            "keys" => 6,
            "remove" => 7,
            "removeall" => 8,
            "comparemode" => 9,
            _ => {
                hr = DISP_E_UNKNOWNNAME;
                DISPID_UNKNOWN
            }
        };
        *rgDispId.add(i as usize) = dispid;
    }
    hr
}

unsafe extern "system" fn dict_Invoke(
    this: *mut c_void,
    dispIdMember: i32,
    _riid: *const Iid,
    _lcid: u32,
    wFlags: u16,
    pDispParams: *mut c_void,
    pVarResult: *mut c_void,
    _pExcepInfo: *mut c_void,
    _puArgErr: *mut u32,
) -> HResult {
    if this.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut DictionaryObject);
    let hr_aff = obj.check_thread_affinity();
    if hr_aff != S_OK {
        return hr_aff;
    }
    let params = if pDispParams.is_null() {
        DispParams {
            rgvarg: core::ptr::null_mut(),
            rgdispidNamedArgs: core::ptr::null_mut(),
            cArgs: 0,
            cNamedArgs: 0,
        }
    } else {
        *(pDispParams as *const DispParams)
    };

    match dispIdMember {
        0 => {
            // Item (Default property)
            if (wFlags & (DISPATCH_PROPERTYPUT | DISPATCH_PROPERTYPUTREF)) != 0 {
                if params.cArgs < 2 || params.rgvarg.is_null() {
                    return DISP_E_BADPARAMCOUNT;
                }
                // In property put: rgvarg[0] = value, rgvarg[1] = key
                let val_arg = &*params.rgvarg;
                let key_arg = &*params.rgvarg.add(1);
                dict_put_Item(this, key_arg, val_arg)
            } else if (wFlags & (DISPATCH_PROPERTYGET | DISPATCH_METHOD)) != 0 {
                if params.cArgs < 1 || params.rgvarg.is_null() {
                    return DISP_E_BADPARAMCOUNT;
                }
                let key_arg = &*params.rgvarg;
                if !pVarResult.is_null() {
                    dict_get_Item(this, key_arg, pVarResult as *mut Variant)
                } else {
                    let mut tmp = Variant::empty();
                    dict_get_Item(this, key_arg, &mut tmp)
                }
            } else {
                DISP_E_MEMBERNOTFOUND
            }
        }
        1 => {
            // Add(key, item)
            if params.cArgs < 2 || params.rgvarg.is_null() {
                return DISP_E_BADPARAMCOUNT;
            }
            // Reverse order in rgvarg: rgvarg[1] = key, rgvarg[0] = item
            let key_arg = &*params.rgvarg.add(1);
            let item_arg = &*params.rgvarg;
            dict_Add(this, key_arg, item_arg)
        }
        2 => {
            // Count
            let mut count: i32 = 0;
            let hr = dict_get_Count(this, &mut count);
            if hr == S_OK && !pVarResult.is_null() {
                *(pVarResult as *mut Variant) = Variant::from_i32(count);
            }
            hr
        }
        3 => {
            // Exists(key)
            if params.cArgs < 1 || params.rgvarg.is_null() {
                return DISP_E_BADPARAMCOUNT;
            }
            let key_arg = &*params.rgvarg;
            let mut exists_b: i16 = 0;
            let hr = dict_Exists(this, key_arg, &mut exists_b);
            if hr == S_OK && !pVarResult.is_null() {
                *(pVarResult as *mut Variant) = Variant::from_bool(exists_b != 0);
            }
            hr
        }
        4 => {
            // Items()
            if !pVarResult.is_null() {
                dict_Items(this, pVarResult as *mut Variant)
            } else {
                let mut tmp = Variant::empty();
                dict_Items(this, &mut tmp)
            }
        }
        5 => {
            // Key(old_key) = new_key
            if (wFlags & (DISPATCH_PROPERTYPUT | DISPATCH_PROPERTYPUTREF)) != 0 {
                if params.cArgs < 2 || params.rgvarg.is_null() {
                    return DISP_E_BADPARAMCOUNT;
                }
                let new_key = &*params.rgvarg;
                let old_key = &*params.rgvarg.add(1);
                dict_put_Key(this, old_key, new_key)
            } else {
                DISP_E_MEMBERNOTFOUND
            }
        }
        6 => {
            // Keys()
            if !pVarResult.is_null() {
                dict_Keys(this, pVarResult as *mut Variant)
            } else {
                let mut tmp = Variant::empty();
                dict_Keys(this, &mut tmp)
            }
        }
        7 => {
            // Remove(key)
            if params.cArgs < 1 || params.rgvarg.is_null() {
                return DISP_E_BADPARAMCOUNT;
            }
            let key_arg = &*params.rgvarg;
            dict_Remove(this, key_arg)
        }
        8 => {
            // RemoveAll()
            dict_RemoveAll(this)
        }
        9 => {
            // CompareMode
            if (wFlags & (DISPATCH_PROPERTYPUT | DISPATCH_PROPERTYPUTREF)) != 0 {
                if params.cArgs < 1 || params.rgvarg.is_null() {
                    return DISP_E_BADPARAMCOUNT;
                }
                let mode_val = (*params.rgvarg).as_i32().unwrap_or(0);
                dict_put_CompareMode(this, mode_val)
            } else {
                DISP_E_MEMBERNOTFOUND
            }
        }
        -4 => {
            // _NewEnum
            let mut p_enum: *mut c_void = core::ptr::null_mut();
            let hr = dict__NewEnum(this, &mut p_enum);
            if hr == S_OK && !pVarResult.is_null() {
                *(pVarResult as *mut Variant) = Variant::from_unknown(p_enum);
            }
            hr
        }
        _ => DISP_E_MEMBERNOTFOUND,
    }
}

unsafe extern "system" fn dict_put_Item(
    this: *mut c_void,
    key: *const crate::types::Variant,
    item: *const crate::types::Variant,
) -> HResult {
    if this.is_null() || key.is_null() || item.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut DictionaryObject);
    let hr_aff = obj.check_thread_affinity();
    if hr_aff != S_OK {
        return hr_aff;
    }
    let k_str = (*key).to_string_representation();
    let mut map = obj.items.lock().unwrap();
    let mut item_clone = crate::types::Variant::empty();
    crate::vba::__vbaVarDup(&mut item_clone, item);
    map.insert(k_str, item_clone);
    S_OK
}

unsafe extern "system" fn dict_putref_Item(
    this: *mut c_void,
    key: *const crate::types::Variant,
    item: *const crate::types::Variant,
) -> HResult {
    dict_put_Item(this, key, item)
}

unsafe extern "system" fn dict_get_Item(
    this: *mut c_void,
    key: *const crate::types::Variant,
    ret_item: *mut crate::types::Variant,
) -> HResult {
    if this.is_null() || key.is_null() || ret_item.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut DictionaryObject);
    let hr_aff = obj.check_thread_affinity();
    if hr_aff != S_OK {
        return hr_aff;
    }
    let k_str = (*key).to_string_representation();
    let mut map = obj.items.lock().unwrap();
    if let Some(val) = map.get(&k_str) {
        crate::vba::__vbaVarDup(ret_item, val);
    } else {
        let empty_var = crate::types::Variant::empty();
        map.insert(k_str, empty_var);
        *ret_item = crate::types::Variant::empty();
    }
    S_OK
}

unsafe extern "system" fn dict_Add(
    this: *mut c_void,
    key: *const crate::types::Variant,
    item: *const crate::types::Variant,
) -> HResult {
    if this.is_null() || key.is_null() || item.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut DictionaryObject);
    let hr_aff = obj.check_thread_affinity();
    if hr_aff != S_OK {
        return hr_aff;
    }
    let k_str = (*key).to_string_representation();
    let mut map = obj.items.lock().unwrap();
    if map.contains_key(&k_str) {
        return -2146827771; // 0x800A01C9 (Error 457)
    }
    let mut item_clone = crate::types::Variant::empty();
    crate::vba::__vbaVarDup(&mut item_clone, item);
    map.insert(k_str, item_clone);
    S_OK
}

unsafe extern "system" fn dict_get_Count(this: *mut c_void, ret_count: *mut i32) -> HResult {
    if this.is_null() || ret_count.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut DictionaryObject);
    let hr_aff = obj.check_thread_affinity();
    if hr_aff != S_OK {
        return hr_aff;
    }
    let map = obj.items.lock().unwrap();
    *ret_count = map.len() as i32;
    S_OK
}

unsafe extern "system" fn dict_Exists(
    this: *mut c_void,
    key: *const crate::types::Variant,
    ret_exists: *mut i16,
) -> HResult {
    if this.is_null() || key.is_null() || ret_exists.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut DictionaryObject);
    let hr_aff = obj.check_thread_affinity();
    if hr_aff != S_OK {
        return hr_aff;
    }
    let k_str = (*key).to_string_representation();
    let map = obj.items.lock().unwrap();
    *ret_exists = if map.contains_key(&k_str) { -1 } else { 0 };
    S_OK
}

unsafe extern "system" fn dict_Items(
    this: *mut c_void,
    ret_items: *mut crate::types::Variant,
) -> HResult {
    if this.is_null() || ret_items.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut DictionaryObject);
    let map = obj.items.lock().unwrap();
    let count = map.len() as u32;
    let psa = crate::types::SafeArray::create_vector(
        core::mem::size_of::<crate::types::Variant>() as u32,
        0,
        count,
        crate::types::safearray::FADF_HAVEVARTYPE | crate::types::safearray::FADF_VARIANT,
    );
    if !psa.is_null() && !(*psa).pv_data.is_null() {
        let p_elem = (*psa).pv_data as *mut crate::types::Variant;
        for (i, (_, val)) in map.iter().enumerate() {
            crate::vba::__vbaVarDup(p_elem.add(i), val);
        }
    }
    let mut v = crate::types::Variant::empty();
    v.vt = crate::types::variant::VT_ARRAY | crate::types::variant::VT_VARIANT;
    let ptr_bytes = (psa as usize).to_ne_bytes();
    v.data[..ptr_bytes.len()].copy_from_slice(&ptr_bytes);
    *ret_items = v;
    S_OK
}

unsafe extern "system" fn dict_put_Key(
    this: *mut c_void,
    key: *const crate::types::Variant,
    new_key: *const crate::types::Variant,
) -> HResult {
    if this.is_null() || key.is_null() || new_key.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut DictionaryObject);
    let old_k = (*key).to_string_representation();
    let new_k = (*new_key).to_string_representation();
    let mut map = obj.items.lock().unwrap();
    if let Some(val) = map.remove(&old_k) {
        map.insert(new_k, val);
    }
    S_OK
}

unsafe extern "system" fn dict_put_CompareMode(_this: *mut c_void, _comp_mode: i32) -> HResult {
    S_OK
}

unsafe extern "system" fn dict_Remove(
    this: *mut c_void,
    key: *const crate::types::Variant,
) -> HResult {
    if this.is_null() || key.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut DictionaryObject);
    let k_str = (*key).to_string_representation();
    let mut map = obj.items.lock().unwrap();
    map.remove(&k_str);
    S_OK
}

unsafe extern "system" fn dict_RemoveAll(this: *mut c_void) -> HResult {
    if this.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut DictionaryObject);
    let mut map = obj.items.lock().unwrap();
    map.clear();
    S_OK
}

unsafe extern "system" fn dict_Keys(
    this: *mut c_void,
    ret_keys: *mut crate::types::Variant,
) -> HResult {
    if this.is_null() || ret_keys.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut DictionaryObject);
    let map = obj.items.lock().unwrap();
    let count = map.len() as u32;
    let psa = crate::types::SafeArray::create_vector(
        core::mem::size_of::<crate::types::Variant>() as u32,
        0,
        count,
        crate::types::safearray::FADF_HAVEVARTYPE | crate::types::safearray::FADF_VARIANT,
    );
    if !psa.is_null() && !(*psa).pv_data.is_null() {
        let p_elem = (*psa).pv_data as *mut crate::types::Variant;
        for (i, (k, _)) in map.iter().enumerate() {
            let b = crate::types::sys_alloc_string(k);
            *p_elem.add(i) = crate::types::Variant::from_bstr(b);
        }
    }
    let mut v = crate::types::Variant::empty();
    v.vt = crate::types::variant::VT_ARRAY | crate::types::variant::VT_VARIANT;
    let ptr_bytes = (psa as usize).to_ne_bytes();
    v.data[..ptr_bytes.len()].copy_from_slice(&ptr_bytes);
    *ret_keys = v;
    S_OK
}

unsafe extern "system" fn dict_get_CompareMode(
    _this: *mut c_void,
    ret_comp_mode: *mut i32,
) -> HResult {
    if !ret_comp_mode.is_null() {
        *ret_comp_mode = 0;
    }
    S_OK
}

#[repr(C)]
pub struct EnumVariantObject {
    pub vtable: *const IEnumVARIANTVtbl,
    pub ref_count: AtomicU32,
    pub items: Vec<Variant>,
    pub current_index: usize,
}

unsafe impl Sync for EnumVariantObject {}
unsafe impl Send for EnumVariantObject {}

unsafe extern "system" fn enum_QueryInterface(
    this: *mut c_void,
    riid: *const Iid,
    ppv: *mut *mut c_void,
) -> HResult {
    if ppv.is_null() {
        return E_POINTER;
    }
    if riid.is_null() {
        *ppv = core::ptr::null_mut();
        return E_INVALIDARG;
    }
    let iid = *riid;
    if iid == IID_IUnknown || iid == IID_IEnumVARIANT {
        let obj = &*(this as *mut EnumVariantObject);
        obj.ref_count.fetch_add(1, Ordering::SeqCst);
        *ppv = this;
        return S_OK;
    }
    *ppv = core::ptr::null_mut();
    E_NOINTERFACE
}

unsafe extern "system" fn enum_AddRef(this: *mut c_void) -> u32 {
    let obj = &*(this as *mut EnumVariantObject);
    obj.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn enum_Release(this: *mut c_void) -> u32 {
    let obj_ptr = this as *mut EnumVariantObject;
    let old_count = (*obj_ptr).ref_count.fetch_sub(1, Ordering::SeqCst);
    if old_count == 1 {
        let _ = Box::from_raw(obj_ptr);
        0
    } else {
        old_count - 1
    }
}

unsafe extern "system" fn enum_Next(
    this: *mut c_void,
    celt: u32,
    rgVar: *mut Variant,
    pceltFetched: *mut u32,
) -> HResult {
    if this.is_null() || (celt > 1 && rgVar.is_null()) {
        return E_POINTER;
    }
    if celt == 0 {
        return S_OK;
    }
    let obj = &mut *(this as *mut EnumVariantObject);
    let mut count = 0;
    while count < celt && obj.current_index < obj.items.len() {
        if !rgVar.is_null() {
            *rgVar.add(count as usize) = obj.items[obj.current_index];
        }
        obj.current_index += 1;
        count += 1;
    }
    if !pceltFetched.is_null() {
        *pceltFetched = count;
    }
    if count == celt {
        S_OK
    } else {
        S_FALSE
    }
}

unsafe extern "system" fn enum_Skip(this: *mut c_void, celt: u32) -> HResult {
    if this.is_null() {
        return E_POINTER;
    }
    let obj = &mut *(this as *mut EnumVariantObject);
    obj.current_index = (obj.current_index + celt as usize).min(obj.items.len());
    S_OK
}

unsafe extern "system" fn enum_Reset(this: *mut c_void) -> HResult {
    if this.is_null() {
        return E_POINTER;
    }
    let obj = &mut *(this as *mut EnumVariantObject);
    obj.current_index = 0;
    S_OK
}

unsafe extern "system" fn enum_Clone(this: *mut c_void, ppEnum: *mut *mut c_void) -> HResult {
    if ppEnum.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut EnumVariantObject);
    let clone_obj = Box::new(EnumVariantObject {
        vtable: &ENUM_VARIANT_VTBL,
        ref_count: AtomicU32::new(1),
        items: obj.items.clone(),
        current_index: obj.current_index,
    });
    *ppEnum = Box::into_raw(clone_obj) as *mut c_void;
    S_OK
}

pub static ENUM_VARIANT_VTBL: IEnumVARIANTVtbl = IEnumVARIANTVtbl {
    QueryInterface: enum_QueryInterface,
    AddRef: enum_AddRef,
    Release: enum_Release,
    Next: enum_Next,
    Skip: enum_Skip,
    Reset: enum_Reset,
    Clone: enum_Clone,
};

impl EnumVariantObject {
    pub fn new(items: Vec<Variant>) -> *mut EnumVariantObject {
        let boxed = Box::new(EnumVariantObject {
            vtable: &ENUM_VARIANT_VTBL,
            ref_count: AtomicU32::new(1),
            items,
            current_index: 0,
        });
        Box::into_raw(boxed)
    }
}

unsafe extern "system" fn dict__NewEnum(this: *mut c_void, ret_enum: *mut *mut c_void) -> HResult {
    if this.is_null() || ret_enum.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut DictionaryObject);
    let map = obj.items.lock().unwrap();
    let keys: Vec<Variant> = map.keys().map(|k| Variant::from_bstr(sys_alloc_string(k))).collect();
    *ret_enum = EnumVariantObject::new(keys) as *mut c_void;
    S_OK
}

unsafe extern "system" fn dict_HashVal(
    _this: *mut c_void,
    _key: *const crate::types::Variant,
    ret_hash: *mut crate::types::Variant,
) -> HResult {
    if !ret_hash.is_null() {
        *ret_hash = crate::types::Variant::empty();
    }
    S_OK
}

static DICT_VTBL: IDictionaryVtbl = IDictionaryVtbl {
    QueryInterface: dict_QueryInterface,
    AddRef: dict_AddRef,
    Release: dict_Release,
    GetTypeInfoCount: dict_GetTypeInfoCount,
    GetTypeInfo: dict_GetTypeInfo,
    GetIDsOfNames: dict_GetIDsOfNames,
    Invoke: dict_Invoke,
    putref_Item: dict_putref_Item,
    put_Item: dict_put_Item,
    get_Item: dict_get_Item,
    Add: dict_Add,
    get_Count: dict_get_Count,
    Exists: dict_Exists,
    Items: dict_Items,
    put_Key: dict_put_Key,
    put_CompareMode: dict_put_CompareMode,
    Remove: dict_Remove,
    RemoveAll: dict_RemoveAll,
    Keys: dict_Keys,
    get_CompareMode: dict_get_CompareMode,
    _NewEnum: dict__NewEnum,
    HashVal: dict_HashVal,
};

impl DictionaryObject {
    /// Creates and returns a newly allocated, heap-backed `DictionaryObject` bound to the caller's STA apartment thread.
    pub fn new() -> *mut DictionaryObject {
        let boxed = Box::new(DictionaryObject {
            vtable: &DICT_VTBL,
            ref_count: AtomicU32::new(1),
            magic: DICTIONARY_MAGIC,
            items: std::sync::Mutex::new(std::collections::HashMap::new()),
            owner_thread: Some(std::thread::current().id()),
        });
        Box::into_raw(boxed)
    }

    /// Verifies whether the calling thread matches the object's STA apartment affinity.
    ///
    /// # Returns
    /// * `S_OK` if within the apartment or free-threaded.
    /// * `RPC_E_WRONG_THREAD` (`0x8001010E`) if cross-apartment invocation without marshaling.
    #[inline]
    pub fn check_thread_affinity(&self) -> HResult {
        if let Some(owner) = self.owner_thread {
            if owner != std::thread::current().id() {
                return crate::types::RPC_E_WRONG_THREAD;
            }
        }
        S_OK
    }

    /// Sets or clears the STA apartment thread affinity.
    pub fn set_owner_thread(&mut self, thread_id: Option<std::thread::ThreadId>) {
        self.owner_thread = thread_id;
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
    if old > 0 {
        old - 1
    } else {
        0
    }
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

/// Retrieves a pointer to the `IClassFactory` implementation for `Scripting.Dictionary`.
///
/// # Safety
///
/// `riid` and `ppv` must point to valid memory or null according to standard COM calling conventions.
pub unsafe fn get_dictionary_class_factory(riid: *const Iid, ppv: *mut *mut c_void) -> HResult {
    cf_QueryInterface(
        &DICT_FACTORY as *const DictionaryClassFactory as *mut c_void,
        riid,
        ppv,
    )
}

/// Determines the friendly type name for an active COM object pointer.
pub unsafe fn get_object_type_name(p_obj: *mut c_void) -> Option<String> {
    if p_obj.is_null() {
        return None;
    }
    // 1. Try querying IDispatch::GetTypeInfo(0) -> ITypeInfo::GetDocumentation(-1)
    let p_unk = p_obj as *const *const usize;
    if !p_unk.is_null() && !(*p_unk).is_null() {
        let vtbl = *p_unk;
        // In standard IDispatch, GetTypeInfo is method index 4 (offset 16 bytes)
        let get_type_info_fn: unsafe extern "system" fn(
            *mut c_void,
            u32,
            u32,
            *mut *mut c_void,
        ) -> HResult = core::mem::transmute(*vtbl.add(4));
        let mut p_ti: *mut c_void = core::ptr::null_mut();
        let hr = get_type_info_fn(p_obj, 0, 0x0409, &mut p_ti);
        if hr == S_OK && !p_ti.is_null() {
            let ti_vtbl = *(p_ti as *const *const usize);
            // In standard ITypeInfo, GetDocumentation is method index 12 (offset 48 bytes)
            let get_doc_fn: unsafe extern "system" fn(
                *mut c_void,
                i32,
                *mut Bstr,
                *mut Bstr,
                *mut u32,
                *mut Bstr,
            ) -> HResult = core::mem::transmute(*ti_vtbl.add(12));
            let mut name_bstr: Bstr = core::ptr::null_mut();
            let hr_doc = get_doc_fn(
                p_ti,
                -1,
                &mut name_bstr,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            );
            let release_fn: unsafe extern "system" fn(*mut c_void) -> u32 =
                core::mem::transmute(*ti_vtbl.add(2));
            release_fn(p_ti);
            if hr_doc == S_OK && !name_bstr.is_null() {
                let name = crate::types::bstr_to_string(name_bstr);
                SysFreeString(name_bstr);
                return Some(name);
            }
        }
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
    } else if clsid == crate::types::CLSID_FileSystemObject {
        return crate::activex::get_fso_class_factory(riid, ppv);
    } else if clsid == crate::types::CLSID_WshShell {
        return crate::activex::get_wsh_shell_class_factory(riid, ppv);
    } else if clsid == crate::types::CLSID_WshNetwork {
        return crate::activex::get_wsh_network_class_factory(riid, ppv);
    } else if clsid == crate::types::CLSID_Recordset {
        return crate::activex::get_recordset_class_factory(riid, ppv);
    } else if clsid == crate::types::CLSID_ScriptControl {
        return crate::activex::get_scriptcontrol_class_factory(riid, ppv);
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

/// Method invocation on zombie objects strictly returns E_UNEXPECTED (0x8000FFFF).
#[no_mangle]
pub unsafe extern "system" fn Zombie_Invoke() -> HResult {
    crate::types::E_UNEXPECTED
}

pub static ZOMBIE_IDISPATCH_VTBL: IDispatchVtbl = IDispatchVtbl {
    QueryInterface: Zombie_QueryInterface,
    AddRef: Zombie_AddRef,
    Release: Zombie_Release,
    GetTypeInfoCount: Zombie_GetTypeInfoCount_Vtbl,
    GetTypeInfo: Zombie_GetTypeInfo_Vtbl,
    GetIDsOfNames: Zombie_GetIDsOfNames_Vtbl,
    Invoke: Zombie_Invoke_Vtbl,
};

unsafe extern "system" fn Zombie_GetTypeInfoCount_Vtbl(
    _this: *mut c_void,
    pctinfo: *mut u32,
) -> HResult {
    if !pctinfo.is_null() {
        *pctinfo = 0;
    }
    S_OK
}

unsafe extern "system" fn Zombie_GetTypeInfo_Vtbl(
    _this: *mut c_void,
    _i_t_info: u32,
    _lcid: u32,
    pp_t_info: *mut *mut c_void,
) -> HResult {
    if !pp_t_info.is_null() {
        *pp_t_info = core::ptr::null_mut();
    }
    crate::types::E_UNEXPECTED
}

unsafe extern "system" fn Zombie_GetIDsOfNames_Vtbl(
    _this: *mut c_void,
    _riid: *const Iid,
    _rgsz_names: *const *const u16,
    _c_names: u32,
    _lcid: u32,
    _rg_disp_id: *mut i32,
) -> HResult {
    crate::types::E_UNEXPECTED
}

unsafe extern "system" fn Zombie_Invoke_Vtbl(
    _this: *mut c_void,
    _disp_id_member: i32,
    _riid: *const Iid,
    _lcid: u32,
    _w_flags: u16,
    _p_disp_params: *mut c_void,
    _p_var_result: *mut c_void,
    _p_excep_info: *mut c_void,
    _pu_arg_err: *mut u32,
) -> HResult {
    crate::types::E_UNEXPECTED
}

/// VB6 Class instance modeling reference counting, circular graphs, and Zombie resurrection.
#[repr(C)]
pub struct VbClassInstance {
    pub vtbl: *const IDispatchVtbl,
    pub ref_count: AtomicU32,
    pub is_zombie: bool,
    pub is_terminating: bool,
    pub name: String,
    pub partner: Option<*mut VbClassInstance>,
    pub terminate_action: Option<Box<dyn Fn(*mut VbClassInstance)>>,
}

impl VbClassInstance {
    pub fn new(name: &str) -> *mut Self {
        let instance = Box::new(Self {
            vtbl: &BASIC_CLASS_IDISPATCH_VTBL,
            ref_count: AtomicU32::new(1),
            is_zombie: false,
            is_terminating: false,
            name: name.to_string(),
            partner: None,
            terminate_action: None,
        });
        Box::into_raw(instance)
    }

    pub unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
        let inst = this as *mut Self;
        if inst.is_null() {
            return 0;
        }
        (*inst).ref_count.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub unsafe extern "system" fn release(this: *mut c_void) -> u32 {
        let inst = this as *mut Self;
        if inst.is_null() {
            return 0;
        }
        let old = (*inst).ref_count.fetch_sub(1, Ordering::SeqCst);
        if old == 1 {
            if !(*inst).is_terminating {
                (*inst).is_terminating = true;
                if let Some(ref action) = (*inst).terminate_action {
                    action(inst);
                }
                if (*inst).ref_count.load(Ordering::SeqCst) > 0 {
                    (*inst).is_zombie = true;
                    (*inst).vtbl = &ZOMBIE_IDISPATCH_VTBL;
                    return (*inst).ref_count.load(Ordering::SeqCst);
                }
            }
            if let Some(p) = (*inst).partner.take() {
                Self::release(p as *mut c_void);
            }
            let _ = Box::from_raw(inst);
            return 0;
        }
        old - 1
    }

    pub unsafe extern "system" fn invoke(
        this: *mut c_void,
        _disp_id: i32,
        _riid: *const Iid,
        _lcid: u32,
        _flags: u16,
        _params: *mut c_void,
        _result: *mut c_void,
        _excep: *mut c_void,
        _arg_err: *mut u32,
    ) -> HResult {
        let inst = this as *mut Self;
        if inst.is_null() {
            return E_POINTER;
        }
        if (*inst).is_zombie {
            return crate::types::E_UNEXPECTED;
        }
        S_OK
    }
}

pub static BASIC_CLASS_IDISPATCH_VTBL: IDispatchVtbl = IDispatchVtbl {
    QueryInterface: BASIC_CLASS_QueryInterface,
    AddRef: VbClassInstance::add_ref,
    Release: VbClassInstance::release,
    GetTypeInfoCount: BASIC_CLASS_GetTypeInfoCount_Vtbl,
    GetTypeInfo: BASIC_CLASS_GetTypeInfo_Vtbl,
    GetIDsOfNames: BASIC_CLASS_GetIDsOfNames_Vtbl,
    Invoke: VbClassInstance::invoke,
};

unsafe extern "system" fn BASIC_CLASS_GetTypeInfoCount_Vtbl(
    _this: *mut c_void,
    pctinfo: *mut u32,
) -> HResult {
    if !pctinfo.is_null() {
        *pctinfo = 0;
    }
    S_OK
}

unsafe extern "system" fn BASIC_CLASS_GetTypeInfo_Vtbl(
    _this: *mut c_void,
    _i_t_info: u32,
    _lcid: u32,
    pp_t_info: *mut *mut c_void,
) -> HResult {
    if !pp_t_info.is_null() {
        *pp_t_info = core::ptr::null_mut();
    }
    S_OK
}

unsafe extern "system" fn BASIC_CLASS_GetIDsOfNames_Vtbl(
    _this: *mut c_void,
    _riid: *const Iid,
    _rgsz_names: *const *const u16,
    _c_names: u32,
    _lcid: u32,
    _rg_disp_id: *mut i32,
) -> HResult {
    S_OK
}

/// Invokes a member on an `IDispatch` object by its UTF-16 member name.
///
/// # Safety
///
/// `p_obj` must point to a valid COM object implementing `IDispatch` or null.
pub unsafe fn dispatch_invoke_member(
    p_obj: *mut c_void,
    member_name: *const u16,
    w_flags: u16,
    args: *mut Variant,
    c_args: u32,
    p_ret: *mut Variant,
) -> HResult {
    if p_obj.is_null() {
        crate::err::set_vb_error(91, None, None); // Object variable or With block variable not set
        return E_POINTER;
    }
    let dispid = if member_name.is_null() {
        DISPID_VALUE
    } else {
        let vtable = *(p_obj as *mut *mut usize);
        if vtable.is_null() {
            crate::err::set_vb_error(91, None, None);
            return E_POINTER;
        }
        let get_ids_fn: unsafe extern "system" fn(
            *mut c_void,
            *const Iid,
            *const *const u16,
            u32,
            u32,
            *mut i32,
        ) -> HResult = core::mem::transmute(*vtable.add(5));
        let mut id: i32 = DISPID_UNKNOWN;
        let rgsz: [*const u16; 1] = [member_name];
        let hr = get_ids_fn(p_obj, &IID_NULL, rgsz.as_ptr(), 1, 0, &mut id);
        if hr != S_OK || id == DISPID_UNKNOWN {
            crate::err::set_vb_error(438, None, None); // Object doesn't support this property or method
            return DISP_E_UNKNOWNNAME;
        }
        id
    };

    dispatch_invoke_id(p_obj, dispid, w_flags, args, c_args, p_ret)
}

/// Invokes a member on an `IDispatch` object by its DISPID.
///
/// # Safety
///
/// `p_obj` must point to a valid COM object implementing `IDispatch` or null.
pub unsafe fn dispatch_invoke_id(
    p_obj: *mut c_void,
    dispid: i32,
    w_flags: u16,
    args: *mut Variant,
    c_args: u32,
    p_ret: *mut Variant,
) -> HResult {
    if p_obj.is_null() {
        crate::err::set_vb_error(91, None, None);
        return E_POINTER;
    }
    let vtable = *(p_obj as *mut *mut usize);
    if vtable.is_null() {
        crate::err::set_vb_error(91, None, None);
        return E_POINTER;
    }
    let invoke_fn: unsafe extern "system" fn(
        *mut c_void,
        i32,
        *const Iid,
        u32,
        u16,
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *mut u32,
    ) -> HResult = core::mem::transmute(*vtable.add(6));

    let mut named_put = DISPID_PROPERTYPUT;
    let mut params = if (w_flags & (DISPATCH_PROPERTYPUT | DISPATCH_PROPERTYPUTREF)) != 0 {
        DispParams {
            rgvarg: args,
            rgdispidNamedArgs: &mut named_put,
            cArgs: c_args + 1,
            cNamedArgs: 1,
        }
    } else {
        DispParams {
            rgvarg: args,
            rgdispidNamedArgs: core::ptr::null_mut(),
            cArgs: c_args,
            cNamedArgs: 0,
        }
    };

    let hr = invoke_fn(
        p_obj,
        dispid,
        &IID_NULL,
        0,
        w_flags,
        &mut params as *mut DispParams as *mut c_void,
        p_ret as *mut c_void,
        core::ptr::null_mut(),
        core::ptr::null_mut(),
    );

    if hr != S_OK {
        if hr == DISP_E_UNKNOWNNAME || hr == DISP_E_MEMBERNOTFOUND {
            crate::err::set_vb_error(438, None, None);
        } else if hr == DISP_E_BADPARAMCOUNT {
            crate::err::set_vb_error(450, None, None);
        } else {
            crate::err::set_vb_error(440, None, None);
        }
    }
    hr
}

// ----------------------------------------------------------------------------
// Visual Basic 6 Collection Object (VbCollectionObject)
// ----------------------------------------------------------------------------

#[repr(C)]
pub struct VbCollectionObject {
    pub vtable: *const IDispatchVtbl,
    pub ref_count: AtomicU32,
    pub items: std::sync::Mutex<Vec<(Option<String>, Variant)>>,
}

unsafe impl Sync for VbCollectionObject {}
unsafe impl Send for VbCollectionObject {}

unsafe extern "system" fn coll_QueryInterface(
    this: *mut c_void,
    riid: *const Iid,
    ppv: *mut *mut c_void,
) -> HResult {
    if ppv.is_null() {
        return E_POINTER;
    }
    if riid.is_null() {
        *ppv = core::ptr::null_mut();
        return E_INVALIDARG;
    }
    let iid = *riid;
    if iid == IID_IUnknown || iid == IID_IDispatch {
        let obj = &*(this as *mut VbCollectionObject);
        obj.ref_count.fetch_add(1, Ordering::SeqCst);
        *ppv = this;
        return S_OK;
    }
    *ppv = core::ptr::null_mut();
    E_NOINTERFACE
}

unsafe extern "system" fn coll_AddRef(this: *mut c_void) -> u32 {
    let obj = &*(this as *mut VbCollectionObject);
    obj.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn coll_Release(this: *mut c_void) -> u32 {
    let obj_ptr = this as *mut VbCollectionObject;
    let old_count = (*obj_ptr).ref_count.fetch_sub(1, Ordering::SeqCst);
    if old_count == 1 {
        let _ = Box::from_raw(obj_ptr);
        0
    } else {
        old_count - 1
    }
}

unsafe extern "system" fn coll_GetTypeInfoCount(_this: *mut c_void, pctinfo: *mut u32) -> HResult {
    if !pctinfo.is_null() {
        *pctinfo = 0;
    }
    S_OK
}

unsafe extern "system" fn coll_GetTypeInfo(
    _this: *mut c_void,
    _iTInfo: u32,
    _lcid: u32,
    ppTInfo: *mut *mut c_void,
) -> HResult {
    if !ppTInfo.is_null() {
        *ppTInfo = core::ptr::null_mut();
    }
    E_NOTIMPL
}

unsafe extern "system" fn coll_GetIDsOfNames(
    _this: *mut c_void,
    _riid: *const Iid,
    rgszNames: *const *const u16,
    cNames: u32,
    _lcid: u32,
    rgDispId: *mut i32,
) -> HResult {
    if rgszNames.is_null() || rgDispId.is_null() || cNames == 0 {
        return E_POINTER;
    }
    let name_slice = core::slice::from_raw_parts(rgszNames, cNames as usize);
    let dispid_slice = core::slice::from_raw_parts_mut(rgDispId, cNames as usize);
    for (i, &p_name) in name_slice.iter().enumerate() {
        if p_name.is_null() {
            dispid_slice[i] = DISPID_UNKNOWN;
            continue;
        }
        let mut len = 0;
        while *p_name.add(len) != 0 {
            len += 1;
        }
        let u16_slice = core::slice::from_raw_parts(p_name, len);
        let s = String::from_utf16_lossy(u16_slice).to_ascii_lowercase();
        match s.as_str() {
            "item" => dispid_slice[i] = 0,
            "add" => dispid_slice[i] = 1,
            "count" => dispid_slice[i] = 2,
            "remove" => dispid_slice[i] = 3,
            "_newenum" => dispid_slice[i] = -4,
            _ => dispid_slice[i] = DISPID_UNKNOWN,
        }
    }
    S_OK
}

unsafe extern "system" fn coll_Invoke(
    this: *mut c_void,
    disp_id: i32,
    _riid: *const Iid,
    _lcid: u32,
    _flags: u16,
    params: *mut c_void,
    result: *mut c_void,
    _excep: *mut c_void,
    _arg_err: *mut u32,
) -> HResult {
    if this.is_null() {
        return E_POINTER;
    }
    let obj = &*(this as *mut VbCollectionObject);
    let p_params = params as *mut DispParams;
    let (c_args, rgvarg) = if !p_params.is_null() {
        ((*p_params).cArgs, (*p_params).rgvarg)
    } else {
        (0, core::ptr::null_mut())
    };

    match disp_id {
        0 => {
            // Item(Index) - 1-based index or String key
            if c_args < 1 || rgvarg.is_null() {
                return DISP_E_BADPARAMCOUNT;
            }
            let key_arg = &*rgvarg;
            let list = obj.items.lock().unwrap();
            if let Some(idx) = key_arg.as_i32() {
                let u_idx = (idx - 1) as usize;
                if u_idx < list.len() {
                    if !result.is_null() {
                        *(result as *mut Variant) = list[u_idx].1;
                    }
                    return S_OK;
                } else {
                    return DISP_E_BADINDEX;
                }
            } else if let Some(b) = key_arg.as_bstr() {
                let k_str = crate::types::bstr_to_string(b);
                for (opt_k, val) in list.iter() {
                    if let Some(ref k) = opt_k {
                        if k.eq_ignore_ascii_case(&k_str) {
                            if !result.is_null() {
                                *(result as *mut Variant) = *val;
                            }
                            return S_OK;
                        }
                    }
                }
                return DISP_E_BADINDEX;
            }
            DISP_E_BADINDEX
        }
        1 => {
            // Add(Item, [Key], [Before], [After])
            if c_args < 1 || rgvarg.is_null() {
                return DISP_E_BADPARAMCOUNT;
            }
            let item_var = *rgvarg.add((c_args - 1) as usize);
            let key_opt = if c_args >= 2 {
                let key_var = &*rgvarg.add((c_args - 2) as usize);
                key_var.as_bstr().map(|b| unsafe { crate::types::bstr_to_string(b) })
            } else {
                None
            };
            let mut list = obj.items.lock().unwrap();
            list.push((key_opt, item_var));
            S_OK
        }
        2 => {
            // Count
            let list = obj.items.lock().unwrap();
            if !result.is_null() {
                *(result as *mut Variant) = Variant::from_i32(list.len() as i32);
            }
            S_OK
        }
        3 => {
            // Remove(Index)
            if c_args < 1 || rgvarg.is_null() {
                return DISP_E_BADPARAMCOUNT;
            }
            let key_arg = &*rgvarg;
            let mut list = obj.items.lock().unwrap();
            if let Some(idx) = key_arg.as_i32() {
                let u_idx = (idx - 1) as usize;
                if u_idx < list.len() {
                    list.remove(u_idx);
                    return S_OK;
                } else {
                    return DISP_E_BADINDEX;
                }
            } else if let Some(b) = key_arg.as_bstr() {
                let k_str = crate::types::bstr_to_string(b);
                let mut found_pos = None;
                for (pos, (opt_k, _)) in list.iter().enumerate() {
                    if let Some(ref k) = opt_k {
                        if k.eq_ignore_ascii_case(&k_str) {
                            found_pos = Some(pos);
                            break;
                        }
                    }
                }
                if let Some(p) = found_pos {
                    list.remove(p);
                    return S_OK;
                } else {
                    return DISP_E_BADINDEX;
                }
            }
            DISP_E_BADINDEX
        }
        -4 => {
            // _NewEnum
            let list = obj.items.lock().unwrap();
            let variants: Vec<Variant> = list.iter().map(|(_, v)| *v).collect();
            let enum_obj = EnumVariantObject::new(variants) as *mut c_void;
            if !result.is_null() {
                *(result as *mut Variant) = Variant::from_unknown(enum_obj);
            }
            S_OK
        }
        _ => DISP_E_MEMBERNOTFOUND,
    }
}

pub static VB_COLLECTION_VTBL: IDispatchVtbl = IDispatchVtbl {
    QueryInterface: coll_QueryInterface,
    AddRef: coll_AddRef,
    Release: coll_Release,
    GetTypeInfoCount: coll_GetTypeInfoCount,
    GetTypeInfo: coll_GetTypeInfo,
    GetIDsOfNames: coll_GetIDsOfNames,
    Invoke: coll_Invoke,
};

impl VbCollectionObject {
    pub fn new() -> *mut VbCollectionObject {
        let boxed = Box::new(VbCollectionObject {
            vtable: &VB_COLLECTION_VTBL,
            ref_count: AtomicU32::new(1),
            items: std::sync::Mutex::new(Vec::new()),
        });
        Box::into_raw(boxed)
    }

    pub fn add_item(&self, item: Variant, key: Option<String>) {
        let mut list = self.items.lock().unwrap();
        list.push((key, item));
    }
}

