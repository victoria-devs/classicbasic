//! Retroactive Regression Tests across all compiled VB6 PE32 Executables
//!
//! Validates that every previous iteration's compiled PE32 binary (both Native code
//! and P-Code) in `artifacts/` executes cleanly without regressions using the `vb6-run` runner.

use std::path::Path;
use std::process::Command;

fn run_binary_test(filename: &str) -> String {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("artifacts");
    let path = if base.join("bin").join(filename).exists() {
        base.join("bin").join(filename)
    } else {
        base.join(filename)
    };
    assert!(path.exists(), "Binary not found: {}", path.display());

    let runner = env!("CARGO_BIN_EXE_vb6-run");
    let output = Command::new(runner)
        .arg(&path)
        .output()
        .unwrap_or_else(|e| panic!("Failed to execute vb6-run with {}: {}", filename, e));

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "Binary {} failed with exit status {:?}\n--- STDOUT ---\n{}\n--- STDERR ---\n{}",
            filename, output.status, stdout, stderr
        );
    }
    stdout
}

#[test]
fn test_retro_app01_helloworld_pcode() {
    run_binary_test("App01-helloworld-pcode.exe");
}

#[test]
fn test_retro_app03_bstr_native() {
    run_binary_test("App03-bstr-native.exe");
}

#[test]
fn test_retro_app03_bstr_pcode() {
    run_binary_test("App03-bstr-pcode.exe");
}

#[test]
fn test_retro_app04_variant_native() {
    run_binary_test("App04-variant-native.exe");
}

#[test]
fn test_retro_app04_variant_pcode() {
    run_binary_test("App04-variant-pcode.exe");
}

#[test]
fn test_retro_app05_fileio_native() {
    run_binary_test("App05-fileio-native.exe");
}

#[test]
fn test_retro_app05_fileio_pcode() {
    run_binary_test("App05-fileio-pcode.exe");
}

#[test]
fn test_retro_app06_errorhandling_native() {
    run_binary_test("App06-errorhandling-native.exe");
}

#[test]
fn test_retro_app06_errorhandling_pcode() {
    run_binary_test("App06-errorhandling-pcode.exe");
}

#[test]
fn test_retro_app07_env_datetime_fs_native() {
    run_binary_test("App07-env-datetime-fs-native.exe");
}

#[test]
fn test_retro_app07_env_datetime_fs_pcode() {
    run_binary_test("App07-env-datetime-fs-pcode.exe");
}

#[test]
fn test_retro_app08_arrays_native() {
    run_binary_test("App08-arrays-native.exe");
}

#[test]
fn test_retro_app08_arrays_pcode() {
    run_binary_test("App08-arrays-pcode.exe");
}

#[test]
fn test_retro_app09_classic_fileio_native() {
    run_binary_test("App09-classic-fileio-native.exe");
}

#[test]
fn test_retro_app09_classic_fileio_pcode() {
    run_binary_test("App09-classic-fileio-pcode.exe");
}

#[test]
fn test_retro_app10_core_primitives_native() {
    run_binary_test("App10-core-primitives-native.exe");
}

#[test]
fn test_retro_app10_core_primitives_pcode() {
    run_binary_test("App10-core-primitives-pcode.exe");
}

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

#[test]
fn test_retro_app21_udt_fixed_strings_native() {
    let stdout = run_binary_test("App21-udt-fixed-strings-native.exe");
    assert!(stdout.contains("TEST 1: FIXED_LENGTH_STRINGS=PASS"));
    assert!(stdout.contains("TEST 2: LSET_RSET_ALIGNMENT=PASS"));
    assert!(stdout.contains("TEST 3: UDT_ASSIGNMENT_DEEP_COPY=PASS"));
    assert!(stdout.contains("TEST 4: LSET_UDT_BYTE_PUNNING=PASS"));
    assert!(stdout.contains("TEST 5: UDT_ARRAYS_AND_COPY=PASS"));
    assert!(stdout.contains("TEST 6: FIXED_STRING_AND_UDT_FILE_IO=PASS"));
    assert!(stdout.contains("TEST 7: UDT_ANSI_UNICODE_MARSHALING=PASS"));
}

