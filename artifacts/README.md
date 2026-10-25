# Reverse Engineering Artifacts & Compiled PE32 Test Fixtures

This directory contains the 77 compiled 32-bit Visual Basic 6.0 PE32 executables (`artifacts/bin/`), original VB6 project sources (`artifacts/src/`), reference specifications (`artifacts/REFERENCE.md`), and canonical symbol export dumps (`artifacts/ref/msvbvm60.txt`) used to validate the **ClassicBasic** workspace (`cb-x86`, `cb-pe`, `cb-vfs`, `cb-win32`, `cb-webserver`, `msvbvm60`, and `vb6-run`).

---

## Directory Layout

- **[`REFERENCE.md`](REFERENCE.md)**: Specification of the Windows XP SP3 baseline build environment, VB6 SP6 (Build 8176) compiler, and `C2.EXE`/`LINK.EXE` toolchain.
- **`bin/`**: 77 compiled 32-bit Windows PE32 executables (`App01` through `App40` in both Native x86 and P-Code compilation modes).
- **`src/`**: Original Visual Basic 6.0 source projects (`.vbp`, `.bas`, `.cls`) corresponding to each compiled fixture.
- **`ref/msvbvm60.txt`**: Canonical `dumpbin /exports` symbol table of `MSVBVM60.DLL` (635 named exports, ordinals `100` through `2010`).

---

## Compiled PE32 Executable Catalog (`artifacts/bin/`)

Every fixture (except `App01`, which is P-Code only) is compiled into both **Native Code** (`*-native.exe`, executed by `cb-x86` + `msvbvm60::x86::bridge`) and **P-Code** (`*-pcode.exe`, executed by `msvbvm60::pcode::vm`):

