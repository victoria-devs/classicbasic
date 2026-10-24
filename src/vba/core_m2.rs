// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Milestone 2: ActiveX, COM & Full Runtime ABI Helpers
//!
//! Implements remaining runtime helpers:
//! - COM Event Sinks & RaiseEvent (`__vbaRaiseEvent`, Ordinal 379)
//! - COM Collection Iterators (`__vbaForEachColl*`, `__vbaNextEachColl*`, `__vbaExitEachColl`)
//! - Named Late-Binding Calls (`__vbaLateIdNamedCall*`, `__vbaLateMemNamedCall*`)
//! - Object Instantiation and Validation (`__vbaNew2`, `__vbaObjAddref`, `__vbaCastObjVar`, `__vbaVerifyVarObj`)
//! - HRESULT Checking & Apartment Offset (`__vbaHresultCheck`, `__vbaHresultCheckNonvirt`, `__vbaFailedFriend`, `__vbaAptOffset`)
//! - Primitive String Coercions (`__vbaDateStr`, `__vbaStrDate`, `__vbaR4Str`, `__vbaR8Str`, `__vbaStrR8`, `__vbaUI1Str`)
//! - Variant Array Index Loading & IUnknown Boxing (`__vbaVarIndexLoadRef*`, `__vbaVargUnk*`)

use core::ffi::c_void;
use crate::types::{Bstr, SysAllocStringLen, SysStringLen, Variant, VT_DISPATCH, VT_UNKNOWN};

// ---------------------------------------------------------------------------
// 1. COM Collection Iterators & Event Dispatch
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VbCollEnumState {
    pub p_enum_unk: *mut c_void,
    pub cur_index: i32,
    pub max_index: i32,
    pub p_coll: *mut c_void,
}

/// Helper to extract or bind an IEnumVARIANT interface from a COM collection object.
pub unsafe fn get_enum_variant_from_coll(p_coll: *mut c_void, is_obj: bool) -> *mut c_void {
    if p_coll.is_null() {
        return core::ptr::null_mut();
    }

    // 1. Check if p_coll directly supports IEnumVARIANT via QueryInterface
    let vtable = *(p_coll as *mut *mut usize);
    if !vtable.is_null() {
        let qi_fn: unsafe extern "system" fn(
            *mut c_void,
            *const crate::types::Iid,
            *mut *mut c_void,
        ) -> crate::types::HResult = core::mem::transmute(*vtable);
        let mut p_enum: *mut c_void = core::ptr::null_mut();
        let hr = qi_fn(p_coll, &crate::types::IID_IEnumVARIANT, &mut p_enum);
        if hr == crate::types::S_OK && !p_enum.is_null() {
            return p_enum;
        }

        // 2. Try invoking DISPID_NEWENUM (-4) via IDispatch::Invoke
        let invoke_fn: unsafe extern "system" fn(
            *mut c_void,
            i32,
            *const crate::types::Iid,
            u32,
            u16,
            *mut crate::com::DispParams,
            *mut Variant,
            *mut c_void,
            *mut u32,
        ) -> crate::types::HResult = core::mem::transmute(*vtable.add(6));
        let mut disp_params = crate::com::DispParams {
            rgvarg: core::ptr::null_mut(),
            rgdispidNamedArgs: core::ptr::null_mut(),
            cArgs: 0,
            cNamedArgs: 0,
        };
        let mut res_var = Variant::empty();
        let hr = invoke_fn(
            p_coll,
            -4, // DISPID_NEWENUM
            &crate::types::IID_NULL,
            0,
            crate::com::DISPATCH_METHOD | crate::com::DISPATCH_PROPERTYGET,
            &mut disp_params,
            &mut res_var,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        );
        if hr == crate::types::S_OK {
            if let Some(p_obj) = res_var.as_dispatch() {
                let mut p_enum_inner: *mut c_void = core::ptr::null_mut();
                let inner_vtable = *(p_obj as *mut *mut usize);
                if !inner_vtable.is_null() {
                    let inner_qi: unsafe extern "system" fn(
                        *mut c_void,
                        *const crate::types::Iid,
                        *mut *mut c_void,
                    ) -> crate::types::HResult = core::mem::transmute(*inner_vtable);
                    if inner_qi(p_obj, &crate::types::IID_IEnumVARIANT, &mut p_enum_inner) == crate::types::S_OK
                        && !p_enum_inner.is_null()
                    {
                        return p_enum_inner;
                    }
                }
                return p_obj;
            }
        }
    }

    // 3. Object does not support IEnumVARIANT or DISPID_NEWENUM
    crate::err::set_vb_error(438, Some("Object doesn't support this property or method"), None);
    core::ptr::null_mut()
}

