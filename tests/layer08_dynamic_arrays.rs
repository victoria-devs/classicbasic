//! Layer 08: Dynamic Arrays & SafeArray Manipulation Tests
//!
//! Granular tests for ReDim, ReDim Preserve, LBound, UBound, Split, Join,
//! Filter, Array(), VarIndexLoad, and Erase corresponding to App08-dynamic-arrays.

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
        let items = [Variant::from_i32(100), Variant::from_i32(200), Variant::from_i32(300)];
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
            SafeArrayBound { c_elements: 2, l_lbound: 1 }, // Dimension 1 (1 To 2)
            SafeArrayBound { c_elements: 5, l_lbound: 1 }, // Dimension 2 (1 To 5)
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
            SafeArrayBound { c_elements: 2, l_lbound: 1 },
            SafeArrayBound { c_elements: 10, l_lbound: 1 },
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
            SafeArrayBound { c_elements: 4, l_lbound: 1 },
            SafeArrayBound { c_elements: 10, l_lbound: 1 },
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
