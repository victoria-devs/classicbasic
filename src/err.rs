//! Structured Error Handling and `ErrObject` implementation.
//!
//! Provides the runtime state machine for Visual Basic 6 structured error handling
//! (`On Error Resume Next`, `On Error GoTo <Label>`, `On Error GoTo 0`), runtime
//! error code lookup, and the intrinsic COM `ErrObject` (`Err.Number`, `Err.Description`,
//! `Err.Source`, `Err.Clear`, `Err.Raise`, `Err.LastDllError`).

use core::ffi::c_void;
use std::sync::Mutex;

use crate::types::{bstr_to_string, sys_alloc_string, Bstr, Variant, S_OK};

/// Internal thread/global state for the Visual Basic 6 error handling engine.
#[derive(Debug, Clone)]
pub struct VbErrorState {
    /// Visual Basic runtime error number (e.g., 11 for Division by zero, 0 for no error).
    pub number: i32,
    /// Source of the error (e.g., project or class name).
    pub source: String,
    /// Human-readable description of the error.
    pub description: String,
    /// Path to the Help file associated with this error.
    pub help_file: String,
    /// Context ID in the Help file.
    pub help_context: i32,
    /// Error code returned by the last Win32 API call (`GetLastError`).
    pub last_dll_error: i32,
    /// Current error handler mode:
    /// - `0`: No active handler / `On Error GoTo 0` (unhandled exceptions abort).
    /// - `-1`: `On Error Resume Next` (ignore error and continue next instruction).
    /// - `> 0`: `On Error GoTo <Label>` (jump to specified label/block).
    pub handler_mode: i32,
}

impl Default for VbErrorState {
    fn default() -> Self {
        Self {
            number: 0,
            source: String::new(),
            description: String::new(),
            help_file: String::new(),
            help_context: 0,
            last_dll_error: 0,
            handler_mode: 0,
        }
    }
}

/// Global error state tracked across invocations.
static GLOBAL_ERROR_STATE: Mutex<Option<VbErrorState>> = Mutex::new(None);

/// Ensures error state is initialized and returns a locked guard.
fn with_error_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut VbErrorState) -> R,
{
    let mut guard = GLOBAL_ERROR_STATE.lock().unwrap();
    if guard.is_none() {
        *guard = Some(VbErrorState::default());
    }
    f(guard.as_mut().unwrap())
}

/// Returns the standard Visual Basic 6 error description for a given error code.
pub fn get_standard_error_description(code: i32) -> &'static str {
    match code {
        3 => "Return without GoSub",
        5 => "Invalid procedure call or argument",
        6 => "Overflow",
        7 => "Out of memory",
        9 => "Subscript out of range",
        10 => "This array is fixed or temporarily locked",
        11 => "Division by zero",
        13 => "Type mismatch",
        14 => "Out of string space",
        20 => "Resume without error",
        28 => "Out of stack space",
        52 => "Bad file name or number",
        53 => "File not found",
        54 => "Bad file mode",
        55 => "File already open",
        57 => "Device I/O error",
        58 => "File already exists",
        61 => "Disk full",
        62 => "Input past end of file",
        67 => "Too many files",
        68 => "Device unavailable",
        70 => "Permission denied",
        75 => "Path/File access error",
        76 => "Path not found",
        91 => "Object variable or With block variable not set",
        92 => "For loop not initialized",
        94 => "Invalid use of Null",
        424 => "Object required",
        429 => "ActiveX component can't create object",
        438 => "Object doesn't support this property or method",
        440 => "Automation error",
        1001 => "Custom Raised Business Error",
        _ => "Application-defined or object-defined error",
    }
}

/// Clears the current runtime error state (`Err.Clear`).
pub fn clear_vb_error() {
    with_error_state(|state| {
        state.number = 0;
        state.source.clear();
        state.description.clear();
        state.help_file.clear();
        state.help_context = 0;
    });
}

/// Sets the current runtime error state.
pub fn set_vb_error(number: i32, description: Option<&str>, source: Option<&str>) {
    with_error_state(|state| {
        state.number = number;
        state.description = match description {
            Some(d) => d.to_string(),
            None => get_standard_error_description(number).to_string(),
        };
        state.source = match source {
            Some(s) => s.to_string(),
            None => String::new(),
        };
    });
}

/// Retrieves the current `Err.Number`.
pub fn get_vb_error_number() -> i32 {
    with_error_state(|state| state.number)
}

/// Retrieves the current `Err.Description`.
pub fn get_vb_error_description() -> String {
    with_error_state(|state| state.description.clone())
}

