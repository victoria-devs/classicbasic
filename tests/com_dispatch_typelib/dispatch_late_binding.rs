//! Regression Suite v0.2.3 (Phase 2.3): Clean-Room Late-Binding Dispatch Engine Tests
//!
//! Validates Phase 2.3 COM infrastructure:
//! - `IDispatch::GetIDsOfNames` case-insensitive member name resolution
//! - `IDispatch::Invoke` execution with `DISPPARAMS` (reverse argument ordering)
//! - Late-bound Property Get (`DISPATCH_PROPERTYGET`), Property Put (`DISPATCH_PROPERTYPUT`), and Method (`DISPATCH_METHOD`)
//! - Default member (DISPID 0 / `DISPID_VALUE`) invocation
//! - Compiler runtime helpers: `__vbaLateMemCall*`, `__vbaLateIdCall*`, `__vbaVarLateMemCall*`
//! - Variant object extraction (`__vbaUnkVar`) and reference assignment (`__vbaVarSetObjAddref`)
//! - Error handling: non-existent member raises VB error 438 ("Object doesn't support this property or method")

use core::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use msvbvm60::com::*;
use msvbvm60::err::*;
use msvbvm60::mem::*;
use msvbvm60::pe::shims::*;
use msvbvm60::rtc::*;
use msvbvm60::threading::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;

fn create_test_dictionary() -> *mut c_void {
    let mut p_obj: *mut c_void = core::ptr::null_mut();
    unsafe {
        let hr = get_dictionary_class_factory(&IID_IClassFactory, &mut p_obj);
        assert_eq!(hr, S_OK);
        let cf_vtbl = *(p_obj as *mut *mut usize);
        let create_instance_fn: unsafe extern "system" fn(
            *mut c_void,
            *mut c_void,
            *const Iid,
            *mut *mut c_void,
        ) -> HResult = core::mem::transmute(*cf_vtbl.add(3));
        let mut dict_ptr: *mut c_void = core::ptr::null_mut();
        let hr_inst = create_instance_fn(p_obj, core::ptr::null_mut(), &IID_IDispatch, &mut dict_ptr);
        assert_eq!(hr_inst, S_OK);
        com_release(p_obj);
        dict_ptr
    }
}

#[test]
fn test_dict_get_ids_of_names() {
    let dict = create_test_dictionary();
    assert!(!dict.is_null());

    unsafe {
        let vtable = *(dict as *mut *mut usize);
        let get_ids_fn: unsafe extern "system" fn(
            *mut c_void,
            *const Iid,
            *const *const u16,
            u32,
            u32,
            *mut i32,
        ) -> HResult = core::mem::transmute(*vtable.add(5));

        // Test standard member names and case insensitivity
        let names_to_test = [
            ("Item", 0),
            ("item", 0),
            ("ITEM", 0),
            ("Add", 1),
            ("add", 1),
            ("Count", 2),
            ("Exists", 3),
            ("Items", 4),
            ("Key", 5),
            ("Keys", 6),
            ("Remove", 7),
            ("RemoveAll", 8),
            ("CompareMode", 9),
        ];

        for (name_str, expected_dispid) in names_to_test {
            let utf16 = format!("{}\0", name_str).encode_utf16().collect::<Vec<u16>>();
            let mut dispid: i32 = -999;
            let name_ptrs: [*const u16; 1] = [utf16.as_ptr()];
            let hr = get_ids_fn(dict, &IID_NULL, name_ptrs.as_ptr(), 1, 0, &mut dispid);
            assert_eq!(hr, S_OK, "GetIDsOfNames failed for {}", name_str);
            assert_eq!(dispid, expected_dispid, "Unexpected DISPID for {}", name_str);
        }

        // Test unknown member name
        let unknown = "MetodoQueNaoExiste\0".encode_utf16().collect::<Vec<u16>>();
        let mut unk_dispid: i32 = 0;
        let unk_ptrs: [*const u16; 1] = [unknown.as_ptr()];
        let hr_unk = get_ids_fn(dict, &IID_NULL, unk_ptrs.as_ptr(), 1, 0, &mut unk_dispid);
        assert_eq!(hr_unk, DISP_E_UNKNOWNNAME);
        assert_eq!(unk_dispid, DISPID_UNKNOWN);

        com_release(dict);
    }
}

