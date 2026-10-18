// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # COM Event Sinks, Connection Points, Collections & IEnumVARIANT Dispatch
//!
//! Validates:
//! - Collection enumeration via genuine `IEnumVARIANT` dispatch (`_NewEnum` DISPID -4)
//! - `VbCollectionObject` COM dispatch: `Add`, `Item`, `Count`, `Remove`, `_NewEnum`
//! - `IEnumVARIANT` interface methods: `Next`, `Skip`, `Reset`, `Clone`
//! - Runtime helpers: `__vbaForEachCollVar`, `__vbaNextEachCollVar`, `__vbaForEachCollObj`, `__vbaNextEachCollObj`, `__vbaExitEachColl`
//! - Event sinks and connection points: `__vbaRaiseEvent`, `__vbaHresultCheck`, `__vbaFailedFriend`
//! - Object creation and indexing helpers: `__vbaNew2`, `__vbaObjAddref`, `__vbaVarIndexLoadRef`

use core::ffi::c_void;
use msvbvm60::com::*;
use msvbvm60::types::*;
use msvbvm60::vba::core_m2::*;

#[test]
fn test_collection_iterators_obj_and_var() {
    unsafe {
        let p_sink = msvbvm60::com::VbClassInstance::new("EventSink") as *mut core::ffi::c_void;
        assert!(!p_sink.is_null());

        let mut enum_state = VbCollEnumState {
            p_enum_unk: core::ptr::null_mut(),
            cur_index: 0,
            max_index: 3,
            p_coll: p_sink,
        };

        let mut target_obj: *mut core::ffi::c_void = core::ptr::null_mut();
        let res = __vbaForEachCollObj(&mut enum_state as *mut _ as *mut _, p_sink, &mut target_obj);
        assert_eq!(res, -1);
        assert!(!target_obj.is_null());

        let res2 = __vbaNextEachCollObj(&mut enum_state as *mut _ as *mut _, &mut target_obj);
        assert_eq!(res2, -1);

        let res3 = __vbaNextEachCollObj(&mut enum_state as *mut _ as *mut _, &mut target_obj);
        assert_eq!(res3, -1);

        let res4 = __vbaNextEachCollObj(&mut enum_state as *mut _ as *mut _, &mut target_obj);
        assert_eq!(res4, -1);

        let res5 = __vbaNextEachCollObj(&mut enum_state as *mut _ as *mut _, &mut target_obj);
        assert_eq!(res5, -1);

        let res_end = __vbaNextEachCollObj(&mut enum_state as *mut _ as *mut _, &mut target_obj);
        assert_eq!(res_end, 0);

        // Address iterators
        let mut target_ad: *mut core::ffi::c_void = core::ptr::null_mut();
        enum_state.cur_index = 0;
        let res_ad = __vbaForEachCollAd(&mut enum_state as *mut _ as *mut _, p_sink, &mut target_ad);
        assert_eq!(res_ad, -1);
        let res_ad2 = __vbaNextEachCollAd(&mut enum_state as *mut _ as *mut _, &mut target_ad);
        assert_eq!(res_ad2, -1);

        let res_exit = __vbaExitEachColl(&mut enum_state as *mut _ as *mut _);
        assert_eq!(res_exit, 0);

        // Variant iterator
        let mut enum_state_var = VbCollEnumState {
            p_enum_unk: core::ptr::null_mut(),
            cur_index: 0,
            max_index: 4,
            p_coll: p_sink,
        };
        let mut target_var = Variant::empty();
        let res_v1 = __vbaForEachCollVar(&mut enum_state_var as *mut _ as *mut _, p_sink, &mut target_var);
        assert_eq!(res_v1, -1);
        assert_eq!(target_var.as_i32(), Some(100));

        let res_v2 = __vbaNextEachCollVar(&mut enum_state_var as *mut _ as *mut _, &mut target_var);
        assert_eq!(res_v2, -1);
        assert_eq!(target_var.as_i32(), Some(250));

        let res_v3 = __vbaNextEachCollVar(&mut enum_state_var as *mut _ as *mut _, &mut target_var);
        assert_eq!(res_v3, -1);
        assert_eq!(target_var.as_i32(), Some(50));

        let res_v4 = __vbaNextEachCollVar(&mut enum_state_var as *mut _ as *mut _, &mut target_var);
        assert_eq!(res_v4, -1);
        let b = target_var.as_bstr().unwrap();
        assert_eq!(bstr_to_string(b), "EXTRA");

        let res_v_end = __vbaNextEachCollVar(&mut enum_state_var as *mut _ as *mut _, &mut target_var);
        assert_eq!(res_v_end, 0);
    }
}

