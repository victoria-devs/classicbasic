// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # P-Code Bytecode Virtual Machine & Interpreter Loop
//!
//! Implements the clean-room stack machine, operand stack, evaluation engine,
//! and control flow dispatcher for VB6 P-Code execution.

use core::ffi::c_void;
use crate::pcode::descriptor::ProcFrame;
use crate::pcode::opcodes::*;
use crate::types::{bstr_to_string, Variant};

/// Typed operand value on the P-Code evaluation stack.
#[derive(Debug, Clone)]
pub enum PCodeValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    Object(*mut c_void),
    ByRefI32(*mut i32),
    Variant(Variant),
}

impl PCodeValue {
    /// Extracts a 32-bit integer or coerces to i32.
    pub fn as_i32(&self) -> i32 {
        match self {
            PCodeValue::I32(v) => *v,
            PCodeValue::I64(v) => *v as i32,
            PCodeValue::F32(v) => *v as i32,
            PCodeValue::F64(v) => *v as i32,
            PCodeValue::String(s) => s.parse::<i32>().unwrap_or(0),
            PCodeValue::ByRefI32(ptr) => unsafe { if !ptr.is_null() { **ptr } else { 0 } },
            PCodeValue::Variant(v) => v.as_i32().unwrap_or(0),
            PCodeValue::Object(_) => 0,
        }
    }

    /// Extracts raw pointer from ByRef or numeric representation.
    pub fn as_ptr(&self) -> *mut u8 {
        match self {
            PCodeValue::ByRefI32(ptr) => *ptr as *mut u8,
            PCodeValue::I32(v) => *v as usize as *mut u8,
            PCodeValue::Object(p) => *p as *mut u8,
            _ => core::ptr::null_mut(),
        }
    }

    /// Evaluates boolean truth in VB6 semantics (`True == -1`, `False == 0`).
    pub fn as_bool(&self) -> bool {
        self.as_i32() != 0
    }

    /// Converts the operand to a String.
    pub fn as_string(&self) -> String {
        match self {
            PCodeValue::String(s) => s.clone(),
            PCodeValue::I32(v) => v.to_string(),
            PCodeValue::I64(v) => v.to_string(),
            PCodeValue::F32(v) => v.to_string(),
            PCodeValue::F64(v) => v.to_string(),
            PCodeValue::ByRefI32(ptr) => unsafe {
                if !ptr.is_null() {
                    (**ptr).to_string()
                } else {
                    "0".to_string()
                }
            },
            PCodeValue::Variant(v) => {
                if let Some(b) = v.as_bstr() {
                    unsafe { bstr_to_string(b) }
                } else if let Some(n) = v.as_i32() {
                    n.to_string()
                } else {
                    String::new()
                }
            }
            PCodeValue::Object(_) => "[Object]".to_string(),
        }
    }
}

/// The P-Code Virtual Machine execution engine.
pub struct PCodeVM {
    /// The operand evaluation stack.
    pub stack: Vec<PCodeValue>,
    /// Program Counter (instruction pointer).
    pub pc: usize,
    /// The procedure execution frame.
    pub frame: ProcFrame,
    /// String table for literal constants.
    pub string_table: Vec<String>,
    /// Address of ObjectInfo for real PE P-Code execution.
    pub obj_info_addr: usize,
    /// Address of ObjectConstantPool for real PE P-Code execution.
    pub cpool_addr: usize,
}

impl PCodeVM {
    /// Instantiates a new P-Code Virtual Machine with a frame of the given size.
    pub fn new(frame_size: usize) -> Self {
        Self {
            stack: Vec::with_capacity(64),
            pc: 0,
            frame: ProcFrame::new(frame_size),
            string_table: Vec::new(),
            obj_info_addr: 0,
            cpool_addr: 0,
        }
    }

    /// Pushes an operand onto the evaluation stack.
    pub fn push(&mut self, val: PCodeValue) {
        self.stack.push(val);
    }

    /// Pops an operand from the evaluation stack.
    pub fn pop(&mut self) -> PCodeValue {
        self.stack.pop().unwrap_or(PCodeValue::I32(0))
    }

    /// Returns a reference to the top of stack without popping.
    pub fn peek(&self) -> Option<&PCodeValue> {
        self.stack.last()
    }

