# Reverse Engineering Artifacts & Test Data

This directory contains test inputs, reference specifications, and symbol export tables used for validating the **`msvbvm60`** runtime clone.

---

## 📦 Directory Structure

```text
artifacts/
├── README.md               # Directory overview and fixture catalog
├── REFERENCE.md            # Windows XP SP3 baseline reference specification
├── bin/                    # Compiled 32-bit PE32 executables and test libraries
│   ├── App01-helloworld-pcode.exe
│   ├── App03-bstr-native.exe / App03-bstr-pcode.exe
│   ├── App04-variant-native.exe / App04-variant-pcode.exe
│   ├── App05-fileio-native.exe / App05-fileio-pcode.exe
│   ├── App06-errorhandling-native.exe / App06-errorhandling-pcode.exe
│   ├── App07-env-datetime-fs-native.exe / App07-env-datetime-fs-pcode.exe
│   ├── App08-arrays-native.exe / App08-arrays-pcode.exe
│   ├── App09-classic-fileio-native.exe / App09-classic-fileio-pcode.exe
│   ├── App10-core-primitives-native.exe / App10-core-primitives-pcode.exe
│   ├── App11-com-factory-native.exe / App11-com-factory-pcode.exe
│   ├── App12-typelib-introspection-native.exe / App12-typelib-introspection-pcode.exe
│   ├── App13-late-binding-dispatch-native.exe / App13-late-binding-dispatch-pcode.exe
│   ├── App14-activex-shims-native.exe / App14-activex-shims-pcode.exe
│   ├── App15-activex-consolidation-native.exe / App15-activex-consolidation-pcode.exe
│   ├── App16-threading-sta-concurrency-native.exe / App16-threading-sta-concurrency-pcode.exe
│   ├── App17-esoteric-quirks-native.exe / App17-esoteric-quirks-pcode.exe
│   ├── App18-complex-strings-native.exe / App18-complex-strings-pcode.exe
│   ├── App19-math-rounding-native.exe / App19-math-rounding-pcode.exe
│   ├── App20-loops-iterators-native.exe / App20-loops-iterators-pcode.exe
│   ├── App21-udt-fixed-strings-native.exe / App21-udt-fixed-strings-pcode.exe
│   ├── App22-extended-rtc-native.exe / App22-extended-rtc-pcode.exe
│   └── App23-rtc-complete-native.exe / App23-rtc-complete-pcode.exe
├── src/                    # Original Visual Basic 6 source projects (.bas, .vbp, .cls)
│   ├── App03.bas / App03.vbp
│   ├── App04-variant.bas / App04-variant.vbp
│   ├── App05-fileio.bas / App05-fileio.vbp
│   ├── App06-errorhandling.bas / App06-errorhandling.vbp
│   ├── App07-env-datetime-fs.bas / App07-env-datetime-fs.vbp
│   ├── App08-arrays.bas / App08-arrays.vbp
│   ├── App09-classic-fileio.bas / App09-classic-fileio.vbp
│   ├── App10-core-primitives.bas / App10-core-primitives.vbp
│   ├── App11-com-factory.bas / App11-com-factory.vbp
│   ├── App12-typelib-introspection.bas / App12-typelib-introspection.vbp
│   ├── App13-late-binding-dispatch.bas / App13-late-binding-dispatch.vbp
│   ├── App14-activex-shims.bas / App14-activex-shims.vbp
│   ├── App15-activex-consolidation.bas / App15-activex-consolidation.vbp
│   ├── App16-threading-sta-concurrency.bas / App16-threading-sta-concurrency.vbp
│   ├── App17-esoteric-quirks.bas / App17-esoteric-quirks.vbp
│   ├── App18-complex-strings.bas / App18-complex-strings.vbp
│   ├── App19-math-rounding.bas / App19-math-rounding.vbp
│   ├── App20-loops-iterators.bas / App20-loops-iterators.vbp
│   ├── App21-udt-fixed-strings.bas / App21-udt-fixed-strings.vbp
│   ├── App22-extended-rtc.bas / App22-extended-rtc.vbp
│   ├── App23-rtc-complete.bas / App23-rtc-complete.vbp
│   ├── App24-numeric-coercions.bas / App24-numeric-coercions.vbp
│   ├── App25-variant-bitwise-relational.bas / App25-text-compare.bas / App25-variant-bitwise-relational.vbp
│   ├── App26-core-language-closure.bas / App26-core-language-closure.vbp
│   └── CNode.cls / CZombie.cls
└── ref/                    # Reference symbol exports and dumpbin outputs
    └── msvbvm60.txt

```

