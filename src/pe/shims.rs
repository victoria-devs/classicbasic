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

/// Global virtual memory base address where the current PE image is mapped.
pub static mut GLOBAL_MODULE_HANDLE: usize = 0x00400000;
/// Global mapped size of the active PE module.
pub static mut GLOBAL_MODULE_SIZE: usize = 0x10000;

/// Emulates Win32 `kernel32.dll!ExitProcess`.
///
/// Terminates the process immediately and passes the exit code to the host OS.
///
/// # Arguments
/// * `u_exit_code` - 32-bit unsigned process exit code.
#[no_mangle]
pub unsafe extern "system" fn win32_ExitProcess(u_exit_code: u32) {
    #[cfg(feature = "std")]
    println!("[win32 shim] ExitProcess called with code {}", u_exit_code);
    std::process::exit(u_exit_code as i32);
}

/// Emulates Win32 `kernel32.dll!GetModuleHandleA`.
///
/// Returns a handle (base address) to the mapped module for ANSI module names.
///
/// # Arguments
/// * `lp_module_name` - Pointer to null-terminated module name string, or null for current module.
///
/// # Returns
/// Virtual address base of the loaded module.
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

/// Emulates Win32 `kernel32.dll!GetModuleHandleW`.
///
/// Returns a handle (base address) to the mapped module for Unicode UTF-16 module names.
///
/// # Arguments
/// * `_lp_module_name` - Pointer to UTF-16 module name string, or null.
///
/// # Returns
/// Virtual address base of the loaded module.
#[no_mangle]
pub unsafe extern "system" fn win32_GetModuleHandleW(_lp_module_name: *const u16) -> usize {
    GLOBAL_MODULE_HANDLE
}

/// Emulates Win32 `kernel32.dll!GetCommandLineA`.
///
/// # Returns
/// Pointer to an ANSI string containing the process command line.
#[no_mangle]
pub unsafe extern "system" fn win32_GetCommandLineA() -> *const u8 {
    CMDLINE_A.as_ptr()
}

/// Emulates Win32 `kernel32.dll!GetCommandLineW`.
///
/// # Returns
/// Pointer to a UTF-16 string containing the process command line.
#[no_mangle]
pub unsafe extern "system" fn win32_GetCommandLineW() -> *const u16 {
    CMDLINE_W.as_ptr()
}

/// Emulates Win32 `kernel32.dll!GetVersion`.
///
/// Returns Windows OS version information.
/// Emulates Windows 2000 SP4: Major 5, Minor 0, Build 2195 (`0x08930005`).
///
/// # Returns
/// 32-bit packed OS version number.
#[no_mangle]
pub unsafe extern "system" fn win32_GetVersion() -> u32 {
    0x08930005
}

/// Emulates Win32 `kernel32.dll!GetLastError`.
///
/// # Returns
/// Last Win32 error code (`0` for success).
#[no_mangle]
pub unsafe extern "system" fn win32_GetLastError() -> u32 {
    0
}

/// Emulates Win32 `kernel32.dll!SetLastError`.
///
/// # Arguments
/// * `_dw_err_code` - 32-bit error code to store for calling thread.
#[no_mangle]
pub unsafe extern "system" fn win32_SetLastError(_dw_err_code: u32) {}

/// Emulates Win32 `kernel32.dll!AllocConsole`.
///
/// Initializes a console window for the process.
///
/// # Returns
/// Non-zero (`1` / `TRUE`) indicating success.
#[no_mangle]
pub unsafe extern "system" fn win32_AllocConsole() -> i32 {
    1 // TRUE
}

/// Emulates Win32 `kernel32.dll!GetStdHandle`.
///
/// Retrieves standard device handles (`STD_INPUT_HANDLE`, `STD_OUTPUT_HANDLE`, `STD_ERROR_HANDLE`).
///
/// # Arguments
/// * `n_std_handle` - Device identifier (`-10` for stdin, `-11` for stdout, `-12` for stderr).
///
/// # Returns
/// Virtual handle representing the standard stream.
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

/// Emulates Win32 `kernel32.dll!WriteConsoleA`.
///
/// Writes characters from a buffer directly to Linux standard output.
///
/// # Arguments
/// * `_h_console_output` - Handle to the console output buffer.
/// * `lp_buffer` - Pointer to buffer containing characters to write.
/// * `n_number_of_chars_to_write` - Count of characters to write.
/// * `lp_number_of_chars_written` - Output pointer receiving number of characters written.
/// * `_lp_reserved` - Reserved.
///
/// # Returns
/// Non-zero (`1`) on success.
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

/// Emulates Win32 `kernel32.dll!GetStartupInfoA`.
///
/// Fills the `STARTUPINFOA` structure with default parameters.
///
/// # Arguments
/// * `lp_startup_info` - Pointer to 68-byte STARTUPINFOA buffer.
#[no_mangle]
pub unsafe extern "system" fn win32_GetStartupInfoA(lp_startup_info: *mut u8) {
    if !lp_startup_info.is_null() {
        // Zero-fill 68-byte STARTUPINFOA struct
        core::ptr::write_bytes(lp_startup_info, 0, 68);
        *(lp_startup_info as *mut u32) = 68; // cb size
    }
}

/// Emulates Win32 `user32.dll!MessageBoxA`.
///
/// Displays a graphical-style ASCII modal dialog in the Linux terminal.
///
/// # Arguments
/// * `_hwnd` - Parent window handle.
/// * `lp_text` - Null-terminated ANSI string with message text.
/// * `lp_caption` - Null-terminated ANSI string with dialog caption title.
/// * `_u_type` - Dialog style bitmask.
///
/// # Returns
/// Button ID clicked by user (`1` / `IDOK`).
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

/// Fallback stub for unhandled Win32 API imports.
///
/// # Returns
/// Default status code `0`.
#[no_mangle]
pub unsafe extern "system" fn win32_DefaultStub() -> i32 {
    #[cfg(feature = "std")]
    eprintln!("[win32 shim] Unimplemented Win32 API called!");
    0
}
