// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Layer 18: Complex String Manipulation, Embedded Nulls & Codepage Conversions (Phase 2.5B Suite 3).
//!
//! Validates:
//! - BSTR binary safety: embedded null (`\0`) preservation, length prefix integrity (`Len` / `LenB`).
//! - Bidirectional codepage conversions via `StrConv` (`vbFromUnicode` = 128, `vbUnicode` = 64, `vbProperCase` = 3).
//! - `String$` intrinsic allocation with character code wrapping (`charcode Mod 256`) and string truncation.
//! - Native `Like` operator (`__vbaStrLike` / `__vbaStrTextLike`): wildcards `?`, `*`, `#`, ranges `[0-9]`, negation `[!...]`, and escapes `[[]`, `[.]`.
//! - `vbNullString` (raw NULL 0x0 pointer) equivalence to empty string in all runtime helpers without AVs.
//! - `Replace` intrinsic: 1-based start truncation, substitution count bounds, and `vbBinaryCompare` vs `vbTextCompare`.

use msvbvm60::rtc::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;

#[test]
fn test_bstr_with_embedded_null_characters_preserves_length_and_content() {
    // GIVEN a BSTR containing embedded null characters (14 characters total: "Hello\0World\0VB")
    unsafe {
        let s_hello = sys_alloc_string("Hello");
        let s_null1 = SysAllocStringLen([0u16].as_ptr(), 1);
        let s_world = sys_alloc_string("World");
        let s_null2 = SysAllocStringLen([0u16].as_ptr(), 1);
        let s_vb = sys_alloc_string("VB");

        let p1 = __vbaStrCat(s_hello, s_null1);
        let p2 = __vbaStrCat(p1, s_world);
        let p3 = __vbaStrCat(p2, s_null2);
        let s_bstr = __vbaStrCat(p3, s_vb);

        // WHEN passed through Len, __vbaStrCat, and Left$ / Right$ operations
        let len_chars = __vbaLenBstr(s_bstr);
        let len_bytes = __vbaLenBstrB(s_bstr);

        // THEN Len(str) must report 14 characters (reading length from offset -4, not stopping at '\0')
        assert_eq!(len_chars, 14, "Len must report 14 characters");
        assert_eq!(len_bytes, 28, "LenB must report 28 bytes");

        // AND concatenating with "End" must produce "Hello\0World\0VBEnd" (17 chars)
        let s_end = sys_alloc_string("End");
        let s_concat = __vbaStrCat(s_bstr, s_end);
        assert_eq!(__vbaLenBstr(s_concat), 17, "Concat length must be 17");
        assert_eq!(__vbaLenBstrB(s_concat), 34, "Concat bytes must be 34");

        // AND the exact binary byte sequence must be preserved across copies and moves
        let s_right = rtcRightCharBstr(s_concat, 6);
        assert_eq!(__vbaLenBstr(s_right), 6);
        let right_slice = bstr_as_slice(s_right);
        assert_eq!(right_slice[0], 0, "Right$(6) first char must be embedded null");
        assert_eq!(right_slice[1], 'V' as u16);
        assert_eq!(right_slice[2], 'B' as u16);
        assert_eq!(right_slice[3], 'E' as u16);
        assert_eq!(right_slice[4], 'n' as u16);
        assert_eq!(right_slice[5], 'd' as u16);

        let v_one = Variant::from_i32(1);
        let s_mid_null = rtcMidCharBstr(s_concat, 6, &v_one as *const Variant);
        assert_eq!(__vbaLenBstr(s_mid_null), 1);
        assert_eq!(*s_mid_null, 0, "Mid$(6, 1) must be embedded null");

        let v_five = Variant::from_i32(5);
        let s_mid_world = rtcMidCharBstr(s_concat, 7, &v_five as *const Variant);
        assert_eq!(bstr_to_string(s_mid_world), "World");
    }
}

#[test]
fn test_strconv_bidirectional_unicode_and_ansi_byte_array_conversions() {
    // GIVEN a UTF-16 BSTR "Architecture"
    unsafe {
        let s_origin = sys_alloc_string("Architecture");
        let v_origin = Variant::from_bstr(s_origin);

        // WHEN converted to ANSI via StrConv(s, vbFromUnicode = 128)
        let mut v_ansi = Variant::empty();
        rtcStrConvVar2(&mut v_ansi, &v_origin, 128, 0);

        // THEN the resulting string contains single-byte Windows-1252 codepage bytes
        let ansi_bstr = v_ansi.as_bstr().expect("StrConv vbFromUnicode must yield BSTR");
        assert_eq!(__vbaLenBstrB(ansi_bstr), 12, "ANSI BSTR byte length must be 12");

        // AND StrConv(result, vbUnicode = 64) must cleanly reconstruct the identical UTF-16 BSTR
        let mut v_restored = Variant::empty();
        rtcStrConvVar2(&mut v_restored, &v_ansi, 64, 0);

        let restored_bstr = v_restored.as_bstr().expect("StrConv vbUnicode must yield BSTR");
        assert_eq!(bstr_to_string(restored_bstr), "Architecture");
        assert_eq!(__vbaLenBstr(restored_bstr), 12);
        assert_eq!(__vbaLenBstrB(restored_bstr), 24);
    }
}

