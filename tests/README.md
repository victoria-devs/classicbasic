# ClassicBasic — Versioned Regression Test Suite Catalog

This directory contains the automated regression test suites for **ClassicBasic** (`msvbvm60`). Test files are named with their target runtime SemVer version (`v<major>_<minor>_<patch>_<subsystem>.rs`) in strict sequential order, allowing immediate identification of which version milestone and runtime capability broke whenever a regression occurs.

Every test function corresponds directly to an export ordinal, runtime subsystem, or behavioral scenario observed in real VB6 binaries.

---

## Running Tests

```bash
# Run all 274 tests (24 versioned regression suites + retroactive PE32 binary suite)
cargo test

# Run a specific version's regression suite
cargo test --test v0_1_1_lifecycle
cargo test --test v0_1_2_dynamic_arrays
cargo test --test v0_1_3_classic_fileio
cargo test --test v0_1_4_core_primitives
cargo test --test v0_1_5_loops_iterators
cargo test --test v0_1_6_udt_fixed_strings
cargo test --test v0_1_7_extended_rtc
cargo test --test v0_1_8_rtc_complete
cargo test --test v0_2_1_com_instantiation
cargo test --test v0_2_2_typelib_parser
cargo test --test v0_2_3_dispatch_engine
cargo test --test v0_2_4_activex_shims
cargo test --test v0_2_5_consolidation_typeinfo
cargo test --test v0_2_6_threading_apartments
cargo test --test v0_2_7_esoteric_zombie_aliasing
cargo test --test v0_2_8_complex_strings
cargo test --test v0_2_9_complex_math

# Run retroactive binary regression suite across all compiled PE32 executables
cargo test --test retroactive_binaries
```

---

## Exhaustive Regression Test Catalog

