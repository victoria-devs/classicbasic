# ClassicBasic

Clean-room Rust re-implementation of the Microsoft Visual Basic 6.0 runtime (`msvbvm60.dll`), x86/PE32/Win32 execution foundations, and standalone runner (`vb6-run`) for executing compiled 32-bit Visual Basic 6.0 binaries (Native Code and P-Code) directly on Linux (`x86_64-alpine-linux-musl`) **without Wine**.

[![Runtime Version](https://img.shields.io/badge/version-v0.2.17-blue.svg)](ROADMAP.md)
[![Tests](https://img.shields.io/badge/tests-332%20passed-brightgreen.svg)](tests/README.md)
[![Retroactive Binaries](https://img.shields.io/badge/PE32%20fixtures-59%2F59%20passed-brightgreen.svg)](artifacts/README.md)
[![ABI Coverage](https://img.shields.io/badge/ABI%20coverage-100.0%25%20(635%2F635)-blueviolet.svg)](ROADMAP.md)
[![Overall Progress](https://img.shields.io/badge/progress%20to%20v1.0.0-80.0%25-brightgreen.svg)](ROADMAP.md)
[![Target](https://img.shields.io/badge/target-x86__64--alpine--linux--musl-blue.svg)](Dockerfile)

---

## High-Level Overview

ClassicBasic is structured as a modular **Cargo Workspace** (`crates/*`) separating hardware/OS foundations, the Visual Basic 6.0 Virtual Machine, execution runners, and the WebAssembly GUI client into peer crates:

- **Foundations (`cb-x86`, `cb-pe`, `cb-vfs`, `cb-win32`, `cb-net`)**: 32-bit x86 CPU/x87 FPU/TIB/SEH emulator, PE32 & `VB5!` header parsers, sandboxed Virtual Registry (`registry.json`) & VFS, Win32 (`KERNEL32`, `ADVAPI32`, `USER32`, `GDI32`) shims, and WebSocket display wire protocol.
- **VB6 Runtime (`msvbvm60`)**: Complete implementation of all 635 `MSVBVM60.DLL` exports, COM/ActiveX (`IUnknown`, `IDispatch`, `ITypeInfo`, `FSO`, `WScript`, `ADODB`), `BSTR`/`VARIANT`/`SAFEARRAY`/`Currency`, P-Code VM (`ProcCallEngine`), and 32-bit x86 IAT ordinal bridge.
- **Runners & Client (`vb6-run`, `cb-session`, `cb-client`)**: Standalone CLI and `--gui-web` runner, multi-tenant session broker, and WebAssembly HTML5 Canvas 2D client.

---

## Project Status & Progress (`v0.2.17`)

| Milestone / Metric | Weight | Status | Summary |
| :--- | :---: | :---: | :--- |
| **Milestone 1: Core Language Foundations** | 45% | **100% Complete (`v0.1.11`)** | 181/181 `rtc*` standard library functions + 318+ `__vba*` compiler helpers. |
| **Milestone 2: ActiveX, COM & 100% ABI** | 35% | **100% Complete (`v0.2.17`)** | 635/635 ordinals wired (0 stubs), COM/ActiveX, P-Code VM & 32-bit x86 CPU engine (`X86Cpu`). |
| **Milestone 3: Win32 GUI Web Canvas** | 20% | **Planned (`v0.3.1` $\to$ `v1.0.0`)** | 10 sequential phases (`+2.0%` each) for `USER32`/`GDI32`, `.rsrc` Forms, WebSockets & WASM client. |
| **Overall Progress toward `v1.0.0`** | **100%** | **80.0% Complete** | **332/332 automated tests** and **59/59 compiled PE32 binaries** (`App01`–`App34`) passing. |

---

## Architecture & Project Documentation

- **[Workspace & Crate Architecture (`docs/architecture/workspace-architecture.md`)](docs/architecture/workspace-architecture.md)**: Peer-level `crates/*` organization, acyclic dependency graph (DAG), and responsibility of each crate.
- **[Win32 GUI & GDI Web Canvas Specification (`docs/architecture/win32-gui-gdi.md`)](docs/architecture/win32-gui-gdi.md)**: Detailed design for `cb-win32`, `cb-net`, `cb-client`, `cb-session`, and the 10-phase Milestone 3 execution plan.
- **[Project Roadmap & ABI Matrix (`ROADMAP.md`)](ROADMAP.md)**: Complete history of Phases 1.1–2.17, mathematical versioning model, and Milestone 3 roadmap (`v0.3.1` $\to$ `v1.0.0`).
- **[Automated Test Suites (`tests/README.md`)](tests/README.md)**: Guide to the 6 domain-oriented integration and retroactive binary test suites.
- **[PE32 Fixtures & Reference Toolchain (`artifacts/README.md`)](artifacts/README.md)**: Catalog of the 59 compiled Visual Basic 6.0 Native and P-Code test executables and [Windows XP SP3 reference baseline (`artifacts/REFERENCE.md`)](artifacts/REFERENCE.md).

---

## Build, Run & Test

### 1. Build & Execute a VB6 PE32 Binary

```bash
# Build workspace in release mode
cargo build --bin vb6-run --release

# Inspect PE32 sections and VB5! metadata without executing
./target/x86_64-alpine-linux-musl/release/vb6-run artifacts/bin/App01-helloworld-pcode.exe --inspect

# Run a compiled VB6 binary in headless mode (Native or P-Code)
./target/x86_64-alpine-linux-musl/release/vb6-run artifacts/bin/App34-native-x86-cpu-engine-native.exe --headless
```

### 2. Run Lints & Automated Test Suites

```bash
# Verify zero compiler or Clippy warnings across all 9 workspace crates
cargo clippy --all-targets -- -D warnings

# Run all 332 tests across all domain suites and all 59 compiled PE32 binaries
cargo test -- --test-threads=1

# Run an individual test suite
cargo test --test foundations
cargo test --test types_automation
cargo test --test runtime_library
cargo test --test com_dispatch_typelib
cargo test --test retroactive_pe
cargo test --test retroactive_binaries
```

### 3. Container Build (Podman / Docker)

```bash
podman build -t classicbasic:latest .
podman run --rm classicbasic:latest /app/artifacts/bin/App20-loops-iterators-native.exe
```

---

## Authorship & Legal Notice

- **Architecture, Domain Guidance & Concept**: Victoria M. ([@victoria-devs](https://github.com/victoria-devs))
- **AI Pair Programming & Implementation**: Google Antigravity ([@antigravity-bot](https://github.com/antigravity-bot))

Distributed under the MIT License. See [`LICENSE.md`](LICENSE.md) for details.

> [!NOTE]
> **Trademark & Clean-Room Notice**:
> Microsoft®, Windows®, Visual Basic®, Visual Studio®, MSVB, and MSVBVM60 are registered trademarks of Microsoft Corporation. ClassicBasic is an independent clean-room re-implementation developed for educational study and software interoperability under Sections 29 and 30.61 of the *Copyright Act of Canada* (R.S.C., 1985, c. C-42). No proprietary Microsoft source code or system DLLs are distributed in this repository.
