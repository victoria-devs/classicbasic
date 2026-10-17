// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Regression Suite v0.1.4 (Phase 1.5): Loops, Iterators & Flow Control Tests
//!
//! Validates:
//! - Typed For..Next loop checks: `__vbaI2ForNextCheck`, `__vbaI4ForNextCheck`, `__vbaR4ForNextCheck`, `__vbaR8ForNextCheck`.
//! - Currency 64-bit fixed-point loops: `__vbaCyForInit`, `__vbaCyForNext`.
//! - Variant dynamic loops with arbitrary stepping: `__vbaVarForInit`, `__vbaVarForNext`.
//! - SafeArray For Each iteration with lock tracking: `__vbaForEachAry`, `__vbaNextEachAry`, `__vbaExitEachAry`.
//! - Variant-wrapped array For Each iteration: `__vbaForEachVar`, `__vbaNextEachVar`, `__vbaExitEachVar`.
//! - Local subroutine call stack management: `__vbaGosub`, `__vbaGosubReturn`, `__vbaGosubFree`.
//! - Computed branching index validation: `__vbaOnGoCheck` (valid bounds [0, 255], Error 5 on overflow/underflow).
//! - Type narrowing and conversion helpers: `__vbaI2I4`, `__vbaR8ErrVar`.

use msvbvm60::err::{clear_vb_error, get_vb_error_number};
use msvbvm60::types::*;
use msvbvm60::vba::*;

#[test]
fn test_typed_for_next_checks_integer_long_single_double() {
    unsafe {
        // 1. Integer (i16) For..Next: 1 to 10 step 2
        let mut i2_val: i16 = 1;
        let mut i2_count = 0;
        let mut i2_sum = 0;
        while __vbaI2ForNextCheck(i2_val, 10, 2) != 0 {
            i2_sum += i2_val as i32;
            i2_count += 1;
            i2_val += 2;
        }
        assert_eq!(i2_count, 5);
        assert_eq!(i2_sum, 25); // 1 + 3 + 5 + 7 + 9

        // 2. Long (i32) For..Next negative step: 100 to 50 step -10
        let mut i4_val: i32 = 100;
        let mut i4_count = 0;
        while __vbaI4ForNextCheck(i4_val, 50, -10) != 0 {
            i4_count += 1;
            i4_val -= 10;
        }
        assert_eq!(i4_count, 6); // 100, 90, 80, 70, 60, 50

        // Zero-trip loop
        assert_eq!(__vbaI4ForNextCheck(10, 1, 1), 0);
        assert_eq!(__vbaI4ForNextCheck(1, 10, -1), 0);

        // 3. Single (f32) For..Next: 0.5 to 2.5 step 0.5
        let mut r4_val: f32 = 0.5;
        let mut r4_count = 0;
        while __vbaR4ForNextCheck(r4_val, 2.5, 0.5) != 0 {
            r4_count += 1;
            r4_val += 0.5;
        }
        assert_eq!(r4_count, 5);

        // 4. Double (f64) For..Next: 10.0 to 2.0 step -2.0
        let mut r8_val: f64 = 10.0;
        let mut r8_count = 0;
        while __vbaR8ForNextCheck(r8_val, 2.0, -2.0) != 0 {
            r8_count += 1;
            r8_val -= 2.0;
        }
        assert_eq!(r8_count, 5);
    }
}

#[test]
fn test_currency_for_next_init_and_iteration() {
    unsafe {
        // Currency 10.0000@ to 25.0000@ step 5.0000@
        let mut cy_counter: i64 = 0;
        let cy_start = 100_000i64; // 10.0000
        let cy_end = 250_000i64;   // 25.0000
        let cy_step = 50_000i64;   // 5.0000

        let mut cy_count = 0;
        let mut cy_total = 0i64;

        let should_run = __vbaCyForInit(&mut cy_counter, cy_start, cy_end, cy_step);
        assert_ne!(should_run, 0);
        assert_eq!(cy_counter, cy_start);

        loop {
            cy_total += cy_counter;
            cy_count += 1;
            if __vbaCyForNext(&mut cy_counter, cy_end, cy_step) == 0 {
                break;
            }
        }

        assert_eq!(cy_count, 4);
        assert_eq!(cy_total, 700_000i64); // (10 + 15 + 20 + 25) * 10000

        // Zero-trip Currency loop
        let mut zero_counter: i64 = 0;
        assert_eq!(__vbaCyForInit(&mut zero_counter, 100_000, 10_000, 10_000), 0);
    }
}