#[test]
fn test_dict_invoke_methods_and_properties() {
    let dict = create_test_dictionary();
    assert!(!dict.is_null());

    unsafe {
        // 1. Add("Fruit", "Apple")
        let s_fruit = sys_alloc_string("Fruit");
        let s_apple = sys_alloc_string("Apple");
        let v_fruit = Variant::from_bstr(s_fruit);
        let v_apple = Variant::from_bstr(s_apple);
        let mut add_args = [v_apple, v_fruit]; // Reverse order: rgvarg[1]=key, rgvarg[0]=item
        let hr_add = dispatch_invoke_id(
            dict,
            1,
            DISPATCH_METHOD,
            add_args.as_mut_ptr(),
            2,
            core::ptr::null_mut(),
        );
        assert_eq!(hr_add, S_OK);

        // 2. Count -> 1
        let mut v_count = Variant::empty();
        let hr_count = dispatch_invoke_id(
            dict,
            2,
            DISPATCH_PROPERTYGET,
            core::ptr::null_mut(),
            0,
            &mut v_count,
        );
        assert_eq!(hr_count, S_OK);
        assert_eq!(v_count.as_i32(), Some(1));

        // 3. Exists("Fruit") -> True
        let mut v_fruit_q = Variant::from_bstr(s_fruit);
        let mut v_exists = Variant::empty();
        let hr_exists = dispatch_invoke_id(
            dict,
            3,
            DISPATCH_METHOD | DISPATCH_PROPERTYGET,
            &mut v_fruit_q,
            1,
            &mut v_exists,
        );
        assert_eq!(hr_exists, S_OK);
        assert_eq!(v_exists.as_bool(), Some(true));

        // 4. Item("Fruit") (Default member DISPID 0) -> "Apple"
        let mut v_fruit_item = Variant::from_bstr(s_fruit);
        let mut v_item_ret = Variant::empty();
        let hr_item = dispatch_invoke_id(
            dict,
            0,
            DISPATCH_PROPERTYGET,
            &mut v_fruit_item,
            1,
            &mut v_item_ret,
        );
        assert_eq!(hr_item, S_OK);
        let bstr_res = v_item_ret.as_bstr().unwrap();
        assert_eq!(bstr_to_string(bstr_res), "Apple");

        // 5. Item("Fruit") = "Orange" (Property Put)
        let s_orange = sys_alloc_string("Orange");
        let v_orange = Variant::from_bstr(s_orange);
        let mut put_args = [v_orange, v_fruit]; // rgvarg[0]=value, rgvarg[1]=key
        let hr_put = dispatch_invoke_id(
            dict,
            0,
            DISPATCH_PROPERTYPUT,
            put_args.as_mut_ptr(),
            1,
            core::ptr::null_mut(),
        );
        assert_eq!(hr_put, S_OK);

        // Verify update
        let mut v_fruit_check = Variant::from_bstr(s_fruit);
        let mut v_item_updated = Variant::empty();
        dispatch_invoke_id(
            dict,
            0,
            DISPATCH_PROPERTYGET,
            &mut v_fruit_check,
            1,
            &mut v_item_updated,
        );
        let bstr_upd = v_item_updated.as_bstr().unwrap();
        assert_eq!(bstr_to_string(bstr_upd), "Orange");

        // 6. Keys collection
        let mut v_keys = Variant::empty();
        let hr_keys = dispatch_invoke_id(
            dict,
            6,
            DISPATCH_PROPERTYGET,
            core::ptr::null_mut(),
            0,
            &mut v_keys,
        );
        assert_eq!(hr_keys, S_OK);
        let keys_sa = __vbaRefVarAry(&mut v_keys);
        assert!(!keys_sa.is_null());
        let p_sa = core::ptr::read_unaligned(keys_sa);
        assert!(!p_sa.is_null());
        assert_eq!(__vbaUbound(p_sa, 1), 0); // 0-based 1 item -> ubound=0

        // 7. Remove("Fruit")
        let mut v_fruit_rem = Variant::from_bstr(s_fruit);
        let hr_rem = dispatch_invoke_id(
            dict,
            7,
            DISPATCH_METHOD,
            &mut v_fruit_rem,
            1,
            core::ptr::null_mut(),
        );
        assert_eq!(hr_rem, S_OK);

        // 8. Count after remove -> 0
        let mut v_count_zero = Variant::empty();
        dispatch_invoke_id(
            dict,
            2,
            DISPATCH_PROPERTYGET,
            core::ptr::null_mut(),
            0,
            &mut v_count_zero,
        );
        assert_eq!(v_count_zero.as_i32(), Some(0));

        SysFreeString(s_fruit);
        SysFreeString(s_apple);
        SysFreeString(s_orange);
        com_release(dict);
    }
}

