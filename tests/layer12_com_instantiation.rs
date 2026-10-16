//! Layer 12: COM Instantiation, Virtual Registry, and Class Factory Tests
//!
//! Validates Phase 2.1 COM infrastructure:
//! - Virtualized JSON Registry (ProgID -> CLSID -> InprocServer32)
//! - `DllGetClassObject` and `IClassFactory::CreateInstance`
//! - `rtcCreateObject2`, `rtcCreateObject`, and error 429 on missing ProgID
//! - COM helper functions: `rtcIsObject`, `__vbaObjIs`, `__vbaObjVar`, `__vbaObjSetAddref`
//! - Introspection type naming (`rtcTypeName`) for active COM objects

use core::ffi::c_void;
use std::path::Path;

use msvbvm60::com::*;
use msvbvm60::err::*;
use msvbvm60::registry::*;
use msvbvm60::rtc::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;

#[test]
fn test_virtual_registry_defaults_and_resolution() {
    let reg = Registry::with_defaults();
    let clsid = reg.resolve_progid("Scripting.Dictionary");
    assert_eq!(clsid, Some(CLSID_Dictionary));

    let entry = reg.get_class_entry(&CLSID_Dictionary).expect("Dictionary entry not found");
    assert_eq!(entry.prog_id, "Scripting.Dictionary");
    assert_eq!(entry.friendly_name, "Microsoft Scripting Dictionary");
    let inproc = entry.inproc_server.as_ref().expect("InprocServer32 missing");
    assert_eq!(inproc.server_path, "scrrun.dll");
    assert_eq!(inproc.threading_model, "Both");

    // Case-insensitive ProgID lookup
    let clsid_lower = reg.resolve_progid("scripting.dictionary");
    assert_eq!(clsid_lower, Some(CLSID_Dictionary));

    // Non-existent ProgID
    assert!(reg.resolve_progid("NonExistent.Component.99").is_none());
}

#[test]
fn test_virtual_registry_json_serialization_roundtrip() {
    let mut reg = Registry::default();
    let dummy_clsid = Guid::new(0x12345678, 0x1234, 0x5678, [1, 2, 3, 4, 5, 6, 7, 8]);
    reg.register_class(
        "MyApp.Engine",
        dummy_clsid,
        "MyApp Engine Coclass",
        "myengine.dll",
        "Apartment",
    );

    let temp_dir = std::env::temp_dir();
    let temp_reg_file = temp_dir.join("test_registry_roundtrip.json");
    reg.save_to_file(&temp_reg_file).expect("Failed to save registry to file");

    let loaded = Registry::load_or_default(&temp_reg_file);
    assert_eq!(loaded.resolve_progid("MyApp.Engine"), Some(dummy_clsid));
    let entry = loaded.get_class_entry(&dummy_clsid).unwrap();
    assert_eq!(entry.friendly_name, "MyApp Engine Coclass");
    assert_eq!(entry.inproc_server.as_ref().unwrap().server_path, "myengine.dll");

    let _ = std::fs::remove_file(temp_reg_file);
}

#[test]
fn test_dll_get_class_object_and_class_factory() {
    unsafe {
        // Null pointer checks
        assert_eq!(DllGetClassObject(core::ptr::null(), &IID_IClassFactory, core::ptr::null_mut()), E_POINTER);

        // Unknown CLSID returns CLASS_E_CLASSNOTAVAILABLE
        let unknown_clsid = Guid::new(0x11111111, 0x2222, 0x3333, [0; 8]);
        let mut p_cf: *mut c_void = core::ptr::null_mut();
        let hr_fail = DllGetClassObject(&unknown_clsid, &IID_IClassFactory, &mut p_cf);
        assert_eq!(hr_fail, CLASS_E_CLASSNOTAVAILABLE);
        assert!(p_cf.is_null());

        // Known CLSID (CLSID_Dictionary) returns IClassFactory
        let hr_ok = DllGetClassObject(&CLSID_Dictionary, &IID_IClassFactory, &mut p_cf);
        assert_eq!(hr_ok, S_OK);
        assert!(!p_cf.is_null());

        // Test IClassFactory::AddRef and Release
        let cf_vtable = *(p_cf as *mut *mut usize);
        let cf_add_ref: unsafe extern "system" fn(*mut c_void) -> u32 = core::mem::transmute(*cf_vtable.add(1));
        let cf_release: unsafe extern "system" fn(*mut c_void) -> u32 = core::mem::transmute(*cf_vtable.add(2));

        let ref1 = cf_add_ref(p_cf);
        assert!(ref1 > 1);
        let ref2 = cf_release(p_cf);
        assert_eq!(ref2, ref1 - 1);

        // Test IClassFactory::CreateInstance aggregation rejection
        let cf_create_instance: unsafe extern "system" fn(
            *mut c_void,
            *mut c_void,
            *const Iid,
            *mut *mut c_void,
        ) -> HResult = core::mem::transmute(*cf_vtable.add(3));

        let mut dummy_outer: usize = 0xDEADBEEF;
        let mut p_obj: *mut c_void = core::ptr::null_mut();
        let hr_agg = cf_create_instance(
            p_cf,
            &mut dummy_outer as *mut usize as *mut c_void,
            &IID_IDispatch,
            &mut p_obj,
        );
        assert_eq!(hr_agg, CLASS_E_NOAGGREGATION);

        // Test IClassFactory::CreateInstance successful instantiation
        let hr_create = cf_create_instance(
            p_cf,
            core::ptr::null_mut(),
            &IID_IDispatch,
            &mut p_obj,
        );
        assert_eq!(hr_create, S_OK);
        assert!(!p_obj.is_null());

        // Release the created object
        com_release(p_obj);
    }
}

