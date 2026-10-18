// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # COM & TypeLib: Class Factories & In-Proc Server Lifecycle
//!
//! DllGetClassObject, IClassFactory::CreateInstance, Virtual COM Registry,
//! CreateObject/GetObject, and ActiveX provider shims (FSO, WSH, ADODB).

use core::ffi::c_void;
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

    let entry = reg
        .get_class_entry(&CLSID_Dictionary)
        .expect("Dictionary entry not found");
    assert_eq!(entry.prog_id, "Scripting.Dictionary");
    assert_eq!(entry.friendly_name, "Microsoft Scripting Dictionary");
    let inproc = entry
        .inproc_server
        .as_ref()
        .expect("InprocServer32 missing");
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
    reg.save_to_file(&temp_reg_file)
        .expect("Failed to save registry to file");

    let loaded = Registry::load_or_default(&temp_reg_file);
    assert_eq!(loaded.resolve_progid("MyApp.Engine"), Some(dummy_clsid));
    let entry = loaded.get_class_entry(&dummy_clsid).unwrap();
    assert_eq!(entry.friendly_name, "MyApp Engine Coclass");
    assert_eq!(
        entry.inproc_server.as_ref().unwrap().server_path,
        "myengine.dll"
    );

    let _ = std::fs::remove_file(temp_reg_file);
}

