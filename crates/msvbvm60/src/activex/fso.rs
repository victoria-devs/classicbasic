// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! `Scripting.FileSystemObject` COM Automation Component.
//!
//! Provides file and directory manipulation, path resolution, and sequential text file I/O
//! (`TextStream`) for Visual Basic 6.0 applications running on Linux.

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use crate::com::{
    DispParams, IDispatchVtbl, DISPID_UNKNOWN, DISP_E_MEMBERNOTFOUND, DISP_E_UNKNOWNNAME,
};
use crate::types::{
    bstr_to_string, sys_alloc_string, Bstr, DISP_E_BADINDEX, E_NOINTERFACE, E_POINTER, HResult,
    Iid, Variant, IID_IDispatch, IID_IUnknown, S_OK, VT_BOOL, VT_BSTR, VT_DISPATCH, VT_I4,
};

// ----------------------------------------------------------------------------
// Helper Functions for Parameter Extraction
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

unsafe fn extract_bool_arg(p_params: *mut DispParams, idx: usize, default: bool) -> bool {
    if let Some(var) = extract_arg(p_params, idx) {
        if var.vt == VT_BOOL {
            let val = core::ptr::read_unaligned(var.data.as_ptr() as *const i16);
            val != 0
        } else if var.vt == VT_I4 {
            let val = core::ptr::read_unaligned(var.data.as_ptr() as *const i32);
            val != 0
        } else {
            default
        }
    } else {
        default
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
// FileSystemObject
// ----------------------------------------------------------------------------

const DISPID_FSO_BUILDPATH: i32 = 1000;
const DISPID_FSO_GETDRIVENAME: i32 = 1001;
const DISPID_FSO_GETPARENTFOLDERNAME: i32 = 1002;
const DISPID_FSO_GETFILENAME: i32 = 1003;
const DISPID_FSO_GETBASENAME: i32 = 1004;
const DISPID_FSO_GETEXTENSIONNAME: i32 = 1005;
const DISPID_FSO_GETABSOLUTEPATHNAME: i32 = 1006;
const DISPID_FSO_FILEEXISTS: i32 = 1007;
const DISPID_FSO_FOLDEREXISTS: i32 = 1008;
const DISPID_FSO_CREATETEXTFILE: i32 = 1009;
const DISPID_FSO_OPENTEXTFILE: i32 = 1010;
const DISPID_FSO_GETFILE: i32 = 1011;
const DISPID_FSO_DELETEFILE: i32 = 1012;
const DISPID_FSO_DELETEFOLDER: i32 = 1013;
const DISPID_FSO_GETSPECIALFOLDER: i32 = 1014;

#[repr(C)]
pub struct FileSystemObject {
    pub vtable: *const IDispatchVtbl,
    pub ref_count: AtomicU32,
}

unsafe extern "system" fn fso_QueryInterface(
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
        fso_AddRef(this);
        return S_OK;
    }
    *ppv = core::ptr::null_mut();
    E_NOINTERFACE
}

unsafe extern "system" fn fso_AddRef(this: *mut core::ffi::c_void) -> u32 {
    let obj = &*(this as *mut FileSystemObject);
    obj.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn fso_Release(this: *mut core::ffi::c_void) -> u32 {
    let obj_ptr = this as *mut FileSystemObject;
    let old_count = (*obj_ptr).ref_count.fetch_sub(1, Ordering::SeqCst);
    if old_count == 1 {
        let _ = Box::from_raw(obj_ptr);
        0
    } else {
        old_count - 1
    }
}

unsafe extern "system" fn fso_GetTypeInfoCount(
    _this: *mut core::ffi::c_void,
    pctinfo: *mut u32,
) -> HResult {
    if !pctinfo.is_null() {
        *pctinfo = 1;
    }
    S_OK
}

unsafe extern "system" fn fso_GetTypeInfo(
    _this: *mut core::ffi::c_void,
    iTInfo: u32,
    _lcid: u32,
    ppTInfo: *mut *mut core::ffi::c_void,
) -> HResult {
    if ppTInfo.is_null() {
        return E_POINTER;
    }
    if iTInfo != 0 {
        *ppTInfo = core::ptr::null_mut();
        return DISP_E_BADINDEX;
    }
    *ppTInfo = crate::com::NativeTypeInfo::new(
        "FileSystemObject",
        crate::types::CLSID_FileSystemObject,
        &[
            "BuildPath",
            "GetDriveName",
            "GetParentFolderName",
            "GetFileName",
            "GetBaseName",
            "GetExtensionName",
            "GetAbsolutePathName",
            "FileExists",
            "FolderExists",
            "CreateTextFile",
            "OpenTextFile",
            "GetFile",
            "DeleteFile",
            "DeleteFolder",
        ],
    );
    S_OK
}

unsafe extern "system" fn fso_GetIDsOfNames(
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
            "buildpath" => DISPID_FSO_BUILDPATH,
            "getdrivename" => DISPID_FSO_GETDRIVENAME,
            "getparentfoldername" => DISPID_FSO_GETPARENTFOLDERNAME,
            "getfilename" => DISPID_FSO_GETFILENAME,
            "getbasename" => DISPID_FSO_GETBASENAME,
            "getextensionname" => DISPID_FSO_GETEXTENSIONNAME,
            "getabsolutepathname" => DISPID_FSO_GETABSOLUTEPATHNAME,
            "fileexists" => DISPID_FSO_FILEEXISTS,
            "folderexists" => DISPID_FSO_FOLDEREXISTS,
            "createtextfile" => DISPID_FSO_CREATETEXTFILE,
            "opentextfile" => DISPID_FSO_OPENTEXTFILE,
            "getfile" => DISPID_FSO_GETFILE,
            "deletefile" => DISPID_FSO_DELETEFILE,
            "deletefolder" => DISPID_FSO_DELETEFOLDER,
            "getspecialfolder" => DISPID_FSO_GETSPECIALFOLDER,
            _ => {
                hr = DISP_E_UNKNOWNNAME;
                DISPID_UNKNOWN
            }
        };
        *rgDispId.add(i) = dispid;
    }

    hr
}

