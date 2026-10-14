//! Symbol Resolver for MSVBVM60.DLL and Win32 dependencies.

use crate::pe::shims::*;
use crate::*;

/// Resolves a symbol from a specified DLL by name or ordinal.
pub fn resolve_symbol(dll_name: &str, symbol_name: Option<&str>, ordinal: Option<u16>) -> usize {
    let dll = dll_name.to_ascii_lowercase();

    if dll.starts_with("msvbvm60") || dll.starts_with("msvbvm50") {
        resolve_msvbvm60(symbol_name, ordinal)
    } else if dll.starts_with("kernel32") {
        resolve_kernel32(symbol_name, ordinal)
    } else if dll.starts_with("user32") {
        resolve_user32(symbol_name, ordinal)
    } else {
        #[cfg(feature = "std")]
        eprintln!("[pe resolver] Warning: Unrecognized DLL '{}', returning stub", dll_name);
        win32_DefaultStub as *const () as usize
    }
}

fn resolve_kernel32(symbol: Option<&str>, _ordinal: Option<u16>) -> usize {
    match symbol {
        Some("ExitProcess") => win32_ExitProcess as *const () as usize,
        Some("GetModuleHandleA") => win32_GetModuleHandleA as *const () as usize,
        Some("GetModuleHandleW") => win32_GetModuleHandleW as *const () as usize,
        Some("GetCommandLineA") => win32_GetCommandLineA as *const () as usize,
        Some("GetCommandLineW") => win32_GetCommandLineW as *const () as usize,
        Some("GetStartupInfoA") => win32_GetStartupInfoA as *const () as usize,
        Some("GetVersion") => win32_GetVersion as *const () as usize,
        Some("AllocConsole") => win32_AllocConsole as *const () as usize,
        Some("GetStdHandle") => win32_GetStdHandle as *const () as usize,
        Some("WriteConsoleA") => win32_WriteConsoleA as *const () as usize,
        Some("GetLastError") => win32_GetLastError as *const () as usize,
        Some("SetLastError") => win32_SetLastError as *const () as usize,
        _ => win32_DefaultStub as *const () as usize,
    }
}

fn resolve_user32(symbol: Option<&str>, _ordinal: Option<u16>) -> usize {
    match symbol {
        Some("MessageBoxA") => win32_MessageBoxA as *const () as usize,
        _ => win32_DefaultStub as *const () as usize,
    }
}

