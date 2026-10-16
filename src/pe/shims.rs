//! Minimal Win32 API Shims for KERNEL32 and USER32.
//!
//! Provides emulation for fundamental Win32 APIs imported by legacy executables,
//! redirecting them to standard POSIX / Rust behavior.

use std::ffi::CStr;

static CMDLINE_A: &[u8] = b"vb6_app.exe\0";
static CMDLINE_W: &[u16] = &[
    b'v' as u16,
    b'b' as u16,
    b'6' as u16,
    b'_' as u16,
    b'a' as u16,
    b'p' as u16,
    b'p' as u16,
    b'.' as u16,
    b'e' as u16,
    b'x' as u16,
    b'e' as u16,
    0,
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

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

static LAST_ERROR: AtomicU32 = AtomicU32::new(0);
static NEXT_HANDLE: AtomicUsize = AtomicUsize::new(100);
static FILE_TABLE: Mutex<Option<HashMap<usize, File>>> = Mutex::new(None);

fn with_file_table<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<usize, File>) -> R,
{
    let mut guard = FILE_TABLE.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    f(guard.as_mut().unwrap())
}

/// Emulates Win32 `kernel32.dll!GetLastError`.
///
/// # Returns
/// Last Win32 error code stored for the calling thread.
#[no_mangle]
pub unsafe extern "system" fn win32_GetLastError() -> u32 {
    LAST_ERROR.load(Ordering::Relaxed)
}

/// Emulates Win32 `kernel32.dll!SetLastError`.
///
/// # Arguments
/// * `dw_err_code` - 32-bit error code to store for calling thread.
#[no_mangle]
pub unsafe extern "system" fn win32_SetLastError(dw_err_code: u32) {
    LAST_ERROR.store(dw_err_code, Ordering::Relaxed);
}

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
        let _ = std::io::stdout().flush();
        if !lp_number_of_chars_written.is_null() {
            *lp_number_of_chars_written = n_number_of_chars_to_write;
        }
    }
    1
}

const GENERIC_READ: u32 = 0x80000000;
const GENERIC_WRITE: u32 = 0x40000000;
const CREATE_NEW: u32 = 1;
const CREATE_ALWAYS: u32 = 2;
const OPEN_EXISTING: u32 = 3;
const OPEN_ALWAYS: u32 = 4;
const TRUNCATE_EXISTING: u32 = 5;
const INVALID_HANDLE_VALUE: usize = 0xFFFFFFFF;

/// Emulates Win32 `kernel32.dll!CreateFileA`.
///
/// Opens or creates a file or I/O device and maps it to a unique virtual handle.
#[no_mangle]
pub unsafe extern "system" fn win32_CreateFileA(
    lp_file_name: *const i8,
    dw_desired_access: u32,
    _dw_share_mode: u32,
    _lp_security_attributes: *mut core::ffi::c_void,
    dw_creation_disposition: u32,
    _dw_flags_and_attributes: u32,
    _h_template_file: usize,
) -> usize {
    if lp_file_name.is_null() {
        win32_SetLastError(87); // ERROR_INVALID_PARAMETER
        return INVALID_HANDLE_VALUE;
    }
    let c_str = CStr::from_ptr(lp_file_name);
    let path_str = c_str.to_string_lossy();

    let mut options = OpenOptions::new();
    let read = (dw_desired_access & GENERIC_READ) != 0;
    let write = (dw_desired_access & GENERIC_WRITE) != 0;

    match dw_creation_disposition {
        CREATE_NEW => {
            options.create_new(true).write(true).read(read);
        }
        CREATE_ALWAYS => {
            options.create(true).truncate(true).write(true).read(read);
        }
        OPEN_EXISTING => {
            options.read(read).write(write);
        }
        OPEN_ALWAYS => {
            options.create(true).write(write).read(read);
        }
        TRUNCATE_EXISTING => {
            options.truncate(true).write(true).read(read);
        }
        _ => {
            options.read(read).write(write);
        }
    }

    match options.open(path_str.as_ref()) {
        Ok(file) => {
            let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
            with_file_table(|table| {
                table.insert(handle, file);
            });
            win32_SetLastError(0);
            handle
        }
        Err(e) => {
            let err_code = match e.kind() {
                std::io::ErrorKind::NotFound => 2,         // ERROR_FILE_NOT_FOUND
                std::io::ErrorKind::PermissionDenied => 5, // ERROR_ACCESS_DENIED
                std::io::ErrorKind::AlreadyExists => 80,   // ERROR_FILE_EXISTS
                _ => 1,                                    // ERROR_INVALID_FUNCTION
            };
            win32_SetLastError(err_code);
            INVALID_HANDLE_VALUE
        }
    }
}