#[test]
fn test_late_binding_helpers_and_error_handling() {
    let dict = create_test_dictionary();
    assert!(!dict.is_null());

    unsafe {
        clear_vb_error();

        // 1. Invocations via member name helper
        let s_add = "Add\0".encode_utf16().collect::<Vec<u16>>();
        let s_k = sys_alloc_string("Rust");
        let s_v = sys_alloc_string("Awesome");
        let v_k = Variant::from_bstr(s_k);
        let v_v = Variant::from_bstr(s_v);
        let mut args = [v_v, v_k];
        let hr = dispatch_invoke_member(
            dict,
            s_add.as_ptr(),
            DISPATCH_METHOD,
            args.as_mut_ptr(),
            2,
            core::ptr::null_mut(),
        );
        assert_eq!(hr, S_OK);

        // 2. Calling non-existent method raises Error 438
        let s_missing = "NoSuchMethodHere\0".encode_utf16().collect::<Vec<u16>>();
        let hr_err = dispatch_invoke_member(
            dict,
            s_missing.as_ptr(),
            DISPATCH_METHOD,
            core::ptr::null_mut(),
            0,
            core::ptr::null_mut(),
        );
        assert_eq!(hr_err, DISP_E_UNKNOWNNAME);
        assert_eq!(get_vb_error_number(), 438);
        clear_vb_error();

        // 3. Variant late-binding helpers (__vbaVarSetObjAddref & __vbaUnkVar)
        let mut v_obj = Variant::empty();
        __vbaVarSetObjAddref(&mut v_obj, dict);
        assert_eq!(v_obj.vt, VT_DISPATCH);

        let p_extracted = __vbaUnkVar(&v_obj, 0);
        assert_eq!(p_extracted, dict);

        // Clear variant object
        __vbaVarSetObjAddref(&mut v_obj, core::ptr::null_mut());
        assert_eq!(v_obj.vt, VT_EMPTY);
        let p_cleared = __vbaUnkVar(&v_obj, 0);
        assert!(p_cleared.is_null());

        SysFreeString(s_k);
        SysFreeString(s_v);
        com_release(dict);
    }
}

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

