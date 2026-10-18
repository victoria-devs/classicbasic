// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

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
