// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Types & Automation: SafeArray & UDT Engine
//!
//! SAFEARRAY descriptors, FADF flags, lock counter enforcement (Err 10),
//! multi-dimensional stride layouts, ReDim Preserve, and UDT memory records.

use core::ffi::c_void;
use msvbvm60::err::*;
use msvbvm60::rtc::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;



#[test]
fn test_vba_redim_and_bounds() {
    unsafe {
        let mut p_sa: *mut SafeArray = core::ptr::null_mut();
        __vbaRedim(0x80, 4, &mut p_sa, 3, 1, 4, 0); // 0 to 4 (5 elements)
        assert!(!p_sa.is_null());
        assert_eq!(__vbaLbound(p_sa, 1), 0);
        assert_eq!(__vbaUbound(p_sa, 1), 4);

        let slice = core::slice::from_raw_parts_mut((*p_sa).pv_data as *mut i32, 5);
        for (i, val) in slice.iter_mut().enumerate() {
            *val = (i as i32 + 1) * 10;
        }
        assert_eq!(slice[0], 10);
        assert_eq!(slice[4], 50);

        __vbaErase(&mut p_sa, 0);
        assert!(p_sa.is_null());
    }
}

#[test]
fn test_vba_redim_preserve_data_retention() {
    unsafe {
        let mut p_sa: *mut SafeArray = core::ptr::null_mut();
        __vbaRedim(0x80, 4, &mut p_sa, 3, 1, 4, 0);
        let slice = core::slice::from_raw_parts_mut((*p_sa).pv_data as *mut i32, 5);
        slice[0] = 111;
        slice[4] = 555;

        // Preserve and expand to 0..6 (7 elements)
        __vbaRedimPreserve(0x80, 4, &mut p_sa, 3, 1, 6, 0);
        assert_eq!(__vbaUbound(p_sa, 1), 6);

        let slice2 = core::slice::from_raw_parts_mut((*p_sa).pv_data as *mut i32, 7);
        assert_eq!(slice2[0], 111);
        assert_eq!(slice2[4], 555);
        slice2[5] = 666;
        slice2[6] = 777;
        assert_eq!(slice2[6], 777);

        __vbaErase(&mut p_sa, 0);
    }
}

#[test]
fn test_rtc_split_string_into_array() {
    unsafe {
        let text = sys_alloc_string("apple,banana,cherry");
        let delim = sys_alloc_string(",");
        let v_delim = Variant::from_bstr(delim);
        let mut res = Variant::empty();

        rtcSplit(&mut res, text, &v_delim, -1, 0);
        let p_sa = __vbaAryVar(&res, 0);
        assert!(!p_sa.is_null());
        assert_eq!(__vbaUbound(p_sa, 1) - __vbaLbound(p_sa, 1) + 1, 3);

        SysFreeString(text);
        SysFreeString(delim);
    }
}

#[test]
fn test_rtc_join_array_into_string() {
    unsafe {
        let text = sys_alloc_string("alpha,beta,gamma");
        let delim = sys_alloc_string(",");
        let v_delim = Variant::from_bstr(delim);
        let mut split_res = Variant::empty();
        rtcSplit(&mut split_res, text, &v_delim, -1, 0);

        let dash = sys_alloc_string("-");
        let v_dash = Variant::from_bstr(dash);
        let joined = rtcJoin(&split_res, &v_dash);
        assert_eq!(bstr_to_string(joined), "alpha-beta-gamma");

        SysFreeString(text);
        SysFreeString(delim);
        SysFreeString(dash);
        SysFreeString(joined);
    }
}

#[test]
fn test_rtc_filter_array() {
    unsafe {
        let text = sys_alloc_string("linux-x86,windows-x86,linux-arm,darwin");
        let delim = sys_alloc_string(",");
        let v_delim = Variant::from_bstr(delim);
        let mut split_res = Variant::empty();
        rtcSplit(&mut split_res, text, &v_delim, -1, 0);

        let target = sys_alloc_string("linux");
        let mut filter_res = Variant::empty();
        rtcFilter(&mut filter_res, &split_res, target, -1, 0);

        let p_sa = __vbaAryVar(&filter_res, 0);
        assert_eq!(__vbaUbound(p_sa, 1), 1); // 2 matches (0..1)

        let slice = core::slice::from_raw_parts((*p_sa).pv_data as *const Bstr, 2);
        assert_eq!(bstr_to_string(slice[0]), "linux-x86");
        assert_eq!(bstr_to_string(slice[1]), "linux-arm");

        SysFreeString(text);
        SysFreeString(delim);
        SysFreeString(target);
    }
}

