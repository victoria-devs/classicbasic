// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Runtime Library: Mathematics & Financial Engines
//!
//! Bankers rounding (__vbaRound), Fix vs Int negative truncation,
//! trigonometric FPU intrinsics, and financial annuities (Pmt, NPV, IRR, DDB).

use msvbvm60::err::*;
use msvbvm60::rtc::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;
use msvbvm60::*;




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




#[test]
fn test_bankers_rounding_exact_tie_breaking_for_integers_and_currency() {
    // GIVEN values exactly halfway between two integers: 1.5, 2.5, 3.5, 4.5, -2.5, -3.5
    // WHEN passed to CInt, CLng, or Round()
    // THEN 1.5 must round to 2 (even)
    // AND 2.5 must round to 2 (even)
    // AND 3.5 must round to 4 (even)
    // AND 4.5 must round to 4 (even)
    // AND -2.5 must round to -2 (even), strictly adhering to IEEE 754 Round-to-Nearest-Even.
    // AND -3.5 must round to -4 (even).
    unsafe {
        // CInt (__vbaFpI2)
        assert_eq!(__vbaFpI2(1.5), 2);
        assert_eq!(__vbaFpI2(2.5), 2);
        assert_eq!(__vbaFpI2(3.5), 4);
        assert_eq!(__vbaFpI2(4.5), 4);
        assert_eq!(__vbaFpI2(-2.5), -2);
        assert_eq!(__vbaFpI2(-3.5), -4);

        // CLng (__vbaFpI4)
        assert_eq!(__vbaFpI4(1.5), 2);
        assert_eq!(__vbaFpI4(2.5), 2);
        assert_eq!(__vbaFpI4(3.5), 4);
        assert_eq!(__vbaFpI4(4.5), 4);
        assert_eq!(__vbaFpI4(-2.5), -2);
        assert_eq!(__vbaFpI4(-3.5), -4);

        // Core round_to_even
        assert_eq!(round_to_even(1.5) as i64, 2);
        assert_eq!(round_to_even(2.5) as i64, 2);
        assert_eq!(round_to_even(3.5) as i64, 4);
        assert_eq!(round_to_even(4.5) as i64, 4);
        assert_eq!(round_to_even(-2.5) as i64, -2);
        assert_eq!(round_to_even(-3.5) as i64, -4);

        // rtcRound
        let v_1_5 = Variant::from_f64(1.5);
        let mut v_res = Variant::empty();
        rtcRound(&mut v_res, &v_1_5, 0);
        assert_eq!(v_res.as_i32().unwrap(), 2);

        let v_2_5 = Variant::from_f64(2.5);
        rtcRound(&mut v_res, &v_2_5, 0);
        assert_eq!(v_res.as_i32().unwrap(), 2);

        let v_3_5 = Variant::from_f64(3.5);
        rtcRound(&mut v_res, &v_3_5, 0);
        assert_eq!(v_res.as_i32().unwrap(), 4);

        let v_4_5 = Variant::from_f64(4.5);
        rtcRound(&mut v_res, &v_4_5, 0);
        assert_eq!(v_res.as_i32().unwrap(), 4);

        let v_neg_2_5 = Variant::from_f64(-2.5);
        rtcRound(&mut v_res, &v_neg_2_5, 0);
        assert_eq!(v_res.as_i32().unwrap(), -2);

        // Currency from_f64 banker's rounding on fractional cents
        let c1 = Currency::from_f64(1.00015);
        assert_eq!(c1.0, 10002);
        let c2 = Currency::from_f64(1.00025);
        assert_eq!(c2.0, 10002);
    }
}