#[test]
fn test_string_function_character_code_overflow_and_multibyte_wrapping() {
    unsafe {
        // GIVEN the String$ allocation intrinsic
        // WHEN called as String$(5, 65) -> must return "AAAAA"
        let v_65 = Variant::from_i32(65);
        let s_a = rtcStringBstr(5, &v_65);
        assert_eq!(bstr_to_string(s_a), "AAAAA");
        assert_eq!(__vbaLenBstr(s_a), 5);

        // WHEN called with a character code > 255 (e.g., String$(3, 1024))
        // THEN the character code must mask to low-byte (1024 mod 256 = 0) or adhere to OEM/ANSI mapping
        // AND prevent integer truncation panics or buffer overruns in the allocator.
        let v_1024 = Variant::from_i32(1024);
        let s_wrap = rtcStringBstr(3, &v_1024);
        assert_eq!(__vbaLenBstr(s_wrap), 3);
        assert_eq!(__vbaLenBstrB(s_wrap), 6);
        let slice = bstr_as_slice(s_wrap);
        assert_eq!(slice, &[0u16, 0u16, 0u16]);

        // String$ with string argument takes first character
        let v_str = Variant::from_bstr(sys_alloc_string("VisualBasic"));
        let s_first = rtcStringBstr(4, &v_str);
        assert_eq!(bstr_to_string(s_first), "VVVV");
    }
}

#[test]
fn test_like_operator_complex_pattern_matching_with_character_ranges() {
    // GIVEN string patterns executed against the native Like operator (__vbaStrLike)
    unsafe {
        // WHEN matching "File09_v2.dat" against "File[0-9][0-9]_[!a-z]#[.][a-z]*"
        // THEN evaluation must return TRUE
        let s_file = sys_alloc_string("File09_v2.dat");
        let p_file = sys_alloc_string("File[0-9][0-9]_[!a-z]#[.][a-z]*");
        assert_eq!(__vbaStrLike(s_file, p_file), -1, "Complex pattern must evaluate to True (-1)");

        // WHEN matching "ABC" against "[!A-Z]*"
        // THEN evaluation must return FALSE, strictly respecting VB6 bracket negation '[!]' semantics.
        let s_abc = sys_alloc_string("ABC");
        let p_neg = sys_alloc_string("[!A-Z]*");
        assert_eq!(__vbaStrLike(s_abc, p_neg), 0, "Negated range [!A-Z] must reject uppercase 'A'");

        // Positive test for negation: "abc" against "[!A-Z]*" must match
        let s_lower = sys_alloc_string("abc");
        assert_eq!(__vbaStrLike(s_lower, p_neg), -1, "Negated range [!A-Z] must accept lowercase 'a'");
    }
}

#[test]
fn test_bstr_null_pointer_equivalence_to_empty_string_in_all_helpers() {
    // GIVEN a raw NULL pointer representing an uninitialized BSTR (vbNullString)
    unsafe {
        let null_bstr: Bstr = core::ptr::null_mut();

        // WHEN passed to __vbaStrCat, __vbaLenBstr, rtcTrimBstr, or InStr
        // THEN __vbaLenBstr must return 0 without dereferencing pointer 0x0
        assert_eq!(__vbaLenBstr(null_bstr), 0);
        assert_eq!(__vbaLenBstrB(null_bstr), 0);

        // AND concatenating vbNullString with "Test" must return "Test"
        let s_test = sys_alloc_string("Test");
        let s_cat = __vbaStrCat(null_bstr, s_test);
        assert_eq!(bstr_to_string(s_cat), "Test");

        let s_cat_rev = __vbaStrCat(s_test, null_bstr);
        assert_eq!(bstr_to_string(s_cat_rev), "Test");

        // Trim on null string returns empty
        let s_trimmed = rtcTrimBstr(null_bstr);
        assert_eq!(bstr_to_string(s_trimmed), "");

        // AND InStr(1, vbNullString, "A") must return 0 without raising memory access violations.
        let s_a = sys_alloc_string("A");
        assert_eq!(__vbaInStr(1, null_bstr as usize, s_a as usize, 0), 0);

        // InStr(1, "Test", vbNullString) returns 1
        assert_eq!(__vbaInStr(1, s_test as usize, null_bstr as usize, 0), 1);
    }
}