| Fixture Pair (`-native.exe` & `-pcode.exe`) | Milestone / Phase | Validated Subsystems & ABI Exports |
| :--- | :---: | :--- |
| **`App01-helloworld-pcode.exe`** | Phase 1.1 | PE32 loader (`cb-pe`), `VBHeader` (`"VB5!"`) parser, `ThunRTMain`, and P-Code console startup. |
| **`App03-bstr-*`** | Phase 1.1 | `BSTR` lifecycle and string operations (`Concat`, `Len`, `Mid`, `Left`, `Right`, `InStr`, `UCase`, `CStr`). |
| **`App04-variant-*`** | Phase 1.1 | `VARIANT` dynamic coercion engine (`VarAdd`, `VarSub`, `VarMul`, `VarDiv`, `VarIdiv`, `VarMod`, `VarCat`, `VarNeg`, `VarNot`, `VarTst`, `VarType`). |
| **`App05-fileio-*`** | Phase 1.1 | Win32 File I/O shims in `cb-win32` (`CreateFileA`, `WriteFile`, `ReadFile`, `CloseHandle`, `GetFileSize`). |
| **`App06-errorhandling-*`** | Phase 1.1 | Structured error handling (`On Error Resume Next`, `On Error GoTo <Label>`, `Resume`, `ErrObject`). |
| **`App07-env-datetime-fs-*`** | Phase 1.1 | Environment (`Environ$`), directory (`CurDir$`, `MkDir`, `RmDir`), and date/time (`Date$`, `Time$`, `Now`, `Timer`). |
| **`App08-arrays-*`** | Phase 1.2 | Dynamic `SafeArray` lifecycle (`ReDim`, `ReDim Preserve`, `LBound`, `UBound`, `Erase`, `Split`, `Join`, `Filter`, `Array()`). |
| **`App09-classic-fileio-*`** | Phase 1.3 | Classic VB6 file channels (`#1..#511`), `FreeFile`, `Open`, `Print #`, `Line Input #`, `Put #`, `Get #`, `Seek`, `LOF`, `EOF`, `Dir$`. |
| **`App10-core-primitives-*`** | Phase 1.4 | Numeric/type conversions (`CLng`, `CDbl`, `CCur`, `CBool`), math (`Round`, `Sqr`), and `TypeName`. |
| **`App11-com-factory-*`** | Phase 2.1 | Virtual Registry (`cb-vfs` `registry.json`), `DllGetClassObject`, `IClassFactory`, `rtcCreateObject2`, Error 429. |
| **`App12-typelib-introspection-*`** | Phase 2.2 | MSFT TypeLib binary parser, `DispidCache`, `__vbaNew`, `__vbaCastObj`, `Scripting.Dictionary` dual vtable. |
| **`App13-late-binding-dispatch-*`** | Phase 2.3 | `IDispatch` late-binding dispatch engine (`__vbaLateMemCall*`, `__vbaLateIdCall*`, `__vbaVarLateMem*`). |
| **`App14-activex-shims-*`** | Phase 2.4 | `Scripting.FileSystemObject`, `TextStream`, `WScript.Shell`, `WScript.Network`, `ADODB.Recordset`. |
| **`App15-activex-consolidation-*`** | Phase 2.5 | Self-describing `ITypeInfo` vtable on native shims and dynamic `rtcCallByName` (ordinal 715). |
| **`App16-threading-sta-concurrency-*`** | Phase 2.6 | Multithreading, per-thread TLS `ErrObject` isolation, and STA apartment affinity. |
| **`App17-esoteric-quirks-*`** | Phase 2.7 | Zombie objects (`E_UNEXPECTED`), `SafeArray` lock violations (Error 10), `ByRef` aliasing, deep `ByRef` `Variant` dereferencing. |
| **`App18-complex-strings-*`** | Phase 2.8 | Embedded nulls (`\0`), `StrConv` ANSI/Unicode conversions, `String$` overflow, and `Like` pattern matching (`__vbaStrLike`). |
| **`App19-math-rounding-*`** | Phase 2.9 | Banker's Rounding, x87 Pentium FDIV workarounds (`_adj_fdiv*`), `Currency` 128-bit precision, `_CI*` transcendentals, 96-bit `Decimal`. |
| **`App20-loops-iterators-*`** | Phase 1.5 | Typed/`Currency`/`Variant` `For..Next` loops, `SafeArray` and `Variant` `For Each` iterators, `GoSub..Return`, `On..GoTo`. |
| **`App21-udt-fixed-strings-*`** | Phase 1.6 | `Type..End Type` UDT records, `CopyRecord`, UDT arrays, `VT_RECORD`, ANSI/Unicode record marshaling, fixed-length strings `String * N`. |
| **`App22-extended-rtc-*`** | Phase 1.7 | Financial suite (`SLN`, `SYD`, `DDB`, `PMT`, `IPMT`, `PPMT`, `PV`, `FV`, `NPer`, `Rate`, `NPV`, `IRR`, `MIRR`), calendar math (`DateAdd`, `DateDiff`, `DatePart`), formatters (`FormatNumber`, `FormatCurrency`, `FormatPercent`, `FormatDateTime`), `Switch`, `Partition`, registry settings. |
| **`App23-rtc-complete-*`** | Phase 1.8 | Complete closure of all 181 `rtc*` exports (`Left`/`Right`/`Mid`/`Trim` Variant overloads, `LenB`/`MidB`/`InStrB`/`ChrB`/`AscB`/`ChrW`/`AscW`, `CVErr`, `IsError`, `IsMissing`, `Erl`, `QBColor`). |
| **`App24-numeric-coercions-*`** | Phase 1.9 | 40 typed numeric math, `Abs`/`Sgn`, `Currency` coercions (`__vba*Cy*`), floating-point/Date conversions (`__vbaFpCDbl*`, `__vbaR8Fix*`, `__vbaR8Int*`), and `*ErrVar` helpers. |
| **`App25-variant-bitwise-relational-*`** | Phase 1.10 | Variant bitwise algebra (`__vbaVarOr`, `__vbaVarXor`, `__vbaVarEqv`, `__vbaVarImp`, `__vbaVarPow`), relational tests (`__vbaVarTst*`), `Option Compare Text` (`__vbaVarText*`), and vector stores. |
| **`App26-core-language-closure-*`** | Phase 1.11 | `ParamArray`/VarArgs (`__vbaVarg*`), dynamic Variant array redimensioning (`__vbaRedimVar*`), byte string operations (`__vbaInStrB`, `__vbaMidStmtBstrB`), and `VarLike`. |
| **`App27-dcom-ipc-*`** | Phase 2.10 | Running Object Table (`IRunningObjectTable`), `GetObject`, and ActiveX component IPC registration. |
| **`App28-event-sinks-collections-*`** | Phase 2.11 | COM connection-point event sinks (`WithEvents`, `RaiseEvent`, `__vbaRaiseEvent`) and `VBA.Collection` `For Each` iterators (`__vbaForEachCollObj`, `__vbaForEachCollVar`). |
| **`App29-abi32-ordinals-*`** | Phase 2.12 | 32-bit struct layout & alignment (`VarPtr`, `Len`, `LenB`), deep UDT copy (`__vbaCopyBytes`, `__vbaRecAssign`), and `GetMem*`/`PutMem*`. |
| **`App30-native-x86-fpu-seh-*`** | Phase 2.13 | Native x86 transcendental x87 FPU pipeline (`Sqr`, `Sin`, `Cos`, `Tan`, `Atn`, `Exp`, `Log`), `__vbaChkstk`, and SEH division-by-zero/overflow trapping. |
| **`App31-pcode-vm-*`** | Phase 2.14 | P-Code stack machine execution (`ProcCallEngine`, `MethCallEngine`), `Select Case` jump tables, loops, and class method dispatch. |
| **`App32-pcode-engine-*`** | Phase 2.15 | Clean-room MSVBVM60 P-Code VM engine (`ProcCallEngine`, `MethCallEngine`, `VCallHresult`), `ProcedureDescriptor` & `ObjectInfo` constant pool reader, nested procedure frames, ByRef parameter binding, Select Case jump tables, error trapping and resume. |
| **`App33-ide-introspection-eb-tip-*`** | Phase 2.16 | IDE hosting, project introspection & debugger telemetry exports (`EbGetHandleOfExecutingProject`, `EbIsProjectOnStack`, `EbGetErrorInfo`, `EbGetObjConnectionCounts`, `EbGetVBAObject`, `TipSetOption`, `TipGetAddressOfPredeclaredInstance`, `TipCreateInstanceEx`, `CreateIExprSrvObj`). |
| **`App34-native-x86-engine-*`** | Phase 2.17 | Native x86 CPU instruction emulator (`cb-x86`), 32-bit ALU and register operations, stdcall stack frames and ByRef parameter passing, x87 FPU pipeline, Win32 `FS:[0]` TIB and SEH unwinding, native class method dispatch and object lifecycle. |
| **`App35-win32-user32-windowing-*`** | Phase 3.1 | Win32 Handle table (`HWND`, `HDC`, `HGDIOBJ`), `VirtualWindowTree`, `VirtualMessageQueue` (`Condvar`), and core `USER32` window lifecycle APIs (`CreateWindowExA`, `ShowWindow`, `GetMessageA`, `DispatchMessageA`). |
| **`App36-win32-gdi32-surface-*`** | Phase 3.2 | `GDI32` 32-bit BGRA `VirtualSurface`, Bresenham line rasterizer, geometric primitives (`Rectangle`, `Ellipse`, `RoundRect`), Pens, Brushes, and coalescing `DirtyRectTracker`. |
| **`App37-gdi-text-fonts-*`** | Phase 3.3 | Static 96 DPI font metric layout calculations (`MS Sans Serif`, `Tahoma`, `Arial`, `Courier New`), `DrawTextA`, `TextOutA`, and `0x07 FRAME_TEXT` structured delegation commands. |
| **`App38-vb6-form-deserializer-*`** | Phase 3.4 | VB6 `.frm` form definition deserializer into `VirtualWindowTree`, Twips-to-Pixels geometric conversion, 10 intrinsic controls COM vtables, property mutation, and form unload lifecycle. |
| **`App39-vb6-form-events-*`** | Phase 3.5 | VB6 Form event sinks (`Form_Load`, `Command1_Click`, `Text1_Change`, `Check1_Click`, `Option1_Click`, `Form_Unload`), ListBox (`AddItem`, `ListCount`, `Clear`), and UI dispatch pipeline. |
| **`App40-gui-websocket-stream-*`** | Phase 3.6 | RFC 6455 WebSocket Server (`cb-webserver`), atomic binary wire protocol frames (`0x02`..`0x07`), GDI pixel damage tracking (`ValidateRect`/`GetUpdateRect`), cursors, VB Clipboard object, text run metrics, and JSON input injection to `VirtualMessageQueue`. |
| **`App41-gui-wasm-client-*`** | Phase 3.7 | WebAssembly HTML5 Canvas 2D client (`cb-client`), linear memory RGBA framebuffer blitter, dirty rect tracking, delegated vector text rendering, DOM pointer/keyboard input event stream injection, and Form/control canvas sync. |

> [!NOTE]
> **Clean-Room & Proprietary Binaries Notice**:
> This repository **does not distribute** any proprietary Microsoft system DLLs (`MSVBVM60.DLL`, `SCRRUN.DLL`, etc.). Only small test fixture executables compiled from our own clean-room VB6 test sources (`artifacts/src/`) are included for automated regression testing.
