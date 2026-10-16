# Reverse Engineering Artifacts & Test Data

This directory contains test inputs, reference specifications, and symbol export tables used for validating the **`msvbvm60`** runtime clone.

---

## 📦 Directory Contents

| File | Size | Description |
| :--- | :--- | :--- |
| **`App01-helloworld-pcode.exe`** | 12 KB | Sample standalone test binary compiled with Visual Basic 6.0 (PE32 / P-Code Sub Main console application). Used to validate the PE32 loader, `VBHeader` parser, and native Linux container execution. |
| **`App03-bstr-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating BSTR lifecycle and string operations (`Concat`, `Len`, `Mid`, `Left`, `Right`, `InStr`, `UCase`, `CStr`). |
| **`App03-bstr-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating BSTR lifecycle and string operations (`Concat`, `Len`, `Mid`, `Left`, `Right`, `InStr`, `UCase`, `CStr`). |
| **`App04-variant-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating the Full VARIANT dynamic coercion engine (`VarAdd`, `VarSub`, `VarMul`, `VarDiv`, `VarIdiv`, `VarMod`, `VarCat`, `VarNeg`, `VarNot`, `VarTst`, and `VarType`). |
| **`App04-variant-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating the Full VARIANT dynamic coercion engine (`VarAdd`, `VarSub`, `VarMul`, `VarDiv`, `VarIdiv`, `VarMod`, `VarCat`, `VarNeg`, `VarNot`, `VarTst`, and `VarType`). |
| **`App05-fileio-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating Win32 File I/O shims (`CreateFileA`, `WriteFile`, `ReadFile`, `CloseHandle`, `GetFileSize`, and error handling). |
| **`App05-fileio-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating Win32 File I/O shims (`CreateFileA`, `WriteFile`, `ReadFile`, `CloseHandle`, `GetFileSize`, and error handling). |
| **`App06-errorhandling-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating structured error handling (`On Error Resume Next`, `On Error GoTo <Label>`, `On Error GoTo 0`, and COM `ErrObject` methods). |
| **`App06-errorhandling-pcode.exe`** | 20 KB | Compiled P-Code VB6 console application validating structured error handling (`On Error Resume Next`, `On Error GoTo <Label>`, `On Error GoTo 0`, and COM `ErrObject` methods). |
| **`App07-env-datetime-fs-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating environment variables (`Environ$`), working directory (`CurDir$`), date/time (`Date$`, `Time$`, `Now`, `Timer`), and filesystem (`MkDir`, `RmDir`). |
| **`App07-env-datetime-fs-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating environment variables (`Environ$`), working directory (`CurDir$`), date/time (`Date$`, `Time$`, `Now`, `Timer`), and filesystem (`MkDir`, `RmDir`). |
| **`App08-arrays-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating dynamic arrays, `SafeArray` lifecycle (`ReDim`, `ReDim Preserve`, `LBound`, `UBound`, `Erase`, `AryCopy`), array strings (`Split`, `Join`, `Filter`), and `Array()`. |
| **`App08-arrays-pcode.exe`** | 16 KB | Compiled P-Code VB6 console application validating dynamic arrays, `SafeArray` lifecycle (`ReDim`, `ReDim Preserve`, `LBound`, `UBound`, `Erase`, `AryCopy`), array strings (`Split`, `Join`, `Filter`), and `Array()`. |
| **`App09-classic-fileio-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating file channels (`#1..#511`), `FreeFile`, `Open`, `Print #`, `Line Input #`, `Put #`, `Get #`, `Seek`, `LOF`, `EOF`, `Dir$`. |
| **`App09-classic-fileio-pcode.exe`** | 16 KB | Compiled P-Code VB6 console application validating file channels (`#1..#511`), `FreeFile`, `Open`, `Print #`, `Line Input #`, `Put #`, `Get #`, `Seek`, `LOF`, `EOF`, `Dir$`. |
| **`App10-core-primitives-native.exe`** | 24 KB | Compiled Native Code VB6 console application validating conversions (`CLng`, `CDbl`, `CCur`, etc.), math (`Round`, `Sqr`), string functions, and `TypeName`. |
| **`App10-core-primitives-pcode.exe`** | 16 KB | Compiled P-Code VB6 console application validating conversions (`CLng`, `CDbl`, `CCur`, etc.), math (`Round`, `Sqr`), string functions, and `TypeName`. |
| **`App11-com-factory-native.exe`** | 16 KB | Compiled Native Code VB6 console application validating `registry.json`, `DllGetClassObject`, `IClassFactory`, `rtcCreateObject2`, error 429. |
| **`App11-com-factory-pcode.exe`** | 16 KB | Compiled P-Code VB6 console application validating `registry.json`, `DllGetClassObject`, `IClassFactory`, `rtcCreateObject2`, error 429. |
| **`App12-typelib-introspection-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating MSFT TypeLib parser, `DispidCache`, `__vbaNew`, `__vbaCastObj`, `IDictionary` dual vtable. |
| **`App12-typelib-introspection-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating MSFT TypeLib parser, `DispidCache`, `__vbaNew`, `__vbaCastObj`, `IDictionary` dual vtable. |
| **`App13-late-binding-dispatch-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating `IDispatch` late-binding dispatch engine, `__vbaLateMemCall*`, `__vbaLateIdCall*`, `__vbaVarLateMem*`. |
| **`App13-late-binding-dispatch-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating `IDispatch` late-binding dispatch engine, `__vbaLateMemCall*`, `__vbaLateIdCall*`, `__vbaVarLateMem*`. |
| **`App14-activex-shims-native.exe`** | 24 KB | Compiled Native Code VB6 console application validating `Scripting.FileSystemObject`, `TextStream`, `WScript.Shell`, `WScript.Network`, `ADODB.Recordset`. |
| **`App14-activex-shims-pcode.exe`** | 16 KB | Compiled P-Code VB6 console application validating `Scripting.FileSystemObject`, `TextStream`, `WScript.Shell`, `WScript.Network`, `ADODB.Recordset`. |
| **`App15-activex-consolidation-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating self-describing `ITypeInfo` vtable on native shims and `rtcCallByName` (715). |
| **`App15-activex-consolidation-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating self-describing `ITypeInfo` vtable on native shims and `rtcCallByName` (715). |
| **`App16-threading-sta-concurrency-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating multithreading, TLS `ErrObject` isolation, and STA apartment affinity. |
| **`App16-threading-sta-concurrency-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating multithreading, TLS `ErrObject` isolation, and STA apartment affinity. |
| **`App17-esoteric-quirks-native.exe`** | 24 KB | Compiled Native Code VB6 console application validating zombie objects (`E_UNEXPECTED`), SafeArray lock violations, ByRef memory aliasing, deep ByRef variant dereferencing. |
| **`App17-esoteric-quirks-pcode.exe`** | 16 KB | Compiled P-Code VB6 console application validating zombie objects (`E_UNEXPECTED`), SafeArray lock violations, ByRef memory aliasing, deep ByRef variant dereferencing. |
| **`scrrun.dll`** | 172 KB | Reference Microsoft Scripting Runtime v5.8 TypeLib binary used for automated TypeLib parsing test validation. |
| **`msvbvm60.txt`** | 27 KB | Export symbol dump generated with `dumpbin /exports`. Contains the complete listing of all 635 named export symbols (ordinals 100 through 2010), hints, and relative virtual addresses (RVAs) used for ABI compatibility mapping. |
| **`REFERENCE.md`** | 3.7 KB | Specification of the reference Windows XP SP3 baseline build environment (mounted at `/mnt/winxp`), compiler toolchain (VB6 SP6 Build 8176, C2/LINK 6.00.8168), and System32 library baseline. |

> [!NOTE]
> **Proprietary Binaries Notice**:
> In accordance with software copyright standards, this repository **does not distribute** proprietary Microsoft binaries or dynamic-link libraries (`MSVBVM60.DLL`). 
> Users wishing to perform side-by-side differential analysis or binary verification may supply their own legally obtained copy extracted from a licensed Windows installation or official redistributable package.

> [!IMPORTANT]
> **Trademark Notice**:
> Microsoft®, Windows®, Visual Basic®, and MSVB are registered trademarks or trademarks of Microsoft Corporation or their respective owners. ClassicBasic is an independent clean-room implementation not affiliated with Microsoft Corporation.
