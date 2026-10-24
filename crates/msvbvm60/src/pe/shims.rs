//! Win32 API Shims (re-exported from `cb-win32`) and OLEAUT32 Automation Shims.

pub use cb_win32::shims::*;

use crate::types::{HResult, SafeArray, Variant, E_INVALIDARG, E_POINTER, S_OK};

#[no_mangle]
pub unsafe extern "system" fn oleaut32_VariantInit(pvarg: *mut Variant) {
    if !pvarg.is_null() {
        *pvarg = Variant::empty();
    }
}

#[no_mangle]
pub unsafe extern "system" fn oleaut32_VariantClear(pvarg: *mut Variant) -> HResult {
    if !pvarg.is_null() {
        crate::vba::__vbaFreeVar(pvarg);
    }
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn oleaut32_VariantCopy(
    pvarg_dest: *mut Variant,
    pvarg_src: *const Variant,
) -> HResult {
    if pvarg_dest.is_null() || pvarg_src.is_null() {
        return E_POINTER;
    }
    crate::vba::__vbaVarCopy(pvarg_dest, pvarg_src);
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn oleaut32_VariantCopyInd(
    pvar_dest: *mut Variant,
    pvarg_src: *const Variant,
) -> HResult {
    oleaut32_VariantCopy(pvar_dest, pvarg_src)
}

#[no_mangle]
pub unsafe extern "system" fn oleaut32_VariantChangeType(
    pvarg_dest: *mut Variant,
    pvar_src: *const Variant,
    _w_flags: u16,
    _vt: u16,
) -> HResult {
    if pvarg_dest.is_null() || pvar_src.is_null() {
        return E_POINTER;
    }
    crate::vba::__vbaVarCopy(pvarg_dest, pvar_src);
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn oleaut32_SafeArrayCreateVector(
    vt: u16,
    l_lbound: i32,
    c_elements: u32,
) -> *mut SafeArray {
    let elem_size = match vt {
        2 | 11 => 2, // VT_I2, VT_BOOL
        3 | 4 => 4,  // VT_I4, VT_R4
        5..=7 => 8,  // VT_R8, VT_CY, VT_DATE
        8 => core::mem::size_of::<crate::types::Bstr>() as u32,
        9 | 13 => core::mem::size_of::<usize>() as u32,
        12 => core::mem::size_of::<Variant>() as u32,
        _ => 4,
    };
    SafeArray::create_vector(elem_size, l_lbound, c_elements, 0)
}

#[no_mangle]
pub unsafe extern "system" fn oleaut32_SafeArrayDestroy(psa: *mut SafeArray) -> HResult {
    if psa.is_null() {
        return S_OK;
    }
    match SafeArray::destroy_checked(psa) {
        Ok(_) => S_OK,
        Err(hr) => hr,
    }
}

#[no_mangle]
pub unsafe extern "system" fn oleaut32_SafeArrayPtrOfIndex(
    psa: *mut SafeArray,
    rg_indices: *const i32,
    ppv_data: *mut *mut core::ffi::c_void,
) -> HResult {
    if psa.is_null() || rg_indices.is_null() || ppv_data.is_null() {
        return E_INVALIDARG;
    }
    let c_dims = (*psa).c_dims as usize;
    let indices = core::slice::from_raw_parts(rg_indices, c_dims);
    match SafeArray::ptr_of_index(psa, indices) {
        Ok(ptr) => {
            *ppv_data = ptr;
            S_OK
        }
        Err(hr) => hr,
    }
}

#[no_mangle]
pub unsafe extern "system" fn oleaut32_SafeArrayGetDim(psa: *mut SafeArray) -> u32 {
    if psa.is_null() {
        0
    } else {
        (*psa).c_dims as u32
    }
}

#[no_mangle]
pub unsafe extern "system" fn oleaut32_SafeArrayGetLBound(
    psa: *mut SafeArray,
    n_dim: u32,
    p_lbound: *mut i32,
) -> HResult {
    if psa.is_null() || p_lbound.is_null() || n_dim == 0 || n_dim > (*psa).c_dims as u32 {
        return E_INVALIDARG;
    }
    *p_lbound = (*psa).rgsabound[(n_dim - 1) as usize].l_lbound;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn oleaut32_SafeArrayGetUBound(
    psa: *mut SafeArray,
    n_dim: u32,
    p_ubound: *mut i32,
) -> HResult {
    if psa.is_null() || p_ubound.is_null() || n_dim == 0 || n_dim > (*psa).c_dims as u32 {
        return E_INVALIDARG;
    }
    let b = (*psa).rgsabound[(n_dim - 1) as usize];
    *p_ubound = b.l_lbound + (b.c_elements as i32) - 1;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn oleaut32_SafeArrayAccessData(
    psa: *mut SafeArray,
    ppv_data: *mut *mut core::ffi::c_void,
) -> HResult {
    if psa.is_null() || ppv_data.is_null() {
        return E_POINTER;
    }
    (*psa).c_locks += 1;
    *ppv_data = (*psa).pv_data;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn oleaut32_SafeArrayUnaccessData(psa: *mut SafeArray) -> HResult {
    if psa.is_null() {
        return E_POINTER;
    }
    if (*psa).c_locks > 0 {
        (*psa).c_locks -= 1;
    }
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn oleaut32_SafeArrayLock(psa: *mut SafeArray) -> HResult {
    if psa.is_null() {
        return E_POINTER;
    }
    (*psa).c_locks += 1;
    S_OK
}

#[no_mangle]
pub unsafe extern "system" fn oleaut32_SafeArrayUnlock(psa: *mut SafeArray) -> HResult {
    if psa.is_null() {
        return E_POINTER;
    }
    if (*psa).c_locks > 0 {
        (*psa).c_locks -= 1;
    }
    S_OK
}