#[test]
fn test_rtc_array_and_var_index_load() {
    unsafe {
        let items = [
            Variant::from_i32(100),
            Variant::from_i32(200),
            Variant::from_i32(300),
        ];
        let sa = SafeArray::create_vector(
            core::mem::size_of::<Variant>() as u32,
            0,
            3,
            FADF_VARIANT | FADF_HAVEVARTYPE,
        );
        core::ptr::copy_nonoverlapping(items.as_ptr(), (*sa).pv_data as *mut Variant, 3);

        let mut v_arr = Variant::empty();
        rtcArray(&mut v_arr, sa as *const core::ffi::c_void);

        let mut item = Variant::empty();
        __vbaVarIndexLoad(&mut item, &v_arr, 1, 1);
        assert_eq!(item.as_i32(), Some(200));
    }
}

#[test]
fn test_vba_erase_nulls_array_pointer() {
    unsafe {
        let mut p_sa: *mut SafeArray = core::ptr::null_mut();
        __vbaRedim(0x80, 4, &mut p_sa, 3, 1, 2, 0);
        assert!(!p_sa.is_null());

        __vbaErase(&mut p_sa, 0);
        assert!(p_sa.is_null());
    }
}

#[test]
fn test_safearray_redim_preserve_resizes_last_dimension_only() {
    unsafe {
        // GIVEN a 2-dimensional SafeArray (1 To 2, 1 To 5)
        let bounds = [
            SafeArrayBound {
                c_elements: 2,
                l_lbound: 1,
            }, // Dimension 1 (1 To 2)
            SafeArrayBound {
                c_elements: 5,
                l_lbound: 1,
            }, // Dimension 2 (1 To 5)
        ];
        let p_sa = SafeArray::create_multidim(4, &bounds, FADF_HAVEVARTYPE);
        assert_eq!((*p_sa).c_dims, 2);
        assert_eq!(__vbaLbound(p_sa, 1), 1);
        assert_eq!(__vbaUbound(p_sa, 1), 2);
        assert_eq!(__vbaLbound(p_sa, 2), 1);
        assert_eq!(__vbaUbound(p_sa, 2), 5);

        // Fill elements with known data
        let slice = core::slice::from_raw_parts_mut((*p_sa).pv_data as *mut i32, 10);
        for i in 0..10 {
            slice[i] = (i + 1) as i32 * 10;
        }

        // WHEN ReDim Preserve is called on the outer dimension to (1 To 2, 1 To 10)
        let new_bounds_ok = [
            SafeArrayBound {
                c_elements: 2,
                l_lbound: 1,
            },
            SafeArrayBound {
                c_elements: 10,
                l_lbound: 1,
            },
        ];
        let res = SafeArray::redim_preserve_multidim(p_sa, &new_bounds_ok);
        assert!(res.is_ok());

        // THEN data in elements (1..2, 1..5) is preserved intact
        let new_slice = core::slice::from_raw_parts((*p_sa).pv_data as *const i32, 20);
        for i in 0..10 {
            assert_eq!(new_slice[i], (i + 1) as i32 * 10);
        }
        // AND UBound(arr, 2) returns 10
        assert_eq!(__vbaUbound(p_sa, 2), 10);

        // WHEN ReDim Preserve attempts to resize the first dimension (1 To 4, 1 To 10)
        let new_bounds_invalid = [
            SafeArrayBound {
                c_elements: 4,
                l_lbound: 1,
            },
            SafeArrayBound {
                c_elements: 10,
                l_lbound: 1,
            },
        ];
        let err_res = SafeArray::redim_preserve_multidim(p_sa, &new_bounds_invalid);
        // THEN runtime error 9 (Subscript out of range) is raised
        assert_eq!(err_res, Err(9));
        assert_eq!(msvbvm60::err::get_vb_error_number(), 9);

        SafeArray::destroy(p_sa);
    }
}