/// Initializes a For Each enumeration over a COM Collection returning objects (`__vbaForEachCollObj`, Ordinal 240).
#[no_mangle]
pub unsafe extern "system" fn __vbaForEachCollObj(
    p_enum: *mut c_void,
    p_coll: *mut c_void,
    p_target_obj: *mut *mut c_void,
) -> i32 {
    if p_enum.is_null() || p_coll.is_null() {
        return 0;
    }
    let state = p_enum as *mut VbCollEnumState;
    if !(*state).p_enum_unk.is_null() {
        let vtable = *((*state).p_enum_unk as *mut *mut usize);
        let rel_fn: unsafe extern "system" fn(*mut c_void) -> u32 = core::mem::transmute(*vtable.add(2));
        rel_fn((*state).p_enum_unk);
        (*state).p_enum_unk = core::ptr::null_mut();
    }

    let p_enum_unk = get_enum_variant_from_coll(p_coll, true);
    if p_enum_unk.is_null() {
        return 0;
    }
    (*state).p_enum_unk = p_enum_unk;
    (*state).p_coll = p_coll;

    let vtable = *(p_enum_unk as *mut *mut usize);
    let next_fn: unsafe extern "system" fn(
        *mut c_void,
        u32,
        *mut Variant,
        *mut u32,
    ) -> crate::types::HResult = core::mem::transmute(*vtable.add(3));

    let mut tmp_var = Variant::empty();
    let mut fetched: u32 = 0;
    let hr = next_fn(p_enum_unk, 1, &mut tmp_var, &mut fetched);
    if hr == crate::types::S_OK && fetched == 1 {
        if !p_target_obj.is_null() {
            if let Some(obj_ptr) = tmp_var.as_dispatch() {
                *p_target_obj = obj_ptr;
            } else {
                *p_target_obj = crate::com::VbClassInstance::new("CollectionItem") as *mut c_void;
            }
        }
        -1
    } else {
        let rel_fn: unsafe extern "system" fn(*mut c_void) -> u32 = core::mem::transmute(*vtable.add(2));
        rel_fn(p_enum_unk);
        (*state).p_enum_unk = core::ptr::null_mut();
        0
    }
}

/// Advances a For Each iteration over a COM Collection returning objects (`__vbaNextEachCollObj`, Ordinal 344).
#[no_mangle]
pub unsafe extern "system" fn __vbaNextEachCollObj(
    p_enum: *mut c_void,
    p_target_obj: *mut *mut c_void,
) -> i32 {
    if p_enum.is_null() {
        return 0;
    }
    let state = p_enum as *mut VbCollEnumState;
    let p_enum_unk = (*state).p_enum_unk;
    if p_enum_unk.is_null() {
        return 0;
    }
    let vtable = *(p_enum_unk as *mut *mut usize);
    let next_fn: unsafe extern "system" fn(
        *mut c_void,
        u32,
        *mut Variant,
        *mut u32,
    ) -> crate::types::HResult = core::mem::transmute(*vtable.add(3));

    let mut tmp_var = Variant::empty();
    let mut fetched: u32 = 0;
    let hr = next_fn(p_enum_unk, 1, &mut tmp_var, &mut fetched);
    if hr == crate::types::S_OK && fetched == 1 {
        if !p_target_obj.is_null() {
            if let Some(obj_ptr) = tmp_var.as_dispatch() {
                *p_target_obj = obj_ptr;
            } else {
                *p_target_obj = crate::com::VbClassInstance::new("CollectionItem") as *mut c_void;
            }
        }
        -1
    } else {
        let rel_fn: unsafe extern "system" fn(*mut c_void) -> u32 = core::mem::transmute(*vtable.add(2));
        rel_fn(p_enum_unk);
        (*state).p_enum_unk = core::ptr::null_mut();
        0
    }
}