/// Retrieves the current `Err.Source`.
pub fn get_vb_error_source() -> String {
    with_error_state(|state| state.source.clone())
}

/// Sets the error handler mode (`On Error ...`).
pub fn set_vb_error_handler(mode: i32) {
    with_error_state(|state| {
        state.handler_mode = mode;
    });
}

/// Retrieves the error handler mode.
pub fn get_vb_error_handler() -> i32 {
    with_error_state(|state| state.handler_mode)
}

// ----------------------------------------------------------------------------
// COM Intrinsic ErrObject Implementation
// ----------------------------------------------------------------------------

/// COM VTable for Visual Basic 6 `_ErrObject` interface.
///
/// Matches the exact dual-interface VTable layout expected by MSVBVM60-compiled binaries:
/// - Slots 0..2: `IUnknown` (`QueryInterface`, `AddRef`, `Release`)
/// - Slots 3..6: `IDispatch` (`GetTypeInfoCount`, `GetTypeInfo`, `GetIDsOfNames`, `Invoke`)
/// - Slot 7  (0x1C): `get_Number`
/// - Slot 8  (0x20): `put_Number`
/// - Slot 9  (0x24): `get_Source`
/// - Slot 10 (0x28): `put_Source`
/// - Slot 11 (0x2C): `get_Description`
/// - Slot 12 (0x30): `put_Description`
/// - Slot 13 (0x34): `get_HelpFile`
/// - Slot 14 (0x38): `put_HelpFile`
/// - Slot 15 (0x3C): `get_HelpContext`
/// - Slot 16 (0x40): `put_HelpContext`
/// - Slot 17 (0x44): `Raise`
/// - Slot 18 (0x48): `Clear`
/// - Slot 19 (0x4C): `get_LastDllError`
#[repr(C)]
pub struct ErrObjectVTable {
    // IUnknown
    pub QueryInterface: unsafe extern "system" fn(this: *mut c_void, riid: *const c_void, ppv: *mut *mut c_void) -> i32,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,

    // IDispatch
    pub GetTypeInfoCount: unsafe extern "system" fn(this: *mut c_void, pctinfo: *mut u32) -> i32,
    pub GetTypeInfo: unsafe extern "system" fn(this: *mut c_void, iTInfo: u32, lcid: u32, ppTInfo: *mut *mut c_void) -> i32,
    pub GetIDsOfNames: unsafe extern "system" fn(this: *mut c_void, riid: *const c_void, rgszNames: *const *const u16, cNames: u32, lcid: u32, rgDispId: *mut i32) -> i32,
    pub Invoke: unsafe extern "system" fn(this: *mut c_void, dispIdMember: i32, riid: *const c_void, lcid: u32, wFlags: u16, pDispParams: *mut c_void, pVarResult: *mut c_void, pExcepInfo: *mut c_void, puArgErr: *mut u32) -> i32,

    // _ErrObject methods
    pub get_Number: unsafe extern "system" fn(this: *mut c_void, p_val: *mut i32) -> i32,
    pub put_Number: unsafe extern "system" fn(this: *mut c_void, val: i32) -> i32,
    pub get_Source: unsafe extern "system" fn(this: *mut c_void, pbstr: *mut Bstr) -> i32,
    pub put_Source: unsafe extern "system" fn(this: *mut c_void, bstr: Bstr) -> i32,
    pub get_Description: unsafe extern "system" fn(this: *mut c_void, pbstr: *mut Bstr) -> i32,
    pub put_Description: unsafe extern "system" fn(this: *mut c_void, bstr: Bstr) -> i32,
    pub get_HelpFile: unsafe extern "system" fn(this: *mut c_void, pbstr: *mut Bstr) -> i32,
    pub put_HelpFile: unsafe extern "system" fn(this: *mut c_void, bstr: Bstr) -> i32,
    pub get_HelpContext: unsafe extern "system" fn(this: *mut c_void, p_val: *mut i32) -> i32,
    pub put_HelpContext: unsafe extern "system" fn(this: *mut c_void, val: i32) -> i32,
    pub Raise: unsafe extern "system" fn(
        this: *mut c_void,
        number: i32,
        source: *const Variant,
        desc: *const Variant,
        helpfile: *const Variant,
        helpcontext: *const Variant,
    ) -> i32,
    pub Clear: unsafe extern "system" fn(this: *mut c_void) -> i32,
    pub get_LastDllError: unsafe extern "system" fn(this: *mut c_void, p_val: *mut i32) -> i32,
}

/// In-memory instance representing the intrinsic `Err` COM object.
#[repr(C)]
pub struct ErrObjectInstance {
    pub vtable: *const ErrObjectVTable,
}

