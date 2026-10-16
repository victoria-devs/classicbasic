// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! `ADODB.Recordset`, `Fields`, and `Field` COM Automation Components (`msado15.dll`).
//!
//! Provides in-memory disconnected recordsets, metadata definition (`Fields.Append`),
//! and row-oriented data navigation (`AddNew`, `Update`, `MoveFirst`, `MoveNext`, `EOF`)
//! for Visual Basic 6.0 applications running on Linux.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crate::com::{
    DispParams, IDispatchVtbl, DISPID_UNKNOWN, DISP_E_MEMBERNOTFOUND, DISP_E_UNKNOWNNAME,
};
use crate::types::{
    bstr_to_string, sys_alloc_string, Bstr, E_NOINTERFACE, E_POINTER, HResult, Iid, Variant,
    IID_IDispatch, IID_IUnknown, S_OK, VT_BOOL, VT_BSTR, VT_DISPATCH, VT_I4,
};

// ----------------------------------------------------------------------------
// Shared Recordset State
// ----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct FieldDef {
    pub name: String,
    pub field_type: i32,
    pub defined_size: i32,
}

#[derive(Default, Debug)]
pub struct RecordsetState {
    pub field_defs: Vec<FieldDef>,
    pub rows: Vec<Vec<Variant>>,
    pub current_row: isize,
    pub is_open: bool,
}

impl RecordsetState {
    pub fn find_field_index(&self, name: &str) -> Option<usize> {
        let name_lower = name.to_ascii_lowercase();
        self.field_defs
            .iter()
            .position(|f| f.name.to_ascii_lowercase() == name_lower)
    }
}

// ----------------------------------------------------------------------------
// Parameter Extraction Helpers
// ----------------------------------------------------------------------------

unsafe fn extract_arg(p_params: *mut DispParams, idx: usize) -> Option<Variant> {
    if p_params.is_null() {
        return None;
    }
    let params = &*p_params;
    if idx >= params.cArgs as usize {
        return None;
    }
    let rev_idx = params.cArgs as usize - 1 - idx;
    Some(*params.rgvarg.add(rev_idx))
}

unsafe fn extract_string_arg(p_params: *mut DispParams, idx: usize) -> Option<String> {
    let var = extract_arg(p_params, idx)?;
    if var.vt == VT_BSTR {
        let bstr = core::ptr::read_unaligned(var.data.as_ptr() as *const Bstr);
        Some(bstr_to_string(bstr))
    } else if var.vt == (crate::types::VT_BYREF | crate::types::VT_VARIANT) {
        let inner = core::ptr::read_unaligned(var.data.as_ptr() as *const *mut Variant);
        if !inner.is_null() && (*inner).vt == VT_BSTR {
            let bstr = core::ptr::read_unaligned((*inner).data.as_ptr() as *const Bstr);
            Some(bstr_to_string(bstr))
        } else {
            None
        }
    } else {
        None
    }
}

unsafe fn extract_i32_arg(p_params: *mut DispParams, idx: usize, default: i32) -> i32 {
    if let Some(var) = extract_arg(p_params, idx) {
        if var.vt == VT_I4 {
            core::ptr::read_unaligned(var.data.as_ptr() as *const i32)
        } else if var.vt == crate::types::VT_I2 {
            core::ptr::read_unaligned(var.data.as_ptr() as *const i16) as i32
        } else {
            default
        }
    } else {
        default
    }
}

// ----------------------------------------------------------------------------
// RecordsetObject
// ----------------------------------------------------------------------------

const DISPID_RS_DEFAULT: i32 = 0;
const DISPID_RS_FIELDS: i32 = 6000;
const DISPID_RS_OPEN: i32 = 6001;
const DISPID_RS_CLOSE: i32 = 6002;
const DISPID_RS_ADDNEW: i32 = 6003;
const DISPID_RS_UPDATE: i32 = 6004;
const DISPID_RS_RECORDCOUNT: i32 = 6005;
const DISPID_RS_MOVEFIRST: i32 = 6006;
const DISPID_RS_MOVENEXT: i32 = 6007;
const DISPID_RS_MOVEPREVIOUS: i32 = 6008;
const DISPID_RS_MOVELAST: i32 = 6009;
const DISPID_RS_EOF: i32 = 6010;
const DISPID_RS_BOF: i32 = 6011;

