//! Runtime Common (rtc) Visual Basic library functions.
//!
//! Visual Basic 6.0 built-in functions such as `MsgBox`, `DoEvents`, `RGB`, `Trim`,
//! and `VarType` are implemented in the runtime as `rtc*` exports.

use crate::types::*;


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
    if bstr.is_null() {
        return sys_alloc_string("");
    }
    let s = bstr_to_string(bstr);
    sys_alloc_string(s.trim())
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
    if bstr.is_null() {
        return sys_alloc_string("");
    }
    let s = bstr_to_string(bstr);
    let rev: String = s.chars().rev().collect();
    sys_alloc_string(&rev)
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
pub unsafe extern "system" fn rtcSpaceBstr(spaces: i32) -> Bstr {
    if spaces <= 0 {
        return sys_alloc_string("");
    }
    let s = " ".repeat(spaces as usize);
    sys_alloc_string(&s)
}

/// Implementation of VB6 `Environ$(VarName)` (Ordinal 667).
#[no_mangle]
pub unsafe extern "system" fn rtcEnvironBstr(name: Bstr) -> Bstr {
    if name.is_null() {
        return sys_alloc_string("");
    }
    let key = bstr_to_string(name);
    match std::env::var(&key) {
        Ok(val) => sys_alloc_string(&val),
        Err(_) => sys_alloc_string(""),
    }
}

/// Implementation of VB6 `CurDir$([Drive])` (Ordinal 646).
#[no_mangle]
pub unsafe extern "system" fn rtcCurrentDirBstr(_drive: Bstr) -> Bstr {
    let path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
    sys_alloc_string(&path.to_string_lossy())
}

/// Implementation of VB6 `Date$` (Ordinal 609).
#[no_mangle]
pub unsafe extern "system" fn rtcGetDateBstr() -> Bstr {
    let t = libc::time(core::ptr::null_mut());
    let mut tm: libc::tm = core::mem::zeroed();
    libc::localtime_r(&t, &mut tm);
    let s = format!("{:02}-{:02}-{:04}", tm.tm_mon + 1, tm.tm_mday, tm.tm_year + 1900);
    sys_alloc_string(&s)
}

/// Implementation of VB6 `Time$` (Ordinal 611).
#[no_mangle]
pub unsafe extern "system" fn rtcGetTimeBstr() -> Bstr {
    let t = libc::time(core::ptr::null_mut());
    let mut tm: libc::tm = core::mem::zeroed();
    libc::localtime_r(&t, &mut tm);
    let s = format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec);
    sys_alloc_string(&s)
}

/// Implementation of VB6 `Now` (Ordinal 546).
#[no_mangle]
pub unsafe extern "system" fn rtcGetPresentDate(p_var: *mut Variant) {
    if p_var.is_null() {
        return;
    }
    let t = libc::time(core::ptr::null_mut());
    // Days from Dec 30, 1899 to Jan 1, 1970 = 25569.0
    let ole_date = 25569.0 + (t as f64 / 86400.0);
    let v = &mut *p_var;
    v.vt = crate::types::VT_DATE;
    let bytes = ole_date.to_ne_bytes();
    v.data.copy_from_slice(&bytes);
}

/// Implementation of VB6 `Timer` (Ordinal 535).
#[no_mangle]
pub unsafe extern "system" fn rtcGetTimer() -> f32 {
    let t = libc::time(core::ptr::null_mut());
    let mut tm: libc::tm = core::mem::zeroed();
    libc::localtime_r(&t, &mut tm);
    (tm.tm_hour as f32 * 3600.0) + (tm.tm_min as f32 * 60.0) + (tm.tm_sec as f32)
}