fn resolve_msvbvm60(symbol: Option<&str>, ordinal: Option<u16>) -> usize {
    // 1. Try resolving by Ordinal
    if let Some(ord) = ordinal {
        match ord {
            100 => return ThunRTMain as *const () as usize,
            104 => return DllGetClassObject as *const () as usize,
            106 => return DllRegisterServer as *const () as usize,
            107 => return DllUnregisterServer as *const () as usize,
            516 => return rtcMsgBox as *const () as usize,
            517 => return rtcDoEvents as *const () as usize,
            518 => return rtcRgb as *const () as usize,
            520 => return rtcBeep as *const () as usize,
            591 => return rtcTrimBstr as *const () as usize,
            595 => return rtcVarType as *const () as usize,
            596 => return crate::stubs::rtcInStr as *const () as usize,
            600 => return rtcRandomize as *const () as usize,
            601 => return rtcRandomNext as *const () as usize,
            _ => {}
        }
    }

    // 2. Try resolving by Symbol Name
    if let Some(name) = symbol {
        match name {
            // Core Runtime
            "ThunRTMain" => return ThunRTMain as *const () as usize,
            "ProcCallEngine" => return ProcCallEngine as *const () as usize,
            "MethCallEngine" => return MethCallEngine as *const () as usize,
            "DllGetClassObject" => return DllGetClassObject as *const () as usize,
            "DllCanUnloadNow" => return DllCanUnloadNow as *const () as usize,
            "DllRegisterServer" => return DllRegisterServer as *const () as usize,
            "DllUnregisterServer" => return DllUnregisterServer as *const () as usize,

            // Memory
            "VarPtr" => return VarPtr as *const () as usize,
            "GetMem1" => return GetMem1 as *const () as usize,
            "GetMem2" => return GetMem2 as *const () as usize,
            "GetMem4" => return GetMem4 as *const () as usize,
            "GetMem8" => return GetMem8 as *const () as usize,
            "GetMemStr" => return GetMemStr as *const () as usize,
            "GetMemVar" => return GetMemVar as *const () as usize,
            "PutMem1" => return PutMem1 as *const () as usize,
            "PutMem2" => return PutMem2 as *const () as usize,
            "PutMem4" => return PutMem4 as *const () as usize,
            "PutMem8" => return PutMem8 as *const () as usize,
            "PutMemStr" => return PutMemStr as *const () as usize,
            "PutMemVar" => return PutMemVar as *const () as usize,
            "SetMemVar" => return SetMemVar as *const () as usize,

            // VBA Helpers
            "__vbaCopyBytes" => return __vbaCopyBytes as *const () as usize,
            "__vbaCopyBytesZero" => return __vbaCopyBytesZero as *const () as usize,
            "__vbaFreeStr" => return __vbaFreeStr as *const () as usize,
            "__vbaFreeStrList" => return __vbaFreeStrList as *const () as usize,
            "__vbaFreeVar" => return __vbaFreeVar as *const () as usize,
            "__vbaFreeVarList" => return __vbaFreeVarList as *const () as usize,
            "__vbaStrCopy" => return __vbaStrCopy as *const () as usize,
            "__vbaStrMove" => return __vbaStrMove as *const () as usize,
            "__vbaStrCat" => return __vbaStrCat as *const () as usize,
            "__vbaStrCmp" => return __vbaStrCmp as *const () as usize,
            "__vbaStrComp" => return crate::stubs::__vbaStrComp as *const () as usize,
            "__vbaVarAdd" => return __vbaVarAdd as *const () as usize,
            "__vbaVarSub" => return crate::stubs::__vbaVarSub as *const () as usize,
            "__vbaVarMul" => return crate::stubs::__vbaVarMul as *const () as usize,
            "__vbaVarDiv" => return crate::stubs::__vbaVarDiv as *const () as usize,
            "__vbaVarMove" => return __vbaVarMove as *const () as usize,
            "__vbaVarCopy" => return __vbaVarCopy as *const () as usize,
            "__vbaAryConstruct" => return __vbaAryConstruct as *const () as usize,
            "__vbaAryDestruct" => return __vbaAryDestruct as *const () as usize,
            "__vbaAryLock" => return __vbaAryLock as *const () as usize,
            "__vbaAryUnlock" => return __vbaAryUnlock as *const () as usize,
            "__vbaOnError" => return __vbaOnError as *const () as usize,
            "__vbaExceptHandler" => return __vbaExceptHandler as *const () as usize,

            // RTC Library
            "rtcRgb" => return rtcRgb as *const () as usize,
            "rtcDoEvents" => return rtcDoEvents as *const () as usize,
            "rtcBeep" => return rtcBeep as *const () as usize,
            "rtcVarType" => return rtcVarType as *const () as usize,
            "rtcMsgBox" => return rtcMsgBox as *const () as usize,
            "rtcRandomize" => return rtcRandomize as *const () as usize,
            "rtcRnd" => return rtcRandomNext as *const () as usize,
            "rtcTrimBstr" => return rtcTrimBstr as *const () as usize,
            "rtcInStr" => return crate::stubs::rtcInStr as *const () as usize,

            // Math
            "_CIcos" => return _CIcos as *const () as usize,
            "_CIsin" => return _CIsin as *const () as usize,
            "_CIsqrt" => return _CIsqrt as *const () as usize,
            "_CIatan" => return _CIatan as *const () as usize,
            "_CIexp" => return _CIexp as *const () as usize,
            "_CIlog" => return _CIlog as *const () as usize,
            "_CItan" => return _CItan as *const () as usize,
            "_allmul" => return _allmul as *const () as usize,
            _ => {}
        }
    }

    // Default fallback to generic stub
    win32_DefaultStub as *const () as usize
}
