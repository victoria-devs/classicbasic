//! # MSVBVM60 - Visual Basic 6.0 Native Runtime Clone in Rust
//!
//! Re-implementation and reverse-engineering study clone of Microsoft's `MSVBVM60.DLL`
//! (Visual Basic Virtual Machine 6.0).
//!
//! ## Overview of MSVBVM60.DLL
//!
//! `MSVBVM60.DLL` is the runtime execution engine for all software built with Microsoft
//! Visual Basic 6.0 (released in 1998, code named "Thunder"). Whether a VB6 program was compiled
//! to Native Code (x86 machine instructions via an optimizing backend based on MSVC C2) or P-Code
//! (interpreted bytecode), it depends directly on this DLL.
//!
//! ### Key Responsibilities:
//! 1. **Application Lifecycle (`ThunRTMain`)**:
//!    - Native VB6 binaries have a minimal entry point that pushes the address of a `VBHeader`
//!      structure and invokes `ThunRTMain` ("Thunder Runtime Main", ordinal 100).
//!    - `ThunRTMain` validates the `"VB5!"` magic bytes, initializes COM/OLE apartments,
//!      registers SEH exception handlers (`__vbaExceptHandler`), loads forms/modules,
//!      invokes `Sub Main` or creates the initial Form, and manages the Win32 message pump.
//!
//! 2. **P-Code Interpreter Engines (`ProcCallEngine`, `MethCallEngine`)**:
//!    - For applications compiled in P-Code mode, execution is interpreted instruction-by-instruction
//!      by these engines.
//!
//! 3. **Type System and Object Model**:
//!    - **BSTR**: Length-prefixed (32-bit byte count at offset -4), UTF-16, null-terminated COM strings.
//!    - **VARIANT**: 16-byte tagged unions (`VT_I4`, `VT_BSTR`, `VT_DISPATCH`, etc.) enabling VB6 dynamic typing.
//!    - **SAFEARRAY**: Multi-dimensional bounded array descriptors.
//!    - **Currency (CY)**: 64-bit integer fixed-point with 4 decimal places.
//!    - **COM Interfaces**: `BASIC_CLASS_*`, `EVENT_SINK_*`, and `Zombie_*` implement `IUnknown` and `IDispatch`.
//!
//! 4. **Memory Manipulation (`GetMem*`, `PutMem*`, `VarPtr`)**:
//!    - Since VB6 lacks native pointer syntax, programmers historically used undocumented runtime
//!      exports like `VarPtr` (which passes back a variable's address) and `GetMem4`/`PutMem4`
//!      for direct memory manipulation and high-performance Win32 API interop.
//!
//! 5. **VBA Standard Library (`rtc*`)**:
//!    - Functions like `MsgBox` (`rtcMsgBox`), `DoEvents` (`rtcDoEvents`), `RGB` (`rtcRgb`),
//!      `InStr` (`rtcInStr`), `Trim` (`rtcTrimBstr`), and `Shell` (`rtcShell`).
//!
//! 6. **Compiler Runtime Helpers (`__vba*`)**:
//!    - String allocation/copying/concatenation (`__vbaStrCopy`, `__vbaStrCat`, `__vbaFreeStr`).
//!    - Variant math and conversions (`__vbaVarAdd`, `__vbaI4Str`, `__vbaVarMove`).
//!    - Array allocation and bounds checking (`__vbaAryConstruct`, `__vbaAryLock`).
//!    - Error handling (`__vbaOnError`, `__vbaExceptHandler`).
//!
//! 7. **Calling Conventions**:
//!    - VB6 on x86 utilizes `__stdcall` (`extern "system"` in Rust) for the vast majority of exports.
//!    - Specific CRT math routines (`_CI*`, `_adj_fdiv_*`, `_allmul`) use `extern "C"`.
//!
//! 8. **Export Ordinals (`msvbvm60.def`)**:
//!    - Windows DLLs often link by ordinal. This implementation includes a complete `.def` file
//!      covering all 635 exports ranging from ordinal 100 (`ThunRTMain`) to 2010 (`EbGetVBAObject`).

#![allow(non_snake_case)]
#![allow(unused_variables)]