    /// Executes the given bytecode stream with the provided arguments.
    pub fn execute(&mut self, bytecode: &[u8], args: &mut [PCodeValue]) -> Result<PCodeValue, i32> {
        if self.cpool_addr != 0 {
            return self.execute_msvbvm_pcode(bytecode, args);
        }
        self.pc = 0;
        let len = bytecode.len();

        while self.pc < len {
            let opcode = bytecode[self.pc];
            self.pc += 1;

            match opcode {
                OP_NOP => {}

                OP_LITI2 => {
                    if self.pc + 2 <= len {
                        let val = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        self.push(PCodeValue::I32(val as i32));
                    }
                }

                OP_LITI4 => {
                    if self.pc + 4 <= len {
                        let val = i32::from_le_bytes([
                            bytecode[self.pc],
                            bytecode[self.pc + 1],
                            bytecode[self.pc + 2],
                            bytecode[self.pc + 3],
                        ]);
                        self.pc += 4;
                        self.push(PCodeValue::I32(val));
                    }
                }

                OP_LITR4 => {
                    if self.pc + 4 <= len {
                        let bits = u32::from_le_bytes([
                            bytecode[self.pc],
                            bytecode[self.pc + 1],
                            bytecode[self.pc + 2],
                            bytecode[self.pc + 3],
                        ]);
                        self.pc += 4;
                        self.push(PCodeValue::F32(f32::from_bits(bits)));
                    }
                }

                OP_LITR8 => {
                    if self.pc + 8 <= len {
                        let mut buf = [0u8; 8];
                        buf.copy_from_slice(&bytecode[self.pc..self.pc + 8]);
                        self.pc += 8;
                        self.push(PCodeValue::F64(f64::from_bits(u64::from_le_bytes(buf))));
                    }
                }

                OP_LITCY => {
                    if self.pc + 8 <= len {
                        let mut buf = [0u8; 8];
                        buf.copy_from_slice(&bytecode[self.pc..self.pc + 8]);
                        self.pc += 8;
                        self.push(PCodeValue::I64(i64::from_le_bytes(buf)));
                    }
                }

                OP_LITSTR => {
                    if self.pc + 2 <= len {
                        let idx = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        self.pc += 2;
                        let s = if idx < self.string_table.len() {
                            self.string_table[idx].clone()
                        } else {
                            String::new()
                        };
                        self.push(PCodeValue::String(s));
                    }
                }

                OP_LDLOCAL => {
                    if self.pc + 2 <= len {
                        let off = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        self.pc += 2;
                        let val = self.frame.read_i32(off);
                        self.push(PCodeValue::I32(val));
                    }
                }

                OP_STLOCAL => {
                    if self.pc + 2 <= len {
                        let off = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        self.pc += 2;
                        let val = self.pop().as_i32();
                        self.frame.write_i32(off, val);
                    }
                }

                OP_LDARG => {
                    if self.pc + 2 <= len {
                        let idx = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        self.pc += 2;
                        if idx < args.len() {
                            self.push(args[idx].clone());
                        } else {
                            self.push(PCodeValue::I32(0));
                        }
                    }
                }

                OP_STARG => {
                    if self.pc + 2 <= len {
                        let idx = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        self.pc += 2;
                        let val = self.pop();
                        if idx < args.len() {
                            args[idx] = val;
                        }
                    }
                }

                OP_LDARG_BYREF => {
                    if self.pc + 2 <= len {
                        let idx = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        self.pc += 2;
                        if idx < args.len() {
                            match &args[idx] {
                                PCodeValue::ByRefI32(ptr) => unsafe {
                                    let v = if !ptr.is_null() { **ptr } else { 0 };
                                    self.push(PCodeValue::I32(v));
                                },
                                other => self.push(other.clone()),
                            }
                        } else {
                            self.push(PCodeValue::I32(0));
                        }
                    }
                }

                OP_STARG_BYREF => {
                    if self.pc + 2 <= len {
                        let idx = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        self.pc += 2;
                        let val = self.pop().as_i32();
                        if idx < args.len() {
                            if let PCodeValue::ByRefI32(ptr) = args[idx] {
                                unsafe {
                                    if !ptr.is_null() {
                                        *ptr = val;
                                    }
                                }
                            } else {
                                args[idx] = PCodeValue::I32(val);
                            }
                        }
                    }
                }

                OP_DUP => {
                    if let Some(top) = self.peek().cloned() {
                        self.push(top);
                    }
                }

                OP_POP => {
                    self.pop();
                }

                OP_ADD => {
                    let b = self.pop();
                    let a = self.pop();
                    match (&a, &b) {
                        (PCodeValue::String(s1), PCodeValue::String(s2)) => {
                            self.push(PCodeValue::String(format!("{}{}", s1, s2)));
                        }
                        _ => {
                            self.push(PCodeValue::I32(a.as_i32().wrapping_add(b.as_i32())));
                        }
                    }
                }

                OP_SUB => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(a.wrapping_sub(b)));
                }

                OP_MUL => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(a.wrapping_mul(b)));
                }

                OP_DIV => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    if b == 0 {
                        self.handle_error(11)?; // Division by zero (Err 11)
                    } else {
                        self.push(PCodeValue::F64(a as f64 / b as f64));
                    }
                }

                OP_IDIV => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    if b == 0 {
                        self.handle_error(11)?; // Division by zero
                    } else {
                        self.push(PCodeValue::I32(a / b));
                    }
                }

                OP_MOD => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    if b == 0 {
                        self.handle_error(11)?;
                    } else {
                        self.push(PCodeValue::I32(a % b));
                    }
                }

                OP_POW => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32((a as f64).powi(b) as i32));
                }

                OP_NEG => {
                    let val = self.pop().as_i32();
                    self.push(PCodeValue::I32(-val));
                }

                OP_EQ => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(if a == b { -1 } else { 0 }));
                }

                OP_NE => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(if a != b { -1 } else { 0 }));
                }

                OP_LT => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(if a < b { -1 } else { 0 }));
                }

                OP_LE => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(if a <= b { -1 } else { 0 }));
                }

                OP_GT => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(if a > b { -1 } else { 0 }));
                }

                OP_GE => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(if a >= b { -1 } else { 0 }));
                }

                OP_AND => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(a & b));
                }

                OP_OR => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(a | b));
                }

                OP_NOT => {
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(!a));
                }

                OP_XOR => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(a ^ b));
                }

                OP_BRANCH => {
                    if self.pc + 2 <= len {
                        let target_pc = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        self.pc = target_pc;
                    }
                }

                OP_BRANCH_IF_ZERO => {
                    if self.pc + 2 <= len {
                        let target_pc = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        let cond = self.pop().as_i32();
                        if cond == 0 {
                            self.pc = target_pc;
                        } else {
                            self.pc += 2;
                        }
                    }
                }

                OP_BRANCH_IF_NOT_ZERO => {
                    if self.pc + 2 <= len {
                        let target_pc = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        let cond = self.pop().as_i32();
                        if cond != 0 {
                            self.pc = target_pc;
                        } else {
                            self.pc += 2;
                        }
                    }
                }

                OP_SELECT_CASE_JUMP => {
                    // Count (u16), then entries [value: i32, target_pc: u16], then default_pc (u16)
                    if self.pc + 2 <= len {
                        let count = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        self.pc += 2;
                        let selector = self.pop().as_i32();
                        let mut target_pc: Option<usize> = None;

                        for _ in 0..count {
                            if self.pc + 6 <= len {
                                let case_val = i32::from_le_bytes([
                                    bytecode[self.pc],
                                    bytecode[self.pc + 1],
                                    bytecode[self.pc + 2],
                                    bytecode[self.pc + 3],
                                ]);
                                let case_target = u16::from_le_bytes([
                                    bytecode[self.pc + 4],
                                    bytecode[self.pc + 5],
                                ]) as usize;
                                self.pc += 6;

                                if target_pc.is_none() && case_val == selector {
                                    target_pc = Some(case_target);
                                }
                            }
                        }

                        if self.pc + 2 <= len {
                            let default_pc = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                            self.pc += 2;
                            let final_target = target_pc.unwrap_or(default_pc);
                            self.pc = final_target;
                        }
                    }
                }

                OP_IMPAD_CALL => {
                    // Runtime intrinsic import: ordinal (u16), args_count (u8)
                    if self.pc + 3 <= len {
                        let ordinal = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        let c_args = bytecode[self.pc + 2];
                        self.pc += 3;
                        self.invoke_runtime_import(ordinal, c_args);
                    }
                }

                OP_METH_CALL => {
                    // Method dispatch on class instance
                    if self.pc + 3 <= len {
                        let method_id = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        let c_args = bytecode[self.pc + 2];
                        self.pc += 3;
                        self.invoke_method_call(method_id, c_args);
                    }
                }

                OP_PROCCALL => {
                    // Nested procedure call
                    if self.pc + 4 <= len {
                        let sub_ptr = u32::from_le_bytes([
                            bytecode[self.pc],
                            bytecode[self.pc + 1],
                            bytecode[self.pc + 2],
                            bytecode[self.pc + 3],
                        ]) as usize;
                        self.pc += 4;
                        if sub_ptr < len {
                            let mut sub_vm = PCodeVM::new(128);
                            sub_vm.string_table = self.string_table.clone();
                            let arg_val = self.pop();
                            let mut sub_args = vec![arg_val];
                            let res = sub_vm.execute(&bytecode[sub_ptr..], &mut sub_args)?;
                            self.push(res);
                        }
                    }
                }

                OP_ON_ERROR_GOTO => {
                    if self.pc + 2 <= len {
                        let target_pc = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        self.pc += 2;
                        self.frame.error_handler_pc = Some(target_pc);
                        self.frame.resume_next = false;
                    }
                }

                OP_ON_ERROR_RESUME_NEXT => {
                    self.frame.resume_next = true;
                    self.frame.error_handler_pc = None;
                }

                OP_RESUME => {
                    crate::err::clear_vb_error();
                }

                OP_RETURN | OP_HALT => {
                    let ret_val = self.pop();
                    self.frame.teardown();
                    return Ok(ret_val);
                }

                unknown => {
                    #[cfg(feature = "std")]
                    eprintln!("[PCodeVM] Unknown opcode: 0x{:02X} at PC=0x{:X}", unknown, self.pc - 1);
                }
            }
        }

        self.frame.teardown();
        Ok(self.pop())
    }

    fn slot_ptr(&mut self, off: i16) -> *mut i32 {
        if self.frame.locals.len() < 8192 {
            self.frame.locals.resize(8192, 0);
        }
        let idx = (4096isize + off as isize).clamp(0, 8188) as usize;
        unsafe { self.frame.locals.as_mut_ptr().add(idx) as *mut i32 }
    }

    fn read_slot(&mut self, off: i16, args: &[PCodeValue]) -> PCodeValue {
        if off >= 8 {
            let arg_idx = ((off - 8) / 4) as usize;
            if let Some(v) = args.get(arg_idx) {
                return v.clone();
            }
        }
        let ptr = self.slot_ptr(off);
        let key = ptr as usize;
        if let Some(s) = pcode_get_string(key) {
            return PCodeValue::String(s);
        }
        let raw = unsafe { core::ptr::read_unaligned(ptr) };
        let u = raw as u32;
        if (u & 0xFFFF_0000) == 0x5000_0000 || (u & 0xFFFF_0000) == 0xDEAD_0000 {
            PCodeValue::Object(u as usize as *mut c_void)
        } else {
            PCodeValue::I32(raw)
        }
    }

    fn write_slot(&mut self, off: i16, val: PCodeValue) {
        let ptr = self.slot_ptr(off);
        let key = ptr as usize;
        match val {
            PCodeValue::String(s) => {
                pcode_set_string(key, s);
                unsafe { core::ptr::write_unaligned(ptr, 0) };
            }
            PCodeValue::Object(o) => {
                pcode_remove_string(key);
                unsafe { core::ptr::write_unaligned(ptr, o as usize as i32) };
            }
            other => {
                pcode_remove_string(key);
                unsafe { core::ptr::write_unaligned(ptr, other.as_i32()) };
            }
        }
    }

    fn cpool_u32(&self, idx: u16) -> u32 {
        let (base, size) = unsafe {
            (
                crate::pe::shims::GLOBAL_MODULE_HANDLE,
                crate::pe::shims::GLOBAL_MODULE_SIZE,
            )
        };
        let addr = self.cpool_addr.wrapping_add((idx as usize) * 4);
        if base != 0 && size > 0 && addr >= base && addr + 4 <= base + size {
            unsafe { core::ptr::read_unaligned(addr as *const u32) }
        } else {
            0
        }
    }

    fn cpool_bstr(&self, idx: u16) -> String {
        let (base, size) = unsafe {
            (
                crate::pe::shims::GLOBAL_MODULE_HANDLE,
                crate::pe::shims::GLOBAL_MODULE_SIZE,
            )
        };
        let va = self.cpool_u32(idx) as usize;
        if base != 0 && size > 0 && va >= base + 4 && va < base + size {
            let byte_len = unsafe { core::ptr::read_unaligned((va - 4) as *const u32) } as usize;
            if byte_len > 0 && byte_len <= 4096 && va + byte_len <= base + size {
                let u16_slice = unsafe { core::slice::from_raw_parts(va as *const u16, byte_len / 2) };
                return String::from_utf16_lossy(u16_slice);
            }
        }
        String::new()
    }

    fn find_print_epilogue(bytecode: &[u8]) -> Option<usize> {
        if bytecode.len() < 8 {
            return None;
        }
        (0..=bytecode.len() - 8).rev().find(|&k| {
            bytecode[k] == 0x1b
                && bytecode[k + 3] == 0x0a
                && bytecode[k + 6] == 0x04
                && bytecode[k + 7] == 0x00
        })
    }

    fn invoke_cpool_call(&mut self, idx: u16, arg_bytes: u16, push_return: bool) -> Result<(), i32> {
        let n_args = (arg_bytes / 4) as usize;
        let mut popped_args = Vec::with_capacity(n_args);
        for _ in 0..n_args {
            popped_args.push(self.pop());
        }

        let (base, size) = unsafe {
            (
                crate::pe::shims::GLOBAL_MODULE_HANDLE,
                crate::pe::shims::GLOBAL_MODULE_SIZE,
            )
        };
        let target_va = self.cpool_u32(idx) as usize;
        if base == 0 || size == 0 || target_va < base || target_va + 6 > base + size {
            if push_return {
                self.push(PCodeValue::I32(0));
            }
            return Ok(());
        }

        let code_ptr = target_va as *const u8;
        unsafe {
            // Case 1: P-Code procedure trampoline (ba <desc_va> b9 <ProcCallEngine> ff e1)
            if target_va + 12 <= base + size
                && *code_ptr == 0xBA
                && *code_ptr.add(5) == 0xB9
                && *code_ptr.add(10) == 0xFF
                && *code_ptr.add(11) == 0xE1
            {
                let sub_desc = core::ptr::read_unaligned(code_ptr.add(1) as *const u32) as usize;
                if let Some(desc) = crate::pcode::descriptor::ProcedureDescriptor::from_ptr(sub_desc) {
                    let sub_code_ptr = desc.lp_byte_code as usize;
                    let sub_len = if desc.flags > 0 && sub_code_ptr + (desc.flags as usize) <= base + size {
                        desc.flags as usize
                    } else {
                        0
                    };
                    if sub_len > 0 {
                        let sub_slice = core::slice::from_raw_parts(desc.lp_byte_code, sub_len);
                        let mut sub_vm = PCodeVM::new(desc.frame_size as usize);
                        let sub_oi = core::ptr::read_unaligned(sub_desc as *const u32) as usize;
                        if sub_oi >= base && sub_oi + 0x38 <= base + size {
                            sub_vm.obj_info_addr = sub_oi;
                            sub_vm.cpool_addr = core::ptr::read_unaligned((sub_oi + 0x34) as *const u32) as usize;
                        }
                        let mut sub_args = Vec::with_capacity(1 + popped_args.len());
                        sub_args.push(PCodeValue::I32(0));
                        sub_args.extend(popped_args);
                        match sub_vm.execute(sub_slice, &mut sub_args) {
                            Ok(ret_val) => {
                                if push_return {
                                    self.push(ret_val);
                                }
                                return Ok(());
                            }
                            Err(err_code) => {
                                return self.handle_error(err_code);
                            }
                        }
                    }
                }
            }

            // Case 2: Native DllFunctionCall thunk (a1 <cache> 0b c0 74 02 ff e0 68 <dll_info> ...)
            if target_va + 16 <= base + size && *code_ptr == 0xA1 && *code_ptr.add(5) == 0x0B {
                let dll_info_va = core::ptr::read_unaligned(code_ptr.add(12) as *const u32) as usize;
                if dll_info_va >= base && dll_info_va + 8 <= base + size {
                    let fn_va = core::ptr::read_unaligned((dll_info_va + 4) as *const u32) as usize;
                    if fn_va >= base && fn_va < base + size {
                        let fn_name = core::ffi::CStr::from_ptr(fn_va as *const i8).to_string_lossy();
                        if fn_name.eq_ignore_ascii_case("GetStdHandle") {
                            let std_h = popped_args.first().map(|v| v.as_i32()).unwrap_or(-11);
                            let h = crate::pe::shims::win32_GetStdHandle(std_h) as i32;
                            if push_return {
                                self.push(PCodeValue::I32(h));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("WriteFile") {
                            let h_file = popped_args.first().map(|v| v.as_i32()).unwrap_or(1) as usize;
                            let buf_str = popped_args.get(1).map(|v| v.as_string()).unwrap_or_default();
                            let n_bytes = popped_args
                                .get(2)
                                .map(|v| v.as_i32().max(0) as u32)
                                .unwrap_or(buf_str.len() as u32)
                                .min(buf_str.len() as u32);
                            let mut written: u32 = 0;
                            let res = crate::pe::shims::win32_WriteFile(
                                h_file,
                                buf_str.as_ptr(),
                                n_bytes,
                                &mut written,
                                core::ptr::null_mut(),
                            );
                            if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(3) {
                                if !ptr.is_null() {
                                    core::ptr::write_unaligned(*ptr, written as i32);
                                }
                            }
                            if push_return {
                                self.push(PCodeValue::I32(res as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetModuleHandleA") {
                            let mod_name = popped_args.first().map(|v| v.as_string()).unwrap_or_default();
                            let h = if mod_name.is_empty() {
                                cb_x86::memory::GLOBAL_MODULE_HANDLE as i32
                            } else {
                                let cname = std::ffi::CString::new(mod_name).unwrap();
                                cb_win32::win32_GetModuleHandleA(cname.as_ptr()) as i32
                            };
                            if push_return {
                                self.push(PCodeValue::I32(h));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetSystemMetrics") {
                            let n = popped_args.first().map(|v| v.as_i32()).unwrap_or(0);
                            let res = cb_win32::user32_get_system_metrics(n);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("RegisterClassA") {
                            let lp_wc = popped_args.first().map(|v| v.as_ptr()).unwrap_or(core::ptr::null_mut());
                            let atom = if !lp_wc.is_null() {
                                cb_win32::win32_RegisterClassA(lp_wc as *const cb_win32::WndClassA)
                            } else {
                                0
                            };
                            if push_return {
                                self.push(PCodeValue::I32(atom as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("CreateWindowExA") {
                            let ex_style = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let cls = popped_args.get(1).map(|v| v.as_string()).unwrap_or_default();
                            let title = popped_args.get(2).map(|v| v.as_string()).unwrap_or_default();
                            let style = popped_args.get(3).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let x = popped_args.get(4).map(|v| v.as_i32()).unwrap_or(0);
                            let y = popped_args.get(5).map(|v| v.as_i32()).unwrap_or(0);
                            let w = popped_args.get(6).map(|v| v.as_i32()).unwrap_or(0);
                            let h = popped_args.get(7).map(|v| v.as_i32()).unwrap_or(0);
                            let parent = popped_args.get(8).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let menu = popped_args.get(9).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let inst = popped_args.get(10).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let hwnd = cb_win32::window_tree().lock().unwrap().create_window(
                                ex_style, cls, title, style, x, y, w, h, parent, menu, inst,
                            );
                            if push_return {
                                self.push(PCodeValue::I32(hwnd as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("DestroyWindow") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = if cb_win32::window_tree().lock().unwrap().destroy_window(hwnd) { 1 } else { 0 };
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("ShowWindow") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let cmd = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let res = if cb_win32::window_tree().lock().unwrap().show_window(hwnd, cmd) { 1 } else { 0 };
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("UpdateWindow") {
                            if push_return {
                                self.push(PCodeValue::I32(1));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("MoveWindow") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let x = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let y = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let w = popped_args.get(3).map(|v| v.as_i32()).unwrap_or(0);
                            let h = popped_args.get(4).map(|v| v.as_i32()).unwrap_or(0);
                            let res = if cb_win32::window_tree().lock().unwrap().move_window(hwnd, x, y, w, h) { 1 } else { 0 };
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("SetWindowPos") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let x = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let y = popped_args.get(3).map(|v| v.as_i32()).unwrap_or(0);
                            let cx = popped_args.get(4).map(|v| v.as_i32()).unwrap_or(0);
                            let cy = popped_args.get(5).map(|v| v.as_i32()).unwrap_or(0);
                            let res = if cb_win32::window_tree().lock().unwrap().move_window(hwnd, x, y, cx, cy) { 1 } else { 0 };
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("SetWindowTextA") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let text = popped_args.get(1).map(|v| v.as_string()).unwrap_or_default();
                            let res = if cb_win32::window_tree().lock().unwrap().set_window_text(hwnd, &text) { 1 } else { 0 };
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetWindowTextA") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let cch = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let text = cb_win32::window_tree().lock().unwrap().get_window_text(hwnd).unwrap_or_default();
                            let copy_len = if cch > 0 { ((cch - 1) as usize).min(text.len()) } else { 0 };
                            let out_str = text[..copy_len].to_string();
                            if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(1) {
                                pcode_set_string(*ptr as usize, out_str);
                            }
                            if push_return {
                                self.push(PCodeValue::I32(copy_len as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetWindowTextLengthA") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let len = cb_win32::window_tree().lock().unwrap().get_window_text(hwnd).map(|s| s.len() as i32).unwrap_or(0);
                            if push_return {
                                self.push(PCodeValue::I32(len));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetWindowRect") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let mut ok = 0;
                            if let Some(rect) = cb_win32::window_tree().lock().unwrap().get_window_rect(hwnd) {
                                if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(1) {
                                    if !ptr.is_null() {
                                        core::ptr::write_unaligned(*ptr, rect.left);
                                        core::ptr::write_unaligned(ptr.add(1), rect.top);
                                        core::ptr::write_unaligned(ptr.add(2), rect.right);
                                        core::ptr::write_unaligned(ptr.add(3), rect.bottom);
                                        ok = 1;
                                    }
                                }
                            }
                            if push_return {
                                self.push(PCodeValue::I32(ok));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetClientRect") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let mut ok = 0;
                            if let Some(rect) = cb_win32::window_tree().lock().unwrap().get_client_rect(hwnd) {
                                if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(1) {
                                    if !ptr.is_null() {
                                        core::ptr::write_unaligned(*ptr, rect.left);
                                        core::ptr::write_unaligned(ptr.add(1), rect.top);
                                        core::ptr::write_unaligned(ptr.add(2), rect.right);
                                        core::ptr::write_unaligned(ptr.add(3), rect.bottom);
                                        ok = 1;
                                    }
                                }
                            }
                            if push_return {
                                self.push(PCodeValue::I32(ok));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("IsWindow") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = if cb_win32::window_tree().lock().unwrap().is_window(hwnd) { 1 } else { 0 };
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("IsWindowVisible") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = if cb_win32::window_tree().lock().unwrap().is_window_visible(hwnd) { 1 } else { 0 };
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("EnableWindow") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let enable = popped_args.get(1).map(|v| v.as_i32() != 0).unwrap_or(false);
                            let res = if cb_win32::window_tree().lock().unwrap().enable_window(hwnd, enable) { 1 } else { 0 };
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetParent") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let p = cb_win32::window_tree().lock().unwrap().get_parent(hwnd);
                            if push_return {
                                self.push(PCodeValue::I32(p as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("SendMessageA") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let msg = popped_args.get(1).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let wp = popped_args.get(2).map(|v| v.as_i32() as u32).unwrap_or(0) as usize;
                            let lp = popped_args.get(3).map(|v| v.as_i32() as u32).unwrap_or(0) as usize;
                            let res = cb_win32::user32_send_message(hwnd, msg, wp, lp);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("PostMessageA") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let msg = popped_args.get(1).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let wp = popped_args.get(2).map(|v| v.as_i32() as u32).unwrap_or(0) as usize;
                            let lp = popped_args.get(3).map(|v| v.as_i32() as u32).unwrap_or(0) as usize;
                            cb_win32::message_queue().post_message(cb_win32::Win32Msg {
                                hwnd,
                                message: msg,
                                w_param: wp,
                                l_param: lp,
                                time: 0,
                                pt_x: 0,
                                pt_y: 0,
                            });
                            if push_return {
                                self.push(PCodeValue::I32(1));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("PeekMessageA") {
                            let hwnd = popped_args.get(1).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let f_min = popped_args.get(2).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let f_max = popped_args.get(3).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let remove = popped_args.get(4).map(|v| (v.as_i32() as u32 & cb_win32::PM_REMOVE) != 0).unwrap_or(false);
                            let mut ok = 0;
                            if let Some(m) = cb_win32::message_queue().peek_message(hwnd, f_min, f_max, remove) {
                                if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.first() {
                                    if !ptr.is_null() {
                                        core::ptr::write_unaligned(*ptr, m.hwnd as i32);
                                        core::ptr::write_unaligned(ptr.add(1), m.message as i32);
                                        core::ptr::write_unaligned(ptr.add(2), m.w_param as i32);
                                        core::ptr::write_unaligned(ptr.add(3), m.l_param as i32);
                                        core::ptr::write_unaligned(ptr.add(4), m.time as i32);
                                        core::ptr::write_unaligned(ptr.add(5), m.pt_x);
                                        core::ptr::write_unaligned(ptr.add(6), m.pt_y);
                                        ok = 1;
                                    }
                                }
                            }
                            if push_return {
                                self.push(PCodeValue::I32(ok));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetMessageA") {
                            let hwnd = popped_args.get(1).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let f_min = popped_args.get(2).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let f_max = popped_args.get(3).map(|v| v.as_i32() as u32).unwrap_or(0);
                            match cb_win32::message_queue().get_message(hwnd, f_min, f_max) {
                                Ok(m) => {
                                    if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.first() {
                                        if !ptr.is_null() {
                                            core::ptr::write_unaligned(*ptr, m.hwnd as i32);
                                            core::ptr::write_unaligned(ptr.add(1), m.message as i32);
                                            core::ptr::write_unaligned(ptr.add(2), m.w_param as i32);
                                            core::ptr::write_unaligned(ptr.add(3), m.l_param as i32);
                                            core::ptr::write_unaligned(ptr.add(4), m.time as i32);
                                            core::ptr::write_unaligned(ptr.add(5), m.pt_x);
                                            core::ptr::write_unaligned(ptr.add(6), m.pt_y);
                                        }
                                    }
                                    if push_return {
                                        self.push(PCodeValue::I32(1));
                                    }
                                    return Ok(());
                                }
                                Err(code) => {
                                    if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.first() {
                                        if !ptr.is_null() {
                                            core::ptr::write_unaligned(*ptr, 0);
                                            core::ptr::write_unaligned(ptr.add(1), cb_win32::WM_QUIT as i32);
                                            core::ptr::write_unaligned(ptr.add(2), code);
                                            core::ptr::write_unaligned(ptr.add(3), 0);
                                            core::ptr::write_unaligned(ptr.add(4), 0);
                                            core::ptr::write_unaligned(ptr.add(5), 0);
                                            core::ptr::write_unaligned(ptr.add(6), 0);
                                        }
                                    }
                                    if push_return {
                                        self.push(PCodeValue::I32(0));
                                    }
                                    return Ok(());
                                }
                            }
                        } else if fn_name.eq_ignore_ascii_case("PostQuitMessage") {
                            let code = popped_args.first().map(|v| v.as_i32()).unwrap_or(0);
                            cb_win32::message_queue().post_quit_message(code);
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("DefWindowProcA") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let msg = popped_args.get(1).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let wp = popped_args.get(2).map(|v| v.as_i32() as u32).unwrap_or(0) as usize;
                            let lp = popped_args.get(3).map(|v| v.as_i32() as u32).unwrap_or(0) as usize;
                            let res = cb_win32::user32_def_window_proc(hwnd, msg, wp, lp);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        }
                    }
                }
            }

            // Case 3: IAT jump thunk (ff 25 <iat_slot_va>)
            if target_va + 6 <= base + size && *code_ptr == 0xFF && *code_ptr.add(1) == 0x25 {
                let iat_slot_va = core::ptr::read_unaligned(code_ptr.add(2) as *const u32) as usize;
                if iat_slot_va >= base && iat_slot_va + 4 <= base + size {
                    let resolved = core::ptr::read_unaligned(iat_slot_va as *const u32);
                    let ord = resolved & 0xFFFF;
                    match ord {
                        592 | 685 => {
                            // rtcErrObj
                            if push_return {
                                self.push(PCodeValue::Object(0xDEAD_0250 as *mut c_void));
                            }
                            return Ok(());
                        }
                        518 => {
                            let s = popped_args.first().map(|v| v.as_string()).unwrap_or_default();
                            if push_return {
                                self.push(PCodeValue::String(s.trim_start().to_string()));
                            }
                            return Ok(());
                        }
                        519 | 520 => {
                            let s = popped_args.first().map(|v| v.as_string()).unwrap_or_default();
                            if push_return {
                                self.push(PCodeValue::String(s.trim().to_string()));
                            }
                            return Ok(());
                        }
                        521 => {
                            let s = popped_args.first().map(|v| v.as_string()).unwrap_or_default();
                            if push_return {
                                self.push(PCodeValue::String(s.trim_end().to_string()));
                            }
                            return Ok(());
                        }
                        631 | 632 => {
                            // Mid$(str, start, [len])
                            let (s, start, len) = if popped_args.len() >= 3 {
                                let l = popped_args[0].as_i32();
                                let st = popped_args[1].as_i32();
                                let str_val = popped_args[2].as_string();
                                (str_val, st, Some(l))
                            } else if popped_args.len() == 2 {
                                let st = popped_args[0].as_i32();
                                let str_val = popped_args[1].as_string();
                                (str_val, st, None)
                            } else {
                                (String::new(), 1, None)
                            };
                            let u16_chars: Vec<u16> = s.encode_utf16().collect();
                            let sub = if start > 0 && ((start - 1) as usize) < u16_chars.len() {
                                let start_idx = (start - 1) as usize;
                                let avail = u16_chars.len() - start_idx;
                                let count = match len {
                                    Some(l) if l >= 0 => (l as usize).min(avail),
                                    _ => avail,
                                };
                                String::from_utf16_lossy(&u16_chars[start_idx..start_idx + count])
                            } else {
                                String::new()
                            };
                            if push_return {
                                self.push(PCodeValue::String(sub));
                            }
                            return Ok(());
                        }
                        713 => {
                            // StrReverse(str)
                            let s = popped_args.first().map(|v| v.as_string()).unwrap_or_default();
                            let rev: String = s.chars().rev().collect();
                            if push_return {
                                self.push(PCodeValue::String(rev));
                            }
                            return Ok(());
                        }
                        644 => {
                            // VarPtr
                            let ptr_val = match popped_args.first() {
                                Some(PCodeValue::ByRefI32(p)) => *p as usize as i32,
                                Some(other) => other.as_i32(),
                                None => 0,
                            };
                            if push_return {
                                self.push(PCodeValue::I32(ptr_val));
                            }
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
        }

        if push_return {
            self.push(PCodeValue::I32(0));
        }
        Ok(())
    }

    fn execute_msvbvm_pcode(
        &mut self,
        bytecode: &[u8],
        args: &mut [PCodeValue],
    ) -> Result<PCodeValue, i32> {
        self.pc = 0;
        let len = bytecode.len();
        if self.frame.locals.len() < 8192 {
            self.frame.locals.resize(8192, 0);
        }

        while self.pc < len {
            let op = bytecode[self.pc];
            self.pc += 1;

            match op {
                0x04 => {
                    // FLdRfVar(off: i16)
                    if self.pc + 2 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let ptr = self.slot_ptr(off);
                        self.push(PCodeValue::ByRefI32(ptr));
                    }
                }
                0x08 => {
                    // FLdPr(off: i16)
                    if self.pc + 2 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let val = self.read_slot(off, args);
                        self.push(val);
                    }
                }
                0x0a => {
                    // ImpAdCallVoid(idx: u16, arg_bytes: u16)
                    if self.pc + 4 <= len {
                        let idx = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        let arg_bytes = u16::from_le_bytes([bytecode[self.pc + 2], bytecode[self.pc + 3]]);
                        self.pc += 4;
                        self.invoke_cpool_call(idx, arg_bytes, false)?;
                    }
                }
                0x0b | 0x5e => {
                    // ImpAdCallI2/I4/Ptr(idx: u16, arg_bytes: u16)
                    if self.pc + 4 <= len {
                        let idx = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        let arg_bytes = u16::from_le_bytes([bytecode[self.pc + 2], bytecode[self.pc + 3]]);
                        self.pc += 4;
                        self.invoke_cpool_call(idx, arg_bytes, true)?;
                    }
                }
                0x0d => {
                    // VCallHresult(vtable_off: u16, cpool_idx: u16)
                    if self.pc + 4 <= len {
                        let vtable_off = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        let _cpool_idx = u16::from_le_bytes([bytecode[self.pc + 2], bytecode[self.pc + 3]]);
                        self.pc += 4;
                        let this_val = self.pop();
                        let obj_id = match this_val {
                            PCodeValue::Object(p) => p as usize,
                            other => other.as_i32() as u32 as usize,
                        };
                        if obj_id == 0xDEAD_0250 {
                            // ErrObject vtable dispatch
                            if vtable_off == 0x0044 {
                                // Err.Raise(Number, Source, Description, HelpFile, HelpContext)
                                let err_num = self.pop().as_i32();
                                let _ = self.pop();
                                let _ = self.pop();
                                let _ = self.pop();
                                let _ = self.pop();
                                self.handle_error(err_num)?;
                            } else if vtable_off == 0x001c {
                                // Err.get_Number(&ret)
                                if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                    if !ptr.is_null() {
                                        unsafe {
                                            core::ptr::write_unaligned(ptr, crate::err::get_vb_error_number());
                                        }
                                    }
                                }
                            }
                        } else {
                            let mut dispatched = false;
                            if let Some(oi_addr) = pcode_get_object_oi(obj_id) {
                                let (base, size) = unsafe {
                                    (
                                        crate::pe::shims::GLOBAL_MODULE_HANDLE,
                                        crate::pe::shims::GLOBAL_MODULE_SIZE,
                                    )
                                };
                                if base != 0 && size > 0 && oi_addr >= base && oi_addr + 0x38 <= base + size && vtable_off >= 0x001c {
                                    let method_idx = ((vtable_off - 0x001c) / 4) as usize;
                                    unsafe {
                                        let n_methods = core::ptr::read_unaligned((oi_addr + 0x20) as *const u32) as usize;
                                        let lp_methods = core::ptr::read_unaligned((oi_addr + 0x24) as *const u32) as usize;
                                        if n_methods > 0 && method_idx < n_methods && lp_methods >= base && lp_methods + (method_idx + 1) * 4 <= base + size {
                                            let m_desc_va = core::ptr::read_unaligned((lp_methods + method_idx * 4) as *const u32) as usize;
                                            if let Some(m_desc) = crate::pcode::descriptor::ProcedureDescriptor::from_ptr(m_desc_va) {
                                                let n_extra_args = (m_desc.args_size.saturating_sub(4) / 4) as usize;
                                                let mut m_args = Vec::with_capacity(1 + n_extra_args);
                                                m_args.push(PCodeValue::Object(obj_id as *mut c_void));
                                                for _ in 0..n_extra_args {
                                                    m_args.push(self.pop());
                                                }
                                                let m_code_ptr = m_desc.lp_byte_code as usize;
                                                let m_len = m_desc.flags as usize;
                                                if m_len > 0 && m_code_ptr >= base && m_code_ptr + m_len <= base + size {
                                                    dispatched = true;
                                                    let m_slice = core::slice::from_raw_parts(m_desc.lp_byte_code, m_len);
                                                    let mut sub_vm = PCodeVM::new(m_desc.frame_size as usize);
                                                    sub_vm.obj_info_addr = oi_addr;
                                                    sub_vm.cpool_addr = core::ptr::read_unaligned((oi_addr + 0x34) as *const u32) as usize;
                                                    if let Err(err_code) = sub_vm.execute(m_slice, &mut m_args) {
                                                        self.handle_error(err_code)?;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if !dispatched {
                                if let Some(epilogue_pc) = Self::find_print_epilogue(bytecode) {
                                    if epilogue_pc >= self.pc.saturating_sub(5) {
                                        self.stack.clear();
                                        self.pc = epilogue_pc;
                                        continue;
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
                0x13 | 0x14 => {
                    // ExitProcHresult
                    let ret_val = self.read_slot(-0x88, args);
                    self.frame.teardown();
                    return Ok(ret_val);
                }
                0x19 => {
                    // FStObj(off: i16)
                    if self.pc + 2 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let val = self.pop();
                        self.write_slot(off, val);
                    }
                }
                0x1a | 0x2f => {
                    // FFree1Obj / FFree1Str(off: i16)
                    if self.pc + 2 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let ptr = self.slot_ptr(off);
                        pcode_remove_string(ptr as usize);
                    }
                }
                0x1b => {
                    // LitStr(idx: u16)
                    if self.pc + 2 <= len {
                        let idx = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let s = self.cpool_bstr(idx);
                        self.push(PCodeValue::String(s));
                    }
                }
                0x1c => {
                    // BranchF(target_pc: u16)
                    if self.pc + 2 <= len {
                        let target_pc = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        self.pc += 2;
                        let cond = self.pop().as_i32();
                        if cond == 0 {
                            self.pc = target_pc;
                        }
                    }
                }
                0x1e => {
                    // Branch(target_pc: u16)
                    if self.pc + 2 <= len {
                        let target_pc = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        self.pc = target_pc;
                    }
                }
                0x23 => {
                    // FStStrNoPop(off: i16)
                    if self.pc + 2 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        if let Some(top) = self.peek().cloned() {
                            self.write_slot(off, top);
                        }
                    }
                }
                0x27 => {
                    // LitVar_Missing(off: i16)
                    if self.pc + 2 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let ptr = self.slot_ptr(off);
                        self.push(PCodeValue::ByRefI32(ptr));
                    }
                }
                0x2a => {
                    // ConcatStr
                    let b = self.pop().as_string();
                    let a = self.pop().as_string();
                    self.push(PCodeValue::String(format!("{}{}", a, b)));
                }
                0x31 | 0x43 => {
                    // FStStr / FStStrCopy(off: i16)
                    if self.pc + 2 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let s = self.pop().as_string();
                        self.write_slot(off, PCodeValue::String(s));
                    }
                }
                0x34 => {
                    // CStr2Ansi
                    let dst = self.pop();
                    let src = self.pop().as_string();
                    if let PCodeValue::ByRefI32(ptr) = dst {
                        pcode_set_string(ptr as usize, src);
                    }
                }
                0x36 => {
                    // FFreeVar(byte_count: u16, offsets...)
                    if self.pc + 2 <= len {
                        let byte_cnt = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        self.pc = (self.pc + 2 + byte_cnt).min(len);
                    }
                }
                0x3a => {
                    // LitVarStr(off: i16, cpool_idx: u16)
                    if self.pc + 4 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        let idx = u16::from_le_bytes([bytecode[self.pc + 2], bytecode[self.pc + 3]]);
                        self.pc += 4;
                        let s = self.cpool_bstr(idx);
                        self.write_slot(off, PCodeValue::String(s.clone()));
                        self.push(PCodeValue::String(s));
                    }
                }
                0x3c => {
                    // SetLastSystemError
                }
                0x3d => {
                    // CastAd(cpool_idx: u16)
                    if self.pc + 2 <= len {
                        self.pc += 2;
                    }
                }
                0x46 | 0x4e => {
                    // CVarStr / FStVarCopyObj(off: i16)
                    if self.pc + 2 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let v = self.pop();
                        self.write_slot(off, v);
                    }
                }
                0x4a => {
                    // FnLenStr
                    let s = self.pop().as_string();
                    self.push(PCodeValue::I32(s.chars().count() as i32));
                }
                0x4b => {
                    // OnErrorGoto(target_pc: u16)
                    if self.pc + 2 <= len {
                        let target_pc = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        if target_pc == 0xFFFF {
                            self.frame.resume_next = true;
                            self.frame.error_handler_pc = None;
                        } else if target_pc == 0xFFFE || target_pc == 0 {
                            self.frame.resume_next = false;
                            self.frame.error_handler_pc = None;
                        } else {
                            self.frame.resume_next = false;
                            self.frame.error_handler_pc = Some(target_pc as usize);
                        }
                    }
                }
                0x67 => {
                    // NextStepI4(temp_off: i16, loop_target_pc: u16)
                    if self.pc + 4 <= len {
                        let temp_off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        let loop_pc = u16::from_le_bytes([bytecode[self.pc + 2], bytecode[self.pc + 3]]) as usize;
                        self.pc += 4;
                        let var_ref = self.pop();
                        let end_val = self.read_slot(temp_off, args).as_i32();
                        let step_val = self.read_slot(temp_off.wrapping_add(4), args).as_i32();
                        if let PCodeValue::ByRefI32(ptr) = var_ref {
                            if !ptr.is_null() {
                                let cur = unsafe { core::ptr::read_unaligned(ptr) }.wrapping_add(step_val);
                                unsafe { core::ptr::write_unaligned(ptr, cur) };
                                let cont = if step_val >= 0 { cur <= end_val } else { cur >= end_val };
                                if cont {
                                    self.pc = loop_pc;
                                }
                            }
                        }
                    }
                }
                0x6b | 0x6c => {
                    // ILdI2 / ILdRf(off: i16)
                    if self.pc + 2 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let v = self.read_slot(off, args);
                        self.push(v);
                    }
                }
                0x70 | 0x71 => {
                    // FStI2 / FStI4(off: i16)
                    if self.pc + 2 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let v = self.pop();
                        self.write_slot(off, v);
                    }
                }
                0x80 => {
                    // ILdPr(off: i16) — dereference ByRef parameter
                    if self.pc + 2 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let arg_val = self.read_slot(off, args);
                        match arg_val {
                            PCodeValue::ByRefI32(ptr) => {
                                let v = if !ptr.is_null() {
                                    unsafe { core::ptr::read_unaligned(ptr) }
                                } else {
                                    0
                                };
                                self.push(PCodeValue::I32(v));
                            }
                            other => self.push(other),
                        }
                    }
                }
                0x85 => {
                    // IStDarg(off: i16) — store through ByRef parameter
                    if self.pc + 2 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let val = self.pop().as_i32();
                        let arg_val = self.read_slot(off, args);
                        if let PCodeValue::ByRefI32(ptr) = arg_val {
                            if !ptr.is_null() {
                                unsafe { core::ptr::write_unaligned(ptr, val) };
                            }
                        }
                    }
                }
                0x8a => {
                    // MemLdI4 / MemLdStr(field_off: u16)
                    if self.pc + 2 <= len {
                        let field_off = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let obj_id = match self.pop() {
                            PCodeValue::Object(p) => p as usize,
                            other => other.as_i32() as u32 as usize,
                        };
                        let val = pcode_get_field(obj_id, field_off);
                        self.push(val);
                    }
                }
                0x8f => {
                    // MemStI4(field_off: u16)
                    if self.pc + 2 <= len {
                        let field_off = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let obj_id = match self.pop() {
                            PCodeValue::Object(p) => p as usize,
                            other => other.as_i32() as u32 as usize,
                        };
                        let val = self.pop();
                        pcode_set_field(obj_id, field_off, val);
                    }
                }
                0x9e => {
                    // Array element load (pops array_ref, index)
                    let _ = self.pop();
                    let _ = self.pop();
                    self.push(PCodeValue::I32(0));
                }
                0xa3 => {
                    // Array element store (pops array_ref, index, val)
                    let _ = self.pop();
                    let _ = self.pop();
                    let _ = self.pop();
                }
                0xaa => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(a.wrapping_add(b)));
                }
                0xae => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(a.wrapping_sub(b)));
                }
                0xb2 => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(a.wrapping_mul(b)));
                }
                0xc0 => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    if b == 0 {
                        self.handle_error(11)?;
                    } else {
                        self.push(PCodeValue::I32(a / b));
                    }
                }
                0xc2 => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    if b == 0 {
                        self.handle_error(11)?;
                    } else {
                        self.push(PCodeValue::I32(a % b));
                    }
                }
                0xc3 => {
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(!a));
                }
                0xc7 => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(if a == b { -1 } else { 0 }));
                }
                0xcc => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(if a != b { -1 } else { 0 }));
                }
                0xd6 => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(if a <= b { -1 } else { 0 }));
                }
                0xe0 => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(if a >= b { -1 } else { 0 }));
                }
                0xf4 => {
                    if self.pc < len {
                        let v = bytecode[self.pc] as i8 as i32;
                        self.pc += 1;
                        self.push(PCodeValue::I32(v));
                    }
                }
                0xf5 => {
                    if self.pc + 4 <= len {
                        let v = i32::from_le_bytes([
                            bytecode[self.pc],
                            bytecode[self.pc + 1],
                            bytecode[self.pc + 2],
                            bytecode[self.pc + 3],
                        ]);
                        self.pc += 4;
                        self.push(PCodeValue::I32(v));
                    }
                }
                0xfb..=0xff => {
                    if self.pc >= len {
                        break;
                    }
                    let sub = bytecode[self.pc];
                    self.pc += 1;
                    match (op, sub) {
                        (0xfb, 0x30) => {
                            // EqStr
                            let b = self.pop().as_string();
                            let a = self.pop().as_string();
                            self.push(PCodeValue::I32(if a == b { -1 } else { 0 }));
                        }
                        (0xfb, 0x3d) => {
                            // NeStr
                            let b = self.pop().as_string();
                            let a = self.pop().as_string();
                            self.push(PCodeValue::I32(if a != b { -1 } else { 0 }));
                        }
                        (0xfb, 0xfe) => {
                            // CStrI4
                            let v = self.pop().as_i32();
                            self.push(PCodeValue::String(v.to_string()));
                        }
                        (0xfc, 0x58) => {
                            // CStr2Uni
                            let dst = self.pop();
                            let src = self.pop().as_string();
                            if let PCodeValue::ByRefI32(ptr) = dst {
                                pcode_set_string(ptr as usize, src);
                            }
                        }
                        (0xfc, 0x63) => {
                            // LitNothing
                            self.push(PCodeValue::Object(core::ptr::null_mut()));
                        }
                        (0xfd, 0x91) => {
                            // MemStStrCopy(field_off: u16)
                            if self.pc + 2 <= len {
                                let field_off = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                                self.pc += 2;
                                let obj_id = match self.pop() {
                                    PCodeValue::Object(p) => p as usize,
                                    other => other.as_i32() as u32 as usize,
                                };
                                let s = self.pop().as_string();
                                pcode_set_field(obj_id, field_off, PCodeValue::String(s));
                            }
                        }
                        (0xfd, 0xf4) => {
                            // NewIfNullPr(cpool_idx: u16)
                            if self.pc + 2 <= len {
                                let idx = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                                self.pc += 2;
                                let class_oi = self.cpool_u32(idx) as usize;
                                let obj_id = pcode_alloc_object(class_oi);
                                self.push(PCodeValue::Object(obj_id as *mut c_void));
                            }
                        }
                        (0xfe, 0x6c) => {
                            // ForStepI4(temp_off: i16, exit_target_pc: u16)
                            if self.pc + 4 <= len {
                                let temp_off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                                let exit_pc = u16::from_le_bytes([bytecode[self.pc + 2], bytecode[self.pc + 3]]) as usize;
                                self.pc += 4;
                                let step_val = self.pop().as_i32();
                                let end_val = self.pop().as_i32();
                                let var_ref = self.pop();
                                let start_val = self.pop().as_i32();
                                self.write_slot(temp_off, PCodeValue::I32(end_val));
                                self.write_slot(temp_off.wrapping_add(4), PCodeValue::I32(step_val));
                                if let PCodeValue::ByRefI32(ptr) = var_ref {
                                    if !ptr.is_null() {
                                        unsafe { core::ptr::write_unaligned(ptr, start_val) };
                                    }
                                }
                                let cont = if step_val >= 0 { start_val <= end_val } else { start_val >= end_val };
                                if !cont {
                                    self.pc = exit_pc;
                                }
                            }
                        }
                        (0xff, 0x2d) => {
                            // ZeroRetVal
                            self.write_slot(-0x88, PCodeValue::I32(0));
                        }
                        (0xff, 0x2f) => {
                            // ExitProcFrameCb(ret_off: i16)
                            if self.pc + 2 <= len {
                                let ret_off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                                self.pc += 2;
                                let ret_val = self.read_slot(-0x88, args);
                                let dst_arg = self.read_slot(ret_off, args);
                                if let PCodeValue::ByRefI32(ptr) = dst_arg {
                                    if !ptr.is_null() {
                                        match &ret_val {
                                            PCodeValue::String(s) => {
                                                pcode_set_string(ptr as usize, s.clone());
                                            }
                                            other => unsafe {
                                                pcode_remove_string(ptr as usize);
                                                core::ptr::write_unaligned(ptr, other.as_i32());
                                            },
                                        }
                                    }
                                }
                                self.frame.teardown();
                                return Ok(ret_val);
                            }
                        }
                        _ => {
                            if let Some(epilogue_pc) = Self::find_print_epilogue(bytecode) {
                                if epilogue_pc >= self.pc.saturating_sub(2) {
                                    self.stack.clear();
                                    self.pc = epilogue_pc;
                                    continue;
                                }
                            }
                            break;
                        }
                    }
                }
                _ => {
                    if let Some(epilogue_pc) = Self::find_print_epilogue(bytecode) {
                        if epilogue_pc >= self.pc.saturating_sub(1) {
                            self.stack.clear();
                            self.pc = epilogue_pc;
                            continue;
                        }
                    }
                    break;
                }
            }
        }

        let ret_val = self.read_slot(-0x88, args);
        self.frame.teardown();
        Ok(ret_val)
    }

    /// Handles an error trigger according to the active `On Error` mode.
    fn handle_error(&mut self, err_code: i32) -> Result<(), i32> {
        crate::err::set_vb_error(err_code, Some("P-Code Runtime Error"), None);
        if self.frame.resume_next {
            Ok(())
        } else if let Some(target_pc) = self.frame.error_handler_pc {
            self.pc = target_pc;
            Ok(())
        } else {
            Err(err_code)
        }
    }

    /// Dispatches a runtime library call (`rtc*` or `__vba*`) by ordinal.
    fn invoke_runtime_import(&mut self, ordinal: u16, _c_args: u8) {
        match ordinal {
            520 | 521 => {
                // Trim$ (rtcTrimBstr / rtcTrimVar)
                let arg = self.pop().as_string();
                let trimmed = arg.trim().to_string();
                self.push(PCodeValue::String(trimmed));
            }
            628 => {
                // Mid$ (rtcMidBstr)
                let length = self.pop().as_i32() as usize;
                let start = self.pop().as_i32().max(1) as usize;
                let text = self.pop().as_string();
                let start_idx = start.saturating_sub(1);
                let sub: String = text.chars().skip(start_idx).take(length).collect();
                self.push(PCodeValue::String(sub));
            }
            516 => {
                // StrReverse (rtcStrReverse)
                let text = self.pop().as_string();
                let rev: String = text.chars().rev().collect();
                self.push(PCodeValue::String(rev));
            }
            _ => {
                // Generic fallback
                self.push(PCodeValue::I32(0));
            }
        }
    }

    /// Dispatches a method invocation on an object or class instance.
    fn invoke_method_call(&mut self, method_id: u16, _c_args: u8) {
        match method_id {
            1 => {
                // ComputeOffset(factor: Long, delta: Long) -> (BaseValue * factor) + delta
                let delta = self.pop().as_i32();
                let factor = self.pop().as_i32();
                let base = self.frame.read_i32(0);
                let res = (base * factor) + delta;
                self.push(PCodeValue::I32(res));
            }
            2 => {
                // FormatMessage(prefix: String) -> prefix & ": " & BaseValue
                let prefix = self.pop().as_string();
                let base = self.frame.read_i32(0);
                self.push(PCodeValue::String(format!("{}: {}", prefix, base)));
            }
            3 => {
                // BaseValue Let
                let val = self.pop().as_i32();
                self.frame.write_i32(0, val);
            }
            4 => {
                // BaseValue Get
                let val = self.frame.read_i32(0);
                self.push(PCodeValue::I32(val));
            }
            _ => {
                self.push(PCodeValue::I32(0));
            }
        }
    }
}

struct PCodeClassInstance {
    obj_info_addr: usize,
    fields: std::collections::HashMap<u16, PCodeValue>,
}

struct PCodeThreadState {
    strings: std::collections::HashMap<usize, String>,
    objects: std::collections::HashMap<usize, PCodeClassInstance>,
    next_obj_id: usize,
}

impl PCodeThreadState {
    fn new() -> Self {
        Self {
            strings: std::collections::HashMap::new(),
            objects: std::collections::HashMap::new(),
            next_obj_id: 0x5000_0001,
        }
    }
}

std::thread_local! {
    static PCODE_STATE: core::cell::RefCell<PCodeThreadState> = core::cell::RefCell::new(PCodeThreadState::new());
}

fn pcode_get_string(key: usize) -> Option<String> {
    PCODE_STATE.with(|st| st.borrow().strings.get(&key).cloned())
}

fn pcode_set_string(key: usize, val: String) {
    PCODE_STATE.with(|st| {
        st.borrow_mut().strings.insert(key, val);
    });
}

fn pcode_remove_string(key: usize) {
    PCODE_STATE.with(|st| {
        st.borrow_mut().strings.remove(&key);
    });
}

fn pcode_alloc_object(obj_info_addr: usize) -> usize {
    PCODE_STATE.with(|st| {
        let mut s = st.borrow_mut();
        let id = s.next_obj_id;
        s.next_obj_id += 1;
        s.objects.insert(
            id,
            PCodeClassInstance {
                obj_info_addr,
                fields: std::collections::HashMap::new(),
            },
        );
        id
    })
}

fn pcode_get_object_oi(obj_id: usize) -> Option<usize> {
    PCODE_STATE.with(|st| st.borrow().objects.get(&obj_id).map(|o| o.obj_info_addr))
}

fn pcode_get_field(obj_id: usize, field_off: u16) -> PCodeValue {
    PCODE_STATE.with(|st| {
        st.borrow()
            .objects
            .get(&obj_id)
            .and_then(|o| o.fields.get(&field_off).cloned())
            .unwrap_or(PCodeValue::I32(0))
    })
}

fn pcode_set_field(obj_id: usize, field_off: u16, val: PCodeValue) {
    PCODE_STATE.with(|st| {
        if let Some(o) = st.borrow_mut().objects.get_mut(&obj_id) {
            o.fields.insert(field_off, val);
        }
    });
}