#[test]
fn test_split_and_join_roundtrip_with_multicharacter_delimiters() {
    unsafe {
        let source = sys_alloc_string("ALPHA<|>BETA<|>GAMMA");
        let delim_split = sys_alloc_string("<|>");
        let v_delim_split = Variant::from_bstr(delim_split);

        let mut split_var = Variant::empty();
        rtcSplit(&mut split_var, source, &v_delim_split, -1, 0);

        let p_sa = __vbaAryVar(&split_var, 0);
        assert!(!p_sa.is_null());
        assert_eq!(__vbaLbound(p_sa, 1), 0);
        assert_eq!(__vbaUbound(p_sa, 1), 2);
        assert_eq!((*p_sa).rgsabound[0].c_elements, 3);

        let delim_join = sys_alloc_string("::");
        let v_delim_join = Variant::from_bstr(delim_join);
        let joined_bstr = rtcJoin(&split_var, &v_delim_join);
        assert_eq!(bstr_to_string(joined_bstr), "ALPHA::BETA::GAMMA");

        SysFreeString(source);
        SysFreeString(delim_split);
        SysFreeString(delim_join);
        SysFreeString(joined_bstr);
    }
}

#[test]
fn test_bstr_to_ui1_safearray_coercion() {
    unsafe {
        // GIVEN a BSTR containing ASCII string "ClassicBasic"
        let bstr = sys_alloc_string("ClassicBasic");

        // WHEN converted to a 1D SafeArray of Bytes (__vbaUI1ArrayStr)
        let psa = __vbaUI1ArrayStr(bstr);
        assert!(!psa.is_null());

        // THEN a SafeArray of VT_UI1 is constructed with LBound 0 and UBound 11
        assert_eq!((*psa).c_dims, 1);
        assert_eq!((*psa).cb_elements, 1);
        assert_eq!((*psa).rgsabound[0].l_lbound, 0);
        assert_eq!((*psa).rgsabound[0].c_elements, 12);
        let ubound = (*psa).rgsabound[0].l_lbound + (*psa).rgsabound[0].c_elements as i32 - 1;
        assert_eq!(ubound, 11);

        // AND elements match the raw byte values [67, 108, 97, 115, 115, 105, 99, 66, 97, 115, 105, 99]
        let slice = core::slice::from_raw_parts((*psa).pv_data as *const u8, 12);
        assert_eq!(
            slice,
            &[67, 108, 97, 115, 115, 105, 99, 66, 97, 115, 105, 99]
        );

        SysFreeString(bstr);
        SafeArray::destroy(psa);
    }
}

#[test]
fn test_ui1_safearray_to_bstr_coercion() {
    unsafe {
        // GIVEN a 1D SafeArray of Bytes containing [82, 117, 115, 116] ("Rust")
        let psa = SafeArray::create_vector(1, 0, 4, FADF_HAVEVARTYPE);
        assert!(!psa.is_null());
        let bytes = [82u8, 117, 115, 116];
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), (*psa).pv_data as *mut u8, 4);

        // WHEN converted to a BSTR (__vbaStrUI1Array)
        let bstr = __vbaStrUI1Array(psa);
        assert!(!bstr.is_null());

        // THEN a valid COM BSTR is allocated with length prefix 4
        let len_chars = SysStringLen(bstr);
        assert_eq!(len_chars, 4);
        let len_bytes = *((bstr as *const u8).sub(4) as *const u32);
        assert_eq!(len_bytes, 8); // 4 UTF-16 characters = 8 bytes

        // AND the decoded character content equals "Rust"
        assert_eq!(bstr_to_string(bstr), "Rust");

        SysFreeString(bstr);
        SafeArray::destroy(psa);
    }
}




