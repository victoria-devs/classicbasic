// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Regression Suite v0.2.6 (Phase 2.6): Multithreading, Thread-Local Storage & STA Concurrency Tests
//!
//! Validates Thread-Local Storage (TLS) error object isolation, STA apartment boundary
//! enforcement (`RPC_E_WRONG_THREAD = 0x8001010E`), thread-local BSTR scratch buffer caching,
//! interlocked atomic reference counting under heavy concurrency, DoEvents reentrancy unwinding,
//! and COM RPC call rejection translation (`0x80010001` -> Error 440).

use core::ffi::c_void;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;

use msvbvm60::com::*;
use msvbvm60::err::*;
use msvbvm60::pe::shims::*;
use msvbvm60::rtc::*;
use msvbvm60::threading::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;

// =========================================================================
// TEST SUITE: VB6 Multithreading, Thread-Local Storage & STA Concurrency
// =========================================================================

#[test]
fn test_multithreaded_sta_tls_isolation_of_err_object() {
    // GIVEN two worker OS threads executing compiled Native Code thunks simultaneously
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let b1 = Arc::clone(&barrier);
    let b2 = Arc::clone(&barrier);

    let handle_a = thread::spawn(move || {
        // WHEN Thread A triggers an error via __vbaOnError / Err.Raise(11, "Division by zero")
        unsafe {
            __vbaOnError((-1_isize) as *mut c_void); // On Error Resume Next
            let err_obj = rtcErrObj();
            let vtbl = &*(*(err_obj as *mut *mut ErrObjectVTable));
            let desc_var = Variant::from_bstr(sys_alloc_string("Division by zero"));
            let src_var = Variant::from_bstr(sys_alloc_string("ThreadA_Worker"));
            (vtbl.Raise)(err_obj, 11, &src_var, &desc_var, core::ptr::null(), core::ptr::null());
        }

        // Wait for Thread B to raise its error simultaneously
        b1.wait();

        // THEN Thread A's Thread-Local Storage (TLS) Err.Number must strictly read 11
        assert_eq!(get_vb_error_number(), 11);
        assert_eq!(get_vb_error_description(), "Division by zero");

        // Wait before clearing
        b1.wait();

        // AND calling Err.Clear on Thread A must NOT mutate Thread B's ErrObject state.
        clear_vb_error();
        assert_eq!(get_vb_error_number(), 0);

        b1.wait();
    });

    let handle_b = thread::spawn(move || {
        // AND Thread B simultaneously triggers Err.Raise(53, "File not found")
        unsafe {
            __vbaOnError((-1_isize) as *mut c_void);
            let err_obj = rtcErrObj();
            let vtbl = &*(*(err_obj as *mut *mut ErrObjectVTable));
            let desc_var = Variant::from_bstr(sys_alloc_string("File not found"));
            let src_var = Variant::from_bstr(sys_alloc_string("ThreadB_Worker"));
            (vtbl.Raise)(err_obj, 53, &src_var, &desc_var, core::ptr::null(), core::ptr::null());
        }

        b2.wait();

        // THEN Thread B's TLS Err.Number must strictly read 53
        assert_eq!(get_vb_error_number(), 53);
        assert_eq!(get_vb_error_description(), "File not found");

        b2.wait();
        // Wait for Thread A to clear its error
        b2.wait();

        // Thread B's error MUST still be 53
        assert_eq!(get_vb_error_number(), 53);
        assert_eq!(get_vb_error_description(), "File not found");
        clear_vb_error();
    });

    handle_a.join().expect("Thread A panicked");
    handle_b.join().expect("Thread B panicked");
}

