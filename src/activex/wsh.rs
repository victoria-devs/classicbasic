// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! `WScript.Shell` and `WScript.Network` COM Automation Components (`wshom.ocx`).
//!
//! Provides host environment queries, directory management, and virtualized Windows Registry
//! access for Visual Basic 6.0 applications running on Linux.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::com::{
    DispParams, IDispatchVtbl, DISPID_UNKNOWN, DISP_E_MEMBERNOTFOUND, DISP_E_UNKNOWNNAME,
};
use crate::types::{
    bstr_to_string, sys_alloc_string, Bstr, E_NOINTERFACE, E_POINTER, HResult, Iid, Variant,
    IID_IDispatch, IID_IUnknown, S_OK, VT_BSTR, VT_I4,
};

// ----------------------------------------------------------------------------
// Parameter Extraction Helpers
// ----------------------------------------------------------------------------

unsafe fn extract_arg(p_params: *mut DispParams, idx: usize) -> Option<Variant> {
    if p_params.is_null() {
        return None;
    }
    let params = &*p_params;
    if idx >= params.cArgs as usize {
        return None;
    }
    let rev_idx = params.cArgs as usize - 1 - idx;
    Some(*params.rgvarg.add(rev_idx))
}

unsafe fn extract_string_arg(p_params: *mut DispParams, idx: usize) -> Option<String> {
    let var = extract_arg(p_params, idx)?;
    if var.vt == VT_BSTR {
        let bstr = core::ptr::read_unaligned(var.data.as_ptr() as *const Bstr);
        Some(bstr_to_string(bstr))
    } else if var.vt == (crate::types::VT_BYREF | crate::types::VT_VARIANT) {
        let inner = core::ptr::read_unaligned(var.data.as_ptr() as *const *mut Variant);
        if !inner.is_null() && (*inner).vt == VT_BSTR {
            let bstr = core::ptr::read_unaligned((*inner).data.as_ptr() as *const Bstr);
            Some(bstr_to_string(bstr))
        } else {
            None
        }
    } else {
        None
    }
}

// ----------------------------------------------------------------------------
// WshShellObject
// ----------------------------------------------------------------------------

const DISPID_WSH_CURRENTDIRECTORY: i32 = 4000;
const DISPID_WSH_EXPANDENV: i32 = 4001;
const DISPID_WSH_REGREAD: i32 = 4002;
const DISPID_WSH_REGWRITE: i32 = 4003;
const DISPID_WSH_REGDELETE: i32 = 4004;
const DISPID_WSH_RUN: i32 = 4005;
const DISPID_WSH_SPECIALFOLDERS: i32 = 4006;

#[repr(C)]
pub struct WshShellObject {
    pub vtable: *const IDispatchVtbl,
    pub ref_count: AtomicU32,
}

unsafe extern "system" fn wsh_QueryInterface(
    this: *mut core::ffi::c_void,
    riid: *const Iid,
    ppv: *mut *mut core::ffi::c_void,
) -> HResult {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    let iid = *riid;
    if iid == IID_IUnknown || iid == IID_IDispatch {
        *ppv = this;
        wsh_AddRef(this);
        return S_OK;
    }
    *ppv = core::ptr::null_mut();
    E_NOINTERFACE
}