#[test]
fn test_genuine_ienumvariant_dispatch_on_vb_collection() {
    unsafe {
        // Create a VbCollectionObject directly
        let coll = VbCollectionObject::new();
        let p_coll = coll as *mut c_void;

        // Add 3 items via IDispatch: Add(Item, [Key])
        // In VB6 DispParams, args are in reverse order: [Key, Item] or just [Item]
        let vtable = *(p_coll as *mut *mut usize);
        let invoke_fn: unsafe extern "system" fn(
            *mut c_void,
            i32,
            *const Iid,
            u32,
            u16,
            *mut DispParams,
            *mut Variant,
            *mut c_void,
            *mut u32,
        ) -> HResult = core::mem::transmute(*vtable.add(6));

        // Add "Alpha"
        let s_alpha = sys_alloc_string("Alpha");
        let mut v_arg1 = Variant::from_bstr(s_alpha);
        let mut dp1 = DispParams {
            rgvarg: &mut v_arg1,
            rgdispidNamedArgs: core::ptr::null_mut(),
            cArgs: 1,
            cNamedArgs: 0,
        };
        let mut res = Variant::empty();
        let hr1 = invoke_fn(p_coll, 1, &IID_NULL, 0, DISPATCH_METHOD, &mut dp1, &mut res, core::ptr::null_mut(), core::ptr::null_mut());
        assert_eq!(hr1, S_OK);

        // Add 42
        let mut v_arg2 = Variant::from_i32(42);
        let mut dp2 = DispParams {
            rgvarg: &mut v_arg2,
            rgdispidNamedArgs: core::ptr::null_mut(),
            cArgs: 1,
            cNamedArgs: 0,
        };
        let hr2 = invoke_fn(p_coll, 1, &IID_NULL, 0, DISPATCH_METHOD, &mut dp2, &mut res, core::ptr::null_mut(), core::ptr::null_mut());
        assert_eq!(hr2, S_OK);

        // Add 3.14
        let mut v_arg3 = Variant::from_f64(3.14);
        let mut dp3 = DispParams {
            rgvarg: &mut v_arg3,
            rgdispidNamedArgs: core::ptr::null_mut(),
            cArgs: 1,
            cNamedArgs: 0,
        };
        let hr3 = invoke_fn(p_coll, 1, &IID_NULL, 0, DISPATCH_METHOD, &mut dp3, &mut res, core::ptr::null_mut(), core::ptr::null_mut());
        assert_eq!(hr3, S_OK);

        // Verify Count via IDispatch (DISPID 2)
        let mut dp_empty = DispParams {
            rgvarg: core::ptr::null_mut(),
            rgdispidNamedArgs: core::ptr::null_mut(),
            cArgs: 0,
            cNamedArgs: 0,
        };
        let hr_cnt = invoke_fn(p_coll, 2, &IID_NULL, 0, DISPATCH_PROPERTYGET, &mut dp_empty, &mut res, core::ptr::null_mut(), core::ptr::null_mut());
        assert_eq!(hr_cnt, S_OK);
        assert_eq!(res.as_i32(), Some(3));

        // Now iterate using __vbaForEachCollVar / __vbaNextEachCollVar
        let mut enum_state = VbCollEnumState {
            p_enum_unk: core::ptr::null_mut(),
            cur_index: 0,
            max_index: 0,
            p_coll: core::ptr::null_mut(),
        };

        let mut item = Variant::empty();
        let loop_cont = __vbaForEachCollVar(&mut enum_state as *mut _ as *mut _, p_coll, &mut item);
        assert_eq!(loop_cont, -1);
        assert_eq!(bstr_to_string(item.as_bstr().unwrap()), "Alpha");

        let loop_cont2 = __vbaNextEachCollVar(&mut enum_state as *mut _ as *mut _, &mut item);
        assert_eq!(loop_cont2, -1);
        assert_eq!(item.as_i32(), Some(42));

        let loop_cont3 = __vbaNextEachCollVar(&mut enum_state as *mut _ as *mut _, &mut item);
        assert_eq!(loop_cont3, -1);
        assert!((item.as_f64().unwrap() - 3.14).abs() < 1e-6);

        let loop_end = __vbaNextEachCollVar(&mut enum_state as *mut _ as *mut _, &mut item);
        assert_eq!(loop_end, 0);

        // Clean up
        SysFreeString(s_alpha);
        let rel_fn: unsafe extern "system" fn(*mut c_void) -> u32 = core::mem::transmute(*vtable.add(2));
        rel_fn(p_coll);
    }
}

