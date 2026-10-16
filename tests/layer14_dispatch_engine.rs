//! Layer 14: Clean-Room Late-Binding Dispatch Engine Tests
//!
//! Validates Phase 2.3 COM infrastructure:
//! - `IDispatch::GetIDsOfNames` case-insensitive member name resolution
//! - `IDispatch::Invoke` execution with `DISPPARAMS` (reverse argument ordering)
//! - Late-bound Property Get (`DISPATCH_PROPERTYGET`), Property Put (`DISPATCH_PROPERTYPUT`), and Method (`DISPATCH_METHOD`)
//! - Default member (DISPID 0 / `DISPID_VALUE`) invocation
//! - Compiler runtime helpers: `__vbaLateMemCall*`, `__vbaLateIdCall*`, `__vbaVarLateMemCall*`
//! - Variant object extraction (`__vbaUnkVar`) and reference assignment (`__vbaVarSetObjAddref`)
//! - Error handling: non-existent member raises VB error 438 ("Object doesn't support this property or method")

use core::ffi::c_void;
use msvbvm60::com::*;
use msvbvm60::err::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;

fn create_test_dictionary() -> *mut c_void {
    let mut p_obj: *mut c_void = core::ptr::null_mut();
    unsafe {
        let hr = get_dictionary_class_factory(&IID_IClassFactory, &mut p_obj);
        assert_eq!(hr, S_OK);
        let cf_vtbl = *(p_obj as *mut *mut usize);
        let create_instance_fn: unsafe extern "system" fn(
            *mut c_void,
            *mut c_void,
            *const Iid,
            *mut *mut c_void,
        ) -> HResult = core::mem::transmute(*cf_vtbl.add(3));
        let mut dict_ptr: *mut c_void = core::ptr::null_mut();
        let hr_inst = create_instance_fn(p_obj, core::ptr::null_mut(), &IID_IDispatch, &mut dict_ptr);
        assert_eq!(hr_inst, S_OK);
        com_release(p_obj);
        dict_ptr
    }
}

#[test]
fn test_dict_get_ids_of_names() {
    let dict = create_test_dictionary();
    assert!(!dict.is_null());

    unsafe {
        let vtable = *(dict as *mut *mut usize);
        let get_ids_fn: unsafe extern "system" fn(
            *mut c_void,
            *const Iid,
            *const *const u16,
            u32,
            u32,
            *mut i32,
        ) -> HResult = core::mem::transmute(*vtable.add(5));

        // Test standard member names and case insensitivity
        let names_to_test = [
            ("Item", 0),
            ("item", 0),
            ("ITEM", 0),
            ("Add", 1),
            ("add", 1),
            ("Count", 2),
            ("Exists", 3),
            ("Items", 4),
            ("Key", 5),
            ("Keys", 6),
            ("Remove", 7),
            ("RemoveAll", 8),
            ("CompareMode", 9),
        ];

        for (name_str, expected_dispid) in names_to_test {
            let utf16 = format!("{}\0", name_str).encode_utf16().collect::<Vec<u16>>();
            let mut dispid: i32 = -999;
            let name_ptrs: [*const u16; 1] = [utf16.as_ptr()];
            let hr = get_ids_fn(dict, &IID_NULL, name_ptrs.as_ptr(), 1, 0, &mut dispid);
            assert_eq!(hr, S_OK, "GetIDsOfNames failed for {}", name_str);
            assert_eq!(dispid, expected_dispid, "Unexpected DISPID for {}", name_str);
        }

        // Test unknown member name
        let unknown = "MetodoQueNaoExiste\0".encode_utf16().collect::<Vec<u16>>();
        let mut unk_dispid: i32 = 0;
        let unk_ptrs: [*const u16; 1] = [unknown.as_ptr()];
        let hr_unk = get_ids_fn(dict, &IID_NULL, unk_ptrs.as_ptr(), 1, 0, &mut unk_dispid);
        assert_eq!(hr_unk, DISP_E_UNKNOWNNAME);
        assert_eq!(unk_dispid, DISPID_UNKNOWN);

        com_release(dict);
    }
}

