// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Regression Suite v0.2.5 (Phase 2.5): ActiveX Consolidation & Self-Describing ITypeInfo Tests
//!
//! Verifies:
//! - COM `ITypeInfo` self-describing introspection on standard ActiveX components
//! - Dynamic member invocation via `rtcCallByName` (Ordinal 715)
//! - `TypeName` reflection backed by `ITypeInfo::GetDocumentation(-1)`
//! - Error trapping semantics on unknown properties/methods (Error 438)

use msvbvm60::err::*;
use msvbvm60::rtc::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;

#[test]
fn test_self_describing_type_info_introspection() {
    unsafe {
        // Test Dictionary GetTypeInfo
        let s_progid = sys_alloc_string("Scripting.Dictionary");
        let mut v_dict = Variant::empty();
        rtcCreateObject2(&mut v_dict, s_progid, core::ptr::null_mut());
        assert_eq!(v_dict.vt, VT_DISPATCH);
        let dict_ptr = core::ptr::read_unaligned(v_dict.data.as_ptr() as *const *mut core::ffi::c_void);
        assert!(!dict_ptr.is_null());

        let vtable = *(dict_ptr as *const *const usize);
        let get_ti_fn: unsafe extern "system" fn(
            *mut core::ffi::c_void,
            u32,
            u32,
            *mut *mut core::ffi::c_void,
        ) -> HResult = core::mem::transmute(*vtable.add(4));

        let mut p_ti: *mut core::ffi::c_void = core::ptr::null_mut();
        let hr = get_ti_fn(dict_ptr, 0, 0x0409, &mut p_ti);
        assert_eq!(hr, S_OK);
        assert!(!p_ti.is_null());

        let ti_vtbl = *(p_ti as *const *const usize);
        let get_doc_fn: unsafe extern "system" fn(
            *mut core::ffi::c_void,
            i32,
            *mut Bstr,
            *mut Bstr,
            *mut u32,
            *mut Bstr,
        ) -> HResult = core::mem::transmute(*ti_vtbl.add(12));

        // Introspect coclass name (MEMBERID_NIL = -1)
        let mut name_bstr: Bstr = core::ptr::null_mut();
        let hr_doc = get_doc_fn(
            p_ti,
            -1,
            &mut name_bstr,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        );
        assert_eq!(hr_doc, S_OK);
        assert_eq!(bstr_to_string(name_bstr), "Dictionary");
        SysFreeString(name_bstr);

        // Introspect first member name (memid = 0)
        let mut member_bstr: Bstr = core::ptr::null_mut();
        let hr_mem = get_doc_fn(
            p_ti,
            0,
            &mut member_bstr,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        );
        assert_eq!(hr_mem, S_OK);
        assert_eq!(bstr_to_string(member_bstr), "Add");
        SysFreeString(member_bstr);

        let release_fn: unsafe extern "system" fn(*mut core::ffi::c_void) -> u32 =
            core::mem::transmute(*ti_vtbl.add(2));
        release_fn(p_ti);

        SysFreeString(s_progid);
        __vbaFreeVar(&mut v_dict);
    }
}