#[test]
fn test_variant_for_next_init_and_iteration() {
    unsafe {
        // Variant loop: 1 to 3 Step 0.5
        let mut counter = Variant::empty();
        let mut temp_step = Variant::empty();
        let mut temp_end = Variant::empty();

        let v_start = Variant::from_i32(1);
        let v_end = Variant::from_f64(3.0);
        let v_step = Variant::from_f64(0.5);

        let init_res = __vbaVarForInit(
            &mut counter,
            &mut temp_step,
            &mut temp_end,
            &v_start,
            &v_end,
            &v_step,
        );
        assert_ne!(init_res, 0);
        assert_eq!(counter.as_f64().unwrap(), 1.0);

        let mut count = 0;
        let mut sum = 0.0f64;
        loop {
            sum += counter.as_f64().unwrap();
            count += 1;
            if __vbaVarForNext(&mut counter, &mut temp_step, &mut temp_end) == 0 {
                break;
            }
        }
        assert_eq!(count, 5); // 1.0, 1.5, 2.0, 2.5, 3.0
        assert!((sum - 10.0).abs() < 1e-6);

        // Negative step Variant loop: 10 to 2 Step -2
        let v_neg_start = Variant::from_i32(10);
        let v_neg_end = Variant::from_i32(2);
        let v_neg_step = Variant::from_i32(-2);

        let mut count_neg = 0;
        if __vbaVarForInit(
            &mut counter,
            &mut temp_step,
            &mut temp_end,
            &v_neg_start,
            &v_neg_end,
            &v_neg_step,
        ) != 0
        {
            loop {
                count_neg += 1;
                if __vbaVarForNext(&mut counter, &mut temp_step, &mut temp_end) == 0 {
                    break;
                }
            }
        }
        assert_eq!(count_neg, 5); // 10, 8, 6, 4, 2
    }
}

#[test]
fn test_for_each_safearray_and_exit_each_ary_lock_management() {
    unsafe {
        let bstr_size = core::mem::size_of::<Bstr>() as u32;
        let psa = SafeArray::create_vector(bstr_size, 1, 4, FADF_HAVEVARTYPE | FADF_BSTR);
        assert!(!psa.is_null());

        let words = ["First", "Second", "Third", "Fourth"];
        for (idx, w) in words.iter().enumerate() {
            let bstr = sys_alloc_string(w);
            let slice = core::slice::from_raw_parts_mut((*psa).pv_data as *mut Bstr, 4);
            slice[idx] = bstr;
        }

        // Full iteration
        let mut enum_state = VbAryEnumState::default();
        let mut item = Variant::empty();
        let mut items = Vec::new();

        let has_items = __vbaForEachAry(
            8, // VT_BSTR
            &mut enum_state as *mut _ as *mut core::ffi::c_void,
            core::ptr::null_mut(),
            &mut item,
            psa,
        );
        assert_ne!(has_items, 0);
        assert_eq!((*psa).c_locks, 1); // Array locked during traversal

        loop {
            items.push(item.to_string_representation());
            if __vbaNextEachAry(
                8,
                core::ptr::null_mut(),
                &mut item,
                &mut enum_state as *mut _ as *mut core::ffi::c_void,
            ) == 0
            {
                break;
            }
        }

        assert_eq!(items, vec!["First", "Second", "Third", "Fourth"]);
        assert_eq!((*psa).c_locks, 0); // Lock cleanly released

        // Early exit iteration using __vbaExitEachAry
        let mut exit_state = VbAryEnumState::default();
        let mut item_exit = Variant::empty();

        let init = __vbaForEachAry(
            8,
            &mut exit_state as *mut _ as *mut core::ffi::c_void,
            core::ptr::null_mut(),
            &mut item_exit,
            psa,
        );
        assert_ne!(init, 0);
        assert_eq!((*psa).c_locks, 1);

        // Early Exit after 1st element
        __vbaExitEachAry(&mut exit_state as *mut _ as *mut core::ffi::c_void);
        assert_eq!((*psa).c_locks, 0); // Lock freed on early exit

        SafeArray::destroy(psa);
    }
}