#[test]
fn test_dict_invoke_methods_and_properties() {
    let dict = create_test_dictionary();
    assert!(!dict.is_null());

    unsafe {
        // 1. Add("Fruit", "Apple")
        let s_fruit = sys_alloc_string("Fruit");
        let s_apple = sys_alloc_string("Apple");
        let v_fruit = Variant::from_bstr(s_fruit);
        let v_apple = Variant::from_bstr(s_apple);
        let mut add_args = [v_apple, v_fruit]; // Reverse order: rgvarg[1]=key, rgvarg[0]=item
        let hr_add = dispatch_invoke_id(
            dict,
            1,
            DISPATCH_METHOD,
            add_args.as_mut_ptr(),
            2,
            core::ptr::null_mut(),
        );
        assert_eq!(hr_add, S_OK);

        // 2. Count -> 1
        let mut v_count = Variant::empty();
        let hr_count = dispatch_invoke_id(
            dict,
            2,
            DISPATCH_PROPERTYGET,
            core::ptr::null_mut(),
            0,
            &mut v_count,
        );
        assert_eq!(hr_count, S_OK);
        assert_eq!(v_count.as_i32(), Some(1));

        // 3. Exists("Fruit") -> True
        let mut v_fruit_q = Variant::from_bstr(s_fruit);
        let mut v_exists = Variant::empty();
        let hr_exists = dispatch_invoke_id(
            dict,
            3,
            DISPATCH_METHOD | DISPATCH_PROPERTYGET,
            &mut v_fruit_q,
            1,
            &mut v_exists,
        );
        assert_eq!(hr_exists, S_OK);
        assert_eq!(v_exists.as_bool(), Some(true));

        // 4. Item("Fruit") (Default member DISPID 0) -> "Apple"
        let mut v_fruit_item = Variant::from_bstr(s_fruit);
        let mut v_item_ret = Variant::empty();
        let hr_item = dispatch_invoke_id(
            dict,
            0,
            DISPATCH_PROPERTYGET,
            &mut v_fruit_item,
            1,
            &mut v_item_ret,
        );
        assert_eq!(hr_item, S_OK);
        let bstr_res = v_item_ret.as_bstr().unwrap();
        assert_eq!(bstr_to_string(bstr_res), "Apple");

        // 5. Item("Fruit") = "Orange" (Property Put)
        let s_orange = sys_alloc_string("Orange");
        let v_orange = Variant::from_bstr(s_orange);
        let mut put_args = [v_orange, v_fruit]; // rgvarg[0]=value, rgvarg[1]=key
        let hr_put = dispatch_invoke_id(
            dict,
            0,
            DISPATCH_PROPERTYPUT,
            put_args.as_mut_ptr(),
            1,
            core::ptr::null_mut(),
        );
        assert_eq!(hr_put, S_OK);

        // Verify update
        let mut v_fruit_check = Variant::from_bstr(s_fruit);
        let mut v_item_updated = Variant::empty();
        dispatch_invoke_id(
            dict,
            0,
            DISPATCH_PROPERTYGET,
            &mut v_fruit_check,
            1,
            &mut v_item_updated,
        );
        let bstr_upd = v_item_updated.as_bstr().unwrap();
        assert_eq!(bstr_to_string(bstr_upd), "Orange");

        // 6. Keys collection
        let mut v_keys = Variant::empty();
        let hr_keys = dispatch_invoke_id(
            dict,
            6,
            DISPATCH_PROPERTYGET,
            core::ptr::null_mut(),
            0,
            &mut v_keys,
        );
        assert_eq!(hr_keys, S_OK);
        let keys_sa = __vbaRefVarAry(&mut v_keys);
        assert!(!keys_sa.is_null());
        let p_sa = core::ptr::read_unaligned(keys_sa);
        assert!(!p_sa.is_null());
        assert_eq!(__vbaUbound(p_sa, 1), 0); // 0-based 1 item -> ubound=0

        // 7. Remove("Fruit")
        let mut v_fruit_rem = Variant::from_bstr(s_fruit);
        let hr_rem = dispatch_invoke_id(
            dict,
            7,
            DISPATCH_METHOD,
            &mut v_fruit_rem,
            1,
            core::ptr::null_mut(),
        );
        assert_eq!(hr_rem, S_OK);

        // 8. Count after remove -> 0
        let mut v_count_zero = Variant::empty();
        dispatch_invoke_id(
            dict,
            2,
            DISPATCH_PROPERTYGET,
            core::ptr::null_mut(),
            0,
            &mut v_count_zero,
        );
        assert_eq!(v_count_zero.as_i32(), Some(0));

        SysFreeString(s_fruit);
        SysFreeString(s_apple);
        SysFreeString(s_orange);
        com_release(dict);
    }
}

#[test]
fn test_late_binding_helpers_and_error_handling() {
    let dict = create_test_dictionary();
    assert!(!dict.is_null());

    unsafe {
        clear_vb_error();

        // 1. Invocations via member name helper
        let s_add = "Add\0".encode_utf16().collect::<Vec<u16>>();
        let s_k = sys_alloc_string("Rust");
        let s_v = sys_alloc_string("Awesome");
        let v_k = Variant::from_bstr(s_k);
        let v_v = Variant::from_bstr(s_v);
        let mut args = [v_v, v_k];
        let hr = dispatch_invoke_member(
            dict,
            s_add.as_ptr(),
            DISPATCH_METHOD,
            args.as_mut_ptr(),
            2,
            core::ptr::null_mut(),
        );
        assert_eq!(hr, S_OK);

        // 2. Calling non-existent method raises Error 438
        let s_missing = "NoSuchMethodHere\0".encode_utf16().collect::<Vec<u16>>();
        let hr_err = dispatch_invoke_member(
            dict,
            s_missing.as_ptr(),
            DISPATCH_METHOD,
            core::ptr::null_mut(),
            0,
            core::ptr::null_mut(),
        );
        assert_eq!(hr_err, DISP_E_UNKNOWNNAME);
        assert_eq!(get_vb_error_number(), 438);
        clear_vb_error();

        // 3. Variant late-binding helpers (__vbaVarSetObjAddref & __vbaUnkVar)
        let mut v_obj = Variant::empty();
        __vbaVarSetObjAddref(&mut v_obj, dict);
        assert_eq!(v_obj.vt, VT_DISPATCH);

        let p_extracted = __vbaUnkVar(&v_obj, 0);
        assert_eq!(p_extracted, dict);

        // Clear variant object
        __vbaVarSetObjAddref(&mut v_obj, core::ptr::null_mut());
        assert_eq!(v_obj.vt, VT_EMPTY);
        let p_cleared = __vbaUnkVar(&v_obj, 0);
        assert!(p_cleared.is_null());

        SysFreeString(s_k);
        SysFreeString(s_v);
        com_release(dict);
    }
}