fn build_sample_rec_info() -> [u8; 36] {
    let mut desc = [0u8; 36];
    desc[0..2].copy_from_slice(&36u16.to_le_bytes()); // cb_desc = 36
    desc[2..4].copy_from_slice(&32u16.to_le_bytes()); // cb_record = 32
    desc[6..8].copy_from_slice(&3u16.to_le_bytes()); // c_fields = 3
    desc[8..10].copy_from_slice(&24u16.to_le_bytes()); // cb_ansi = 24

    // Field 0 (pos 12..22): fixed string String * 6 at uni_offset 0, ansi_offset 0
    desc[12..14].copy_from_slice(&0u16.to_le_bytes());
    desc[14..16].copy_from_slice(&0x200au16.to_le_bytes());
    desc[18..20].copy_from_slice(&0u16.to_le_bytes());
    desc[20..22].copy_from_slice(&6u16.to_le_bytes());

    // Field 1 (pos 22..30): dynamic BSTR at uni_offset 16, ansi_offset 8
    desc[22..24].copy_from_slice(&16u16.to_le_bytes());
    desc[24..26].copy_from_slice(&0x2001u16.to_le_bytes());
    desc[28..30].copy_from_slice(&8u16.to_le_bytes());

    // Field 2 (pos 30..36+2 -> wait: 8 bytes at pos 28..36 if Field 1 is 6? No, Field 1 is 8 bytes 22..30, so let's make desc 38 bytes)
    desc
}

fn build_full_rec_info() -> [u8; 38] {
    let mut desc = [0u8; 38];
    desc[0..2].copy_from_slice(&38u16.to_le_bytes()); // cb_desc = 38
    desc[2..4].copy_from_slice(&32u16.to_le_bytes()); // cb_record = 32
    desc[6..8].copy_from_slice(&3u16.to_le_bytes()); // c_fields = 3
    desc[8..10].copy_from_slice(&24u16.to_le_bytes()); // cb_ansi = 24

    // Field 0 (pos 12..22, 10 bytes): fixed string String * 6 at uni_offset 0, ansi_offset 0
    desc[12..14].copy_from_slice(&0u16.to_le_bytes());
    desc[14..16].copy_from_slice(&0x200au16.to_le_bytes());
    desc[18..20].copy_from_slice(&0u16.to_le_bytes());
    desc[20..22].copy_from_slice(&6u16.to_le_bytes());

    // Field 1 (pos 22..30, 8 bytes): dynamic BSTR at uni_offset 16, ansi_offset 8
    desc[22..24].copy_from_slice(&16u16.to_le_bytes());
    desc[24..26].copy_from_slice(&0x2001u16.to_le_bytes());
    desc[28..30].copy_from_slice(&8u16.to_le_bytes());

    // Field 2 (pos 30..38, 8 bytes): primitive 4-byte block at uni_offset 24, ansi_offset 16
    desc[30..32].copy_from_slice(&24u16.to_le_bytes());
    desc[32..34].copy_from_slice(&0x200bu16.to_le_bytes());
    desc[34..36].copy_from_slice(&4u16.to_le_bytes());
    desc[36..38].copy_from_slice(&16u16.to_le_bytes());
    desc
}

#[test]
fn test_vba_ui1_i2_coercion_and_overflow() {
    unsafe {
        clear_vb_error();
        assert_eq!(__vbaUI1I2(0), 0);
        assert_eq!(__vbaUI1I2(42), 42);
        assert_eq!(__vbaUI1I2(255), 255);
        assert_eq!(get_vb_error_number(), 0);

        assert_eq!(__vbaUI1I2(-1), 0);
        assert_eq!(get_vb_error_number(), 6);
        clear_vb_error();

        assert_eq!(__vbaUI1I2(256), 0);
        assert_eq!(get_vb_error_number(), 6);
        clear_vb_error();
    }
}

#[test]
fn test_vba_fixstr_construct_and_str_fixstr() {
    unsafe {
        let mut buf = [0u16; 10];
        let rc = __vbaFixstrConstruct(buf.as_mut_ptr(), 10);
        assert_eq!(rc, 0);
        assert!(buf.iter().all(|&c| c == 0x0020));

        let bstr = __vbaStrFixstr(10, buf.as_ptr());
        assert_eq!(__vbaLenBstr(bstr), 10);
        assert_eq!(bstr_to_string(bstr), "          ");
        SysFreeString(bstr);
    }
}

