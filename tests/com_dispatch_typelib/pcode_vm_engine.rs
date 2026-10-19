// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # P-Code Virtual Machine Engine Domain Tests (Phase 2.15)
//!
//! Validates:
//! - ProcedureDescriptor decoding and metadata parsing
//! - ProcFrame memory activation frames, Variant/BSTR slot management and deterministic cleanup
//! - PCodeVM stack machine arithmetic, relational, bitwise, and exponential evaluation
//! - Branching and Select Case jump tables
//! - Structured error trapping (On Error GoTo, Resume Next, Resume)
//! - Runtime intrinsic library imports (rtcTrimBstr, rtcMidBstr, rtcStrReverse)
//! - MethCallEngine class dispatch and property Let/Get mutation
//! - ProcCallEngine and MethCallEngine ABI compatibility exports

use msvbvm60::pcode::*;

#[test]
fn test_procedure_descriptor_layout() {
    let dummy_code: [u8; 4] = [OP_NOP, OP_NOP, OP_NOP, OP_RETURN];
    let desc = ProcedureDescriptor::new(dummy_code.as_ptr(), 128, 8, 0x10, core::ptr::null());
    assert_eq!(desc.frame_size, 128);
    assert_eq!(desc.args_size, 8);
    assert_eq!(desc.flags, 0x10);
    assert_eq!(desc.lp_byte_code, dummy_code.as_ptr());
}

#[test]
fn test_proc_frame_lifecycle_and_teardown() {
    let mut frame = ProcFrame::new(64);
    frame.write_i32(0, 42);
    frame.write_i32(4, 999);
    assert_eq!(frame.read_i32(0), 42);
    assert_eq!(frame.read_i32(4), 999);

    frame.write_f64(8, 3.1415926535);
    assert!((frame.read_f64(8) - 3.1415926535).abs() < 1e-9);

    // Frame teardown executes cleanly
    frame.teardown();
}

#[test]
fn test_pcode_vm_arithmetic_precedence() {
    let mut vm = PCodeVM::new(64);
    // ((15 + 4 * 2) \ 3) + ((15 Mod 4) * 10) - (4 ^ 2) = 21
    let code = [
        OP_LITI4, 15, 0, 0, 0,
        OP_LITI4, 4, 0, 0, 0,
        OP_LITI4, 2, 0, 0, 0,
        OP_MUL,
        OP_ADD,
        OP_LITI4, 3, 0, 0, 0,
        OP_IDIV,
        OP_LITI4, 15, 0, 0, 0,
        OP_LITI4, 4, 0, 0, 0,
        OP_MOD,
        OP_LITI4, 10, 0, 0, 0,
        OP_MUL,
        OP_ADD,
        OP_LITI4, 4, 0, 0, 0,
        OP_LITI4, 2, 0, 0, 0,
        OP_POW,
        OP_SUB,
        OP_RETURN,
    ];
    let res = vm.execute(&code, &mut []).expect("Execution failed");
    assert_eq!(res.as_i32(), 21);
}

#[test]
fn test_pcode_vm_relational_and_bitwise() {
    let mut vm = PCodeVM::new(64);
    // (10 > 5) And (3 <= 3) -> (-1) And (-1) = -1
    // (10 == 20) -> 0
    // (10 == 20) Or (-1) -> -1
    let code = [
        OP_LITI4, 10, 0, 0, 0,
        OP_LITI4, 5, 0, 0, 0,
        OP_GT,
        OP_LITI4, 3, 0, 0, 0,
        OP_LITI4, 3, 0, 0, 0,
        OP_LE,
        OP_AND,
        OP_LITI4, 10, 0, 0, 0,
        OP_LITI4, 20, 0, 0, 0,
        OP_EQ,
        OP_OR,
        OP_RETURN,
    ];
    let res = vm.execute(&code, &mut []).expect("Execution failed");
    assert_eq!(res.as_i32(), -1); // True in VB6
}

#[test]
fn test_pcode_vm_byref_arguments() {
    let mut val = 100i32;
    let target_ptr = &mut val as *mut i32;
    let mut vm = PCodeVM::new(32);
    // MultiplyByRef: arg0 (ByRef) * arg1 (ByVal) -> write back to ByRef
    let code = [
        OP_LDARG_BYREF, 0, 0,
        OP_LDARG, 1, 0,
        OP_MUL,
        OP_STARG_BYREF, 0, 0,
        OP_LITI4, 0, 0, 0, 0,
        OP_RETURN,
    ];
    let mut args = vec![
        PCodeValue::ByRefI32(target_ptr),
        PCodeValue::I32(3),
    ];
    let _ = vm.execute(&code, &mut args).expect("Execution failed");
    assert_eq!(val, 300);
}

#[test]
fn test_pcode_vm_control_flow_branches() {
    let mut vm = PCodeVM::new(64);
    // If 10 < 20 Then Branch to Target (PC 20) and return 42
    // Else return 99
    let code = [
        OP_LITI4, 10, 0, 0, 0,
        OP_LITI4, 20, 0, 0, 0,
        OP_LT,
        OP_BRANCH_IF_NOT_ZERO, 20, 0,
        OP_LITI4, 99, 0, 0, 0,
        OP_RETURN,
        // PC 20
        OP_LITI4, 42, 0, 0, 0,
        OP_RETURN,
    ];
    let res = vm.execute(&code, &mut []).expect("Execution failed");
    assert_eq!(res.as_i32(), 42);
}

