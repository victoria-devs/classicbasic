// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Native 32-bit x86 Execution Engine Tests
//!
//! Validates the clean-room 32-bit x86 CPU emulator (`X86Cpu`), instruction set interpreter,
//! EFLAGS arithmetic logic, Win32 `FS:[0]` Thread Information Block (TIB), SEH exception unwinding,
//! x87 FPU pipeline, and COM vtable method dispatch.

use msvbvm60::x86::*;

#[test]
fn test_registers_and_sub_registers() {
    let mut regs = Registers::default();
    regs.set_reg32(0, 0x12345678); // EAX
    assert_eq!(regs.eax, 0x12345678);
    assert_eq!(regs.get_reg16(0), 0x5678);
    assert_eq!(regs.get_reg8(0), 0x78); // AL
    assert_eq!(regs.get_reg8(4), 0x56); // AH

    regs.set_reg16(0, 0xABCD);
    assert_eq!(regs.eax, 0x1234ABCD);

    regs.set_reg8(0, 0xEE);
    assert_eq!(regs.eax, 0x1234ABEE);

    regs.set_reg8(4, 0x99);
    assert_eq!(regs.eax, 0x123499EE);
}

#[test]
fn test_eflags_and_conditional_branches() {
    let mut flags = Flags::new();

    // 10 - 10 = 0 (ZF=1, CF=0, SF=0)
    flags.update_sub_32(10, 10, 0);
    assert!(flags.zf());
    assert!(!flags.cf());
    assert!(!flags.sf());
    assert!(flags.eval_condition(0x4)); // JE / JZ
    assert!(!flags.eval_condition(0x5)); // JNE / JNZ

    // 5 - 10 = -5 (ZF=0, CF=1, SF=1)
    flags.update_sub_32(5, 10, (5u32).wrapping_sub(10));
    assert!(!flags.zf());
    assert!(flags.cf());
    assert!(flags.sf());
    assert!(flags.eval_condition(0x2)); // JB / JC
    assert!(flags.eval_condition(0xC)); // JL

    // Logic: 0x55 & 0xAA = 0 (ZF=1, CF=0, OF=0)
    flags.update_logic_32(0x55 & 0xAA);
    assert!(flags.zf());
    assert!(!flags.cf());
    assert!(!flags.of());
}

#[test]
fn test_alu_arithmetic_instructions() {
    let mut cpu = X86Cpu::new();
    // Test: ((1000 * 3) + (350 * 2) - (75 * 4)) \ 5 = 680
    // Uses EBX as accumulator to avoid being clobbered by EDX:EAX multiply/divide
    let code = vec![
        0xB8, 0xE8, 0x03, 0x00, 0x00, // mov eax, 1000
        0xB9, 0x03, 0x00, 0x00, 0x00, // mov ecx, 3
        0xF7, 0xE9,                   // imul ecx (eax = 3000)
        0x89, 0xC3,                   // mov ebx, eax (ebx = 3000)
        0xB8, 0x5E, 0x01, 0x00, 0x00, // mov eax, 350
        0xB9, 0x02, 0x00, 0x00, 0x00, // mov ecx, 2
        0xF7, 0xE9,                   // imul ecx (eax = 700)
        0x01, 0xC3,                   // add ebx, eax (ebx = 3700)
        0xB8, 0x4B, 0x00, 0x00, 0x00, // mov eax, 75
        0xB9, 0x04, 0x00, 0x00, 0x00, // mov ecx, 4
        0xF7, 0xE9,                   // imul ecx (eax = 300)
        0x29, 0xC3,                   // sub ebx, eax (ebx = 3400)
        0x89, 0xD8,                   // mov eax, ebx (eax = 3400)
        0x99,                         // cdq
        0xB9, 0x05, 0x00, 0x00, 0x00, // mov ecx, 5
        0xF7, 0xF9,                   // idiv ecx (eax = 680)
        0xF4,                         // hlt
    ];
    cpu.mem.load_code(0x00401000, &code);
    cpu.regs.eip = 0x00401000;
    cpu.run(100).expect("ALU code execution failed");
    assert_eq!(cpu.regs.eax, 680);
    assert_eq!(cpu.regs.edx, 0);
}

