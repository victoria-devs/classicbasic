// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Retroactive Execution: App01 through App10 (Core Primitives)

use super::common::run_binary_test;

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
