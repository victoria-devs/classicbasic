// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Regression Suite `v0.1.6` — Phase 1.7: Extended Standard Library (`rtc*`) (`App22`)
//!
//! Validates financial suite (`SLN`, `SYD`, `DDB`, `PMT`, `IPMT`, `PPMT`, `PV`, `FV`, `NPer`,
//! `Rate`, `NPV`, `IRR`, `MIRR`), calendar & date mathematics (`DateSerial`, `TimeSerial`, `DateValue`,
//! `TimeValue`, `Day`, `Month`, `Year`, `Hour`, `Minute`, `Second`, `Weekday`, `DateAdd`, `DateDiff`,
//! `DatePart`, `WeekdayName`, `MonthName`, `IsDate`), locale formatters (`FormatNumber`,
//! `FormatCurrency`, `FormatPercent`, `FormatDateTime`), flow utilities (`Switch`, `Partition`),
//! and virtual registry management (`SaveSetting`, `GetSetting`, `GetAllSettings`, `DeleteSetting`).

use msvbvm60::rtc::*;
use msvbvm60::types::*;

#[test]
fn test_financial_depreciation_sln_syd_ddb() {
    unsafe {
        // SLN(cost=10000, salvage=1000, life=5) = (10000 - 1000) / 5 = 1800
        let sln = rtcSLN(10000.0, 1000.0, 5.0);
        assert!((sln - 1800.0).abs() < 1e-6);

        // SYD(cost=10000, salvage=1000, life=5, period=1) = ((10000 - 1000) * (5 - 1 + 1) * 2) / (5 * 6) = 3000
        let syd = rtcSYD(10000.0, 1000.0, 5.0, 1.0);
        assert!((syd - 3000.0).abs() < 1e-6);

        // DDB(cost=10000, salvage=1000, life=5, period=1, factor=2.0) = 10000 * (2/5) = 4000
        let ddb = rtcDDB(10000.0, 1000.0, 5.0, 1.0, core::ptr::null());
        assert!((ddb - 4000.0).abs() < 1e-6);
    }
}

#[test]
fn test_financial_annuities_pmt_ipmt_ppmt_pv_fv_nper_rate() {
    unsafe {
        let rate = 0.08 / 12.0;
        let nper = 120.0;
        let pv = -10000.0;

        let pmt = rtcPMT(rate, nper, pv, core::ptr::null(), core::ptr::null());
        assert!((pmt - 121.3276).abs() < 0.01);

        let ipmt = rtcIPMT(rate, 1.0, nper, pv, core::ptr::null(), core::ptr::null());
        let ppmt = rtcPPMT(rate, 1.0, nper, pv, core::ptr::null(), core::ptr::null());
        assert!(((ipmt + ppmt) - pmt).abs() < 0.001);

        let pv_calc = rtcPV(rate, nper, -pmt, core::ptr::null(), core::ptr::null());
        assert!((pv_calc - 10000.0).abs() < 0.1);

        let fv = rtcFV(rate, nper, -100.0, core::ptr::null(), core::ptr::null());
        assert!(fv > 18000.0);

        let nper_calc = rtcNPer(rate, -pmt, 10000.0, core::ptr::null(), core::ptr::null());
        assert!((nper_calc - 120.0).abs() < 0.1);

        let rate_calc = rtcRate(nper, -pmt, 10000.0, core::ptr::null(), core::ptr::null(), core::ptr::null());
        assert!((rate_calc - rate).abs() < 0.0001);
    }
}

