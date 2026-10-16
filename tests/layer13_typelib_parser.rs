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

use msvbvm60::err::*;
use msvbvm60::typelib::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;
fn guid_to_bytes(g: &Guid) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[0..4].copy_from_slice(&g.data1.to_le_bytes());
    b[4..6].copy_from_slice(&g.data2.to_le_bytes());
    b[6..8].copy_from_slice(&g.data3.to_le_bytes());
    b[8..16].copy_from_slice(&g.data4);
    b
}

fn build_synthetic_msft_typelib_bytes() -> Vec<u8> {
    let mut data = Vec::new();

    // 1. Header (84 bytes)
    data.extend_from_slice(&MSFT_MAGIC.to_le_bytes()); // 0..4: "MSFT"
    data.extend_from_slice(&0x00010002u32.to_le_bytes()); // 4..8: version
    data.extend_from_slice(&0i32.to_le_bytes()); // 8..12: posguid (GUID 0 at offset 0)
    data.extend_from_slice(&0x0409u32.to_le_bytes()); // 12..16: lcid (en-US)
    data.extend_from_slice(&0u32.to_le_bytes()); // 16..20: lcid2
    data.extend_from_slice(&0u32.to_le_bytes()); // 20..24: varflags (0 -> hdr_len=84)
    data.extend_from_slice(&0x00000001u32.to_le_bytes()); // 24..28: version (1.0)
    data.extend_from_slice(&0u32.to_le_bytes()); // 28..32: flags
    data.extend_from_slice(&3u32.to_le_bytes()); // 32..36: nrtypeinfos = 3
    data.extend_from_slice(&(-1i32).to_le_bytes()); // 36..40: helpstring
    data.extend_from_slice(&0u32.to_le_bytes()); // 40..44: helpstringcontext
    data.extend_from_slice(&0u32.to_le_bytes()); // 44..48: helpcontext
    data.extend_from_slice(&0u32.to_le_bytes()); // 48..52: nametable count
    data.extend_from_slice(&0u32.to_le_bytes()); // 52..56: nametable chars
    data.extend_from_slice(&0i32.to_le_bytes()); // 56..60: name_offset in nametab (0 = "Scripting")
    data.extend_from_slice(&(-1i32).to_le_bytes()); // 60..64: helpfile
    data.extend_from_slice(&(-1i32).to_le_bytes()); // 64..68: custom data
    data.extend_from_slice(&[0u8; 16]); // 68..84: reserved (4 * 4 bytes)

    // Typeinfo offsets in header: nrtypeinfos * 4 = 3 * 4 = 12 bytes
    data.extend_from_slice(&0i32.to_le_bytes()); // TypeInfo 0
    data.extend_from_slice(&100i32.to_le_bytes()); // TypeInfo 1
    data.extend_from_slice(&200i32.to_le_bytes()); // TypeInfo 2

    // 2. SegDir: 15 segments of 16 bytes = 240 bytes
    // Total header + segdir = 84 + 12 + 240 = 336 bytes.
    let typeinfo_off = 336i32;
    let typeinfo_len = 300i32; // 3 * 100 bytes
    let guidtab_off = typeinfo_off + typeinfo_len; // 636
    let guidtab_len = 64i32; // 4 * 16 bytes
    let nametab_off = guidtab_off + guidtab_len; // 700
    let nametab_len = 400i32;
    let mem_off = nametab_off + nametab_len; // 1100

    let mut segs = vec![(0i32, 0i32); 15];
    segs[0] = (typeinfo_off, typeinfo_len); // pTypeInfoTab
    segs[5] = (guidtab_off, guidtab_len); // pGuidTab
    segs[7] = (nametab_off, nametab_len); // pNameTab

    for (off, len) in segs {
        data.extend_from_slice(&off.to_le_bytes());
        data.extend_from_slice(&len.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes()); // res1
        data.extend_from_slice(&0i32.to_le_bytes()); // res2
    }

    assert_eq!(data.len(), 336);

    // 3. TypeInfoTab (3 * 100 bytes = 300 bytes)
    // TypeInfo 0: "IDictionary" (Dispatch, IID_IDictionary, 10 funcs)
    {
        let mut ti = vec![0u8; 100];
        ti[0..4].copy_from_slice(&4u32.to_le_bytes()); // typekind = Dispatch (4)
        ti[4..8].copy_from_slice(&mem_off.to_le_bytes()); // memoffset
        ti[24..28].copy_from_slice(&10u32.to_le_bytes()); // c_funcs = 10, c_vars = 0
        ti[44..48].copy_from_slice(&16i32.to_le_bytes()); // t_posguid = 16 (GUID 1)
        ti[52..56].copy_from_slice(&24i32.to_le_bytes()); // t_name_offset = 24 ("IDictionary")
        ti[76..78].copy_from_slice(&1u16.to_le_bytes()); // c_impl_types
        ti[78..80].copy_from_slice(&28u16.to_le_bytes()); // cb_size_vft
        data.extend_from_slice(&ti);
    }

    // TypeInfo 1: "Dictionary" (CoClass, CLSID_Dictionary)
    {
        let mut ti = vec![0u8; 100];
        ti[0..4].copy_from_slice(&5u32.to_le_bytes()); // typekind = CoClass (5)
        ti[4..8].copy_from_slice(&(-1i32).to_le_bytes()); // memoffset
        ti[24..28].copy_from_slice(&0u32.to_le_bytes()); // c_funcs = 0
        ti[44..48].copy_from_slice(&32i32.to_le_bytes()); // t_posguid = 32 (GUID 2)
        ti[52..56].copy_from_slice(&48i32.to_le_bytes()); // t_name_offset = 48 ("Dictionary")
        data.extend_from_slice(&ti);
    }

    // TypeInfo 2: "FileSystemObject" (CoClass, CLSID_FSO)
    {
        let mut ti = vec![0u8; 100];
        ti[0..4].copy_from_slice(&5u32.to_le_bytes()); // typekind = CoClass (5)
        ti[4..8].copy_from_slice(&(-1i32).to_le_bytes()); // memoffset
        ti[24..28].copy_from_slice(&0u32.to_le_bytes()); // c_funcs = 0
        ti[44..48].copy_from_slice(&48i32.to_le_bytes()); // t_posguid = 48 (GUID 3)
        ti[52..56].copy_from_slice(&72i32.to_le_bytes()); // t_name_offset = 72 ("FileSystemObject")
        data.extend_from_slice(&ti);
    }

    assert_eq!(data.len(), 636);

    // 4. GuidTab (4 * 16 bytes = 64 bytes)
    // GUID 0: Lib GUID {420B2830-E718-11CF-893D-00A0C9054228}
    let lib_guid = Guid::parse("420B2830-E718-11CF-893D-00A0C9054228").unwrap();
    data.extend_from_slice(&guid_to_bytes(&lib_guid));

    // GUID 1: IID_IDictionary {420B2832-E718-11CF-893D-00A0C9054228}
    data.extend_from_slice(&guid_to_bytes(&IID_IDictionary));

    // GUID 2: CLSID_Dictionary {EE09B103-97E0-11CF-978F-00A02463E06F}
    data.extend_from_slice(&guid_to_bytes(&CLSID_Dictionary));

    // GUID 3: CLSID_FSO {0D43FE01-F093-11CF-8940-00A0C9054228}
    let fso_guid = Guid::parse("0D43FE01-F093-11CF-8940-00A0C9054228").unwrap();
    data.extend_from_slice(&guid_to_bytes(&fso_guid));

    assert_eq!(data.len(), 700);

    // 5. NameTab (strings with 8-byte prefix + 1-byte len + 3-byte pad + string + alignment)
    fn add_name(buf: &mut Vec<u8>, s: &str, target_size: usize) {
        let start = buf.len();
        buf.extend_from_slice(&[0u8; 8]); // prefix
        buf.push(s.len() as u8); // len
        buf.extend_from_slice(&[0u8; 3]); // pad
        buf.extend_from_slice(s.as_bytes()); // string
        while buf.len() - start < target_size {
            buf.push(0);
        }
    }

    add_name(&mut data, "Scripting", 24); // off 0
    add_name(&mut data, "IDictionary", 24); // off 24
    add_name(&mut data, "Dictionary", 24); // off 48
    add_name(&mut data, "FileSystemObject", 32); // off 72
    add_name(&mut data, "Add", 16); // off 104
    add_name(&mut data, "Count", 20); // off 120
    add_name(&mut data, "Exists", 20); // off 140
    add_name(&mut data, "Remove", 20); // off 160
    add_name(&mut data, "Items", 20); // off 180
    add_name(&mut data, "Key", 16); // off 200
    add_name(&mut data, "Keys", 20); // off 216
    add_name(&mut data, "RemoveAll", 24); // off 236
    add_name(&mut data, "CompareMode", 24); // off 260
    add_name(&mut data, "Item", 20); // off 284

    // Pad nametab to mem_off (1100)
    while data.len() < mem_off as usize {
        data.push(0);
    }

    assert_eq!(data.len(), 1100);

    // 6. Function table for IDictionary at mem_off (1100)
    let num_funcs = 10;
    let rec_size = 24;
    let dispid_table_rel = 4 + (num_funcs * rec_size); // 4 + 240 = 244
    data.extend_from_slice(&(dispid_table_rel as i32).to_le_bytes());

    // 10 function records (24 bytes each)
    let func_defs: Vec<(i32, i32, u16, u32, u16)> = vec![
        (1, 104, 0, 1, 2),  // Add: memid 1, name "Add", vt 0, inv Func (1), 2 params
        (2, 120, 4, 2, 0),  // Count: memid 2, name "Count", vt 4, inv PropGet (2), 0 params
        (3, 140, 8, 1, 2),  // Exists: memid 3, name "Exists", vt 8, inv Func (1), 2 params
        (4, 180, 12, 2, 0), // Items: memid 4, name "Items", vt 12, inv PropGet (2), 0 params
        (5, 200, 16, 4, 2), // Key: memid 5, name "Key", vt 16, inv PropPut (4), 2 params
        (6, 216, 20, 2, 0), // Keys: memid 6, name "Keys", vt 20, inv PropGet (2), 0 params
        (7, 160, 24, 1, 1), // Remove: memid 7, name "Remove", vt 24, inv Func (1), 1 param
        (8, 236, 28, 1, 0), // RemoveAll: memid 8, name "RemoveAll", vt 28, inv Func (1), 0 params
        (9, 260, 32, 2, 0), // CompareMode: memid 9, name "CompareMode", vt 32, inv PropGet (2), 0 params
        (0, 284, 36, 2, 1), // Item: memid 0, name "Item", vt 36, inv PropGet (2), 1 param
    ];

    for (_memid, _name_off, vt_off, inv_kind, params) in &func_defs {
        let mut rec = vec![0u8; 24];
        rec[0..4].copy_from_slice(&(rec_size as u32).to_le_bytes()); // rec_size
        rec[12..16].copy_from_slice(&(*vt_off as u32).to_le_bytes()); // vtable_offset
        let flags = (*inv_kind as u32) << 3;
        rec[16..20].copy_from_slice(&flags.to_le_bytes()); // inv_kind
        rec[20..24].copy_from_slice(&(*params as u32).to_le_bytes()); // c_params
        data.extend_from_slice(&rec);
    }

    // DISPID and Name Offsets table at d_pos = 1100 + 244
    data.extend_from_slice(&0u32.to_le_bytes()); // padding header

    // DISPIDs array (10 * 4 bytes)
    for (memid, _, _, _, _) in &func_defs {
        data.extend_from_slice(&memid.to_le_bytes());
    }

    // Name offsets array (10 * 4 bytes)
    for (_, name_off, _, _, _) in &func_defs {
        data.extend_from_slice(&name_off.to_le_bytes());
    }

    data
}

#[test]
fn test_parse_msft_typelib_synthetic() {
    let msft_bytes = build_synthetic_msft_typelib_bytes();
    let typelib = parse_msft_bytes(&msft_bytes)
        .expect("Failed to parse clean-room synthetic MSFT TypeLib");

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
    assert_eq!(dict_info.c_funcs, 10);

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
    assert_eq!(remove_func.c_params, 1);

    // Also test PE TypeLib extraction on synthetic binary
    let mut pe_mock = vec![0u8; 1024];
    pe_mock[0..2].copy_from_slice(b"MZ");
    pe_mock[256..264].copy_from_slice(&[0x4D, 0x53, 0x46, 0x54, 0x02, 0x00, 0x01, 0x00]);
    pe_mock.extend_from_slice(&msft_bytes[8..]); // remainder of MSFT
    let extracted = extract_typelib_bytes_from_pe(&pe_mock)
        .expect("Failed to extract synthetic typelib from PE mock");
    assert_eq!(&extracted[0..4], b"MSFT");
}

#[test]
fn test_dispid_cache_lookups() {
    let msft_bytes = build_synthetic_msft_typelib_bytes();
    let typelib = parse_msft_bytes(&msft_bytes).expect("Failed to parse synthetic typelib");

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