| Version | Phase | Regression Suite File | Scope & Fixture | Tested Functions / Scenarios |
| :---: | :---: | :--- | :--- | :--- |
| `v0.1.1` | **Phase 1.1** | [`v0_1_1_lifecycle.rs`](v0_1_1_lifecycle.rs) | **App01 Application Lifecycle** | `VarPtr`, `GetMem4`, `PutMem4`, `VBHeader`, `DllMain`, `__vbaEnd` clean channel flushing. |
| `v0.1.1` | **Phase 1.1** | [`v0_1_1_strings_bstr.rs`](v0_1_1_strings_bstr.rs) | **App02/03 Strings & BSTR** | `SysAllocString`, `SysFreeString`, `__vbaStrCat`, `__vbaLenBstr`, `Mid$`, `Left$`, `Right$`, `InStr`, `UCase$`, `StrReverse`, `Trim$`, `Space$`, `__vbaStrCopy`, `__vbaStrMove`, `__vbaMidStmtBstr`. |
| `v0.1.1` | **Phase 1.1** | [`v0_1_1_math_types.rs`](v0_1_1_math_types.rs) | **App03 Math & Numeric Types** | `Currency` fixed-point, `RGB`, `__vbaFPInt` (floor), `__vbaFpR4`, `__vbaStrI2`, `__vbaStrI4`, `__vbaStrR4`. |
| `v0.1.1` | **Phase 1.1** | [`v0_1_1_variant_engine.rs`](v0_1_1_variant_engine.rs) | **App04 Variant Engine** | Tagged unions, arithmetic coercion (`+`, `-`, `*`, `/`, `\`, `Mod`), string concatenation (`&`), `Not`, `And`, relational comparisons, `VarType`, `__vbaBoolVarNull`. |
| `v0.1.1` | **Phase 1.1** | [`v0_1_1_win32_shims.rs`](v0_1_1_win32_shims.rs) | **App05 Win32 KERNEL32 Shims** | `CreateFileA`, `WriteFile`, `ReadFile`, `GetFileSize`, `CloseHandle`, `GetLastError`, `GetStdHandle`. |
| `v0.1.1` | **Phase 1.1** | [`v0_1_1_err_handling.rs`](v0_1_1_err_handling.rs) | **App06 Structured Error Handling** | `__vbaOnError`, `ErrObject` COM VTable (`QueryInterface`, `AddRef`, `Release`, `get_Number`, `put_Number`, `Raise`, `Clear`), `__vbaObjSet`, `__vbaFreeObj`, `__vbaResume`. |
| `v0.1.1` | **Phase 1.1** | [`v0_1_1_env_datetime_fs.rs`](v0_1_1_env_datetime_fs.rs) | **App07 Environment & Filesystem** | `Environ$`, `CurDir$`, `Date$`, `Time$`, `Timer`, `Now` (`rtcGetPresentDate`, `__vbaDateVar`), `MkDir`, `RmDir`. |
| `v0.1.1` | **Phase 1.1** | [`v0_1_1_pe_loader.rs`](v0_1_1_pe_loader.rs) | **PE32 Loader Engine** | DOS header validation, synthetic VB6 binary loading, section RVA mapping, `VB5!` header detection. |
| `v0.1.2` | **Phase 1.2** | [`v0_1_2_dynamic_arrays.rs`](v0_1_2_dynamic_arrays.rs) | **App08 Dynamic Arrays** | `SafeArray`, `ReDim`, `ReDim Preserve`, `LBound`, `UBound`, `Split`, `Join`, `Filter`, `Array()`, `__vbaVarIndexLoad`, `__vbaErase`, `__vbaUI1ArrayStr`, `__vbaStrUI1Array` (`Byte()` coercion). |
| `v0.1.3` | **Phase 1.3** | [`v0_1_3_classic_fileio.rs`](v0_1_3_classic_fileio.rs) | **App09 Classic File I/O** | Virtual file channels (`#1..#511`), `FreeFile`, `Open`, `Print #`, `Line Input #`, `Put #`, `Get #`, `Seek`, `LOF`, `EOF`, `FileCopy`, `Name..As`, `Dir$`, `Reset`, `Kill`. |
| `v0.1.4` | **Phase 1.4** | [`v0_1_4_core_primitives.rs`](v0_1_4_core_primitives.rs) | **App10 Core Primitives** | `TypeName`, `IsEmpty`, `IsNull`, `IsArray`, `IsNumeric`, `IIf`, `Choose`, `Command$`, `Asc`, `Chr$`, `Hex$`, `Oct$`, `Val`, `Str$`, `Format$`, `LCase$`, `Replace`, `InStrRev`, `Sqr`, `Round`, `Int`, `Fix`, `Abs`, `Sgn`, conversions (`CBool`, `CInt`, `CLng`, `CCur`, `CSng`, `CDbl`), `End`. |
| `v0.1.5` | **Phase 1.5** | [`v0_1_5_loops_iterators.rs`](v0_1_5_loops_iterators.rs) | **App20 Loops, Iterators & Flow Control** | Typed `For..Next` (`__vbaI2/I4/R4/R8ForNextCheck`), 64-bit `Currency` fixed-point loops (`__vbaCyForInit`, `__vbaCyForNext`), dynamic `Variant` loops (`__vbaVarForInit`, `__vbaVarForNext`), `SafeArray` and `Variant` `For Each` iteration and lock cleanup (`__vbaForEachAry`, `__vbaNextEachAry`, `__vbaExitEachAry`, `__vbaForEachVar`, `__vbaNextEachVar`, `__vbaExitEachVar`), local subroutines (`__vbaGosub`, `__vbaGosubReturn`, `__vbaGosubFree`), computed branch validation (`__vbaOnGoCheck`). |
| `v0.1.6` | **Phase 1.6** | [`v0_1_6_udt_fixed_strings.rs`](v0_1_6_udt_fixed_strings.rs) | **App21 UDTs & Fixed Strings** | Record copy, assignment, destruction (`CopyRecord`, `__vbaRecAssign`, `__vbaRecDestruct`, `__vbaRecDestructAnsi`, `__vbaRecAnsiToUni`, `__vbaRecUniToAnsi`, `__vbaAryRecCopy`, `__vbaAryRecMove`), `VT_RECORD` variant conversions (`__vbaUdtVar`, `__vbaCVarAryUdt`, `__vbaUI1I2`), fixed-length string manipulation (`__vbaFixstrConstruct`, `__vbaStrFixstr`, `__vbaLsetFixstr`, `__vbaLsetFixstrFree`, `__vbaRsetFixstr`, `__vbaRsetFixstrFree`, `__vbaGetFxStr3/4`, `__vbaPutFxStr3/4`, `__vbaGetOwner3/4`, `__vbaPutOwner3/4`). |
| `v0.1.7` | **Phase 1.7** | [`v0_1_7_extended_rtc.rs`](v0_1_7_extended_rtc.rs) | **App22 Extended Standard Library** | Financial suite (`SLN`, `SYD`, `DDB`, `PMT`, `IPMT`, `PPMT`, `PV`, `FV`, `NPer`, `Rate`, `NPV`, `IRR`, `MIRR`), calendar & date mathematics (`DateSerial`, `TimeSerial`, `DateValue`, `TimeValue`, `Day`, `Month`, `Year`, `Hour`, `Minute`, `Second`, `Weekday`, `DateAdd`, `DateDiff`, `DatePart`, `WeekdayName`, `MonthName`, `IsDate`), locale formatters (`FormatNumber`, `FormatCurrency`, `FormatPercent`, `FormatDateTime`), flow utilities (`Switch`, `Partition`), and virtual registry management (`SaveSetting`, `GetSetting`, `GetAllSettings`, `DeleteSetting`). |
| `v0.1.8` | **Phase 1.8** | [`v0_1_8_rtc_complete.rs`](v0_1_8_rtc_complete.rs) | **App23 Standard Library Complete Closure** | Complete implementation of 73 remaining `rtc*`/`rt*` symbols (Variant string overloads, byte/char functions, `CVErr`/`IsError`/`IsMissing`/`Erl`, `Hex`/`Oct`/`Format` on Variant, `CurDir`/`Environ`/`FileDateTime`/`ChDrive`, `Date`/`Time` Variant statements, `QBColor`, and headless shims). Reaching 100% of all 181 `rtc*` runtime exports. |
| `v0.1.9` | **Phase 1.9** | `v0_1_9_numeric_coercions.rs` *(Planned)* | **App24 Numeric Intrinsics & Coercions** | 40 typed numeric and primitive math/coercion helpers: `__vbaI2Abs`, `__vbaI4Abs`, `__vbaCyAbs`, `__vbaI2Sgn`, `__vbaI4Sgn`, `__vbaUI1Sgn`, `__vbaCySgn`, `__vbaR4Sgn`, `__vbaR8Sgn`, `__vbaCyFix`, `__vbaCyInt`, `__vbaCySub`, `__vbaCyI2`, `__vbaCyI4`, `__vbaCyUI1`, `__vbaI2Cy`, `__vbaI4Cy`, `__vbaR4Cy`, `__vbaUI1Cy`, `__vbaFpCmpCy`, `__vbaDateR4`, `__vbaDateR8`, `__vbaFpCDblR4/R8`, `__vbaFpCSngR4/R8`, `__vbaFpUI1`, `__vbaPowerR8`, `__vbaR8FixI2/I4`, `__vbaR8IntI2/I4`, `__vbaBoolErrVar`, `__vbaCyErrVar`, `__vbaI2ErrVar`, `__vbaR4ErrVar`, `__vbaUI1ErrVar`, `__vbaUI1I4`, `__vbaUI1Var`, `__vbaVarDateVar`. |
| `v0.1.10` | **Phase 1.10** | `v0_1_10_variant_bitwise_relational.rs` *(Planned)* | **App25 Variant Bitwise & Relational Testing** | 20 compiler helpers for Variant boolean/bitwise algebra and value comparison testing: `__vbaVarOr`, `__vbaVarXor`, `__vbaVarEqv`, `__vbaVarImp`, `__vbaVarPow`, `__vbaVarTstEq..Ge`, `__vbaVarTextTstEq..Ne`, `__vbaVarTextCmpEq..Ne`, `__vbaVarSetVar`, `__vbaVarSetVarAddref`, `__vbaVarZero`, `__vbaVarIndexStore`, `__vbaVar2Vec`. |
| `v0.1.11` | **Phase 1.11** | `v0_1_11_core_language_closure.rs` *(Planned)* | **App26 Core Language 100% Closure** | 14 compiler helpers completing the Core Language: advanced strings (`__vbaInStrB`, `__vbaInStrVarB`, `__vbaMidStmtBstrB`, `__vbaMidStmtVar`, `__vbaMidStmtVarB`, `__vbaStrCompVar`, `__vbaStrTextCmp`, `__vbaStrVarCopy`, `__vbaVarLike*`, `__vbaVarTextLike*`, `rtcStrReverse`, `__vbaStr2Vec`), arrays & VarArgs (`__vbaAryVarVarg`, `__vbaFreeVarg`, `__vbaVargParmRef`, `__vbaVargVar*`, `__vbaAryConstruct2`, `__vbaAryMove`, `__vbaAryRebase1Var`, `__vbaDerefAry*`, `__vbaLdZeroAry`, `__vbaStrAryToAnsi/Unicode`, `__vbaRedimVar*`, `__vbaRedimPreserveVar*`, `__vbaEraseKeepData/NoPop`), and core execution control (`__vbaError`, `__vbaStopExe`, `__vbaCheckType*`). Reaching **100% Core Language Completion**. |
| `v0.2.1` | **Phase 2.1** | [`v0_2_1_com_instantiation.rs`](v0_2_1_com_instantiation.rs) | **App11 COM Factory & Instantiation** | `registry.json`, `DllGetClassObject`, `IClassFactory`, `rtcCreateObject2`, `rtcCreateObject`, error 429, `rtcIsObject`, `__vbaObjIs`, `__vbaObjVar`, `__vbaObjSetAddref`, `TypeName`. |
| `v0.2.2` | **Phase 2.2** | [`v0_2_2_typelib_parser.rs`](v0_2_2_typelib_parser.rs) | **App12 TypeLib Parser & Introspection** | MSFT binary parser (`MSFT\x02\x00\x01\x00`), synthetic `ITypeLib`/`ITypeInfo` generation, `DispidCache`, `__vbaNew` (341), `__vbaCastObj` (178), `__vbaStrVarMove` (437), `IDictionary` dual-interface vtable. |
| `v0.2.3` | **Phase 2.3** | [`v0_2_3_dispatch_engine.rs`](v0_2_3_dispatch_engine.rs) | **App13 Late-Binding Dispatch Engine** | `IDispatch::GetIDsOfNames`, `IDispatch::Invoke`, `__vbaLateMemCall*`, `__vbaLateIdCall*`, `__vbaVarLateMem*`, `__vbaUnkVar`, `__vbaVarSetObjAddref`, error trapping (438, 448, 450). |
| `v0.2.4` | **Phase 2.4** | [`v0_2_4_activex_shims.rs`](v0_2_4_activex_shims.rs) | **App14 ActiveX Shims & Instantiation** | `Scripting.FileSystemObject` & `TextStream`, `WScript.Shell`, `WScript.Network`, `ADODB.Recordset` (`Fields`, `Field`, `AddNew`, `Update`), `rtcGetObject` (626). |
| `v0.2.5` | **Phase 2.5** | [`v0_2_5_consolidation_typeinfo.rs`](v0_2_5_consolidation_typeinfo.rs) | **App15 ActiveX Consolidation** | COM `ITypeInfo` vtable on native shims, dynamic member invocation via `rtcCallByName` (715), `TypeName` reflection, error 438 trapping. |
| `v0.2.6` | **Phase 2.6** | [`v0_2_6_threading_apartments.rs`](v0_2_6_threading_apartments.rs) | **App16 STA Concurrency & Threading** | STA apartments (TLS isolation, RPC rejection), worker thread context lifecycle, SafeArray lock violations, ByRef aliasing, BSTR embedded nulls, Banker's rounding, FDIV workarounds, ROT & IPC dispatch proxy. |
| `v0.2.7` | **Phase 2.7** | [`v0_2_7_esoteric_zombie_aliasing.rs`](v0_2_7_esoteric_zombie_aliasing.rs) | **App17 Esoteric Quirks & Zombie Objects** | Zombie object invocation lockout (`E_UNEXPECTED`), SafeArray lock violations during iteration (Error 10), ByRef parameter memory aliasing, deep ByRef variant dereferencing, re-entrant SEH frame unwinding, circular reference cleanup. |
| `v0.2.8` | **Phase 2.8** | [`v0_2_8_complex_strings.rs`](v0_2_8_complex_strings.rs) | **App18 Complex Strings & Codepages** | BSTR embedded null handling, `vbNullString` zero pointer equivalence, bidirectional `StrConv` (Unicode/ANSI Windows-1252 & casing), `String$` character code mod 256 wrapping, `Like` operator wildcard pattern matching with ranges, negation, and escapes. |
| `v0.2.9` | **Phase 2.9** | [`v0_2_9_complex_math.rs`](v0_2_9_complex_math.rs) | **App19 Complex Math & FDIV** | IEEE 754 Banker's Rounding (Round-to-Nearest-Even), CRT/C2 FPU workaround routines (`_adj_fdiv_*`, `_CI*`), integer division `\` and `Mod` truncation and Error 11 trapping, Currency 128-bit precision math, transcendental functions with domain error traps (Error 5, Error 6), 96-bit Variant Decimal arithmetic. |
| `All` | **E2E** | [`retroactive_binaries.rs`](retroactive_binaries.rs) | **Retroactive Binary Regression Suite** | Executes all 43 compiled PE32 binaries (`artifacts/bin/*.exe`, Native & P-Code, App01–App23) in process isolation via `vb6-run`. |

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
App21-udt-fixed-strings-native.exe / App21-udt-fixed-strings-pcode.exe
App22-extended-rtc-native.exe / App22-extended-rtc-pcode.exe
App23-rtc-complete-native.exe / App23-rtc-complete-pcode.exe
```