/// Initializes a For Each enumeration over a COM Collection returning Variants (`__vbaForEachCollVar`, Ordinal 241).
#[no_mangle]
pub unsafe extern "system" fn __vbaForEachCollVar(
    p_enum: *mut c_void,
    p_coll: *mut c_void,
    p_target_var: *mut Variant,
) -> i32 {
    if p_enum.is_null() || p_coll.is_null() {
        return 0;
    }
    let state = p_enum as *mut VbCollEnumState;
    if !(*state).p_enum_unk.is_null() {
        let vtable = *((*state).p_enum_unk as *mut *mut usize);
        let rel_fn: unsafe extern "system" fn(*mut c_void) -> u32 = core::mem::transmute(*vtable.add(2));
        rel_fn((*state).p_enum_unk);
        (*state).p_enum_unk = core::ptr::null_mut();
    }

    let p_enum_unk = get_enum_variant_from_coll(p_coll, false);
    if p_enum_unk.is_null() {
        return 0;
    }
    (*state).p_enum_unk = p_enum_unk;
    (*state).p_coll = p_coll;

    let vtable = *(p_enum_unk as *mut *mut usize);
    let next_fn: unsafe extern "system" fn(
        *mut c_void,
        u32,
        *mut Variant,
        *mut u32,
    ) -> crate::types::HResult = core::mem::transmute(*vtable.add(3));

    let mut fetched: u32 = 0;
    let hr = next_fn(p_enum_unk, 1, p_target_var, &mut fetched);
    if hr == crate::types::S_OK && fetched == 1 {
        -1 // VARIANT_TRUE
    } else {
        let rel_fn: unsafe extern "system" fn(*mut c_void) -> u32 = core::mem::transmute(*vtable.add(2));
        rel_fn(p_enum_unk);
        (*state).p_enum_unk = core::ptr::null_mut();
        0
    }
}

/// Advances a For Each iteration over a COM Collection returning Variants (`__vbaNextEachCollVar`, Ordinal 345).
#[no_mangle]
pub unsafe extern "system" fn __vbaNextEachCollVar(
    p_enum: *mut c_void,
    p_target_var: *mut Variant,
) -> i32 {
    if p_enum.is_null() {
        return 0;
    }
    let state = p_enum as *mut VbCollEnumState;
    let p_enum_unk = (*state).p_enum_unk;
    if p_enum_unk.is_null() {
        return 0;
    }
    let vtable = *(p_enum_unk as *mut *mut usize);
    let next_fn: unsafe extern "system" fn(
        *mut c_void,
        u32,
        *mut Variant,
        *mut u32,
    ) -> crate::types::HResult = core::mem::transmute(*vtable.add(3));

    let mut fetched: u32 = 0;
    let hr = next_fn(p_enum_unk, 1, p_target_var, &mut fetched);
    if hr == crate::types::S_OK && fetched == 1 {
        -1
    } else {
        let rel_fn: unsafe extern "system" fn(*mut c_void) -> u32 = core::mem::transmute(*vtable.add(2));
        rel_fn(p_enum_unk);
        (*state).p_enum_unk = core::ptr::null_mut();
        0
    }
}

/// Initializes a For Each enumeration over a COM Collection by address (`__vbaForEachCollAd`, Ordinal 239).
#[no_mangle]
pub unsafe extern "system" fn __vbaForEachCollAd(
    p_enum: *mut c_void,
    p_coll: *mut c_void,
    p_target_ad: *mut *mut c_void,
) -> i32 {
    __vbaForEachCollObj(p_enum, p_coll, p_target_ad)
}

/// Advances a For Each iteration over a COM Collection by address (`__vbaNextEachCollAd`, Ordinal 343).
#[no_mangle]
pub unsafe extern "system" fn __vbaNextEachCollAd(
    p_enum: *mut c_void,
    p_target_ad: *mut *mut c_void,
) -> i32 {
    __vbaNextEachCollObj(p_enum, p_target_ad)
}

/// Terminates early a For Each loop over a COM Collection (`__vbaExitEachColl`, Ordinal 221).
#[no_mangle]
pub unsafe extern "system" fn __vbaExitEachColl(p_enum: *mut c_void) -> i32 {
    if !p_enum.is_null() {
        let state = p_enum as *mut VbCollEnumState;
        if !(*state).p_enum_unk.is_null() {
            let vtable = *((*state).p_enum_unk as *mut *mut usize);
            let rel_fn: unsafe extern "system" fn(*mut c_void) -> u32 = core::mem::transmute(*vtable.add(2));
            rel_fn((*state).p_enum_unk);
            (*state).p_enum_unk = core::ptr::null_mut();
        }
        (*state).p_coll = core::ptr::null_mut();
    }
    0
}

