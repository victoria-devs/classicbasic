//! COM and Visual Basic 6.0 SAFEARRAY structures.

pub const FADF_AUTO: u16 = 0x0001;
pub const FADF_STATIC: u16 = 0x0002;
pub const FADF_EMBEDDED: u16 = 0x0004;
pub const FADF_FIXEDSIZE: u16 = 0x0010;
pub const FADF_RECORD: u16 = 0x0020;
pub const FADF_HAVEIID: u16 = 0x0040;
pub const FADF_HAVEVARTYPE: u16 = 0x0080;
pub const FADF_BSTR: u16 = 0x0100;
pub const FADF_UNKNOWN: u16 = 0x0200;
pub const FADF_DISPATCH: u16 = 0x0400;
pub const FADF_VARIANT: u16 = 0x0800;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SafeArrayBound {
    pub c_elements: u32,
    pub l_lbound: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SafeArray {
    pub c_dims: u16,
    pub f_features: u16,
    pub cb_elements: u32,
    pub c_locks: u32,
    pub pv_data: *mut core::ffi::c_void,
    pub rgsabound: [SafeArrayBound; 1],
}
