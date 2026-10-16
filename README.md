# ClassicBasic (`msvbvm60`)

Clean-room Rust re-implementation of the Microsoft Visual Basic 6.0 runtime (`msvbvm60.dll`) and standalone PE32 loader (`vb6-run`) for executing compiled 32-bit Visual Basic 6.0 binaries (Native Code and P-Code) directly on Linux (`x86_64-alpine-linux-musl`) **without Wine**.

[![Runtime Version](https://img.shields.io/badge/version-v0.2.7--alpha-blue.svg)](ROADMAP.md)
[![Tests](https://img.shields.io/badge/tests-209%20passed-brightgreen.svg)](tests/)
[![Target](https://img.shields.io/badge/target-x86__64--alpine--linux--musl-blue.svg)](Dockerfile)
[![Symbol Exports](https://img.shields.io/badge/symbol%20exports-635%20%2F%20635%20(100%25)-success.svg)](msvbvm60.def)
[![Overall Completeness](https://img.shields.io/badge/overall%20completeness-~48%25-orange.svg)](ROADMAP.md)
[![Roadmap](https://img.shields.io/badge/roadmap-Phase%202.5B%20in%20progress-purple.svg)](ROADMAP.md)

---

## Project Status & Completion Forecast

| Metric | Current Status | Notes |
| :--- | :---: | :--- |
| **Current Runtime Version** | **`v0.2.7-alpha`** | Updated per iteration; current milestone: ActiveX & COM hardening. |
| **Active Iteration** | **Phase 2.5B — Suite 2** | Esoteric Quirks, Zombie Objects & Memory Aliasing ([`layer17`](tests/layer17_esoteric_zombie_aliasing.rs)). |
| **Overall Completeness** | **~48%** | • **Milestone 1 (Core CLI)**: ~65% (Foundational Iterations: 100%)<br>• **Milestone 2 (ActiveX/COM ABI)**: ~45% (Phases 2.1–2.5A: 100%, 2.5B: 40%)<br>• **Milestone 3 (Win32 GUI)**: 30% (Windowing tree & WebSocket mode) |
| **Current Delivery Pace** | **1–2 Phases / Week** | Continuous pair-programming iterations backed by containerized CI. |
| **Milestone 2 Completion Target** | **Q1 2027 (`v0.7.0`)** | Full 32-bit execution, P-Code interpreter loop, and all 635 exports. |
| **General Availability (v1.0.0)** | **Q2 2027 (`v1.0.0`)** | Interactive Web-Streamed GUI canvas engine (Milestone 3). |

> [!TIP]
> **Detailed Roadmap & Subsystem Breakdown**:
> For the complete phased execution plan, SemVer version forecast, component dependency graph, and granular ABI compatibility matrix, see **[`ROADMAP.md`](ROADMAP.md)**.

---

## Overview

ClassicBasic is architected in two clean-room components:
1. **`msvbvm60` (Runtime Library)**: Implements the Visual Basic 6.0 virtual machine ABI (all 635 export ordinals defined in `msvbvm60.def`), including data types, memory management, intrinsic library functions (`rtc*`), and compiler helpers (`__vba*`).
2. **`vb6-run` (Standalone PE32 Loader)**: Maps 32-bit Windows PE executables into virtual memory, applies base relocations (`.reloc`), resolves Import Address Table (IAT) entries against `msvbvm60` and Win32 API shims (`KERNEL32`, `USER32`, `OLEAUT32`, `ADVAPI32`), locates the `VB5!` header, and invokes `ThunRTMain`.

### Core Architectural Subsystems

- **Lifecycle & Execution Entry (`src/thun_rt.rs`)**: Implements `ThunRTMain` (Ordinal 100), parses `VBHeader`, `ProjectInfo`, and `ObjectTable`, and dispatches execution to `Sub Main` or Form initialization. Supports `--headless` (default for CLI/microservices) and `--gui-web` (WebAssembly canvas bridge).
- **Automation Data Types (`src/types/`)**:
  - `BSTR`: Length-prefixed UTF-16 COM strings (`SysAllocStringLen`, `SysFreeString`), in-place mutation (`Mid$` statement), and byte array conversions (`Byte()` $\leftrightarrow$ `String`).
  - `VARIANT`: 16-byte tagged unions supporting dynamic type coercion, arithmetic, unary operations, ByRef pointer chaining, and deep dereferencing (`to_byval`).
  - `SAFEARRAY`: Multi-dimensional bounded array descriptors with dynamic bounds, in-place resizing (`ReDim Preserve`), and lock violation tracking (`c_locks > 0` $\to$ Runtime Error 10).
  - `Currency`: 64-bit fixed-point integer scaled by 10,000.
- **Error Handling & SEH (`src/err.rs`)**: Structured error state machine (`__vbaOnError`, `__vbaResume`, `__vbaExitProc`), thread-local SEH frame stack unwinding (`unwind_to_handler`), and COM `ErrObject` (`Number`, `Description`, `Source`, `Clear`, `Raise`, `LastDllError`).
- **Memory Primitives (`src/mem.rs`)**: Low-level memory manipulation (`VarPtr`, `GetMem1..8`, `PutMem1..8`, `SetMemVar`, `SetMemObj`).
- **Virtualized Windows Registry (`src/registry/`)**: Sandboxed registry emulation persisting keys in `registry.json`. Transparently shims Win32 `ADVAPI32` APIs (`RegOpenKeyExA`, `RegQueryValueExA`, `RegCloseKey`) and maps `ProgID` $\to$ `CLSID` $\to$ `InprocServer32`.
- **COM & Class Factory (`src/com.rs`)**: In-process COM lifecycle (`DllGetClassObject`, `IClassFactory::CreateInstance`, `IUnknown`), reference counting (`__vbaObjSetAddref`, `__vbaFreeObj`), `IDictionary` dual-interface vtable, `IDispatch` late-binding dispatch engine, zombie object lockout (`E_UNEXPECTED = 0x8000FFFF`), and circular reference cleanup.
- **TypeLib Introspection & DISPID Cache (`src/typelib/`)**: Clean-room MSFT binary parser (`MSFT\x02\x00\x01\x00`) extracting `ITypeLib` and `ITypeInfo` from PE `.rsrc` sections or standalone `.tlb` files, powering early-bound (`__vbaNew`, `__vbaCastObj`) and late-bound dispatch invocations.
- **ActiveX Component Shims (`src/activex/`)**: Concrete implementations of foundational legacy components: `Scripting.FileSystemObject` & `TextStream` (`scrrun.dll`), `WScript.Shell` & `WScript.Network` (`wshom.ocx`), and `ADODB.Recordset` (`msado15.dll`).

---

## Quickstart & CLI Usage

### 1. Build and Run Standalone Binaries

```bash
# Build release runner binary (targets x86_64-alpine-linux-musl by default)
cargo build --bin vb6-run --release

# Inspect PE sections and VBHeader metadata without executing
./target/x86_64-alpine-linux-musl/release/vb6-run artifacts/App01-helloworld-pcode.exe --inspect

# Run in headless mode (default)
./target/x86_64-alpine-linux-musl/release/vb6-run artifacts/App17-esoteric-quirks-native.exe --headless

# Run in web-streamed GUI mode
./target/x86_64-alpine-linux-musl/release/vb6-run artifacts/App10-core-primitives-native.exe --gui-web
```

### 2. Command-Line Options

| Option | Description |
| :--- | :--- |
| `--inspect` | Parse and display PE32 sections and `VBHeader` metadata without executing. |
| `--dump-info` | Dump `VBHeader`, `ProjectInfo`, and `ObjectTable` structures. |
| `--run` | Execute the binary (default action). |
| `--headless` | Execute in headless mode for CLI, backend, and containerized microservices (default). |
| `--gui-web` | Execute in Web-Streamed GUI mode (WebSocket / WebAssembly canvas bridge). |

---

## Verification & Testing

ClassicBasic features a 100% automated test suite divided into granular layer suites mirroring compilation milestones:

```bash
# Run all 209 tests (18 layered unit test suites + retroactive regression suite)
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

# Run retroactive binary regression tests across all 31 compiled PE32 executables
cargo test --test retroactive_binaries
```

### Layer Test Suite Directory

| Layer Test Suite | Scope & Verifications |
| :--- | :--- |
| [`tests/layer01_lifecycle.rs`](tests/layer01_lifecycle.rs) | `VarPtr`, `GetMem4`, `PutMem4`, `VBHeader`, `DllMain`, `__vbaEnd` clean channel flushing. |
| [`tests/layer02_strings.rs`](tests/layer02_strings.rs) | `SysAllocString`, `SysFreeString`, `__vbaStrCat`, `__vbaLenBstr`, `Mid$`, `Left$`, `Right$`, `InStr`, `UCase$`, `StrReverse`, `Trim$`, `Space$`, `__vbaStrCopy`, `__vbaStrMove`, `__vbaMidStmtBstr`. |
| [`tests/layer03_math_types.rs`](tests/layer03_math_types.rs) | `Currency` fixed-point, `RGB`, `__vbaFPInt`, `__vbaFpR4`, `__vbaStrI2`, `__vbaStrI4`, `__vbaStrR4`. |
| [`tests/layer04_variants.rs`](tests/layer04_variants.rs) | Tagged unions, arithmetic coercion (`+`, `-`, `*`, `/`, `\`, `Mod`), string concatenation (`&`), `Not`, `And`, relational comparisons, `VarType`, `__vbaBoolVarNull`. |
| [`tests/layer05_win32_shims.rs`](tests/layer05_win32_shims.rs) | `CreateFileA`, `WriteFile`, `ReadFile`, `GetFileSize`, `CloseHandle`, `GetLastError`, `GetStdHandle`. |
| [`tests/layer06_err_handling.rs`](tests/layer06_err_handling.rs) | `__vbaOnError`, `ErrObject` COM VTable (`QueryInterface`, `AddRef`, `Release`, `get_Number`, `put_Number`, `Raise`, `Clear`), `__vbaObjSet`, `__vbaFreeObj`, `__vbaResume`. |
| [`tests/layer07_env_datetime_fs.rs`](tests/layer07_env_datetime_fs.rs) | `Environ$`, `CurDir$`, `Date$`, `Time$`, `Timer`, `Now` (`rtcGetPresentDate`, `__vbaDateVar`), `MkDir`, `RmDir`. |
| [`tests/layer08_dynamic_arrays.rs`](tests/layer08_dynamic_arrays.rs) | `SafeArray`, `ReDim`, `ReDim Preserve`, `LBound`, `UBound`, `Split`, `Join`, `Filter`, `Array()`, `__vbaVarIndexLoad`, `__vbaErase`, `__vbaUI1ArrayStr`, `__vbaStrUI1Array` (`Byte()` coercion). |
| [`tests/layer09_classic_fileio.rs`](tests/layer09_classic_fileio.rs) | Virtual file channels (`#1..#511`), `FreeFile`, `Open`, `Print #`, `Line Input #`, `Put #`, `Get #`, `Seek`, `LOF`, `EOF`, `FileCopy`, `Name..As`, `Dir$`, `Reset`, `Kill`. |
| [`tests/layer10_pe_loader.rs`](tests/layer10_pe_loader.rs) | DOS header validation, synthetic VB6 binary loading, section RVA mapping, `VB5!` header detection. |
| [`tests/layer11_core_primitives.rs`](tests/layer11_core_primitives.rs) | `TypeName`, `IsEmpty`, `IsNull`, `IsArray`, `IsNumeric`, `IIf`, `Choose`, `Command$`, `Asc`, `Chr$`, `Hex$`, `Oct$`, `Val`, `Str$`, `Format$`, `LCase$`, `Replace`, `InStrRev`, `Sqr`, `Round`, `Int`, `Fix`, `Abs`, `Sgn`, conversions (`CBool`, `CInt`, `CLng`, `CCur`, `CSng`, `CDbl`), `End`. |
| [`tests/layer12_com_instantiation.rs`](tests/layer12_com_instantiation.rs) | `registry.json`, `DllGetClassObject`, `IClassFactory`, `rtcCreateObject2`, `rtcCreateObject`, error 429, `rtcIsObject`, `__vbaObjIs`, `__vbaObjVar`, `__vbaObjSetAddref`, `TypeName`. |
| [`tests/layer13_typelib_parser.rs`](tests/layer13_typelib_parser.rs) | MSFT binary parser (`MSFT\x02\x00\x01\x00`), `ITypeLib`/`ITypeInfo` from `scrrun.dll`, `DispidCache`, `__vbaNew` (341), `__vbaCastObj` (178), `__vbaStrVarMove` (437), `IDictionary` dual-interface vtable. |
| [`tests/layer14_dispatch_engine.rs`](tests/layer14_dispatch_engine.rs) | `IDispatch::GetIDsOfNames`, `IDispatch::Invoke`, `__vbaLateMemCall*`, `__vbaLateIdCall*`, `__vbaVarLateMem*`, `__vbaUnkVar`, `__vbaVarSetObjAddref`, error trapping (438, 448, 450). |
| [`tests/layer15_activex_shims.rs`](tests/layer15_activex_shims.rs) | `Scripting.FileSystemObject` & `TextStream`, `WScript.Shell`, `WScript.Network`, `ADODB.Recordset` (`Fields`, `Field`, `AddNew`, `Update`), `rtcGetObject` (626). |
| [`tests/layer16_consolidation_typeinfo.rs`](tests/layer16_consolidation_typeinfo.rs) | COM `ITypeInfo` vtable on native shims, dynamic member invocation via `rtcCallByName` (715), `TypeName` reflection, error 438 trapping. |
| [`tests/layer16_threading_apartments.rs`](tests/layer16_threading_apartments.rs) | STA apartments (TLS isolation, RPC rejection), worker thread context lifecycle, SafeArray lock violations, ByRef aliasing, BSTR embedded nulls, Banker's rounding, FDIV workarounds, ROT & IPC dispatch proxy. |
| [`tests/layer17_esoteric_zombie_aliasing.rs`](tests/layer17_esoteric_zombie_aliasing.rs) | Zombie object invocation lockout (`E_UNEXPECTED`), SafeArray lock violations during iteration (Error 10), ByRef parameter memory aliasing, deep ByRef variant dereferencing, re-entrant SEH frame unwinding, circular reference cleanup. |
| [`tests/retroactive_binaries.rs`](tests/retroactive_binaries.rs) | Executes all 31 compiled PE32 binaries (`artifacts/*.exe`, Native & P-Code, App01–App17) in process isolation via `vb6-run`. |

---

## Container Build (Podman / Docker)

ClassicBasic is built container-first on Alpine Linux with static musl compilation:

```bash
# Build production container image
podman build -t classicbasic:latest .

# Run default test binary inside container
podman run --rm classicbasic:latest

# Run custom binary headlessly
podman run --rm classicbasic:latest /app/artifacts/App17-esoteric-quirks-native.exe
```

---

## Documentation

Rustdoc HTML documentation is generated via:

```bash
cargo doc --no-deps --bins
```

Generated documentation will be located at `target/x86_64-alpine-linux-musl/doc/msvbvm60/index.html`.

---

## Repository Structure

```text
├── Cargo.toml               # Crate configuration (cdylib, rlib, and vb6-run binary)
├── msvbvm60.def             # 635 exact export ordinals (100 to 2010)
├── Dockerfile               # Multi-stage Alpine Linux container build
├── ROADMAP.md               # Phased roadmap, SemVer versioning forecast & ABI matrix
├── docs/                    # Generated rustdoc HTML documentation
├── artifacts/               # Compiled VB6 Native and P-Code test executables (App01..App17)
├── tests/                   # Modular layered integration test project (209 granular tests)
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
│   ├── layer13_typelib_parser.rs
│   ├── layer14_dispatch_engine.rs
│   ├── layer15_activex_shims.rs
│   ├── layer16_consolidation_typeinfo.rs
│   ├── layer16_threading_apartments.rs
│   ├── layer17_esoteric_zombie_aliasing.rs
│   └── retroactive_binaries.rs
└── src/
    ├── lib.rs               # Library root, DllMain, and export re-exports
    ├── thun_rt.rs           # ThunRTMain, ExecutionMode, and P-Code / Native dispatchers
    ├── err.rs               # Structured error handling, SEH frames, and COM ErrObject
    ├── mem.rs               # VarPtr, GetMem1..8, PutMem1..8, SetMemVar, SetMemObj
    ├── com.rs               # DllFunctionCall dynamic resolver, COM interfaces, and ClassFactory
    ├── threading.rs         # Multithreading, worker contexts, and STA concurrency
    ├── activex/             # Standard ActiveX component shims (FSO, WScript, ADODB)
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
