//! Layer 17: Esoteric Runtime Quirks, Zombie Objects & Memory Aliasing Tests
//!
//! Declarative unit and integration test specifications for the hardening phase of
//! `msvbvm60` covering:
//! 1. Zombie object state transition and invocation lockout (`E_UNEXPECTED`)
//! 2. SafeArray lock violation during ByRef iteration (Runtime Error 10)
//! 3. Memory aliasing with shared ByRef variable parameters
//! 4. Deep dereferencing of nested ByRef Variant structures
//! 5. Cascading On Error GoTo across multiple re-entrant SEH frames
//! 6. Circular reference breakup and deterministic heap reclamation

use core::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use msvbvm60::com::*;
use msvbvm60::err::*;
use msvbvm60::mem::*;
use msvbvm60::types::*;
use msvbvm60::vba::*;

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