pub mod com;
pub mod math;
pub mod mem;
pub mod rtc;
pub mod stubs;
pub mod thun_rt;
pub mod types;
pub mod vba;

use core::ffi::c_void;

// Re-export primary modules for convenient usage as a library
pub use com::*;
pub use math::*;
pub use mem::*;
pub use rtc::*;
pub use thun_rt::*;
pub use types::*;
pub use vba::*;

// ----------------------------------------------------------------------------
// Windows DLL Entry Point
// ----------------------------------------------------------------------------

const DLL_PROCESS_DETACH: u32 = 0;
const DLL_PROCESS_ATTACH: u32 = 1;
const DLL_THREAD_ATTACH: u32 = 2;
const DLL_THREAD_DETACH: u32 = 3;

/// Dynamic-link library (DLL) entry point.
///
/// Called by the Windows operating system loader when processes or threads
/// attach to or detach from MSVBVM60.DLL.
#[no_mangle]
pub unsafe extern "system" fn DllMain(
    _hinst_dll: *mut c_void,
    fdw_reason: u32,
    _lp_reserved: *mut c_void,
) -> i32 {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            #[cfg(feature = "std")]
            #[cfg(feature = "trace_calls")]
            eprintln!("[msvbvm60] DLL_PROCESS_ATTACH: Visual Basic 6.0 Runtime initialized");
        }
        DLL_THREAD_ATTACH => {}
        DLL_THREAD_DETACH => {}
        DLL_PROCESS_DETACH => {
            #[cfg(feature = "std")]
            #[cfg(feature = "trace_calls")]
            eprintln!("[msvbvm60] DLL_PROCESS_DETACH: Visual Basic 6.0 Runtime uninitialized");
        }
        _ => {}
    }
    1 // TRUE (success)
}

// ----------------------------------------------------------------------------
// Unit Tests (verifying foundational data types and memory logic)
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varptr_passthrough() {
        let mut x: u32 = 0x12345678;
        let ptr = &mut x as *mut u32 as *mut c_void;
        unsafe {
            assert_eq!(VarPtr(ptr), ptr);
        }
    }

    #[test]
    fn test_getmem_putmem() {
        let mut val: u32 = 0xCAFEBABE;
        let mut dest: u32 = 0;

        unsafe {
            let res = GetMem4(&val, &mut dest);
            assert_eq!(res, S_OK);
            assert_eq!(dest, 0xCAFEBABE);

            let res_put = PutMem4(&mut val, 0x12345678);
            assert_eq!(res_put, S_OK);
            assert_eq!(val, 0x12345678);
        }
    }

    #[test]
    fn test_currency_conversion() {
        let cy = Currency::from_f64(123.4567);
        assert_eq!(cy.0, 1234567);
        let back = cy.to_f64();
        assert!((back - 123.4567).abs() < 1e-4);
    }

    #[test]
    fn test_variant_integer() {
        let v = Variant::from_i32(42);
        assert_eq!(v.vt, VT_I4);
        assert_eq!(v.as_i32(), Some(42));
    }

    #[test]
    fn test_rgb_macro() {
        unsafe {
            // In Windows COLORREF: RGB(255, 0, 128) -> 0x008000FF
            let color = rtcRgb(255, 0, 128);
            assert_eq!(color as u32, 0x008000FF);
        }
    }

    #[test]
    fn test_vbheader_magic() {
        let mut hdr = VBHeader {
            sz_magic: *b"VB5!",
            runtime_build_version: 0,
            sz_language_dll: [0; 14],
            sz_sec_lang_dll: [0; 14],
            runtime_revision: 0,
            lcid: 0x409,
            sec_lcid: 0,
            lp_sub_main: 0,
            lp_project_data: 0,
            mdl_int_pool_cache: 0,
            mdl_int_pool_cache2: 0,
            dw_thread_flags: 0,
            dw_thread_count: 1,
            w_forms_count: 1,
            w_external_component_count: 0,
            dw_thunk_count: 0,
            lp_gui_table: 0,
            lp_ide_data: 0,
            lp_project_dependencies: 0,
            lp_import_table: 0,
            lp_export_table: 0,
        };
        assert!(hdr.is_valid());

        hdr.sz_magic = *b"TEST";
        assert!(!hdr.is_valid());
    }
}
