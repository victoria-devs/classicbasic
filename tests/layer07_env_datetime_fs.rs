//! Layer 07: Environment, Date/Time & Filesystem Tests
//!
//! Granular tests for Environ$, CurDir$, Date$, Time$, Timer, Now,
//! MkDir, RmDir, and date math helpers corresponding to App07-env-datetime-fs.

use msvbvm60::rtc::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;

#[test]
fn test_rtc_environ_bstr() {
    unsafe {
        let var_name = sys_alloc_string("PATH");
        let val = rtcEnvironBstr(var_name);
        let s = bstr_to_string(val);
        assert!(!s.is_empty());
        SysFreeString(var_name);
        SysFreeString(val);
    }
}

#[test]
fn test_rtc_current_dir_bstr() {
    unsafe {
        let val = rtcCurrentDirBstr(core::ptr::null_mut());
        let s = bstr_to_string(val);
        assert!(!s.is_empty());
        SysFreeString(val);
    }
}

#[test]
fn test_rtc_get_date_bstr_format() {
    unsafe {
        let val = rtcGetDateBstr();
        let s = bstr_to_string(val);
        // MM-DD-YYYY has 10 characters
        assert_eq!(s.len(), 10);
        SysFreeString(val);
    }
}

#[test]
fn test_rtc_get_time_bstr_format() {
    unsafe {
        let val = rtcGetTimeBstr();
        let s = bstr_to_string(val);
        // HH:MM:SS has 8 characters
        assert_eq!(s.len(), 8);
        SysFreeString(val);
    }
}

#[test]
fn test_rtc_get_timer_bounds() {
    unsafe {
        let t = rtcGetTimer();
        // Seconds since midnight: 0.0 .. 86400.0
        assert!(t >= 0.0 && t <= 86400.0);
    }
}

#[test]
fn test_rtc_get_present_date_now() {
    unsafe {
        let mut v_now = Variant::empty();
        rtcGetPresentDate(&mut v_now);
        assert_eq!(v_now.vt, VT_DATE);
        let serial = __vbaDateVar(&v_now);
        assert!(serial > 25569.0); // Past 1970-01-01
    }
}

#[test]
fn test_rtc_make_and_remove_dir() {
    unsafe {
        let dir_name = sys_alloc_string("test_layer07_dir");
        rtcMakeDir(dir_name);
        assert!(std::path::Path::new("test_layer07_dir").is_dir());

        rtcRemoveDir(dir_name);
        assert!(!std::path::Path::new("test_layer07_dir").exists());

        SysFreeString(dir_name);
    }
}