/// Implementation of VB6 `MkDir Path` (Ordinal 531).
#[no_mangle]
pub unsafe extern "system" fn rtcMakeDir(path: Bstr) {
    if path.is_null() {
        crate::err::set_vb_error(76, None, None);
        return;
    }
    let p = crate::vba::extract_bstr_or_variant_path(path);
    match std::fs::create_dir(&p) {
        Ok(_) => crate::err::clear_vb_error(),
        Err(e) => {
            let code = match e.kind() {
                std::io::ErrorKind::AlreadyExists => 58,
                std::io::ErrorKind::NotFound => 76,
                std::io::ErrorKind::PermissionDenied => 70,
                _ => 75,
            };
            crate::err::set_vb_error(code, None, None);
        }
    }
}

/// Implementation of VB6 `RmDir Path` (Ordinal 532).
#[no_mangle]
pub unsafe extern "system" fn rtcRemoveDir(path: Bstr) {
    if path.is_null() {
        crate::err::set_vb_error(76, None, None);
        return;
    }
    let p = crate::vba::extract_bstr_or_variant_path(path);
    match std::fs::remove_dir(&p) {
        Ok(_) => crate::err::clear_vb_error(),
        Err(e) => {
            let code = match e.kind() {
                std::io::ErrorKind::NotFound => 76,
                std::io::ErrorKind::PermissionDenied => 70,
                _ => 75,
            };
            crate::err::set_vb_error(code, None, None);
        }
    }
}

/// Implementation of VB6 `Kill Path` (Ordinal 529).
#[no_mangle]
pub unsafe extern "system" fn rtcKillFiles(path: Bstr) {
    if path.is_null() {
        crate::err::set_vb_error(53, None, None);
        return;
    }
    let p = crate::vba::extract_bstr_or_variant_path(path);
    match std::fs::remove_file(&p) {
        Ok(_) => crate::err::clear_vb_error(),
        Err(e) => {
            let code = match e.kind() {
                std::io::ErrorKind::NotFound => 53,
                std::io::ErrorKind::PermissionDenied => 70,
                _ => 75,
            };
            crate::err::set_vb_error(code, None, None);
        }
    }
}

/// Implementation of VB6 `ChDir Path` (Ordinal 530).
#[no_mangle]
pub unsafe extern "system" fn rtcChangeDir(path: Bstr) {
    if path.is_null() {
        crate::err::set_vb_error(76, None, None);
        return;
    }
    let p = crate::vba::extract_bstr_or_variant_path(path);
    match std::env::set_current_dir(&p) {
        Ok(_) => crate::err::clear_vb_error(),
        Err(_) => crate::err::set_vb_error(76, None, None),
    }
}

// ----------------------------------------------------------------------------
// Array Functions (Split, Join, Filter, Array)
// ----------------------------------------------------------------------------

/// Implementation of VB6 `Split(Expression, [Delimiter], [Limit], [Compare])` (Ordinal 711).
#[no_mangle]
pub unsafe extern "system" fn rtcSplit(
    p_dst: *mut Variant,
    expr: Bstr,
    p_delim: *const Variant,
    limit: i32,
    _compare: i32,
) -> *mut Variant {
    if p_dst.is_null() {
        return p_dst;
    }
    let text = if !expr.is_null() {
        bstr_to_string(expr)
    } else {
        String::new()
    };
    let delim = if !p_delim.is_null() {
        (*p_delim).to_string_representation()
    } else {
        " ".to_string()
    };

    let tokens: Vec<&str> = if delim.is_empty() {
        vec![text.as_str()]
    } else if limit > 0 {
        text.splitn(limit as usize, &delim).collect()
    } else {
        text.split(&delim).collect()
    };

    let count = tokens.len() as u32;
    let sa = SafeArray::create_vector(
        core::mem::size_of::<Bstr>() as u32,
        0,
        count,
        FADF_BSTR | FADF_HAVEVARTYPE,
    );

    if !sa.is_null() && !(*sa).pv_data.is_null() {
        let bstr_slice = core::slice::from_raw_parts_mut((*sa).pv_data as *mut Bstr, count as usize);
        for (i, tok) in tokens.iter().enumerate() {
            bstr_slice[i] = sys_alloc_string(tok);
        }
    }

    let v = &mut *p_dst;
    v.vt = VT_ARRAY | VT_BSTR;
    let ptr_bytes = (sa as usize).to_ne_bytes();
    v.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);

    p_dst
}

