// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Multithreading, Thread-Local Storage & STA Concurrency Engine.
//!
//! Provides worker thread context initialization (`EbCreateContext`, `EbDestroyContext`,
//! `EbSetContextWorkerThread`), STA apartment affinity validation, and thread lifecycle helpers.

use core::ffi::c_void;
use std::cell::Cell;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::types::S_OK;

/// Internal execution context descriptor for multithreaded Visual Basic worker threads.
#[repr(C)]
pub struct EbContext {
    /// Context identifier.
    pub id: u32,
    /// Reference count for the context descriptor.
    pub ref_count: AtomicU32,
    /// Reserved internal flags.
    pub flags: u32,
}

static NEXT_CONTEXT_ID: AtomicU32 = AtomicU32::new(1);

thread_local! {
    /// Active worker thread execution context attached to the calling thread.
    static ACTIVE_WORKER_CONTEXT: Cell<*mut EbContext> = const { Cell::new(core::ptr::null_mut()) };
}

/// Allocates a new worker thread execution context (`EbCreateContext`, Ordinal 274).
///
/// # Returns
/// Pointer to the newly allocated `EbContext` descriptor.
#[no_mangle]
pub unsafe extern "system" fn EbCreateContext() -> *mut c_void {
    let ctx = Box::new(EbContext {
        id: NEXT_CONTEXT_ID.fetch_add(1, Ordering::Relaxed),
        ref_count: AtomicU32::new(1),
        flags: 0,
    });
    Box::into_raw(ctx) as *mut c_void
}

/// Destroys a worker thread execution context (`EbDestroyContext`, Ordinal 275).
///
/// # Arguments
/// * `p_context` - Pointer to `EbContext` to deallocate.
///
/// # Returns
/// `S_OK` (0) indicating success.
#[no_mangle]
pub unsafe extern "system" fn EbDestroyContext(p_context: *mut c_void) -> i32 {
    if !p_context.is_null() {
        let ctx_ptr = p_context as *mut EbContext;
        ACTIVE_WORKER_CONTEXT.with(|c| {
            if c.get() == ctx_ptr {
                c.set(core::ptr::null_mut());
            }
        });
        let _ = Box::from_raw(ctx_ptr);
    }
    S_OK
}

/// Attaches the calling OS thread to a worker thread execution context (`EbSetContextWorkerThread`, Ordinal 276).
///
/// Sets the thread-local context pointer and initializes the calling thread's
/// runtime TLS state for VB6 execution.
///
/// # Arguments
/// * `p_context` - Pointer to the `EbContext` descriptor.
///
/// # Returns
/// `S_OK` (0) indicating success.
#[no_mangle]
pub unsafe extern "system" fn EbSetContextWorkerThread(p_context: *mut c_void) -> i32 {
    ACTIVE_WORKER_CONTEXT.with(|c| {
        c.set(p_context as *mut EbContext);
    });
    // Ensure thread-local error state is initialized for this thread
    crate::err::clear_vb_error();
    S_OK
}

/// Returns the active worker context pointer for the calling thread, or null if none.
pub fn get_active_worker_context() -> *mut c_void {
    ACTIVE_WORKER_CONTEXT.with(|c| c.get() as *mut c_void)
}