---

## 📋 Fixture Executable Catalog (`artifacts/bin/`)

| File | Size | Description |
| :--- | :--- | :--- |
| **`bin/App01-helloworld-pcode.exe`** | 12 KB | Sample standalone test binary compiled with Visual Basic 6.0 (PE32 / P-Code Sub Main console application). Validates PE32 loader, `VBHeader` parser, and container execution. |
| **`bin/App03-bstr-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating BSTR lifecycle and string operations (`Concat`, `Len`, `Mid`, `Left`, `Right`, `InStr`, `UCase`, `CStr`). |
| **`bin/App03-bstr-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating BSTR lifecycle and string operations (`Concat`, `Len`, `Mid`, `Left`, `Right`, `InStr`, `UCase`, `CStr`). |
| **`bin/App04-variant-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating the Full VARIANT dynamic coercion engine (`VarAdd`, `VarSub`, `VarMul`, `VarDiv`, `VarIdiv`, `VarMod`, `VarCat`, `VarNeg`, `VarNot`, `VarTst`, and `VarType`). |
| **`bin/App04-variant-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating the Full VARIANT dynamic coercion engine (`VarAdd`, `VarSub`, `VarMul`, `VarDiv`, `VarIdiv`, `VarMod`, `VarCat`, `VarNeg`, `VarNot`, `VarTst`, and `VarType`). |
| **`bin/App05-fileio-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating Win32 File I/O shims (`CreateFileA`, `WriteFile`, `ReadFile`, `CloseHandle`, `GetFileSize`, and error handling). |
| **`bin/App05-fileio-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating Win32 File I/O shims (`CreateFileA`, `WriteFile`, `ReadFile`, `CloseHandle`, `GetFileSize`, and error handling). |
| **`bin/App06-errorhandling-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating structured error handling (`On Error Resume Next`, `On Error GoTo <Label>`, `On Error GoTo 0`, and COM `ErrObject` methods). |
| **`bin/App06-errorhandling-pcode.exe`** | 20 KB | Compiled P-Code VB6 console application validating structured error handling (`On Error Resume Next`, `On Error GoTo <Label>`, `On Error GoTo 0`, and COM `ErrObject` methods). |
| **`bin/App07-env-datetime-fs-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating environment variables (`Environ$`), working directory (`CurDir$`), date/time (`Date$`, `Time$`, `Now`, `Timer`), and filesystem (`MkDir`, `RmDir`). |
| **`bin/App07-env-datetime-fs-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating environment variables (`Environ$`), working directory (`CurDir$`), date/time (`Date$`, `Time$`, `Now`, `Timer`), and filesystem (`MkDir`, `RmDir`). |
| **`bin/App08-arrays-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating dynamic arrays, `SafeArray` lifecycle (`ReDim`, `ReDim Preserve`, `LBound`, `UBound`, `Erase`, `AryCopy`), array strings (`Split`, `Join`, `Filter`), and `Array()`. |
| **`bin/App08-arrays-pcode.exe`** | 16 KB | Compiled P-Code VB6 console application validating dynamic arrays, `SafeArray` lifecycle (`ReDim`, `ReDim Preserve`, `LBound`, `UBound`, `Erase`, `AryCopy`), array strings (`Split`, `Join`, `Filter`), and `Array()`. |
| **`bin/App09-classic-fileio-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating file channels (`#1..#511`), `FreeFile`, `Open`, `Print #`, `Line Input #`, `Put #`, `Get #`, `Seek`, `LOF`, `EOF`, `Dir$`. |
| **`bin/App09-classic-fileio-pcode.exe`** | 16 KB | Compiled P-Code VB6 console application validating file channels (`#1..#511`), `FreeFile`, `Open`, `Print #`, `Line Input #`, `Put #`, `Get #`, `Seek`, `LOF`, `EOF`, `Dir$`. |
| **`bin/App10-core-primitives-native.exe`** | 24 KB | Compiled Native Code VB6 console application validating conversions (`CLng`, `CDbl`, `CCur`, etc.), math (`Round`, `Sqr`), string functions, and `TypeName`. |
| **`bin/App10-core-primitives-pcode.exe`** | 16 KB | Compiled P-Code VB6 console application validating conversions (`CLng`, `CDbl`, `CCur`, etc.), math (`Round`, `Sqr`), string functions, and `TypeName`. |
| **`bin/App11-com-factory-native.exe`** | 16 KB | Compiled Native Code VB6 console application validating `registry.json`, `DllGetClassObject`, `IClassFactory`, `rtcCreateObject2`, error 429. |
| **`bin/App11-com-factory-pcode.exe`** | 16 KB | Compiled P-Code VB6 console application validating `registry.json`, `DllGetClassObject`, `IClassFactory`, `rtcCreateObject2`, error 429. |
| **`bin/App12-typelib-introspection-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating MSFT TypeLib parser, `DispidCache`, `__vbaNew`, `__vbaCastObj`, `IDictionary` dual vtable. |
| **`bin/App12-typelib-introspection-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating MSFT TypeLib parser, `DispidCache`, `__vbaNew`, `__vbaCastObj`, `IDictionary` dual vtable. |
| **`bin/App13-late-binding-dispatch-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating `IDispatch` late-binding dispatch engine, `__vbaLateMemCall*`, `__vbaLateIdCall*`, `__vbaVarLateMem*`. |
| **`bin/App13-late-binding-dispatch-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating `IDispatch` late-binding dispatch engine, `__vbaLateMemCall*`, `__vbaLateIdCall*`, `__vbaVarLateMem*`. |
| **`bin/App14-activex-shims-native.exe`** | 24 KB | Compiled Native Code VB6 console application validating `Scripting.FileSystemObject`, `TextStream`, `WScript.Shell`, `WScript.Network`, `ADODB.Recordset`. |
| **`bin/App14-activex-shims-pcode.exe`** | 16 KB | Compiled P-Code VB6 console application validating `Scripting.FileSystemObject`, `TextStream`, `WScript.Shell`, `WScript.Network`, `ADODB.Recordset`. |
| **`bin/App15-activex-consolidation-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating self-describing `ITypeInfo` vtable on native shims and `rtcCallByName` (715). |
| **`bin/App15-activex-consolidation-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating self-describing `ITypeInfo` vtable on native shims and `rtcCallByName` (715). |
| **`bin/App16-threading-sta-concurrency-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating multithreading, TLS `ErrObject` isolation, and STA apartment affinity. |
| **`bin/App16-threading-sta-concurrency-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating multithreading, TLS `ErrObject` isolation, and STA apartment affinity. |
| **`bin/App17-esoteric-quirks-native.exe`** | 24 KB | Compiled Native Code VB6 console application validating zombie objects (`E_UNEXPECTED`), SafeArray lock violations, ByRef memory aliasing, deep ByRef variant dereferencing. |
| **`bin/App17-esoteric-quirks-pcode.exe`** | 16 KB | Compiled P-Code VB6 console application validating zombie objects (`E_UNEXPECTED`), SafeArray lock violations, ByRef memory aliasing, deep ByRef variant dereferencing. |
| **`bin/App18-complex-strings-native.exe`** | 24 KB | Compiled Native Code VB6 console application validating complex string manipulation, embedded nulls, StrConv ANSI/Unicode conversions, String$ overflow, and Like operator. |
| **`bin/App18-complex-strings-pcode.exe`** | 16 KB | Compiled P-Code VB6 console application validating complex string manipulation, embedded nulls, StrConv ANSI/Unicode conversions, String$ overflow, and Like operator. |
| **`bin/App19-math-rounding-native.exe`** | 28 KB | Compiled Native Code VB6 console application validating Banker's Rounding, x87 Pentium FDIV workarounds, Currency 128-bit precision, transcendental functions, and 96-bit Variant Decimal arithmetic. |
| **`bin/App19-math-rounding-pcode.exe`** | 20 KB | Compiled P-Code VB6 console application validating Banker's Rounding, x87 Pentium FDIV workarounds, Currency 128-bit precision, transcendental functions, and 96-bit Variant Decimal arithmetic. |
| **`bin/App20-loops-iterators-native.exe`** | 24 KB | Compiled Native Code VB6 console application validating typed `For..Next`, 64-bit `Currency` loops, dynamic `Variant` loops, `SafeArray` and `Variant` `For Each` iteration, `GoSub..Return`, and `On..GoTo`. |
| **`bin/App20-loops-iterators-pcode.exe`** | 16 KB | Compiled P-Code VB6 console application validating typed `For..Next`, 64-bit `Currency` loops, dynamic `Variant` loops, `SafeArray` and `Variant` `For Each` iteration, `GoSub..Return`, and `On..GoTo`. |
| **`bin/App21-udt-fixed-strings-native.exe`** | 28 KB | Compiled Native Code VB6 console application validating `Type..End Type` UDT records, record assignment, UDT arrays, `VT_RECORD` Variant wrapping, ANSI/Unicode conversions, and fixed-length strings `String * N`. |
| **`bin/App21-udt-fixed-strings-pcode.exe`** | 20 KB | Compiled P-Code VB6 console application validating `Type..End Type` UDT records, record assignment, UDT arrays, `VT_RECORD` Variant wrapping, ANSI/Unicode conversions, and fixed-length strings `String * N`. |
| **`bin/App22-extended-rtc-native.exe`** | 28 KB | Compiled Native Code VB6 console application validating financial suite (`SLN`, `SYD`, `DDB`, `PMT`, `IPMT`, `PPMT`, `PV`, `FV`, `NPer`, `Rate`, `NPV`, `IRR`, `MIRR`), calendar math (`DateSerial`, `TimeSerial`, `DateAdd`, `DateDiff`, `DatePart`, `Weekday`, `WeekdayName`, `MonthName`, `IsDate`), locale formatters (`FormatNumber`, `FormatCurrency`, `FormatPercent`, `FormatDateTime`), flow utilities (`Switch`, `Partition`), and virtual registry (`SaveSetting`, `GetSetting`, `GetAllSettings`, `DeleteSetting`). |
| **`bin/App22-extended-rtc-pcode.exe`** | 20 KB | Compiled P-Code VB6 console application validating financial suite (`SLN`, `SYD`, `DDB`, `PMT`, `IPMT`, `PPMT`, `PV`, `FV`, `NPer`, `Rate`, `NPV`, `IRR`, `MIRR`), calendar math (`DateSerial`, `TimeSerial`, `DateAdd`, `DateDiff`, `DatePart`, `Weekday`, `WeekdayName`, `MonthName`, `IsDate`), locale formatters (`FormatNumber`, `FormatCurrency`, `FormatPercent`, `FormatDateTime`), flow utilities (`Switch`, `Partition`), and virtual registry (`SaveSetting`, `GetSetting`, `GetAllSettings`, `DeleteSetting`). |
| **`bin/App23-rtc-complete-native.exe`** | 28 KB | Compiled Native Code VB6 console application validating 100% complete closure of `rtc*` runtime exports (Variant string operations `Left`/`Right`/`Mid`/`Trim`/`LTrim`/`RTrim`/`LCase`/`UCase`/`Space`/`String`/`InStr`/`Len`/`StrComp`, byte/char ops `LenB`/`MidB`/`LeftB`/`RightB`/`InStrB`/`Chr`/`Asc`/`ChrB`/`AscB`/`ChrW`/`AscW`, `CVErr`/`IsError`/`IsMissing`/`Erl`, `Hex`/`Oct`/`Format` on Variant, `CurDir`/`Environ`/`FileDateTime`/`ChDrive`, `Date`/`Time` Variant statements, `QBColor`, and headless shims). |
| **`bin/App23-rtc-complete-pcode.exe`** | 20 KB | Compiled P-Code VB6 console application validating 100% complete closure of `rtc*` runtime exports (Variant string operations `Left`/`Right`/`Mid`/`Trim`/`LTrim`/`RTrim`/`LCase`/`UCase`/`Space`/`String`/`InStr`/`Len`/`StrComp`, byte/char ops `LenB`/`MidB`/`LeftB`/`RightB`/`InStrB`/`Chr`/`Asc`/`ChrB`/`AscB`/`ChrW`/`AscW`, `CVErr`/`IsError`/`IsMissing`/`Erl`, `Hex`/`Oct`/`Format` on Variant, `CurDir`/`Environ`/`FileDateTime`/`ChDrive`, `Date`/`Time` Variant statements, `QBColor`, and headless shims). |
| **`bin/App24-numeric-coercions-native.exe`** | 28 KB *(Planned)* | Compiled Native Code VB6 console application validating 40 typed numeric math, Abs/Sgn, Currency coercion, and floating-point/Date conversion helpers (`Phase 1.9`). |
| **`bin/App24-numeric-coercions-pcode.exe`** | 20 KB *(Planned)* | Compiled P-Code VB6 console application validating 40 typed numeric math, Abs/Sgn, Currency coercion, and floating-point/Date conversion helpers (`Phase 1.9`). |
| **`bin/App25-variant-bitwise-relational-native.exe`** | 28 KB *(Planned)* | Compiled Native Code VB6 console application validating 20 Variant bitwise (`Or`, `Xor`, `Eqv`, `Imp`), relational testing (`VarTst*`), Option Compare Text (`VarText*`), and vector operations (`Phase 1.10`). |
| **`bin/App25-variant-bitwise-relational-pcode.exe`** | 20 KB *(Planned)* | Compiled P-Code VB6 console application validating 20 Variant bitwise (`Or`, `Xor`, `Eqv`, `Imp`), relational testing (`VarTst*`), Option Compare Text (`VarText*`), and vector operations (`Phase 1.10`). |
| **`bin/App26-core-language-closure-native.exe`** | 28 KB *(Planned)* | Compiled Native Code VB6 console application validating 14 Core Language closure helpers: ParamArray/VarArgs (`Varg*`), dynamic Variant arrays, byte strings (`InStrB`, `MidB`), and pattern matching (`VarLike*`) (`Phase 1.11`). |
| **`bin/App26-core-language-closure-pcode.exe`** | 20 KB *(Planned)* | Compiled P-Code VB6 console application validating 14 Core Language closure helpers: ParamArray/VarArgs (`Varg*`), dynamic Variant arrays, byte strings (`InStrB`, `MidB`), and pattern matching (`VarLike*`) (`Phase 1.11`). |
| **`ref/msvbvm60.txt`** | 27 KB | Export symbol dump generated with `dumpbin /exports`. Complete listing of all 635 named export symbols (ordinals 100 through 2010), hints, and RVAs. |
| **`REFERENCE.md`** | 3.7 KB | Specification of the reference Windows XP SP3 baseline build environment (mounted at `/mnt/winxp`), compiler toolchain (VB6 SP6 Build 8176, C2/LINK 6.00.8168), and System32 library baseline. |

> [!NOTE]
> **Proprietary Binaries Notice**:
> In accordance with software copyright standards, this repository **does not distribute** proprietary Microsoft binaries or dynamic-link libraries (`MSVBVM60.DLL`, `SCRRUN.DLL`, etc.). 
> TypeLib parsing and ActiveX tests utilize 100% synthetic clean-room byte structures generated in-memory.
> Users wishing to perform side-by-side differential analysis or binary verification may supply their own legally obtained copy extracted from a licensed Windows installation or official redistributable package.

> [!IMPORTANT]
> **Trademark Notice**:
> Microsoft®, Windows®, Visual Basic®, and MSVB are registered trademarks or trademarks of Microsoft Corporation or their respective owners. ClassicBasic is an independent clean-room implementation not affiliated with Microsoft Corporation.