unsafe extern "system" fn fso_Invoke(
    _this: *mut core::ffi::c_void,
    dispIdMember: i32,
    _riid: *const Iid,
    _lcid: u32,
    _wFlags: u16,
    pDispParams: *mut core::ffi::c_void,
    pVarResult: *mut core::ffi::c_void,
    _pExcepInfo: *mut core::ffi::c_void,
    _puArgErr: *mut u32,
) -> HResult {
    let params = pDispParams as *mut DispParams;
    let res = pVarResult as *mut Variant;

    if !res.is_null() {
        *res = Variant::empty();
    }

    match dispIdMember {
        DISPID_FSO_BUILDPATH => {
            let path = extract_string_arg(params, 0).unwrap_or_default();
            let name = extract_string_arg(params, 1).unwrap_or_default();
            let combined = if path.is_empty() {
                name
            } else if name.is_empty() {
                path
            } else if path.ends_with('\\') || path.ends_with('/') {
                format!("{}{}", path, name)
            } else {
                format!("{}\\{}", path, name)
            };
            if !res.is_null() {
                let bstr = sys_alloc_string(&combined);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_FSO_GETDRIVENAME => {
            let path = extract_string_arg(params, 0).unwrap_or_default();
            let drive = if path.len() >= 2 && path.as_bytes()[1] == b':' {
                path[0..2].to_string()
            } else {
                String::new()
            };
            if !res.is_null() {
                let bstr = sys_alloc_string(&drive);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_FSO_GETPARENTFOLDERNAME => {
            let path = extract_string_arg(params, 0).unwrap_or_default();
            let trimmed = path.trim_end_matches(['\\', '/']);
            let parent = if let Some(idx) = trimmed.rfind(['\\', '/']) {
                trimmed[..idx].to_string()
            } else {
                String::new()
            };
            if !res.is_null() {
                let bstr = sys_alloc_string(&parent);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_FSO_GETFILENAME => {
            let path = extract_string_arg(params, 0).unwrap_or_default();
            let name = if let Some(idx) = path.rfind(['\\', '/']) {
                path[idx + 1..].to_string()
            } else {
                path
            };
            if !res.is_null() {
                let bstr = sys_alloc_string(&name);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_FSO_GETBASENAME => {
            let path = extract_string_arg(params, 0).unwrap_or_default();
            let name = if let Some(idx) = path.rfind(['\\', '/']) {
                &path[idx + 1..]
            } else {
                &path
            };
            let base = if let Some(dot_idx) = name.rfind('.') {
                &name[..dot_idx]
            } else {
                name
            };
            if !res.is_null() {
                let bstr = sys_alloc_string(base);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_FSO_GETEXTENSIONNAME => {
            let path = extract_string_arg(params, 0).unwrap_or_default();
            let name = if let Some(idx) = path.rfind(['\\', '/']) {
                &path[idx + 1..]
            } else {
                &path
            };
            let ext = if let Some(dot_idx) = name.rfind('.') {
                &name[dot_idx + 1..]
            } else {
                ""
            };
            if !res.is_null() {
                let bstr = sys_alloc_string(ext);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_FSO_GETABSOLUTEPATHNAME => {
            let path = extract_string_arg(params, 0).unwrap_or_default();
            let resolved = crate::vba::resolve_virtual_path(&path);
            if !res.is_null() {
                let bstr = sys_alloc_string(&resolved);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_FSO_FILEEXISTS => {
            let path = extract_string_arg(params, 0).unwrap_or_default();
            let resolved = crate::vba::resolve_virtual_path(&path);
            let exists = Path::new(&resolved).is_file();
            if !res.is_null() {
                let r = &mut *res;
                r.vt = VT_BOOL;
                let b_val: i16 = if exists { -1 } else { 0 };
                r.data[0..2].copy_from_slice(&b_val.to_ne_bytes());
            }
            S_OK
        }
        DISPID_FSO_FOLDEREXISTS => {
            let path = extract_string_arg(params, 0).unwrap_or_default();
            let resolved = crate::vba::resolve_virtual_path(&path);
            let exists = Path::new(&resolved).is_dir();
            if !res.is_null() {
                let r = &mut *res;
                r.vt = VT_BOOL;
                let b_val: i16 = if exists { -1 } else { 0 };
                r.data[0..2].copy_from_slice(&b_val.to_ne_bytes());
            }
            S_OK
        }
        DISPID_FSO_CREATETEXTFILE => {
            let path = extract_string_arg(params, 0).unwrap_or_default();
            let overwrite = extract_bool_arg(params, 1, true);
            let resolved = crate::vba::resolve_virtual_path(&path);

            if !overwrite && Path::new(&resolved).exists() {
                crate::err::set_vb_error(58, Some("File already exists"), None);
                return S_OK;
            }

            // Create or truncate file
            let _ = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&resolved);

            let ts = TextStreamObject::new(&resolved, 2);
            if !res.is_null() {
                let r = &mut *res;
                r.vt = VT_DISPATCH;
                let ptr_bytes = (ts as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_FSO_OPENTEXTFILE => {
            let path = extract_string_arg(params, 0).unwrap_or_default();
            let iomode = extract_i32_arg(params, 1, 1);
            let resolved = crate::vba::resolve_virtual_path(&path);

            let ts = TextStreamObject::new(&resolved, iomode);
            if !res.is_null() {
                let r = &mut *res;
                r.vt = VT_DISPATCH;
                let ptr_bytes = (ts as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_FSO_GETFILE => {
            let path = extract_string_arg(params, 0).unwrap_or_default();
            let resolved = crate::vba::resolve_virtual_path(&path);
            let file_obj = FileObject::new(&resolved);
            if !res.is_null() {
                let r = &mut *res;
                r.vt = VT_DISPATCH;
                let ptr_bytes = (file_obj as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_FSO_DELETEFILE => {
            let path = extract_string_arg(params, 0).unwrap_or_default();
            let resolved = crate::vba::resolve_virtual_path(&path);
            let _ = fs::remove_file(&resolved);
            S_OK
        }
        DISPID_FSO_DELETEFOLDER => {
            let path = extract_string_arg(params, 0).unwrap_or_default();
            let resolved = crate::vba::resolve_virtual_path(&path);
            let _ = fs::remove_dir_all(&resolved);
            S_OK
        }
        DISPID_FSO_GETSPECIALFOLDER => {
            let folder_spec = extract_i32_arg(params, 0, 2);
            let folder_path = match folder_spec {
                0 => "C:\\Windows".to_string(),
                1 => "C:\\Windows\\System32".to_string(),
                _ => std::env::temp_dir().to_string_lossy().to_string(),
            };
            if !res.is_null() {
                let bstr = sys_alloc_string(&folder_path);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        _ => DISP_E_MEMBERNOTFOUND,
    }
}

static FSO_VTBL: IDispatchVtbl = IDispatchVtbl {
    QueryInterface: fso_QueryInterface,
    AddRef: fso_AddRef,
    Release: fso_Release,
    GetTypeInfoCount: fso_GetTypeInfoCount,
    GetTypeInfo: fso_GetTypeInfo,
    GetIDsOfNames: fso_GetIDsOfNames,
    Invoke: fso_Invoke,
};

impl FileSystemObject {
    pub fn new() -> *mut FileSystemObject {
        let boxed = Box::new(FileSystemObject {
            vtable: &FSO_VTBL,
            ref_count: AtomicU32::new(1),
        });
        Box::into_raw(boxed)
    }
}

// ----------------------------------------------------------------------------
// TextStreamObject
// ----------------------------------------------------------------------------

const DISPID_TS_WRITELINE: i32 = 2000;
const DISPID_TS_WRITE: i32 = 2001;
const DISPID_TS_READLINE: i32 = 2002;
const DISPID_TS_READALL: i32 = 2003;
const DISPID_TS_ATENDOFSTREAM: i32 = 2004;
const DISPID_TS_ATENDOFLINE: i32 = 2005;
const DISPID_TS_CLOSE: i32 = 2006;

pub struct TextStreamData {
    pub path: String,
    pub iomode: i32, // 1 = ForReading, 2 = ForWriting, 8 = ForAppending
    pub is_closed: bool,
    pub buffer: Vec<u8>,
    pub read_pos: usize,
}

#[repr(C)]
pub struct TextStreamObject {
    pub vtable: *const IDispatchVtbl,
    pub ref_count: AtomicU32,
    pub data: Mutex<TextStreamData>,
}

unsafe extern "system" fn ts_QueryInterface(
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
        ts_AddRef(this);
        return S_OK;
    }
    *ppv = core::ptr::null_mut();
    E_NOINTERFACE
}

unsafe extern "system" fn ts_AddRef(this: *mut core::ffi::c_void) -> u32 {
    let obj = &*(this as *mut TextStreamObject);
    obj.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn ts_Release(this: *mut core::ffi::c_void) -> u32 {
    let obj_ptr = this as *mut TextStreamObject;
    let old_count = (*obj_ptr).ref_count.fetch_sub(1, Ordering::SeqCst);
    if old_count == 1 {
        let _ = Box::from_raw(obj_ptr);
        0
    } else {
        old_count - 1
    }
}

unsafe extern "system" fn ts_GetTypeInfoCount(
    _this: *mut core::ffi::c_void,
    pctinfo: *mut u32,
) -> HResult {
    if !pctinfo.is_null() {
        *pctinfo = 1;
    }
    S_OK
}

unsafe extern "system" fn ts_GetTypeInfo(
    _this: *mut core::ffi::c_void,
    iTInfo: u32,
    _lcid: u32,
    ppTInfo: *mut *mut core::ffi::c_void,
) -> HResult {
    if ppTInfo.is_null() {
        return E_POINTER;
    }
    if iTInfo != 0 {
        *ppTInfo = core::ptr::null_mut();
        return DISP_E_BADINDEX;
    }
    *ppTInfo = crate::com::NativeTypeInfo::new(
        "TextStream",
        crate::types::IID_NULL,
        &[
            "WriteLine",
            "Write",
            "ReadLine",
            "ReadAll",
            "AtEndOfStream",
            "Close",
        ],
    );
    S_OK
}

unsafe extern "system" fn ts_GetIDsOfNames(
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
            "writeline" => DISPID_TS_WRITELINE,
            "write" => DISPID_TS_WRITE,
            "readline" => DISPID_TS_READLINE,
            "readall" => DISPID_TS_READALL,
            "atendofstream" => DISPID_TS_ATENDOFSTREAM,
            "atendofline" => DISPID_TS_ATENDOFLINE,
            "close" => DISPID_TS_CLOSE,
            _ => {
                hr = DISP_E_UNKNOWNNAME;
                DISPID_UNKNOWN
            }
        };
        *rgDispId.add(i) = dispid;
    }

    hr
}

unsafe extern "system" fn ts_Invoke(
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
    let obj = &*(this as *mut TextStreamObject);
    let params = pDispParams as *mut DispParams;
    let res = pVarResult as *mut Variant;

    if !res.is_null() {
        *res = Variant::empty();
    }

    let mut guard = match obj.data.lock() {
        Ok(g) => g,
        Err(_) => return S_OK,
    };

    match dispIdMember {
        DISPID_TS_WRITELINE => {
            let line = extract_string_arg(params, 0).unwrap_or_default();
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&guard.path)
            {
                let _ = writeln!(file, "{}\r", line);
            }
            S_OK
        }
        DISPID_TS_WRITE => {
            let text = extract_string_arg(params, 0).unwrap_or_default();
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&guard.path)
            {
                let _ = write!(file, "{}", text);
            }
            S_OK
        }
        DISPID_TS_READLINE => {
            let mut line = String::new();
            let buf = &guard.buffer;
            let mut pos = guard.read_pos;

            while pos < buf.len() {
                let b = buf[pos];
                pos += 1;
                if b == b'\n' {
                    break;
                } else if b != b'\r' {
                    line.push(b as char);
                }
            }
            guard.read_pos = pos;

            if !res.is_null() {
                let bstr = sys_alloc_string(&line);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_TS_READALL => {
            let content = if guard.read_pos < guard.buffer.len() {
                String::from_utf8_lossy(&guard.buffer[guard.read_pos..]).to_string()
            } else {
                String::new()
            };
            guard.read_pos = guard.buffer.len();

            if !res.is_null() {
                let bstr = sys_alloc_string(&content);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_TS_ATENDOFSTREAM => {
            let is_eof = guard.read_pos >= guard.buffer.len();
            if !res.is_null() {
                let r = &mut *res;
                r.vt = VT_BOOL;
                let b_val: i16 = if is_eof { -1 } else { 0 };
                r.data[0..2].copy_from_slice(&b_val.to_ne_bytes());
            }
            S_OK
        }
        DISPID_TS_ATENDOFLINE => {
            let is_eol = guard.read_pos >= guard.buffer.len()
                || guard.buffer[guard.read_pos] == b'\r'
                || guard.buffer[guard.read_pos] == b'\n';
            if !res.is_null() {
                let r = &mut *res;
                r.vt = VT_BOOL;
                let b_val: i16 = if is_eol { -1 } else { 0 };
                r.data[0..2].copy_from_slice(&b_val.to_ne_bytes());
            }
            S_OK
        }
        DISPID_TS_CLOSE => {
            guard.is_closed = true;
            S_OK
        }
        _ => DISP_E_MEMBERNOTFOUND,
    }
}

static TS_VTBL: IDispatchVtbl = IDispatchVtbl {
    QueryInterface: ts_QueryInterface,
    AddRef: ts_AddRef,
    Release: ts_Release,
    GetTypeInfoCount: ts_GetTypeInfoCount,
    GetTypeInfo: ts_GetTypeInfo,
    GetIDsOfNames: ts_GetIDsOfNames,
    Invoke: ts_Invoke,
};

impl TextStreamObject {
    pub fn new(path: &str, iomode: i32) -> *mut TextStreamObject {
        let mut buffer = Vec::new();
        if iomode == 1 {
            if let Ok(mut f) = fs::File::open(path) {
                let _ = f.read_to_end(&mut buffer);
            }
        }
        let boxed = Box::new(TextStreamObject {
            vtable: &TS_VTBL,
            ref_count: AtomicU32::new(1),
            data: Mutex::new(TextStreamData {
                path: path.to_string(),
                iomode,
                is_closed: false,
                buffer,
                read_pos: 0,
            }),
        });
        Box::into_raw(boxed)
    }
}

// ----------------------------------------------------------------------------
// FileObject
// ----------------------------------------------------------------------------

const DISPID_FILE_SIZE: i32 = 3000;
const DISPID_FILE_NAME: i32 = 3001;
const DISPID_FILE_PATH: i32 = 3002;

#[repr(C)]
pub struct FileObject {
    pub vtable: *const IDispatchVtbl,
    pub ref_count: AtomicU32,
    pub path: String,
    pub name: String,
    pub size: u64,
}

unsafe extern "system" fn file_QueryInterface(
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
        file_AddRef(this);
        return S_OK;
    }
    *ppv = core::ptr::null_mut();
    E_NOINTERFACE
}

unsafe extern "system" fn file_AddRef(this: *mut core::ffi::c_void) -> u32 {
    let obj = &*(this as *mut FileObject);
    obj.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn file_Release(this: *mut core::ffi::c_void) -> u32 {
    let obj_ptr = this as *mut FileObject;
    let old_count = (*obj_ptr).ref_count.fetch_sub(1, Ordering::SeqCst);
    if old_count == 1 {
        let _ = Box::from_raw(obj_ptr);
        0
    } else {
        old_count - 1
    }
}

unsafe extern "system" fn file_GetTypeInfoCount(
    _this: *mut core::ffi::c_void,
    pctinfo: *mut u32,
) -> HResult {
    if !pctinfo.is_null() {
        *pctinfo = 1;
    }
    S_OK
}

unsafe extern "system" fn file_GetTypeInfo(
    _this: *mut core::ffi::c_void,
    iTInfo: u32,
    _lcid: u32,
    ppTInfo: *mut *mut core::ffi::c_void,
) -> HResult {
    if ppTInfo.is_null() {
        return E_POINTER;
    }
    if iTInfo != 0 {
        *ppTInfo = core::ptr::null_mut();
        return DISP_E_BADINDEX;
    }
    *ppTInfo = crate::com::NativeTypeInfo::new(
        "File",
        crate::types::IID_NULL,
        &["Size", "Name", "Path"],
    );
    S_OK
}

unsafe extern "system" fn file_GetIDsOfNames(
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
            "size" => DISPID_FILE_SIZE,
            "name" => DISPID_FILE_NAME,
            "path" => DISPID_FILE_PATH,
            _ => {
                hr = DISP_E_UNKNOWNNAME;
                DISPID_UNKNOWN
            }
        };
        *rgDispId.add(i) = dispid;
    }

    hr
}

unsafe extern "system" fn file_Invoke(
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
    let obj = &*(this as *mut FileObject);
    let res = pVarResult as *mut Variant;

    if !res.is_null() {
        *res = Variant::empty();
    }

    match dispIdMember {
        DISPID_FILE_SIZE => {
            if !res.is_null() {
                let r = &mut *res;
                r.vt = VT_I4;
                let s = obj.size as i32;
                r.data[0..4].copy_from_slice(&s.to_ne_bytes());
            }
            S_OK
        }
        DISPID_FILE_NAME => {
            if !res.is_null() {
                let bstr = sys_alloc_string(&obj.name);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        DISPID_FILE_PATH => {
            if !res.is_null() {
                let bstr = sys_alloc_string(&obj.path);
                let r = &mut *res;
                r.vt = VT_BSTR;
                let ptr_bytes = (bstr as usize).to_ne_bytes();
                r.data[..core::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
            }
            S_OK
        }
        _ => DISP_E_MEMBERNOTFOUND,
    }
}

static FILE_VTBL: IDispatchVtbl = IDispatchVtbl {
    QueryInterface: file_QueryInterface,
    AddRef: file_AddRef,
    Release: file_Release,
    GetTypeInfoCount: file_GetTypeInfoCount,
    GetTypeInfo: file_GetTypeInfo,
    GetIDsOfNames: file_GetIDsOfNames,
    Invoke: file_Invoke,
};

impl FileObject {
    pub fn new(path: &str) -> *mut FileObject {
        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
            .to_string();

        let boxed = Box::new(FileObject {
            vtable: &FILE_VTBL,
            ref_count: AtomicU32::new(1),
            path: path.to_string(),
            name,
            size,
        });
        Box::into_raw(boxed)
    }
}