/// Emulates Win32 `kernel32.dll!WriteFile`.
///
/// Writes data to a specified file, standard output, or standard error stream.
#[no_mangle]
pub unsafe extern "system" fn win32_WriteFile(
    h_file: usize,
    lp_buffer: *const u8,
    n_number_of_bytes_to_write: u32,
    lp_number_of_bytes_written: *mut u32,
    _lp_overlapped: *mut core::ffi::c_void,
) -> i32 {
    if lp_buffer.is_null() {
        if !lp_number_of_bytes_written.is_null() {
            *lp_number_of_bytes_written = 0;
        }
        return 1;
    }

    let bytes = core::slice::from_raw_parts(lp_buffer, n_number_of_bytes_to_write as usize);

    if h_file == 1 || h_file == win32_GetStdHandle(-11) {
        let mut out = std::io::stdout();
        let _ = out.write_all(bytes);
        let _ = out.flush();
        if !lp_number_of_bytes_written.is_null() {
            *lp_number_of_bytes_written = n_number_of_bytes_to_write;
        }
        return 1;
    } else if h_file == 2 || h_file == win32_GetStdHandle(-12) {
        let mut err = std::io::stderr();
        let _ = err.write_all(bytes);
        let _ = err.flush();
        if !lp_number_of_bytes_written.is_null() {
            *lp_number_of_bytes_written = n_number_of_bytes_to_write;
        }
        return 1;
    }

    with_file_table(|table| {
        if let Some(file) = table.get_mut(&h_file) {
            match file.write_all(bytes) {
                Ok(_) => {
                    let _ = file.flush();
                    if !lp_number_of_bytes_written.is_null() {
                        *lp_number_of_bytes_written = n_number_of_bytes_to_write;
                    }
                    1
                }
                Err(_) => {
                    win32_SetLastError(29); // ERROR_WRITE_FAULT
                    0
                }
            }
        } else {
            win32_SetLastError(6); // ERROR_INVALID_HANDLE
            0
        }
    })
}

/// Emulates Win32 `kernel32.dll!ReadFile`.
///
/// Reads data from the specified file or input stream into buffer.
#[no_mangle]
pub unsafe extern "system" fn win32_ReadFile(
    h_file: usize,
    lp_buffer: *mut u8,
    n_number_of_bytes_to_read: u32,
    lp_number_of_bytes_read: *mut u32,
    _lp_overlapped: *mut core::ffi::c_void,
) -> i32 {
    if lp_buffer.is_null() {
        if !lp_number_of_bytes_read.is_null() {
            *lp_number_of_bytes_read = 0;
        }
        return 1;
    }

    let buf = core::slice::from_raw_parts_mut(lp_buffer, n_number_of_bytes_to_read as usize);

    if h_file == 0 {
        let mut stdin = std::io::stdin();
        match stdin.read(buf) {
            Ok(n) => {
                if !lp_number_of_bytes_read.is_null() {
                    *lp_number_of_bytes_read = n as u32;
                }
                return 1;
            }
            Err(_) => return 0,
        }
    }

    with_file_table(|table| {
        if let Some(file) = table.get_mut(&h_file) {
            match file.read(buf) {
                Ok(n) => {
                    if !lp_number_of_bytes_read.is_null() {
                        *lp_number_of_bytes_read = n as u32;
                    }
                    1
                }
                Err(_) => {
                    win32_SetLastError(30); // ERROR_READ_FAULT
                    0
                }
            }
        } else {
            win32_SetLastError(6); // ERROR_INVALID_HANDLE
            0
        }
    })
}

/// Emulates Win32 `kernel32.dll!CloseHandle`.
///
/// Closes an open object or file handle.
#[no_mangle]
pub unsafe extern "system" fn win32_CloseHandle(h_object: usize) -> i32 {
    if h_object <= 2 {
        return 1;
    }
    with_file_table(|table| {
        if table.remove(&h_object).is_some() {
            1
        } else {
            win32_SetLastError(6); // ERROR_INVALID_HANDLE
            0
        }
    })
}

/// Emulates Win32 `kernel32.dll!GetFileSize`.
///
/// Retrieves the size of the specified file, in bytes.
#[no_mangle]
pub unsafe extern "system" fn win32_GetFileSize(h_file: usize, lp_file_size_high: *mut u32) -> u32 {
    with_file_table(|table| {
        if let Some(file) = table.get(&h_file) {
            match file.metadata() {
                Ok(meta) => {
                    let len = meta.len();
                    if !lp_file_size_high.is_null() {
                        *lp_file_size_high = (len >> 32) as u32;
                    }
                    (len & 0xFFFFFFFF) as u32
                }
                Err(_) => {
                    win32_SetLastError(6);
                    0xFFFFFFFF
                }
            }
        } else {
            win32_SetLastError(6);
            0xFFFFFFFF
        }
    })
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
        CStr::from_ptr(lp_caption)
            .to_str()
            .unwrap_or("Visual Basic 6")
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

/// Emulates Win32 `kernel32.dll!GetTickCount`.
///
/// Returns the number of milliseconds that have elapsed since the system was started.
#[no_mangle]
pub unsafe extern "system" fn win32_GetTickCount() -> u32 {
    #[cfg(feature = "std")]
    {
        use std::time::SystemTime;
        static START: std::sync::OnceLock<SystemTime> = std::sync::OnceLock::new();
        let start = START.get_or_init(SystemTime::now);
        start.elapsed().map(|d| d.as_millis() as u32).unwrap_or(0)
    }
    #[cfg(not(feature = "std"))]
    {
        0
    }
}