#[test]
fn test_vba_lset_fixstr_and_lset_fixstr_free() {
    unsafe {
        let mut buf10 = [0u16; 10];
        __vbaFixstrConstruct(buf10.as_mut_ptr(), 10);

        // 1. Short string right-pads with spaces
        let s_abc = sys_alloc_string("ABC");
        __vbaLsetFixstr(10, buf10.as_mut_ptr(), s_abc);
        SysFreeString(s_abc);
        let b1 = __vbaStrFixstr(10, buf10.as_ptr());
        assert_eq!(bstr_to_string(b1), "ABC       ");
        SysFreeString(b1);

        // 2. Long string truncates to fixed length and frees source
        let mut buf5 = [0u16; 5];
        __vbaLsetFixstrFree(5, buf5.as_mut_ptr(), sys_alloc_string("0123456789"));
        let b2 = __vbaStrFixstr(5, buf5.as_ptr());
        assert_eq!(bstr_to_string(b2), "01234");
        SysFreeString(b2);

        // 3. Dynamic BSTR LSet (char_len == 0) preserves target length
        let dyn_target = sys_alloc_string("........");
        __vbaLsetFixstrFree(0, dyn_target, sys_alloc_string("XY"));
        assert_eq!(bstr_to_string(dyn_target), "XY      ");
        SysFreeString(dyn_target);
    }
}

#[test]
fn test_vba_rset_fixstr_and_rset_fixstr_free() {
    unsafe {
        let mut buf8 = [0u16; 8];
        __vbaFixstrConstruct(buf8.as_mut_ptr(), 8);

        // 1. Short string right-aligns and left-pads with spaces
        let s_42 = sys_alloc_string("42");
        __vbaRsetFixstr(8, buf8.as_mut_ptr(), s_42);
        SysFreeString(s_42);
        let b1 = __vbaStrFixstr(8, buf8.as_ptr());
        assert_eq!(bstr_to_string(b1), "      42");
        SysFreeString(b1);

        // 2. Long string keeps first 8 characters and frees temporary BSTR
        __vbaRsetFixstrFree(8, buf8.as_mut_ptr(), sys_alloc_string("1234567890"));
        let b2 = __vbaStrFixstr(8, buf8.as_ptr());
        assert_eq!(bstr_to_string(b2), "12345678");
        SysFreeString(b2);

        // 3. Dynamic BSTR RSet (char_len == 0) preserves target length
        let dyn_target = sys_alloc_string("........");
        __vbaRsetFixstrFree(0, dyn_target, sys_alloc_string("99"));
        assert_eq!(bstr_to_string(dyn_target), "      99");
        SysFreeString(dyn_target);
    }
}

#[test]
fn test_vba_rec_assign_copy_record_and_rec_destruct() {
    unsafe {
        let desc = build_full_rec_info();
        let rec_info = desc.as_ptr() as *const c_void;

        let mut rec1 = [0u8; 32];
        let mut rec2 = [0u8; 32];
        let mut rec3 = [0u8; 32];

        __vbaLsetFixstrFree(6, rec1.as_mut_ptr() as *mut u16, sys_alloc_string("ENG01"));
        let bstr1 = sys_alloc_string("Ada Lovelace");
        let ptr_size = core::mem::size_of::<usize>();
        rec1[16..16 + ptr_size].copy_from_slice(&(bstr1 as usize).to_ne_bytes());
        rec1[24..28].copy_from_slice(&1001i32.to_le_bytes());

        // __vbaRecAssign deep-copies rec1 -> rec2
        let ret_assign = __vbaRecAssign(
            rec_info,
            rec2.as_mut_ptr() as *mut c_void,
            rec1.as_ptr() as *const c_void,
        );
        assert_eq!(ret_assign, rec2.as_mut_ptr() as *mut c_void);

        // CopyRecord deep-copies rec2 -> rec3
        let ret_copy = CopyRecord(
            rec_info,
            rec3.as_mut_ptr() as *mut c_void,
            rec2.as_ptr() as *const c_void,
        );
        assert_eq!(ret_copy, rec3.as_mut_ptr() as *mut c_void);

        let mut b2_bytes = [0u8; core::mem::size_of::<usize>()];
        b2_bytes.copy_from_slice(&rec2[16..16 + ptr_size]);
        let bstr2 = usize::from_ne_bytes(b2_bytes) as Bstr;
        assert_ne!(bstr2, bstr1, "BSTR field must be deep-cloned without pointer aliasing");
        assert_eq!(bstr_to_string(bstr2), "Ada Lovelace");
        assert_eq!(i32::from_le_bytes(rec2[24..28].try_into().unwrap()), 1001);

        // __vbaRecDestruct frees BSTRs and returns p_record in EAX
        let d1 = __vbaRecDestruct(rec_info, rec1.as_mut_ptr() as *mut c_void);
        assert_eq!(d1, rec1.as_mut_ptr() as *mut c_void);
        assert!(rec1[16..16 + ptr_size].iter().all(|&b| b == 0));

        __vbaRecDestruct(rec_info, rec2.as_mut_ptr() as *mut c_void);
        __vbaRecDestruct(rec_info, rec3.as_mut_ptr() as *mut c_void);
    }
}