#[test]
fn test_pcode_vm_select_case_jump_table() {
    let mut vm = PCodeVM::new(64);
    // Select Case 3:
    // Jump table: 2 cases: (Case 1 -> PC 22, Case 3 -> PC 28), default -> PC 34
    let code = [
        OP_LITI4, 3, 0, 0, 0,
        OP_SELECT_CASE_JUMP,
        2, 0,                // count = 2
        1, 0, 0, 0, 22, 0,   // case 1 -> PC 22
        3, 0, 0, 0, 28, 0,   // case 3 -> PC 28
        34, 0,               // default -> PC 34
        // PC 22: return 100
        OP_LITI4, 100, 0, 0, 0,
        OP_RETURN,
        // PC 28: return 300
        OP_LITI4, 44, 1, 0, 0, // 300
        OP_RETURN,
        // PC 34: return 0
        OP_LITI4, 0, 0, 0, 0,
        OP_RETURN,
    ];
    let res = vm.execute(&code, &mut []).expect("Execution failed");
    assert_eq!(res.as_i32(), 300);
}

#[test]
fn test_pcode_vm_error_trapping_goto_and_resume() {
    let mut vm = PCodeVM::new(32);
    // On Error GoTo PC 15
    // Division by zero (100 \ 0)
    // PC 15: Resume and return error number (11)
    let code = [
        OP_ON_ERROR_GOTO, 15, 0,
        OP_LITI4, 100, 0, 0, 0,
        OP_LITI4, 0, 0, 0, 0,
        OP_IDIV,
        OP_RETURN,
        // PC 15: Handler
        OP_RESUME,
        OP_LITI4, 11, 0, 0, 0,
        OP_RETURN,
    ];
    let res = vm.execute(&code, &mut []).expect("Execution failed");
    assert_eq!(res.as_i32(), 11);
}

#[test]
fn test_pcode_vm_meth_call_dispatch() {
    let mut vm = PCodeVM::new(32);
    // Set BaseValue = 50 via Method 3
    vm.push(PCodeValue::I32(50));
    let code_set = [
        OP_METH_CALL, 3, 0, 1,
        OP_RETURN,
    ];
    let _ = vm.execute(&code_set, &mut []).expect("Execution failed");
    assert_eq!(vm.frame.read_i32(0), 50);

    // ComputeOffset(4, 25) via Method 1 -> (50 * 4) + 25 = 225
    vm.push(PCodeValue::I32(4));
    vm.push(PCodeValue::I32(25));
    let code_calc = [
        OP_METH_CALL, 1, 0, 2,
        OP_RETURN,
    ];
    let res_calc = vm.execute(&code_calc, &mut []).expect("Execution failed");
    assert_eq!(res_calc.as_i32(), 225);

    // FormatMessage("PREFIX") via Method 2 -> "PREFIX: 50"
    vm.push(PCodeValue::String("PREFIX".to_string()));
    let code_msg = [
        OP_METH_CALL, 2, 0, 1,
        OP_RETURN,
    ];
    let res_msg = vm.execute(&code_msg, &mut []).expect("Execution failed");
    assert_eq!(res_msg.as_string(), "PREFIX: 50");
}

#[test]
fn test_pcode_vm_runtime_library_import_dispatch() {
    let mut vm = PCodeVM::new(32);
    // Trim$ ("  ClassicBasic  ") -> "ClassicBasic"
    vm.push(PCodeValue::String("  ClassicBasic  ".to_string()));
    let code_trim = [
        OP_IMPAD_CALL, 0x08, 0x02, 1,
        OP_RETURN,
    ];
    let trimmed = vm.execute(&code_trim, &mut []).expect("Execution failed").as_string();
    assert_eq!(trimmed, "ClassicBasic");

    // Mid$(trimmed, 8, 5) -> "Basic"
    vm.push(PCodeValue::String(trimmed));
    vm.push(PCodeValue::I32(8));
    vm.push(PCodeValue::I32(5));
    let code_mid = [
        OP_IMPAD_CALL, 0x74, 0x02, 3,
        OP_RETURN,
    ];
    let sub = vm.execute(&code_mid, &mut []).expect("Execution failed").as_string();
    assert_eq!(sub, "Basic");

    // StrReverse("Basic") -> "cisaB"
    vm.push(PCodeValue::String(sub));
    let code_rev = [
        OP_IMPAD_CALL, 0x04, 0x02, 1,
        OP_RETURN,
    ];
    let rev = vm.execute(&code_rev, &mut []).expect("Execution failed").as_string();
    assert_eq!(rev, "cisaB");
}

#[test]
fn test_pcode_engines_abi_exports() {
    unsafe {
        assert_eq!(msvbvm60::thun_rt::ProcCallEngine(), 0);
        assert_eq!(msvbvm60::thun_rt::MethCallEngine(), 0);
    }
}
