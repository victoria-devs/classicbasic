# ClassicBasic (`msvbvm60`)

Clean-room Rust re-implementation of the Microsoft Visual Basic 6.0 runtime (`msvbvm60.dll`) and standalone PE32 loader (`vb6-run`) for executing compiled 32-bit Visual Basic 6.0 binaries (Native Code and P-Code) directly on Linux (`x86_64-alpine-linux-musl`) **without Wine**.

[![Runtime Version](https://img.shields.io/badge/version-v0.2.10--alpha-blue.svg)](ROADMAP.md)
[![Tests](https://img.shields.io/badge/tests-235%20passed-brightgreen.svg)](tests/)
[![Target](https://img.shields.io/badge/target-x86__64--alpine--linux--musl-blue.svg)](Dockerfile)
[![Overall Completeness](https://img.shields.io/badge/overall%20completeness-~55%25-orange.svg)](ROADMAP.md)
[![Roadmap](https://img.shields.io/badge/roadmap-Phase%201.5%20complete%20%7C%20Phase%201.6%20next-purple.svg)](ROADMAP.md)

---

## Project Status & Completion Forecast

| Metric | Current Status | Notes |
| :--- | :---: | :--- |
| **Current Runtime Version** | **`v0.2.10-alpha`** | SemVer tracking per phase; active runtime hardening. |
| **Active Phase** | **Phase 1.5 (Loops & Iterators)** | Phase 1.5 (Loops, Iterators & Flow Control) completed; Phase 1.6 (UDTs & Fixed Strings) next. |
| **Overall Completeness** | **~55%** | Milestone 1: ~71% (Phases 1.1–1.5 Done); Milestone 2: ~50% (Phases 2.1–2.9 Done). |
| **Current Delivery Pace** | **1–2 Phases / Sprint** | Continuous pair-programming iterations backed by containerized CI. |
| **Milestone 1 & 2 Target (`v0.7.0`)** | **Q1 2027** | Full 32-bit execution, P-Code interpreter loop, and all 635 exports. |
| **General Availability (`v1.0.0`)** | **Q2 2027** | Interactive Web-Streamed GUI canvas engine (Milestone 3). |

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

ClassicBasic features a 100% automated test suite divided into granular layer suites mirroring compilation milestones:

```bash
# Run all 227 tests (20 layered unit test suites + retroactive regression suite)
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

# Run retroactive binary regression tests across all 35 compiled PE32 executables
cargo test --test retroactive_binaries
```

### Automated Test Suite Catalog

ClassicBasic maintains an exhaustive 20-layer test suite with 227 automated unit/integration tests and retroactive regression verification across 35 compiled PE32 executables.

For the exhaustive catalog of test files, tested runtime functions, and binary regression matrices, refer to **[`tests/README.md`](tests/README.md)**.

---

## Container Build (Podman / Docker)

ClassicBasic is built container-first on Alpine Linux with static musl compilation:

```bash
# Build production container image
podman build -t classicbasic:latest .

# Run default test binary inside container
podman run --rm classicbasic:latest

# Run custom binary headlessly
podman run --rm classicbasic:latest /app/artifacts/bin/App19-math-rounding-native.exe
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
│   ├── bin/                 # Compiled VB6 PE32 test fixtures (App01..App19)
│   ├── src/                 # Original Visual Basic 6 sources (.bas, .vbp, .cls)
│   └── ref/                 # Reference symbol export dumps (msvbvm60.txt)
├── tests/                   # Modular layered integration test project (227 granular tests)
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
│   ├── layer18_complex_strings.rs
│   ├── layer19_complex_math.rs
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