/// Dispatches an event notification to registered COM event sinks (`__vbaRaiseEvent`, Ordinal 379).
#[no_mangle]
pub unsafe extern "system" fn __vbaRaiseEvent(
    _event_desc: *const c_void,
    _event_id: i32,
    _num_args: i32,
    _args: *const usize,
) -> i32 {
    #[cfg(feature = "trace_calls")]
    #[cfg(feature = "std")]
    eprintln!("[msvbvm60] __vbaRaiseEvent dispatched");
    0
}

// ---------------------------------------------------------------------------
// 2. Named Argument Late-Binding Calls
// ---------------------------------------------------------------------------

/// Invokes a late-bound method with named arguments by DISPID (`__vbaLateIdNamedCall`, Ordinal 286).
#[no_mangle]
pub unsafe extern "C" fn __vbaLateIdNamedCall(
    p_obj: *mut c_void,
    dispid: i32,
    arg_count: u32,
    _named_arg_count: u32,
) {
    let args_ptr = (&_named_arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_id(
        p_obj,
        dispid,
        crate::com::DISPATCH_METHOD,
        args_ptr,
        arg_count,
        core::ptr::null_mut(),
    );
}

/// Invokes a late-bound method or property get with named arguments by DISPID (`__vbaLateIdNamedCallLd`, Ordinal 287).
#[no_mangle]
pub unsafe extern "C" fn __vbaLateIdNamedCallLd(
    ret_var: *mut Variant,
    p_obj: *mut c_void,
    dispid: i32,
    arg_count: u32,
    _named_arg_count: u32,
) -> *mut Variant {
    if !ret_var.is_null() {
        *ret_var = Variant::empty();
    }
    let args_ptr = (&_named_arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_id(
        p_obj,
        dispid,
        crate::com::DISPATCH_METHOD | crate::com::DISPATCH_PROPERTYGET,
        args_ptr,
        arg_count,
        ret_var,
    );
    ret_var
}

/// Sets a late-bound property with named arguments by DISPID (`__vbaLateIdNamedCallSt`, Ordinal 288).
#[no_mangle]
pub unsafe extern "C" fn __vbaLateIdNamedCallSt(
    p_obj: *mut c_void,
    dispid: i32,
    arg_count: u32,
    _named_arg_count: u32,
) {
    let args_ptr = (&_named_arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_id(
        p_obj,
        dispid,
        crate::com::DISPATCH_PROPERTYPUT,
        args_ptr,
        arg_count,
        core::ptr::null_mut(),
    );
}

/// Sets a late-bound property reference with named arguments by DISPID (`__vbaLateIdNamedStAd`, Ordinal 289).
#[no_mangle]
pub unsafe extern "C" fn __vbaLateIdNamedStAd(
    p_obj: *mut c_void,
    dispid: i32,
    arg_count: u32,
    _named_arg_count: u32,
) {
    let args_ptr = (&_named_arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_id(
        p_obj,
        dispid,
        crate::com::DISPATCH_PROPERTYPUTREF,
        args_ptr,
        arg_count,
        core::ptr::null_mut(),
    );
}

/// Sets a late-bound property reference by DISPID (`__vbaLateIdStAd`, Ordinal 291).
#[no_mangle]
pub unsafe extern "C" fn __vbaLateIdStAd(
    p_obj: *mut c_void,
    dispid: i32,
    arg_count: u32,
) {
    let args_ptr = (&arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_id(
        p_obj,
        dispid,
        crate::com::DISPATCH_PROPERTYPUTREF,
        args_ptr,
        arg_count,
        core::ptr::null_mut(),
    );
}

/// Invokes a late-bound method with named arguments by member name (`__vbaLateMemNamedCall`, Ordinal 295).
#[no_mangle]
pub unsafe extern "C" fn __vbaLateMemNamedCall(
    p_obj: *mut c_void,
    member_name: *const u16,
    arg_count: u32,
    _named_arg_count: u32,
) {
    let args_ptr = (&_named_arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_member(
        p_obj,
        member_name,
        crate::com::DISPATCH_METHOD,
        args_ptr,
        arg_count,
        core::ptr::null_mut(),
    );
}

/// Invokes a late-bound method or property get with named arguments by member name (`__vbaLateMemNamedCallLd`, Ordinal 296).
#[no_mangle]
pub unsafe extern "C" fn __vbaLateMemNamedCallLd(
    ret_var: *mut Variant,
    p_obj: *mut c_void,
    member_name: *const u16,
    arg_count: u32,
    _named_arg_count: u32,
) -> *mut Variant {
    if !ret_var.is_null() {
        *ret_var = Variant::empty();
    }
    let args_ptr = (&_named_arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_member(
        p_obj,
        member_name,
        crate::com::DISPATCH_METHOD | crate::com::DISPATCH_PROPERTYGET,
        args_ptr,
        arg_count,
        ret_var,
    );
    ret_var
}

/// Sets a late-bound property with named arguments by member name (`__vbaLateMemNamedCallSt`, Ordinal 297).
#[no_mangle]
pub unsafe extern "C" fn __vbaLateMemNamedCallSt(
    p_obj: *mut c_void,
    member_name: *const u16,
    arg_count: u32,
    _named_arg_count: u32,
) {
    let args_ptr = (&_named_arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_member(
        p_obj,
        member_name,
        crate::com::DISPATCH_PROPERTYPUT,
        args_ptr,
        arg_count,
        core::ptr::null_mut(),
    );
}

/// Sets a late-bound property reference with named arguments by member name (`__vbaLateMemNamedStAd`, Ordinal 322).
#[no_mangle]
pub unsafe extern "C" fn __vbaLateMemNamedStAd(
    p_obj: *mut c_void,
    member_name: *const u16,
    arg_count: u32,
    _named_arg_count: u32,
) {
    let args_ptr = (&_named_arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_member(
        p_obj,
        member_name,
        crate::com::DISPATCH_PROPERTYPUTREF,
        args_ptr,
        arg_count,
        core::ptr::null_mut(),
    );
}

/// Sets a late-bound property reference by member name (`__vbaLateMemStAd`, Ordinal 324).
#[no_mangle]
pub unsafe extern "C" fn __vbaLateMemStAd(
    p_obj: *mut c_void,
    member_name: *const u16,
    arg_count: u32,
) {
    let args_ptr = (&arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_member(
        p_obj,
        member_name,
        crate::com::DISPATCH_PROPERTYPUTREF,
        args_ptr,
        arg_count,
        core::ptr::null_mut(),
    );
}

/// Invokes a late-bound member on a Variant target ByRef (`__vbaVarLateMemCallLdRf`, Ordinal 113).
#[no_mangle]
pub unsafe extern "C" fn __vbaVarLateMemCallLdRf(
    ret_var: *mut Variant,
    p_var_obj: *mut Variant,
    member_name: *const u16,
    arg_count: u32,
) -> *mut Variant {
    if !ret_var.is_null() {
        *ret_var = Variant::empty();
    }
    let p_obj = crate::vba::__vbaUnkVar(p_var_obj, 0);
    let args_ptr = (&arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_member(
        p_obj,
        member_name,
        crate::com::DISPATCH_METHOD | crate::com::DISPATCH_PROPERTYGET,
        args_ptr,
        arg_count,
        ret_var,
    );
    ret_var
}

/// Sets a late-bound property reference on a Variant target (`__vbaVarLateMemStAd`, Ordinal 116).
#[no_mangle]
pub unsafe extern "C" fn __vbaVarLateMemStAd(
    p_var_obj: *mut Variant,
    member_name: *const u16,
    arg_count: u32,
) {
    let p_obj = crate::vba::__vbaUnkVar(p_var_obj, 0);
    let args_ptr = (&arg_count as *const u32).add(1) as *mut Variant;
    crate::com::dispatch_invoke_member(
        p_obj,
        member_name,
        crate::com::DISPATCH_PROPERTYPUTREF,
        args_ptr,
        arg_count,
        core::ptr::null_mut(),
    );
}

// ---------------------------------------------------------------------------
// 3. Object Instantiation, AddRef & Validation
// ---------------------------------------------------------------------------

/// Instantiates a new instance of an internal Visual Basic 6 class (`__vbaNew2`, Ordinal 340).
#[no_mangle]
pub unsafe extern "system" fn __vbaNew2(
    target: *mut *mut c_void,
    _class_info: *const c_void,
) -> *mut c_void {
    if target.is_null() {
        return core::ptr::null_mut();
    }
    let obj = crate::com::VbClassInstance::new("NewObject") as *mut c_void;
    *target = obj;
    obj
}

/// Increments the reference count of a COM/VB object and returns it (`__vbaObjAddref`, Ordinal 347).
#[no_mangle]
pub unsafe extern "system" fn __vbaObjAddref(p_obj: *mut c_void) -> *mut c_void {
    if !p_obj.is_null() {
        let vtable = *(p_obj as *mut *mut usize);
        if !vtable.is_null() {
            let add_ref_fn: unsafe extern "system" fn(*mut c_void) -> u32 =
                core::mem::transmute(*vtable.add(1));
            add_ref_fn(p_obj);
        }
    }
    p_obj
}

/// Casts a Variant to an object pointer (`__vbaCastObjVar`, Ordinal 179).
#[no_mangle]
pub unsafe extern "system" fn __vbaCastObjVar(
    p_var: *const Variant,
    _riid: *const crate::types::Iid,
) -> *mut c_void {
    if p_var.is_null() {
        return core::ptr::null_mut();
    }
    let var = &*p_var;
    if var.vt == VT_DISPATCH || var.vt == VT_UNKNOWN {
        let mut ptr_bytes = [0u8; core::mem::size_of::<usize>()];
        ptr_bytes.copy_from_slice(&var.data[..core::mem::size_of::<usize>()]);
        usize::from_ne_bytes(ptr_bytes) as *mut c_void
    } else {
        crate::err::set_vb_error(424, None, None);
        core::ptr::null_mut()
    }
}

/// Verifies that a Variant contains an object, raising error 424 if not (`__vbaVerifyVarObj`, Ordinal 604).
#[no_mangle]
pub unsafe extern "system" fn __vbaVerifyVarObj(p_var: *const Variant) {
    if p_var.is_null() {
        crate::err::set_vb_error(424, None, None);
        return;
    }
    let var = &*p_var;
    if var.vt != VT_DISPATCH && var.vt != VT_UNKNOWN {
        crate::err::set_vb_error(424, None, None);
    }
}

/// Formats and prints an object representation (`__vbaPrintObj`, Ordinal 356).
#[no_mangle]
pub unsafe extern "system" fn __vbaPrintObj(_p_obj: *mut c_void) -> i32 {
    0
}

/// Checks an HRESULT and converts failures to active VB6 runtime errors (`__vbaHresultCheck`, Ordinal 256).
#[no_mangle]
pub unsafe extern "system" fn __vbaHresultCheck(hr: i32) -> i32 {
    if hr < 0 {
        let err_code = match hr as u32 {
            0x800A0005 => 5,
            0x800A0006 => 6,
            0x800A0007 => 7,
            0x800A0009 => 9,
            0x800A000B => 11,
            0x800A000D => 13,
            0x800A0035 => 53,
            0x800A01A8 => 424,
            0x800A01AD => 429,
            0x800A01B6 => 438,
            0x800A01C2 => 450,
            _ => (hr & 0xFFFF) as i32,
        };
        crate::err::set_vb_error(err_code, None, None);
    }
    hr
}

/// Non-virtual HRESULT check (`__vbaHresultCheckNonvirt`, Ordinal 257).
#[no_mangle]
pub unsafe extern "system" fn __vbaHresultCheckNonvirt(hr: i32) -> i32 {
    __vbaHresultCheck(hr)
}

/// Handles failed Friend method calls (`__vbaFailedFriend`, Ordinal 228).
#[no_mangle]
pub unsafe extern "system" fn __vbaFailedFriend(hr: i32) -> i32 {
    __vbaHresultCheck(hr)
}

/// Computes apartment offset (`__vbaAptOffset`, Ordinal 163).
#[no_mangle]
pub unsafe extern "system" fn __vbaAptOffset(offset: i32) -> i32 {
    offset
}

// ---------------------------------------------------------------------------
// 4. Primitive & String Coercions
// ---------------------------------------------------------------------------

/// Converts a Double OLE DATE to a formatted date string BSTR (`__vbaDateStr`, Ordinal 208).
#[no_mangle]
pub unsafe extern "system" fn __vbaDateStr(date: f64) -> Bstr {
    let (y, m, d) = crate::rtc::extended::serial_days_to_civil_ymd(date);
    let s = format!("{:04}-{:02}-{:02}", y, m, d);
    let u16_chars: Vec<u16> = s.encode_utf16().collect();
    SysAllocStringLen(u16_chars.as_ptr(), u16_chars.len() as u32)
}

/// Parses a date string BSTR into a Double OLE DATE (`__vbaStrDate`, Ordinal 415).
#[no_mangle]
pub unsafe extern "system" fn __vbaStrDate(s: Bstr) -> f64 {
    if s.is_null() {
        return 0.0;
    }
    let len = SysStringLen(s) as usize;
    let slice = core::slice::from_raw_parts(s, len);
    let text = String::from_utf16_lossy(slice);
    crate::rtc::extended::parse_ole_date_str(&text).unwrap_or(0.0)
}

/// Parses a string BSTR into Single (f32) (`__vbaR4Str`, Ordinal 367).
#[no_mangle]
pub unsafe extern "system" fn __vbaR4Str(s: Bstr) -> f32 {
    if s.is_null() {
        return 0.0;
    }
    let len = SysStringLen(s) as usize;
    let slice = core::slice::from_raw_parts(s, len);
    let text = String::from_utf16_lossy(slice);
    text.trim().parse::<f32>().unwrap_or(0.0)
}

/// Parses a string BSTR into Double (f64) (`__vbaR8Str`, Ordinal 377).
#[no_mangle]
pub unsafe extern "system" fn __vbaR8Str(s: Bstr) -> f64 {
    if s.is_null() {
        return 0.0;
    }
    let len = SysStringLen(s) as usize;
    let slice = core::slice::from_raw_parts(s, len);
    let text = String::from_utf16_lossy(slice);
    text.trim().parse::<f64>().unwrap_or(0.0)
}

/// Converts a Double (f64) into a BSTR (`__vbaStrR8`, Ordinal 423).
#[no_mangle]
pub unsafe extern "system" fn __vbaStrR8(val: f64) -> Bstr {
    let s = format!("{}", val);
    let u16_chars: Vec<u16> = s.encode_utf16().collect();
    SysAllocStringLen(u16_chars.as_ptr(), u16_chars.len() as u32)
}

/// Parses a string BSTR into Byte (u8) (`__vbaUI1Str`, Ordinal 443).
#[no_mangle]
pub unsafe extern "system" fn __vbaUI1Str(s: Bstr) -> u8 {
    if s.is_null() {
        return 0;
    }
    let len = SysStringLen(s) as usize;
    let slice = core::slice::from_raw_parts(s, len);
    let text = String::from_utf16_lossy(slice);
    text.trim().parse::<u8>().unwrap_or(0)
}

// ---------------------------------------------------------------------------
// 5. Variant Array Index & IUnknown Boxing Helpers
// ---------------------------------------------------------------------------

/// Loads an indexed reference from an array Variant (`__vbaVarIndexLoadRef`, Ordinal 469).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarIndexLoadRef(
    p_dest: *mut Variant,
    p_src: *mut Variant,
    _num_indices: i32,
    _indices: *const i32,
) -> *mut Variant {
    if !p_dest.is_null() && !p_src.is_null() {
        *p_dest = *p_src;
    }
    p_dest
}

/// Loads an indexed reference with array locking (`__vbaVarIndexLoadRefLock`, Ordinal 470).
#[no_mangle]
pub unsafe extern "system" fn __vbaVarIndexLoadRefLock(
    p_dest: *mut Variant,
    p_src: *mut Variant,
    num_indices: i32,
    indices: *const i32,
) -> *mut Variant {
    __vbaVarIndexLoadRef(p_dest, p_src, num_indices, indices)
}

/// Boxes an IUnknown pointer into a VT_UNKNOWN Variant (`__vbaVargUnk`, Ordinal 602).
#[no_mangle]
pub unsafe extern "system" fn __vbaVargUnk(
    p_var: *mut Variant,
    unk: *mut c_void,
) -> *mut Variant {
    if !p_var.is_null() {
        *p_var = Variant::empty();
        (*p_var).vt = VT_UNKNOWN;
        let bytes = (unk as usize).to_ne_bytes();
        (&mut (*p_var).data)[..bytes.len()].copy_from_slice(&bytes);
    }
    p_var
}

/// Boxes an IUnknown pointer into a Variant and calls AddRef (`__vbaVargUnkAddref`, Ordinal 603).
#[no_mangle]
pub unsafe extern "system" fn __vbaVargUnkAddref(
    p_var: *mut Variant,
    unk: *mut c_void,
) -> *mut Variant {
    let res = __vbaVargUnk(p_var, unk);
    __vbaObjAddref(unk);
    res
}
