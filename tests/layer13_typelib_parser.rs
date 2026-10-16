//! Layer 13: Clean-Room MSFT TypeLib Parser and DISPID Introspection Tests
//!
//! Validates Phase 2.2 COM infrastructure:
//! - Binary MSFT TypeLib reader (`MSFT\x02\x00\x01\x00`) parsing `ITypeLib` and `ITypeInfo`
//! - Extraction of TypeLib metadata (`IDictionary`, `Dictionary`, `FileSystemObject`, `IFileSystem`)
//! - DISPID Cache lookups (`lookup_dispid`, `lookup_func`)
//! - Early-bound typed instantiation via `__vbaNew` and `VbaClassDescriptor`
//! - Early-bound dual-interface method invocations (`Add`, `Item`, `Count`, `Exists`, `Remove`)
//! - Object casting (`__vbaCastObj`) and string variant transfer (`__vbaStrVarMove`)

use core::ffi::c_void;
use std::path::Path;

use msvbvm60::err::*;
use msvbvm60::typelib::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;

#[test]
fn test_parse_msft_typelib_scrrun() {
    let dll_path = Path::new("artifacts/scrrun.dll");
    assert!(dll_path.exists(), "artifacts/scrrun.dll must exist");

    let typelib = parse_from_file(dll_path).expect("Failed to parse MSFT TypeLib from scrrun.dll");

    // Validate TypeLib header metadata
    assert_eq!(typelib.name, "Scripting");
    let expected_lib_guid = Guid::parse("420B2830-E718-11CF-893D-00A0C9054228").unwrap();
    assert_eq!(typelib.guid, Some(expected_lib_guid));

    // Verify key TypeInfos
    let dict_info = typelib
        .get_type_info_by_name("IDictionary")
        .expect("IDictionary not found in typelib");
    assert_eq!(dict_info.type_kind, TypeKind::Dispatch);
    assert_eq!(dict_info.guid, Some(IID_IDictionary));
    assert!(dict_info.c_funcs >= 15);

    let coclass_dict = typelib
        .get_type_info_by_name("Dictionary")
        .expect("Dictionary coclass not found in typelib");
    assert_eq!(coclass_dict.type_kind, TypeKind::CoClass);
    assert_eq!(coclass_dict.guid, Some(CLSID_Dictionary));

    let fso_info = typelib
        .get_type_info_by_name("FileSystemObject")
        .expect("FileSystemObject coclass not found in typelib");
    assert_eq!(fso_info.type_kind, TypeKind::CoClass);

    // Verify IDictionary function members and DISPIDs
    let add_func = dict_info
        .funcs
        .iter()
        .find(|f| f.name == "Add")
        .expect("Add method not found");
    assert_eq!(add_func.memid, 1);
    assert_eq!(add_func.c_params, 2);

    let count_func = dict_info
        .funcs
        .iter()
        .find(|f| f.name == "Count")
        .expect("Count property not found");
    assert_eq!(count_func.memid, 2);

    let exists_func = dict_info
        .funcs
        .iter()
        .find(|f| f.name == "Exists")
        .expect("Exists method not found");
    assert_eq!(exists_func.memid, 3);
    assert_eq!(exists_func.c_params, 2);

    let remove_func = dict_info
        .funcs
        .iter()
        .find(|f| f.name == "Remove")
        .expect("Remove method not found");
    assert_eq!(remove_func.memid, 7);
}

#[test]
fn test_dispid_cache_lookups() {
    let dll_path = Path::new("artifacts/scrrun.dll");
    let typelib = parse_from_file(dll_path).expect("Failed to parse scrrun.dll");

    let mut cache = DispidCache::new();
    cache.register_typelib(&typelib);

    // Case-insensitive DISPID lookups
    assert_eq!(cache.lookup_dispid(&IID_IDictionary, "Add"), Some(1));
    assert_eq!(cache.lookup_dispid(&IID_IDictionary, "add"), Some(1));
    assert_eq!(cache.lookup_dispid(&IID_IDictionary, "ADD"), Some(1));
    assert_eq!(cache.lookup_dispid(&IID_IDictionary, "Count"), Some(2));
    assert_eq!(cache.lookup_dispid(&IID_IDictionary, "count"), Some(2));
    assert_eq!(cache.lookup_dispid(&IID_IDictionary, "Exists"), Some(3));
    assert_eq!(cache.lookup_dispid(&IID_IDictionary, "Items"), Some(4));
    assert_eq!(cache.lookup_dispid(&IID_IDictionary, "Key"), Some(5));
    assert_eq!(cache.lookup_dispid(&IID_IDictionary, "Keys"), Some(6));
    assert_eq!(cache.lookup_dispid(&IID_IDictionary, "Remove"), Some(7));
    assert_eq!(cache.lookup_dispid(&IID_IDictionary, "RemoveAll"), Some(8));
    assert_eq!(
        cache.lookup_dispid(&IID_IDictionary, "CompareMode"),
        Some(9)
    );

    // Reverse lookup by DISPID
    let add_meta = cache
        .lookup_func(&IID_IDictionary, 1)
        .expect("Func metadata not found");
    assert_eq!(add_meta.name, "Add");

    let count_meta = cache
        .lookup_func(&IID_IDictionary, 2)
        .expect("Func metadata not found");
    assert_eq!(count_meta.name, "Count");

    // Non-existent member
    assert_eq!(
        cache.lookup_dispid(&IID_IDictionary, "NonExistentMethod"),
        None
    );
}