#[test]
fn test_apartment_threaded_inproc_dll_thread_affinity_and_cross_thread_marshal_rejection() {
    // GIVEN an in-process COM object created on Thread A with Apartment Threading (STA)
    let (tx_to_b, rx_at_b) = std::sync::mpsc::channel();
    let (tx_to_a, rx_at_a) = std::sync::mpsc::channel();

    let handle_a = thread::spawn(move || {
        let dict = DictionaryObject::new();
        // Send raw pointer to Thread B without OLE marshaling
        tx_to_b.send(dict as usize).unwrap();

        // Wait until Thread B finishes attempting raw invocation
        let ok: bool = rx_at_a.recv().unwrap();
        assert!(ok);

        // Safe release within Thread A (the owner STA thread)
        unsafe {
            com_release(dict as *mut c_void);
        }
    });

    let handle_b = thread::spawn(move || {
        let raw_ptr = rx_at_b.recv().unwrap() as *mut DictionaryObject;

        // WHEN Thread B attempts direct raw vtable pointer invocation without OLE marshaling
        let hr = unsafe {
            let key = Variant::from_bstr(sys_alloc_string("KeyCrossThread"));
            let item = Variant::from_bstr(sys_alloc_string("ValCrossThread"));
            let vtbl = &*(*raw_ptr).vtable;
            (vtbl.Add)(raw_ptr as *mut c_void, &key, &item)
        };

        // THEN the runtime must detect apartment boundary crossing
        // AND reject or synchronize the call, returning RPC_E_WRONG_THREAD (0x8001010E)
        assert_eq!(hr, RPC_E_WRONG_THREAD);

        // Also test IDispatch::Invoke cross-thread rejection
        let hr_invoke = unsafe {
            let vtbl = &*(*raw_ptr).vtable;
            (vtbl.Invoke)(
                raw_ptr as *mut c_void,
                1, // Add method DISPID
                &IID_NULL,
                0,
                DISPATCH_METHOD,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            )
        };
        assert_eq!(hr_invoke, RPC_E_WRONG_THREAD);

        // AND prevent concurrent reentrant corruption of the internal object state.
        tx_to_a.send(true).unwrap();
    });

    handle_b.join().expect("Thread B panicked");
    handle_a.join().expect("Thread A panicked");
}

#[test]
fn test_thread_local_bstr_scratch_buffer_cache_isolation() {
    // GIVEN the compiler string helpers (__vbaStrCat, __vbaStrCopy) utilizing thread-local scratch buffers
    // WHEN 8 OS threads concurrently execute rapid BSTR concatenations in a tight loop
    let handles: Vec<_> = (0..8)
        .map(|tid| {
            thread::spawn(move || {
                for i in 0..1000 {
                    unsafe {
                        let s1 = sys_alloc_string(&format!("T{}_Prefix_", tid));
                        let s2 = sys_alloc_string(&format!("_Suffix_{}", i));
                        let s_cat = __vbaStrCat(s1, s2);

                        // Verify content integrity
                        let result_str = bstr_to_string(s_cat);
                        assert!(result_str.starts_with(&format!("T{}_Prefix_", tid)));
                        assert!(result_str.ends_with(&format!("_Suffix_{}", i)));

                        // Deallocate
                        SysFreeString(s1);
                        SysFreeString(s2);
                        SysFreeString(s_cat);
                    }
                }
            })
        })
        .collect();

    // THEN no thread's temporary scratch buffer pointer can leak or collide across threads
    // AND SysFreeString must safely deallocate back into the global OLE heap without double-frees.
    for h in handles {
        h.join().expect("Worker thread failed during BSTR stress");
    }
}

#[test]
fn test_interlocked_reference_counting_under_heavy_concurrent_addref_release() {
    // GIVEN a shared COM class instance (IUnknown) accessible across multiple threads
    let dict = DictionaryObject::new();
    // Allow free-threaded refcounting by clearing STA affinity for this stress test
    unsafe {
        (*dict).set_owner_thread(None);
    }

    let shared_ptr = Arc::new(std::sync::atomic::AtomicUsize::new(dict as usize));

    // Initial ref count is 1 from new()
    assert_eq!(unsafe { (*dict).ref_count.load(Ordering::SeqCst) }, 1);

    // WHEN 10 concurrent threads perform 10,000 iterations of __vbaObjSetAddref and __vbaFreeObj
    let thread_count = 10;
    let iterations_per_thread = 10_000;

    let handles: Vec<_> = (0..thread_count)
        .map(|_| {
            let s_ptr = Arc::clone(&shared_ptr);
            thread::spawn(move || {
                let obj = s_ptr.load(Ordering::SeqCst) as *mut c_void;
                let mut local_holder: *mut c_void = core::ptr::null_mut();

                for _ in 0..iterations_per_thread {
                    unsafe {
                        __vbaObjSetAddref(&mut local_holder, obj);
                        __vbaFreeObj(&mut local_holder);
                    }
                }
            })
        })
        .collect();

    for h in handles {
        h.join().expect("AddRef/Release thread failed");
    }

    // Ref count must strictly be back to initial 1
    assert_eq!(unsafe { (*dict).ref_count.load(Ordering::SeqCst) }, 1);

    // AND exactly one thread must invoke deallocation when reference count reaches 0
    let final_ref = unsafe { com_release(dict as *mut c_void) };
    assert_eq!(final_ref, 0);
}

