//! Layer 02: Strings and BSTR Manipulation Tests
//!
//! Granular tests for COM BSTR allocation, concatenation, length, slicing
//! (Mid, Left, Right), search (InStr), casing (UCase), reverse (StrReverse),
//! trimming, and copying helpers corresponding to App02-strings.

use msvbvm60::*;

#[test]
fn test_sys_alloc_and_free_string() {
    unsafe {
        let b = sys_alloc_string("Hello VB6");
        assert_eq!(bstr_len(b), 9);
        assert_eq!(bstr_byte_len(b), 18);
        assert_eq!(bstr_to_string(b), "Hello VB6");
        SysFreeString(b);
    }
}

#[test]
fn test_vba_str_cat_concatenation() {
    unsafe {
        let s1 = sys_alloc_string("Classic");
        let s2 = sys_alloc_string("Basic");
        let res = __vbaStrCat(s1, s2);
        assert_eq!(bstr_to_string(res), "ClassicBasic");
        SysFreeString(s1);
        SysFreeString(s2);
        SysFreeString(res);
    }
}

#[test]
fn test_vba_len_bstr() {
    unsafe {
        let s = sys_alloc_string("123456789");
        assert_eq!(__vbaLenBstr(s), 9);
        SysFreeString(s);
    }
}

#[test]
fn test_rtc_mid_char_bstr() {
    unsafe {
        let s = sys_alloc_string("Classic Basic v6.0");
        let v_len = Variant::from_i32(5);
        let mid = rtcMidCharBstr(s, 9, &v_len as *const Variant);
        assert_eq!(bstr_to_string(mid), "Basic");
        SysFreeString(mid);
        SysFreeString(s);
    }
}

#[test]
fn test_rtc_left_char_bstr() {
    unsafe {
        let s = sys_alloc_string("Classic Basic v6.0");
        let left = rtcLeftCharBstr(s, 7);
        assert_eq!(bstr_to_string(left), "Classic");
        SysFreeString(left);
        SysFreeString(s);
    }
}

#[test]
fn test_rtc_right_char_bstr() {
    unsafe {
        let s = sys_alloc_string("Classic Basic v6.0");
        let right = rtcRightCharBstr(s, 4);
        assert_eq!(bstr_to_string(right), "v6.0");
        SysFreeString(right);
        SysFreeString(s);
    }
}

#[test]
fn test_vba_instr_substring_search() {
    unsafe {
        let full = sys_alloc_string("Classic Basic v6.0");
        let needle = sys_alloc_string("Basic");
        let pos = __vbaInStr(1, full as usize, needle as usize, 0);
        assert_eq!(pos, 9);
        SysFreeString(needle);
        SysFreeString(full);
    }
}

#[test]
fn test_rtc_upper_case_bstr() {
    unsafe {
        let s = sys_alloc_string("visual basic");
        let u = rtcUpperCaseBstr(s);
        assert_eq!(bstr_to_string(u), "VISUAL BASIC");
        SysFreeString(u);
        SysFreeString(s);
    }
}

#[test]
fn test_rtc_str_reverse() {
    unsafe {
        let s = sys_alloc_string("ClassicBasic");
        let rev = rtcStrReverse(s);
        assert_eq!(bstr_to_string(rev), "cisaBcissalC");
        SysFreeString(rev);
        SysFreeString(s);
    }
}

#[test]
fn test_rtc_trim_bstr() {
    unsafe {
        let s = sys_alloc_string("   clean text   ");
        let trimmed = rtcTrimBstr(s);
        assert_eq!(bstr_to_string(trimmed), "clean text");
        SysFreeString(trimmed);
        SysFreeString(s);
    }
}

#[test]
fn test_rtc_space_bstr() {
    unsafe {
        let sp = rtcSpaceBstr(5);
        assert_eq!(bstr_to_string(sp), "     ");
        SysFreeString(sp);
    }
}

#[test]
fn test_vba_str_copy() {
    unsafe {
        let mut dst: Bstr = core::ptr::null_mut();
        let src = sys_alloc_string("Original");
        __vbaStrCopy(&mut dst, src);
        assert_eq!(bstr_to_string(dst), "Original");
        SysFreeString(src);
        SysFreeString(dst);
    }
}

#[test]
fn test_vba_str_move() {
    unsafe {
        let mut src: Bstr = sys_alloc_string("WillBeMoved");
        let mut dst: Bstr = core::ptr::null_mut();
        __vbaStrMove(&mut dst, &mut src);
        assert_eq!(bstr_to_string(dst), "WillBeMoved");
        assert!(src.is_null());
        SysFreeString(dst);
    }
}

#[test]
fn test_bstr_lifecycle_preserves_length_prefix_and_null_terminator() {
    let rust_str = "ClassicBasic";
    let utf16: Vec<u16> = rust_str.encode_utf16().collect();
    assert_eq!(utf16.len(), 12);

    unsafe {
        let bstr = SysAllocStringLen(utf16.as_ptr(), utf16.len() as u32);
        assert!(!bstr.is_null());

        // 1. Pointer points directly to UTF-16 code units
        let slice = core::slice::from_raw_parts(bstr, utf16.len());
        assert_eq!(slice, &utf16[..]);

        // 2. 32-bit integer at offset -4 matches byte length (24 bytes)
        let byte_len = *((bstr as *const u8).sub(4) as *const u32);
        assert_eq!(byte_len, 24);

        // 3. Memory is terminated with a 16-bit null (0x0000)
        let term = *bstr.add(utf16.len());
        assert_eq!(term, 0x0000);

        // 4. SysFreeString safely reclaims buffer
        SysFreeString(bstr);
    }
}

#[test]
fn test_mid_statement_mutates_bstr_in_place_without_changing_total_length() {
    unsafe {
        // GIVEN a target BSTR containing "HELLO WORLD"
        let mut target = sys_alloc_string("HELLO WORLD");
        let replacement = sys_alloc_string("RUST!");

        // WHEN __vbaMidStmtBstr is called with Start = 7, Length = 5, Replacement = "RUST!"
        __vbaMidStmtBstr(&mut target, 7, 5, replacement);

        // THEN target BSTR is mutated in-place to "HELLO RUST!"
        assert_eq!(bstr_to_string(target), "HELLO RUST!");

        // AND the 32-bit length prefix at offset -4 remains strictly 11 characters (22 bytes)
        let len_prefix = *((target as *const u8).sub(4) as *const u32);
        assert_eq!(len_prefix, 22);

        SysFreeString(target);
        SysFreeString(replacement);
    }
}

#[test]
fn test_mid_statement_truncates_replacement_if_exceeding_target_boundary() {
    unsafe {
        // GIVEN a target BSTR containing "ABCDEF"
        let mut target = sys_alloc_string("ABCDEF");
        let replacement = sys_alloc_string("12345");

        // WHEN __vbaMidStmtBstr is called with Start = 5, Length = 10, Replacement = "12345"
        __vbaMidStmtBstr(&mut target, 5, 10, replacement);

        // THEN it only copies the characters that fit within the original buffer bounds ("EF" -> "12")
        // AND target BSTR becomes "ABCD12" without reallocating or growing the buffer size
        assert_eq!(bstr_to_string(target), "ABCD12");

        let len_prefix = *((target as *const u8).sub(4) as *const u32);
        assert_eq!(len_prefix, 12);

        SysFreeString(target);
        SysFreeString(replacement);
    }
}