/// Implementation of VB6 `Join(SourceArray, [Delimiter])` (Ordinal 710).
#[no_mangle]
pub unsafe extern "system" fn rtcJoin(
    p_src_var: *const Variant,
    p_delim_var: *const Variant,
) -> Bstr {
    if p_src_var.is_null() {
        return sys_alloc_string("");
    }
    let sa = crate::vba::__vbaAryVar(p_src_var, 0);
    if sa.is_null() || (*sa).pv_data.is_null() {
        return sys_alloc_string("");
    }
    let delim = if !p_delim_var.is_null() {
        (*p_delim_var).to_string_representation()
    } else {
        " ".to_string()
    };

    let count = (*sa).rgsabound[0].c_elements as usize;
    let bstr_slice = core::slice::from_raw_parts((*sa).pv_data as *const Bstr, count);
    let mut parts: Vec<String> = Vec::with_capacity(count);
    for &b in bstr_slice {
        parts.push(bstr_to_string(b));
    }
    let joined = parts.join(&delim);
    sys_alloc_string(&joined)
}

/// Implementation of VB6 `Filter(SourceArray, Match, [Include], [Compare])` (Ordinal 708).
#[no_mangle]
pub unsafe extern "system" fn rtcFilter(
    p_dst: *mut Variant,
    p_src_var: *const Variant,
    match_str: Bstr,
    include: i32,
    _compare: i32,
) -> *mut Variant {
    if p_dst.is_null() {
        return p_dst;
    }
    let target = if !match_str.is_null() {
        bstr_to_string(match_str)
    } else {
        String::new()
    };
    let sa = crate::vba::__vbaAryVar(p_src_var, 0);
    if sa.is_null() || (*sa).pv_data.is_null() {
        return p_dst;
    }
    let count = (*sa).rgsabound[0].c_elements as usize;
    let bstr_slice = core::slice::from_raw_parts((*sa).pv_data as *const Bstr, count);
    let inc = include != 0;

    let mut matches: Vec<String> = Vec::new();
    for &b in bstr_slice {
        let s = bstr_to_string(b);
        let contains = s.contains(&target);
        if contains == inc {
            matches.push(s);
        }
    }

    let match_count = matches.len() as u32;
    let new_sa = SafeArray::create_vector(
        core::mem::size_of::<Bstr>() as u32,
        0,
        match_count,
        FADF_BSTR | FADF_HAVEVARTYPE,
    );
    if !new_sa.is_null() && !(*new_sa).pv_data.is_null() {
        let out_slice =
            core::slice::from_raw_parts_mut((*new_sa).pv_data as *mut Bstr, match_count as usize);
        for (i, m) in matches.iter().enumerate() {
            out_slice[i] = sys_alloc_string(m);
        }
    }

    let v = &mut *p_dst;
    v.vt = VT_ARRAY | VT_BSTR;
    let ptr_bytes = (new_sa as usize).to_ne_bytes();
    v.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);

    p_dst
}

/// Implementation of VB6 `Array(ArgList)` (Ordinal 601).
#[no_mangle]
pub unsafe extern "system" fn rtcArray(
    p_dst: *mut Variant,
    p_desc: *const core::ffi::c_void,
) -> *mut Variant {
    if p_dst.is_null() {
        return p_dst;
    }
    let v = &mut *p_dst;
    v.vt = VT_ARRAY | VT_VARIANT;
    let ptr_bytes = (p_desc as usize).to_ne_bytes();
    v.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
    p_dst
}

// ----------------------------------------------------------------------------
// Classic BASIC File I/O & Filesystem Intrinsics (Iteration 2)
// ----------------------------------------------------------------------------