#[test]
fn test_retro_app21_udt_fixed_strings_pcode() {
    let stdout = run_binary_test("App21-udt-fixed-strings-pcode.exe");
    assert!(stdout.contains("TEST 1: FIXED_LENGTH_STRINGS=PASS"));
    assert!(stdout.contains("TEST 2: LSET_RSET_ALIGNMENT=PASS"));
    assert!(stdout.contains("TEST 3: UDT_ASSIGNMENT_DEEP_COPY=PASS"));
    assert!(stdout.contains("TEST 4: LSET_UDT_BYTE_PUNNING=PASS"));
    assert!(stdout.contains("TEST 5: UDT_ARRAYS_AND_COPY=PASS"));
    assert!(stdout.contains("TEST 6: FIXED_STRING_AND_UDT_FILE_IO=PASS"));
    assert!(stdout.contains("TEST 7: UDT_ANSI_UNICODE_MARSHALING=PASS"));
}

#[test]
fn test_retro_app22_extended_rtc_native() {
    let stdout = run_binary_test("App22-extended-rtc-native.exe");
    assert!(stdout.contains("TEST 1: FINANCIAL_FUNCTIONS=PASS"));
    assert!(stdout.contains("TEST 2: CALENDAR_AND_DATE_MATH=PASS"));
    assert!(stdout.contains("TEST 3: LOCALE_FORMATTERS=PASS"));
    assert!(stdout.contains("TEST 4: UTILITY_SWITCH_AND_PARTITION=PASS"));
    assert!(stdout.contains("TEST 5: REGISTRY_SETTINGS=PASS"));
    assert!(stdout.contains("=== App22 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app22_extended_rtc_pcode() {
    let stdout = run_binary_test("App22-extended-rtc-pcode.exe");
    assert!(stdout.contains("TEST 1: FINANCIAL_FUNCTIONS=PASS"));
    assert!(stdout.contains("TEST 2: CALENDAR_AND_DATE_MATH=PASS"));
    assert!(stdout.contains("TEST 3: LOCALE_FORMATTERS=PASS"));
    assert!(stdout.contains("TEST 4: UTILITY_SWITCH_AND_PARTITION=PASS"));
    assert!(stdout.contains("TEST 5: REGISTRY_SETTINGS=PASS"));
    assert!(stdout.contains("=== App22 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app23_rtc_complete_native() {
    let stdout = run_binary_test("App23-rtc-complete-native.exe");
    assert!(stdout.contains("=== ClassicBasic App23: Complete Standard Library Closure (rtc*) ==="));
    assert!(stdout.contains("Test 1: Variant String Overloads passed."));
    assert!(stdout.contains("Test 2: Byte and Char String Functions passed."));
    assert!(stdout.contains("Test 3: Errors, Inspection and QBColor passed."));
    assert!(stdout.contains("Test 4: Environment and File System helpers passed."));
    assert!(stdout.contains("Test 5: Date and Time Variant helpers passed."));
    assert!(stdout.contains("=== App23 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app23_rtc_complete_pcode() {
    let stdout = run_binary_test("App23-rtc-complete-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App23: Complete Standard Library Closure (rtc*) ==="));
    assert!(stdout.contains("Test 1: Variant String Overloads passed."));
    assert!(stdout.contains("Test 2: Byte and Char String Functions passed."));
    assert!(stdout.contains("Test 3: Errors, Inspection and QBColor passed."));
    assert!(stdout.contains("Test 4: Environment and File System helpers passed."));
    assert!(stdout.contains("Test 5: Date and Time Variant helpers passed."));
    assert!(stdout.contains("=== App23 Test Suite Finished Successfully ==="));
}




#[test]
fn test_retro_app24_numeric_coercions_native() {
    let stdout = run_binary_test("App24-numeric-coercions-native.exe");
    assert!(stdout.contains("=== ClassicBasic App24: Numeric Intrinsics & Primitive Coercions ==="));
    assert!(stdout.contains("Test 1: Signed and Absolute Value passed."));
    assert!(stdout.contains("Test 2: Currency Math and Conversions passed."));
    assert!(stdout.contains("Test 3: Floating Point and Date Conversions passed."));
    assert!(stdout.contains("Test 4: Error Variant and Type Coercions passed."));
    assert!(stdout.contains("=== App24 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app24_numeric_coercions_pcode() {
    let stdout = run_binary_test("App24-numeric-coercions-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App24: Numeric Intrinsics & Primitive Coercions ==="));
    assert!(stdout.contains("Test 1: Signed and Absolute Value passed."));
    assert!(stdout.contains("Test 2: Currency Math and Conversions passed."));
    assert!(stdout.contains("Test 3: Floating Point and Date Conversions passed."));
    assert!(stdout.contains("Test 4: Error Variant and Type Coercions passed."));
    assert!(stdout.contains("=== App24 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app25_variant_bitwise_relational_native() {
    let stdout = run_binary_test("App25-variant-bitwise-relational-native.exe");
    assert!(stdout.contains("=== ClassicBasic App25: Variant Bitwise, Relational Testing & Vector Assignments ==="));
    assert!(stdout.contains("Test 1: Variant Bitwise Operations passed."));
    assert!(stdout.contains("Test 2: Variant Relational Testing passed."));
    assert!(stdout.contains("Test 3: Variant Assignment and Vector Ops passed."));
    assert!(stdout.contains("Test 4: Variant Text Comparisons (Option Compare Text) passed."));
    assert!(stdout.contains("=== App25 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app25_variant_bitwise_relational_pcode() {
    let stdout = run_binary_test("App25-variant-bitwise-relational-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App25: Variant Bitwise, Relational Testing & Vector Assignments ==="));
    assert!(stdout.contains("Test 1: Variant Bitwise Operations passed."));
    assert!(stdout.contains("Test 2: Variant Relational Testing passed."));
    assert!(stdout.contains("Test 3: Variant Assignment and Vector Ops passed."));
    assert!(stdout.contains("Test 4: Variant Text Comparisons (Option Compare Text) passed."));
    assert!(stdout.contains("=== App25 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app26_core_language_closure_native() {
    let stdout = run_binary_test("App26-core-language-closure-native.exe");
    assert!(stdout.contains("=== ClassicBasic App26: Dynamic Arrays, VarArgs & Core Language 100% Closure ==="));
    assert!(stdout.contains("Test 1: ParamArray and VarArgs passed."));
    assert!(stdout.contains("Test 2: Dynamic Variant Arrays passed."));
    assert!(stdout.contains("Test 3: Byte Strings and Advanced String Ops passed."));
    assert!(stdout.contains("Test 4: Variant Like Pattern Matching passed."));
    assert!(stdout.contains("Test 5: Type Checking and Control Flow passed."));
    assert!(stdout.contains("=== App26 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app26_core_language_closure_pcode() {
    let stdout = run_binary_test("App26-core-language-closure-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App26: Dynamic Arrays, VarArgs & Core Language 100% Closure ==="));
    assert!(stdout.contains("Test 1: ParamArray and VarArgs passed."));
    assert!(stdout.contains("Test 2: Dynamic Variant Arrays passed."));
    assert!(stdout.contains("Test 3: Byte Strings and Advanced String Ops passed."));
    assert!(stdout.contains("Test 4: Variant Like Pattern Matching passed."));
    assert!(stdout.contains("Test 5: Type Checking and Control Flow passed."));
    assert!(stdout.contains("=== App26 Test Suite Finished Successfully ==="));
}