#[test]
fn test_ienumvariant_methods_next_skip_reset_clone() {
    unsafe {
        let items = vec![
            Variant::from_i32(10),
            Variant::from_i32(20),
            Variant::from_i32(30),
            Variant::from_i32(40),
        ];
        let p_enum = EnumVariantObject::new(items) as *mut c_void;
        let vtable = *(p_enum as *mut *mut usize);

        let qi_fn: unsafe extern "system" fn(*mut c_void, *const Iid, *mut *mut c_void) -> HResult =
            core::mem::transmute(*vtable.add(0));
        let rel_fn: unsafe extern "system" fn(*mut c_void) -> u32 =
            core::mem::transmute(*vtable.add(2));
        let next_fn: unsafe extern "system" fn(*mut c_void, u32, *mut Variant, *mut u32) -> HResult =
            core::mem::transmute(*vtable.add(3));
        let skip_fn: unsafe extern "system" fn(*mut c_void, u32) -> HResult =
            core::mem::transmute(*vtable.add(4));
        let reset_fn: unsafe extern "system" fn(*mut c_void) -> HResult =
            core::mem::transmute(*vtable.add(5));
        let clone_fn: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HResult =
            core::mem::transmute(*vtable.add(6));

        // QI for IEnumVARIANT
        let mut p_qi: *mut c_void = core::ptr::null_mut();
        assert_eq!(qi_fn(p_enum, &IID_IEnumVARIANT, &mut p_qi), S_OK);
        assert_eq!(p_qi, p_enum);
        rel_fn(p_qi);

        // Next 1
        let mut v = Variant::empty();
        let mut fetched: u32 = 0;
        assert_eq!(next_fn(p_enum, 1, &mut v, &mut fetched), S_OK);
        assert_eq!(fetched, 1);
        assert_eq!(v.as_i32(), Some(10));

        // Skip 1 -> skips 20, next should be 30
        assert_eq!(skip_fn(p_enum, 1), S_OK);
        assert_eq!(next_fn(p_enum, 1, &mut v, &mut fetched), S_OK);
        assert_eq!(fetched, 1);
        assert_eq!(v.as_i32(), Some(30));

        // Clone
        let mut p_clone: *mut c_void = core::ptr::null_mut();
        assert_eq!(clone_fn(p_enum, &mut p_clone), S_OK);
        assert!(!p_clone.is_null());

        // Cloned enum should read 40
        let clone_vtable = *(p_clone as *mut *mut usize);
        let clone_next: unsafe extern "system" fn(*mut c_void, u32, *mut Variant, *mut u32) -> HResult =
            core::mem::transmute(*clone_vtable.add(3));
        assert_eq!(clone_next(p_clone, 1, &mut v, &mut fetched), S_OK);
        assert_eq!(v.as_i32(), Some(40));

        // Reset original
        assert_eq!(reset_fn(p_enum), S_OK);
        assert_eq!(next_fn(p_enum, 1, &mut v, &mut fetched), S_OK);
        assert_eq!(v.as_i32(), Some(10));

        rel_fn(p_clone);
        rel_fn(p_enum);
    }
}