static LAYER17_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Test 1: Zombie Object State Transition & Invocation Lockout
///
/// GIVEN a VB6 Class instance resurrected during Class_Terminate by assigning 'Set globalRef = Me'
/// WHEN the teardown cycle finishes and the object enters the official COM Zombie state
/// THEN any subsequent method or property call on globalRef must fail immediately with E_UNEXPECTED (0x8000FFFF)
/// AND its IDispatch vtable must point to the runtime's internal Zombie_Invoke / Zombie_VTable stubs.
#[test]
fn test_zombie_object_state_transition_and_invocation_lockout() {
    let _guard = LAYER17_MUTEX.lock().unwrap();

    // 1. Verify standalone Zombie stubs
    unsafe {
        assert_eq!(Zombie_AddRef(core::ptr::null_mut()), 1);
        assert_eq!(Zombie_Release(core::ptr::null_mut()), 1);
        assert_eq!(Zombie_Invoke(), E_UNEXPECTED);
        assert_eq!(Zombie_GetIDsOfNames(), S_OK);
        assert_eq!(Zombie_GetTypeInfo(), S_OK);
        assert_eq!(Zombie_GetTypeInfoCount(), S_OK);

        let mut pv: *mut c_void = core::ptr::null_mut();
        let hr = Zombie_QueryInterface(
            core::ptr::null_mut(),
            &IID_IDispatch as *const Guid,
            &mut pv,
        );
        assert_eq!(hr, S_OK);
    }

    // 2. Simulate resurrecting an object inside Class_Terminate
    let resurrected_slot: Arc<std::sync::Mutex<Option<usize>>> =
        Arc::new(std::sync::Mutex::new(None));
    let resurrected_slot_clone = Arc::clone(&resurrected_slot);

    let instance_ptr = VbClassInstance::new("CZombie");

    unsafe {
        // Set Class_Terminate callback that attempts to resurrect the object ('Set gZombieRef = Me')
        (*instance_ptr).terminate_action = Some(Box::new(move |inst| {
            // Resurrection: AddRef and save pointer in global reference
            VbClassInstance::add_ref(inst as *mut c_void);
            *resurrected_slot_clone.lock().unwrap() = Some(inst as usize);
        }));

        // Teardown cycle: Release the original reference
        let rem = VbClassInstance::release(instance_ptr as *mut c_void);
        assert_eq!(rem, 1); // Resurrected reference kept it alive!

        // Object must now be in ZOMBIE state
        assert!((*instance_ptr).is_zombie);
        assert_eq!((*instance_ptr).vtbl, &ZOMBIE_IDISPATCH_VTBL as *const _);

        // Any subsequent invocation must fail immediately with E_UNEXPECTED (0x8000FFFF)
        let hr = ((*(*instance_ptr).vtbl).Invoke)(
            instance_ptr as *mut c_void,
            1,
            core::ptr::null(),
            0,
            1,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        );
        assert_eq!(hr, E_UNEXPECTED);

        // Releasing the zombie reference frees it cleanly without re-entering Class_Terminate
        let final_rem = VbClassInstance::release(instance_ptr as *mut c_void);
        assert_eq!(final_rem, 0);
    }
}

