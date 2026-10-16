# ClassicBasic (`msvbvm60`)

Clean-room Rust re-implementation of the Microsoft Visual Basic 6.0 runtime (`msvbvm60.dll`) and standalone PE32 loader (`vb6-run`) for executing compiled 32-bit Visual Basic 6.0 binaries (Native Code and P-Code) directly on Linux (`x86_64-alpine-linux-musl`) without Wine.

[![Tests](https://img.shields.io/badge/tests-166%20passed-brightgreen.svg)]()
[![Target](https://img.shields.io/badge/target-x86__64--alpine--linux--musl-blue.svg)]()
[![ABI Exports](https://img.shields.io/badge/exports-635%20%2F%20635-success.svg)]()

---

## Overview

ClassicBasic consists of two main components:
1. **`msvbvm60` (Runtime Library)**: Implements the Visual Basic 6.0 virtual machine ABI (all 635 ordinal exports defined in `msvbvm60.def`), including data types, memory management, intrinsic functions (`rtc*`), and compiler helpers (`__vba*`).
2. **`vb6-run` (Standalone PE32 Loader)**: Maps 32-bit Windows PE executables into virtual memory, applies base relocations (`.reloc`), resolves Import Address Table (IAT) entries against `msvbvm60` and Win32 API shims (`KERNEL32`, `USER32`, `OLEAUT32`), locates the `VB5!` header, and invokes `ThunRTMain`.

### Core Subsystems
- **Execution Entry (`src/thun_rt.rs`)**: Implements `ThunRTMain` (Ordinal 100), parses `VBHeader`, `ProjectInfo`, and `ObjectTable`, and dispatches execution to `Sub Main` or Form initialization. Supports two loader execution modes:
  - `Headless` (`--headless`, default): Executes console, CLI, and batch workloads without graphical output.
  - `GuiWebStream` (`--gui-web`): Initializes the virtual windowing pipeline for streaming GUI forms to a web browser over WebSockets/WebAssembly.
- **Data Types (`src/types/`)**:
  - `BSTR`: UTF-16 length-prefixed COM strings (`SysAllocStringLen`, `SysFreeString`), in-place mutation (`Mid$` statement), and byte array conversions (`Byte()` <-> `String`).
  - `VARIANT`: 16-byte tagged union supporting dynamic type coercion, arithmetic, unary operations, and relational comparisons.
  - `SAFEARRAY`: Multi-dimensional array descriptors with dynamic bounds and in-place resizing.
  - `Currency`: 64-bit fixed-point integer scaled by 10,000.
- **Error Handling (`src/err.rs`)**: Structured error state machine (`__vbaOnError`, `__vbaResume`, `__vbaExitProc`) and COM `ErrObject` (`Err.Number`, `Err.Description`, `Err.Source`, `Err.Clear`, `Err.Raise`).
- **Memory & Pointers (`src/mem.rs`)**: Direct memory primitives (`VarPtr`, `GetMem1..8`, `PutMem1..8`, `SetMemVar`).
- **Virtualized Windows Registry (`src/registry/`)**: Transparently shims Win32 Registry APIs (`ADVAPI32`: `RegOpenKeyExA`, `RegQueryValueExA`, `RegCloseKey`, etc.) and COM `ProgID`/`CLSID` resolution. The application transparently assumes it is querying a native Windows Registry, while state is persisted in an application-sandboxed JSON file (`registry.json`). The storage layer is decoupled behind a provider interface, enabling future centralization via a dedicated registry service pod for distributed client containers.
- **COM & Class Factory (`src/com.rs`)**: In-process COM lifecycle (`DllGetClassObject`, `IClassFactory::CreateInstance`, `IUnknown::QueryInterface/AddRef/Release`), `rtcCreateObject2` / `rtcCreateObject`, object identity (`__vbaObjIs`), reference counting (`__vbaObjSetAddref`, `__vbaFreeObj`), and type inspection (`rtcIsObject`, `rtcTypeName`).

---

## Roadmap & Milestone Completion

Development follows a strict sequential order: completing core language primitives first, followed by COM/ActiveX support, and finally Win32 GUI rendering via WebAssembly/WebSocket canvas streaming.

| Milestone | Completion | Implemented | Remaining |
| :--- | :---: | :--- | :--- |
| **1. Console, CLI & Core Language** | **100%** | • PE32 loader, `.reloc`, IAT resolver, `DllFunctionCall`<br>• Win32 console & file I/O shims (`CreateFileA`, `ReadFile`, `WriteFile`, `CloseHandle`, `GetFileSize`)<br>• `BSTR` & `VARIANT` coercion/math/comparison engines<br>• Error handling (`On Error`, `ErrObject`)<br>• Environment, Date/Time & Filesystem (`Environ$`, `CurDir$`, `Now`, `Timer`, `Date$`, `Time$`, `MkDir`, `RmDir`, `Kill`, `ChDir`)<br>• **Iteration 1 (Arrays — 100%)**: `SafeArray`, `ReDim`, `ReDim Preserve`, `LBound`, `UBound`, `Erase`, `Split`, `Join`, `Filter`, `Array`<br>• **Iteration 2 (Classic File I/O — 100%)**: Virtual file channels (`#1..#511`), `FreeFile`, `Open`, `Close`, `Reset`, `Print #`, `Write #`, `Line Input #`, `Input #`, `Put #`, `Get #`, `EOF`, `LOF`, `Seek`, `Loc`, `Dir$`, `FileCopy`, `FileLen`, `Name..As`<br>• **Iteration 3 (Core Primitives — 100%)**: `Command$`, `End`, `IIf`, `Choose`, `Val`, `Str$`, `Format$`, `Hex$`, `Oct$`, `Asc`, `Chr$`, `CInt`, `CLng`, `CDbl`, `CBool`, `CCur`, `CSng`, `IsNumeric`, `IsEmpty`, `IsNull`, `IsArray`, `TypeName`, `Round`, `Sqr`, `Abs`, `Sgn`, `Int`, `Fix`, `Replace`, `InStrRev`, `LCase$`, `Mid$` statement, `Byte()` coercion | • Milestone Complete |
| **2. ActiveX & COM** | **35%** | • **Phase 2.1 (Discovery & Instantiation — 100%)**: Sandboxed virtual `registry.json` (`ProgID` -> `CLSID` -> `InprocServer32`), concrete `DllGetClassObject`, `IClassFactory::CreateInstance`, `rtcCreateObject` (625), `rtcCreateObject2` (716), missing ProgID error 429, `rtcIsObject` (562), `__vbaObjIs` (348), `__vbaObjVar` (351), `__vbaObjSetAddref` (350), `TypeName` object introspection, and `Scripting.Dictionary` instantiation<br>• Dual-interface `_ErrObject` COM vtable<br>• Reference lifecycle helpers (`__vbaObjSet`, `__vbaFreeObj`, `__vbaHresultCheckObj`)<br>• `GUID`, `IID_IVbaHost`, `BASIC_CLASS_*`, `Zombie_*` | • **Phase 2.2 (Introspection)**: MSFT TypeLib parser (`.tlb`/`.olb`) & DISPID cache<br>• **Phase 2.3 (Late-Binding)**: `IDispatch` (`GetIDsOfNames`, `Invoke`) & `__vbaLateId*` compiler helpers<br>• **Phase 2.4 (ActiveX Shims)**: Standard shims (`Scripting.Dictionary` methods, `Scripting.FileSystemObject`, `WScript.Shell`, `ADODB`)<br>• **Phase 2.5 (Consolidation)**: Self-describing `ITypeInfo` & test layers 13–15 |
| **3. Win32 GUI (Web-Streamed Canvas)** | **30%** | • `VBHeader`, `ProjectInfo`, and `ObjectTable` parsing<br>• Form count and startup object detection<br>• Loader mode separation (`Headless` vs `GuiWebStream`)<br>• `MessageBoxA` shim | • `.rsrc` Form and control deserialization (`ThunderRT6FormDC`, `CommandButton`, `TextBox`, etc.)<br>• Headless virtual Win32 message loop<br>• WebSocket server streaming GDI/framebuffer updates to an in-browser WebAssembly HTML5 `<canvas>` client |

### Milestone Details

1. **Milestone 1 — Core Language (Console, CLI & Backend) [100% Complete]**
   - **Iteration 1 (`App08-arrays`) [100% Complete]**: Multi-dimensional `SAFEARRAY` allocation, locking, bounds queries, `ReDim Preserve`, string tokenization (`Split`/`Join`/`Filter`), and `Variant` array constructors.
   - **Iteration 2 (`App09-classic-fileio`) [100% Complete]**: Runtime file table mapping VB6 `#filenumber` channels (`1..511`) to host file descriptors for sequential, random, and binary I/O (`FreeFile`, `Open`, `Close`, `Reset`, `Print #`, `Write #`, `Line Input #`, `Put #`, `Get #`, `EOF`, `LOF`, `Seek`, `Loc`, `FileCopy`, `FileLen`, `Name ... As ...`, `Dir$`).
   - **Iteration 3 (`App10-core-primitives`) [100% Complete]**: Explicit type conversion functions (`CBool`, `CInt`, `CLng`, `CSng`, `CDbl`, `CCur`), mathematical intrinsics (`Abs`, `Sgn`, `Sqr`, `Fix`, `Int`, `Round`), string utilities (`Asc`, `Chr$`, `Hex$`, `Oct$`, `LCase$`, `Replace`, `InStrRev`, `Str$`, `Val`, `Format$`, `Mid$` statement, `Byte()` coercion), flow control primitives (`IIf`, `Choose`, `End`), `VARIANT` type inspection (`TypeName`, `IsEmpty`, `IsNull`, `IsNumeric`, `IsArray`), and CLI argument retrieval (`Command$`).
2. **Milestone 2 — ActiveX & Component Object Model (COM) [Next — 20% Complete]**
   Enables in-process COM object instantiation, binary Type Library introspection, `IDispatch` late-binding, and legacy Win32 ActiveX component shims on Linux without Wine.

   #### Implementation Phases

   - **Phase 2.1 — Discovery & Instantiation Foundations**
     - **CLSID Registry & Class Factory Resolver**: Linux headless registry mapping `ProgID` → `CLSID` → `(DLL path, threading model)` (analogous to Windows `HKEY_CLASSES_ROOT\CLSID\{...}\InprocServer32`). Persisted in an application-sandboxed `registry.json` file where the application assumes native registry access while resolving locally, decoupled behind a provider interface for future centralized registry service pods.
     - **Concrete `DllGetClassObject` & Multi-DLL Loader**: Extends PE32 loader to map and cross-link multiple in-process COM DLLs/OCXs simultaneously and bind their real `DllGetClassObject` export.
     - **`IClassFactory::CreateInstance` Pipeline**: Implements `CLSID` → `DllGetClassObject` → `IClassFactory` → `CreateInstance` → `IUnknown` → `QueryInterface` chain.
   - **Phase 2.2 — Introspection (Type Library Parser)**
     - **MSFT `.tlb`/`.olb` Parser**: Binary reader for MSFT TypeLib blocks (`MSFT\x02\x00\x01\x00`), parsing `ITypeLib`, `ITypeInfo` (`TKIND_COCLASS`, `TKIND_INTERFACE`, `TKIND_DISPATCH`), `TYPEATTR`, `FUNCDESC` (DISPIDs, parameter signatures, `INVOKEKIND`), and `VARDESC`.
     - **DISPID Interface Cache**: Lookup index mapping `(IID, member_name) → DISPID` and `(IID, DISPID) → FUNCDESC` queried by late-binding engines.
   - **Phase 2.3 — Late-Binding Dispatch Engine**
     - **`IDispatch::GetIDsOfNames` Engine**: Maps member name arrays to DISPIDs using cached TypeLib descriptors with fallback to object `GetTypeInfo`.
     - **`IDispatch::Invoke` Engine**: Executes methods and properties handling `DISPATCH_METHOD`, `PROPERTYGET`, `PROPERTYPUT`, `PROPERTYPUTREF`, positional & named arguments (`DISPPARAMS`), Variant coercion, and `EXCEPINFO`.
     - **VB6 Compiler Helpers (`__vbaLateId*`)**: Implements runtime entry points (ordinals 282–297, 322–324, such as `__vbaLateIdCall`, `__vbaLateIdCallLd`, `__vbaLateIdSt`, `__vbaLateIdNamedCall`) translating compiled call sites into `GetIDsOfNames` + `Invoke`.
   - **Phase 2.4 — Instantiation APIs & Concrete ActiveX Shims**
     - **`CreateObject` / `GetObject`**: Implements `rtcCreateObject` (625) and `rtcCreateObject2` (716) dynamically instantiating objects by ProgID.
     - **Standard ActiveX Shims**:
       1. `Scripting.Dictionary`: Key-value associative store.
       2. `Scripting.FileSystemObject` (FSO): File/folder hierarchy mapping to Milestone 1 filesystem primitives.
       3. `WScript.Shell` & `WScript.Network`: Host execution, environment queries, and network identity.
       4. `ADODB.Connection` & `ADODB.Recordset`: Embedded SQL and tabular recordsets.
   - **Phase 2.5 — Consolidation & Verification**
     - **Self-Describing `ITypeInfo`**: Built-in TypeInfo generation for native Rust shims, ensuring late-binding works without external `.tlb` files.
     - **Layered Test Coverage (Layers 12–15)**: `layer12_com_instantiation`, `layer13_dispatch`, `layer14_typelib`, `layer15_activex_shims`.

   #### Component Dependency Graph

   | # | Component | Depends On | Blocks |
   | :-: | :--- | :--- | :--- |
   | 1 | CLSID Registry & Class Factory Resolver | Multi-DLL PE Loader | 2, 3, 9 |
   | 2 | Concrete `DllGetClassObject` | 1 | 3 |
   | 3 | `IClassFactory::CreateInstance` + `QueryInterface` | 2 | 7, 9 |
   | 4 | MSFT TypeLib Parser (`.tlb`/`.olb`) | — | 5, 6 |
   | 5 | TypeLib DISPID Cache Engine | 4 | 6, 7 |
   | 6 | `IDispatch::GetIDsOfNames` Engine | 5 | 7 |
   | 7 | `IDispatch::Invoke` Dispatch Engine | 3, 6 | 8, 10 |
   | 8 | `__vbaLateId*` Compiler Runtime Helpers | 7 | — |
   | 9 | `rtcCreateObject` / `rtcCreateObject2` | 1, 3 | 10 |
   | 10 | Standard ActiveX Shims (`Dictionary`, `FSO`, ...) | 7, 9 | 11, 12 |
   | 11 | Self-Describing `ITypeInfo` for Shims | 10 | — |
   | 12 | Layered Test Suites (`tests/layer12`..`layer15`) | 1–11 | — |

   #### Tactical Sprint Plan

   - **Sprint A (Discovery & Instantiation)**: Items 1–3 + `Scripting.Dictionary` instantiation test → validates discovery and instantiation pipeline.
   - **Sprint B (Introspection)**: Items 4–5 + real MSFT `.tlb` parser validation test → verifies binary contract and DISPID extraction.
   - **Sprint C (Late-Binding)**: Items 6–8 + compiled VB6 test binary executing `CreateObject("Scripting.Dictionary")` with `.Add` / `.Item` → validates end-to-end late-binding.
   - **Sprint D (Ecosystem Shims)**: Items 9–12 + `Scripting.FileSystemObject`, `WScript.Shell`, and `ADODB` shims → delivers real-world legacy application support.
3. **Milestone 3 — Win32 GUI via WebAssembly / WebSocket Canvas [30% Complete]**
   - Rather than relying on X11, Xvfb, or Wine on the server, GUI executables run headlessly on Linux. Form layouts are parsed from `.rsrc`, controls are managed in a virtual window tree, and drawing operations are streamed over WebSockets to an HTML5 `<canvas>` rendered via WebAssembly in the user's browser, with DOM mouse and keyboard events sent back to the runtime.

---

## Building and Running

### 1. Build and Execute with `vb6-run`

```bash
# Build release binary (targets x86_64-alpine-linux-musl by default)
cargo build --bin vb6-run --release

# Inspect PE sections and VBHeader metadata without running
./target/x86_64-alpine-linux-musl/release/vb6-run artifacts/App01-helloworld-pcode.exe --inspect

# Run in headless mode (default)
./target/x86_64-alpine-linux-musl/release/vb6-run artifacts/App10-core-primitives-native.exe --headless

# Run in web-streamed GUI mode
./target/x86_64-alpine-linux-musl/release/vb6-run artifacts/App10-core-primitives-native.exe --gui-web
```

CLI Options:
- `--inspect`: Parse and display PE32 sections and `VBHeader` metadata without executing.
- `--dump-info`: Dump `VBHeader`, `ProjectInfo`, and `ObjectTable` structures.
- `--run`: Execute the binary (default).
- `--headless`: Execute in headless mode for CLI and backend processes (default).
- `--gui-web`: Execute in Web-Streamed GUI mode (WebSocket / WebAssembly canvas bridge).

### 2. Granular Layered Test Suite

Tests are isolated in the `tests/` test project with one test function per runtime function/scenario, divided across architectural layers mirroring the compilation milestones:

```bash
# Run all 158 tests (11 layered unit test suites + retroactive regression suite)
cargo test

# Run a specific layer's test suite
cargo test --test layer01_lifecycle
cargo test --test layer02_strings
cargo test --test layer08_dynamic_arrays
cargo test --test layer09_classic_fileio
cargo test --test layer11_core_primitives
cargo test --test retroactive_binaries
```

| Layer Test Suite | Description | Scope |
| :--- | :--- | :---: |
| `tests/layer01_lifecycle.rs` | App01 Application Lifecycle | `VarPtr`, `GetMem4`, `PutMem4`, `VBHeader`, `DllMain`, `__vbaEnd` (channel flushing & clean termination) |
| `tests/layer02_strings.rs` | App02/03 Strings & BSTR | `SysAllocString`, `SysFreeString`, `__vbaStrCat`, `__vbaLenBstr`, `Mid$`, `Left$`, `Right$`, `InStr`, `UCase$`, `StrReverse`, `Trim$`, `Space$`, `__vbaStrCopy`, `__vbaStrMove`, `__vbaMidStmtBstr` (in-place mutation) |
| `tests/layer03_math_types.rs` | App03 Math & Numeric Types | `Currency` (fixed-point), `RGB`, `__vbaFPInt` (floor), `__vbaFpR4`, `__vbaStrI2`, `__vbaStrI4`, `__vbaStrR4` |
| `tests/layer04_variants.rs` | App04 Variant Engine | Tagged unions, arithmetic coercion (`+`, `-`, `*`, `/`, `\`, `Mod`), string concatenation (`&`), `Not`, `And`, relational comparisons, `VarType`, `__vbaBoolVarNull` |
| `tests/layer05_win32_shims.rs` | App05 Win32 KERNEL32 Shims | `CreateFileA`, `WriteFile`, `ReadFile`, `GetFileSize`, `CloseHandle`, `GetLastError`, `GetStdHandle` |
| `tests/layer06_err_handling.rs` | App06 Error Handling | `__vbaOnError`, `ErrObject` COM VTable (`QueryInterface`, `AddRef`, `Release`, `get_Number`, `put_Number`, `Raise`, `Clear`), `__vbaObjSet`, `__vbaFreeObj`, `__vbaResume` |
| `tests/layer07_env_datetime_fs.rs` | App07 Environment & Filesystem | `Environ$`, `CurDir$`, `Date$`, `Time$`, `Timer`, `Now` (`rtcGetPresentDate`, `__vbaDateVar`), `MkDir`, `RmDir` |
| `tests/layer08_dynamic_arrays.rs` | App08 Dynamic Arrays (Iteration 1) | `SafeArray`, `ReDim`, `ReDim Preserve`, `LBound`, `UBound`, `Split`, `Join`, `Filter`, `Array()`, `__vbaVarIndexLoad`, `__vbaErase`, `__vbaUI1ArrayStr`, `__vbaStrUI1Array` (`Byte()` coercion) |
| `tests/layer09_classic_fileio.rs` | App09 Classic File I/O (Iteration 2) | Channel table (`#1..#511`), `FreeFile`, `Open` (Output/Append/Input/Binary), `Print #`, `Line Input #`, `Put #`, `Get #`, `Seek`, `LOF`, `EOF`, `FileCopy`, `Name..As`, `Dir$`, `Reset`, `Kill` |
| `tests/layer10_pe_loader.rs` | PE Loader Engine | DOS header validation, synthetic VB6 binary loading, section RVA mapping, `VB5!` header detection |
| `tests/layer11_core_primitives.rs` | App10 Core Primitives (Iteration 3) | `TypeName`, `IsEmpty`, `IsNull`, `IsArray`, `IsNumeric`, `IIf`, `Choose`, `Command$`, `Asc`, `Chr$`, `Hex$`, `Oct$`, `Val`, `Str$`, `Format$`, `LCase$`, `Replace`, `InStrRev`, `Sqr`, `Round`, `Int`, `Fix`, `Abs`, `Sgn`, `CBool`, `CInt`, `CLng`, `CCur`, `CSng`, `CDbl`, `End` |
| `tests/layer12_com_instantiation.rs` | App11 COM Instantiation & Factory (Phase 2.1) | Sandboxed `registry.json`, `DllGetClassObject`, `IClassFactory::CreateInstance`, `rtcCreateObject2`, `rtcCreateObject`, error 429, `rtcIsObject`, `__vbaObjIs`, `__vbaObjVar`, `__vbaObjSetAddref`, `TypeName` |
| `tests/retroactive_binaries.rs` | Retroactive Binary Verification | Executes all 19 compiled PE32 binaries (`artifacts/*.exe`, Native & P-Code) in process isolation via `vb6-run` |

### 3. Container Build (Podman / Docker)

```bash
podman build -t classicbasic:latest .
podman run --rm classicbasic:latest
```

### 4. Documentation

Pre-built Rustdoc HTML documentation is available in `docs/index.html`. To regenerate:

```bash
cargo doc --no-deps --bins
```

---

## Repository Structure

```text
├── Cargo.toml               # Crate configuration (cdylib, rlib, and vb6-run binary)
├── msvbvm60.def             # 635 exact export ordinals (100 to 2010)
├── Dockerfile               # Multi-stage Alpine Linux container build
├── docs/                    # Generated rustdoc HTML documentation
├── artifacts/               # Compiled VB6 Native and P-Code test executables (App01..App11)
├── tests/                   # Modular layered integration test project (166 granular tests)
│   ├── layer01_lifecycle.rs
│   ├── layer02_strings.rs
│   ├── layer03_math_types.rs
│   ├── layer04_variants.rs
│   ├── layer05_win32_shims.rs
│   ├── layer06_err_handling.rs
│   ├── layer07_env_datetime_fs.rs
│   ├── layer08_dynamic_arrays.rs
│   ├── layer09_classic_fileio.rs
│   ├── layer10_pe_loader.rs
│   ├── layer11_core_primitives.rs
│   ├── layer12_com_instantiation.rs
│   └── retroactive_binaries.rs
└── src/
    ├── lib.rs               # Library root, DllMain, and export re-exports
    ├── thun_rt.rs           # ThunRTMain, ExecutionMode, and P-Code / Native dispatchers
    ├── err.rs               # Structured error handling and COM ErrObject
    ├── mem.rs               # VarPtr, GetMem1..8, PutMem1..8, SetMemVar
    ├── com.rs               # DllFunctionCall dynamic resolver, COM interfaces, and ClassFactory
    ├── registry/            # Sandboxed Windows Registry emulation (registry.json)
    ├── math.rs              # x87 FPU routines (_CI*) and 64-bit integer math
    ├── rtc/mod.rs           # VBA standard library functions (rtc*)
    ├── vba/mod.rs           # Compiler runtime helpers (__vba*)
    ├── stubs.rs             # Mapped stubs for unimplemented ordinal exports
    ├── types/               # VBHeader, VARIANT, BSTR, SAFEARRAY, Currency, GUID
    ├── pe/                  # PE32 loader, relocations, symbol resolver, Win32 shims
    └── bin/vb6_run.rs       # Standalone CLI runner and inspector
```

---

## Authorship

- **Architecture, Domain Guidance & Concept**: Victoria M. ([@victoria-devs](https://github.com/victoria-devs))
- **AI Pair Programming & Implementation**: Google Antigravity ([@antigravity-bot](https://github.com/antigravity-bot))

## License & Legal Notice

Distributed under the MIT License. See [LICENSE](LICENSE) for details.

> [!NOTE]
> **Trademark & Interoperability Notice**:
> Microsoft®, Windows®, Visual Basic®, Visual Studio®, MSVB, and MSVBVM60 are registered trademarks of Microsoft Corporation. ClassicBasic is an independent clean-room re-implementation developed for educational study and software interoperability under Sections 29 and 30.61 of the *Copyright Act of Canada* (R.S.C., 1985, c. C-42). No proprietary Microsoft binaries are distributed in this repository.