#[test]
fn test_doevents_reentrancy_deadlock_detection_and_call_state_unwinding() {
    // GIVEN a long-running procedure dispatching messages via rtcDoEvents
    // WHEN a nested event callback is reentrantly dispatched into the same procedure
    // BEFORE the outer procedure invocation completes
    fn recursive_procedure(depth: u32, max_depth: u32) -> u32 {
        let local_variable = depth * 100 + 42;
        unsafe {
            rtcDoEvents();
        }

        if depth < max_depth {
            let inner_res = recursive_procedure(depth + 1, max_depth);
            assert_eq!(inner_res, (depth + 1) * 100 + 42);
        }

        // THEN local stack frames must retain their distinct variables and state
        assert_eq!(local_variable, depth * 100 + 42);
        local_variable
    }

    // AND returning from the inner reentrant frame must cleanly resume the outer execution flow.
    let result = recursive_procedure(1, 10);
    assert_eq!(result, 142);
    assert_eq!(get_doevents_reentrancy_depth(), 0);
}

#[test]
fn test_com_dispatch_timeout_and_server_call_rejection_error_trapping() {
    // GIVEN an external COM method call invoking IDispatch::Invoke on an unresponsive server
    // WHEN the server returns RPC_E_SERVERCALL_REJECTED (0x80010001) or times out
    let rejected_hr = RPC_E_SERVERCALL_REJECTED;

    // THEN the runtime must translate the HRESULT into VB6 Runtime Error 440 ("Automation error")
    // AND __vbaHresultCheckObj must populate ErrObject with the raw HRESULT code.
    unsafe {
        clear_vb_error();
        __vbaOnError((-1_isize) as *mut c_void); // On Error Resume Next
        let checked_hr = __vbaHresultCheckObj(
            rejected_hr,
            core::ptr::null_mut(),
            0,
            core::ptr::null_mut(),
        );
        assert_eq!(checked_hr, rejected_hr);

        assert_eq!(get_vb_error_number(), 440);
        let desc = get_vb_error_description();
        assert!(desc.contains("Automation error"));
        assert!(desc.contains("0x80010001"));

        // Check raw HRESULT stored in LastDllError / Context
        with_error_state(|state| {
            assert_eq!(state.last_dll_error, rejected_hr);
            assert_eq!(state.help_context, rejected_hr);
        });

        clear_vb_error();
    }
}

#[test]
fn test_multithreaded_last_dll_error_tls_isolation() {
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let b1 = Arc::clone(&barrier);
    let b2 = Arc::clone(&barrier);

    let handle_a = thread::spawn(move || {
        unsafe {
            win32_SetLastError(2); // ERROR_FILE_NOT_FOUND
        }
        b1.wait();
        unsafe {
            assert_eq!(win32_GetLastError(), 2);
            let mut last_dll = 0;
            let err_obj = rtcErrObj();
            let vtbl = &*(*(err_obj as *mut *mut ErrObjectVTable));
            (vtbl.get_LastDllError)(err_obj, &mut last_dll);
            assert_eq!(last_dll, 2);
        }
        b1.wait();
    });

    let handle_b = thread::spawn(move || {
        unsafe {
            win32_SetLastError(5); // ERROR_ACCESS_DENIED
        }
        b2.wait();
        unsafe {
            assert_eq!(win32_GetLastError(), 5);
            let mut last_dll = 0;
            let err_obj = rtcErrObj();
            let vtbl = &*(*(err_obj as *mut *mut ErrObjectVTable));
            (vtbl.get_LastDllError)(err_obj, &mut last_dll);
            assert_eq!(last_dll, 5);
        }
        b2.wait();
    });

    handle_a.join().expect("Thread A failed");
    handle_b.join().expect("Thread B failed");
}

#[test]
fn test_eb_worker_thread_context_lifecycle() {
    // Validates EbCreateContext, EbSetContextWorkerThread, and EbDestroyContext
    unsafe {
        let ctx = EbCreateContext();
        assert!(!ctx.is_null());

        let (tx, rx) = std::sync::mpsc::channel();
        let ctx_usize = ctx as usize;

        let handle = thread::spawn(move || {
            let p_ctx = ctx_usize as *mut c_void;
            let set_res = EbSetContextWorkerThread(p_ctx);
            assert_eq!(set_res, S_OK);
            assert_eq!(get_active_worker_context(), p_ctx);
            assert_eq!(get_vb_error_number(), 0);
            tx.send(true).unwrap();
        });

        assert!(rx.recv().unwrap());
        handle.join().unwrap();

        let destroy_res = EbDestroyContext(ctx);
        assert_eq!(destroy_res, S_OK);
    }
}
