// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Retroactive Execution: App11 through App20 (COM, Shims & Quirks)

use super::common::run_binary_test;

#[test]
fn test_retro_app11_com_factory_native() {
    run_binary_test("App11-com-factory-native.exe");
}

#[test]
fn test_retro_app11_com_factory_pcode() {
    run_binary_test("App11-com-factory-pcode.exe");
}

#[test]
fn test_retro_app12_typelib_introspection_native() {
    run_binary_test("App12-typelib-introspection-native.exe");
}

#[test]
fn test_retro_app12_typelib_introspection_pcode() {
    run_binary_test("App12-typelib-introspection-pcode.exe");
}

#[test]
fn test_retro_app13_late_binding_dispatch_native() {
    run_binary_test("App13-late-binding-dispatch-native.exe");
}

#[test]
fn test_retro_app13_late_binding_dispatch_pcode() {
    run_binary_test("App13-late-binding-dispatch-pcode.exe");
}

#[test]
fn test_retro_app14_activex_shims_native() {
    run_binary_test("App14-activex-shims-native.exe");
}

#[test]
fn test_retro_app14_activex_shims_pcode() {
    run_binary_test("App14-activex-shims-pcode.exe");
}

#[test]
fn test_retro_app15_activex_consolidation_native() {
    run_binary_test("App15-activex-consolidation-native.exe");
}

#[test]
fn test_retro_app15_activex_consolidation_pcode() {
    run_binary_test("App15-activex-consolidation-pcode.exe");
}

#[test]
fn test_retro_app16_threading_sta_concurrency_native() {
    run_binary_test("App16-threading-sta-concurrency-native.exe");
}

#[test]
fn test_retro_app16_threading_sta_concurrency_pcode() {
    run_binary_test("App16-threading-sta-concurrency-pcode.exe");
}

#[test]
fn test_retro_app17_esoteric_quirks_native() {
    run_binary_test("App17-esoteric-quirks-native.exe");
}

#[test]
fn test_retro_app17_esoteric_quirks_pcode() {
    run_binary_test("App17-esoteric-quirks-pcode.exe");
}

#[test]
fn test_retro_app18_complex_strings_native() {
    run_binary_test("App18-complex-strings-native.exe");
}

#[test]
fn test_retro_app18_complex_strings_pcode() {
    run_binary_test("App18-complex-strings-pcode.exe");
}

#[test]
fn test_retro_app19_math_rounding_native() {
    run_binary_test("App19-math-rounding-native.exe");
}

#[test]
fn test_retro_app19_math_rounding_pcode() {
    run_binary_test("App19-math-rounding-pcode.exe");
}

#[test]
fn test_retro_app20_loops_iterators_native() {
    run_binary_test("App20-loops-iterators-native.exe");
}

#[test]
fn test_retro_app20_loops_iterators_pcode() {
    run_binary_test("App20-loops-iterators-pcode.exe");
}
