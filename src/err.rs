//! Structured Error Handling and `ErrObject` implementation.
//!
//! Provides the runtime state machine for Visual Basic 6 structured error handling
//! (`On Error Resume Next`, `On Error GoTo <Label>`, `On Error GoTo 0`), runtime
//! error code lookup, and the intrinsic COM `ErrObject` (`Err.Number`, `Err.Description`,
//! `Err.Source`, `Err.Clear`, `Err.Raise`, `Err.LastDllError`).

use core::ffi::c_void;

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

use std::cell::RefCell;

/// Represents an active SEH frame on the procedure execution stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbSehFrame {
    /// Frame sequence identifier.
    pub frame_id: usize,
    /// Error handling mode (-1 = Resume Next, 0 = GoTo 0 / unhandled, >0 = GoTo Label).
    pub handler_mode: i32,
    /// Target label address or landing block identifier.
    pub catch_target: usize,
}

thread_local! {
    /// Thread-local error state ensuring complete error isolation per OS thread.
    static TLS_ERROR_STATE: RefCell<VbErrorState> = RefCell::new(VbErrorState::default());
    /// Thread-local SEH frame stack tracking re-entrant procedure error handlers.
    static TLS_SEH_FRAMES: RefCell<Vec<VbSehFrame>> = const { RefCell::new(Vec::new()) };
}

/// Pushes an SEH frame onto the thread-local frame stack.
pub fn push_seh_frame(handler_mode: i32, catch_target: usize) -> usize {
    TLS_SEH_FRAMES.with(|frames| {
        let mut list = frames.borrow_mut();
        let frame_id = list.len() + 1;
        list.push(VbSehFrame {
            frame_id,
            handler_mode,
            catch_target,
        });
        set_vb_error_handler(handler_mode);
        list.len()
    })
}

/// Pops the top SEH frame from the thread-local frame stack.
pub fn pop_seh_frame() -> Option<VbSehFrame> {
    TLS_SEH_FRAMES.with(|frames| {
        let mut list = frames.borrow_mut();
        let popped = list.pop();
        if let Some(top) = list.last() {
            set_vb_error_handler(top.handler_mode);
        } else {
            set_vb_error_handler(0);
        }
        popped
    })
}

/// Retrieves the current depth of the SEH frame stack.
pub fn get_seh_frame_depth() -> usize {
    TLS_SEH_FRAMES.with(|frames| frames.borrow().len())
}

/// Clears all SEH frames for the current thread.
pub fn clear_seh_frames() {
    TLS_SEH_FRAMES.with(|frames| frames.borrow_mut().clear());
    set_vb_error_handler(0);
}

/// Unwinds the SEH frame stack until an active error handler is located.
///
/// Frames configured with `handler_mode == 0` (unhandled / On Error GoTo 0) are popped.
/// When a frame with `handler_mode != 0` is reached, unwinding stops, that frame
/// remains at the top of the stack, the thread error state is set to `error_code`,
/// and `Ok(frame)` is returned. If no handler is found, returns `Err(error_code)`.
pub fn unwind_to_handler(error_code: i32) -> Result<VbSehFrame, i32> {
    set_vb_error(error_code, None, None);
    TLS_SEH_FRAMES.with(|frames| {
        let mut list = frames.borrow_mut();
        while let Some(top) = list.last() {
            if top.handler_mode != 0 {
                let handler = top.clone();
                set_vb_error_handler(handler.handler_mode);
                return Ok(handler);
            }
            list.pop();
        }
        set_vb_error_handler(0);
        Err(error_code)
    })
}