#[repr(C)]
pub struct RecordsetObject {
    pub vtable: *const IDispatchVtbl,
    pub ref_count: AtomicU32,
    pub state: Arc<Mutex<RecordsetState>>,
}

unsafe extern "system" fn rs_QueryInterface(
    this: *mut core::ffi::c_void,
    riid: *const Iid,
    ppv: *mut *mut core::ffi::c_void,
) -> HResult {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    let iid = *riid;
    if iid == IID_IUnknown || iid == IID_IDispatch {
        *ppv = this;
        rs_AddRef(this);
        return S_OK;
    }
    *ppv = core::ptr::null_mut();
    E_NOINTERFACE
}

unsafe extern "system" fn rs_AddRef(this: *mut core::ffi::c_void) -> u32 {
    let obj = &*(this as *mut RecordsetObject);
    obj.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn rs_Release(this: *mut core::ffi::c_void) -> u32 {
    let obj_ptr = this as *mut RecordsetObject;
    let old_count = (*obj_ptr).ref_count.fetch_sub(1, Ordering::SeqCst);
    if old_count == 1 {
        let _ = Box::from_raw(obj_ptr);
        0
    } else {
        old_count - 1
    }
}

unsafe extern "system" fn rs_GetTypeInfoCount(
    _this: *mut core::ffi::c_void,
    pctinfo: *mut u32,
) -> HResult {
    if !pctinfo.is_null() {
        *pctinfo = 1;
    }
    S_OK
}

unsafe extern "system" fn rs_GetTypeInfo(
    _this: *mut core::ffi::c_void,
    _iTInfo: u32,
    _lcid: u32,
    ppTInfo: *mut *mut core::ffi::c_void,
) -> HResult {
    if !ppTInfo.is_null() {
        *ppTInfo = core::ptr::null_mut();
    }
    S_OK
}

unsafe extern "system" fn rs_GetIDsOfNames(
    _this: *mut core::ffi::c_void,
    _riid: *const Iid,
    rgszNames: *const *const u16,
    cNames: u32,
    _lcid: u32,
    rgDispId: *mut i32,
) -> HResult {
    if rgszNames.is_null() || rgDispId.is_null() || cNames == 0 {
        return E_POINTER;
    }

    let mut hr = S_OK;
    for i in 0..cNames as usize {
        let name_ptr = *rgszNames.add(i);
        let name = crate::types::utf16_ptr_to_string(name_ptr).to_ascii_lowercase();

        let dispid = match name.as_str() {
            "fields" => DISPID_RS_FIELDS,
            "open" => DISPID_RS_OPEN,
            "close" => DISPID_RS_CLOSE,
            "addnew" => DISPID_RS_ADDNEW,
            "update" => DISPID_RS_UPDATE,
            "recordcount" => DISPID_RS_RECORDCOUNT,
            "movefirst" => DISPID_RS_MOVEFIRST,
            "movenext" => DISPID_RS_MOVENEXT,
            "moveprevious" => DISPID_RS_MOVEPREVIOUS,
            "movelast" => DISPID_RS_MOVELAST,
            "eof" => DISPID_RS_EOF,
            "bof" => DISPID_RS_BOF,
            _ => {
                hr = DISP_E_UNKNOWNNAME;
                DISPID_UNKNOWN
            }
        };
        *rgDispId.add(i) = dispid;
    }

    hr
}

unsafe extern "system" fn rs_Invoke(
    this: *mut core::ffi::c_void,
    dispIdMember: i32,
    _riid: *const Iid,
    _lcid: u32,
    _wFlags: u16,
    _pDispParams: *mut core::ffi::c_void,
    pVarResult: *mut core::ffi::c_void,
    _pExcepInfo: *mut core::ffi::c_void,
    _puArgErr: *mut u32,
) -> HResult {
    let obj = &*(this as *mut RecordsetObject);
    let res = pVarResult as *mut Variant;

    if !res.is_null() {
        *res = Variant::empty();
    }

    let mut state = match obj.state.lock() {
        Ok(s) => s,
        Err(_) => return S_OK,
    };

    match dispIdMember {
        DISPID_RS_FIELDS => {
            let fields_obj = FieldsObject::new(Arc::clone(&obj.state));
            if !res.is_null() {
                let r = &mut *res;
                r.vt = VT_DISPATCH;
                let ptr_bytes = (fields_obj as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_RS_OPEN => {
            state.is_open = true;
            state.current_row = if state.rows.is_empty() { -1 } else { 0 };
            S_OK
        }
        DISPID_RS_CLOSE => {
            state.is_open = false;
            S_OK
        }
        DISPID_RS_ADDNEW => {
            let num_fields = state.field_defs.len();
            let new_row = vec![Variant::empty(); num_fields];
            state.rows.push(new_row);
            state.current_row = state.rows.len() as isize - 1;
            S_OK
        }
        DISPID_RS_UPDATE => {
            // In disconnected in-memory mode, changes are written in-place to the active row
            S_OK
        }
        DISPID_RS_RECORDCOUNT => {
            if !res.is_null() {
                let r = &mut *res;
                r.vt = VT_I4;
                let count = state.rows.len() as i32;
                r.data[0..4].copy_from_slice(&count.to_ne_bytes());
            }
            S_OK
        }
        DISPID_RS_MOVEFIRST => {
            state.current_row = if state.rows.is_empty() { -1 } else { 0 };
            S_OK
        }
        DISPID_RS_MOVENEXT => {
            if state.current_row < state.rows.len() as isize {
                state.current_row += 1;
            }
            S_OK
        }
        DISPID_RS_MOVEPREVIOUS => {
            if state.current_row >= 0 {
                state.current_row -= 1;
            }
            S_OK
        }
        DISPID_RS_MOVELAST => {
            state.current_row = if state.rows.is_empty() {
                -1
            } else {
                state.rows.len() as isize - 1
            };
            S_OK
        }
        DISPID_RS_EOF => {
            let is_eof = state.current_row < 0 || state.current_row >= state.rows.len() as isize;
            if !res.is_null() {
                let r = &mut *res;
                r.vt = VT_BOOL;
                let b_val: i16 = if is_eof { -1 } else { 0 };
                r.data[0..2].copy_from_slice(&b_val.to_ne_bytes());
            }
            S_OK
        }
        DISPID_RS_BOF => {
            let is_bof = state.current_row < 0;
            if !res.is_null() {
                let r = &mut *res;
                r.vt = VT_BOOL;
                let b_val: i16 = if is_bof { -1 } else { 0 };
                r.data[0..2].copy_from_slice(&b_val.to_ne_bytes());
            }
            S_OK
        }
        DISPID_RS_DEFAULT => {
            let fields_obj = FieldsObject::new(Arc::clone(&obj.state));
            let hr = fields_Invoke(
                fields_obj as *mut core::ffi::c_void,
                DISPID_FIELDS_DEFAULT,
                _riid,
                _lcid,
                _wFlags,
                _pDispParams,
                pVarResult,
                _pExcepInfo,
                _puArgErr,
            );
            fields_Release(fields_obj as *mut core::ffi::c_void);
            hr
        }
        _ => DISP_E_MEMBERNOTFOUND,
    }
}

static RS_VTBL: IDispatchVtbl = IDispatchVtbl {
    QueryInterface: rs_QueryInterface,
    AddRef: rs_AddRef,
    Release: rs_Release,
    GetTypeInfoCount: rs_GetTypeInfoCount,
    GetTypeInfo: rs_GetTypeInfo,
    GetIDsOfNames: rs_GetIDsOfNames,
    Invoke: rs_Invoke,
};

impl RecordsetObject {
    pub fn new() -> *mut RecordsetObject {
        let boxed = Box::new(RecordsetObject {
            vtable: &RS_VTBL,
            ref_count: AtomicU32::new(1),
            state: Arc::new(Mutex::new(RecordsetState::default())),
        });
        Box::into_raw(boxed)
    }
}

// ----------------------------------------------------------------------------
// FieldsObject
// ----------------------------------------------------------------------------

const DISPID_FIELDS_DEFAULT: i32 = 0;
const DISPID_FIELDS_ITEM: i32 = 7000;
const DISPID_FIELDS_COUNT: i32 = 7001;
const DISPID_FIELDS_APPEND: i32 = 7002;

#[repr(C)]
pub struct FieldsObject {
    pub vtable: *const IDispatchVtbl,
    pub ref_count: AtomicU32,
    pub state: Arc<Mutex<RecordsetState>>,
}

unsafe extern "system" fn fields_QueryInterface(
    this: *mut core::ffi::c_void,
    riid: *const Iid,
    ppv: *mut *mut core::ffi::c_void,
) -> HResult {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    let iid = *riid;
    if iid == IID_IUnknown || iid == IID_IDispatch {
        *ppv = this;
        fields_AddRef(this);
        return S_OK;
    }
    *ppv = core::ptr::null_mut();
    E_NOINTERFACE
}

unsafe extern "system" fn fields_AddRef(this: *mut core::ffi::c_void) -> u32 {
    let obj = &*(this as *mut FieldsObject);
    obj.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn fields_Release(this: *mut core::ffi::c_void) -> u32 {
    let obj_ptr = this as *mut FieldsObject;
    let old_count = (*obj_ptr).ref_count.fetch_sub(1, Ordering::SeqCst);
    if old_count == 1 {
        let _ = Box::from_raw(obj_ptr);
        0
    } else {
        old_count - 1
    }
}

unsafe extern "system" fn fields_GetTypeInfoCount(
    _this: *mut core::ffi::c_void,
    pctinfo: *mut u32,
) -> HResult {
    if !pctinfo.is_null() {
        *pctinfo = 1;
    }
    S_OK
}

unsafe extern "system" fn fields_GetTypeInfo(
    _this: *mut core::ffi::c_void,
    _iTInfo: u32,
    _lcid: u32,
    ppTInfo: *mut *mut core::ffi::c_void,
) -> HResult {
    if !ppTInfo.is_null() {
        *ppTInfo = core::ptr::null_mut();
    }
    S_OK
}

unsafe extern "system" fn fields_GetIDsOfNames(
    _this: *mut core::ffi::c_void,
    _riid: *const Iid,
    rgszNames: *const *const u16,
    cNames: u32,
    _lcid: u32,
    rgDispId: *mut i32,
) -> HResult {
    if rgszNames.is_null() || rgDispId.is_null() || cNames == 0 {
        return E_POINTER;
    }

    let mut hr = S_OK;
    for i in 0..cNames as usize {
        let name_ptr = *rgszNames.add(i);
        let name = crate::types::utf16_ptr_to_string(name_ptr).to_ascii_lowercase();

        let dispid = match name.as_str() {
            "item" => DISPID_FIELDS_ITEM,
            "count" => DISPID_FIELDS_COUNT,
            "append" => DISPID_FIELDS_APPEND,
            _ => {
                hr = DISP_E_UNKNOWNNAME;
                DISPID_UNKNOWN
            }
        };
        *rgDispId.add(i) = dispid;
    }

    hr
}

unsafe extern "system" fn fields_Invoke(
    this: *mut core::ffi::c_void,
    dispIdMember: i32,
    _riid: *const Iid,
    _lcid: u32,
    _wFlags: u16,
    pDispParams: *mut core::ffi::c_void,
    pVarResult: *mut core::ffi::c_void,
    _pExcepInfo: *mut core::ffi::c_void,
    _puArgErr: *mut u32,
) -> HResult {
    let obj = &*(this as *mut FieldsObject);
    let params = pDispParams as *mut DispParams;
    let res = pVarResult as *mut Variant;

    if !res.is_null() {
        *res = Variant::empty();
    }

    let mut state = match obj.state.lock() {
        Ok(s) => s,
        Err(_) => return S_OK,
    };

    match dispIdMember {
        DISPID_FIELDS_COUNT => {
            if !res.is_null() {
                let r = &mut *res;
                r.vt = VT_I4;
                let count = state.field_defs.len() as i32;
                r.data[0..4].copy_from_slice(&count.to_ne_bytes());
            }
            S_OK
        }
        DISPID_FIELDS_APPEND => {
            let name = extract_string_arg(params, 0).unwrap_or_default();
            let ftype = extract_i32_arg(params, 1, 200); // default adVarChar
            let size = extract_i32_arg(params, 2, 50);

            state.field_defs.push(FieldDef {
                name,
                field_type: ftype,
                defined_size: size,
            });
            S_OK
        }
        DISPID_FIELDS_DEFAULT | DISPID_FIELDS_ITEM => {
            let mut field_idx = None;
            if let Some(arg_str) = extract_string_arg(params, 0) {
                field_idx = state.find_field_index(&arg_str);
            } else if let Some(arg_var) = extract_arg(params, 0) {
                if arg_var.vt == VT_I4 || arg_var.vt == crate::types::VT_I2 {
                    let idx = core::ptr::read_unaligned(arg_var.data.as_ptr() as *const i32) as usize;
                    if idx < state.field_defs.len() {
                        field_idx = Some(idx);
                    }
                }
            }

            if let Some(idx) = field_idx {
                let field_obj = FieldObject::new(Arc::clone(&obj.state), idx);
                if !res.is_null() {
                    let r = &mut *res;
                    r.vt = VT_DISPATCH;
                    let ptr_bytes = (field_obj as usize).to_ne_bytes();
                    r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
                }
                S_OK
            } else {
                crate::err::set_vb_error(3265, Some("Item cannot be found in the collection corresponding to the requested name or ordinal"), None);
                DISP_E_MEMBERNOTFOUND
            }
        }
        _ => DISP_E_MEMBERNOTFOUND,
    }
}

static FIELDS_VTBL: IDispatchVtbl = IDispatchVtbl {
    QueryInterface: fields_QueryInterface,
    AddRef: fields_AddRef,
    Release: fields_Release,
    GetTypeInfoCount: fields_GetTypeInfoCount,
    GetTypeInfo: fields_GetTypeInfo,
    GetIDsOfNames: fields_GetIDsOfNames,
    Invoke: fields_Invoke,
};

impl FieldsObject {
    pub fn new(state: Arc<Mutex<RecordsetState>>) -> *mut FieldsObject {
        let boxed = Box::new(FieldsObject {
            vtable: &FIELDS_VTBL,
            ref_count: AtomicU32::new(1),
            state,
        });
        Box::into_raw(boxed)
    }
}

// ----------------------------------------------------------------------------
// FieldObject
// ----------------------------------------------------------------------------

const DISPID_FIELD_DEFAULT: i32 = 0;
const DISPID_FIELD_VALUE: i32 = 8000;
const DISPID_FIELD_NAME: i32 = 8001;
const DISPID_FIELD_TYPE: i32 = 8002;
const DISPID_FIELD_DEFINEDSIZE: i32 = 8003;

#[repr(C)]
pub struct FieldObject {
    pub vtable: *const IDispatchVtbl,
    pub ref_count: AtomicU32,
    pub state: Arc<Mutex<RecordsetState>>,
    pub field_index: usize,
}

unsafe extern "system" fn field_QueryInterface(
    this: *mut core::ffi::c_void,
    riid: *const Iid,
    ppv: *mut *mut core::ffi::c_void,
) -> HResult {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    let iid = *riid;
    if iid == IID_IUnknown || iid == IID_IDispatch {
        *ppv = this;
        field_AddRef(this);
        return S_OK;
    }
    *ppv = core::ptr::null_mut();
    E_NOINTERFACE
}

unsafe extern "system" fn field_AddRef(this: *mut core::ffi::c_void) -> u32 {
    let obj = &*(this as *mut FieldObject);
    obj.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn field_Release(this: *mut core::ffi::c_void) -> u32 {
    let obj_ptr = this as *mut FieldObject;
    let old_count = (*obj_ptr).ref_count.fetch_sub(1, Ordering::SeqCst);
    if old_count == 1 {
        let _ = Box::from_raw(obj_ptr);
        0
    } else {
        old_count - 1
    }
}

unsafe extern "system" fn field_GetTypeInfoCount(
    _this: *mut core::ffi::c_void,
    pctinfo: *mut u32,
) -> HResult {
    if !pctinfo.is_null() {
        *pctinfo = 1;
    }
    S_OK
}

unsafe extern "system" fn field_GetTypeInfo(
    _this: *mut core::ffi::c_void,
    _iTInfo: u32,
    _lcid: u32,
    ppTInfo: *mut *mut core::ffi::c_void,
) -> HResult {
    if !ppTInfo.is_null() {
        *ppTInfo = core::ptr::null_mut();
    }
    S_OK
}

unsafe extern "system" fn field_GetIDsOfNames(
    _this: *mut core::ffi::c_void,
    _riid: *const Iid,
    rgszNames: *const *const u16,
    cNames: u32,
    _lcid: u32,
    rgDispId: *mut i32,
) -> HResult {
    if rgszNames.is_null() || rgDispId.is_null() || cNames == 0 {
        return E_POINTER;
    }

    let mut hr = S_OK;
    for i in 0..cNames as usize {
        let name_ptr = *rgszNames.add(i);
        let name = crate::types::utf16_ptr_to_string(name_ptr).to_ascii_lowercase();

        let dispid = match name.as_str() {
            "value" => DISPID_FIELD_VALUE,
            "name" => DISPID_FIELD_NAME,
            "type" => DISPID_FIELD_TYPE,
            "definedsize" => DISPID_FIELD_DEFINEDSIZE,
            _ => {
                hr = DISP_E_UNKNOWNNAME;
                DISPID_UNKNOWN
            }
        };
        *rgDispId.add(i) = dispid;
    }

    hr
}

unsafe extern "system" fn field_Invoke(
    this: *mut core::ffi::c_void,
    dispIdMember: i32,
    _riid: *const Iid,
    _lcid: u32,
    wFlags: u16,
    pDispParams: *mut core::ffi::c_void,
    pVarResult: *mut core::ffi::c_void,
    _pExcepInfo: *mut core::ffi::c_void,
    _puArgErr: *mut u32,
) -> HResult {
    let obj = &*(this as *mut FieldObject);
    let params = pDispParams as *mut DispParams;
    let res = pVarResult as *mut Variant;

    if !res.is_null() {
        *res = Variant::empty();
    }

    let mut state = match obj.state.lock() {
        Ok(s) => s,
        Err(_) => return S_OK,
    };

    match dispIdMember {
        DISPID_FIELD_DEFAULT | DISPID_FIELD_VALUE => {
            if (wFlags & (crate::com::DISPATCH_PROPERTYPUT | crate::com::DISPATCH_PROPERTYPUTREF))
                != 0
            {
                // Property Put: update the active row value
                if !params.is_null() && (*params).cArgs > 0 {
                    let put_val = core::ptr::read_unaligned((*params).rgvarg);
                    let row_idx = state.current_row;
                    if row_idx >= 0 && (row_idx as usize) < state.rows.len() {
                        let row = &mut state.rows[row_idx as usize];
                        if obj.field_index < row.len() {
                            row[obj.field_index] = put_val;
                        }
                    }
                }
            } else {
                // Property Get: read the active row value
                let row_idx = state.current_row;
                if row_idx >= 0 && (row_idx as usize) < state.rows.len() {
                    let row = &state.rows[row_idx as usize];
                    if obj.field_index < row.len() {
                        if !res.is_null() {
                            *res = row[obj.field_index];
                        }
                    }
                }
            }
            S_OK
        }
        DISPID_FIELD_NAME => {
            if obj.field_index < state.field_defs.len() {
                let name = &state.field_defs[obj.field_index].name;
                if !res.is_null() {
                    let bstr = sys_alloc_string(name);
                    let r = &mut *res;
                    r.vt = VT_BSTR;
                    let ptr_bytes = (bstr as usize).to_ne_bytes();
                    r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
                }
            }
            S_OK
        }
        DISPID_FIELD_TYPE => {
            if obj.field_index < state.field_defs.len() {
                let ftype = state.field_defs[obj.field_index].field_type;
                if !res.is_null() {
                    let r = &mut *res;
                    r.vt = VT_I4;
                    r.data[0..4].copy_from_slice(&ftype.to_ne_bytes());
                }
            }
            S_OK
        }
        DISPID_FIELD_DEFINEDSIZE => {
            if obj.field_index < state.field_defs.len() {
                let size = state.field_defs[obj.field_index].defined_size;
                if !res.is_null() {
                    let r = &mut *res;
                    r.vt = VT_I4;
                    r.data[0..4].copy_from_slice(&size.to_ne_bytes());
                }
            }
            S_OK
        }
        _ => DISP_E_MEMBERNOTFOUND,
    }
}

static FIELD_VTBL: IDispatchVtbl = IDispatchVtbl {
    QueryInterface: field_QueryInterface,
    AddRef: field_AddRef,
    Release: field_Release,
    GetTypeInfoCount: field_GetTypeInfoCount,
    GetTypeInfo: field_GetTypeInfo,
    GetIDsOfNames: field_GetIDsOfNames,
    Invoke: field_Invoke,
};

impl FieldObject {
    pub fn new(state: Arc<Mutex<RecordsetState>>, field_index: usize) -> *mut FieldObject {
        let boxed = Box::new(FieldObject {
            vtable: &FIELD_VTBL,
            ref_count: AtomicU32::new(1),
            state,
            field_index,
        });
        Box::into_raw(boxed)
    }
}