#[test]
fn test_dll_get_class_object_and_class_factory() {
    unsafe {
        // Null pointer checks
        assert_eq!(
            DllGetClassObject(core::ptr::null(), &IID_IClassFactory, core::ptr::null_mut()),
            E_POINTER
        );

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
        let cf_add_ref: unsafe extern "system" fn(*mut c_void) -> u32 =
            core::mem::transmute(*cf_vtable.add(1));
        let cf_release: unsafe extern "system" fn(*mut c_void) -> u32 =
            core::mem::transmute(*cf_vtable.add(2));

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
        let hr_create = cf_create_instance(p_cf, core::ptr::null_mut(), &IID_IDispatch, &mut p_obj);
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
        assert_eq!(
            get_vb_error_description(),
            "ActiveX component can't create object"
        );
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




#[test]
fn test_fso_path_and_file_manipulation() {
    unsafe {
        // Instantiate FileSystemObject
        let s_progid = sys_alloc_string("Scripting.FileSystemObject");
        let mut v_fso = Variant::empty();
        rtcCreateObject2(&mut v_fso, s_progid, core::ptr::null_mut());
        assert_eq!(v_fso.vt, VT_DISPATCH);
        let fso_ptr = core::ptr::read_unaligned(v_fso.data.as_ptr() as *const *mut core::ffi::c_void);
        assert!(!fso_ptr.is_null());

        // Test BuildPath
        let mut ret = Variant::empty();
        let s_path = sys_alloc_string("C:\\Work");
        let s_file = sys_alloc_string("document.doc");
        let mut v_p = Variant::empty();
        v_p.vt = VT_BSTR;
        v_p.data[..core::mem::size_of::<usize>()].copy_from_slice(&(s_path as usize).to_ne_bytes());
        let mut v_f = Variant::empty();
        v_f.vt = VT_BSTR;
        v_f.data[..core::mem::size_of::<usize>()].copy_from_slice(&(s_file as usize).to_ne_bytes());
        let mut args_bp = [v_f, v_p]; // Reverse order
        let name_bp = sys_alloc_string("BuildPath");
        let hr = dispatch_invoke_member(
            fso_ptr,
            name_bp as *const u16,
            DISPATCH_METHOD,
            args_bp.as_mut_ptr(),
            2,
            &mut ret,
        );
        assert_eq!(hr, S_OK);
        assert_eq!(ret.vt, VT_BSTR);
        let bp_str = bstr_to_string(core::ptr::read_unaligned(ret.data.as_ptr() as *const Bstr));
        assert_eq!(bp_str, "C:\\Work\\document.doc");
        SysFreeString(name_bp);
        SysFreeString(s_path);
        SysFreeString(s_file);

        // Test GetDriveName, GetParentFolderName, GetBaseName, GetExtensionName
        let s_full = sys_alloc_string("C:\\Work\\SubFolder\\file.txt");
        let mut v_full = Variant::empty();
        v_full.vt = VT_BSTR;
        v_full.data[..core::mem::size_of::<usize>()].copy_from_slice(&(s_full as usize).to_ne_bytes());

        let name_drive = sys_alloc_string("GetDriveName");
        let mut args_drive = [v_full];
        let mut ret_drive = Variant::empty();
        dispatch_invoke_member(fso_ptr, name_drive as *const u16, DISPATCH_METHOD, args_drive.as_mut_ptr(), 1, &mut ret_drive);
        let drive_str = bstr_to_string(core::ptr::read_unaligned(ret_drive.data.as_ptr() as *const Bstr));
        assert_eq!(drive_str, "C:");
        SysFreeString(name_drive);

        let name_parent = sys_alloc_string("GetParentFolderName");
        let mut args_parent = [v_full];
        let mut ret_parent = Variant::empty();
        dispatch_invoke_member(fso_ptr, name_parent as *const u16, DISPATCH_METHOD, args_parent.as_mut_ptr(), 1, &mut ret_parent);
        let parent_str = bstr_to_string(core::ptr::read_unaligned(ret_parent.data.as_ptr() as *const Bstr));
        assert_eq!(parent_str, "C:\\Work\\SubFolder");
        SysFreeString(name_parent);

        let name_base = sys_alloc_string("GetBaseName");
        let mut args_base = [v_full];
        let mut ret_base = Variant::empty();
        dispatch_invoke_member(fso_ptr, name_base as *const u16, DISPATCH_METHOD, args_base.as_mut_ptr(), 1, &mut ret_base);
        let base_str = bstr_to_string(core::ptr::read_unaligned(ret_base.data.as_ptr() as *const Bstr));
        assert_eq!(base_str, "file");
        SysFreeString(name_base);

        let name_ext = sys_alloc_string("GetExtensionName");
        let mut args_ext = [v_full];
        let mut ret_ext = Variant::empty();
        dispatch_invoke_member(fso_ptr, name_ext as *const u16, DISPATCH_METHOD, args_ext.as_mut_ptr(), 1, &mut ret_ext);
        let ext_str = bstr_to_string(core::ptr::read_unaligned(ret_ext.data.as_ptr() as *const Bstr));
        assert_eq!(ext_str, "txt");
        SysFreeString(name_ext);
        SysFreeString(s_full);

        // Test CreateTextFile, WriteLine, and OpenTextFile
        let test_filename = "layer15_test_fso.txt";
        let s_testfile = sys_alloc_string(test_filename);
        let mut v_tf = Variant::empty();
        v_tf.vt = VT_BSTR;
        v_tf.data[..core::mem::size_of::<usize>()].copy_from_slice(&(s_testfile as usize).to_ne_bytes());
        let mut v_over = Variant::empty();
        v_over.vt = VT_BOOL;
        let minus_one: i16 = -1;
        v_over.data[0..2].copy_from_slice(&minus_one.to_ne_bytes());
        let mut args_create = [v_over, v_tf];
        let name_create = sys_alloc_string("CreateTextFile");
        let mut ret_ts = Variant::empty();
        dispatch_invoke_member(fso_ptr, name_create as *const u16, DISPATCH_METHOD, args_create.as_mut_ptr(), 2, &mut ret_ts);
        assert_eq!(ret_ts.vt, VT_DISPATCH);
        let ts_ptr = core::ptr::read_unaligned(ret_ts.data.as_ptr() as *const *mut core::ffi::c_void);
        assert!(!ts_ptr.is_null());
        SysFreeString(name_create);

        // WriteLine
        let s_l1 = sys_alloc_string("First Line Content");
        let mut v_l1 = Variant::empty();
        v_l1.vt = VT_BSTR;
        v_l1.data[..core::mem::size_of::<usize>()].copy_from_slice(&(s_l1 as usize).to_ne_bytes());
        let mut args_wl = [v_l1];
        let name_wl = sys_alloc_string("WriteLine");
        dispatch_invoke_member(ts_ptr, name_wl as *const u16, DISPATCH_METHOD, args_wl.as_mut_ptr(), 1, core::ptr::null_mut());
        SysFreeString(name_wl);
        SysFreeString(s_l1);

        // Close TextStream
        let name_close = sys_alloc_string("Close");
        dispatch_invoke_member(ts_ptr, name_close as *const u16, DISPATCH_METHOD, core::ptr::null_mut(), 0, core::ptr::null_mut());
        SysFreeString(name_close);
        com_release(ts_ptr);

        // FileExists
        let mut args_fe = [v_tf];
        let name_fe = sys_alloc_string("FileExists");
        let mut ret_fe = Variant::empty();
        dispatch_invoke_member(fso_ptr, name_fe as *const u16, DISPATCH_METHOD, args_fe.as_mut_ptr(), 1, &mut ret_fe);
        assert_eq!(ret_fe.vt, VT_BOOL);
        assert_eq!(core::ptr::read_unaligned(ret_fe.data.as_ptr() as *const i16), -1);
        SysFreeString(name_fe);

        // OpenTextFile (1 = ForReading)
        let mut v_mode = Variant::empty();
        v_mode.vt = VT_I4;
        let one: i32 = 1;
        v_mode.data[0..4].copy_from_slice(&one.to_ne_bytes());
        let mut args_open = [v_mode, v_tf];
        let name_open = sys_alloc_string("OpenTextFile");
        let mut ret_ts_in = Variant::empty();
        dispatch_invoke_member(fso_ptr, name_open as *const u16, DISPATCH_METHOD, args_open.as_mut_ptr(), 2, &mut ret_ts_in);
        let ts_in_ptr = core::ptr::read_unaligned(ret_ts_in.data.as_ptr() as *const *mut core::ffi::c_void);
        assert!(!ts_in_ptr.is_null());
        SysFreeString(name_open);

        // ReadLine
        let name_rl = sys_alloc_string("ReadLine");
        let mut ret_rl = Variant::empty();
        dispatch_invoke_member(ts_in_ptr, name_rl as *const u16, DISPATCH_METHOD, core::ptr::null_mut(), 0, &mut ret_rl);
        let read_line = bstr_to_string(core::ptr::read_unaligned(ret_rl.data.as_ptr() as *const Bstr));
        assert_eq!(read_line, "First Line Content");
        SysFreeString(name_rl);
        com_release(ts_in_ptr);

        // DeleteFile
        let mut args_del = [v_tf];
        let name_del = sys_alloc_string("DeleteFile");
        dispatch_invoke_member(fso_ptr, name_del as *const u16, DISPATCH_METHOD, args_del.as_mut_ptr(), 1, core::ptr::null_mut());
        SysFreeString(name_del);

        // Verify FileExists is false
        let mut ret_fe2 = Variant::empty();
        let name_fe2 = sys_alloc_string("FileExists");
        dispatch_invoke_member(fso_ptr, name_fe2 as *const u16, DISPATCH_METHOD, args_del.as_mut_ptr(), 1, &mut ret_fe2);
        assert_eq!(core::ptr::read_unaligned(ret_fe2.data.as_ptr() as *const i16), 0);
        SysFreeString(name_fe2);
        SysFreeString(s_testfile);

        SysFreeString(s_progid);
        com_release(fso_ptr);
    }
}

#[test]
fn test_wsh_environment_and_registry() {
    unsafe {
        // Instantiate WScript.Shell
        let s_wsh = sys_alloc_string("WScript.Shell");
        let mut v_wsh = Variant::empty();
        rtcCreateObject2(&mut v_wsh, s_wsh, core::ptr::null_mut());
        assert_eq!(v_wsh.vt, VT_DISPATCH);
        let wsh_ptr = core::ptr::read_unaligned(v_wsh.data.as_ptr() as *const *mut core::ffi::c_void);
        assert!(!wsh_ptr.is_null());

        // Test CurrentDirectory
        let name_cur = sys_alloc_string("CurrentDirectory");
        let mut ret_cur = Variant::empty();
        let hr = dispatch_invoke_member(
            wsh_ptr,
            name_cur as *const u16,
            DISPATCH_PROPERTYGET,
            core::ptr::null_mut(),
            0,
            &mut ret_cur,
        );
        assert_eq!(hr, S_OK);
        assert_eq!(ret_cur.vt, VT_BSTR);
        let cur_str = bstr_to_string(core::ptr::read_unaligned(ret_cur.data.as_ptr() as *const Bstr));
        assert!(!cur_str.is_empty());
        SysFreeString(name_cur);

        // Test RegWrite & RegRead & RegDelete
        let s_key = sys_alloc_string("HKCU\\Software\\TestApp\\Setting");
        let s_val = sys_alloc_string("Value123");
        let mut v_k = Variant::empty();
        v_k.vt = VT_BSTR;
        v_k.data[..core::mem::size_of::<usize>()].copy_from_slice(&(s_key as usize).to_ne_bytes());
        let mut v_v = Variant::empty();
        v_v.vt = VT_BSTR;
        v_v.data[..core::mem::size_of::<usize>()].copy_from_slice(&(s_val as usize).to_ne_bytes());
        let mut args_rw = [v_v, v_k];
        let name_rw = sys_alloc_string("RegWrite");
        dispatch_invoke_member(wsh_ptr, name_rw as *const u16, DISPATCH_METHOD, args_rw.as_mut_ptr(), 2, core::ptr::null_mut());
        SysFreeString(name_rw);

        let mut args_rr = [v_k];
        let name_rr = sys_alloc_string("RegRead");
        let mut ret_rr = Variant::empty();
        dispatch_invoke_member(wsh_ptr, name_rr as *const u16, DISPATCH_METHOD, args_rr.as_mut_ptr(), 1, &mut ret_rr);
        let read_val = bstr_to_string(core::ptr::read_unaligned(ret_rr.data.as_ptr() as *const Bstr));
        assert_eq!(read_val, "Value123");
        SysFreeString(name_rr);

        let name_rd = sys_alloc_string("RegDelete");
        dispatch_invoke_member(wsh_ptr, name_rd as *const u16, DISPATCH_METHOD, args_rr.as_mut_ptr(), 1, core::ptr::null_mut());
        SysFreeString(name_rd);
        SysFreeString(s_key);
        SysFreeString(s_val);
        SysFreeString(s_wsh);
        com_release(wsh_ptr);

        // Test WScript.Network
        let s_net = sys_alloc_string("WScript.Network");
        let mut v_net = Variant::empty();
        rtcCreateObject2(&mut v_net, s_net, core::ptr::null_mut());
        assert_eq!(v_net.vt, VT_DISPATCH);
        let net_ptr = core::ptr::read_unaligned(v_net.data.as_ptr() as *const *mut core::ffi::c_void);
        assert!(!net_ptr.is_null());

        let name_user = sys_alloc_string("UserName");
        let mut ret_u = Variant::empty();
        dispatch_invoke_member(net_ptr, name_user as *const u16, DISPATCH_PROPERTYGET, core::ptr::null_mut(), 0, &mut ret_u);
        assert_eq!(ret_u.vt, VT_BSTR);
        let user = bstr_to_string(core::ptr::read_unaligned(ret_u.data.as_ptr() as *const Bstr));
        assert!(!user.is_empty());
        SysFreeString(name_user);
        SysFreeString(s_net);
        com_release(net_ptr);
    }
}

#[test]
fn test_adodb_recordset_lifecycle_and_fields() {
    unsafe {
        // Instantiate ADODB.Recordset
        let s_rs = sys_alloc_string("ADODB.Recordset");
        let mut v_rs = Variant::empty();
        rtcCreateObject2(&mut v_rs, s_rs, core::ptr::null_mut());
        assert_eq!(v_rs.vt, VT_DISPATCH);
        let rs_ptr = core::ptr::read_unaligned(v_rs.data.as_ptr() as *const *mut core::ffi::c_void);
        assert!(!rs_ptr.is_null());

        // Get Fields collection
        let name_fields = sys_alloc_string("Fields");
        let mut ret_fields = Variant::empty();
        dispatch_invoke_member(rs_ptr, name_fields as *const u16, DISPATCH_PROPERTYGET, core::ptr::null_mut(), 0, &mut ret_fields);
        let fields_ptr = core::ptr::read_unaligned(ret_fields.data.as_ptr() as *const *mut core::ffi::c_void);
        assert!(!fields_ptr.is_null());
        SysFreeString(name_fields);

        // Fields.Append "EmpID", 3 (adInteger)
        let s_empid = sys_alloc_string("EmpID");
        let mut v_id = Variant::empty();
        v_id.vt = VT_BSTR;
        v_id.data[..core::mem::size_of::<usize>()].copy_from_slice(&(s_empid as usize).to_ne_bytes());
        let mut v_type = Variant::empty();
        v_type.vt = VT_I4;
        let three: i32 = 3;
        v_type.data[0..4].copy_from_slice(&three.to_ne_bytes());
        let mut args_app1 = [v_type, v_id];
        let name_app = sys_alloc_string("Append");
        dispatch_invoke_member(fields_ptr, name_app as *const u16, DISPATCH_METHOD, args_app1.as_mut_ptr(), 2, core::ptr::null_mut());

        // Fields.Append "EmpName", 200, 50
        let s_empname = sys_alloc_string("EmpName");
        let mut v_name = Variant::empty();
        v_name.vt = VT_BSTR;
        v_name.data[..core::mem::size_of::<usize>()].copy_from_slice(&(s_empname as usize).to_ne_bytes());
        let mut v_type2 = Variant::empty();
        v_type2.vt = VT_I4;
        let t200: i32 = 200;
        v_type2.data[0..4].copy_from_slice(&t200.to_ne_bytes());
        let mut v_size = Variant::empty();
        v_size.vt = VT_I4;
        let s50: i32 = 50;
        v_size.data[0..4].copy_from_slice(&s50.to_ne_bytes());
        let mut args_app2 = [v_size, v_type2, v_name];
        dispatch_invoke_member(fields_ptr, name_app as *const u16, DISPATCH_METHOD, args_app2.as_mut_ptr(), 3, core::ptr::null_mut());
        SysFreeString(name_app);

        // Fields.Count
        let name_count = sys_alloc_string("Count");
        let mut ret_count = Variant::empty();
        dispatch_invoke_member(fields_ptr, name_count as *const u16, DISPATCH_PROPERTYGET, core::ptr::null_mut(), 0, &mut ret_count);
        assert_eq!(core::ptr::read_unaligned(ret_count.data.as_ptr() as *const i32), 2);
        SysFreeString(name_count);

        // Open
        let name_open = sys_alloc_string("Open");
        dispatch_invoke_member(rs_ptr, name_open as *const u16, DISPATCH_METHOD, core::ptr::null_mut(), 0, core::ptr::null_mut());
        SysFreeString(name_open);

        // AddNew
        let name_addnew = sys_alloc_string("AddNew");
        dispatch_invoke_member(rs_ptr, name_addnew as *const u16, DISPATCH_METHOD, core::ptr::null_mut(), 0, core::ptr::null_mut());
        SysFreeString(name_addnew);

        // Set field value: Fields("EmpID").Value = 42
        let mut args_fid = [v_id];
        let name_item = sys_alloc_string("Item");
        let mut ret_fld = Variant::empty();
        dispatch_invoke_member(fields_ptr, name_item as *const u16, DISPATCH_METHOD, args_fid.as_mut_ptr(), 1, &mut ret_fld);
        let fld_ptr = core::ptr::read_unaligned(ret_fld.data.as_ptr() as *const *mut core::ffi::c_void);
        assert!(!fld_ptr.is_null());
        SysFreeString(name_item);

        let mut v_val = Variant::empty();
        v_val.vt = VT_I4;
        let val42: i32 = 42;
        v_val.data[0..4].copy_from_slice(&val42.to_ne_bytes());
        let mut args_val = [v_val];
        let name_val = sys_alloc_string("Value");
        dispatch_invoke_member(fld_ptr, name_val as *const u16, DISPATCH_PROPERTYPUT, args_val.as_mut_ptr(), 1, core::ptr::null_mut());

        // Update
        let name_upd = sys_alloc_string("Update");
        dispatch_invoke_member(rs_ptr, name_upd as *const u16, DISPATCH_METHOD, core::ptr::null_mut(), 0, core::ptr::null_mut());
        SysFreeString(name_upd);

        // RecordCount
        let name_rc = sys_alloc_string("RecordCount");
        let mut ret_rc = Variant::empty();
        dispatch_invoke_member(rs_ptr, name_rc as *const u16, DISPATCH_PROPERTYGET, core::ptr::null_mut(), 0, &mut ret_rc);
        assert_eq!(core::ptr::read_unaligned(ret_rc.data.as_ptr() as *const i32), 1);
        SysFreeString(name_rc);

        // Value read back
        let mut ret_val_read = Variant::empty();
        dispatch_invoke_member(fld_ptr, name_val as *const u16, DISPATCH_PROPERTYGET, core::ptr::null_mut(), 0, &mut ret_val_read);
        assert_eq!(ret_val_read.vt, VT_I4);
        assert_eq!(core::ptr::read_unaligned(ret_val_read.data.as_ptr() as *const i32), 42);
        SysFreeString(name_val);
        com_release(fld_ptr);

        // Close
        let name_close = sys_alloc_string("Close");
        dispatch_invoke_member(rs_ptr, name_close as *const u16, DISPATCH_METHOD, core::ptr::null_mut(), 0, core::ptr::null_mut());
        SysFreeString(name_close);

        SysFreeString(s_empid);
        SysFreeString(s_empname);
        SysFreeString(s_rs);
        com_release(fields_ptr);
        com_release(rs_ptr);
    }
}

#[test]
fn test_rtc_get_object() {
    unsafe {
        // 1. Success with class name
        let s_cls = sys_alloc_string("Scripting.Dictionary");
        let mut v_cls = Variant::empty();
        v_cls.vt = VT_BSTR;
        v_cls.data[..core::mem::size_of::<usize>()].copy_from_slice(&(s_cls as usize).to_ne_bytes());
        let mut res = Variant::empty();
        rtcGetObject(&mut res, core::ptr::null_mut(), &mut v_cls);
        assert_eq!(res.vt, VT_DISPATCH);
        let ptr = core::ptr::read_unaligned(res.data.as_ptr() as *const *mut core::ffi::c_void);
        assert!(!ptr.is_null());
        com_release(ptr);
        SysFreeString(s_cls);

        // 2. Error 449 on empty arguments
        msvbvm60::err::clear_vb_error();
        let mut res2 = Variant::empty();
        rtcGetObject(&mut res2, core::ptr::null_mut(), core::ptr::null_mut());
        assert_eq!(msvbvm60::err::get_vb_error_number(), 449);

        // 3. Error 432 on file path without registered handler
        msvbvm60::err::clear_vb_error();
        let s_path = sys_alloc_string("non_existent_file.xyz");
        let mut v_path = Variant::empty();
        v_path.vt = VT_BSTR;
        v_path.data[..core::mem::size_of::<usize>()].copy_from_slice(&(s_path as usize).to_ne_bytes());
        let mut res3 = Variant::empty();
        rtcGetObject(&mut res3, &mut v_path, core::ptr::null_mut());
        assert_eq!(msvbvm60::err::get_vb_error_number(), 432);
        SysFreeString(s_path);
        msvbvm60::err::clear_vb_error();
    }
}