#[test]
fn test_rtc_type_name_uses_type_info_reflection() {
    unsafe {
        // 1. FileSystemObject
        let s_fso = sys_alloc_string("Scripting.FileSystemObject");
        let mut v_fso = Variant::empty();
        rtcCreateObject2(&mut v_fso, s_fso, core::ptr::null_mut());
        let tn_fso = rtcTypeName(&v_fso);
        assert_eq!(bstr_to_string(tn_fso), "FileSystemObject");
        SysFreeString(tn_fso);
        SysFreeString(s_fso);
        __vbaFreeVar(&mut v_fso);

        // 2. WScript.Shell
        let s_wsh = sys_alloc_string("WScript.Shell");
        let mut v_wsh = Variant::empty();
        rtcCreateObject2(&mut v_wsh, s_wsh, core::ptr::null_mut());
        let tn_wsh = rtcTypeName(&v_wsh);
        assert_eq!(bstr_to_string(tn_wsh), "WshShell");
        SysFreeString(tn_wsh);
        SysFreeString(s_wsh);
        __vbaFreeVar(&mut v_wsh);

        // 3. WScript.Network
        let s_net = sys_alloc_string("WScript.Network");
        let mut v_net = Variant::empty();
        rtcCreateObject2(&mut v_net, s_net, core::ptr::null_mut());
        let tn_net = rtcTypeName(&v_net);
        assert_eq!(bstr_to_string(tn_net), "WshNetwork");
        SysFreeString(tn_net);
        SysFreeString(s_net);
        __vbaFreeVar(&mut v_net);

        // 4. ADODB.Recordset
        let s_rs = sys_alloc_string("ADODB.Recordset");
        let mut v_rs = Variant::empty();
        rtcCreateObject2(&mut v_rs, s_rs, core::ptr::null_mut());
        let tn_rs = rtcTypeName(&v_rs);
        assert_eq!(bstr_to_string(tn_rs), "Recordset");
        SysFreeString(tn_rs);
        SysFreeString(s_rs);
        __vbaFreeVar(&mut v_rs);

        // 5. Scripting.Dictionary
        let s_dict = sys_alloc_string("Scripting.Dictionary");
        let mut v_dict = Variant::empty();
        rtcCreateObject2(&mut v_dict, s_dict, core::ptr::null_mut());
        let tn_dict = rtcTypeName(&v_dict);
        assert_eq!(bstr_to_string(tn_dict), "Dictionary");
        SysFreeString(tn_dict);
        SysFreeString(s_dict);
        __vbaFreeVar(&mut v_dict);

        // 6. Nothing
        let mut v_nothing = Variant::empty();
        v_nothing.vt = VT_DISPATCH;
        let tn_nothing = rtcTypeName(&v_nothing);
        assert_eq!(bstr_to_string(tn_nothing), "Nothing");
        SysFreeString(tn_nothing);
    }
}

