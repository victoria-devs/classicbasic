//! COM and Visual Basic 6.0 SAFEARRAY structures.
//!
//! Visual Basic 6 arrays are represented in memory as OLE Automation `SAFEARRAY` structures,
//! storing dimension counts, feature flags, lock counts, and boundary descriptors (`SAFEARRAYBOUND`).

/// Array allocated on the stack.
pub const FADF_AUTO: u16 = 0x0001;
/// Array statically allocated.
pub const FADF_STATIC: u16 = 0x0002;
/// Array embedded in a structure or record.
pub const FADF_EMBEDDED: u16 = 0x0004;
/// Array cannot be resized or reallocated.
pub const FADF_FIXEDSIZE: u16 = 0x0010;
/// Array contains User Defined Type (UDT) records.
pub const FADF_RECORD: u16 = 0x0020;
/// Array contains interface pointers and has an associated IID.
pub const FADF_HAVEIID: u16 = 0x0040;
/// Array has an associated `VarType` identifying element type.
pub const FADF_HAVEVARTYPE: u16 = 0x0080;
/// Array contains BSTR elements.
pub const FADF_BSTR: u16 = 0x0100;
/// Array contains `IUnknown` interface pointers.
pub const FADF_UNKNOWN: u16 = 0x0200;
/// Array contains `IDispatch` interface pointers.
pub const FADF_DISPATCH: u16 = 0x0400;
/// Array contains `Variant` elements.
pub const FADF_VARIANT: u16 = 0x0800;

/// Boundary descriptor for a single dimension of a `SafeArray`.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SafeArrayBound {
    /// Number of elements in this dimension.
    pub c_elements: u32,
    /// Lower bound index for this dimension (e.g. `0` or `1`).
    pub l_lbound: i32,
}

/// OLE Automation `SAFEARRAY` descriptor structure.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SafeArray {
    /// Count of dimensions.
    pub c_dims: u16,
    /// Flags describing array memory layout and element features (`FADF_*`).
    pub f_features: u16,
    /// Size in bytes of an individual array element.
    pub cb_elements: u32,
    /// Lock counter tracking how many active references are locking the array.
    pub c_locks: u32,
    /// Pointer to the raw array element data.
    pub pv_data: *mut core::ffi::c_void,
    /// Flexible array member containing dimension boundary descriptors.
    pub rgsabound: [SafeArrayBound; 1],
}
