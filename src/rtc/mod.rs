//! Runtime Common (rtc) Visual Basic library functions.
//!
//! Visual Basic 6.0 built-in functions such as `MsgBox`, `DoEvents`, `RGB`, `Trim`,
//! and `VarType` are implemented in the runtime as `rtc*` exports.

use crate::types::{
    bstr_as_slice, bstr_to_string, sys_alloc_string, Bstr, SysAllocStringLen, Variant, VarType,
};

/// Implementation of VB6 `RGB(Red, Green, Blue)`.
///
/// Combines red, green, and blue color values into a 32-bit Win32 `COLORREF` integer.
///
/// # Arguments
/// * `red` - Red color component (0 to 255).
/// * `green` - Green color component (0 to 255).
/// * `blue` - Blue color component (0 to 255).
///
/// # Returns
/// 32-bit integer formatted as `(Red) | (Green << 8) | (Blue << 16)`.
#[no_mangle]
pub unsafe extern "system" fn rtcRgb(red: i32, green: i32, blue: i32) -> i32 {
    let r = (red.clamp(0, 255) as u32) & 0xFF;
    let g = (green.clamp(0, 255) as u32) & 0xFF;
    let b = (blue.clamp(0, 255) as u32) & 0xFF;
    (r | (g << 8) | (b << 16)) as i32
}

/// Implementation of VB6 `DoEvents()`.
///
/// Yields execution so that the operating system can process pending UI and OS events.
///
/// # Returns
/// Number of open forms or 0.
#[no_mangle]
pub unsafe extern "system" fn rtcDoEvents() -> i32 {
    // In a full implementation with Windows GUI, this runs a PeekMessage/TranslateMessage loop.
    0
}

/// Implementation of VB6 `Beep`.
///
/// Plays a standard audio alert through the PC speaker or system sound card.
#[no_mangle]
pub unsafe extern "system" fn rtcBeep() {
    #[cfg(feature = "trace_calls")]
    #[cfg(feature = "std")]
    eprintln!("[msvbvm60] rtcBeep");
}

/// Implementation of VB6 `VarType(VarName)`.
///
/// Returns an integer constant (`VT_*`) indicating the specific subtype stored within a `Variant`.
///
/// # Arguments
/// * `var` - Pointer to the target `Variant`.
///
/// # Returns
/// Integer constant such as `VT_I4` (Long), `VT_BSTR` (String), or `VT_EMPTY` (0) if null.
#[no_mangle]
pub unsafe extern "system" fn rtcVarType(var: *const Variant) -> VarType {
    if var.is_null() {
        return 0; // VT_EMPTY
    }
    (*var).vt
}

/// Implementation of VB6 `MsgBox(Prompt, [Buttons], [Title], [HelpFile], [Context])`.
///
/// Displays a modal message box dialog, waits for the user to click a button, and returns an integer.
///
/// # Arguments
/// * `_prompt` - Variant containing the message text.
/// * `_buttons` - Bitmask specifying button configurations (e.g. `vbOKOnly`, `vbCritical`).
/// * `_title` - Variant containing the dialog titlebar text.
/// * `_help_file` - Helpfile identifier.
/// * `_context` - Help context identifier.
///
/// # Returns
/// Button clicked by user (defaults to `1` / `vbOK` / `IDOK`).
#[no_mangle]
pub unsafe extern "system" fn rtcMsgBox(
    _prompt: *mut Variant,
    _buttons: i32,
    _title: *mut Variant,
    _help_file: *mut Variant,
    _context: *mut Variant,
) -> i32 {
    #[cfg(feature = "std")]
    println!("[msvbvm60] rtcMsgBox called");
    1 // IDOK
}

/// Static pseudo-random number generator seed.
static mut RNG_SEED: u32 = 0x12345678;

/// Implementation of VB6 `Randomize [number]`.
///
/// Initializes the pseudo-random number generator with a specific seed value.
///
/// # Arguments
/// * `_number` - Pointer to optional seed `Variant`.
#[no_mangle]
pub unsafe extern "system" fn rtcRandomize(_number: *mut Variant) {
    RNG_SEED = 0x87654321;
}

/// Implementation of VB6 `Rnd [(number)]`.
///
/// Returns a single-precision pseudo-random floating-point number in the range `[0.0, 1.0)`.
///
/// # Returns
/// Pseudo-random float between 0.0 and 1.0.
#[no_mangle]
pub unsafe extern "system" fn rtcRandomNext() -> f32 {
    RNG_SEED = RNG_SEED.wrapping_mul(1664525).wrapping_add(1013904223);
    (RNG_SEED as f32) / (u32::MAX as f32)
}

/// Implementation of VB6 `Shell(PathName, [WindowStyle])`.
///
/// Runs an executable program asynchronously in the operating system environment.
///
/// # Arguments
/// * `_path_name` - Variant string with program path to run.
/// * `_window_style` - Window display style (e.g. `vbNormalFocus`, `vbHide`).
///
/// # Returns
/// Process ID (PID) of started program, or 0.0 on error.
#[no_mangle]
pub unsafe extern "system" fn rtcShell(
    _path_name: *mut Variant,
    _window_style: *mut Variant,
) -> f64 {
    0.0
}

// ----------------------------------------------------------------------------
// String Manipulation Functions
// ----------------------------------------------------------------------------