#[test]
fn test_bitwise_operations() {
    let mut cpu = X86Cpu::new();
    // (1000 And &HFF) Or (350 And &H0F) Xor &HAA = 68
    let code = vec![
        0xB8, 0xE8, 0x03, 0x00, 0x00, // mov eax, 1000
        0x81, 0xE0, 0xFF, 0x00, 0x00, 0x00, // and eax, 0xFF
        0xB9, 0x5E, 0x01, 0x00, 0x00, // mov ecx, 350
        0x81, 0xE1, 0x0F, 0x00, 0x00, 0x00, // and ecx, 0x0F
        0x09, 0xC8,                   // or eax, ecx
        0x81, 0xF0, 0xAA, 0x00, 0x00, 0x00, // xor eax, 0xAA
        0xF4,                         // hlt
    ];
    cpu.mem.load_code(0x00401000, &code);
    cpu.regs.eip = 0x00401000;
    cpu.run(100).expect("Bitwise execution failed");
    assert_eq!(cpu.regs.eax, 68);
}

#[test]
fn test_stdcall_byref_stack_frame() {
    let mut cpu = X86Cpu::new();
    let target_var_addr = 0x001FFF10;
    cpu.mem.write_u32(target_var_addr, 50);

    // ScaleByRef(ByRef target, ByVal multiplier, ByVal addend) -> (50 * 6) + 20 = 320
    let code = vec![
        // Caller:
        0x68, 0x14, 0x00, 0x00, 0x00, // push 20 (addend)
        0x68, 0x06, 0x00, 0x00, 0x00, // push 6  (multiplier)
        0x68, 0x10, 0xFF, 0x1F, 0x00, // push target_var_addr
        0xE8, 0x04, 0x00, 0x00, 0x00, // call +4 (to ScaleByRef)
        0xF4,                         // hlt
        0x90, 0x90, 0x90,             // padding
        // ScaleByRef at 0x00401017:
        0x8B, 0x44, 0x24, 0x04,       // mov eax, [esp + 4]
        0x8B, 0x08,                   // mov ecx, [eax]
        0x8B, 0x54, 0x24, 0x08,       // mov edx, [esp + 8]
        0x89, 0xC8,                   // mov eax, ecx
        0xF7, 0xEA,                   // imul edx
        0x03, 0x44, 0x24, 0x0C,       // add eax, [esp + 12]
        0x8B, 0x4C, 0x24, 0x04,       // mov ecx, [esp + 4]
        0x89, 0x01,                   // mov [ecx], eax
        0xC2, 0x0C, 0x00,             // ret 12
    ];
    cpu.mem.load_code(0x00401000, &code);
    cpu.regs.eip = 0x00401000;
    cpu.run(100).expect("ScaleByRef execution failed");
    assert_eq!(cpu.mem.read_u32(target_var_addr), 320);
}

#[test]
fn test_loop_and_jump_conditionals() {
    let mut cpu = X86Cpu::new();
    // Sum of even numbers from 2 to 100 = 2550
    let code = vec![
        0xB8, 0x00, 0x00, 0x00, 0x00, // mov eax, 0
        0xB9, 0x01, 0x00, 0x00, 0x00, // mov ecx, 1
        // loop_head (0x0040100A):
        0x89, 0xCA,                   // mov edx, ecx
        0x81, 0xE2, 0x01, 0x00, 0x00, 0x00, // and edx, 1
        0x75, 0x02,                   // jnz +2 (to inc ecx)
        0x01, 0xC8,                   // add eax, ecx
        // skip (0x00401016):
        0x41,                         // inc ecx
        0x81, 0xF9, 0x64, 0x00, 0x00, 0x00, // cmp ecx, 100
        0x7E, 0xEB,                   // jle loop_head (-21 bytes -> 0x0040100A)
        0xF4,                         // hlt
    ];
    cpu.mem.load_code(0x00401000, &code);
    cpu.regs.eip = 0x00401000;
    cpu.run(10_000).expect("Loop execution failed");
    assert_eq!(cpu.regs.eax, 2550);
}