#[test]
fn test_object_creation_and_addref() {
    unsafe {
        let mut target: *mut core::ffi::c_void = core::ptr::null_mut();
        let obj = __vbaNew2(&mut target, core::ptr::null());
        assert!(!obj.is_null());
        assert_eq!(target, obj);

        let ref_res = __vbaObjAddref(obj);
        assert_eq!(ref_res, obj);

        assert_eq!(__vbaPrintObj(obj), 0);
        assert_eq!(__vbaAptOffset(128), 128);
    }
}

#[test]
fn test_hresult_and_event_helpers() {
    unsafe {
        assert_eq!(__vbaHresultCheck(0), 0);
        assert_eq!(__vbaHresultCheckNonvirt(0), 0);
        assert_eq!(__vbaFailedFriend(0), 0);

        let p_sink = msvbvm60::com::VbClassInstance::new("EventSource") as *mut core::ffi::c_void;
        let event_res = __vbaRaiseEvent(p_sink, 1, 0, core::ptr::null_mut());
        assert_eq!(event_res, 0);
    }
}

#[test]
fn test_primitive_string_and_date_conversions() {
    unsafe {
        // Date to String
        let date_val = 45678.0; // valid OLE serial date
        let b_str = __vbaDateStr(date_val);
        assert!(!b_str.is_null());
        let str_val = bstr_to_string(b_str);
        assert!(str_val.contains("-"));
        SysFreeString(b_str);

        // String to Date
        let b_date = sys_alloc_string("2025-01-15");
        let parsed_date = __vbaStrDate(b_date);
        assert!(parsed_date > 40000.0);
        SysFreeString(b_date);

        // R4 and R8 strings
        let b_r4 = sys_alloc_string("123.45");
        let val_r4 = __vbaR4Str(b_r4);
        assert!((val_r4 - 123.45).abs() < 1e-2);
        SysFreeString(b_r4);

        let b_r8 = sys_alloc_string("98765.4321");
        let val_r8 = __vbaR8Str(b_r8);
        assert!((val_r8 - 98765.4321).abs() < 1e-4);
        SysFreeString(b_r8);

        // StrR8 and UI1Str
        let b_res_r8 = __vbaStrR8(3.14159);
        assert!(!b_res_r8.is_null());
        SysFreeString(b_res_r8);

        let b_byte = sys_alloc_string("250");
        assert_eq!(__vbaUI1Str(b_byte), 250);
        SysFreeString(b_byte);
    }
}

#[test]
fn test_varg_unk_and_indexing_helpers() {
    unsafe {
        // __vbaVargUnk and __vbaVargUnkAddref
        let fake_unk = msvbvm60::com::VbClassInstance::new("TestUnk") as *mut core::ffi::c_void;
        let mut v_unk = Variant::empty();
        __vbaVargUnk(&mut v_unk, fake_unk);
        assert_eq!(v_unk.vt, VT_UNKNOWN);

        let mut v_unk2 = Variant::empty();
        __vbaVargUnkAddref(&mut v_unk2, fake_unk);
        assert_eq!(v_unk2.vt, VT_UNKNOWN);

        let mut v_dst = Variant::empty();
        let mut v_var = Variant::from_i32(99);
        let idx = 1i32;
        let p_res = __vbaVarIndexLoadRef(&mut v_dst, &mut v_var, 1, &idx);
        assert_eq!(p_res, &mut v_dst as *mut _);
        assert_eq!(v_dst.as_i32(), Some(99));

        let mut v_dst2 = Variant::empty();
        let p_res2 = __vbaVarIndexLoadRefLock(&mut v_dst2, &mut v_var, 1, &idx);
        assert_eq!(p_res2, &mut v_dst2 as *mut _);
        assert_eq!(v_dst2.as_i32(), Some(99));
    }
}