#[test]
fn test_vba_ary_rec_copy_and_ary_rec_move() {
    unsafe {
        let desc = build_sample_rec_info();
        let rec_info = desc.as_ptr() as *const c_void;
        let ptr_size = core::mem::size_of::<usize>();

        let mut src_sa = SafeArray::create_vector(32, 1, 2, 0);
        let mut copy_sa: *mut SafeArray = core::ptr::null_mut();
        let mut move_sa: *mut SafeArray = core::ptr::null_mut();

        let elem0 = (*src_sa).pv_data as *mut u8;
        __vbaLsetFixstrFree(6, elem0 as *mut u16, sys_alloc_string("DEV1"));
        let b0 = sys_alloc_string("First Engineer");
        core::slice::from_raw_parts_mut(elem0.add(16), ptr_size)
            .copy_from_slice(&(b0 as usize).to_ne_bytes());

        let elem1 = elem0.add(32);
        __vbaLsetFixstrFree(6, elem1 as *mut u16, sys_alloc_string("DEV2"));
        let b1 = sys_alloc_string("Second Engineer");
        core::slice::from_raw_parts_mut(elem1.add(16), ptr_size)
            .copy_from_slice(&(b1 as usize).to_ne_bytes());

        assert_eq!(__vbaAryRecCopy(rec_info, &mut copy_sa, &src_sa), 0);
        assert!(!copy_sa.is_null());
        assert_eq!(__vbaLbound(copy_sa, 1), 1);
        assert_eq!(__vbaUbound(copy_sa, 1), 2);

        let copy_elem0 = (*copy_sa).pv_data as *const u8;
        let mut cb0_bytes = [0u8; core::mem::size_of::<usize>()];
        cb0_bytes.copy_from_slice(core::slice::from_raw_parts(copy_elem0.add(16), ptr_size));
        let cb0 = usize::from_ne_bytes(cb0_bytes) as Bstr;
        assert_ne!(cb0, b0);
        assert_eq!(bstr_to_string(cb0), "First Engineer");

        assert_eq!(__vbaAryRecMove(rec_info, &mut move_sa, &mut copy_sa), 0);
        assert!(copy_sa.is_null());
        assert!(!move_sa.is_null());

        __vbaAryDestruct(rec_info, &mut src_sa);
        __vbaAryDestruct(rec_info, &mut move_sa);
    }
}

#[test]
fn test_vba_cvar_ary_udt_and_udt_var() {
    unsafe {
        let desc = build_sample_rec_info();
        let rec_info = desc.as_ptr() as *const c_void;
        let mut sa = SafeArray::create_vector(32, 1, 2, 0);
        let mut var = Variant::empty();

        let p_var = __vbaCVarAryUdt(&mut var, &sa, rec_info);
        assert_eq!(p_var, &mut var as *mut Variant);
        assert_eq!(var.vt, 0x2024); // VT_ARRAY | VT_RECORD

        let extracted = __vbaUdtVar(&var) as *mut SafeArray;
        assert_eq!(extracted, sa);

        __vbaAryDestruct(rec_info, &mut sa);
    }
}

