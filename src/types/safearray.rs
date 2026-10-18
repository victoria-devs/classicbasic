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

impl SafeArray {
    /// Creates and allocates a 1-dimensional `SafeArray` vector.
    pub unsafe fn create_vector(
        elem_size: u32,
        lbound: i32,
        c_elements: u32,
        features: u16,
    ) -> *mut SafeArray {
        let total_header_size = core::mem::size_of::<SafeArray>();
        let data_bytes = (c_elements as usize) * (elem_size as usize);

        let header_layout = std::alloc::Layout::from_size_align(total_header_size, 8).unwrap();
        let p_sa = std::alloc::alloc_zeroed(header_layout) as *mut SafeArray;
        if p_sa.is_null() {
            return core::ptr::null_mut();
        }

        let data_layout = std::alloc::Layout::from_size_align(data_bytes.max(1), 8).unwrap();
        let p_data = std::alloc::alloc_zeroed(data_layout) as *mut core::ffi::c_void;

        (*p_sa).c_dims = 1;
        (*p_sa).f_features = features;
        (*p_sa).cb_elements = elem_size;
        (*p_sa).c_locks = 0;
        (*p_sa).pv_data = p_data;
        (*p_sa).rgsabound[0] = SafeArrayBound {
            c_elements,
            l_lbound: lbound,
        };

        p_sa
    }

    /// Creates and allocates a multi-dimensional `SafeArray`.
    ///
    /// Bounds are specified in natural dimension order (dimension 1, dimension 2, ...).
    pub unsafe fn create_multidim(
        elem_size: u32,
        bounds: &[SafeArrayBound],
        features: u16,
    ) -> *mut SafeArray {
        let c_dims = bounds.len() as u16;
        if c_dims == 0 {
            return core::ptr::null_mut();
        }
        let extra_bounds_size = (c_dims as usize - 1) * core::mem::size_of::<SafeArrayBound>();
        let total_header_size = core::mem::size_of::<SafeArray>() + extra_bounds_size;

        let mut total_elements: usize = 1;
        for b in bounds {
            total_elements = total_elements.saturating_mul(b.c_elements as usize);
        }
        let data_bytes = total_elements * (elem_size as usize);

        let header_layout = std::alloc::Layout::from_size_align(total_header_size, 8).unwrap();
        let p_sa = std::alloc::alloc_zeroed(header_layout) as *mut SafeArray;
        if p_sa.is_null() {
            return core::ptr::null_mut();
        }

        let data_layout = std::alloc::Layout::from_size_align(data_bytes.max(1), 8).unwrap();
        let p_data = std::alloc::alloc_zeroed(data_layout) as *mut core::ffi::c_void;

        (*p_sa).c_dims = c_dims;
        (*p_sa).f_features = features;
        (*p_sa).cb_elements = elem_size;
        (*p_sa).c_locks = 0;
        (*p_sa).pv_data = p_data;

        let bounds_ptr = (*p_sa).rgsabound.as_mut_ptr();
        for (i, b) in bounds.iter().enumerate() {
            *bounds_ptr.add(i) = *b;
        }

        p_sa
    }

    /// Redimensions an existing `SafeArray`, preserving existing elements.
    pub unsafe fn redim_preserve(p_sa: *mut SafeArray, new_ubound: i32, new_lbound: i32) {
        if p_sa.is_null() {
            return;
        }
        if (*p_sa).c_locks > 0 || ((*p_sa).f_features & FADF_FIXEDSIZE) != 0 {
            crate::err::set_vb_error(10, Some("This array is fixed or temporarily locked"), None);
            return;
        }
        let old_elements = (*p_sa).rgsabound[0].c_elements as usize;
        let elem_size = (*p_sa).cb_elements as usize;
        let new_elements = if new_ubound >= new_lbound {
            (new_ubound - new_lbound + 1) as usize
        } else {
            0
        };

        let new_data_bytes = new_elements * elem_size;
        let new_layout = std::alloc::Layout::from_size_align(new_data_bytes.max(1), 8).unwrap();
        let new_data = std::alloc::alloc_zeroed(new_layout) as *mut core::ffi::c_void;

        if !(*p_sa).pv_data.is_null() {
            let copy_count = old_elements.min(new_elements);
            if copy_count > 0 {
                core::ptr::copy_nonoverlapping((*p_sa).pv_data, new_data, copy_count * elem_size);
            }
            let old_data_bytes = old_elements * elem_size;
            let old_layout = std::alloc::Layout::from_size_align(old_data_bytes.max(1), 8).unwrap();
            std::alloc::dealloc((*p_sa).pv_data as *mut u8, old_layout);
        }

        (*p_sa).pv_data = new_data;
        (*p_sa).rgsabound[0].c_elements = new_elements as u32;
        (*p_sa).rgsabound[0].l_lbound = new_lbound;
    }