/// Test 2: SafeArray Lock Violation During ByRef Iteration
///
/// GIVEN a dynamic SafeArray being iterated via 'For Each x In arr' (which increments cLocks)
/// WHEN user code inside the loop attempts ReDim arr(10) or Erase arr
/// THEN the runtime must trap with VB Error 10: "This array is fixed or temporarily locked"
/// AND must NOT corrupt heap or alter the SafeArray's pvData pointer while locked.
#[test]
fn test_safearray_lock_violation_when_redim_called_inside_byref_iteration() {
    let _guard = LAYER17_MUTEX.lock().unwrap();
    clear_vb_error();

    unsafe {
        // Create dynamic SafeArray with 5 elements (size 4, lbound 1, elements 5)
        let psa = SafeArray::create_vector(4, 1, 5, 0);
        assert!(!psa.is_null());
        assert_eq!((*psa).c_locks, 0);

        // Populate elements
        let data_slice = core::slice::from_raw_parts_mut((*psa).pv_data as *mut i32, 5);
        for (i, val) in data_slice.iter_mut().enumerate() {
            *val = (i as i32 + 1) * 10;
        }
        let original_pv_data = (*psa).pv_data;

        // Initialize For Each iteration state
        let mut enum_state = VbAryEnumState::default();
        let mut current_elem = Variant::empty();
        let mut psa_ptr = psa;

        // Begin enumeration: locks the SafeArray (c_locks increments to 1)
        // elem_type 3 = VT_I4
        let has_elements = __vbaForEachAry(
            3,
            &mut enum_state as *mut _ as *mut c_void,
            core::ptr::null_mut(),
            &mut current_elem,
            psa,
        );
        assert_eq!(has_elements, -1); // VARIANT_TRUE
        assert_eq!((*psa).c_locks, 1);
        assert_eq!(current_elem.to_i32(), Some(10));

        // ATTEMPT 1: ReDim while locked must fail with VB Error 10
        let res_redim = __vbaRedim(0, 4, &mut psa_ptr, VT_I4 as u32, 1, 10, 1);
        assert_eq!(res_redim, 10);
        assert_eq!(get_vb_error_number(), 10);
        assert_eq!(
            get_vb_error_description(),
            "This array is fixed or temporarily locked"
        );
        assert_eq!((*psa).pv_data, original_pv_data);
        clear_vb_error();

        // ATTEMPT 2: Erase while locked must fail with VB Error 10
        let res_erase = __vbaErase(&mut psa_ptr, 0);
        assert_eq!(res_erase, 10);
        assert_eq!(get_vb_error_number(), 10);
        assert_eq!((*psa).pv_data, original_pv_data);
        clear_vb_error();

        // Advance iteration: next element is 20
        let has_next = __vbaNextEachAry(
            3,
            core::ptr::null_mut(),
            &mut current_elem,
            &mut enum_state as *mut _ as *mut c_void,
        );
        assert_eq!(has_next, -1);
        assert_eq!(current_elem.to_i32(), Some(20));
        assert_eq!((*psa).c_locks, 1);

        // Exit loop early: __vbaExitEachAry decrements c_locks back to 0
        __vbaExitEachAry(&mut enum_state as *mut _ as *mut c_void);
        assert_eq!((*psa).c_locks, 0);

        // Now that the array is unlocked, ReDim succeeds!
        let res_redim_success = __vbaRedim(0, 4, &mut psa_ptr, VT_I4 as u32, 1, 10, 1);
        assert_eq!(res_redim_success, 0);
        assert_eq!(get_vb_error_number(), 0);
        assert_eq!((*psa_ptr).rgsabound[0].c_elements, 10);

        // Clean up
        SafeArray::destroy(psa_ptr);
    }
}

/// Test 3: Memory Aliasing When Same Variable Passed ByRef to Multiple Arguments
///
/// GIVEN Sub SwapOrAdd(ByRef a As Long, ByRef b As Long): a = a + 1: b = b * 2: End Sub
/// WHEN invoked with the same variable aliased: Call SwapOrAdd(x, x)
/// THEN mutations to 'a' must immediately be visible when reading 'b'
/// AND final value must strictly adhere to VB6 sequential evaluation without intermediate caching.
#[test]
fn test_memory_aliasing_when_same_variable_passed_byref_to_multiple_arguments() {
    let _guard = LAYER17_MUTEX.lock().unwrap();

    // Routine matching:
    // Sub AliasedSwapOrAdd(ByRef A As Long, ByRef B As Long)
    //     A = A + 1
    //     B = B * 2
    // End Sub
    unsafe fn aliased_swap_or_add(a: *mut i32, b: *mut i32) {
        // Read A, increment by 1, write back
        let mut val_a: u32 = 0;
        GetMem4(a as *const u32, &mut val_a);
        val_a += 1;
        PutMem4(a as *mut u32, val_a);

        // Since B points to the exact same memory address as A,
        // reading B must immediately observe the mutation (val_a)
        let mut val_b: u32 = 0;
        GetMem4(b as *const u32, &mut val_b);
        assert_eq!(val_b, val_a);

        val_b *= 2;
        PutMem4(b as *mut u32, val_b);
    }

    let mut x: i32 = 10;
    unsafe {
        // Call AliasedSwapOrAdd(x, x)
        aliased_swap_or_add(&mut x as *mut i32, &mut x as *mut i32);
    }

    // Initial: 10
    // After A = A + 1: 11
    // After B = B * 2: 11 * 2 = 22
    assert_eq!(x, 22);
}