unsafe impl Send for ErrObjectInstance {}
unsafe impl Sync for ErrObjectInstance {}

// Implementations of COM methods for ErrObject
unsafe extern "system" fn err_QueryInterface(this: *mut c_void, _riid: *const c_void, ppv: *mut *mut c_void) -> i32 {
    if !ppv.is_null() {
        *ppv = this;
    }
    S_OK
}

unsafe extern "system" fn err_AddRef(_this: *mut c_void) -> u32 {
    1
}

unsafe extern "system" fn err_Release(_this: *mut c_void) -> u32 {
    1
}

unsafe extern "system" fn err_GetTypeInfoCount(_this: *mut c_void, pctinfo: *mut u32) -> i32 {
    if !pctinfo.is_null() {
        *pctinfo = 0;
    }
    S_OK
}

unsafe extern "system" fn err_GetTypeInfo(_this: *mut c_void, _iTInfo: u32, _lcid: u32, ppTInfo: *mut *mut c_void) -> i32 {
    if !ppTInfo.is_null() {
        *ppTInfo = core::ptr::null_mut();
    }
    S_OK
}

unsafe extern "system" fn err_GetIDsOfNames(_this: *mut c_void, _riid: *const c_void, _rgszNames: *const *const u16, _cNames: u32, _lcid: u32, _rgDispId: *mut i32) -> i32 {
    S_OK
}

unsafe extern "system" fn err_Invoke(_this: *mut c_void, _dispId: i32, _riid: *const c_void, _lcid: u32, _wFlags: u16, _pDispParams: *mut c_void, _pVarResult: *mut c_void, _pExcepInfo: *mut c_void, _puArgErr: *mut u32) -> i32 {
    S_OK
}

unsafe extern "system" fn err_get_Number(_this: *mut c_void, p_val: *mut i32) -> i32 {
    if !p_val.is_null() {
        *p_val = get_vb_error_number();
    }
    S_OK
}

unsafe extern "system" fn err_put_Number(_this: *mut c_void, val: i32) -> i32 {
    set_vb_error(val, None, None);
    S_OK
}

unsafe extern "system" fn err_get_Source(_this: *mut c_void, pbstr: *mut Bstr) -> i32 {
    if !pbstr.is_null() {
        let src = get_vb_error_source();
        *pbstr = sys_alloc_string(&src);
    }
    S_OK
}

unsafe extern "system" fn err_put_Source(_this: *mut c_void, bstr: Bstr) -> i32 {
    let s = if bstr.is_null() { String::new() } else { bstr_to_string(bstr) };
    with_error_state(|state| {
        state.source = s;
    });
    S_OK
}

unsafe extern "system" fn err_get_Description(_this: *mut c_void, pbstr: *mut Bstr) -> i32 {
    if !pbstr.is_null() {
        let desc = get_vb_error_description();
        *pbstr = sys_alloc_string(&desc);
    }
    S_OK
}

unsafe extern "system" fn err_put_Description(_this: *mut c_void, bstr: Bstr) -> i32 {
    let s = if bstr.is_null() { String::new() } else { bstr_to_string(bstr) };
    with_error_state(|state| {
        state.description = s;
    });
    S_OK
}

unsafe extern "system" fn err_get_HelpFile(_this: *mut c_void, pbstr: *mut Bstr) -> i32 {
    if !pbstr.is_null() {
        let hf = with_error_state(|state| state.help_file.clone());
        *pbstr = sys_alloc_string(&hf);
    }
    S_OK
}

unsafe extern "system" fn err_put_HelpFile(_this: *mut c_void, bstr: Bstr) -> i32 {
    let s = if bstr.is_null() { String::new() } else { bstr_to_string(bstr) };
    with_error_state(|state| {
        state.help_file = s;
    });
    S_OK
}

unsafe extern "system" fn err_get_HelpContext(_this: *mut c_void, p_val: *mut i32) -> i32 {
    if !p_val.is_null() {
        *p_val = with_error_state(|state| state.help_context);
    }
    S_OK
}

unsafe extern "system" fn err_put_HelpContext(_this: *mut c_void, val: i32) -> i32 {
    with_error_state(|state| {
        state.help_context = val;
    });
    S_OK
}

