//! Retroactive Regression Tests across all compiled VB6 PE32 Executables
//!
//! Validates that every previous iteration's compiled PE32 binary (both Native code
//! and P-Code) in `artifacts/` executes cleanly without regressions using the `vb6-run` runner.

use std::path::Path;
use std::process::Command;

fn run_binary_test(filename: &str) -> String {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let base = if manifest_dir.join("artifacts").exists() {
        manifest_dir.join("artifacts")
    } else {
        manifest_dir.join("../../artifacts")
    };
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

#[test]
fn test_retro_app27_dcom_ipc_native() {
    let stdout = run_binary_test("App27-dcom-ipc-native.exe");
    assert!(stdout.contains("=== ClassicBasic App27: DCOM, IPC & Running Object Table ==="));
    assert!(stdout.contains("Test 1: Running Object Table Resolution passed."));
    assert!(stdout.contains("Test 2: Out-Of-Process IPC Simulation passed."));
    assert!(stdout.contains("Test 3: DCOM Error Trapping and Monikers passed."));
    assert!(stdout.contains("Test 4: Connection Points and Dispatch Probing passed."));
    assert!(stdout.contains("=== App27 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app27_dcom_ipc_pcode() {
    let stdout = run_binary_test("App27-dcom-ipc-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App27: DCOM, IPC & Running Object Table ==="));
    assert!(stdout.contains("Test 1: Running Object Table Resolution passed."));
    assert!(stdout.contains("Test 2: Out-Of-Process IPC Simulation passed."));
    assert!(stdout.contains("Test 3: DCOM Error Trapping and Monikers passed."));
    assert!(stdout.contains("Test 4: Connection Points and Dispatch Probing passed."));
    assert!(stdout.contains("=== App27 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app28_event_sinks_collections_native() {
    let stdout = run_binary_test("App28-event-sinks-collections-native.exe");
    assert!(stdout.contains("=== ClassicBasic App28: COM Event Sinks & Collection Iterators ==="));
    assert!(stdout.contains("Test 1: Event sink binding and raising passed."));
    assert!(stdout.contains("Test 2: Event sink unhooking passed."));
    assert!(stdout.contains("Test 3: Collection basics passed."));
    assert!(stdout.contains("Test 4: For Each object iteration passed."));
    assert!(stdout.contains("Test 5: For Each variant iteration passed."));
    assert!(stdout.contains("Test 6: Nested For Each iteration passed."));
    assert!(stdout.contains("=== App28 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app28_event_sinks_collections_pcode() {
    let stdout = run_binary_test("App28-event-sinks-collections-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App28: COM Event Sinks & Collection Iterators ==="));
    assert!(stdout.contains("Test 1: Event sink binding and raising passed."));
    assert!(stdout.contains("Test 2: Event sink unhooking passed."));
    assert!(stdout.contains("Test 3: Collection basics passed."));
    assert!(stdout.contains("Test 4: For Each object iteration passed."));
    assert!(stdout.contains("Test 5: For Each variant iteration passed."));
    assert!(stdout.contains("Test 6: Nested For Each iteration passed."));
    assert!(stdout.contains("=== App28 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app29_abi32_ordinals_native() {
    let stdout = run_binary_test("App29-abi32-ordinals-native.exe");
    assert!(stdout.contains("=== ClassicBasic App29: 32-Bit ABI, Synthetic IRecordInfo & Full Ordinals ==="));
    assert!(stdout.contains("Test 1: Nested UDT assignment and deep copy passed."));
    assert!(stdout.contains("Test 2: 32-Bit ABI field offsets and memory layout passed."));
    assert!(stdout.contains("Test 3: UDT procedure passing and struct return passed."));
    assert!(stdout.contains("Test 4: Struct alignment and LenB passed."));
    assert!(stdout.contains("Test 5: UDT dynamic array preserve passed."));
    assert!(stdout.contains("Test 6: Record with internal dynamic array passed."));
    assert!(stdout.contains("=== App29 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app29_abi32_ordinals_pcode() {
    let stdout = run_binary_test("App29-abi32-ordinals-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App29: 32-Bit ABI, Synthetic IRecordInfo & Full Ordinals ==="));
    assert!(stdout.contains("Test 1: Nested UDT assignment and deep copy passed."));
    assert!(stdout.contains("Test 2: 32-Bit ABI field offsets and memory layout passed."));
    assert!(stdout.contains("Test 3: UDT procedure passing and struct return passed."));
    assert!(stdout.contains("Test 4: Struct alignment and LenB passed."));
    assert!(stdout.contains("Test 5: UDT dynamic array preserve passed."));
    assert!(stdout.contains("Test 6: Record with internal dynamic array passed."));
    assert!(stdout.contains("=== App29 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app30_native_x86_fpu_seh_native() {
    let stdout = run_binary_test("App30-native-x86-fpu-seh-native.exe");
    assert!(stdout.contains("=== ClassicBasic App30: Native x86 Execution, x87 FPU & SEH ==="));
    assert!(stdout.contains("Test 1: Stack probe (__vbaChkstk) passed."));
    assert!(stdout.contains("Test 2: x87 FPU transcendental math passed."));
    assert!(stdout.contains("Test 3: SEH division by zero trapping and resume passed."));
    assert!(stdout.contains("Test 4: SEH overflow and Resume Next passed."));
    assert!(stdout.contains("Test 5: Recursive stack unwinding passed."));
    assert!(stdout.contains("Test 6: Float ByRef/ByVal conventions passed."));
    assert!(stdout.contains("=== App30 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app30_native_x86_fpu_seh_pcode() {
    let stdout = run_binary_test("App30-native-x86-fpu-seh-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App30: Native x86 Execution, x87 FPU & SEH ==="));
    assert!(stdout.contains("Test 1: Stack probe (__vbaChkstk) passed."));
    assert!(stdout.contains("Test 2: x87 FPU transcendental math passed."));
    assert!(stdout.contains("Test 3: SEH division by zero trapping and resume passed."));
    assert!(stdout.contains("Test 4: SEH overflow and Resume Next passed."));
    assert!(stdout.contains("Test 5: Recursive stack unwinding passed."));
    assert!(stdout.contains("Test 6: Float ByRef/ByVal conventions passed."));
    assert!(stdout.contains("=== App30 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app31_pcode_vm_native() {
    let stdout = run_binary_test("App31-pcode-vm-native.exe");
    assert!(stdout.contains("=== ClassicBasic App31: P-Code VM Interpreter ==="));
    assert!(stdout.contains("Test 1: P-Code arithmetic expressions passed."));
    assert!(stdout.contains("Test 2: P-Code Select Case jump tables passed."));
    assert!(stdout.contains("Test 3: P-Code control flow loops passed."));
    assert!(stdout.contains("Test 4: ProcCallEngine calling conventions passed."));
    assert!(stdout.contains("Test 5: MethCallEngine class dispatch passed."));
    assert!(stdout.contains("Test 6: P-Code error trapping passed."));
    assert!(stdout.contains("=== App31 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app31_pcode_vm_pcode() {
    let stdout = run_binary_test("App31-pcode-vm-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App31: P-Code VM Interpreter ==="));
    assert!(stdout.contains("Test 1: P-Code arithmetic expressions passed."));
    assert!(stdout.contains("Test 2: P-Code Select Case jump tables passed."));
    assert!(stdout.contains("Test 3: P-Code control flow loops passed."));
    assert!(stdout.contains("Test 4: ProcCallEngine calling conventions passed."));
    assert!(stdout.contains("Test 5: MethCallEngine class dispatch passed."));
    assert!(stdout.contains("Test 6: P-Code error trapping passed."));
    assert!(stdout.contains("=== App31 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app32_pcode_engine_native() {
    let stdout = run_binary_test("App32-pcode-engine-native.exe");
    assert!(stdout.contains("=== ClassicBasic App32: P-Code VM Engine (ProcCallEngine & MethCallEngine) ==="));
    assert!(stdout.contains("Test 1: P-Code evaluation stack and operator precedence passed."));
    assert!(stdout.contains("Test 2: Nested procedure frames and ByRef parameter binding passed."));
    assert!(stdout.contains("Test 3: P-Code loop branches and Select Case jump tables passed."));
    assert!(stdout.contains("Test 4: MethCallEngine class dispatch and object cleanup passed."));
    assert!(stdout.contains("Test 5: P-Code structured error handling and resume passed."));
    assert!(stdout.contains("Test 6: Runtime library intrinsic import dispatch passed."));
    assert!(stdout.contains("=== App32 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app32_pcode_engine_pcode() {
    let stdout = run_binary_test("App32-pcode-engine-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App32: P-Code VM Engine (ProcCallEngine & MethCallEngine) ==="));
    assert!(stdout.contains("Test 1: P-Code evaluation stack and operator precedence passed."));
    assert!(stdout.contains("Test 2: Nested procedure frames and ByRef parameter binding passed."));
    assert!(stdout.contains("Test 3: P-Code loop branches and Select Case jump tables passed."));
    assert!(stdout.contains("Test 4: MethCallEngine class dispatch and object cleanup passed."));
    assert!(stdout.contains("Test 5: P-Code structured error handling and resume passed."));
    assert!(stdout.contains("Test 6: Runtime library intrinsic import dispatch passed."));
    assert!(stdout.contains("=== App32 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app33_ide_introspection_native() {
    let stdout = run_binary_test("App33-ide-introspection-eb-tip-native.exe");
    assert!(stdout.contains("=== ClassicBasic App33: IDE Hosting, Project Introspection & Debugger Telemetry ==="));
    assert!(stdout.contains("Test 1: Project handle and execution stack query passed."));
    assert!(stdout.contains("Test 2: Predeclared class instance resolution passed."));
    assert!(stdout.contains("Test 3: COM connection counts and telemetry passed."));
    assert!(stdout.contains("Test 4: Runtime expression service and VBA object query passed."));
    assert!(stdout.contains("Test 5: Tip project options and instance management passed."));
    assert!(stdout.contains("Test 6: Library management and project reset passed."));
    assert!(stdout.contains("=== App33 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app33_ide_introspection_pcode() {
    let stdout = run_binary_test("App33-ide-introspection-eb-tip-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App33: IDE Hosting, Project Introspection & Debugger Telemetry ==="));
    assert!(stdout.contains("Test 1: Project handle and execution stack query passed."));
    assert!(stdout.contains("Test 2: Predeclared class instance resolution passed."));
    assert!(stdout.contains("Test 3: COM connection counts and telemetry passed."));
    assert!(stdout.contains("Test 4: Runtime expression service and VBA object query passed."));
    assert!(stdout.contains("Test 5: Tip project options and instance management passed."));
    assert!(stdout.contains("Test 6: Library management and project reset passed."));
    assert!(stdout.contains("=== App33 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app34_native_x86_engine_native() {
    let stdout = run_binary_test("App34-native-x86-engine-native.exe");
    assert!(stdout.contains("=== ClassicBasic App34: Native x86 CPU Execution Engine (Instruction Emulator & FS:[0] TIB) ==="));
    assert!(stdout.contains("Test 1: 32-bit ALU arithmetic and register operations passed."));
    assert!(stdout.contains("Test 2: stdcall stack frames and ByRef parameter passing passed."));
    assert!(stdout.contains("Test 3: Native x86 branching, loops and jump conditionals passed."));
    assert!(stdout.contains("Test 4: x87 FPU instruction pipeline and transcendental math passed."));
    assert!(stdout.contains("Test 5: Win32 FS:[0] TIB and SEH exception frame handling passed."));
    assert!(stdout.contains("Test 6: Native class method dispatch and object lifecycle passed."));
    assert!(stdout.contains("=== App34 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app34_native_x86_engine_pcode() {
    let stdout = run_binary_test("App34-native-x86-engine-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App34: Native x86 CPU Execution Engine (Instruction Emulator & FS:[0] TIB) ==="));
    assert!(stdout.contains("Test 1: 32-bit ALU arithmetic and register operations passed."));
    assert!(stdout.contains("Test 2: stdcall stack frames and ByRef parameter passing passed."));
    assert!(stdout.contains("Test 3: Native x86 branching, loops and jump conditionals passed."));
    assert!(stdout.contains("Test 4: x87 FPU instruction pipeline and transcendental math passed."));
    assert!(stdout.contains("Test 5: Win32 FS:[0] TIB and SEH exception frame handling passed."));
    assert!(stdout.contains("Test 6: Native class method dispatch and object lifecycle passed."));
    assert!(stdout.contains("=== App34 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app35_win32_user32_windowing_native() {
    let stdout = run_binary_test("App35-win32-user32-windowing-native.exe");
    assert!(stdout.contains("=== ClassicBasic App35: Win32 USER32 Virtual Windowing & Condvar Message Pump ==="));
    assert!(stdout.contains("Test 1: System metrics query passed."));
    assert!(stdout.contains("Test 2: Window class registration and creation passed."));
    assert!(stdout.contains("Test 3: Window hierarchy, geometry and text passed."));
    assert!(stdout.contains("Test 4: Window visibility and state control passed."));
    assert!(stdout.contains("Test 5: Synchronous message dispatch and DefWindowProc passed."));
    assert!(stdout.contains("Test 6: Asynchronous queue, pump and lifecycle cleanup passed."));
    assert!(stdout.contains("=== App35 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app35_win32_user32_windowing_pcode() {
    let stdout = run_binary_test("App35-win32-user32-windowing-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App35: Win32 USER32 Virtual Windowing & Condvar Message Pump ==="));
    assert!(stdout.contains("Test 1: System metrics query passed."));
    assert!(stdout.contains("Test 2: Window class registration and creation passed."));
    assert!(stdout.contains("Test 3: Window hierarchy, geometry and text passed."));
    assert!(stdout.contains("Test 4: Window visibility and state control passed."));
    assert!(stdout.contains("Test 5: Synchronous message dispatch and DefWindowProc passed."));
    assert!(stdout.contains("Test 6: Asynchronous queue, pump and lifecycle cleanup passed."));
    assert!(stdout.contains("=== App35 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app36_win32_gdi32_surface_native() {
    let stdout = run_binary_test("App36-win32-gdi32-surface-native.exe");
    assert!(stdout.contains("=== ClassicBasic App36: GDI32 32-Bit Surface, Vector Primitives & Dirty Rects ==="));
    assert!(stdout.contains("Test 1: Device context lifecycle and stock objects passed."));
    assert!(stdout.contains("Test 2: Solid pens, brushes and pixel operations passed."));
    assert!(stdout.contains("Test 3: Vector drawing primitives (LineTo, Rectangle, Ellipse) passed."));
    assert!(stdout.contains("Test 4: Region fills and framing (FillRect, FrameRect, RoundRect) passed."));
    assert!(stdout.contains("Test 5: Bitmaps, memory DCs and raster operations (BitBlt, PatBlt) passed."));
    assert!(stdout.contains("Test 6: Window invalidation and paint lifecycle (BeginPaint, EndPaint) passed."));
    assert!(stdout.contains("=== App36 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app36_win32_gdi32_surface_pcode() {
    let stdout = run_binary_test("App36-win32-gdi32-surface-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App36: GDI32 32-Bit Surface, Vector Primitives & Dirty Rects ==="));
    assert!(stdout.contains("Test 1: Device context lifecycle and stock objects passed."));
    assert!(stdout.contains("Test 2: Solid pens, brushes and pixel operations passed."));
    assert!(stdout.contains("Test 3: Vector drawing primitives (LineTo, Rectangle, Ellipse) passed."));
    assert!(stdout.contains("Test 4: Region fills and framing (FillRect, FrameRect, RoundRect) passed."));
    assert!(stdout.contains("Test 5: Bitmaps, memory DCs and raster operations (BitBlt, PatBlt) passed."));
    assert!(stdout.contains("Test 6: Window invalidation and paint lifecycle (BeginPaint, EndPaint) passed."));
    assert!(stdout.contains("=== App36 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app37_gdi_text_fonts_native() {
    let stdout = run_binary_test("App37-gdi-text-fonts-native.exe");
    assert!(stdout.contains("=== ClassicBasic App37: GDI32 Text Output, Font Management & Metrics ==="));
    assert!(stdout.contains("Test 1: Font creation, selection and lifecycle management passed."));
    assert!(stdout.contains("Test 2: Text metrics (GetTextMetrics) and string extent calculation passed."));
    assert!(stdout.contains("Test 3: Text color, background color and background mode control passed."));
    assert!(stdout.contains("Test 4: Basic text rasterization (TextOut) onto bitmap surface passed."));
    assert!(stdout.contains("Test 5: Advanced text output with background filling and clipping (ExtTextOut) passed."));
    assert!(stdout.contains("Test 6: Formatted text layout and rectangle calculation (DrawText) passed."));
    assert!(stdout.contains("=== App37 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app37_gdi_text_fonts_pcode() {
    let stdout = run_binary_test("App37-gdi-text-fonts-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App37: GDI32 Text Output, Font Management & Metrics ==="));
    assert!(stdout.contains("Test 1: Font creation, selection and lifecycle management passed."));
    assert!(stdout.contains("Test 2: Text metrics (GetTextMetrics) and string extent calculation passed."));
    assert!(stdout.contains("Test 3: Text color, background color and background mode control passed."));
    assert!(stdout.contains("Test 4: Basic text rasterization (TextOut) onto bitmap surface passed."));
    assert!(stdout.contains("Test 5: Advanced text output with background filling and clipping (ExtTextOut) passed."));
    assert!(stdout.contains("Test 6: Formatted text layout and rectangle calculation (DrawText) passed."));
    assert!(stdout.contains("=== App37 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app38_vb6_form_deserializer_native() {
    let stdout = run_binary_test("App38-vb6-form-deserializer-native.exe");
    assert!(stdout.contains("=== ClassicBasic App38: VB6 Form Deserializer & Twips-to-Pixels Engine ==="));
    assert!(stdout.contains("Test 1: Form instantiation, metadata and basic properties passed."));
    assert!(stdout.contains("Test 2: Child controls presence and initial properties verified."));
    assert!(stdout.contains("Test 3: Twips-to-pixels geometric coordinate conversion passed."));
    assert!(stdout.contains("Test 4: Win32 HWND resolution and window tree binding passed."));
    assert!(stdout.contains("Test 5: Dynamic property mutation and state synchronization passed."));
    assert!(stdout.contains("Test 6: Form unload lifecycle and resource teardown passed."));
    assert!(stdout.contains("=== App38 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app38_vb6_form_deserializer_pcode() {
    let stdout = run_binary_test("App38-vb6-form-deserializer-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App38: VB6 Form Deserializer & Twips-to-Pixels Engine ==="));
    assert!(stdout.contains("Test 1: Form instantiation, metadata and basic properties passed."));
    assert!(stdout.contains("Test 2: Child controls presence and initial properties verified."));
    assert!(stdout.contains("Test 3: Twips-to-pixels geometric coordinate conversion passed."));
    assert!(stdout.contains("Test 4: Win32 HWND resolution and window tree binding passed."));
    assert!(stdout.contains("Test 5: Dynamic property mutation and state synchronization passed."));
    assert!(stdout.contains("Test 6: Form unload lifecycle and resource teardown passed."));
    assert!(stdout.contains("=== App38 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app39_vb6_form_events_native() {
    let stdout = run_binary_test("App39-vb6-form-events-native.exe");
    assert!(stdout.contains("=== ClassicBasic App39: VB6 Form Event Sinks & Control Runtime ==="));
    assert!(stdout.contains("Test 1: Form_Load event invocation on Load Form1 passed."));
    assert!(stdout.contains("Test 2: Command1_Click event dispatch passed."));
    assert!(stdout.contains("Test 3: Text1_Change event propagation passed."));
    assert!(stdout.contains("Test 4: Check1_Click and Option1_Click event dispatch passed."));
    assert!(stdout.contains("Test 5: ListBox AddItem, ListCount and Clear methods passed."));
    assert!(stdout.contains("Test 6: Form_Unload event dispatch and event sequence verified."));
    assert!(stdout.contains("=== App39 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app39_vb6_form_events_pcode() {
    let stdout = run_binary_test("App39-vb6-form-events-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App39: VB6 Form Event Sinks & Control Runtime ==="));
    assert!(stdout.contains("Test 1: Form_Load event invocation on Load Form1 passed."));
    assert!(stdout.contains("Test 2: Command1_Click event dispatch passed."));
    assert!(stdout.contains("Test 3: Text1_Change event propagation passed."));
    assert!(stdout.contains("Test 4: Check1_Click and Option1_Click event dispatch passed."));
    assert!(stdout.contains("Test 5: ListBox AddItem, ListCount and Clear methods passed."));
    assert!(stdout.contains("Test 6: Form_Unload event dispatch and event sequence verified."));
    assert!(stdout.contains("=== App39 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app40_gui_websocket_stream_native() {
    let stdout = run_binary_test("App40-gui-websocket-stream-native.exe");
    assert!(stdout.contains("=== ClassicBasic App40: GUI WebSocket Stream & Wire Protocol Surfaces ==="));
    assert!(stdout.contains("Test 1: FRAME_BITMAP_RECT pixel surface and damage rectangle tracking passed."));
    assert!(stdout.contains("Test 2: FRAME_FORM form and control snapshot surface passed."));
    assert!(stdout.contains("Test 3: FRAME_CURSOR cursor shape and position surface passed."));
    assert!(stdout.contains("Test 4: FRAME_CLIPBOARD clipboard text round-trip passed."));
    assert!(stdout.contains("Test 5: FRAME_TEXT text run attributes and layout extents passed."));
    assert!(stdout.contains("Test 6: Input frame to WM_* message queue injection and ordering passed."));
    assert!(stdout.contains("=== App40 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app40_gui_websocket_stream_pcode() {
    let stdout = run_binary_test("App40-gui-websocket-stream-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App40: GUI WebSocket Stream & Wire Protocol Surfaces ==="));
    assert!(stdout.contains("Test 1: FRAME_BITMAP_RECT pixel surface and damage rectangle tracking passed."));
    assert!(stdout.contains("Test 2: FRAME_FORM form and control snapshot surface passed."));
    assert!(stdout.contains("Test 3: FRAME_CURSOR cursor shape and position surface passed."));
    assert!(stdout.contains("Test 4: FRAME_CLIPBOARD clipboard text round-trip passed."));
    assert!(stdout.contains("Test 5: FRAME_TEXT text run attributes and layout extents passed."));
    assert!(stdout.contains("Test 6: Input frame to WM_* message queue injection and ordering passed."));
    assert!(stdout.contains("=== App40 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app41_gui_wasm_client_native() {
    let stdout = run_binary_test("App41-gui-wasm-client-native.exe");
    assert!(stdout.contains("=== ClassicBasic App41: WebAssembly Canvas Client & Streaming Protocol ==="));
    assert!(stdout.contains("Test 1: Canvas viewport geometry and dynamic resize passed."));
    assert!(stdout.contains("Test 2: Bitmap surface memory and color channel blit passed."));
    assert!(stdout.contains("Test 3: Delegated text metrics and canvas draw commands passed."));
    assert!(stdout.contains("Test 4: Browser pointer input to Win32 message pipeline passed."));
    assert!(stdout.contains("Test 5: Browser keyboard events and char stream injection passed."));
    assert!(stdout.contains("Test 6: Form and intrinsic control canvas state synchronization passed."));
    assert!(stdout.contains("=== App41 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app41_gui_wasm_client_pcode() {
    let stdout = run_binary_test("App41-gui-wasm-client-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App41: WebAssembly Canvas Client & Streaming Protocol ==="));
    assert!(stdout.contains("Test 1: Canvas viewport geometry and dynamic resize passed."));
    assert!(stdout.contains("Test 2: Bitmap surface memory and color channel blit passed."));
    assert!(stdout.contains("Test 3: Delegated text metrics and canvas draw commands passed."));
    assert!(stdout.contains("Test 4: Browser pointer input to Win32 message pipeline passed."));
    assert!(stdout.contains("Test 5: Browser keyboard events and char stream injection passed."));
    assert!(stdout.contains("Test 6: Form and intrinsic control canvas state synchronization passed."));
    assert!(stdout.contains("=== App41 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app42_gui_calculator_e2e_native() {
    let stdout = run_binary_test("App42-gui-calculator-e2e-native.exe");
    assert!(stdout.contains("=== ClassicBasic App42: Integrated GUI Web Runner & Form Lifecycle ==="));
    assert!(stdout.contains("Test 1: Form instantiation and Form_Load initialization passed."));
    assert!(stdout.contains("Test 2: Basic calculation lifecycle (addition and subtraction) passed."));
    assert!(stdout.contains("Test 3: Multiplication, division, floating point, and error handling passed."));
    assert!(stdout.contains("Test 4: Keyboard event routing and message pump dispatch passed."));
    assert!(stdout.contains("Test 5: GDI surface damage tracking and validation lifecycle passed."));
    assert!(stdout.contains("Test 6: Form teardown and unload lifecycle passed."));
    assert!(stdout.contains("=== App42 Test Suite Finished Successfully ==="));
}

#[test]
fn test_retro_app42_gui_calculator_e2e_pcode() {
    let stdout = run_binary_test("App42-gui-calculator-e2e-pcode.exe");
    assert!(stdout.contains("=== ClassicBasic App42: Integrated GUI Web Runner & Form Lifecycle ==="));
    assert!(stdout.contains("Test 1: Form instantiation and Form_Load initialization passed."));
    assert!(stdout.contains("Test 2: Basic calculation lifecycle (addition and subtraction) passed."));
    assert!(stdout.contains("Test 3: Multiplication, division, floating point, and error handling passed."));
    assert!(stdout.contains("Test 4: Keyboard event routing and message pump dispatch passed."));
    assert!(stdout.contains("Test 5: GDI surface damage tracking and validation lifecycle passed."));
    assert!(stdout.contains("Test 6: Form teardown and unload lifecycle passed."));
    assert!(stdout.contains("=== App42 Test Suite Finished Successfully ==="));
}