/// Test 4: ByVal Variant Containing ByRef Variant Deep Dereferencing
///
/// GIVEN a nested Variant structure (VT_BYREF | VT_VARIANT pointing to VT_BYREF | VT_I4)
/// WHEN passed ByVal to a function accepting Variant
/// THEN the runtime must resolve deep indirection to the root primitive value
/// AND modifying the ByVal parameter must not mutate the original memory address.
#[test]
fn test_byval_variant_containing_byref_variant_deep_dereferencing() {
    let _guard = LAYER17_MUTEX.lock().unwrap();

    let mut root_val: i32 = 50;

    // Layer 1: ByRef Long (VT_BYREF | VT_I4)
    let layer1_var = Variant::byref_i32(&mut root_val);
    assert_eq!(layer1_var.vt, VT_BYREF | VT_I4);

    // Layer 2: ByRef Variant pointing to Layer 1 (VT_BYREF | VT_VARIANT)
    let layer2_var = Variant::byref_variant(&layer1_var);
    assert_eq!(layer2_var.vt, VT_BYREF | VT_VARIANT);

    // Layer 3: Nested ByRef Variant pointing to Layer 2 (VT_BYREF | VT_VARIANT)
    let layer3_var = Variant::byref_variant(&layer2_var);
    assert_eq!(layer3_var.vt, VT_BYREF | VT_VARIANT);

    // 1. Test deep dereferencing to root primitive
    let dereferenced = layer3_var.deref_variant();
    assert_eq!(dereferenced.vt, VT_BYREF | VT_I4);
    assert_eq!(dereferenced.as_i32(), Some(50));

    // 2. Test passing ByVal: Variant is copied with deep indirection collapsed
    let mut byval_copy = Variant::empty();
    unsafe {
        __vbaVarMove(&mut byval_copy, &layer3_var);
    }
    assert_eq!(byval_copy.vt, VT_I4);
    assert_eq!(byval_copy.as_i32(), Some(50));

    // 3. Modifying the ByVal copy must NOT mutate the root memory address
    byval_copy = Variant::from_i32(999);
    assert_eq!(byval_copy.as_i32(), Some(999));
    assert_eq!(root_val, 50); // Original remains strictly 50!
}

/// Test 5: Cascading On Error GoTo Across Multiple Re-entrant SEH Frames
///
/// GIVEN Procedure A with 'On Error GoTo CatchA' calling Procedure B with NO error handler,
/// which in turn calls Procedure C with 'On Error Resume Next'
/// WHEN an unhandled error occurs in C after disabling Resume Next ('On Error GoTo 0')
/// THEN the runtime error propagation must unwind C, unwind B, and cleanly land on CatchA in Procedure A
/// AND the Thread-Local SEH frame stack must accurately restore to A's frame depth.
#[test]
fn test_cascading_on_error_goto_across_multiple_reentrant_seh_frames() {
    let _guard = LAYER17_MUTEX.lock().unwrap();
    clear_vb_error();
    clear_seh_frames();

    assert_eq!(get_seh_frame_depth(), 0);

    // Procedure A: Establishes 'On Error GoTo CatchA' (catch target: 0x1000)
    let frame_a_depth = push_seh_frame(1, 0x1000);
    assert_eq!(frame_a_depth, 1);
    assert_eq!(get_vb_error_handler(), 1);

    // Procedure A calls Procedure B: NO error handler ('On Error GoTo 0')
    let frame_b_depth = push_seh_frame(0, 0);
    assert_eq!(frame_b_depth, 2);
    assert_eq!(get_vb_error_handler(), 0);

    // Procedure B calls Procedure C: Establishes 'On Error Resume Next'
    let frame_c_depth = push_seh_frame(-1, 0);
    assert_eq!(frame_c_depth, 3);
    assert_eq!(get_vb_error_handler(), -1);

    // Inside Procedure C: Disables Resume Next via 'On Error GoTo 0'
    unsafe {
        __vbaOnError(core::ptr::null_mut());
    }
    assert_eq!(get_vb_error_handler(), 0);

    // Procedure C triggers unhandled Runtime Error 53 ("File not found")
    let unwind_result = unwind_to_handler(53);

    // Error propagation must unwind C and B, and land on CatchA in Procedure A
    assert!(unwind_result.is_ok());
    let landing_frame = unwind_result.unwrap();
    assert_eq!(landing_frame.frame_id, 1);
    assert_eq!(landing_frame.catch_target, 0x1000);
    assert_eq!(landing_frame.handler_mode, 1);

    // SEH frame stack must accurately restore to Procedure A's frame depth
    assert_eq!(get_seh_frame_depth(), 1);
    assert_eq!(get_vb_error_number(), 53);
    assert_eq!(get_vb_error_description(), "File not found");

    // Procedure A exits cleanly: __vbaExitProc pops Procedure A's frame
    unsafe {
        __vbaExitProc();
    }
    assert_eq!(get_seh_frame_depth(), 0);
    clear_vb_error();
}