unsafe extern "system" fn err_Raise(
    _this: *mut c_void,
    number: i32,
    source: *const Variant,
    desc: *const Variant,
    _helpfile: *const Variant,
    _helpcontext: *const Variant,
) -> i32 {
    let mut custom_desc: Option<String> = None;
    let mut custom_source: Option<String> = None;

    if !desc.is_null() {
        if let Some(b) = (*desc).as_bstr() {
            if !b.is_null() {
                custom_desc = Some(bstr_to_string(b));
            }
        }
    }

    if !source.is_null() {
        if let Some(b) = (*source).as_bstr() {
            if !b.is_null() {
                custom_source = Some(bstr_to_string(b));
            }
        }
    }

    set_vb_error(
        number,
        custom_desc.as_deref(),
        custom_source.as_deref(),
    );
    S_OK
}

unsafe extern "system" fn err_Clear(_this: *mut c_void) -> i32 {
    clear_vb_error();
    S_OK
}

unsafe extern "system" fn err_get_LastDllError(_this: *mut c_void, p_val: *mut i32) -> i32 {
    if !p_val.is_null() {
        *p_val = crate::pe::shims::win32_GetLastError() as i32;
    }
    S_OK
}

/// Static vtable instance for `ErrObject`.
static ERR_OBJECT_VTABLE: ErrObjectVTable = ErrObjectVTable {
    QueryInterface: err_QueryInterface,
    AddRef: err_AddRef,
    Release: err_Release,
    GetTypeInfoCount: err_GetTypeInfoCount,
    GetTypeInfo: err_GetTypeInfo,
    GetIDsOfNames: err_GetIDsOfNames,
    Invoke: err_Invoke,
    get_Number: err_get_Number,
    put_Number: err_put_Number,
    get_Source: err_get_Source,
    put_Source: err_put_Source,
    get_Description: err_get_Description,
    put_Description: err_put_Description,
    get_HelpFile: err_get_HelpFile,
    put_HelpFile: err_put_HelpFile,
    get_HelpContext: err_get_HelpContext,
    put_HelpContext: err_put_HelpContext,
    Raise: err_Raise,
    Clear: err_Clear,
    get_LastDllError: err_get_LastDllError,
};

/// Static singleton `ErrObjectInstance`.
static GLOBAL_ERR_OBJECT: ErrObjectInstance = ErrObjectInstance {
    vtable: &ERR_OBJECT_VTABLE,
};

/// Retrieves the pointer to the intrinsic `Err` COM object (`rtcErrObj`, Ordinal 685).
#[no_mangle]
pub unsafe extern "system" fn rtcErrObj() -> *mut c_void {
    &GLOBAL_ERR_OBJECT as *const ErrObjectInstance as *mut c_void
}

// ----------------------------------------------------------------------------
// Compiler Error and Object Management Helpers
// ----------------------------------------------------------------------------

/// Sets or updates the active exception handling mode (`__vbaOnError`, Ordinal 352).
///
/// # Arguments
/// * `handler` - Handler target address or control flag:
///   - `-1` (`0xFFFFFFFF`): `On Error Resume Next`
///   - `0`: `On Error GoTo 0` (reset error handler)
///   - `> 0`: `On Error GoTo <Label>`
#[no_mangle]
pub unsafe extern "system" fn __vbaOnError(handler: *mut c_void) {
    let val = handler as usize as isize;
    if val == -1 {
        set_vb_error_handler(-1);
    } else if val == 0 {
        set_vb_error_handler(0);
    } else {
        set_vb_error_handler(1);
    }
}

/// Resumes execution after an error block has been handled (`__vbaResume`, Ordinal 391).
#[no_mangle]
pub unsafe extern "system" fn __vbaResume(_mode: i32) {}

/// Standard procedure epilogue cleanup (`__vbaExitProc`, Ordinal 223).
#[no_mangle]
pub unsafe extern "system" fn __vbaExitProc() {}

/// Verifies whether an object COM call returned an error HRESULT (`__vbaHresultCheckObj`, Ordinal 258).
#[no_mangle]
pub unsafe extern "system" fn __vbaHresultCheckObj(
    hr: i32,
    _obj: *mut c_void,
    _vtable_offset: i32,
    _info: *mut c_void,
) -> i32 {
    hr
}

/// Sets an object reference variable, managing COM interface pointers (`__vbaObjSet`, Ordinal 349).
#[no_mangle]
pub unsafe extern "system" fn __vbaObjSet(dst: *mut *mut c_void, src: *mut c_void) -> *mut c_void {
    if !dst.is_null() {
        *dst = src;
    }
    src
}

/// Frees an object reference variable (`__vbaFreeObj`, Ordinal 129).
#[no_mangle]
pub unsafe extern "system" fn __vbaFreeObj(dst: *mut *mut c_void) {
    if !dst.is_null() {
        *dst = core::ptr::null_mut();
    }
}
