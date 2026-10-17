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