#[test]
fn test_financial_cashflows_npv_irr_mirr() {
    unsafe {
        let cash_flows = [-1000.0f64, 300.0, 400.0, 400.0, 300.0];
        let bounds = [SafeArrayBound {
            c_elements: 5,
            l_lbound: 0,
        }];
        let sa = SafeArray::create_multidim(core::mem::size_of::<f64>() as u32, &bounds, 0);
        core::ptr::copy_nonoverlapping(cash_flows.as_ptr(), (*sa).pv_data as *mut f64, 5);

        let npv = rtcNPV(0.08, sa);
        assert!(npv > 100.0);

        let irr = rtcIRR(sa, core::ptr::null());
        assert!((irr - 0.1388).abs() < 0.01);

        let mirr = rtcMIRR(sa, 0.1, 0.12);
        assert!(mirr > 0.0);
    }
}

#[test]
fn test_calendar_and_date_math() {
    unsafe {
        let mut d_leap = Variant::empty();
        rtcPackDate(&mut d_leap, 2024, 2, 29);
        assert_eq!(rtcGetDayOfMonth(&d_leap), 29);
        assert_eq!(rtcGetMonthOfYear(&d_leap), 2);
        assert_eq!(rtcGetYear(&d_leap), 2024);

        let mut t1 = Variant::empty();
        rtcPackTime(&mut t1, 14, 35, 42);
        assert_eq!(rtcGetHourOfDay(&t1), 14);
        assert_eq!(rtcGetMinuteOfHour(&t1), 35);
        assert_eq!(rtcGetSecondOfMinute(&t1), 42);

        let mut d_val = Variant::empty();
        let b_dt_str = sys_alloc_string("2025-10-17");
        rtcGetDateValue(&mut d_val, b_dt_str);
        SysFreeString(b_dt_str);
        assert_eq!(rtcGetYear(&d_val), 2025);
        assert_eq!(rtcGetMonthOfYear(&d_val), 10);
        assert_eq!(rtcGetDayOfMonth(&d_val), 17);

        let mut d1 = Variant::empty();
        rtcPackDate(&mut d1, 2025, 1, 15);

        let mut d2 = Variant::empty();
        let b_m = sys_alloc_string("m");
        rtcDateAdd(&mut d2, b_m, 3.0, &d1);
        SysFreeString(b_m);
        assert_eq!(rtcGetMonthOfYear(&d2), 4);
        assert_eq!(rtcGetDayOfMonth(&d2), 15);

        let mut d3 = Variant::empty();
        let b_d = sys_alloc_string("d");
        rtcDateAdd(&mut d3, b_d, 20.0, &d1);
        assert_eq!(rtcGetMonthOfYear(&d3), 2);
        assert_eq!(rtcGetDayOfMonth(&d3), 4);

        let diff = rtcDateDiff(b_d, &d1, &d3, core::ptr::null(), core::ptr::null());
        SysFreeString(b_d);
        assert_eq!(diff, 20);

        let b_yyyy = sys_alloc_string("yyyy");
        let part_y = rtcDatePart(b_yyyy, &d1, core::ptr::null(), core::ptr::null());
        SysFreeString(b_yyyy);
        assert_eq!(part_y, 2025);

        let mut d_sun = Variant::empty();
        rtcPackDate(&mut d_sun, 2025, 1, 5);
        let v_first = Variant::from_i16(1);
        assert_eq!(rtcGetDayOfWeek(&d_sun, &v_first), 1);

        let b_sun = rtcWeekdayName(1, 0, 1);
        assert_eq!(bstr_to_string(b_sun), "Sunday");
        SysFreeString(b_sun);

        let b_oct = rtcMonthName(10, 0);
        assert_eq!(bstr_to_string(b_oct), "October");
        SysFreeString(b_oct);

        let v_valid = Variant::from_bstr(sys_alloc_string("2025-10-17"));
        assert_eq!(rtcIsDate(&v_valid), -1);

        let v_invalid = Variant::from_bstr(sys_alloc_string("invalid_date"));
        assert_eq!(rtcIsDate(&v_invalid), 0);
    }
}