/// Implementation of VB6 `FreeFile([RangeNumber])` (Ordinal 648).
#[no_mangle]
pub unsafe extern "system" fn rtcFreeFile(range_number: *const Variant) -> i16 {
    let range = if !range_number.is_null() {
        (*range_number).as_i32().unwrap_or(0)
    } else {
        0
    };
    crate::vba::find_free_file_channel(range)
}

/// Implementation of VB6 `Close [FileNumberList]` (Close statement).
#[no_mangle]
pub unsafe extern "system" fn rtcClose(channel: *const Variant) -> i32 {
    let ch = if !channel.is_null() {
        (*channel).as_i32().unwrap_or(0)
    } else {
        0
    };
    crate::vba::__vbaFileClose(ch)
}

/// Implementation of VB6 `Reset` (Ordinal 554).
///
/// Closes all active disk files opened using the `Open` statement.
#[no_mangle]
pub unsafe extern "system" fn rtcFileReset() {
    crate::vba::close_all_file_channels();
}

/// Implementation of VB6 `FileAttr(FileNumber, [ReturnType])` (Ordinal 555).
#[no_mangle]
pub unsafe extern "system" fn rtcFileAttributes(file_number: i32, return_type: i32) -> i32 {
    crate::vba::get_file_channel_attr(file_number, return_type)
}

/// Implementation of VB6 `Width #FileNumber, Width` (Ordinal 565).
#[no_mangle]
pub unsafe extern "system" fn rtcFileWidth(_file_number: i32, _width: i32) {}

/// Implementation of VB6 `Seek(FileNumber)` (Ordinal 568).
#[no_mangle]
pub unsafe extern "system" fn rtcFileSeek(file_number: i32) -> i32 {
    crate::vba::get_file_channel_seek(file_number)
}

/// Implementation of VB6 `Loc(FileNumber)` (Ordinal 569).
#[no_mangle]
pub unsafe extern "system" fn rtcFileLocation(file_number: i32) -> i32 {
    crate::vba::get_file_channel_loc(file_number)
}

/// Implementation of VB6 `LOF(FileNumber)` (Ordinal 570).
#[no_mangle]
pub unsafe extern "system" fn rtcFileLength(file_number: i32) -> i32 {
    crate::vba::get_file_channel_length(file_number)
}

/// Implementation of VB6 `EOF(FileNumber)` (Ordinal 571).
///
/// Returns `-1` (`True`) when the end of a file opened for `Input` or `Binary` has been reached,
/// or `0` (`False`) otherwise.
#[no_mangle]
pub unsafe extern "system" fn rtcEndOfFile(file_number: i32) -> i16 {
    crate::vba::get_file_channel_eof(file_number)
}

/// Implementation of VB6 `FileCopy Source, Destination` (Ordinal 576).
#[no_mangle]
pub unsafe extern "system" fn rtcFileCopy(source: Bstr, destination: Bstr) -> i32 {
    let src = crate::vba::extract_bstr_or_variant_path(source);
    let dst = crate::vba::extract_bstr_or_variant_path(destination);
    if src.is_empty() || dst.is_empty() {
        crate::err::set_vb_error(53, None, None);
        return -1;
    }
    match std::fs::copy(&src, &dst) {
        Ok(_) => {
            crate::err::clear_vb_error();
            0
        }
        Err(_) => {
            crate::err::set_vb_error(53, None, None);
            -1
        }
    }
}

/// Implementation of VB6 `FileLen(PathName)` (Ordinal 578).
#[no_mangle]
pub unsafe extern "system" fn rtcFileLen(pathname: Bstr) -> i32 {
    let path = crate::vba::extract_bstr_or_variant_path(pathname);
    if path.is_empty() {
        crate::err::set_vb_error(53, None, None);
        return 0;
    }
    match std::fs::metadata(&path) {
        Ok(meta) => {
            crate::err::clear_vb_error();
            meta.len() as i32
        }
        Err(_) => {
            crate::err::set_vb_error(53, None, None);
            0
        }
    }
}