/// Ensures error state is initialized for the calling thread and invokes closure.
pub fn with_error_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut VbErrorState) -> R,
{
    TLS_ERROR_STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        f(&mut state)
    })
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
    pub QueryInterface: unsafe extern "system" fn(
        this: *mut c_void,
        riid: *const c_void,
        ppv: *mut *mut c_void,
    ) -> i32,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,

    // IDispatch
    pub GetTypeInfoCount: unsafe extern "system" fn(this: *mut c_void, pctinfo: *mut u32) -> i32,
    pub GetTypeInfo: unsafe extern "system" fn(
        this: *mut c_void,
        iTInfo: u32,
        lcid: u32,
        ppTInfo: *mut *mut c_void,
    ) -> i32,
    pub GetIDsOfNames: unsafe extern "system" fn(
        this: *mut c_void,
        riid: *const c_void,
        rgszNames: *const *const u16,
        cNames: u32,
        lcid: u32,
        rgDispId: *mut i32,
    ) -> i32,
    pub Invoke: unsafe extern "system" fn(
        this: *mut c_void,
        dispIdMember: i32,
        riid: *const c_void,
        lcid: u32,
        wFlags: u16,
        pDispParams: *mut c_void,
        pVarResult: *mut c_void,
        pExcepInfo: *mut c_void,
        puArgErr: *mut u32,
    ) -> i32,

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
unsafe extern "system" fn err_QueryInterface(
    this: *mut c_void,
    _riid: *const c_void,
    ppv: *mut *mut c_void,
) -> i32 {
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

unsafe extern "system" fn err_GetTypeInfo(
    _this: *mut c_void,
    _iTInfo: u32,
    _lcid: u32,
    ppTInfo: *mut *mut c_void,
) -> i32 {
    if !ppTInfo.is_null() {
        *ppTInfo = core::ptr::null_mut();
    }
    S_OK
}

unsafe extern "system" fn err_GetIDsOfNames(
    _this: *mut c_void,
    _riid: *const c_void,
    _rgszNames: *const *const u16,
    _cNames: u32,
    _lcid: u32,
    _rgDispId: *mut i32,
) -> i32 {
    S_OK
}

unsafe extern "system" fn err_Invoke(
    _this: *mut c_void,
    _dispId: i32,
    _riid: *const c_void,
    _lcid: u32,
    _wFlags: u16,
    _pDispParams: *mut c_void,
    _pVarResult: *mut c_void,
    _pExcepInfo: *mut c_void,
    _puArgErr: *mut u32,
) -> i32 {
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
    let s = if bstr.is_null() {
        String::new()
    } else {
        bstr_to_string(bstr)
    };
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
    let s = if bstr.is_null() {
        String::new()
    } else {
        bstr_to_string(bstr)
    };
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
    let s = if bstr.is_null() {
        String::new()
    } else {
        bstr_to_string(bstr)
    };
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

    set_vb_error(number, custom_desc.as_deref(), custom_source.as_deref());
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
    let mode = if val == -1 {
        -1
    } else if val == 0 {
        0
    } else {
        1
    };
    set_vb_error_handler(mode);
    TLS_SEH_FRAMES.with(|frames| {
        let mut list = frames.borrow_mut();
        if let Some(top) = list.last_mut() {
            top.handler_mode = mode;
            top.catch_target = if mode > 0 { handler as usize } else { 0 };
        }
    });
}

/// Resumes execution after an error block has been handled (`__vbaResume`, Ordinal 391).
#[no_mangle]
pub unsafe extern "system" fn __vbaResume(_mode: i32) {}

/// Standard procedure epilogue cleanup (`__vbaExitProc`, Ordinal 223).
#[no_mangle]
pub unsafe extern "system" fn __vbaExitProc() {
    pop_seh_frame();
}

/// Verifies whether an object COM call returned an error HRESULT (`__vbaHresultCheckObj`, Ordinal 258).
///
/// If `hr < 0` (FAILED), translates the COM HRESULT into the appropriate VB6 runtime
/// error code (e.g. 440 for Automation error, 438 for Member not found, 429 for Can't create object),
/// populates the thread-local `ErrObject`, and preserves the raw HRESULT.
#[no_mangle]
pub unsafe extern "system" fn __vbaHresultCheckObj(
    hr: i32,
    _obj: *mut c_void,
    _vtable_offset: i32,
    _info: *mut c_void,
) -> i32 {
    if hr < 0 {
        let vb_err = match hr as u32 {
            0x80020006 => 438, // DISP_E_UNKNOWNNAME
            0x8002000E => 450, // DISP_E_PARAMNOTFOUND
            0x80040154 => 429, // REGDB_E_CLASSNOTREG
            0x80010001 => 440, // RPC_E_CALL_REJECTED / RPC_E_SERVERCALL_REJECTED
            0x8001010E => 440, // RPC_E_WRONG_THREAD
            0x800706BA => 462, // RPC_S_SERVER_UNAVAILABLE
            0x8000FFFF => 440, // E_UNEXPECTED
            _ => 440,          // Automation error
        };
        with_error_state(|state| {
            state.number = vb_err;
            state.description = format!("Automation error (0x{:08X})", hr as u32);
            state.help_context = hr;
            state.last_dll_error = hr;
        });
    }
    hr
}

/// Sets an object reference variable, managing COM interface pointers (`__vbaObjSet`, Ordinal 349).
#[no_mangle]
pub unsafe extern "system" fn __vbaObjSet(dst: *mut *mut c_void, src: *mut c_void) -> *mut c_void {
    if !dst.is_null() {
        let old = *dst;
        *dst = src;
        if !old.is_null() {
            crate::com::com_release(old);
        }
    }
    src
}

/// Sets an object reference variable and calls AddRef (`__vbaObjSetAddref`, Ordinal 350).
#[no_mangle]
pub unsafe extern "system" fn __vbaObjSetAddref(
    dst: *mut *mut c_void,
    src: *mut c_void,
) -> *mut c_void {
    if !src.is_null() {
        crate::com::com_add_ref(src);
    }
    __vbaObjSet(dst, src)
}

/// Frees an object reference variable (`__vbaFreeObj`, Ordinal 129).
#[no_mangle]
pub unsafe extern "system" fn __vbaFreeObj(dst: *mut *mut c_void) {
    if !dst.is_null() {
        let old = *dst;
        *dst = core::ptr::null_mut();
        if !old.is_null() {
            crate::com::com_release(old);
        }
    }
}

/// Compares two object references for identity (`__vbaObjIs`, Ordinal 348).
#[no_mangle]
pub unsafe extern "system" fn __vbaObjIs(p_obj: *mut c_void, p_target: *mut c_void) -> i16 {
    if p_obj == p_target {
        return -1; // VARIANT_TRUE
    }
    if p_obj.is_null() || p_target.is_null() {
        return 0; // VARIANT_FALSE
    }
    // In COM identity rules, two pointers belong to the same object iff their IUnknown pointers match
    let mut unk1: *mut c_void = core::ptr::null_mut();
    let mut unk2: *mut c_void = core::ptr::null_mut();
    let hr1 = crate::com::com_query_interface(p_obj, &crate::types::IID_IUnknown, &mut unk1);
    let hr2 = crate::com::com_query_interface(p_target, &crate::types::IID_IUnknown, &mut unk2);
    let is_same = hr1 == crate::types::S_OK && hr2 == crate::types::S_OK && unk1 == unk2;
    if !unk1.is_null() {
        crate::com::com_release(unk1);
    }
    if !unk2.is_null() {
        crate::com::com_release(unk2);
    }
    if is_same {
        -1
    } else {
        0
    }
}

/// Extracts an object interface pointer from a Variant (`__vbaObjVar`, Ordinal 351).
#[no_mangle]
pub unsafe extern "system" fn __vbaObjVar(p_var: *const Variant) -> *mut c_void {
    if p_var.is_null() {
        return core::ptr::null_mut();
    }
    let var = &*p_var;
    let vt = var.vt;
    let base_vt = vt & !crate::types::VT_BYREF;
    if base_vt != crate::types::VT_DISPATCH && base_vt != crate::types::VT_UNKNOWN {
        return core::ptr::null_mut();
    }
    if (vt & crate::types::VT_BYREF) != 0 {
        let pptr = usize::from_ne_bytes(
            var.data[..core::mem::size_of::<usize>()]
                .try_into()
                .unwrap(),
        ) as *const *mut c_void;
        if !pptr.is_null() {
            *pptr
        } else {
            core::ptr::null_mut()
        }
    } else {
        usize::from_ne_bytes(
            var.data[..core::mem::size_of::<usize>()]
                .try_into()
                .unwrap(),
        ) as *mut c_void
    }
}
