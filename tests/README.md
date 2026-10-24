# ClassicBasic — Domain-Oriented Modular Test Architecture

This directory contains the workspace-wide automated integration and retroactive binary test suites (**332 tests across 6 suites**) validating the **ClassicBasic** Cargo Workspace (`crates/*`: `cb-x86`, `cb-pe`, `cb-vfs`, `cb-win32`, `cb-net`, `msvbvm60`, `vb6-run`, `cb-session`, `cb-client`).

The test layer is organized into modular architectural domains:

1. **`tests/foundations/`** *(50 tests)*: Core runtime lifecycle (`ThunRTMain`), PE32 loader (`cb-pe` + `msvbvm60::pe`), 32-bit x86 CPU & x87 FPU execution engine (`cb-x86`), Win32 `FS:[0]` TIB & SEH unwinding, `ErrObject` dual vtable, `VarPtr`/`GetMem*`/`PutMem*` primitives, and Win32 shims (`cb-win32`).
2. **`tests/types_automation/`** *(82 tests)*: Automation types including `BSTR` length-prefixed UTF-16 strings, `Variant` tagging and multi-level `VT_BYREF` dereferencing, `SafeArray` descriptors and lock counters, `Currency` 64-bit fixed-point arithmetic, and `Date` `1899-12-30` epoch serials.
3. **`tests/runtime_library/`** *(76 tests)*: Complete standard library implementations (`rtc*` and `__vba*`) including string manipulation, Banker's Rounding math and financial functions, classic file I/O channels, and explicit numeric conversions.
4. **`tests/com_dispatch_typelib/`** *(60 tests)*: COM runtime infrastructure, class factories, ROT, `IDispatch` early and late binding, MSFT TypeLib reader, `ITypeInfo` reflection, Virtual Registry (`cb-vfs`), ActiveX shims (`FSO`, `WScript`, `ADODB`), connection points, event sinks, and genuine `IEnumVARIANT` collection iteration.
5. **`tests/retroactive_pe/`** *(5 grouped suites)* & **`tests/retroactive_binaries.rs`** *(59 individual binary tests)*: End-to-end headless execution verification of all 59 compiled PE32 executables (30 Native x86 binaries + 29 P-Code binaries) through `vb6-run`.

---

## Running Tests

```bash
# Run all 332 tests across the workspace
cargo test -- --test-threads=1

# Run a specific architectural domain suite
cargo test --test foundations
cargo test --test types_automation
cargo test --test runtime_library
cargo test --test com_dispatch_typelib
cargo test --test retroactive_pe

# Run the 59 individual compiled PE32 binary tests
cargo test --test retroactive_binaries
```

---

## Architectural Domain Hierarchy

| Domain Suite | Module | Subsystems & Crates Validated |
| :--- | :--- | :--- |
| **`foundations`** | `execution_lifecycle` | DOS/PE32 validation (`cb-pe`), section mapping, `VBHeader` (`"VB5!"`), `ThunRTMain`, `__vbaEnd`, channel flushing, virtual path resolution (`cb-vfs`). |
| | `native_x86_cpu_engine` | 32-bit `X86Cpu`, GPRs, `EFLAGS`, ModR/M, ALU, `X87Fpu` transcendental pipeline, `FS:[0]` TIB SEH unwinding, and COM vtable dispatch (`cb-x86`). |
| | `err_object_seh` | Dual vtable `ErrObject`, `__vbaOnError`, SEH frame unwinding, re-entrant error handling, `__vbaResume` (`msvbvm60::err`). |
| | `memory_primitives` | `VarPtr`, `GetMem*`/`PutMem*` (`msvbvm60::mem`), `KERNEL32` file I/O shims, `ADVAPI32` registry shims (`cb-win32`), and `OLEAUT32` shims. |
| **`types_automation`** | `bstr` | UTF-16 length prefixing, zero-terminator bounds, `Mid$` statement in-place mutation, concatenation, DBCS/codepage conversions. |
| | `variant` | 16-byte tagged unions, `VT_BYREF` multi-level indirection, deep dereferencing, 3-valued boolean logic (`VT_NULL`), bitwise algebra (`Imp`, `Eqv`, `Xor`, `Or`, `And`), relational testing. |
| | `safearray` | Multi-dimensional descriptors, `FADF_*` flags, lock counter verification (`c_locks > 0` $\to$ Err 10), stride calculation, `ReDim Preserve`, UDT records. |
| | `currency_date` | 64-bit integer 10,000 fixed-point arithmetic, IEEE float `Date` serials, leap year / day zero (`1899-12-30`) quirks. |
| **`runtime_library`** | `strings` | `rtc*` and `__vba*` string library: `InStr`, `InStrRev`, `Mid`, `Left`, `Right`, `Replace`, `Format` masks, `StrConv`. |
| | `math_financial` | Banker's rounding (`__vbaRound`), `Fix` vs `Int` negative truncation, trigonometric FPU routines, `PMT`/`NPV`/`IRR`/`MIRR`/`SLN`/`SYD`/`DDB`. |
| | `conversions` | Explicit coercions (`__vbaI2Var`, `__vbaR8Str`, etc.), overflow traps (`ERR_OVERFLOW = 6`), date and boolean parsers. |
| **`com_dispatch_typelib`**| `class_factory_rot` | In-proc COM servers, `DllGetClassObject`, `IClassFactory`, Virtual COM Registry (`cb-vfs`), ActiveX shims (`FSO`, `WSH`, `ADODB`), DCOM/ROT. |
| | `dispatch_late_binding` | `IDispatch::Invoke`, early/late-binding dispatch tables, zombie object lockout (`E_UNEXPECTED = 0x8000FFFF`), circular reference reclamation, STA threading apartments. |
| | `typelib_parser` | Binary MSFT typelib parsing, `ITypeLib`/`ITypeInfo` introspection, DISPID cache, `rtcCallByName`, `Eb*`/`Tip*` IDE introspection. |
| | `event_sinks_collections`| Connection points (`__vbaRaiseEvent`), genuine `IEnumVARIANT` dispatch (`_NewEnum` DISPID `-4`), `VBA.Collection`, `For Each` iterators. |
| **`retroactive_pe` & `retroactive_binaries`** | `App01`–`App31` | End-to-end execution of all 59 compiled PE32 binaries (Native x86 via `cb-x86` + `msvbvm60::x86::bridge` and P-Code via `msvbvm60::pcode::vm`) using `vb6-run`. |