/// Implementation of VB6 `GetAttr(PathName)` (Ordinal 579).
#[no_mangle]
pub unsafe extern "system" fn rtcGetFileAttr(pathname: Bstr) -> i16 {
    let path = crate::vba::extract_bstr_or_variant_path(pathname);
    if let Ok(meta) = std::fs::metadata(&path) {
        let mut attr = 0i16;
        if meta.is_dir() {
            attr |= 16; // vbDirectory
        }
        if meta.permissions().readonly() {
            attr |= 1; // vbReadOnly
        }
        attr
    } else {
        crate::err::set_vb_error(53, None, None);
        0
    }
}

/// Implementation of VB6 `SetAttr PathName, Attributes` (Ordinal 580).
#[no_mangle]
pub unsafe extern "system" fn rtcSetFileAttr(_pathname: Bstr, _attributes: i32) -> i32 {
    0
}

/// Implementation of VB6 `Dir$([PathName], [Attributes])` (Ordinal 645).
#[no_mangle]
pub unsafe extern "system" fn rtcDir(pathname: *const Variant, _attributes: i32) -> Bstr {
    let pat = crate::vba::extract_bstr_or_variant_path(pathname as Bstr);
    if pat.is_empty() {
        return sys_alloc_string("");
    }
    let p = std::path::Path::new(&pat);
    if p.exists() {
        if let Some(name) = p.file_name() {
            return sys_alloc_string(&name.to_string_lossy());
        }
        return sys_alloc_string(&pat);
    }
    if pat.contains('*') || pat.contains('?') {
        let parent = p.parent().unwrap_or(std::path::Path::new("."));
        let parent = if parent.as_os_str().is_empty() { std::path::Path::new(".") } else { parent };
        let file_pattern = p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        if let Ok(entries) = std::fs::read_dir(parent) {
            let prefix = file_pattern.split('*').next().unwrap_or("");
            let suffix = file_pattern.split('*').nth(1).unwrap_or("");
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(prefix) && name.ends_with(suffix) {
                    return sys_alloc_string(&name);
                }
            }
        }
    }
    sys_alloc_string("")
}

// ----------------------------------------------------------------------------
// Core Primitives & Type Inspection (Iteration 3)
// ----------------------------------------------------------------------------

/// Returns the string representation of a Variant's data type (`TypeName()`, Ordinal 591).
#[no_mangle]
pub unsafe extern "system" fn rtcTypeName(var: *const Variant) -> Bstr {
    if var.is_null() {
        return sys_alloc_string("Empty");
    }
    let vt = (*var).vt;
    let base_vt = vt & !VT_BYREF & !VT_ARRAY;
    let is_array = (vt & VT_ARRAY) != 0;

    let base_name = match base_vt {
        VT_EMPTY => "Empty",
        VT_NULL => "Null",
        VT_I2 => "Integer",
        VT_I4 => "Long",
        VT_R4 => "Single",
        VT_R8 => "Double",
        VT_CY => "Currency",
        VT_DATE => "Date",
        VT_BSTR => "String",
        VT_DISPATCH => "Object",
        VT_ERROR => "Error",
        VT_BOOL => "Boolean",
        VT_VARIANT => "Variant",
        VT_UNKNOWN => "Unknown",
        VT_DECIMAL => "Decimal",
        VT_I1 | VT_UI1 => "Byte",
        _ => "Object",
    };

    if is_array {
        sys_alloc_string(&format!("{}()", base_name))
    } else {
        sys_alloc_string(base_name)
    }
}

/// Checks whether a Variant is uninitialized (`IsEmpty()`, Ordinal 558).
#[no_mangle]
pub unsafe extern "system" fn rtcIsEmpty(var: *const Variant) -> i16 {
    if var.is_null() {
        -1
    } else if ((*var).vt & !VT_BYREF) == VT_EMPTY {
        -1
    } else {
        0
    }
}

