# ClassicBasic — Layered Test Suite Catalog

This directory contains the automated test suites for **ClassicBasic** (`msvbvm60`). Tests are organized into isolated layer files mirroring the runtime's compilation milestones and ABI progression.

Every test function corresponds directly to an export ordinal, runtime subsystem, or behavioral scenario observed in real VB6 binaries.

---

## Running Tests

```bash
# Run all 227 tests (20 layered test suites + retroactive regression suite)
cargo test

# Run a specific layer test suite
cargo test --test layer01_lifecycle
cargo test --test layer08_dynamic_arrays
cargo test --test layer09_classic_fileio
cargo test --test layer11_core_primitives
cargo test --test layer14_dispatch_engine
cargo test --test layer15_activex_shims
cargo test --test layer16_consolidation_typeinfo
cargo test --test layer16_threading_apartments
cargo test --test layer17_esoteric_zombie_aliasing
cargo test --test layer18_complex_strings
cargo test --test layer19_complex_math

# Run retroactive binary regression suite across all compiled PE32 executables
cargo test --test retroactive_binaries
```

---

## Exhaustive Layer Test Catalog

| Layer Test File | Scope & Verifications | Tested Functions / Scenarios |
| :--- | :--- | :--- |
| [`layer01_lifecycle.rs`](layer01_lifecycle.rs) | **App01 Application Lifecycle** | `VarPtr`, `GetMem4`, `PutMem4`, `VBHeader`, `DllMain`, `__vbaEnd` clean channel flushing. |
| [`layer02_strings.rs`](layer02_strings.rs) | **App02/03 Strings & BSTR** | `SysAllocString`, `SysFreeString`, `__vbaStrCat`, `__vbaLenBstr`, `Mid$`, `Left$`, `Right$`, `InStr`, `UCase$`, `StrReverse`, `Trim$`, `Space$`, `__vbaStrCopy`, `__vbaStrMove`, `__vbaMidStmtBstr`. |
| [`layer03_math_types.rs`](layer03_math_types.rs) | **App03 Math & Numeric Types** | `Currency` fixed-point, `RGB`, `__vbaFPInt` (floor), `__vbaFpR4`, `__vbaStrI2`, `__vbaStrI4`, `__vbaStrR4`. |
| [`layer04_variants.rs`](layer04_variants.rs) | **App04 Variant Engine** | Tagged unions, arithmetic coercion (`+`, `-`, `*`, `/`, `\`, `Mod`), string concatenation (`&`), `Not`, `And`, relational comparisons, `VarType`, `__vbaBoolVarNull`. |
| [`layer05_win32_shims.rs`](layer05_win32_shims.rs) | **App05 Win32 KERNEL32 Shims** | `CreateFileA`, `WriteFile`, `ReadFile`, `GetFileSize`, `CloseHandle`, `GetLastError`, `GetStdHandle`. |
| [`layer06_err_handling.rs`](layer06_err_handling.rs) | **App06 Structured Error Handling** | `__vbaOnError`, `ErrObject` COM VTable (`QueryInterface`, `AddRef`, `Release`, `get_Number`, `put_Number`, `Raise`, `Clear`), `__vbaObjSet`, `__vbaFreeObj`, `__vbaResume`. |
| [`layer07_env_datetime_fs.rs`](layer07_env_datetime_fs.rs) | **App07 Environment & Filesystem** | `Environ$`, `CurDir$`, `Date$`, `Time$`, `Timer`, `Now` (`rtcGetPresentDate`, `__vbaDateVar`), `MkDir`, `RmDir`. |
| [`layer08_dynamic_arrays.rs`](layer08_dynamic_arrays.rs) | **App08 Dynamic Arrays (Iteration 1)** | `SafeArray`, `ReDim`, `ReDim Preserve`, `LBound`, `UBound`, `Split`, `Join`, `Filter`, `Array()`, `__vbaVarIndexLoad`, `__vbaErase`, `__vbaUI1ArrayStr`, `__vbaStrUI1Array` (`Byte()` coercion). |
| [`layer09_classic_fileio.rs`](layer09_classic_fileio.rs) | **App09 Classic File I/O (Iteration 2)** | Virtual file channels (`#1..#511`), `FreeFile`, `Open`, `Print #`, `Line Input #`, `Put #`, `Get #`, `Seek`, `LOF`, `EOF`, `FileCopy`, `Name..As`, `Dir$`, `Reset`, `Kill`. |
| [`layer10_pe_loader.rs`](layer10_pe_loader.rs) | **PE Loader Engine** | DOS header validation, synthetic VB6 binary loading, section RVA mapping, `VB5!` header detection. |
| [`layer11_core_primitives.rs`](layer11_core_primitives.rs) | **App10 Core Primitives (Iteration 3)** | `TypeName`, `IsEmpty`, `IsNull`, `IsArray`, `IsNumeric`, `IIf`, `Choose`, `Command$`, `Asc`, `Chr$`, `Hex$`, `Oct$`, `Val`, `Str$`, `Format$`, `LCase$`, `Replace`, `InStrRev`, `Sqr`, `Round`, `Int`, `Fix`, `Abs`, `Sgn`, conversions (`CBool`, `CInt`, `CLng`, `CCur`, `CSng`, `CDbl`), `End`. |
| [`layer12_com_instantiation.rs`](layer12_com_instantiation.rs) | **App11 COM Factory & Instantiation (Phase 2.1)** | `registry.json`, `DllGetClassObject`, `IClassFactory`, `rtcCreateObject2`, `rtcCreateObject`, error 429, `rtcIsObject`, `__vbaObjIs`, `__vbaObjVar`, `__vbaObjSetAddref`, `TypeName`. |
| [`layer13_typelib_parser.rs`](layer13_typelib_parser.rs) | **App12 TypeLib Parser & Introspection (Phase 2.2)** | MSFT binary parser (`MSFT\x02\x00\x01\x00`), synthetic `ITypeLib`/`ITypeInfo` generation, `DispidCache`, `__vbaNew` (341), `__vbaCastObj` (178), `__vbaStrVarMove` (437), `IDictionary` dual-interface vtable. |
| [`layer14_dispatch_engine.rs`](layer14_dispatch_engine.rs) | **App13 Late-Binding Dispatch Engine (Phase 2.3)** | `IDispatch::GetIDsOfNames`, `IDispatch::Invoke`, `__vbaLateMemCall*`, `__vbaLateIdCall*`, `__vbaVarLateMem*`, `__vbaUnkVar`, `__vbaVarSetObjAddref`, error trapping (438, 448, 450). |
| [`layer15_activex_shims.rs`](layer15_activex_shims.rs) | **App14 ActiveX Shims & Instantiation (Phase 2.4)** | `Scripting.FileSystemObject` & `TextStream`, `WScript.Shell`, `WScript.Network`, `ADODB.Recordset` (`Fields`, `Field`, `AddNew`, `Update`), `rtcGetObject` (626). |
| [`layer16_consolidation_typeinfo.rs`](layer16_consolidation_typeinfo.rs) | **App15 ActiveX Consolidation (Phase 2.5A)** | COM `ITypeInfo` vtable on native shims, dynamic member invocation via `rtcCallByName` (715), `TypeName` reflection, error 438 trapping. |
| [`layer16_threading_apartments.rs`](layer16_threading_apartments.rs) | **App16 STA Concurrency & Threading (Phase 2.5B Suite 1)** | STA apartments (TLS isolation, RPC rejection), worker thread context lifecycle, SafeArray lock violations, ByRef aliasing, BSTR embedded nulls, Banker's rounding, FDIV workarounds, ROT & IPC dispatch proxy. |
| [`layer17_esoteric_zombie_aliasing.rs`](layer17_esoteric_zombie_aliasing.rs) | **App17 Esoteric Quirks & Zombie Objects (Phase 2.5B Suite 2)** | Zombie object invocation lockout (`E_UNEXPECTED`), SafeArray lock violations during iteration (Error 10), ByRef parameter memory aliasing, deep ByRef variant dereferencing, re-entrant SEH frame unwinding, circular reference cleanup. |
| [`layer18_complex_strings.rs`](layer18_complex_strings.rs) | **App18 Complex Strings & Codepages (Phase 2.5B Suite 3)** | BSTR embedded null handling, `vbNullString` zero pointer equivalence, bidirectional `StrConv` (Unicode/ANSI Windows-1252 & casing), `String$` character code mod 256 wrapping, `Like` operator wildcard pattern matching with ranges, negation, and escapes. |
| [`layer19_complex_math.rs`](layer19_complex_math.rs) | **App19 Complex Math & FDIV (Phase 2.5B Suite 4)** | IEEE 754 Banker's Rounding (Round-to-Nearest-Even), CRT/C2 FPU workaround routines (`_adj_fdiv_*`, `_CI*`), integer division `\` and `Mod` truncation and Error 11 trapping, Currency 128-bit precision math, transcendental functions with domain error traps (Error 5, Error 6), 96-bit Variant Decimal arithmetic. |
| [`layer24_loops_iterators.rs`](layer24_loops_iterators.rs) | **App20 Loops, Iterators & Flow Control (Phase 1.5)** | Typed `For..Next` (`__vbaI2ForNextCheck`, `__vbaI4ForNextCheck`, `__vbaR4ForNextCheck`, `__vbaR8ForNextCheck`), 64-bit `Currency` fixed-point loops (`__vbaCyForInit`, `__vbaCyForNext`), dynamic `Variant` loops (`__vbaVarForInit`, `__vbaVarForNext`), `SafeArray` and `Variant` `For Each` iteration and lock cleanup (`__vbaForEachAry`, `__vbaNextEachAry`, `__vbaExitEachAry`, `__vbaForEachVar`, `__vbaNextEachVar`, `__vbaExitEachVar`), local subroutines (`__vbaGosub`, `__vbaGosubReturn`, `__vbaGosubFree`), computed branch validation (`__vbaOnGoCheck`). |
| [`retroactive_binaries.rs`](retroactive_binaries.rs) | **Retroactive Binary Regression Suite** | Executes all 37 compiled PE32 binaries (`artifacts/bin/*.exe`, Native & P-Code, App01–App20) in process isolation via `vb6-run`. |

---

## Retroactive Binary Regression Suite Coverage

The retroactive test suite guarantees zero regressions across all compiled Visual Basic 6 executables built on the reference environment (`/mnt/winxp`):

```text
App01-helloworld-pcode.exe
App03-bstr-native.exe / App03-bstr-pcode.exe
App04-variant-native.exe / App04-variant-pcode.exe
App05-fileio-native.exe / App05-fileio-pcode.exe
App06-errorhandling-native.exe / App06-errorhandling-pcode.exe
App07-env-datetime-fs-native.exe / App07-env-datetime-fs-pcode.exe
App08-arrays-native.exe / App08-arrays-pcode.exe
App09-classic-fileio-native.exe / App09-classic-fileio-pcode.exe
App10-core-primitives-native.exe / App10-core-primitives-pcode.exe
App11-com-factory-native.exe / App11-com-factory-pcode.exe
App12-typelib-introspection-native.exe / App12-typelib-introspection-pcode.exe
App13-late-binding-dispatch-native.exe / App13-late-binding-dispatch-pcode.exe
App14-activex-shims-native.exe / App14-activex-shims-pcode.exe
App15-activex-consolidation-native.exe / App15-activex-consolidation-pcode.exe
App16-threading-sta-concurrency-native.exe / App16-threading-sta-concurrency-pcode.exe
App17-esoteric-quirks-native.exe / App17-esoteric-quirks-pcode.exe
App18-complex-strings-native.exe / App18-complex-strings-pcode.exe
App19-math-rounding-native.exe / App19-math-rounding-pcode.exe
App20-loops-iterators-native.exe / App20-loops-iterators-pcode.exe
```