#[test]
fn test_crt_fpu_pentium_fdiv_workaround_routines_conform_to_c2_abi() {
    // GIVEN compiler-generated calls to FDIV workaround exports:
    // _adj_fdiv_m32, _adj_fdiv_m64, _adj_fdiv_r, _adj_fdiv16i, _adj_fprem, _adj_fprem1
    // WHEN executed with floating point numerators and denominators on the virtual x87 stack
    // THEN the returned quotient must match MSVC C2 runtime precision bit-for-bit
    // AND the FPU status word condition codes (C0, C1, C2, C3) must be set identically.
    let mut fpu = VirtualFpu::new();

    // 1. _adj_fdiv_m32
    fpu.push(100.0);
    fpu.fdiv_m32(4.0f32);
    assert_eq!(fpu.st(0), 25.0);

    // 2. _adj_fdiv_m64
    fpu.push(10.0);
    fpu.fdiv_m64(2.5);
    assert_eq!(fpu.st(0), 4.0);

    // 3. _adj_fdiv_m16i
    fpu.push(42.0);
    fpu.fdiv_m16i(7);
    assert_eq!(fpu.st(0), 6.0);

    // 4. _adj_fdiv_r: divides st(0) by st(1)
    fpu.push(5.0);  // st(1) = 5.0
    fpu.push(20.0); // st(0) = 20.0
    fpu.fdiv_r();
    assert_eq!(fpu.st(0), 4.0);

    // 5. _adj_fprem: st(0) % st(1) with condition codes
    fpu.push(3.0);  // st(1) = 3.0
    fpu.push(11.0); // st(0) = 11.0
    fpu.fprem();
    assert_eq!(fpu.st(0), 2.0); // 11 % 3 = 2, quotient = 3 (0b011) -> C1=1, C3=1, C0=0
    assert_eq!(fpu.status_word & VirtualFpu::C0, 0);
    assert_ne!(fpu.status_word & VirtualFpu::C3, 0);
    assert_ne!(fpu.status_word & VirtualFpu::C1, 0);
    assert_eq!(fpu.status_word & VirtualFpu::C2, 0);

    // 6. _adj_fprem1: IEEE remainder
    fpu.push(4.0); // st(1) = 4.0
    fpu.push(7.0); // st(0) = 7.0
    fpu.fprem1();
    assert_eq!(fpu.st(0), -1.0); // 7 - round(7/4)*4 = 7 - 2*4 = -1.0
}

#[test]
fn test_floating_point_modulo_and_integer_division_truncation_rules() {
    // GIVEN floating point arguments passed to Integer Division '\' (__vbaVarIdiv) and 'Mod' (__vbaVarMod)
    // WHEN evaluating 10.8 \ 2.4
    // THEN operands are rounded to Long first (11 \ 2) yielding 5
    // WHEN evaluating 10.6 Mod 3.2
    // THEN operands are rounded to Long first (11 Mod 3) yielding 2
    // AND division by zero in both operators must reliably raise Runtime Error 11 (Division by zero).
    unsafe {
        let v_10_8 = Variant::from_f64(10.8);
        let v_2_4 = Variant::from_f64(2.4);
        let mut v_idiv = Variant::empty();
        __vbaVarIdiv(&mut v_idiv, &v_2_4, &v_10_8);
        assert_eq!(v_idiv.as_i32().unwrap(), 5);

        let v_10_6 = Variant::from_f64(10.6);
        let v_3_2 = Variant::from_f64(3.2);
        let mut v_mod = Variant::empty();
        __vbaVarMod(&mut v_mod, &v_3_2, &v_10_6);
        assert_eq!(v_mod.as_i32().unwrap(), 2);

        // Banker's tie-breaking: 11.5 rounds to 12; 2.5 rounds to 2 -> 12 \ 2 = 6
        let v_11_5 = Variant::from_f64(11.5);
        let v_2_5 = Variant::from_f64(2.5);
        let mut v_tie = Variant::empty();
        __vbaVarIdiv(&mut v_tie, &v_2_5, &v_11_5);
        assert_eq!(v_tie.as_i32().unwrap(), 6);

        // Division by zero traps with Error 11
        clear_vb_error();
        let v_10 = Variant::from_i32(10);
        let v_0 = Variant::from_i32(0);
        let mut v_err = Variant::empty();
        __vbaVarIdiv(&mut v_err, &v_0, &v_10);
        assert_eq!(get_vb_error_number(), 11, "Idiv by zero must raise Error 11");

        clear_vb_error();
        __vbaVarMod(&mut v_err, &v_0, &v_10);
        assert_eq!(get_vb_error_number(), 11, "Mod by zero must raise Error 11");
        clear_vb_error();
    }
}