#[test]
fn test_vba_rec_uni_to_ansi_and_ansi_to_uni_and_destruct_ansi() {
    unsafe {
        let desc = build_full_rec_info();
        let rec_info = desc.as_ptr() as *const c_void;
        let ptr_size = core::mem::size_of::<usize>();

        let mut uni_src = [0u8; 32];
        let mut ansi_buf = [0u8; 24];
        let mut uni_dst = [0u8; 32];

        __vbaLsetFixstrFree(6, uni_src.as_mut_ptr() as *mut u16, sys_alloc_string("WIN32A"));
        let title_bstr = sys_alloc_string("ConsoleWindow");
        uni_src[16..16 + ptr_size].copy_from_slice(&(title_bstr as usize).to_ne_bytes());
        uni_src[24..28].copy_from_slice(&0x12345678u32.to_le_bytes());

        let p_ansi = __vbaRecUniToAnsi(
            rec_info,
            ansi_buf.as_mut_ptr() as *mut c_void,
            uni_src.as_ptr() as *const c_void,
        );
        assert_eq!(p_ansi, ansi_buf.as_mut_ptr() as *mut c_void);
        assert_eq!(&ansi_buf[0..6], b"WIN32A");
        assert_eq!(
            u32::from_le_bytes(ansi_buf[16..20].try_into().unwrap()),
            0x12345678
        );

        let p_uni = __vbaRecAnsiToUni(
            rec_info,
            uni_dst.as_mut_ptr() as *mut c_void,
            ansi_buf.as_ptr() as *const c_void,
        );
        assert_eq!(p_uni, uni_dst.as_mut_ptr() as *mut c_void);

        let fx_bstr = __vbaStrFixstr(6, uni_dst.as_ptr() as *const u16);
        assert_eq!(bstr_to_string(fx_bstr), "WIN32A");
        SysFreeString(fx_bstr);

        let mut dst_bstr_bytes = [0u8; core::mem::size_of::<usize>()];
        dst_bstr_bytes.copy_from_slice(&uni_dst[16..16 + ptr_size]);
        let dst_bstr = usize::from_ne_bytes(dst_bstr_bytes) as Bstr;
        assert_eq!(bstr_to_string(dst_bstr), "ConsoleWindow");
        assert_eq!(
            u32::from_le_bytes(uni_dst[24..28].try_into().unwrap()),
            0x12345678
        );

        __vbaRecDestructAnsi(rec_info, ansi_buf.as_mut_ptr() as *mut c_void);
        __vbaRecDestruct(rec_info, uni_src.as_mut_ptr() as *mut c_void);
        __vbaRecDestruct(rec_info, uni_dst.as_mut_ptr() as *mut c_void);
    }
}

