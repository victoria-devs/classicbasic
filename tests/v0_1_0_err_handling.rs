//! Regression Suite v0.1.0 (Phase 1.1): Structured Error Handling & COM ErrObject Tests
//!
//! Granular tests for On Error modes, Err.Number, Err.Description, Err.Source,
//! Err.Clear, Err.Raise, COM IDispatch VTable, and __vbaObjSet corresponding to App06-err-handling.

use msvbvm60::err::*;
use msvbvm60::types::*;

static ERR_TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_vba_on_error_modes() {
    let _guard = ERR_TEST_MUTEX.lock().unwrap();
    unsafe {
        __vbaOnError((-1isize) as *mut _); // On Error Resume Next
        assert_eq!(get_vb_error_handler(), -1);

        __vbaOnError(0 as *mut _); // On Error GoTo 0
        assert_eq!(get_vb_error_handler(), 0);

        __vbaOnError(100 as *mut _); // On Error GoTo <Label>
        assert_eq!(get_vb_error_handler(), 1);
    }
}

#[test]
fn test_set_and_clear_vb_error() {
    let _guard = ERR_TEST_MUTEX.lock().unwrap();
    set_vb_error(11, None, None);
    assert_eq!(get_vb_error_number(), 11);
    assert_eq!(get_vb_error_description(), "Division by zero");

    clear_vb_error();
    assert_eq!(get_vb_error_number(), 0);
    assert_eq!(get_vb_error_description(), "");
}

#[test]
fn test_standard_error_descriptions() {
    assert_eq!(get_standard_error_description(11), "Division by zero");
    assert_eq!(get_standard_error_description(13), "Type mismatch");
    assert_eq!(get_standard_error_description(9), "Subscript out of range");
    assert_eq!(get_standard_error_description(53), "File not found");
}

#[test]
fn test_rtc_err_obj_com_vtable_query_interface() {
    unsafe {
        let err_ptr = rtcErrObj();
        assert!(!err_ptr.is_null());
        let err_inst = &*(err_ptr as *const ErrObjectInstance);
        let vtable = &*err_inst.vtable;

        let mut pv: *mut core::ffi::c_void = core::ptr::null_mut();
        let hr = (vtable.QueryInterface)(err_ptr, core::ptr::null(), &mut pv);
        assert_eq!(hr, 0);
        assert_eq!(pv, err_ptr);
    }
}

#[test]
fn test_rtc_err_obj_com_vtable_addref_and_release() {
    unsafe {
        let err_ptr = rtcErrObj();
        let err_inst = &*(err_ptr as *const ErrObjectInstance);
        let vtable = &*err_inst.vtable;

        assert_eq!((vtable.AddRef)(err_ptr), 1);
        assert_eq!((vtable.Release)(err_ptr), 1);
    }
}

#[test]
fn test_rtc_err_obj_put_and_get_number() {
    let _guard = ERR_TEST_MUTEX.lock().unwrap();
    unsafe {
        let err_ptr = rtcErrObj();
        let err_inst = &*(err_ptr as *const ErrObjectInstance);
        let vtable = &*err_inst.vtable;

        (vtable.put_Number)(err_ptr, 13);
        let mut num: i32 = 0;
        (vtable.get_Number)(err_ptr, &mut num);
        assert_eq!(num, 13);
        assert_eq!(get_vb_error_description(), "Type mismatch");
        (vtable.Clear)(err_ptr);
    }
}

#[test]
fn test_rtc_err_obj_raise_and_clear() {
    let _guard = ERR_TEST_MUTEX.lock().unwrap();
    unsafe {
        let err_ptr = rtcErrObj();
        let err_inst = &*(err_ptr as *const ErrObjectInstance);
        let vtable = &*err_inst.vtable;

        let b_source = sys_alloc_string("BillingService");
        let b_desc = sys_alloc_string("Invoice generation failed");
        let v_src = Variant::from_bstr(b_source);
        let v_dsc = Variant::from_bstr(b_desc);

        (vtable.Raise)(
            err_ptr,
            1001,
            &v_src,
            &v_dsc,
            core::ptr::null(),
            core::ptr::null(),
        );

        assert_eq!(get_vb_error_number(), 1001);
        assert_eq!(get_vb_error_source(), "BillingService");
        assert_eq!(get_vb_error_description(), "Invoice generation failed");

        (vtable.Clear)(err_ptr);
        assert_eq!(get_vb_error_number(), 0);

        SysFreeString(b_source);
        SysFreeString(b_desc);
    }
}

#[test]
fn test_vba_obj_set_and_free() {
    unsafe {
        let err_ptr = rtcErrObj();
        let mut slot: *mut core::ffi::c_void = core::ptr::null_mut();
        let ret = __vbaObjSet(&mut slot, err_ptr);
        assert_eq!(ret, err_ptr);
        assert_eq!(slot, err_ptr);

        __vbaFreeObj(&mut slot);
        assert!(slot.is_null());
    }
}

#[test]
fn test_vba_resume_and_exit_proc() {
    unsafe {
        __vbaResume(0);
        __vbaExitProc();
    }
}

#[test]
fn test_on_error_resume_next_sets_errobject_and_continues_execution() {
    let _guard = ERR_TEST_MUTEX.lock().unwrap();
    unsafe {
        // GIVEN an active __vbaOnError handler configured for Resume Next
        __vbaOnError((-1isize) as *mut _);
        assert_eq!(get_vb_error_handler(), -1);

        // WHEN a division by zero error occurs (__vbaVarDiv by 0)
        let v_num = Variant::from_f64(10.0);
        let v_zero = Variant::from_f64(0.0);
        let mut v_res = Variant::empty();
        msvbvm60::vba::__vbaVarDiv(&mut v_res, &v_zero, &v_num);

        // THEN execution does not terminate
        // AND Err.Number is populated with 11 (Division by zero)
        assert_eq!(get_vb_error_number(), 11);
        // AND Err.Description contains "Division by zero"
        assert_eq!(get_vb_error_description(), "Division by zero");

        // AND Err.Clear resets Err.Number back to 0
        clear_vb_error();
        assert_eq!(get_vb_error_number(), 0);
        assert_eq!(get_vb_error_description(), "");
    }
}
