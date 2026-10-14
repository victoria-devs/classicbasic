//! Minimal Win32 API Shims for KERNEL32 and USER32.
//!
//! Provides emulation for fundamental Win32 APIs imported by legacy executables,
//! redirecting them to standard POSIX / Rust behavior.

use std::ffi::CStr;

static CMDLINE_A: &[u8] = b"vb6_app.exe\0";
static CMDLINE_W: &[u16] = &[
    b'v' as u16, b'b' as u16, b'6' as u16, b'_' as u16,
    b'a' as u16, b'p' as u16, b'p' as u16, b'.' as u16,
    b'e' as u16, b'x' as u16, b'e' as u16, 0,
];

pub static mut GLOBAL_MODULE_HANDLE: usize = 0x00400000;
pub static mut GLOBAL_MODULE_SIZE: usize = 0x10000;

#[no_mangle]
pub unsafe extern "system" fn win32_ExitProcess(u_exit_code: u32) {
    #[cfg(feature = "std")]
    println!("[win32 shim] ExitProcess called with code {}", u_exit_code);
    std::process::exit(u_exit_code as i32);
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetModuleHandleA(lp_module_name: *const i8) -> usize {
    if lp_module_name.is_null() {
        GLOBAL_MODULE_HANDLE
    } else {
        #[cfg(feature = "std")]
        {
            let name = CStr::from_ptr(lp_module_name).to_string_lossy();
            println!("[win32 shim] GetModuleHandleA('{}')", name);
        }
        GLOBAL_MODULE_HANDLE
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetModuleHandleW(_lp_module_name: *const u16) -> usize {
    GLOBAL_MODULE_HANDLE
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetCommandLineA() -> *const u8 {
    CMDLINE_A.as_ptr()
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetCommandLineW() -> *const u16 {
    CMDLINE_W.as_ptr()
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetVersion() -> u32 {
    // Windows 2000 SP4: Major 5, Minor 0, Build 2195 (0x08930005)
    0x08930005
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetLastError() -> u32 {
    0
}

#[no_mangle]
pub unsafe extern "system" fn win32_SetLastError(_dw_err_code: u32) {}

#[no_mangle]
pub unsafe extern "system" fn win32_AllocConsole() -> i32 {
    1 // TRUE
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetStdHandle(n_std_handle: i32) -> usize {
    if n_std_handle == -11 {
        1 // stdout
    } else if n_std_handle == -12 {
        2 // stderr
    } else {
        0 // stdin
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_WriteConsoleA(
    _h_console_output: usize,
    lp_buffer: *const u8,
    n_number_of_chars_to_write: u32,
    lp_number_of_chars_written: *mut u32,
    _lp_reserved: *mut u8,
) -> i32 {
    if !lp_buffer.is_null() && n_number_of_chars_to_write > 0 {
        let slice = core::slice::from_raw_parts(lp_buffer, n_number_of_chars_to_write as usize);
        let s = String::from_utf8_lossy(slice);
        print!("{}", s);
        use std::io::Write;
        let _ = std::io::stdout().flush();
        if !lp_number_of_chars_written.is_null() {
            *lp_number_of_chars_written = n_number_of_chars_to_write;
        }
    }
    1
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetStartupInfoA(lp_startup_info: *mut u8) {
    if !lp_startup_info.is_null() {
        // Zero-fill 68-byte STARTUPINFOA struct
        core::ptr::write_bytes(lp_startup_info, 0, 68);
        *(lp_startup_info as *mut u32) = 68; // cb size
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_MessageBoxA(
    _hwnd: usize,
    lp_text: *const i8,
    lp_caption: *const i8,
    _u_type: u32,
) -> i32 {
    let title = if lp_caption.is_null() {
        "Visual Basic 6"
    } else {
        CStr::from_ptr(lp_caption).to_str().unwrap_or("Visual Basic 6")
    };

    let text = if lp_text.is_null() {
        ""
    } else {
        CStr::from_ptr(lp_text).to_str().unwrap_or("")
    };

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ [MessageBox] {:<49} ║", title);
    println!("╠════════════════════════════════════════════════════════════════╣");
    println!("║ {:<62} ║", text);
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    1 // IDOK
}

/// Fallback for unhandled Win32 API imports
#[no_mangle]
pub unsafe extern "system" fn win32_DefaultStub() -> i32 {
    #[cfg(feature = "std")]
    eprintln!("[win32 shim] Unimplemented Win32 API called!");
    0
}
