// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Standard ActiveX Component Shims.
//!
//! Provides clean-room COM automation implementations for common legacy components:
//! - `Scripting.FileSystemObject` & `TextStream` (Scripting Runtime)
//! - `WScript.Shell` & `WScript.Network` (Windows Script Host)
//! - `ADODB.Recordset`, `Fields`, and `Field` (ActiveX Data Objects)

pub mod adodb;
pub mod fso;
pub mod scriptcontrol;
pub mod wsh;

pub use adodb::*;
pub use fso::*;
pub use scriptcontrol::*;
pub use wsh::*;

use crate::com::IClassFactoryVtbl;
use crate::types::{
    CLASS_E_NOAGGREGATION, E_NOINTERFACE, E_POINTER, HResult, Iid, IID_IClassFactory, IID_IUnknown, S_OK,
};
use std::sync::atomic::{AtomicU32, Ordering};

pub struct GenericClassFactory {
    pub vtable: &'static IClassFactoryVtbl,
    pub ref_count: AtomicU32,
    pub create_instance_fn: fn() -> *mut core::ffi::c_void,
}

unsafe extern "system" fn gcf_QueryInterface(
    this: *mut core::ffi::c_void,
    riid: *const Iid,
    ppv: *mut *mut core::ffi::c_void,
) -> HResult {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    let iid = *riid;
    if iid == IID_IUnknown || iid == IID_IClassFactory {
        *ppv = this;
        gcf_AddRef(this);
        return S_OK;
    }
    *ppv = core::ptr::null_mut();
    E_NOINTERFACE
}

unsafe extern "system" fn gcf_AddRef(this: *mut core::ffi::c_void) -> u32 {
    let cf = &*(this as *mut GenericClassFactory);
    cf.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn gcf_Release(this: *mut core::ffi::c_void) -> u32 {
    let cf = &*(this as *mut GenericClassFactory);
    let old = cf.ref_count.fetch_sub(1, Ordering::SeqCst);
    if old > 0 {
        old - 1
    } else {
        0
    }
}

unsafe extern "system" fn gcf_CreateInstance(
    this: *mut core::ffi::c_void,
    p_unk_outer: *mut core::ffi::c_void,
    riid: *const Iid,
    ppv: *mut *mut core::ffi::c_void,
) -> HResult {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    *ppv = core::ptr::null_mut();
    if !p_unk_outer.is_null() {
        return CLASS_E_NOAGGREGATION;
    }

    let cf = &*(this as *mut GenericClassFactory);
    let obj = (cf.create_instance_fn)();

    // Call QueryInterface on created object
    let vtable = *(obj as *mut *mut usize);
    let qi_fn: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *const Iid,
        *mut *mut core::ffi::c_void,
    ) -> HResult = core::mem::transmute(*vtable);
    let hr = qi_fn(obj, riid, ppv);

    // Call Release on initial reference
    let rel_fn: unsafe extern "system" fn(*mut core::ffi::c_void) -> u32 =
        core::mem::transmute(*vtable.add(2));
    rel_fn(obj);

    hr
}

unsafe extern "system" fn gcf_LockServer(_this: *mut core::ffi::c_void, _f_lock: i32) -> HResult {
    S_OK
}

static GCF_VTBL: IClassFactoryVtbl = IClassFactoryVtbl {
    QueryInterface: gcf_QueryInterface,
    AddRef: gcf_AddRef,
    Release: gcf_Release,
    CreateInstance: gcf_CreateInstance,
    LockServer: gcf_LockServer,
};

fn create_fso() -> *mut core::ffi::c_void {
    FileSystemObject::new() as *mut core::ffi::c_void
}

fn create_wsh_shell() -> *mut core::ffi::c_void {
    WshShellObject::new() as *mut core::ffi::c_void
}

fn create_wsh_network() -> *mut core::ffi::c_void {
    WshNetworkObject::new() as *mut core::ffi::c_void
}

fn create_recordset() -> *mut core::ffi::c_void {
    RecordsetObject::new() as *mut core::ffi::c_void
}

fn create_scriptcontrol() -> *mut core::ffi::c_void {
    ScriptControlObject::new() as *mut core::ffi::c_void
}

static FSO_FACTORY: GenericClassFactory = GenericClassFactory {
    vtable: &GCF_VTBL,
    ref_count: AtomicU32::new(1),
    create_instance_fn: create_fso,
};

static WSH_SHELL_FACTORY: GenericClassFactory = GenericClassFactory {
    vtable: &GCF_VTBL,
    ref_count: AtomicU32::new(1),
    create_instance_fn: create_wsh_shell,
};

static WSH_NETWORK_FACTORY: GenericClassFactory = GenericClassFactory {
    vtable: &GCF_VTBL,
    ref_count: AtomicU32::new(1),
    create_instance_fn: create_wsh_network,
};

static RECORDSET_FACTORY: GenericClassFactory = GenericClassFactory {
    vtable: &GCF_VTBL,
    ref_count: AtomicU32::new(1),
    create_instance_fn: create_recordset,
};

static SCRIPTCONTROL_FACTORY: GenericClassFactory = GenericClassFactory {
    vtable: &GCF_VTBL,
    ref_count: AtomicU32::new(1),
    create_instance_fn: create_scriptcontrol,
};

pub unsafe fn get_fso_class_factory(riid: *const Iid, ppv: *mut *mut core::ffi::c_void) -> HResult {
    gcf_QueryInterface(&FSO_FACTORY as *const _ as *mut _, riid, ppv)
}

pub unsafe fn get_wsh_shell_class_factory(riid: *const Iid, ppv: *mut *mut core::ffi::c_void) -> HResult {
    gcf_QueryInterface(&WSH_SHELL_FACTORY as *const _ as *mut _, riid, ppv)
}

pub unsafe fn get_wsh_network_class_factory(riid: *const Iid, ppv: *mut *mut core::ffi::c_void) -> HResult {
    gcf_QueryInterface(&WSH_NETWORK_FACTORY as *const _ as *mut _, riid, ppv)
}

pub unsafe fn get_recordset_class_factory(riid: *const Iid, ppv: *mut *mut core::ffi::c_void) -> HResult {
    gcf_QueryInterface(&RECORDSET_FACTORY as *const _ as *mut _, riid, ppv)
}

pub unsafe fn get_scriptcontrol_class_factory(riid: *const Iid, ppv: *mut *mut core::ffi::c_void) -> HResult {
    gcf_QueryInterface(&SCRIPTCONTROL_FACTORY as *const _ as *mut _, riid, ppv)
}
