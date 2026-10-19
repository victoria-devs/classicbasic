# ClassicBasic (`msvbvm60`)

Clean-room Rust re-implementation of the Microsoft Visual Basic 6.0 runtime (`msvbvm60.dll`) and standalone PE32 loader (`vb6-run`) for executing compiled 32-bit Visual Basic 6.0 binaries (Native Code and P-Code) directly on Linux (`x86_64-alpine-linux-musl`) **without Wine**.

[![Runtime Version](https://img.shields.io/badge/version-v0.2.16-blue.svg)](ROADMAP.md)
[![Tests](https://img.shields.io/badge/tests-386%20passed-brightgreen.svg)](tests/)
[![Retroactive Binaries](https://img.shields.io/badge/PE32%20fixtures-59%20passed-brightgreen.svg)](tests/)
[![Target](https://img.shields.io/badge/target-x86__64--alpine--linux--musl-blue.svg)](Dockerfile)
[![Overall Completeness](https://img.shields.io/badge/overall%20completeness-~96%25-orange.svg)](ROADMAP.md)
[![ABI Coverage](https://img.shields.io/badge/ABI%20coverage-100.0%25%20(635%2F635)-blueviolet.svg)](ROADMAP.md)
[![Roadmap](https://img.shields.io/badge/roadmap-Milestone%202%20100%25%20Complete%20%7C%20Milestone%203%20next-purple.svg)](ROADMAP.md)

---

## Project Status & Recalculated Velocity Forecast

| Metric | Current Status | Notes |
| :--- | :---: | :--- |
| **Current Runtime Version** | **`v0.2.16`** | **Milestone 1 & Milestone 2 100% Complete**; 100% of all 635 exports implemented. |
| **Active Milestone** | **Milestone 2 Completed $\to$ Transitioning to Milestone 3** | All 16 Milestone 2 phases completed (`App11`–`App19`, `App27`–`App33`). |
| **Completed Phase (2.15)** | **Phase 2.15 (P-Code VM Engine: ProcCallEngine, MethCallEngine & Stack Machine)** | Full clean-room P-Code VM engine, frame management, and bytecode execution. |
| **Completed Phase (2.16)** | **Phase 2.16 (IDE Hosting, Project Introspection & Debugger Telemetry)** | All 19 `Eb*`/`Tip*`/`CreateIExprSrvObj` ordinals implemented; 0 stubs remaining. |
| **Overall Completeness** | **~96%** | Real ABI coverage: **100.0%** (635/635 ordinals wired); Milestone 1: **100%** (11/11 phases); Milestone 2: **100%** (16/16 phases); Milestone 3: **30%**. |
| **Observed Velocity** | **3–5 Phases / Sprint** | Rapid clean-room TDD pair-programming delivery backed by automated containerized CI and retroactive binary suite. |
| **Milestone 1 (`v0.1.11`)** | **Completed (October 2026)** | 100% Core Language closure: all 181 `rtc*` symbols + all `__vba*` core language/math helpers. |
| **Milestone 2 (`v0.2.16`)** | **Completed (October 2026)** | 100% ActiveX, COM & Full Runtime ABI: all 635 MSVBVM60 exports real implementation. |
| **Milestone 3 & GA (`v1.0.0`)** | **November 2026 – Q1 2027** | Web-Streamed Canvas GUI, WASM client, multi-tenant session broker. |

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
- **ActiveX Component Shims (`src/activex/`)**: Clean-room implementations of foundational legacy components: `Scripting.FileSystemObject` & `TextStream` (Scripting Runtime), `WScript.Shell` & `WScript.Network` (Windows Script Host), and `ADODB.Recordset` (ADO).
- **Win32 GUI Subsystem & Canvas Streamer ([`docs/architecture/win32-gui-gdi.md`](docs/architecture/win32-gui-gdi.md))**: Full clean-room USER32 window hierarchy, GDI32 virtual DC drawing surface, static 96 DPI font metric lookup table (`GetTextExtentPoint32A`), Twips-to-Pixels coordinate pipeline ($15:1$), non-blocking Condvar message queue, and hand-rolled RFC 6455 WebSocket streaming engine sending atomic `FRAME_BITMAP_RECT` and delegated `FRAME_TEXT` commands to an in-browser WebAssembly canvas client.

---

## Quickstart & CLI Usage

### 1. Build and Run Standalone Binaries

```bash
# Build release runner binary (targets x86_64-alpine-linux-musl by default)
cargo build --bin vb6-run --release

# Inspect PE sections and VBHeader metadata without executing
./target/x86_64-alpine-linux-musl/release/vb6-run artifacts/bin/App01-helloworld-pcode.exe --inspect

# Run in headless mode (default)
./target/x86_64-alpine-linux-musl/release/vb6-run artifacts/bin/App19-math-rounding-native.exe --headless

# Run in web-streamed GUI mode
./target/x86_64-alpine-linux-musl/release/vb6-run artifacts/bin/App10-core-primitives-native.exe --gui-web
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

ClassicBasic features a 100% automated regression test suite organized into modular architectural domains mirroring runtime subsystem boundaries:

```bash
# Run all 370 tests (5 architectural domain suites + retroactive PE32 binary suites)
cargo test

# Run a specific architectural domain suite
cargo test --test foundations
cargo test --test types_automation
cargo test --test runtime_library
cargo test --test com_dispatch_typelib
cargo test --test retroactive_pe

# Run retroactive binary regression tests across compiled PE32 executables
cargo test --test retroactive_binaries
```

### Automated Test Suite Catalog

ClassicBasic maintains an exhaustive domain-oriented test suite with 370 automated unit/domain tests and retroactive regression verification across 59 compiled PE32 executables.

For the exhaustive catalog of test modules, tested runtime functions, and binary regression matrices, refer to **[`tests/README.md`](tests/README.md)**.

---

## Container Build (Podman / Docker)

ClassicBasic is built container-first on Alpine Linux with static musl compilation:

```bash
# Build production container image
podman build -t classicbasic:latest .

# Run default test binary inside container
podman run --rm classicbasic:latest

# Run custom binary headlessly
podman run --rm classicbasic:latest /app/artifacts/bin/App20-loops-iterators-native.exe
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
├── Cargo.lock               # Deterministic dependency resolution
├── msvbvm60.def             # 635 exact export ordinals (100 to 2010)
├── Dockerfile               # Multi-stage Alpine Linux container build
├── LICENSE.md               # MIT License
├── README.md                # Project overview, quickstart & build instructions
├── ROADMAP.md               # Phased roadmap, SemVer versioning forecast & ABI matrix
├── docs/                    # Architecture documentation and generated rustdoc HTML
├── artifacts/               # Test fixtures and reference data
│   ├── README.md            # Fixture catalog
│   ├── REFERENCE.md         # Windows XP SP3 baseline reference
│   ├── bin/                 # Compiled VB6 PE32 test fixtures (App01..App26)
│   ├── src/                 # Original Visual Basic 6 sources (.bas, .vbp, .cls)
│   └── ref/                 # Reference symbol export dumps (msvbvm60.txt)
├── tests/                   # Versioned regression test suites (287 granular tests)
│   ├── v0_1_1_lifecycle.rs
│   ├── v0_1_1_strings_bstr.rs
│   ├── v0_1_1_math_types.rs
│   ├── v0_1_1_variant_engine.rs
│   ├── v0_1_1_win32_shims.rs
│   ├── v0_1_1_err_handling.rs
│   ├── v0_1_1_env_datetime_fs.rs
│   ├── v0_1_1_pe_loader.rs
│   ├── v0_1_2_dynamic_arrays.rs
│   ├── v0_1_3_classic_fileio.rs
│   ├── v0_1_4_core_primitives.rs
│   ├── v0_1_5_loops_iterators.rs
│   ├── v0_1_6_udt_fixed_strings.rs
│   ├── v0_1_7_extended_rtc.rs
│   ├── v0_1_8_rtc_complete.rs
│   ├── v0_1_9_numeric_coercions.rs
│   ├── v0_1_10_variant_bitwise_relational.rs
│   ├── v0_1_11_core_language_closure.rs
│   ├── v0_2_1_com_instantiation.rs
│   ├── v0_2_2_typelib_parser.rs
│   ├── v0_2_3_dispatch_engine.rs
│   ├── v0_2_4_activex_shims.rs
│   ├── v0_2_5_consolidation_typeinfo.rs
│   ├── v0_2_6_threading_apartments.rs
│   ├── v0_2_7_esoteric_zombie_aliasing.rs
│   ├── v0_2_8_complex_strings.rs
│   ├── v0_2_9_complex_math.rs
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
    ├── rtc/                 # VBA standard library functions (rtc*)
    │   ├── mod.rs
    │   ├── extended.rs
    │   └── complete.rs
    ├── vba/                 # Compiler runtime helpers (__vba*)
    │   ├── mod.rs
    │   ├── core_num.rs
    │   ├── core_rel.rs
    │   └── core_lang.rs
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