#[test]
fn test_for_each_variant_wrapping_array_and_exit_each_var() {
    unsafe {
        let psa = SafeArray::create_vector(4, 0, 3, FADF_HAVEVARTYPE);
        let slice = core::slice::from_raw_parts_mut((*psa).pv_data as *mut i32, 3);
        for idx in 0..3 {
            slice[idx] = (idx as i32 + 1) * 100;
        }

        let mut v_arr = Variant::empty();
        v_arr.vt = VT_ARRAY | VT_I4;
        v_arr.set_data_ptr(psa as usize);

        let mut vs_state = [0usize; 4];
        let mut item = Variant::empty();
        let mut sum = 0;

        let init = __vbaForEachVar(
            vs_state.as_mut_ptr() as *mut core::ffi::c_void,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            &mut item,
            &mut v_arr,
        );
        assert_ne!(init, 0);
        assert_eq!((*psa).c_locks, 1);

        loop {
            sum += item.as_i32().unwrap();
            if __vbaNextEachVar(
                vs_state.as_mut_ptr() as *mut core::ffi::c_void,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut item,
            ) == 0
            {
                break;
            }
        }
        assert_eq!(sum, 600); // 100 + 200 + 300
        assert_eq!((*psa).c_locks, 0);

        // Test early exit on Variant array
        let mut es_state = [0usize; 4];
        let mut e_item = Variant::empty();
        __vbaForEachVar(
            es_state.as_mut_ptr() as *mut core::ffi::c_void,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            &mut e_item,
            &mut v_arr,
        );
        assert_eq!((*psa).c_locks, 1);

        __vbaExitEachVar(
            es_state.as_mut_ptr() as *mut core::ffi::c_void,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        );
        assert_eq!((*psa).c_locks, 0);

        SafeArray::destroy(psa);
    }
}

#[test]
fn test_local_gosub_return_and_gosub_free() {
    unsafe {
        let mut frame: usize = 0;

        // Nested GoSub
        __vbaGosub(&mut frame as *mut usize as *mut core::ffi::c_void);
        assert_eq!(frame, 1);

        __vbaGosub(&mut frame as *mut usize as *mut core::ffi::c_void);
        assert_eq!(frame, 2);

        __vbaGosubReturn(&mut frame as *mut usize as *mut core::ffi::c_void);
        assert_eq!(frame, 1);

        __vbaGosubReturn(&mut frame as *mut usize as *mut core::ffi::c_void);
        assert_eq!(frame, 0);

        // GoSubFree on procedure exit with pending GoSub frames
        __vbaGosub(&mut frame as *mut usize as *mut core::ffi::c_void);
        __vbaGosub(&mut frame as *mut usize as *mut core::ffi::c_void);
        assert_eq!(frame, 2);

        __vbaGosubFree(&mut frame as *mut usize as *mut core::ffi::c_void);
        assert_eq!(frame, 0);
    }
}

#[test]
fn test_on_go_check_index_validation() {
    unsafe {
        // Valid indices [0, 255]
        assert_eq!(__vbaOnGoCheck(0), 0);
        assert_eq!(__vbaOnGoCheck(1), 1);
        assert_eq!(__vbaOnGoCheck(5), 5);
        assert_eq!(__vbaOnGoCheck(255), 255);

        // Out of range indices must raise Runtime Error 5 (Invalid procedure call)
        clear_vb_error();
        let _ = __vbaOnGoCheck(-1);
        assert_eq!(get_vb_error_number(), 5);
        clear_vb_error();

        let _ = __vbaOnGoCheck(-100);
        assert_eq!(get_vb_error_number(), 5);
        clear_vb_error();

        let _ = __vbaOnGoCheck(256);
        assert_eq!(get_vb_error_number(), 5);
        clear_vb_error();

        let _ = __vbaOnGoCheck(10000);
        assert_eq!(get_vb_error_number(), 5);
        clear_vb_error();
    }
}

#[test]
fn test_vba_i2i4_conversion_and_vba_r8_err_var() {
    unsafe {
        // __vbaI2I4 within range
        assert_eq!(__vbaI2I4(0), 0);
        assert_eq!(__vbaI2I4(32767), 32767);
        assert_eq!(__vbaI2I4(-32768), -32768);

        // __vbaI2I4 overflow check (Error 6)
        clear_vb_error();
        let _ = __vbaI2I4(32768);
        assert_eq!(get_vb_error_number(), 6);
        clear_vb_error();

        let _ = __vbaI2I4(-32769);
        assert_eq!(get_vb_error_number(), 6);
        clear_vb_error();

        // __vbaR8ErrVar valid
        let v_flt = Variant::from_f64(3.14159);
        let flt_res = __vbaR8ErrVar(&v_flt);
        assert!((flt_res - 3.14159).abs() < 1e-5);

        let v_str = Variant::from_bstr(sys_alloc_string("42.5"));
        let str_res = __vbaR8ErrVar(&v_str);
        assert!((str_res - 42.5).abs() < 1e-5);

        // __vbaR8ErrVar invalid string (Error 13 Type Mismatch)
        clear_vb_error();
        let v_bad_str = Variant::from_bstr(sys_alloc_string("NotANumber"));
        let _ = __vbaR8ErrVar(&v_bad_str);
        assert_eq!(get_vb_error_number(), 13);
        clear_vb_error();
    }
}