    /// Redimensions a multi-dimensional `SafeArray`, preserving existing elements.
    ///
    /// In COM and VB6, `ReDim Preserve` can only resize the LAST dimension.
    /// Attempting to change the number of dimensions or resizing any dimension other
    /// than the last dimension raises runtime error 9 (Subscript out of range).
    pub unsafe fn redim_preserve_multidim(
        p_sa: *mut SafeArray,
        new_bounds: &[SafeArrayBound],
    ) -> Result<(), i32> {
        if p_sa.is_null() {
            return Ok(());
        }
        if (*p_sa).c_locks > 0 || ((*p_sa).f_features & FADF_FIXEDSIZE) != 0 {
            crate::err::set_vb_error(10, Some("This array is fixed or temporarily locked"), None);
            return Err(10);
        }
        let c_dims = (*p_sa).c_dims as usize;
        if new_bounds.len() != c_dims {
            crate::err::set_vb_error(9, None, None);
            return Err(9);
        }

        let bounds_ptr = (*p_sa).rgsabound.as_mut_ptr();
        for i in 0..(c_dims - 1) {
            let old_b = *bounds_ptr.add(i);
            let new_b = new_bounds[i];
            if old_b.c_elements != new_b.c_elements || old_b.l_lbound != new_b.l_lbound {
                crate::err::set_vb_error(9, None, None);
                return Err(9);
            }
        }

        let mut old_total: usize = 1;
        for i in 0..c_dims {
            old_total = old_total.saturating_mul((*bounds_ptr.add(i)).c_elements as usize);
        }

        let mut new_total: usize = 1;
        for b in new_bounds {
            new_total = new_total.saturating_mul(b.c_elements as usize);
        }

        let elem_size = (*p_sa).cb_elements as usize;
        let new_data_bytes = new_total * elem_size;
        let new_layout = std::alloc::Layout::from_size_align(new_data_bytes.max(1), 8).unwrap();
        let new_data = std::alloc::alloc_zeroed(new_layout) as *mut core::ffi::c_void;

        if !(*p_sa).pv_data.is_null() {
            let copy_elements = old_total.min(new_total);
            if copy_elements > 0 {
                core::ptr::copy_nonoverlapping(
                    (*p_sa).pv_data,
                    new_data,
                    copy_elements * elem_size,
                );
            }
            let old_data_bytes = old_total * elem_size;
            let old_layout = std::alloc::Layout::from_size_align(old_data_bytes.max(1), 8).unwrap();
            std::alloc::dealloc((*p_sa).pv_data as *mut u8, old_layout);
        }

        (*p_sa).pv_data = new_data;
        *bounds_ptr.add(c_dims - 1) = new_bounds[c_dims - 1];
        Ok(())
    }

    /// Deallocates the element buffer and the `SafeArray` descriptor.
    pub unsafe fn destroy(p_sa: *mut SafeArray) {
        if p_sa.is_null() {
            return;
        }
        let c_dims = (*p_sa).c_dims as usize;
        let bounds_ptr = (*p_sa).rgsabound.as_ptr();
        let mut total_elements: usize = 1;
        for i in 0..c_dims {
            total_elements =
                total_elements.saturating_mul((*bounds_ptr.add(i)).c_elements as usize);
        }

        if !(*p_sa).pv_data.is_null() {
            let data_bytes = total_elements * ((*p_sa).cb_elements as usize);
            let data_layout = std::alloc::Layout::from_size_align(data_bytes.max(1), 8).unwrap();
            std::alloc::dealloc((*p_sa).pv_data as *mut u8, data_layout);
            (*p_sa).pv_data = core::ptr::null_mut();
        }
        let extra_bounds_size = c_dims.saturating_sub(1) * core::mem::size_of::<SafeArrayBound>();
        let total_header_size = core::mem::size_of::<SafeArray>() + extra_bounds_size;
        let header_layout = std::alloc::Layout::from_size_align(total_header_size, 8).unwrap();
        std::alloc::dealloc(p_sa as *mut u8, header_layout);
    }
}
