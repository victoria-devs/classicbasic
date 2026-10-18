# ClassicBasic — Domain-Oriented Modular Test Architecture

This directory contains the automated test suites for **ClassicBasic** (`msvbvm60`).

The test layer is organized into modular architectural domains reflecting the true binary layout and subsystem boundaries of `msvbvm60.dll`:

1. `tests/foundations/`: Core runtime lifecycle, PE32 loader, SEH unwinding, ErrObject dual vtable, VarPtr, GetMem/PutMem primitives, and Win32 shims.
2. `tests/types_automation/`: Automation types including BSTR length-prefixed strings, Variant tagging and multi-level ByRef dereferencing, SafeArray descriptors and lock counters, Currency 64-bit fixed-point, and Date 1899-12-30 epoch serials.
3. `tests/runtime_library/`: Complete standard library implementations including string manipulation, banker's rounding math and financial functions, and explicit numeric conversions.
4. `tests/com_dispatch_typelib/`: COM runtime infrastructure, class factories, ROT, IDispatch early and late binding, MSFT TypeLib reader, ITypeInfo reflection, connection points, event sinks, and genuine IEnumVARIANT collection iteration.
5. `tests/retroactive_pe/`: Automated headless execution verification of all 59 compiled PE32 executables (both Native Code and P-Code).

---

## Running Tests

```bash
# Run all domain suites and retroactive binary test suites
cargo test

# Run a specific architectural domain suite
cargo test --test foundations
cargo test --test types_automation
cargo test --test runtime_library
cargo test --test com_dispatch_typelib
cargo test --test retroactive_pe

# Run backward-compatible retroactive binaries suite
cargo test --test retroactive_binaries
```

---

## Architectural Domain Hierarchy

| Domain Suite | Module | Subsystems & Capabilities |
| :--- | :--- | :--- |
| **`foundations`** | `execution_lifecycle` | DOS/PE32 validation, section mapping, VBHeader ("VB5!"), `ThunRTMain`, `__vbaEnd`, channel flushing, process exit hooks, virtual path resolution. |
| | `err_object_seh` | Dual vtable `ErrObject`, `__vbaOnError`, SEH frame unwinding, re-entrant error handling, `__vbaResume`. |
| | `memory_primitives` | `VarPtr`, `GetMem*`/`PutMem*`, KERNEL32 file I/O shims, OLEAUT32 and ADVAPI32 shims. |
| **`types_automation`** | `bstr` | UTF-16 length prefixing, zero-terminator bounds, mid-statement in-place mutation, concatenation, DBCS/codepage conversions. |
| | `variant` | Tagged unions, `VT_BYREF` multi-level indirection, deep dereferencing, 3-valued boolean logic (`VT_NULL`), relational testing, vector ops. |
| | `safearray` | Descriptors, FADF flags, lock counter verification (`c_locks > 0` returns Err 10), multi-dimensional stride, `ReDim Preserve`. |
| | `currency_date` | 64-bit integer 10,000 fixed-point arithmetic, IEEE float Date serials, leap year / day zero (1899-12-30) quirks. |
| **`runtime_library`** | `strings` | `rtc*` and `__vba*` string library: `InStr`, `InStrRev`, `Mid`, `Left`, `Right`, `Replace`, `Format` masks, `StrConv`. |
| | `math_financial` | Banker's rounding (`__vbaRound`), `Fix` vs `Int` negative truncation, trigonometric FPU routines, `Pmt`/`NPV`/`IRR`. |
| | `conversions` | Explicit coercions (`__vbaI2Var`, `__vbaR8Str`, etc.), overflow traps (`ERR_OVERFLOW = 6`), date and boolean parsers. |
| **`com_dispatch_typelib`**| `class_factory_rot` | In-proc COM servers, `DllGetClassObject`, `IClassFactory`, Virtual COM Registry, ActiveX shims (FSO, WSH, ADODB). |
| | `dispatch_late_binding` | `IDispatch::Invoke`, early/late-binding dispatch tables, zombie object lockout (`E_UNEXPECTED = 0x8000FFFF`), circular reference reclamation, STA threading apartments. |
| | `typelib_parser` | Binary MSFT typelib parsing, `ITypeLib`/`ITypeInfo` introspection, DISPID cache, `rtcCallByName`. |
| | `event_sinks_collections`| Connection points (`__vbaRaiseEvent`), genuine `IEnumVARIANT` dispatch (`_NewEnum` DISPID -4), `VbCollectionObject`, `ForEach` iterators. |
| **`retroactive_pe`** | `app01_to_app10_primitives` | Headless execution of App01 to App10 native and p-code binaries. |
| | `app11_to_app20_com_quirks` | Headless execution of App11 to App20 native and p-code binaries. |
| | `app21_to_app31_abi_pcode` | Headless execution of App21 to App31 native and p-code binaries. |