#[test]
fn test_currency_multiplication_and_division_intermediate_precision() {
    // GIVEN high-magnitude Currency values (scaled 64-bit integers with 4 decimal places)
    // WHEN performing multiplication followed by division: (10000@ * 30000@) / 10000@
    // THEN arithmetic must execute using 128-bit integer intermediate precision
    // AND prevent premature 64-bit overflow while preserving small fractions (e.g. 1.2345@ * 2 = 2.469@).
    let cur_a = Currency::from_scaled(10_000 * 10_000); // 10000.0000@
    let cur_b = Currency::from_scaled(30_000 * 10_000); // 30000.0000@

    let cur_prod = cur_a.mul(cur_b);
    let cur_res = cur_prod.div(cur_a);
    assert_eq!(cur_res.0, 30_000 * 10_000, "Currency 128-bit mul/div must not overflow");

    // Fraction preservation
    let cur_f1 = Currency::from_f64(1.2345);
    let cur_f2 = Currency::from_f64(2.0);
    let cur_fprod = cur_f1.mul(cur_f2);
    assert_eq!(cur_fprod.0, 24_690, "1.2345 * 2.0 = 2.4690");

    // Runtime helpers __vbaCyMul and __vbaCyMulI2
    unsafe {
        let v_cy1 = __vbaCyMul(10_000 * 10_000, 30_000 * 10_000);
        assert_eq!(v_cy1, 300_000_000 * 10_000);

        let v_cy2 = __vbaCyMulI2(15_000, 3);
        assert_eq!(v_cy2, 45_000);
    }
}

#[test]
fn test_transcendental_functions_edge_cases_and_overflow_traps() {
    // GIVEN intrinsic RTC transcendental math functions (Sin, Cos, Tan, Atn, Exp, Log, Sqr)
    // WHEN evaluating domain boundaries: Log(0), Log(-5), Sqr(-1), Exp(800)
    // THEN Log(<= 0) and Sqr(< 0) must trap and raise Runtime Error 5 (Invalid procedure call)
    // AND Exp(800) must trap and raise Runtime Error 6 (Overflow) instead of emitting NaN or +inf
    // AND Sqr(0.0) must correctly return 0.0 without IEEE floating-point sign ambiguity.
    unsafe {
        assert_eq!(rtcLog(1.0), 0.0);
        assert_eq!(rtcExp(0.0), 1.0);
        assert_eq!(rtcSqr(16.0), 4.0);
        assert_eq!(rtcSqr(0.0), 0.0);

        let pi = rtcAtn(1.0) * 4.0;
        assert!((rtcSin(pi / 2.0) - 1.0).abs() < 1e-6);
        assert!((rtcCos(0.0) - 1.0).abs() < 1e-6);
        assert!(rtcTan(0.0).abs() < 1e-6);

        // Domain boundaries error traps
        clear_vb_error();
        let _ = rtcLog(0.0);
        assert_eq!(get_vb_error_number(), 5, "Log(0) must raise Error 5");

        clear_vb_error();
        let _ = rtcLog(-5.0);
        assert_eq!(get_vb_error_number(), 5, "Log(-5) must raise Error 5");

        clear_vb_error();
        let _ = rtcSqr(-1.0);
        assert_eq!(get_vb_error_number(), 5, "Sqr(-1) must raise Error 5");

        clear_vb_error();
        let _ = rtcExp(800.0);
        assert_eq!(get_vb_error_number(), 6, "Exp(800) must raise Error 6 (Overflow)");
        clear_vb_error();
    }
}

#[test]
fn test_large_variant_decimal_arithmetic_without_float_conversion() {
    // GIVEN 96-bit Variant Decimal numbers (VT_DECIMAL)
    // WHEN performing addition or subtraction near Decimal Max (79,228,162,514,264,337,593,543,950,335)
    // THEN arithmetic must maintain bit-level 96-bit integer fidelity without converting to Double
    // AND exceedances past Decimal Max must raise Runtime Error 6 (Overflow) instead of wrapping silently.
    unsafe {
        let d1 = Variant::from_decimal(Variant::DECIMAL_MAX - 1, 0, false);
        let d2 = Variant::from_decimal(1, 0, false);
        let mut d_sum = Variant::empty();
        __vbaVarAdd(&mut d_sum, &d2, &d1);
        assert_eq!(
            d_sum.to_string_representation(),
            "79228162514264337593543950335",
            "Decimal addition must achieve exact 96-bit max"
        );

        // Overflow past Decimal Max
        clear_vb_error();
        let mut d_overflow = Variant::empty();
        __vbaVarAdd(&mut d_overflow, &d2, &d_sum);
        assert_eq!(get_vb_error_number(), 6, "Exceeding Decimal Max must raise Error 6 (Overflow)");

        // Decimal subtraction
        let mut d_sub = Variant::empty();
        __vbaVarSub(&mut d_sub, &d2, &d_sum);
        assert_eq!(
            d_sub.to_string_representation(),
            "79228162514264337593543950334",
            "Decimal subtraction must preserve bit fidelity"
        );
        clear_vb_error();
    }
}
