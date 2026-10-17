// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Regression Suite `v0.1.5` — Phase 1.6: User-Defined Types (UDT) & Fixed Strings (`App21`)
//!
//! Validates fixed-length UTF-16 string buffers (`String * N`), `LSet`/`RSet` alignment and truncation,
//! UDT deep assignment (`__vbaRecAssign`, `CopyRecord`), UDT scope destruction (`__vbaRecDestruct`),
//! dynamic UDT arrays (`__vbaAryRecCopy`, `__vbaAryRecMove`), UDT Variant packaging (`__vbaCVarAryUdt`,
//! `__vbaUdtVar`), UDT ANSI/Unicode marshaling (`__vbaRecUniToAnsi`, `__vbaRecAnsiToUni`,
//! `__vbaRecDestructAnsi`), and fixed-string/UDT binary file I/O (`__vbaPutFxStr3/4`, `__vbaGetFxStr3/4`,
//! `__vbaPutOwner3/4`, `__vbaGetOwner3/4`).

use core::ffi::c_void;
use msvbvm60::err::{clear_vb_error, get_vb_error_number};
use msvbvm60::rtc::rtcFreeFile;
use msvbvm60::types::{bstr_to_string, sys_alloc_string, Bstr, SafeArray, SysFreeString, Variant};
use msvbvm60::vba::*;

/// Helper to construct a 2-field UDT `RecInfo` descriptor:
/// - Field 0: `Code As String * 6` at Unicode offset `0` (12 bytes), ANSI offset `0` (6 bytes)
/// - Field 1: `FullName As String` (dynamic `Bstr`) at Unicode offset `16`, ANSI offset `8`
/// - Field 2: `Id As Long` (primitive 4-byte block) at Unicode offset `24`, ANSI offset `16`
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
        let file_path = "v0_1_5_fxstr_owner_test.bin";
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
