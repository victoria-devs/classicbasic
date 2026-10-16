//! Retroactive Regression Tests across all compiled VB6 PE32 Executables
//!
//! Validates that every previous iteration's compiled PE32 binary (both Native code
//! and P-Code) in `artifacts/` executes cleanly without regressions using the `vb6-run` runner.

use std::path::Path;
use std::process::Command;

fn run_binary_test(filename: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("artifacts")
        .join(filename);
    assert!(path.exists(), "Binary not found: {}", path.display());

    let runner = env!("CARGO_BIN_EXE_vb6-run");
    let output = Command::new(runner)
        .arg(&path)
        .output()
        .unwrap_or_else(|e| panic!("Failed to execute vb6-run with {}: {}", filename, e));

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "Binary {} failed with exit status {:?}\n--- STDOUT ---\n{}\n--- STDERR ---\n{}",
            filename, output.status, stdout, stderr
        );
    }
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