#[test]
fn test_rtc_call_by_name_methods_and_properties() {
    unsafe {
        // Instantiate FSO
        let s_fso = sys_alloc_string("Scripting.FileSystemObject");
        let mut v_fso = Variant::empty();
        rtcCreateObject2(&mut v_fso, s_fso, core::ptr::null_mut());
        let p_fso = __vbaObjVar(&v_fso);
        assert!(!p_fso.is_null());

        // Invoke CallByName(fso, "BuildPath", VbMethod, "C:\\Test", "file.txt")
        let s_method = sys_alloc_string("BuildPath");
        let s_p = sys_alloc_string("C:\\Test");
        let s_f = sys_alloc_string("file.txt");
        let v_p = Variant::from_bstr(s_p);
        let v_f = Variant::from_bstr(s_f);

        let sa = SafeArray::create_vector(
            core::mem::size_of::<Variant>() as u32,
            0,
            2,
            FADF_VARIANT | FADF_HAVEVARTYPE,
        );
        let slice = core::slice::from_raw_parts_mut((*sa).pv_data as *mut Variant, 2);
        slice[0] = v_p;
        slice[1] = v_f;
        let mut sa_ptr = sa;

        let mut res = Variant::empty();
        rtcCallByName(&mut res, p_fso, s_method, 1, &mut sa_ptr, 0x0409);
        assert_eq!(res.vt, VT_BSTR);
        let bstr_res = core::ptr::read_unaligned(res.data.as_ptr() as *const Bstr);
        assert_eq!(bstr_to_string(bstr_res), "C:\\Test\\file.txt");

        SysFreeString(s_method);
        SysFreeString(s_fso);
        __vbaFreeVar(&mut res);
        __vbaErase(&mut sa_ptr, 0);
        __vbaFreeVar(&mut v_fso);

        // Instantiate Dictionary
        let s_dict = sys_alloc_string("Scripting.Dictionary");
        let mut v_dict = Variant::empty();
        rtcCreateObject2(&mut v_dict, s_dict, core::ptr::null_mut());
        let p_dict = __vbaObjVar(&v_dict);
        assert!(!p_dict.is_null());

        // CallByName dict, "Add", VbMethod, "Key1", "Val1"
        let s_add = sys_alloc_string("Add");
        let s_k1 = sys_alloc_string("Key1");
        let s_v1 = sys_alloc_string("Val1");
        let v_k = Variant::from_bstr(s_k1);
        let v_v = Variant::from_bstr(s_v1);

        let sa_add = SafeArray::create_vector(
            core::mem::size_of::<Variant>() as u32,
            0,
            2,
            FADF_VARIANT | FADF_HAVEVARTYPE,
        );
        let slice_add = core::slice::from_raw_parts_mut((*sa_add).pv_data as *mut Variant, 2);
        slice_add[0] = v_k;
        slice_add[1] = v_v;
        let mut sa_add_ptr = sa_add;

        let mut res_add = Variant::empty();
        rtcCallByName(&mut res_add, p_dict, s_add, 1, &mut sa_add_ptr, 0x0409);
        SysFreeString(s_add);
        __vbaFreeVar(&mut res_add);
        __vbaErase(&mut sa_add_ptr, 0);

        // CallByName dict, "Item", VbGet, "Key1"
        let s_item = sys_alloc_string("Item");
        let s_k_get = sys_alloc_string("Key1");
        let v_k_get = Variant::from_bstr(s_k_get);

        let sa_get = SafeArray::create_vector(
            core::mem::size_of::<Variant>() as u32,
            0,
            1,
            FADF_VARIANT | FADF_HAVEVARTYPE,
        );
        let slice_get = core::slice::from_raw_parts_mut((*sa_get).pv_data as *mut Variant, 1);
        slice_get[0] = v_k_get;
        let mut sa_get_ptr = sa_get;

        let mut res_item = Variant::empty();
        rtcCallByName(&mut res_item, p_dict, s_item, 2, &mut sa_get_ptr, 0x0409);
        assert_eq!(res_item.vt, VT_BSTR);
        let bstr_item = core::ptr::read_unaligned(res_item.data.as_ptr() as *const Bstr);
        assert_eq!(bstr_to_string(bstr_item), "Val1");

        SysFreeString(s_item);
        SysFreeString(s_dict);
        __vbaFreeVar(&mut res_item);
        __vbaErase(&mut sa_get_ptr, 0);
        __vbaFreeVar(&mut v_dict);
    }
}

#[test]
fn test_rtc_call_by_name_error_handling() {
    unsafe {
        clear_vb_error();

        // 1. Null Object -> Err 91
        let mut res = Variant::empty();
        let s_meth = sys_alloc_string("SomeMethod");
        rtcCallByName(
            &mut res,
            core::ptr::null_mut(),
            s_meth,
            1,
            core::ptr::null_mut(),
            0x0409,
        );
        assert_eq!(get_vb_error_number(), 91);
        clear_vb_error();

        // 2. Null ProcName -> Err 5
        let s_fso = sys_alloc_string("Scripting.FileSystemObject");
        let mut v_fso = Variant::empty();
        rtcCreateObject2(&mut v_fso, s_fso, core::ptr::null_mut());
        let p_fso = __vbaObjVar(&v_fso);

        rtcCallByName(
            &mut res,
            p_fso,
            core::ptr::null_mut(),
            1,
            core::ptr::null_mut(),
            0x0409,
        );
        assert_eq!(get_vb_error_number(), 5);
        clear_vb_error();

        // 3. Unknown method -> Err 438
        let s_bad = sys_alloc_string("NonExistentMethod");
        rtcCallByName(
            &mut res,
            p_fso,
            s_bad,
            1,
            core::ptr::null_mut(),
            0x0409,
        );
        assert_eq!(get_vb_error_number(), 438);
        clear_vb_error();

        SysFreeString(s_meth);
        SysFreeString(s_bad);
        SysFreeString(s_fso);
        __vbaFreeVar(&mut v_fso);
    }
}