#[test]
fn test_x87_fpu_pipeline_and_transcendental() {
    let mut cpu = X86Cpu::new();
    // Test Sqr(16) + Sqr(9) + ((16 * 9) / (16 - 9))
    cpu.fpu.push(16.0);
    cpu.fpu.fsqrt();
    assert_eq!(cpu.fpu.get_st(0), 4.0);

    cpu.fpu.push(9.0);
    cpu.fpu.fsqrt();
    assert_eq!(cpu.fpu.get_st(0), 3.0);

    let t1 = cpu.fpu.pop();
    let t2 = cpu.fpu.pop();
    let t3 = (16.0 * 9.0) / (16.0 - 9.0);
    let res = t1 + t2 + t3;
    assert!((res - 27.5714285714286).abs() < 1e-5);

    // Transcendental Atn(1) * 4 = pi
    cpu.fpu.push(1.0);
    cpu.fpu.push(1.0);
    cpu.fpu.fpatan();
    let atn1 = cpu.fpu.pop();
    let pi = 4.0 * atn1;
    assert!((pi - 3.14159265358979).abs() < 1e-6);

    // Sin(pi / 6) = 0.5
    cpu.fpu.push(pi / 6.0);
    cpu.fpu.fsin();
    let s = cpu.fpu.pop();
    assert!((s - 0.5).abs() < 1e-6);
}

#[test]
fn test_win32_tib_and_seh_dispatch() {
    let mut cpu = X86Cpu::new();
    let handler_addr = 0x00402000;
    let seh_record_addr = 0x001FFF00;

    // Verify TIB defaults:
    // FS:[0x00] = 0xFFFFFFFF, FS:[0x04] = STACK_BASE, FS:[0x08] = STACK_LIMIT
    assert_eq!(cpu.mem.read_u32(cpu.regs.fs_base), 0xFFFFFFFF);
    assert_eq!(cpu.mem.read_u32(cpu.regs.fs_base + 4), STACK_BASE);
    assert_eq!(cpu.mem.read_u32(cpu.regs.fs_base + 8), STACK_LIMIT);

    // Register an EXCEPTION_REGISTRATION_RECORD at FS:[0]
    cpu.mem.write_u32(seh_record_addr, 0xFFFFFFFF);
    cpu.mem.write_u32(seh_record_addr + 4, handler_addr);
    cpu.mem.write_u32(cpu.regs.fs_base, seh_record_addr);

    let handler_code = vec![
        0xB8, 0x0B, 0x00, 0x00, 0x00, // mov eax, 11 (caught division by zero)
        0xF4,                         // hlt
    ];
    cpu.mem.load_code(handler_addr, &handler_code);

    // Trigger SEH exception 11
    cpu.trigger_seh_exception(11).expect("SEH dispatch failed");
    assert_eq!(cpu.regs.eip, handler_addr);
    cpu.run(10).expect("Handler execution failed");
    assert_eq!(cpu.regs.eax, 11);
}

#[test]
fn test_native_hook_interception() {
    let mut cpu = X86Cpu::new();
    let hook_addr = 0x70001000;

    // Register a native hook that increments EAX by 100 and returns
    cpu.register_hook(hook_addr, |cpu| {
        cpu.regs.eax += 100;
        cpu.regs.eip = cpu.pop32(); // ret
        Ok(())
    });

    let code = vec![
        0xB8, 0x0A, 0x00, 0x00, 0x00, // mov eax, 10
        0x68, 0x00, 0x10, 0x00, 0x70, // push hook_addr
        0x5A,                         // pop edx
        0xFF, 0xD2,                   // call edx (calls hook_addr)
        0xF4,                         // hlt
    ];
    cpu.mem.load_code(0x00401000, &code);
    cpu.regs.eip = 0x00401000;
    cpu.run(100).expect("Hook execution failed");
    assert_eq!(cpu.regs.eax, 110);
}

