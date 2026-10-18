// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Retroactive Execution: App21 through App31 (ABI32, P-Code, x86/FPU & SEH)

use super::common::run_binary_test;

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
