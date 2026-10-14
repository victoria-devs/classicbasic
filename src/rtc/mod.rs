//! Runtime Common (rtc) Visual Basic library functions.
//!
//! Visual Basic 6.0 built-in functions such as `MsgBox`, `DoEvents`, `RGB`, `Trim`,
//! and `VarType` are implemented in the runtime as `rtc*` exports.

use crate::types::{Bstr, Variant, VarType};

/// Implementation of VB6 `RGB(Red, Green, Blue)`.
/// Returns standard COLORREF: `(Red) | (Green << 8) | (Blue << 16)`.
#[no_mangle]
pub unsafe extern "system" fn rtcRgb(red: i32, green: i32, blue: i32) -> i32 {
    let r = (red.clamp(0, 255) as u32) & 0xFF;
    let g = (green.clamp(0, 255) as u32) & 0xFF;
    let b = (blue.clamp(0, 255) as u32) & 0xFF;
    (r | (g << 8) | (b << 16)) as i32
}

/// Implementation of VB6 `DoEvents()`.
/// Yields execution so that the operating system can process other events.
#[no_mangle]
pub unsafe extern "system" fn rtcDoEvents() -> i32 {
    // In a full implementation with Windows GUI, this runs a PeekMessage/TranslateMessage loop.
    0
}

/// Implementation of VB6 `Beep`.
#[no_mangle]
pub unsafe extern "system" fn rtcBeep() {
    #[cfg(feature = "trace_calls")]
    #[cfg(feature = "std")]
    eprintln!("[msvbvm60] rtcBeep");
}

/// Implementation of VB6 `VarType(VarName)`.
/// Returns an integer indicating the subtype of a Variant.
#[no_mangle]
pub unsafe extern "system" fn rtcVarType(var: *const Variant) -> VarType {
    if var.is_null() {
        return 0; // VT_EMPTY
    }
    (*var).vt
}

/// Implementation of VB6 `MsgBox(...)`.
/// Returns 1 (IDOK) by default.
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

/// Implementation of VB6 `Randomize [number]`.
static mut RNG_SEED: u32 = 0x12345678;

#[no_mangle]
pub unsafe extern "system" fn rtcRandomize(_number: *mut Variant) {
    RNG_SEED = 0x87654321;
}

/// Implementation of VB6 `Rnd [(number)]`.
#[no_mangle]
pub unsafe extern "system" fn rtcRandomNext() -> f32 {
    RNG_SEED = RNG_SEED.wrapping_mul(1664525).wrapping_add(1013904223);
    (RNG_SEED as f32) / (u32::MAX as f32)
}

/// Implementation of VB6 `Shell(PathName [, WindowStyle])`.
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

#[no_mangle]
pub unsafe extern "system" fn rtcTrimBstr(bstr: Bstr) -> Bstr {
    bstr
}

#[no_mangle]
pub unsafe extern "system" fn rtcLeftBstr(bstr: Bstr, _len: i32) -> Bstr {
    bstr
}

#[no_mangle]
pub unsafe extern "system" fn rtcLeftCharBstr(bstr: Bstr, _len: i32) -> Bstr {
    bstr
}

#[no_mangle]
pub unsafe extern "system" fn rtcRightBstr(bstr: Bstr, _len: i32) -> Bstr {
    bstr
}

#[no_mangle]
pub unsafe extern "system" fn rtcRightCharBstr(bstr: Bstr, _len: i32) -> Bstr {
    bstr
}

#[no_mangle]
pub unsafe extern "system" fn rtcMidCharBstr(bstr: Bstr, _start: i32, _len: i32) -> Bstr {
    bstr
}

#[no_mangle]
pub unsafe extern "system" fn rtcStrReverse(bstr: Bstr) -> Bstr {
    bstr
}

#[no_mangle]
pub unsafe extern "system" fn rtcSpaceBstr(_spaces: i32) -> Bstr {
    core::ptr::null_mut()
}