/// Test 6: Circular Reference Breakup and Deterministic Heap Reclamation
///
/// GIVEN Object A referencing Object B, and Object B referencing Object A (circular cycle)
/// WHEN explicitly breaking the cycle by setting A.Child = Nothing
/// THEN both objects' reference counts must hit zero
/// AND Class_Terminate must execute for both objects in deterministic order.
#[test]
fn test_circular_reference_breakup_and_deterministic_heap_reclamation() {
    let _guard = LAYER17_MUTEX.lock().unwrap();

    let node_a_terminated = Arc::new(AtomicBool::new(false));
    let node_b_terminated = Arc::new(AtomicBool::new(false));
    let destruction_order: Arc<std::sync::Mutex<Vec<String>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));

    let term_a = Arc::clone(&node_a_terminated);
    let term_b = Arc::clone(&node_b_terminated);
    let order_a = Arc::clone(&destruction_order);
    let order_b = Arc::clone(&destruction_order);

    unsafe {
        // Allocate NodeA and NodeB
        let node_a = VbClassInstance::new("NodeA");
        let node_b = VbClassInstance::new("NodeB");

        (*node_a).terminate_action = Some(Box::new(move |_| {
            term_a.store(true, Ordering::SeqCst);
            order_a.lock().unwrap().push("NodeA".to_string());
        }));

        (*node_b).terminate_action = Some(Box::new(move |_| {
            term_b.store(true, Ordering::SeqCst);
            order_b.lock().unwrap().push("NodeB".to_string());
        }));

        // NodeA.Partner = NodeB (AddRef NodeB)
        VbClassInstance::add_ref(node_b as *mut c_void);
        (*node_a).partner = Some(node_b);

        // NodeB.Partner = NodeA (AddRef NodeA)
        VbClassInstance::add_ref(node_a as *mut c_void);
        (*node_b).partner = Some(node_a);

        // At this point, both have ref_count = 2 (1 root + 1 cycle)
        assert_eq!((*node_a).ref_count.load(Ordering::SeqCst), 2);
        assert_eq!((*node_b).ref_count.load(Ordering::SeqCst), 2);

        // BREAK THE CYCLE: Set NodeA.Partner = Nothing
        let old_partner = (*node_a).partner.take().unwrap();
        VbClassInstance::release(old_partner as *mut c_void);

        // Now NodeB ref_count drops to 1, NodeA still 2
        assert_eq!((*node_b).ref_count.load(Ordering::SeqCst), 1);
        assert_eq!((*node_a).ref_count.load(Ordering::SeqCst), 2);

        // Release root reference to NodeA (Set nA = Nothing)
        let rem_a = VbClassInstance::release(node_a as *mut c_void);
        assert_eq!(rem_a, 1); // Still held by NodeB.Partner

        // Release root reference to NodeB (Set nB = Nothing)
        // This drops NodeB to 0 -> NodeB Class_Terminate runs!
        // During NodeB teardown, it releases its child NodeA -> NodeA drops to 0 -> NodeA Class_Terminate runs!
        let rem_b = VbClassInstance::release(node_b as *mut c_void);
        assert_eq!(rem_b, 0);

        // Both nodes must have terminated deterministically
        assert!(node_b_terminated.load(Ordering::SeqCst));
        assert!(node_a_terminated.load(Ordering::SeqCst));

        let order = destruction_order.lock().unwrap().clone();
        assert_eq!(order, vec!["NodeB".to_string(), "NodeA".to_string()]);
    }
}