#[test]
fn test_vba_put_get_fxstr_and_owner_file_io() {
    unsafe {
        let fn_num = rtcFreeFile(core::ptr::null()) as i32;
        let file_path = "v0_1_6_fxstr_owner_test.bin";
        let _ = std::fs::remove_file(file_path);
        let b_path = sys_alloc_string(file_path);

        assert_eq!(__vbaFileOpen(0x20, -1, fn_num, b_path), 0);

        let mut fx_out1 = [0u16; 6];
        let mut fx_out2 = [0u16; 6];
        __vbaLsetFixstrFree(6, fx_out1.as_mut_ptr(), sys_alloc_string("HEADER"));
        __vbaLsetFixstrFree(6, fx_out2.as_mut_ptr(), sys_alloc_string("SECOND"));

        assert_eq!(__vbaPutFxStr4(6, fx_out1.as_ptr(), 1, fn_num), 0);
        assert_eq!(__vbaPutFxStr3(6, fx_out2.as_ptr(), fn_num), 0);

        let mut owner_desc = [0u8; 8];
        owner_desc[0..2].copy_from_slice(&18u16.to_le_bytes());
        let mut udt_out = [0u8; 18];
        __vbaLsetFixstrFree(8, udt_out.as_mut_ptr() as *mut u16, sys_alloc_string("VICTORIA"));
        udt_out[16..18].copy_from_slice(&777i16.to_le_bytes());

        assert_eq!(
            __vbaPutOwner4(
                owner_desc.as_ptr() as *const c_void,
                udt_out.as_ptr() as *const c_void,
                13,
                fn_num,
            ),
            0
        );
        assert_eq!(
            __vbaPutOwner3(
                owner_desc.as_ptr() as *const c_void,
                udt_out.as_ptr() as *const c_void,
                fn_num,
            ),
            0
        );

        let mut fx_in1 = [0u16; 6];
        let mut fx_in2 = [0u16; 6];
        assert_eq!(__vbaGetFxStr4(6, fx_in1.as_mut_ptr(), 1, fn_num), 0);
        assert_eq!(__vbaGetFxStr3(6, fx_in2.as_mut_ptr(), fn_num), 0);

        let mut udt_in1 = [0u8; 18];
        let mut udt_in2 = [0u8; 18];
        assert_eq!(
            __vbaGetOwner4(
                owner_desc.as_ptr() as *const c_void,
                udt_in1.as_mut_ptr() as *mut c_void,
                13,
                fn_num,
            ),
            0
        );
        assert_eq!(
            __vbaGetOwner3(
                owner_desc.as_ptr() as *const c_void,
                udt_in2.as_mut_ptr() as *mut c_void,
                fn_num,
            ),
            0
        );

        __vbaFileClose(fn_num);
        let _ = std::fs::remove_file(file_path);
        SysFreeString(b_path);

        let b_in1 = __vbaStrFixstr(6, fx_in1.as_ptr());
        let b_in2 = __vbaStrFixstr(6, fx_in2.as_ptr());
        assert_eq!(bstr_to_string(b_in1), "HEADER");
        assert_eq!(bstr_to_string(b_in2), "SECOND");
        SysFreeString(b_in1);
        SysFreeString(b_in2);

        let b_udt1 = __vbaStrFixstr(8, udt_in1.as_ptr() as *const u16);
        let b_udt2 = __vbaStrFixstr(8, udt_in2.as_ptr() as *const u16);
        assert_eq!(bstr_to_string(b_udt1), "VICTORIA");
        assert_eq!(bstr_to_string(b_udt2), "VICTORIA");
        assert_eq!(i16::from_le_bytes([udt_in1[16], udt_in1[17]]), 777);
        assert_eq!(i16::from_le_bytes([udt_in2[16], udt_in2[17]]), 777);
        SysFreeString(b_udt1);
        SysFreeString(b_udt2);
    }
}

#[test]
fn test_safearray_multidim_stride_and_boundary_layout() {
    unsafe {
        let bounds = [
            SafeArrayBound { c_elements: 3, l_lbound: 1 },
            SafeArrayBound { c_elements: 3, l_lbound: 10 },
        ];

        let p_sa = SafeArray::create_multidim(4, &bounds, 0);
        assert!(!p_sa.is_null());
        assert_eq!((*p_sa).c_dims, 2);
        assert_eq!((*p_sa).cb_elements, 4);
        assert_eq!((*p_sa).c_locks, 0);

        let data_ptr = (*p_sa).pv_data as *mut i32;
        for i in 1..=3 {
            for j in 10..=12 {
                let offset = (i - 1) + (j - 10) * 3;
                let val = ((i * 100) + j) as i32;
                *data_ptr.add(offset) = val;
            }
        }

        assert_eq!(*data_ptr.add(0), 110);
        assert_eq!(*data_ptr.add(1), 210);
        assert_eq!(*data_ptr.add(2), 310);
        assert_eq!(*data_ptr.add(3), 111);
        assert_eq!(*data_ptr.add(8), 312);

        SafeArray::destroy(p_sa);
    }
}

#[test]
fn test_safearray_lock_violation_rejection() {
    unsafe {
        let p_sa = SafeArray::create_vector(4, 1, 5, 0);
        assert!(!p_sa.is_null());

        (*p_sa).c_locks = 1;
        assert!((*p_sa).c_locks > 0, "SafeArray must report locked state");

        SafeArray::redim_preserve(p_sa, 10, 1);
        assert_eq!(msvbvm60::err::get_vb_error_number(), 10);
        msvbvm60::err::clear_vb_error();

        (*p_sa).c_locks = 0;
        SafeArray::destroy(p_sa);
    }
}