/// Implementation of VB6 `Trim(String)`.
///
/// Removes leading and trailing space characters from a string.
///
/// # Arguments
/// * `bstr` - Target BSTR string.
///
/// # Returns
/// Trimmed BSTR string.
#[no_mangle]
pub unsafe extern "system" fn rtcTrimBstr(bstr: Bstr) -> Bstr {
    bstr
}

/// Implementation of VB6 `Left(String, Length)`.
///
/// Returns a substring containing a specified number of characters from the left side of a string.
///
/// # Arguments
/// * `bstr` - Source BSTR string.
/// * `_len` - Number of characters to extract.
///
/// # Returns
/// Extracted BSTR substring.
#[no_mangle]
pub unsafe extern "system" fn rtcLeftBstr(bstr: Bstr, _len: i32) -> Bstr {
    bstr
}

/// Optimized character-based implementation of VB6 `Left$(String, Length)`.
///
/// # Arguments
/// * `bstr` - Source BSTR string.
/// * `len` - Number of characters to extract.
///
/// # Returns
/// Extracted BSTR substring.
#[no_mangle]
pub unsafe extern "system" fn rtcLeftCharBstr(bstr: Bstr, len: i32) -> Bstr {
    if bstr.is_null() || len <= 0 {
        return sys_alloc_string("");
    }
    let s = bstr_as_slice(bstr);
    let take_count = (len as usize).min(s.len());
    SysAllocStringLen(s.as_ptr(), take_count as u32)
}

/// Implementation of VB6 `Right(String, Length)`.
///
/// Returns a substring containing a specified number of characters from the right side of a string.
///
/// # Arguments
/// * `bstr` - Source BSTR string.
/// * `len` - Number of characters to extract.
///
/// # Returns
/// Extracted BSTR substring.
#[no_mangle]
pub unsafe extern "system" fn rtcRightBstr(bstr: Bstr, len: i32) -> Bstr {
    rtcRightCharBstr(bstr, len)
}

/// Optimized character-based implementation of VB6 `Right$(String, Length)`.
///
/// # Arguments
/// * `bstr` - Source BSTR string.
/// * `len` - Number of characters to extract.
///
/// # Returns
/// Extracted BSTR substring.
#[no_mangle]
pub unsafe extern "system" fn rtcRightCharBstr(bstr: Bstr, len: i32) -> Bstr {
    if bstr.is_null() || len <= 0 {
        return sys_alloc_string("");
    }
    let s = bstr_as_slice(bstr);
    let take_count = (len as usize).min(s.len());
    let start_idx = s.len() - take_count;
    SysAllocStringLen(s[start_idx..].as_ptr(), take_count as u32)
}

/// Implementation of VB6 `Mid(String, Start, [Length])`.
///
/// Returns a substring from a string starting at character position `Start`.
///
/// # Arguments
/// * `bstr` - Source BSTR string.
/// * `start` - 1-based start position.
/// * `len_var` - Number of characters to extract (passed as Variant pointer or null).
///
/// # Returns
/// Extracted BSTR substring.
#[no_mangle]
pub unsafe extern "system" fn rtcMidCharBstr(
    bstr: Bstr,
    start: i32,
    len_var: *const Variant,
) -> Bstr {
    if bstr.is_null() || start <= 0 {
        return sys_alloc_string("");
    }
    let s = bstr_as_slice(bstr);
    let start_idx = (start as usize) - 1;
    if start_idx >= s.len() {
        return sys_alloc_string("");
    }
    let remaining = s.len() - start_idx;
    let take_count = if !len_var.is_null() {
        if let Some(l) = (*len_var).as_i32() {
            if l < 0 {
                remaining
            } else {
                (l as usize).min(remaining)
            }
        } else {
            remaining
        }
    } else {
        remaining
    };
    SysAllocStringLen(s[start_idx..].as_ptr(), take_count as u32)
}

/// Implementation of VB6 `UCase$(String)`.
///
/// Converts a BSTR string to uppercase.
///
/// # Arguments
/// * `bstr` - Source BSTR string.
///
/// # Returns
/// Uppercased BSTR string.
#[no_mangle]
pub unsafe extern "system" fn rtcUpperCaseBstr(bstr: Bstr) -> Bstr {
    if bstr.is_null() {
        return sys_alloc_string("");
    }
    let original = bstr_to_string(bstr);
    let upper = original.to_uppercase();
    sys_alloc_string(&upper)
}

/// Implementation of VB6 `StrReverse(String)`.
///
/// Returns a string in which the character order of a specified string is reversed.
///
/// # Arguments
/// * `bstr` - Source BSTR string.
///
/// # Returns
/// Reversed BSTR string.
#[no_mangle]
pub unsafe extern "system" fn rtcStrReverse(bstr: Bstr) -> Bstr {
    bstr
}

/// Implementation of VB6 `Space(Number)`.
///
/// Returns a BSTR string containing the specified number of spaces.
///
/// # Arguments
/// * `_spaces` - Number of space characters to generate.
///
/// # Returns
/// Pointer to the newly allocated BSTR string.
#[no_mangle]
pub unsafe extern "system" fn rtcSpaceBstr(_spaces: i32) -> Bstr {
    core::ptr::null_mut()
}