#[test]
fn test_rtc_create_object2_success_and_lifecycle() {
    unsafe {
        clear_vb_error();
        let prog_id = sys_alloc_string("Scripting.Dictionary");
        let mut v_dict = Variant::empty();
        rtcCreateObject2(&mut v_dict, prog_id, core::ptr::null_mut());

        assert_eq!(get_vb_error_number(), 0);
        assert_eq!(v_dict.vt, VT_DISPATCH);

        // Check rtcIsObject
        assert_eq!(rtcIsObject(&v_dict), -1);

        // Check rtcTypeName returns "Dictionary"
        let b_typ = rtcTypeName(&v_dict);
        let type_name = bstr_to_string(b_typ);
        SysFreeString(b_typ);
        assert_eq!(type_name, "Dictionary");

        // Extract object pointer via __vbaObjVar
        let p_obj = __vbaObjVar(&v_dict);
        assert!(!p_obj.is_null());

        // Assign to an object slot with AddRef
        let mut obj_slot: *mut c_void = core::ptr::null_mut();
        __vbaObjSetAddref(&mut obj_slot, p_obj);
        assert_eq!(obj_slot, p_obj);

        // Free the variant temporary
        __vbaFreeVar(&mut v_dict);
        assert_eq!(v_dict.vt, VT_EMPTY);

        // Check object is not Nothing
        assert_eq!(__vbaObjIs(obj_slot, core::ptr::null_mut()), 0);

        // Free the object slot (Set obj = Nothing)
        __vbaFreeObj(&mut obj_slot);
        assert!(obj_slot.is_null());
        assert_eq!(__vbaObjIs(obj_slot, core::ptr::null_mut()), -1);

        SysFreeString(prog_id);
    }
}

#[test]
fn test_rtc_create_object2_nonexistent_raises_error_429() {
    unsafe {
        clear_vb_error();
        let prog_id = sys_alloc_string("NonExistent.Component.99");
        let mut v_res = Variant::empty();
        rtcCreateObject2(&mut v_res, prog_id, core::ptr::null_mut());

        assert_eq!(get_vb_error_number(), 429);
        assert_eq!(get_vb_error_description(), "ActiveX component can't create object");
        assert_eq!(v_res.vt, VT_EMPTY);

        SysFreeString(prog_id);
        clear_vb_error();
    }
}

#[test]
fn test_rtc_is_object_and_type_name_edge_cases() {
    unsafe {
        // Integer variant (VT_I4) -> IsObject == False (0), TypeName == "Long"
        let v_long = Variant::from_i32(100);
        assert_eq!(rtcIsObject(&v_long), 0);
        let b_long = rtcTypeName(&v_long);
        assert_eq!(bstr_to_string(b_long), "Long");
        SysFreeString(b_long);

        // Integer variant (VT_I2) -> TypeName == "Integer"
        let v_int = Variant::from_i16(100);
        assert_eq!(rtcIsObject(&v_int), 0);
        let b_int = rtcTypeName(&v_int);
        assert_eq!(bstr_to_string(b_int), "Integer");
        SysFreeString(b_int);

        // Null pointer passed to IsObject -> 0
        assert_eq!(rtcIsObject(core::ptr::null()), 0);

        // Nothing object: Variant with VT_DISPATCH and null pointer
        let mut v_nothing = Variant::empty();
        v_nothing.vt = VT_DISPATCH;
        assert_eq!(rtcIsObject(&v_nothing), -1); // IsObject(Nothing) is True in VB6
        let b_not = rtcTypeName(&v_nothing);
        assert_eq!(bstr_to_string(b_not), "Nothing");
        SysFreeString(b_not);

        // ErrObject introspection
        let p_err = rtcErrObj();
        let mut v_err = Variant::empty();
        v_err.vt = VT_DISPATCH;
        let ptr_bytes = (p_err as usize).to_ne_bytes();
        let v_err_ref = &mut v_err;
        v_err_ref.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
        assert_eq!(rtcIsObject(&v_err), -1);
        let b_err = rtcTypeName(&v_err);
        assert_eq!(bstr_to_string(b_err), "ErrObject");
        SysFreeString(b_err);
    }
}