#[test]
fn test_replace_with_binary_and_text_comparisons_and_count_limits() {
    // GIVEN source string "AbcAbaBC"
    unsafe {
        let s_source = sys_alloc_string("AbcAbaBC");
        let s_find = sys_alloc_string("ab");
        let s_rep1 = sys_alloc_string("1");

        // WHEN Replace is invoked with Compare = vbBinaryCompare, Start = 1, Count = 1, Find = "ab"
        // THEN it replaces only the lowercase match at index 4 -> "Abc1aBC"
        let res_bin = rtcReplace(s_source, s_find, s_rep1, 1, 1, 0);
        assert_eq!(bstr_to_string(res_bin), "Abc1aBC");

        // WHEN Replace is invoked with Compare = vbTextCompare, Start = 4, Find = "ab"
        // THEN it evaluates case-insensitively starting from position 4 and returns the sliced remainder.
        let res_text_start = rtcReplace(s_source, s_find, s_rep1, 4, -1, 1);
        let res_str = bstr_to_string(res_text_start);
        assert!(res_str.starts_with("1"), "Slice starting from 4 must begin with replacement 1");

        // Test VB6 Start > 1 truncation behavior with binary compare
        let s_find_a = sys_alloc_string("a");
        let s_underscore = sys_alloc_string("_");
        let res_start = rtcReplace(s_source, s_find_a, s_underscore, 4, -1, 0);
        assert_eq!(bstr_to_string(res_start), "Ab_BC", "Start > 1 must slice prefix in VB6 semantics");
    }
}

#[test]
fn test_like_operator_character_classes_and_escapes() {
    unsafe {
        // Wildcards: ?, *, #
        let s_doc1 = sys_alloc_string("Doc1.txt");
        let p_q = sys_alloc_string("Doc?.txt");
        assert_eq!(__vbaStrLike(s_doc1, p_q), -1);

        let s_doc12 = sys_alloc_string("Doc12.txt");
        let p_star = sys_alloc_string("Doc*.txt");
        assert_eq!(__vbaStrLike(s_doc12, p_star), -1);

        let s_item9 = sys_alloc_string("Item9");
        let p_hash = sys_alloc_string("Item#");
        assert_eq!(__vbaStrLike(s_item9, p_hash), -1);

        let s_item_a = sys_alloc_string("ItemA");
        assert_eq!(__vbaStrLike(s_item_a, p_hash), 0);

        // Bracket character classes and escapes: [[] and [.]
        let s_bracket = sys_alloc_string("a[b");
        let p_bracket = sys_alloc_string("a[[]b");
        assert_eq!(__vbaStrLike(s_bracket, p_bracket), -1);

        let s_dot = sys_alloc_string("test.dat");
        let p_dot = sys_alloc_string("*[.]dat");
        assert_eq!(__vbaStrLike(s_dot, p_dot), -1);

        // Case-insensitive Like (__vbaStrTextLike)
        let s_mix = sys_alloc_string("DocA.txt");
        let p_lower_range = sys_alloc_string("Doc[a-z].txt");
        assert_eq!(__vbaStrTextLike(s_mix, p_lower_range), -1, "TextLike must ignore case in ranges");
    }
}

#[test]
fn test_strconv_casing_options() {
    unsafe {
        let s_text = sys_alloc_string("hello world");
        let v_text = Variant::from_bstr(s_text);

        // vbUpperCase = 1
        let mut v_upper = Variant::empty();
        rtcStrConvVar2(&mut v_upper, &v_text, 1, 0);
        assert_eq!(bstr_to_string(v_upper.as_bstr().unwrap()), "HELLO WORLD");

        // vbLowerCase = 2
        let s_upper = sys_alloc_string("HELLO WORLD");
        let v_upper_in = Variant::from_bstr(s_upper);
        let mut v_lower = Variant::empty();
        rtcStrConvVar2(&mut v_lower, &v_upper_in, 2, 0);
        assert_eq!(bstr_to_string(v_lower.as_bstr().unwrap()), "hello world");

        // vbProperCase = 3
        let mut v_proper = Variant::empty();
        rtcStrConvVar2(&mut v_proper, &v_text, 3, 0);
        assert_eq!(bstr_to_string(v_proper.as_bstr().unwrap()), "Hello World");
    }
}