unsafe extern "system" fn wsh_AddRef(this: *mut core::ffi::c_void) -> u32 {
    let obj = &*(this as *mut WshShellObject);
    obj.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn wsh_Release(this: *mut core::ffi::c_void) -> u32 {
    let obj_ptr = this as *mut WshShellObject;
    let old_count = (*obj_ptr).ref_count.fetch_sub(1, Ordering::SeqCst);
    if old_count == 1 {
        let _ = Box::from_raw(obj_ptr);
        0
    } else {
        old_count - 1
    }
}

unsafe extern "system" fn wsh_GetTypeInfoCount(
    _this: *mut core::ffi::c_void,
    pctinfo: *mut u32,
) -> HResult {
    if !pctinfo.is_null() {
        *pctinfo = 1;
    }
    S_OK
}

unsafe extern "system" fn wsh_GetTypeInfo(
    _this: *mut core::ffi::c_void,
    _iTInfo: u32,
    _lcid: u32,
    ppTInfo: *mut *mut core::ffi::c_void,
) -> HResult {
    if !ppTInfo.is_null() {
        *ppTInfo = core::ptr::null_mut();
    }
    S_OK
}

unsafe extern "system" fn wsh_GetIDsOfNames(
    _this: *mut core::ffi::c_void,
    _riid: *const Iid,
    rgszNames: *const *const u16,
    cNames: u32,
    _lcid: u32,
    rgDispId: *mut i32,
) -> HResult {
    if rgszNames.is_null() || rgDispId.is_null() || cNames == 0 {
        return E_POINTER;
    }

    let mut hr = S_OK;
    for i in 0..cNames as usize {
        let name_ptr = *rgszNames.add(i);
        let name = crate::types::utf16_ptr_to_string(name_ptr).to_ascii_lowercase();

        let dispid = match name.as_str() {
            "currentdirectory" => DISPID_WSH_CURRENTDIRECTORY,
            "expandenvironmentstrings" => DISPID_WSH_EXPANDENV,
            "regread" => DISPID_WSH_REGREAD,
            "regwrite" => DISPID_WSH_REGWRITE,
            "regdelete" => DISPID_WSH_REGDELETE,
            "run" => DISPID_WSH_RUN,
            "specialfolders" => DISPID_WSH_SPECIALFOLDERS,
            _ => {
                hr = DISP_E_UNKNOWNNAME;
                DISPID_UNKNOWN
            }
        };
        *rgDispId.add(i) = dispid;
    }

    hr
}

unsafe extern "system" fn wsh_Invoke(
    _this: *mut core::ffi::c_void,
    dispIdMember: i32,
    _riid: *const Iid,
    _lcid: u32,
    wFlags: u16,
    pDispParams: *mut core::ffi::c_void,
    pVarResult: *mut core::ffi::c_void,
    _pExcepInfo: *mut core::ffi::c_void,
    _puArgErr: *mut u32,
) -> HResult {
    let params = pDispParams as *mut DispParams;
    let res = pVarResult as *mut Variant;

    if !res.is_null() {
        *res = Variant::empty();
    }

    match dispIdMember {
        DISPID_WSH_CURRENTDIRECTORY => {
            if (wFlags & (crate::com::DISPATCH_PROPERTYPUT | crate::com::DISPATCH_PROPERTYPUTREF))
                != 0
            {
                // Property Put
                if !params.is_null() && (*params).cArgs > 0 {
                    let put_var = core::ptr::read_unaligned((*params).rgvarg);
                    if put_var.vt == VT_BSTR {
                        let bstr = core::ptr::read_unaligned(
                            put_var.data.as_ptr() as *const Bstr
                        );
                        let path = bstr_to_string(bstr);
                        let _ = std::env::set_current_dir(path);
                    }
                }
            } else {
                // Property Get
                let cur = std::env::current_dir()
                    .unwrap_or_default()
                    .display()
                    .to_string();
                if !res.is_null() {
                    let bstr = sys_alloc_string(&cur);
                    let r = &mut *res;
                    r.vt = VT_BSTR;
                    let ptr_bytes = (bstr as usize).to_ne_bytes();
                    r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
                }
            }
            S_OK
        }
        DISPID_WSH_EXPANDENV => {
            let str_val = extract_string_arg(params, 0).unwrap_or_default();
            let mut expanded = String::new();
            let mut chars = str_val.chars().peekable();

            while let Some(c) = chars.next() {
                if c == '%' {
                    let mut var_name = String::new();
                    let mut closed = false;
                    for inner in chars.by_ref() {
                        if inner == '%' {
                            closed = true;
                            break;
                        }
                        var_name.push(inner);
                    }
                    if closed && !var_name.is_empty() {
                        if let Ok(v) = std::env::var(&var_name) {
                            expanded.push_str(&v);
                        } else {
                            expanded.push('%');
                            expanded.push_str(&var_name);
                            expanded.push('%');
                        }
                    } else {
                        expanded.push('%');
                        expanded.push_str(&var_name);
                    }
                } else {
                    expanded.push(c);
                }
            }

            if !res.is_null() {
                let bstr = sys_alloc_string(&expanded);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_WSH_REGREAD => {
            let key = extract_string_arg(params, 0).unwrap_or_default();
            let val = crate::registry::reg_read(&key).unwrap_or_default();
            if !res.is_null() {
                let bstr = sys_alloc_string(&val);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_WSH_REGWRITE => {
            let key = extract_string_arg(params, 0).unwrap_or_default();
            let val = extract_string_arg(params, 1).unwrap_or_default();
            crate::registry::reg_write(&key, &val);
            S_OK
        }
        DISPID_WSH_REGDELETE => {
            let key = extract_string_arg(params, 0).unwrap_or_default();
            let _ = crate::registry::reg_delete(&key);
            S_OK
        }
        DISPID_WSH_RUN => {
            if !res.is_null() {
                let r = &mut *res;
                r.vt = VT_I4;
                let zero: i32 = 0;
                r.data[0..4].copy_from_slice(&zero.to_ne_bytes());
            }
            S_OK
        }
        DISPID_WSH_SPECIALFOLDERS => {
            let folder = extract_string_arg(params, 0).unwrap_or_default();
            let cur = std::env::current_dir()
                .unwrap_or_default()
                .display()
                .to_string();
            if !res.is_null() {
                let bstr = sys_alloc_string(&cur);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        _ => DISP_E_MEMBERNOTFOUND,
    }
}

static WSH_VTBL: IDispatchVtbl = IDispatchVtbl {
    QueryInterface: wsh_QueryInterface,
    AddRef: wsh_AddRef,
    Release: wsh_Release,
    GetTypeInfoCount: wsh_GetTypeInfoCount,
    GetTypeInfo: wsh_GetTypeInfo,
    GetIDsOfNames: wsh_GetIDsOfNames,
    Invoke: wsh_Invoke,
};

impl WshShellObject {
    pub fn new() -> *mut WshShellObject {
        let boxed = Box::new(WshShellObject {
            vtable: &WSH_VTBL,
            ref_count: AtomicU32::new(1),
        });
        Box::into_raw(boxed)
    }
}

// ----------------------------------------------------------------------------
// WshNetworkObject
// ----------------------------------------------------------------------------

const DISPID_NET_USERNAME: i32 = 5000;
const DISPID_NET_COMPUTERNAME: i32 = 5001;
const DISPID_NET_USERDOMAIN: i32 = 5002;

#[repr(C)]
pub struct WshNetworkObject {
    pub vtable: *const IDispatchVtbl,
    pub ref_count: AtomicU32,
}

unsafe extern "system" fn net_QueryInterface(
    this: *mut core::ffi::c_void,
    riid: *const Iid,
    ppv: *mut *mut core::ffi::c_void,
) -> HResult {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    let iid = *riid;
    if iid == IID_IUnknown || iid == IID_IDispatch {
        *ppv = this;
        net_AddRef(this);
        return S_OK;
    }
    *ppv = core::ptr::null_mut();
    E_NOINTERFACE
}

unsafe extern "system" fn net_AddRef(this: *mut core::ffi::c_void) -> u32 {
    let obj = &*(this as *mut WshNetworkObject);
    obj.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn net_Release(this: *mut core::ffi::c_void) -> u32 {
    let obj_ptr = this as *mut WshNetworkObject;
    let old_count = (*obj_ptr).ref_count.fetch_sub(1, Ordering::SeqCst);
    if old_count == 1 {
        let _ = Box::from_raw(obj_ptr);
        0
    } else {
        old_count - 1
    }
}

unsafe extern "system" fn net_GetTypeInfoCount(
    _this: *mut core::ffi::c_void,
    pctinfo: *mut u32,
) -> HResult {
    if !pctinfo.is_null() {
        *pctinfo = 1;
    }
    S_OK
}

unsafe extern "system" fn net_GetTypeInfo(
    _this: *mut core::ffi::c_void,
    _iTInfo: u32,
    _lcid: u32,
    ppTInfo: *mut *mut core::ffi::c_void,
) -> HResult {
    if !ppTInfo.is_null() {
        *ppTInfo = core::ptr::null_mut();
    }
    S_OK
}

unsafe extern "system" fn net_GetIDsOfNames(
    _this: *mut core::ffi::c_void,
    _riid: *const Iid,
    rgszNames: *const *const u16,
    cNames: u32,
    _lcid: u32,
    rgDispId: *mut i32,
) -> HResult {
    if rgszNames.is_null() || rgDispId.is_null() || cNames == 0 {
        return E_POINTER;
    }

    let mut hr = S_OK;
    for i in 0..cNames as usize {
        let name_ptr = *rgszNames.add(i);
        let name = crate::types::utf16_ptr_to_string(name_ptr).to_ascii_lowercase();

        let dispid = match name.as_str() {
            "username" => DISPID_NET_USERNAME,
            "computername" => DISPID_NET_COMPUTERNAME,
            "userdomain" => DISPID_NET_USERDOMAIN,
            _ => {
                hr = DISP_E_UNKNOWNNAME;
                DISPID_UNKNOWN
            }
        };
        *rgDispId.add(i) = dispid;
    }

    hr
}

unsafe extern "system" fn net_Invoke(
    _this: *mut core::ffi::c_void,
    dispIdMember: i32,
    _riid: *const Iid,
    _lcid: u32,
    _wFlags: u16,
    _pDispParams: *mut core::ffi::c_void,
    pVarResult: *mut core::ffi::c_void,
    _pExcepInfo: *mut core::ffi::c_void,
    _puArgErr: *mut u32,
) -> HResult {
    let res = pVarResult as *mut Variant;

    if !res.is_null() {
        *res = Variant::empty();
    }

    match dispIdMember {
        DISPID_NET_USERNAME => {
            let user = std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "ClassicBasicUser".to_string());
            if !res.is_null() {
                let bstr = sys_alloc_string(&user);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_NET_COMPUTERNAME => {
            let host = std::env::var("HOSTNAME")
                .or_else(|_| std::env::var("COMPUTERNAME"))
                .unwrap_or_else(|_| "ClassicBasicHost".to_string());
            if !res.is_null() {
                let bstr = sys_alloc_string(&host);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_NET_USERDOMAIN => {
            let domain = std::env::var("USERDOMAIN").unwrap_or_else(|_| "WORKGROUP".to_string());
            if !res.is_null() {
                let bstr = sys_alloc_string(&domain);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        _ => DISP_E_MEMBERNOTFOUND,
    }
}

static NET_VTBL: IDispatchVtbl = IDispatchVtbl {
    QueryInterface: net_QueryInterface,
    AddRef: net_AddRef,
    Release: net_Release,
    GetTypeInfoCount: net_GetTypeInfoCount,
    GetTypeInfo: net_GetTypeInfo,
    GetIDsOfNames: net_GetIDsOfNames,
    Invoke: net_Invoke,
};

impl WshNetworkObject {
    pub fn new() -> *mut WshNetworkObject {
        let boxed = Box::new(WshNetworkObject {
            vtable: &NET_VTBL,
            ref_count: AtomicU32::new(1),
        });
        Box::into_raw(boxed)
    }
}