#[test]
fn test_native_class_vtable_method_dispatch() {
    let mut cpu = X86Cpu::new();
    let inst_addr = 0x00105000;
    let vtable_addr = 0x00104000;
    cpu.mem.write_u32(inst_addr, vtable_addr);
    cpu.mem.write_u32(inst_addr + 4, 40); // m_BaseValue = 40

    // ComputeLinear(factor, delta): (40 * 5) + 30 = 230
    let method_addr = 0x00403000;
    let method_code = vec![
        0x8B, 0x4C, 0x24, 0x04,       // mov ecx, [esp + 4] (this)
        0x8B, 0x41, 0x04,             // mov eax, [ecx + 4] (m_BaseValue = 40)
        0x8B, 0x54, 0x24, 0x08,       // mov edx, [esp + 8] (factor = 5)
        0xF7, 0xEA,                   // imul edx (40 * 5 = 200)
        0x03, 0x44, 0x24, 0x0C,       // add eax, [esp + 12] (200 + 30 = 230)
        0xC2, 0x0C, 0x00,             // ret 12
    ];
    cpu.mem.load_code(method_addr, &method_code);
    cpu.mem.write_u32(vtable_addr + 20, method_addr);

    let caller_code = vec![
        0x68, 0x1E, 0x00, 0x00, 0x00, // push 30 (delta)
        0x68, 0x05, 0x00, 0x00, 0x00, // push 5  (factor)
        0x68, 0x00, 0x50, 0x10, 0x00, // push inst_addr (this)
        0xB8, 0x00, 0x50, 0x10, 0x00, // mov eax, inst_addr
        0x8B, 0x00,                   // mov eax, [eax] (eax = vtable_addr)
        0xFF, 0x50, 0x14,             // call dword ptr [eax + 20]
        0xF4,                         // hlt
    ];
    cpu.mem.load_code(0x00401000, &caller_code);
    cpu.regs.eip = 0x00401000;
    cpu.run(100).expect("Method dispatch failed");
    assert_eq!(cpu.regs.eax, 230);
}

#[test]
fn test_app34_complete_suite_execution() {
    unsafe {
        msvbvm60::thun_rt::run_app34_native_x86_engine_tests();
    }
}

#[test]
fn test_app43_native_vtable_event_dispatch() {
    let exe_path = "../../artifacts/bin/App43.exe";
    let fallback_path = "artifacts/bin/App43.exe";
    let path = if std::path::Path::new(exe_path).exists() {
        exe_path
    } else if std::path::Path::new(fallback_path).exists() {
        fallback_path
    } else {
        return;
    };

    let data = std::fs::read(path).expect("read App43.exe");
    let loaded_pe = msvbvm60::pe::LoadedPe::load(&data).expect("load PE");

    let vb_hdr_ptr = loaded_pe.find_vb_header().expect("find VBHeader");
    let vb_hdr = unsafe { *vb_hdr_ptr };
    let form = unsafe {
        cb_win32::FormDescriptor::from_pe_gui_table(
            loaded_pe.mapped_base as *const u8,
            loaded_pe.mapped_size,
            loaded_pe.preferred_base,
            vb_hdr.lp_gui_table,
        )
    }
    .expect("deserialize form");

    cb_win32::ensure_default_form_runtime();
    {
        let mut lock = cb_win32::global_form_runtime().lock().unwrap();
        *lock = Some(cb_win32::FormRuntime::new(form));
        if let Some(rt) = lock.as_mut() {
            rt.set_control_text("", "");
        }
    }

    // 1. Click "1" -> executes native Command_one_Click at 0x403110 -> calls WriteSymbol(1)
    let res = msvbvm60::thun_rt::dispatch_native_control_click("Command_one").expect("click one");
    assert_eq!(res, "1");

    // 2. Click "+" -> executes native Command_plus_Click at 0x402bf0
    let res = msvbvm60::thun_rt::dispatch_native_control_click("Command_plus").expect("click plus");
    assert_eq!(res, "1 + ");

    // 3. Click "5" -> executes native Command_five_Click at 0x402a70 -> calls WriteSymbol(5)
    let res = msvbvm60::thun_rt::dispatch_native_control_click("Command_five").expect("click five");
    assert_eq!(res, "1 + 5");

    // 4. Click "=" -> executes native Command_equals_Click at 0x403410
    // Instantiates MSScriptControl.ScriptControl, sets Language=VBScript, calls Eval("1 + 5"), updates TextBox!
    let res = msvbvm60::thun_rt::dispatch_native_control_click("Command_equals").expect("click equals");
    assert_eq!(res, "6");
}

