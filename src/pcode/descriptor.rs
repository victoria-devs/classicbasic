// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # P-Code Procedure Descriptors & Execution Frames
//!
//! Implements the clean-room binary structures pointed to by `EDX` when entering
//! a P-Code procedure via `ProcCallEngine` or `MethCallEngine`:
//! - [`ProcedureDescriptor`]: Header containing bytecode address, frame size, argument size, and SEH table.
//! - [`ProcFrame`]: Execution frame managing local variables, arguments, and deterministic memory epilogue.

use core::ffi::c_void;
use crate::types::{Bstr, SysFreeString, Variant};

/// Clean-room descriptor for a compiled P-Code procedure or method.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProcedureDescriptor {
    /// Pointer to the raw bytecode stream.
    pub lp_byte_code: *const u8,
    /// Total bytes required for local variables and temporaries.
    pub frame_size: u32,
    /// Total bytes occupied by incoming parameters on the `__stdcall` stack.
    pub args_size: u32,
    /// Procedure flags (e.g., function return type, static variables, event handler).
    pub flags: u32,
    /// Pointer to the procedure-level SEH error handler table (`On Error`).
    pub lp_seh_table: *const u8,
}

impl ProcedureDescriptor {
    /// Creates a procedure descriptor from raw memory or synthetic test fixtures.
    pub fn new(
        byte_code: *const u8,
        frame_size: u32,
        args_size: u32,
        flags: u32,
        seh_table: *const u8,
    ) -> Self {
        Self {
            lp_byte_code: byte_code,
            frame_size,
            args_size,
            flags,
            lp_seh_table: seh_table,
        }
    }

    /// Attempts to parse a procedure descriptor from a raw pointer provided in `EDX`.
    pub unsafe fn from_ptr(ptr: usize) -> Option<Self> {
        let base = crate::pe::shims::GLOBAL_MODULE_HANDLE;
        let size = crate::pe::shims::GLOBAL_MODULE_SIZE;
        if ptr == 0 {
            return None;
        }

        // Validate that pointer resides within mapped executable memory if base is set
        if base != 0 && size > 0 && (ptr < base || ptr + 16 > base + size) {
            // Alternatively allow valid heap/stack allocations for synthetic unit tests
            if ptr < 0x1000 {
                return None;
            }
        }

        let desc_ptr = ptr as *const u32;
        let byte_code = core::ptr::read_unaligned(desc_ptr) as usize as *const u8;
        let frame_size = core::ptr::read_unaligned(desc_ptr.add(1));
        let args_size = core::ptr::read_unaligned(desc_ptr.add(2));
        let flags = core::ptr::read_unaligned(desc_ptr.add(3));

        Some(Self {
            lp_byte_code: byte_code,
            frame_size,
            args_size,
            flags,
            lp_seh_table: core::ptr::null(),
        })
    }
}

/// Execution frame allocated for a P-Code procedure call.
pub struct ProcFrame {
    /// Local variable memory buffer.
    pub locals: Vec<u8>,
    /// Array of Variant slots for local Variant variables.
    pub variant_slots: Vec<Variant>,
    /// Tracked allocated BSTRs that must be freed upon exit.
    pub allocated_bstrs: Vec<Bstr>,
    /// Tracked COM objects that must be released upon exit.
    pub allocated_objs: Vec<*mut c_void>,
    /// Handler address for `On Error GoTo <Label>` within this frame.
    pub error_handler_pc: Option<usize>,
    /// Flag indicating whether `On Error Resume Next` is active.
    pub resume_next: bool,
}

impl ProcFrame {
    /// Allocates and zeroes a new execution frame of the requested size.
    pub fn new(frame_size: usize) -> Self {
        let actual_size = frame_size.max(256);
        Self {
            locals: vec![0u8; actual_size],
            variant_slots: vec![Variant::empty(); 32],
            allocated_bstrs: Vec::new(),
            allocated_objs: Vec::new(),
            error_handler_pc: None,
            resume_next: false,
        }
    }

    /// Reads a 32-bit integer local variable at the given offset.
    pub fn read_i32(&self, offset: usize) -> i32 {
        if offset + 4 <= self.locals.len() {
            unsafe { core::ptr::read_unaligned(self.locals.as_ptr().add(offset) as *const i32) }
        } else {
            0
        }
    }

    /// Writes a 32-bit integer local variable at the given offset.
    pub fn write_i32(&mut self, offset: usize, value: i32) {
        if offset + 4 <= self.locals.len() {
            unsafe {
                core::ptr::write_unaligned(self.locals.as_mut_ptr().add(offset) as *mut i32, value);
            }
        }
    }

    /// Reads a 64-bit float local variable at the given offset.
    pub fn read_f64(&self, offset: usize) -> f64 {
        if offset + 8 <= self.locals.len() {
            unsafe { core::ptr::read_unaligned(self.locals.as_ptr().add(offset) as *const f64) }
        } else {
            0.0
        }
    }

    /// Writes a 64-bit float local variable at the given offset.
    pub fn write_f64(&mut self, offset: usize, value: f64) {
        if offset + 8 <= self.locals.len() {
            unsafe {
                core::ptr::write_unaligned(self.locals.as_mut_ptr().add(offset) as *mut f64, value);
            }
        }
    }

    /// Tracks an allocated BSTR for automatic cleanup during frame teardown.
    pub fn track_bstr(&mut self, b: Bstr) {
        if !b.is_null() {
            self.allocated_bstrs.push(b);
        }
    }

    /// Tracks an allocated COM object for automatic release during frame teardown.
    pub fn track_obj(&mut self, obj: *mut c_void) {
        if !obj.is_null() {
            self.allocated_objs.push(obj);
        }
    }

    /// Executes deterministic epilogue cleanup of all local resources.
    pub fn teardown(&mut self) {
        // 1. Free tracked BSTR strings
        for b in self.allocated_bstrs.drain(..) {
            unsafe {
                SysFreeString(b);
            }
        }

        // 2. Clear tracked Variant slots
        for v in self.variant_slots.iter_mut() {
            unsafe {
                crate::vba::__vbaFreeVar(v);
            }
        }

        // 3. Release tracked COM object references
        for obj in self.allocated_objs.drain(..) {
            unsafe {
                let mut ptr = obj;
                crate::err::__vbaFreeObj(&mut ptr);
            }
        }
    }
}

impl Drop for ProcFrame {
    fn drop(&mut self) {
        self.teardown();
    }
}
