// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Regression Suite v0.2.9 (Phase 2.9): Complex Math, Banker's Rounding, FPU x87 Workarounds & Overflow Tests
//!
//! Validates:
//! - IEEE 754 Banker's Rounding (Round-to-Nearest-Even) across CInt, CLng, Round(), and Currency.
//! - CRT/C2 FPU workaround routines (_adj_fdiv_*, _CI*) and virtual x87 condition codes.
//! - Floating-point integer division `\` and `Mod` truncation and division by zero trapping (Error 11).
//! - 128-bit Currency intermediate precision avoiding premature 64-bit overflow.
//! - Transcendental functions domain limits and error traps (Error 5, Error 6).
//! - Large 96-bit Variant Decimal arithmetic with overflow detection (Error 6).

use msvbvm60::err::{clear_vb_error, get_vb_error_number};
use msvbvm60::math::*;
use msvbvm60::rtc::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;

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