#[test]
fn test_locale_formatters() {
    unsafe {
        let v_num = Variant::from_f64(1234567.89);
        let b_num = rtcFormatNumber(&v_num, 2, -2, -2, -2);
        let s_num = bstr_to_string(b_num);
        SysFreeString(b_num);
        assert!(s_num.contains("1") && s_num.contains("234"));

        let v_cur = Variant::from_f64(1234.5);
        let b_cur = rtcFormatCurrency(&v_cur, 2, -2, -2, -2);
        let s_cur = bstr_to_string(b_cur);
        SysFreeString(b_cur);
        assert!(s_cur.contains("1") && s_cur.contains("234"));

        let v_pct = Variant::from_f64(0.125);
        let b_pct = rtcFormatPercent(&v_pct, 1, -2, -2, -2);
        let s_pct = bstr_to_string(b_pct);
        SysFreeString(b_pct);
        assert!(s_pct.contains("12.5") && s_pct.contains("%"));

        let mut dt_combined = Variant::empty();
        rtcPackDate(&mut dt_combined, 2025, 5, 20);
        let b_dt = rtcFormatDateTime(&dt_combined, 2);
        let s_dt = bstr_to_string(b_dt);
        SysFreeString(b_dt);
        assert!(s_dt.contains("2025") || s_dt.contains("25"));
    }
}

#[test]
fn test_utility_switch_and_partition() {
    unsafe {
        let switch_args = [
            Variant::from_bool(false),
            Variant::from_bstr(sys_alloc_string("First")),
            Variant::from_bool(true),
            Variant::from_bstr(sys_alloc_string("Second")),
            Variant::from_bool(true),
            Variant::from_bstr(sys_alloc_string("Third")),
        ];
        let bounds = [SafeArrayBound {
            c_elements: 6,
            l_lbound: 0,
        }];
        let sa = SafeArray::create_multidim(
            core::mem::size_of::<Variant>() as u32,
            &bounds,
            FADF_VARIANT,
        );
        core::ptr::copy_nonoverlapping(switch_args.as_ptr(), (*sa).pv_data as *mut Variant, 6);
        let mut sw_res = Variant::empty();
        rtcSwitch(&mut sw_res, sa);
        assert_eq!(sw_res.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default(), "Second");

        let v_num = Variant::from_i32(54);
        let v_st = Variant::from_i32(0);
        let v_sp = Variant::from_i32(99);
        let v_inv = Variant::from_i32(10);
        let mut part_res = Variant::empty();
        rtcPartition(&mut part_res, &v_num, &v_st, &v_sp, &v_inv);
        let part_str = part_res.as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default();
        assert!(part_str.contains("50") && part_str.contains("59"));
    }
}

#[test]
fn test_virtual_registry_settings() {
    unsafe {
        let b_app = sys_alloc_string("ClassicBasicApp22Test");
        let b_sec = sys_alloc_string("Config");
        let b_k = sys_alloc_string("Server");
        let b_v = sys_alloc_string("127.0.0.1");

        rtcSaveSetting(b_app, b_sec, b_k, b_v);

        let v_def = Variant::from_bstr(sys_alloc_string("localhost"));
        let got = rtcGetSetting(b_app, b_sec, b_k, &v_def);
        assert_eq!(bstr_to_string(got), "127.0.0.1");
        SysFreeString(got);

        let mut all = Variant::empty();
        rtcGetAllSettings(&mut all, b_app, b_sec);
        assert_ne!(all.vt & VT_ARRAY, 0);

        let v_sec = Variant::from_bstr(b_sec);
        let v_k = Variant::from_bstr(b_k);
        rtcDeleteSetting(b_app, &v_sec, &v_k);

        let got_after = rtcGetSetting(b_app, b_sec, b_k, &v_def);
        assert_eq!(bstr_to_string(got_after), "localhost");
        SysFreeString(got_after);

        rtcDeleteSetting(b_app, core::ptr::null(), core::ptr::null());
        SysFreeString(b_app);
        SysFreeString(b_sec);
        SysFreeString(b_k);
        SysFreeString(b_v);
    }
}