#[test]
fn test_early_bound_dictionary_lifecycle_and_methods() {
    unsafe {
        // Test 1: Instantiation via VbaClassDescriptor and __vbaNew
        let class_desc = VbaClassDescriptor {
            reserved: 0,
            p_clsid: &CLSID_Dictionary,
            p_iid: &IID_IDictionary,
            reserved2: 0,
        };

        let raw_instance = __vbaNew(&class_desc as *const _ as *const c_void);
        assert!(
            !raw_instance.is_null(),
            "Instance created via __vbaNew must not be null"
        );

        let mut dict_ptr: *mut c_void = core::ptr::null_mut();
        __vbaObjSet(&mut dict_ptr, raw_instance);

        assert!(!dict_ptr.is_null());

        // Cast to IDictionary dual vtable
        let vtable = *(dict_ptr as *mut *mut usize);
        let get_item_fn: unsafe extern "system" fn(
            *mut c_void,
            *const Variant,
            *mut Variant,
        ) -> HResult = core::mem::transmute(*vtable.add(9));
        let add_fn: unsafe extern "system" fn(
            *mut c_void,
            *const Variant,
            *const Variant,
        ) -> HResult = core::mem::transmute(*vtable.add(10));
        let get_count_fn: unsafe extern "system" fn(*mut c_void, *mut i32) -> HResult =
            core::mem::transmute(*vtable.add(11));
        let exists_fn: unsafe extern "system" fn(*mut c_void, *const Variant, *mut i16) -> HResult =
            core::mem::transmute(*vtable.add(12));
        let remove_fn: unsafe extern "system" fn(*mut c_void, *const Variant) -> HResult =
            core::mem::transmute(*vtable.add(16));

        // Initial count is 0
        let mut count: i32 = -1;
        assert_eq!(get_count_fn(dict_ptr, &mut count), S_OK);
        assert_eq!(count, 0);

        // Add "Alpha" -> "Value1"
        let s_k1 = sys_alloc_string("Alpha");
        let s_v1 = sys_alloc_string("Value1");
        let v_k1 = Variant::from_bstr(s_k1);
        let v_v1 = Variant::from_bstr(s_v1);
        assert_eq!(add_fn(dict_ptr, &v_k1, &v_v1), S_OK);

        // Add "Beta" -> "Value2"
        let s_k2 = sys_alloc_string("Beta");
        let s_v2 = sys_alloc_string("Value2");
        let v_k2 = Variant::from_bstr(s_k2);
        let v_v2 = Variant::from_bstr(s_v2);
        assert_eq!(add_fn(dict_ptr, &v_k2, &v_v2), S_OK);

        // Count is now 2
        assert_eq!(get_count_fn(dict_ptr, &mut count), S_OK);
        assert_eq!(count, 2);

        // Exists("Alpha") -> True (-1)
        let mut exists_flag: i16 = 0;
        assert_eq!(exists_fn(dict_ptr, &v_k1, &mut exists_flag), S_OK);
        assert_eq!(exists_flag, -1);

        // Item("Alpha") -> "Value1"
        let mut ret_item = Variant::empty();
        assert_eq!(get_item_fn(dict_ptr, &v_k1, &mut ret_item), S_OK);
        let ret_bstr = __vbaStrVarMove(&mut ret_item);
        let ret_str = bstr_to_string(ret_bstr);
        SysFreeString(ret_bstr);
        assert_eq!(ret_str, "Value1");

        // Remove("Alpha")
        assert_eq!(remove_fn(dict_ptr, &v_k1), S_OK);

        // Exists("Alpha") -> False (0)
        assert_eq!(exists_fn(dict_ptr, &v_k1, &mut exists_flag), S_OK);
        assert_eq!(exists_flag, 0);

        // Count is now 1
        assert_eq!(get_count_fn(dict_ptr, &mut count), S_OK);
        assert_eq!(count, 1);

        // Test 3: Cleanup via __vbaCastObj and __vbaObjSet
        let cast_null = __vbaCastObj(core::ptr::null_mut(), &IID_IDictionary);
        assert!(cast_null.is_null());
        __vbaObjSet(&mut dict_ptr, cast_null);
        assert!(dict_ptr.is_null());

        SysFreeString(s_k1);
        SysFreeString(s_v1);
        SysFreeString(s_k2);
        SysFreeString(s_v2);
    }
}

#[test]
fn test_vba_str_var_move_conversions() {
    unsafe {
        // 1. Move from string Variant (VT_BSTR)
        let s = sys_alloc_string("Hello World");
        let mut v = Variant::from_bstr(s);
        let extracted_bstr = __vbaStrVarMove(&mut v);
        assert_eq!(bstr_to_string(extracted_bstr), "Hello World");
        assert_eq!(v.vt, 0); // VT_EMPTY after move
        SysFreeString(extracted_bstr);

        // 2. Move from numeric Variant (VT_I4)
        let mut v_num = Variant::from_i32(12345);
        let extracted_num_bstr = __vbaStrVarMove(&mut v_num);
        assert_eq!(bstr_to_string(extracted_num_bstr), "12345");
        assert_eq!(v_num.vt, 0);
        SysFreeString(extracted_num_bstr);

        // 3. Move from boolean Variant (VT_BOOL)
        let mut v_bool = Variant::from_bool(true);
        let extracted_bool_bstr = __vbaStrVarMove(&mut v_bool);
        assert_eq!(bstr_to_string(extracted_bool_bstr), "True");
        assert_eq!(v_bool.vt, 0);
        SysFreeString(extracted_bool_bstr);
    }
}