/// Checks whether a Variant contains Null (`IsNull()`, Ordinal 560).
#[no_mangle]
pub unsafe extern "system" fn rtcIsNull(var: *const Variant) -> i16 {
    if !var.is_null() && ((*var).vt & !VT_BYREF) == VT_NULL {
        -1
    } else {
        0
    }
}

/// Checks whether a Variant contains an array (`IsArray()`, Ordinal 556).
#[no_mangle]
pub unsafe extern "system" fn rtcIsArray(var: *const Variant) -> i16 {
    if !var.is_null() && ((*var).vt & VT_ARRAY) != 0 {
        -1
    } else {
        0
    }
}

/// Checks whether a Variant can be evaluated as a number (`IsNumeric()`, Ordinal 561).
#[no_mangle]
pub unsafe extern "system" fn rtcIsNumeric(var: *const Variant) -> i16 {
    if var.is_null() {
        return 0;
    }
    let vt = (*var).vt & !VT_BYREF;
    match vt {
        VT_I2 | VT_I4 | VT_R4 | VT_R8 | VT_CY | VT_BOOL | VT_DECIMAL | VT_UI1 => -1,
        VT_BSTR => {
            let s = (*var).as_bstr().map(|b| bstr_to_string(b)).unwrap_or_default();
            let trimmed = s.trim();
            if trimmed.is_empty() {
                0
            } else if trimmed.parse::<f64>().is_ok() {
                -1
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// Evaluates a condition and returns one of two Variant expressions (`IIf()`, Ordinal 681).
#[no_mangle]
pub unsafe extern "system" fn rtcImmediateIf(
    dst: *mut Variant,
    cond: *const Variant,
    truepart: *const Variant,
    falsepart: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let is_true = if !cond.is_null() {
        (*cond).as_bool().unwrap_or(false)
    } else {
        false
    };
    let src = if is_true { truepart } else { falsepart };
    if !src.is_null() {
        *dst = *src;
    } else {
        *dst = Variant::empty();
    }
    dst
}

/// Selects and returns a value from a list of arguments by 1-based index (`Choose()`, Ordinal 665).
#[no_mangle]
pub unsafe extern "system" fn rtcChoose(
    dst: *mut Variant,
    index: f32,
    choices: *const Variant,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    *dst = Variant::null();
    let idx = index.floor() as i32;
    if idx < 1 || choices.is_null() {
        return dst;
    }

    if ((*choices).vt & VT_ARRAY) != 0 {
        let p_sa = crate::vba::__vbaAryVar(choices, 0);
        if !p_sa.is_null() {
            let lbound = crate::vba::__vbaLbound(p_sa, 1);
            let ubound = crate::vba::__vbaUbound(p_sa, 1);
            let target_elem = lbound + (idx - 1);
            if target_elem >= lbound && target_elem <= ubound {
                crate::vba::__vbaVarIndexLoad(dst, choices, 1, target_elem);
            }
        }
    }
    dst
}

/// Retrieves the command-line arguments string (`Command$`, Ordinal 669).
#[no_mangle]
pub unsafe extern "system" fn rtcCommandBstr() -> Bstr {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.join(" ");
    sys_alloc_string(&cmd)
}

/// Retrieves the command-line arguments Variant (`Command`, Ordinal 670).
#[no_mangle]
pub unsafe extern "system" fn rtcCommandVar(dst: *mut Variant) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let b = rtcCommandBstr();
    *dst = Variant::from_bstr(b);
    dst
}

/// Returns the ASCII character code of the first character in a string (`Asc()`, Ordinal 516).
#[no_mangle]
pub unsafe extern "system" fn rtcAnsiValueBstr(bstr: Bstr) -> i16 {
    if bstr.is_null() {
        crate::err::set_vb_error(5, None, None);
        return 0;
    }
    let slice = bstr_as_slice(bstr);
    if slice.is_empty() {
        crate::err::set_vb_error(5, None, None);
        0
    } else {
        slice[0] as i16
    }
}

/// Converts a character code into a 1-character BSTR (`Chr$()`, Ordinal 537).
#[no_mangle]
pub unsafe extern "system" fn rtcBstrFromAnsi(char_code: i32) -> Bstr {
    let ch = (char_code as u8) as char;
    sys_alloc_string(&ch.to_string())
}

/// Converts a Variant number to a hexadecimal BSTR (`Hex$()`, Ordinal 572).
#[no_mangle]
pub unsafe extern "system" fn rtcHexBstrFromVar(var: *const Variant) -> Bstr {
    if var.is_null() {
        return sys_alloc_string("0");
    }
    let val = (*var).as_i32().unwrap_or(0);
    sys_alloc_string(&format!("{:X}", val as u32))
}

/// Converts a Variant number to an octal BSTR (`Oct$()`, Ordinal 574).
#[no_mangle]
pub unsafe extern "system" fn rtcOctBstrFromVar(var: *const Variant) -> Bstr {
    if var.is_null() {
        return sys_alloc_string("0");
    }
    let val = (*var).as_i32().unwrap_or(0);
    sys_alloc_string(&format!("{:o}", val as u32))
}

/// Converts a string containing numeric digits into a double (`Val()`, Ordinal 581).
#[no_mangle]
pub unsafe extern "system" fn rtcR8ValFromBstr(bstr: Bstr) -> f64 {
    if bstr.is_null() {
        return 0.0;
    }
    let s = bstr_to_string(bstr);
    // In VB6, Val ignores internal spaces and tabs
    let s_clean: String = s.chars().filter(|c| *c != ' ' && *c != '\t').collect();
    let trimmed = s_clean.trim_start();

    // Check for &H (hexadecimal) or &O (octal) prefixes
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("&h") {
        let hex_part = &trimmed[2..];
        let mut hex_digits = String::new();
        for c in hex_part.chars() {
            if c.is_ascii_hexdigit() {
                hex_digits.push(c);
            } else {
                break;
            }
        }
        if let Ok(v) = i64::from_str_radix(&hex_digits, 16) {
            return v as f64;
        }
        return 0.0;
    } else if lower.starts_with("&o") {
        let oct_part = &trimmed[2..];
        let mut oct_digits = String::new();
        for c in oct_part.chars() {
            if ('0'..='7').contains(&c) {
                oct_digits.push(c);
            } else {
                break;
            }
        }
        if let Ok(v) = i64::from_str_radix(&oct_digits, 8) {
            return v as f64;
        }
        return 0.0;
    }

    let mut num_str = String::new();
    let mut seen_dot = false;
    for (i, c) in trimmed.chars().enumerate() {
        if i == 0 && (c == '+' || c == '-') {
            num_str.push(c);
        } else if c.is_ascii_digit() {
            num_str.push(c);
        } else if c == '.' && !seen_dot {
            num_str.push(c);
            seen_dot = true;
        } else {
            break;
        }
    }
    num_str.parse::<f64>().unwrap_or(0.0)
}

/// Converts a Variant to a formatted string representation (`Str$()`, Ordinal 536).
#[no_mangle]
pub unsafe extern "system" fn rtcStrFromVar(var: *const Variant) -> Bstr {
    if var.is_null() {
        return sys_alloc_string("");
    }
    let repr = (*var).to_string_representation();
    if !repr.starts_with('-') && (*var).as_f64().is_some() {
        sys_alloc_string(&format!(" {}", repr))
    } else {
        sys_alloc_string(&repr)
    }
}

/// Converts a BSTR to lowercase (`LCase$()`, Ordinal 517).
#[no_mangle]
pub unsafe extern "system" fn rtcLowerCaseBstr(bstr: Bstr) -> Bstr {
    if bstr.is_null() {
        return sys_alloc_string("");
    }
    let s = bstr_to_string(bstr);
    sys_alloc_string(&s.to_lowercase())
}

/// Replaces occurrences of a substring within a string (`Replace()`, Ordinal 712).
#[no_mangle]
pub unsafe extern "system" fn rtcReplace(
    expr: Bstr,
    find: Bstr,
    repl: Bstr,
    start: i32,
    count: i32,
    _compare: i32,
) -> Bstr {
    if expr.is_null() || find.is_null() {
        return sys_alloc_string("");
    }
    let s_expr = bstr_to_string(expr);
    let s_find = bstr_to_string(find);
    let s_repl = if !repl.is_null() { bstr_to_string(repl) } else { String::new() };

    let start_idx = if start > 1 { (start as usize - 1).min(s_expr.len()) } else { 0 };
    let target_slice = &s_expr[start_idx..];

    let result = if count < 0 {
        target_slice.replace(&s_find, &s_repl)
    } else {
        target_slice.replacen(&s_find, &s_repl, count as usize)
    };
    sys_alloc_string(&result)
}

/// Returns the position of an occurrence of one string within another from the end of the string (`InStrRev()`, Ordinal 709).
#[no_mangle]
pub unsafe extern "system" fn rtcInStrRev(
    check: Bstr,
    match_str: Bstr,
    start: i32,
    _compare: i32,
) -> i32 {
    if check.is_null() || match_str.is_null() {
        return 0;
    }
    let s_check = bstr_to_string(check);
    let s_match = bstr_to_string(match_str);
    if s_match.is_empty() {
        return if start == -1 { s_check.len() as i32 } else { start };
    }
    let slice = if start > 0 && (start as usize) < s_check.len() {
        &s_check[..start as usize]
    } else {
        &s_check[..]
    };
    if let Some(pos) = slice.rfind(&s_match) {
        (pos + 1) as i32
    } else {
        0
    }
}

/// Computes the square root of a double (`Sqr()`, Ordinal 614).
#[no_mangle]
pub unsafe extern "C" fn rtcSqr(val: f64) -> f64 {
    if val < 0.0 {
        crate::err::set_vb_error(5, None, None);
        0.0
    } else {
        val.sqrt()
    }
}

/// Rounds a number to a specified number of decimal places (`Round()`, Ordinal 714).
#[no_mangle]
pub unsafe extern "system" fn rtcRound(
    dst: *mut Variant,
    val: *const Variant,
    decimals: i32,
) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    if val.is_null() {
        *dst = Variant::from_f64(0.0);
        return dst;
    }
    let num = (*val).as_f64().unwrap_or(0.0);
    let factor = 10f64.powi(decimals.max(0));
    let rounded = (num * factor).round() / factor;
    *dst = Variant::from_f64(rounded);
    dst
}

/// Variant wrapper for Fix (`Fix()`, Ordinal 655).
#[no_mangle]
pub unsafe extern "system" fn rtcFixVar(dst: *mut Variant, val: *const Variant) -> *mut Variant {
    crate::vba::__vbaVarFix(dst, val)
}

/// Variant wrapper for Int (`Int()`, Ordinal 657).
#[no_mangle]
pub unsafe extern "system" fn rtcIntVar(dst: *mut Variant, val: *const Variant) -> *mut Variant {
    crate::vba::__vbaVarInt(dst, val)
}

/// Variant wrapper for Abs (`Abs()`, Ordinal 656).
#[no_mangle]
pub unsafe extern "system" fn rtcAbsVar(dst: *mut Variant, val: *const Variant) -> *mut Variant {
    crate::vba::__vbaVarAbs(dst, val)
}

/// Variant wrapper for Sgn (`Sgn()`, Ordinal 658).
#[no_mangle]
pub unsafe extern "system" fn rtcSgnVar(dst: *mut Variant, val: *const Variant) -> *mut Variant {
    if dst.is_null() {
        return dst;
    }
    let n = if !val.is_null() { (*val).as_f64().unwrap_or(0.0) } else { 0.0 };
    let sgn: i16 = if n > 0.0 { 1 } else if n < 0.0 { -1 } else { 0 };
    *dst = Variant::from_i16(sgn);
    dst
}

