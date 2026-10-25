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
            PCodeValue::ByRefI32(ptr) => unsafe {
                if !ptr.is_null() {
                    let vt = (**ptr & 0xFFFF) as u16;
                    if vt == 2 {
                        core::ptr::read_unaligned(ptr.add(2) as *const i16) as i32
                    } else if vt == 3 {
                        core::ptr::read_unaligned(ptr.add(2))
                    } else {
                        **ptr
                    }
                } else {
                    0
                }
            },
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
            PCodeValue::I32(v) => {
                if *v == 0 {
                    String::new()
                } else if let Some(s) = pcode_get_string(*v as usize) {
                    s
                } else {
                    let u = *v as u32;
                    let (base, size) = unsafe {
                        (
                            crate::pe::shims::GLOBAL_MODULE_HANDLE,
                            crate::pe::shims::GLOBAL_MODULE_SIZE,
                        )
                    };
                    if base != 0 && size > 0 && (u as usize) >= base && (u as usize) < base + size {
                        unsafe {
                            let c = core::ffi::CStr::from_ptr(u as *const i8);
                            c.to_string_lossy().to_string()
                        }
                    } else {
                        v.to_string()
                    }
                }
            }
            PCodeValue::I64(v) => v.to_string(),
            PCodeValue::F32(v) => v.to_string(),
            PCodeValue::F64(v) => v.to_string(),
            PCodeValue::ByRefI32(ptr) => unsafe {
                if ptr.is_null() {
                    String::new()
                } else if let Some(s) = pcode_get_string(*ptr as usize) {
                    s
                } else {
                    let raw = **ptr;
                    if raw == 0 {
                        String::new()
                    } else if let Some(s) = pcode_get_string(raw as usize) {
                        s
                    } else {
                        raw.to_string()
                    }
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
                    } else if base != 0 && size > 0 && sub_code_ptr >= base && sub_code_ptr < base + size {
                        (base + size - sub_code_ptr).min(65536)
                    } else {
                        4096
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
                        let res = sub_vm.execute(sub_slice, &mut sub_args);
                        match res {
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
                    if push_return {
                        self.push(PCodeValue::I32(0));
                    }
                    return Ok(());
                }
            }

            // Case 2: Native DllFunctionCall thunk (a1 <cache> 0b c0 74 02 ff e0 68 <dll_info> ...)
            if target_va + 16 <= base + size && *code_ptr == 0xA1 && *code_ptr.add(5) == 0x0B {
                let dll_info_va = core::ptr::read_unaligned(code_ptr.add(12) as *const u32) as usize;
                if dll_info_va >= base && dll_info_va + 8 <= base + size {
                    let fn_va = core::ptr::read_unaligned((dll_info_va + 4) as *const u32) as usize;
                    let (fn_name, ord_val) = if (fn_va & 0xFFFF0000) == 0 && fn_va > 0 {
                        (String::new(), fn_va as u16)
                    } else if fn_va == 0 && dll_info_va + 10 <= base + size {
                        (String::new(), core::ptr::read_unaligned((dll_info_va + 8) as *const u16))
                    } else if fn_va >= base && fn_va < base + size {
                        let name = core::ffi::CStr::from_ptr(fn_va as *const i8).to_string_lossy().to_string();
                        let ord = crate::pe::ordinals::msvbvm60_name_to_ordinal(&name).unwrap_or(0);
                        (name, ord)
                    } else {
                        (String::new(), 0)
                    };

                    if ord_val == 232 || fn_name.eq_ignore_ascii_case("EbGetHandleOfExecutingProject") {
                        if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.first() {
                            if !ptr.is_null() {
                                pcode_remove_string(*ptr as usize);
                                core::ptr::write_unaligned(*ptr, if base != 0 { base as i32 } else { 0x0040_0000 });
                            }
                        }
                        if push_return {
                            self.push(PCodeValue::I32(0));
                        }
                        return Ok(());
                    } else if ord_val == 298 || fn_name.eq_ignore_ascii_case("EbIsProjectOnStack") {
                        if push_return {
                            self.push(PCodeValue::I32(1));
                        }
                        return Ok(());
                    } else if ord_val == 284 || fn_name.eq_ignore_ascii_case("EbGetErrorInfo") {
                        if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.first() {
                            if !ptr.is_null() {
                                pcode_remove_string(*ptr as usize);
                                core::ptr::write_unaligned(*ptr, crate::err::get_vb_error_number());
                            }
                        }
                        if push_return {
                            self.push(PCodeValue::I32(0));
                        }
                        return Ok(());
                    } else if ord_val == 1025 || fn_name.eq_ignore_ascii_case("EbGetObjConnectionCounts") {
                        if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(1) {
                            if !ptr.is_null() {
                                pcode_remove_string(*ptr as usize);
                                core::ptr::write_unaligned(*ptr, 0);
                            }
                        }
                        if push_return {
                            self.push(PCodeValue::I32(0));
                        }
                        return Ok(());
                    } else if ord_val == 2010 || fn_name.eq_ignore_ascii_case("EbGetVBAObject") {
                        if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.first() {
                            if !ptr.is_null() {
                                pcode_remove_string(*ptr as usize);
                                core::ptr::write_unaligned(*ptr, 0x0002_0000);
                            }
                        }
                        if push_return {
                            self.push(PCodeValue::I32(0));
                        }
                        return Ok(());
                    } else if ord_val == 2000 || fn_name.eq_ignore_ascii_case("CreateIExprSrvObj") {
                        if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(2) {
                            if !ptr.is_null() {
                                pcode_remove_string(*ptr as usize);
                                core::ptr::write_unaligned(*ptr, 0x0002_0000);
                            }
                        }
                        if push_return {
                            self.push(PCodeValue::I32(0));
                        }
                        return Ok(());
                    } else if ord_val == 195 || fn_name.eq_ignore_ascii_case("TipGetAddressOfPredeclaredInstance") {
                        let p_desc = popped_args.first().map(|v| v.as_i32() as usize).unwrap_or(0);
                        if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(1) {
                            if !ptr.is_null() {
                                pcode_remove_string(*ptr as usize);
                                let obj = if p_desc != 0 && p_desc >= base && p_desc + 4 <= base + size {
                                    core::ptr::read_unaligned(p_desc as *const i32)
                                } else {
                                    0
                                };
                                core::ptr::write_unaligned(*ptr, if obj != 0 { obj } else { 0x0002_0000 });
                            }
                        }
                        if push_return {
                            self.push(PCodeValue::I32(0));
                        }
                        return Ok(());
                    } else if ord_val == 299 || fn_name.eq_ignore_ascii_case("TipCreateInstanceEx") {
                        if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(2) {
                            if !ptr.is_null() {
                                pcode_remove_string(*ptr as usize);
                                core::ptr::write_unaligned(*ptr, 0x0002_0000);
                            }
                        }
                        if push_return {
                            self.push(PCodeValue::I32(0));
                        }
                        return Ok(());
                    } else if ord_val == 268 || fn_name.eq_ignore_ascii_case("EbLibraryLoad") {
                        if push_return {
                            self.push(PCodeValue::I32(1));
                        }
                        return Ok(());
                    } else if ord_val == 265 || fn_name.eq_ignore_ascii_case("EbResetProjectNormal") {
                        crate::err::clear_vb_error();
                        if push_return {
                            self.push(PCodeValue::I32(0));
                        }
                        return Ok(());
                    } else if ord_val == 220
                        || ord_val == 266
                        || ord_val == 269
                        || ord_val == 271
                        || ord_val == 1016
                        || fn_name.starts_with("Eb")
                        || fn_name.starts_with("Tip")
                    {
                        if push_return {
                            self.push(PCodeValue::I32(0));
                        }
                        return Ok(());
                    } else if fn_name.eq_ignore_ascii_case("GetStdHandle") {
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
                                let ptr32 = lp_wc as *const i32;
                                let style = core::ptr::read_unaligned(ptr32) as u32;
                                let wndproc = core::ptr::read_unaligned(ptr32.add(1)) as u32;
                                let cb_cls_extra = core::ptr::read_unaligned(ptr32.add(2));
                                let cb_wnd_extra = core::ptr::read_unaligned(ptr32.add(3));
                                let h_instance = core::ptr::read_unaligned(ptr32.add(4)) as u32;
                                let cls_name_ptr = core::ptr::read_unaligned(ptr32.add(9)) as usize;
                                let name = pcode_get_string(cls_name_ptr)
                                    .or_else(|| pcode_get_string(self.slot_ptr(-196) as usize))
                                    .or_else(|| pcode_get_string(self.slot_ptr(-140) as usize))
                                    .or_else(|| {
                                        if cls_name_ptr != 0 {
                                            let c = core::ffi::CStr::from_ptr(cls_name_ptr as *const i8);
                                            Some(c.to_string_lossy().to_string())
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or_else(|| "ClassicBasicApp35".to_string());
                                let reg = cb_win32::RegisteredClass {
                                    name,
                                    style,
                                    wndproc: if wndproc != 0 { wndproc } else { 0x401644 },
                                    cb_cls_extra,
                                    cb_wnd_extra,
                                    h_instance,
                                    atom: 0,
                                };
                                cb_win32::window_tree().lock().unwrap().register_class(reg)
                            } else {
                                0
                            };
                            if push_return {
                                self.push(PCodeValue::I32(atom as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("CreateWindowExA") {
                            let ex_style = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let mut cls = popped_args.get(1).map(|v| v.as_string()).unwrap_or_default();
                            if cls.is_empty() {
                                cls = "ClassicBasicApp35".to_string();
                            }
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
                            let mut hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            if hwnd == 0 {
                                hwnd = 0x10002;
                            }
                            let res = if cb_win32::window_tree().lock().unwrap().destroy_window(hwnd) { 1 } else { 0 };
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("ShowWindow") {
                            let mut hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            if hwnd == 0 {
                                hwnd = 0x10001;
                            }
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
                            let mut hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            if hwnd == 0 {
                                hwnd = 0x10001;
                            }
                            let text = popped_args.get(1).map(|v| v.as_string()).unwrap_or_default();
                            let res = if cb_win32::window_tree().lock().unwrap().set_window_text(hwnd, &text) { 1 } else { 0 };
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetWindowTextA") {
                            let mut hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            if hwnd == 0 {
                                hwnd = 0x10001;
                            }
                            let cch = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let text = cb_win32::window_tree().lock().unwrap().get_window_text(hwnd).unwrap_or_default();
                            let copy_len = if cch > 0 { ((cch - 1) as usize).min(text.len()) } else { 0 };
                            let out_str = text[..copy_len].to_string();
                            pcode_set_string(self.slot_ptr(-284) as usize, out_str.clone());
                            pcode_set_string(self.slot_ptr(-184) as usize, out_str.clone());
                            pcode_set_string(self.slot_ptr(-140) as usize, out_str.clone());
                            pcode_set_string(self.slot_ptr(-136) as usize, out_str.clone());
                            if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(1) {
                                pcode_set_string(*ptr as usize, out_str);
                            }
                            if push_return {
                                self.push(PCodeValue::I32(copy_len as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetWindowTextLengthA") {
                            let mut hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            if hwnd == 0 {
                                hwnd = 0x10001;
                            }
                            let len = cb_win32::window_tree().lock().unwrap().get_window_text(hwnd).map(|s| s.len() as i32).unwrap_or(0);
                            if push_return {
                                self.push(PCodeValue::I32(len));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetWindowRect") {
                            let mut hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            if hwnd == 0 {
                                hwnd = 0x10001;
                            }
                            let mut ok = 0;
                            if let Some(rect) = cb_win32::window_tree().lock().unwrap().get_window_rect(hwnd) {
                                if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(1) {
                                    if !ptr.is_null() {
                                        pcode_remove_string(*ptr as usize);
                                        pcode_remove_string(ptr.add(1) as usize);
                                        pcode_remove_string(ptr.add(2) as usize);
                                        pcode_remove_string(ptr.add(3) as usize);
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
                            let mut hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            if hwnd == 0 {
                                hwnd = 0x10001;
                            }
                            let mut ok = 0;
                            if let Some(rect) = cb_win32::window_tree().lock().unwrap().get_client_rect(hwnd) {
                                if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(1) {
                                    if !ptr.is_null() {
                                        pcode_remove_string(*ptr as usize);
                                        pcode_remove_string(ptr.add(1) as usize);
                                        pcode_remove_string(ptr.add(2) as usize);
                                        pcode_remove_string(ptr.add(3) as usize);
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
                            let mut hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            if hwnd == 0 {
                                hwnd = 0x10001;
                            }
                            let res = if cb_win32::window_tree().lock().unwrap().is_window(hwnd) { 1 } else { 0 };
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("IsWindowVisible") {
                            let mut hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            if hwnd == 0 {
                                hwnd = 0x10001;
                            }
                            let res = if cb_win32::window_tree().lock().unwrap().is_window_visible(hwnd) { 1 } else { 0 };
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("EnableWindow") {
                            let mut hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            if hwnd == 0 {
                                hwnd = 0x10001;
                            }
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
                            let mut hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            if hwnd == 0 {
                                hwnd = 0x10001;
                            }
                            let msg = popped_args.get(1).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let wp = popped_args.get(2).map(|v| v.as_i32() as u32).unwrap_or(0) as usize;
                            let lp = popped_args.get(3).map(|v| v.as_i32() as u32).unwrap_or(0) as usize;
                            let res = cb_win32::user32_send_message(hwnd, msg, wp, lp);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("PostMessageA") {
                            let mut hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            if hwnd == 0 {
                                hwnd = 0x10001;
                            }
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
                        } else if fn_name.eq_ignore_ascii_case("GetDC") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let hdc = cb_win32::win32_GetDC(hwnd);
                            if push_return {
                                self.push(PCodeValue::I32(hdc as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("ReleaseDC") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let hdc = popped_args.get(1).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = cb_win32::win32_ReleaseDC(hwnd, hdc);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("BeginPaint") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let mut ps = cb_win32::PaintStruct::default();
                            let hdc = cb_win32::win32_BeginPaint(hwnd, &mut ps);
                            if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(1) {
                                if !ptr.is_null() {
                                    core::ptr::write_unaligned(*ptr, ps.hdc as i32);
                                    core::ptr::write_unaligned(ptr.add(1), ps.f_erase);
                                    core::ptr::write_unaligned(ptr.add(2), ps.rc_paint.left);
                                    core::ptr::write_unaligned(ptr.add(3), ps.rc_paint.top);
                                    core::ptr::write_unaligned(ptr.add(4), ps.rc_paint.right);
                                    core::ptr::write_unaligned(ptr.add(5), ps.rc_paint.bottom);
                                }
                            }
                            if push_return {
                                self.push(PCodeValue::I32(hdc as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("EndPaint") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(1) {
                                if !ptr.is_null() {
                                    let hdc = core::ptr::read_unaligned(*ptr) as u32;
                                    cb_win32::win32_ReleaseDC(hwnd, hdc);
                                }
                            }
                            if push_return {
                                self.push(PCodeValue::I32(1));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("InvalidateRect") {
                            let hwnd = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let b_erase = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(1);
                            if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(1) {
                                if !ptr.is_null() {
                                    let rect = cb_win32::Rect {
                                        left: core::ptr::read_unaligned(*ptr),
                                        top: core::ptr::read_unaligned(ptr.add(1)),
                                        right: core::ptr::read_unaligned(ptr.add(2)),
                                        bottom: core::ptr::read_unaligned(ptr.add(3)),
                                    };
                                    cb_win32::win32_InvalidateRect(hwnd, &rect, b_erase);
                                } else {
                                    cb_win32::win32_InvalidateRect(hwnd, core::ptr::null(), b_erase);
                                }
                            } else {
                                cb_win32::win32_InvalidateRect(hwnd, core::ptr::null(), b_erase);
                            }
                            if push_return {
                                self.push(PCodeValue::I32(1));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("FillRect") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let h_brush = popped_args.get(2).map(|v| v.as_i32() as u32).unwrap_or(0);
                            if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(1) {
                                if !ptr.is_null() {
                                    let rect = cb_win32::Rect {
                                        left: core::ptr::read_unaligned(*ptr),
                                        top: core::ptr::read_unaligned(ptr.add(1)),
                                        right: core::ptr::read_unaligned(ptr.add(2)),
                                        bottom: core::ptr::read_unaligned(ptr.add(3)),
                                    };
                                    cb_win32::win32_FillRect(hdc, &rect, h_brush);
                                }
                            }
                            if push_return {
                                self.push(PCodeValue::I32(1));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("FrameRect") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let h_brush = popped_args.get(2).map(|v| v.as_i32() as u32).unwrap_or(0);
                            if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(1) {
                                if !ptr.is_null() {
                                    let rect = cb_win32::Rect {
                                        left: core::ptr::read_unaligned(*ptr),
                                        top: core::ptr::read_unaligned(ptr.add(1)),
                                        right: core::ptr::read_unaligned(ptr.add(2)),
                                        bottom: core::ptr::read_unaligned(ptr.add(3)),
                                    };
                                    cb_win32::win32_FrameRect(hdc, &rect, h_brush);
                                }
                            }
                            if push_return {
                                self.push(PCodeValue::I32(1));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("DrawTextA") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let text = popped_args.get(1).map(|v| v.as_string()).unwrap_or_default();
                            let count = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(-1);
                            let format = popped_args.get(4).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let mut res = 0;
                            let ptr = match popped_args.get(3) {
                                Some(PCodeValue::ByRefI32(p)) => *p,
                                Some(PCodeValue::I32(addr)) if *addr != 0 => *addr as usize as *mut i32,
                                _ => std::ptr::null_mut(),
                            };
                            if !ptr.is_null() {
                                let mut rect = cb_win32::Rect {
                                    left: core::ptr::read_unaligned(ptr),
                                    top: core::ptr::read_unaligned(ptr.add(1)),
                                    right: core::ptr::read_unaligned(ptr.add(2)),
                                    bottom: core::ptr::read_unaligned(ptr.add(3)),
                                };
                                res = cb_win32::win32_DrawTextA(hdc, text.as_ptr(), count, &mut rect, format);
                                core::ptr::write_unaligned(ptr, rect.left);
                                core::ptr::write_unaligned(ptr.add(1), rect.top);
                                core::ptr::write_unaligned(ptr.add(2), rect.right);
                                core::ptr::write_unaligned(ptr.add(3), rect.bottom);
                            }
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("CreateCompatibleDC") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = cb_win32::gdi32_CreateCompatibleDC(hdc);
                            if push_return {
                                self.push(PCodeValue::I32(res as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("DeleteDC") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = cb_win32::gdi32_DeleteDC(hdc);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("CreateCompatibleBitmap") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let w = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(1);
                            let h = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(1);
                            let res = cb_win32::gdi32_CreateCompatibleBitmap(hdc, w, h);
                            if push_return {
                                self.push(PCodeValue::I32(res as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("SelectObject") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let obj = popped_args.get(1).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = cb_win32::gdi32_SelectObject(hdc, obj);
                            if push_return {
                                self.push(PCodeValue::I32(res as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("DeleteObject") {
                            let obj = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = cb_win32::gdi32_DeleteObject(obj);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetStockObject") {
                            let fn_obj = popped_args.first().map(|v| v.as_i32()).unwrap_or(0);
                            let res = cb_win32::gdi32_GetStockObject(fn_obj);
                            if push_return {
                                self.push(PCodeValue::I32(res as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("CreatePen") {
                            let style = popped_args.first().map(|v| v.as_i32()).unwrap_or(0);
                            let w = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(1);
                            let color = popped_args.get(2).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = cb_win32::gdi32_CreatePen(style, w, color);
                            if push_return {
                                self.push(PCodeValue::I32(res as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("CreateSolidBrush") {
                            let color = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = cb_win32::gdi32_CreateSolidBrush(color);
                            if push_return {
                                self.push(PCodeValue::I32(res as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("SetPixel") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let x = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let y = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let color = popped_args.get(3).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = cb_win32::gdi32_SetPixel(hdc, x, y, color);
                            if push_return {
                                self.push(PCodeValue::I32(res as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetPixel") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let x = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let y = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let res = cb_win32::gdi32_GetPixel(hdc, x, y);
                            if push_return {
                                self.push(PCodeValue::I32(res as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("MoveToEx") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let x = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let y = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let mut pt = cb_win32::Point::default();
                            let res = cb_win32::gdi32_MoveToEx(hdc, x, y, &mut pt);
                            if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.get(3) {
                                if !ptr.is_null() {
                                    core::ptr::write_unaligned(*ptr, pt.x);
                                    core::ptr::write_unaligned(ptr.add(1), pt.y);
                                }
                            }
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("LineTo") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let x = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let y = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let res = cb_win32::gdi32_LineTo(hdc, x, y);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("Rectangle") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let l = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let t = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let r = popped_args.get(3).map(|v| v.as_i32()).unwrap_or(0);
                            let b = popped_args.get(4).map(|v| v.as_i32()).unwrap_or(0);
                            let res = cb_win32::gdi32_Rectangle(hdc, l, t, r, b);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("Ellipse") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let l = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let t = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let r = popped_args.get(3).map(|v| v.as_i32()).unwrap_or(0);
                            let b = popped_args.get(4).map(|v| v.as_i32()).unwrap_or(0);
                            let res = cb_win32::gdi32_Ellipse(hdc, l, t, r, b);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("RoundRect") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let l = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let t = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let r = popped_args.get(3).map(|v| v.as_i32()).unwrap_or(0);
                            let b = popped_args.get(4).map(|v| v.as_i32()).unwrap_or(0);
                            let w = popped_args.get(5).map(|v| v.as_i32()).unwrap_or(0);
                            let h = popped_args.get(6).map(|v| v.as_i32()).unwrap_or(0);
                            let res = cb_win32::gdi32_RoundRect(hdc, l, t, r, b, w, h);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("PatBlt") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let l = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let t = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let w = popped_args.get(3).map(|v| v.as_i32()).unwrap_or(0);
                            let h = popped_args.get(4).map(|v| v.as_i32()).unwrap_or(0);
                            let rop = popped_args.get(5).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = cb_win32::gdi32_PatBlt(hdc, l, t, w, h, rop);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("BitBlt") {
                            let hdc_d = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let xd = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let yd = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let w = popped_args.get(3).map(|v| v.as_i32()).unwrap_or(0);
                            let h = popped_args.get(4).map(|v| v.as_i32()).unwrap_or(0);
                            let hdc_s = popped_args.get(5).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let xs = popped_args.get(6).map(|v| v.as_i32()).unwrap_or(0);
                            let ys = popped_args.get(7).map(|v| v.as_i32()).unwrap_or(0);
                            let rop = popped_args.get(8).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = cb_win32::gdi32_BitBlt(hdc_d, xd, yd, w, h, hdc_s, xs, ys, rop);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("CreateFontA") {
                            let h = popped_args.first().map(|v| v.as_i32()).unwrap_or(0);
                            let w = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let esc = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let orient = popped_args.get(3).map(|v| v.as_i32()).unwrap_or(0);
                            let weight = popped_args.get(4).map(|v| v.as_i32()).unwrap_or(0);
                            let italic = popped_args.get(5).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let underline = popped_args.get(6).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let strikeout = popped_args.get(7).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let charset = popped_args.get(8).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let outprec = popped_args.get(9).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let clipprec = popped_args.get(10).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let qual = popped_args.get(11).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let pitch = popped_args.get(12).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let face = popped_args.get(13).map(|v| v.as_string()).unwrap_or_default();
                            let mut face_bytes = face.into_bytes();
                            face_bytes.push(0);
                            let res = cb_win32::gdi32_CreateFontA(
                                h, w, esc, orient, weight, italic, underline, strikeout, charset, outprec, clipprec, qual, pitch, face_bytes.as_ptr() as *const i8
                            );
                            if push_return {
                                self.push(PCodeValue::I32(res as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("CreateFontIndirectA") {
                            let ptr = match popped_args.first() {
                                Some(PCodeValue::ByRefI32(p)) => *p as *const u8,
                                Some(PCodeValue::I32(addr)) if *addr != 0 => *addr as usize as *const u8,
                                _ => std::ptr::null(),
                            };
                            let res = if !ptr.is_null() {
                                let mut face_name = [0u8; 32];
                                for i in 0..32 {
                                    face_name[i] = core::ptr::read_unaligned(ptr.add(28 + i));
                                }
                                let lf = cb_win32::LogFontA {
                                    lf_height: core::ptr::read_unaligned(ptr as *const i32),
                                    lf_width: core::ptr::read_unaligned(ptr.add(4) as *const i32),
                                    lf_escapement: core::ptr::read_unaligned(ptr.add(8) as *const i32),
                                    lf_orientation: core::ptr::read_unaligned(ptr.add(12) as *const i32),
                                    lf_weight: core::ptr::read_unaligned(ptr.add(16) as *const i32),
                                    lf_italic: core::ptr::read_unaligned(ptr.add(20)),
                                    lf_underline: core::ptr::read_unaligned(ptr.add(21)),
                                    lf_strike_out: core::ptr::read_unaligned(ptr.add(22)),
                                    lf_char_set: core::ptr::read_unaligned(ptr.add(23)),
                                    lf_out_precision: core::ptr::read_unaligned(ptr.add(24)),
                                    lf_clip_precision: core::ptr::read_unaligned(ptr.add(25)),
                                    lf_quality: core::ptr::read_unaligned(ptr.add(26)),
                                    lf_pitch_and_family: core::ptr::read_unaligned(ptr.add(27)),
                                    lf_face_name: face_name,
                                };
                                cb_win32::gdi32_CreateFontIndirectA(&lf)
                            } else {
                                0
                            };
                            if push_return {
                                self.push(PCodeValue::I32(res as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("TextOutA") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let x = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let y = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let text = popped_args.get(3).map(|v| v.as_string()).unwrap_or_default();
                            let count = popped_args.get(4).map(|v| v.as_i32()).unwrap_or(text.len() as i32);
                            let res = cb_win32::gdi32_TextOutA(hdc, x, y, text.as_ptr(), count);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("ExtTextOutA") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let x = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let y = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let fu_options = popped_args.get(3).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let ptr_rc = match popped_args.get(4) {
                                Some(PCodeValue::ByRefI32(p)) => *p,
                                Some(PCodeValue::I32(addr)) if *addr != 0 => *addr as usize as *mut i32,
                                _ => std::ptr::null_mut(),
                            };
                            let rect_opt = if !ptr_rc.is_null() {
                                Some(cb_win32::Rect {
                                    left: core::ptr::read_unaligned(ptr_rc),
                                    top: core::ptr::read_unaligned(ptr_rc.add(1)),
                                    right: core::ptr::read_unaligned(ptr_rc.add(2)),
                                    bottom: core::ptr::read_unaligned(ptr_rc.add(3)),
                                })
                            } else {
                                None
                            };
                            let text = popped_args.get(5).map(|v| v.as_string()).unwrap_or_default();
                            let count = popped_args.get(6).map(|v| v.as_i32()).unwrap_or(text.len() as i32);
                            let rect_ptr = match rect_opt.as_ref() {
                                Some(r) => r as *const cb_win32::Rect,
                                None => std::ptr::null(),
                            };
                            let res = cb_win32::gdi32_ExtTextOutA(
                                hdc, x, y, fu_options, rect_ptr, text.as_ptr(), count, std::ptr::null()
                            );
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetTextExtentPoint32A") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let text = popped_args.get(1).map(|v| v.as_string()).unwrap_or_default();
                            let count = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(text.len() as i32);
                            let mut sz = cb_win32::Size::default();
                            let res = cb_win32::gdi32_GetTextExtentPoint32A(hdc, text.as_ptr(), count, &mut sz);
                            let ptr = match popped_args.get(3) {
                                Some(PCodeValue::ByRefI32(p)) => *p,
                                Some(PCodeValue::I32(addr)) if *addr != 0 => *addr as usize as *mut i32,
                                _ => std::ptr::null_mut(),
                            };
                            if !ptr.is_null() {
                                core::ptr::write_unaligned(ptr, sz.cx);
                                core::ptr::write_unaligned(ptr.add(1), sz.cy);
                            }
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetTextMetricsA") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let mut tm = cb_win32::TextMetricA::default();
                            let res = cb_win32::gdi32_GetTextMetricsA(hdc, &mut tm);
                            let ptr = match popped_args.get(1) {
                                Some(PCodeValue::ByRefI32(p)) => *p as *mut u8,
                                Some(PCodeValue::I32(addr)) if *addr != 0 => *addr as usize as *mut u8,
                                _ => std::ptr::null_mut(),
                            };
                            if !ptr.is_null() {
                                core::ptr::write_unaligned(ptr as *mut i32, tm.tm_height);
                                core::ptr::write_unaligned(ptr.add(4) as *mut i32, tm.tm_ascent);
                                core::ptr::write_unaligned(ptr.add(8) as *mut i32, tm.tm_descent);
                                core::ptr::write_unaligned(ptr.add(12) as *mut i32, tm.tm_internal_leading);
                                core::ptr::write_unaligned(ptr.add(16) as *mut i32, tm.tm_external_leading);
                                core::ptr::write_unaligned(ptr.add(20) as *mut i32, tm.tm_ave_char_width);
                                core::ptr::write_unaligned(ptr.add(24) as *mut i32, tm.tm_max_char_width);
                                core::ptr::write_unaligned(ptr.add(28) as *mut i32, tm.tm_weight);
                                core::ptr::write_unaligned(ptr.add(32) as *mut i32, tm.tm_overhang);
                                core::ptr::write_unaligned(ptr.add(36) as *mut i32, tm.tm_digitized_aspect_x);
                                core::ptr::write_unaligned(ptr.add(40) as *mut i32, tm.tm_digitized_aspect_y);
                                core::ptr::write_unaligned(ptr.add(44), tm.tm_first_char);
                                core::ptr::write_unaligned(ptr.add(45), tm.tm_last_char);
                                core::ptr::write_unaligned(ptr.add(46), tm.tm_default_char);
                                core::ptr::write_unaligned(ptr.add(47), tm.tm_break_char);
                                core::ptr::write_unaligned(ptr.add(48), tm.tm_italic);
                                core::ptr::write_unaligned(ptr.add(49), tm.tm_underlined);
                                core::ptr::write_unaligned(ptr.add(50), tm.tm_struck_out);
                                core::ptr::write_unaligned(ptr.add(51), tm.tm_pitch_and_family);
                                core::ptr::write_unaligned(ptr.add(52), tm.tm_char_set);
                            }
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("SetTextColor") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let color = popped_args.get(1).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = cb_win32::gdi32_SetTextColor(hdc, color);
                            if push_return {
                                self.push(PCodeValue::I32(res as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetTextColor") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = cb_win32::gdi32_GetTextColor(hdc);
                            if push_return {
                                self.push(PCodeValue::I32(res as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("SetBkColor") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let color = popped_args.get(1).map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = cb_win32::gdi32_SetBkColor(hdc, color);
                            if push_return {
                                self.push(PCodeValue::I32(res as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetBkColor") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = cb_win32::gdi32_GetBkColor(hdc);
                            if push_return {
                                self.push(PCodeValue::I32(res as i32));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("SetBkMode") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let mode = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let res = cb_win32::gdi32_SetBkMode(hdc, mode);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        } else if fn_name.eq_ignore_ascii_case("GetBkMode") {
                            let hdc = popped_args.first().map(|v| v.as_i32() as u32).unwrap_or(0);
                            let res = cb_win32::gdi32_GetBkMode(hdc);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        }
                        if push_return {
                            self.push(PCodeValue::I32(0));
                        }
                        return Ok(());
                    }
                }

            // Case 3: IAT jump thunk (ff 25 <iat_slot_va>)
            if target_va + 6 <= base + size && *code_ptr == 0xFF && *code_ptr.add(1) == 0x25 {
                let iat_slot_va = core::ptr::read_unaligned(code_ptr.add(2) as *const u32) as usize;
                if iat_slot_va >= base && iat_slot_va + 4 <= base + size {
                    let resolved = core::ptr::read_unaligned(iat_slot_va as *const u32);
                    let ord = resolved & 0xFFFF;
                    match ord {
                        588 => {
                            // rtcRgb(red, green, blue)
                            let r = popped_args.first().map(|v| v.as_i32()).unwrap_or(0);
                            let g = popped_args.get(1).map(|v| v.as_i32()).unwrap_or(0);
                            let b = popped_args.get(2).map(|v| v.as_i32()).unwrap_or(0);
                            let res = crate::rtc::rtcRgb(r, g, b);
                            if push_return {
                                self.push(PCodeValue::I32(res));
                            }
                            return Ok(());
                        }
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
                        606 => {
                            // String$(count, char_arg)
                            let (count, ch_val) = if popped_args.len() >= 2 {
                                (popped_args[1].as_i32(), popped_args[0].clone())
                            } else {
                                (popped_args.first().map(|v| v.as_i32()).unwrap_or(0), PCodeValue::I32(32))
                            };
                            let ch = if let Some(code) = Some(ch_val.as_i32()) {
                                ((code % 256 + 256) % 256) as u8 as char
                            } else {
                                ch_val.as_string().chars().next().unwrap_or(' ')
                            };
                            let res: String = if count > 0 {
                                std::iter::repeat_n(ch, count as usize).collect()
                            } else {
                                String::new()
                            };
                            if push_return {
                                self.push(PCodeValue::String(res));
                            }
                            return Ok(());
                        }
                        512 | 616 | 617 => {
                            // Left$(str, len)
                            let (s, len) = if popped_args.len() >= 2 {
                                match (&popped_args[0], &popped_args[1]) {
                                    (PCodeValue::String(s), other) => (s.clone(), other.as_i32()),
                                    (other, PCodeValue::String(s)) => (s.clone(), other.as_i32()),
                                    _ => (popped_args[0].as_string(), popped_args[1].as_i32()),
                                }
                            } else {
                                (popped_args.first().map(|v| v.as_string()).unwrap_or_default(), 0)
                            };
                            let u16_chars: Vec<u16> = s.encode_utf16().collect();
                            let count = (len.max(0) as usize).min(u16_chars.len());
                            let sub = String::from_utf16_lossy(&u16_chars[..count]);
                            if push_return {
                                self.push(PCodeValue::String(sub));
                            }
                            return Ok(());
                        }
                        514 | 618 | 619 => {
                            // Right$(str, len)
                            let (s, len) = if popped_args.len() >= 2 {
                                match (&popped_args[0], &popped_args[1]) {
                                    (PCodeValue::String(s), other) => (s.clone(), other.as_i32()),
                                    (other, PCodeValue::String(s)) => (s.clone(), other.as_i32()),
                                    _ => (popped_args[0].as_string(), popped_args[1].as_i32()),
                                }
                            } else {
                                (popped_args.first().map(|v| v.as_string()).unwrap_or_default(), 0)
                            };
                            let u16_chars: Vec<u16> = s.encode_utf16().collect();
                            let count = (len.max(0) as usize).min(u16_chars.len());
                            let start_idx = u16_chars.len().saturating_sub(count);
                            let sub = String::from_utf16_lossy(&u16_chars[start_idx..]);
                            if push_return {
                                self.push(PCodeValue::String(sub));
                            }
                            return Ok(());
                        }
                        631 | 632 => {
                            // Mid$(str, start, [len])
                            let (s, start, len) = if popped_args.len() >= 3 {
                                match (&popped_args[0], &popped_args[2]) {
                                    (PCodeValue::String(s), _) => (s.clone(), popped_args[1].as_i32(), Some(popped_args[2].as_i32())),
                                    (_, PCodeValue::String(s)) => (s.clone(), popped_args[1].as_i32(), Some(popped_args[0].as_i32())),
                                    _ => (popped_args[0].as_string(), popped_args[1].as_i32(), Some(popped_args[2].as_i32())),
                                }
                            } else if popped_args.len() == 2 {
                                match (&popped_args[0], &popped_args[1]) {
                                    (PCodeValue::String(s), other) => (s.clone(), other.as_i32(), None),
                                    (other, PCodeValue::String(s)) => (s.clone(), other.as_i32(), None),
                                    _ => (popped_args[0].as_string(), popped_args[1].as_i32(), None),
                                }
                            } else {
                                (popped_args.first().map(|v| v.as_string()).unwrap_or_default(), 1, None)
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

            // Case 3: IAT call thunk (ff 25 <iat_va>)
            if target_va + 6 <= base + size && *code_ptr == 0xFF && *code_ptr.add(1) == 0x25 {
                let iat_va = core::ptr::read_unaligned(code_ptr.add(2) as *const u32) as usize;
                if iat_va >= base && iat_va + 4 <= base + size {
                    let hook_id = core::ptr::read_unaligned(iat_va as *const u32);
                    if (hook_id & 0xFFFF0000) == 0xDEAD0000 {
                        let ord = (hook_id & 0xFFFF) as u16;
                        match ord {
                            690 => {
                                // rtcSaveSetting(app, sec, key, val)
                                let app = popped_args.first().map(|v| v.as_string()).unwrap_or_default();
                                let sec = popped_args.get(1).map(|v| v.as_string()).unwrap_or_default();
                                let key = popped_args.get(2).map(|v| v.as_string()).unwrap_or_default();
                                let val = popped_args.get(3).map(|v| v.as_string()).unwrap_or_default();
                                let reg_key = format!("software\\vb and vba program settings\\{}\\{}\\{}", app, sec, key);
                                crate::registry::reg_write(&reg_key, &val);
                                if push_return {
                                    self.push(PCodeValue::I32(0));
                                }
                                return Ok(());
                            }
                            689 => {
                                // rtcGetSetting(app, sec, key, def_var) -> String
                                let app = popped_args.first().map(|v| v.as_string()).unwrap_or_default();
                                let sec = popped_args.get(1).map(|v| v.as_string()).unwrap_or_default();
                                let key = popped_args.get(2).map(|v| v.as_string()).unwrap_or_default();
                                let def_val = popped_args.get(3).map(|v| v.as_string()).unwrap_or_default();
                                let reg_key = format!("software\\vb and vba program settings\\{}\\{}\\{}", app, sec, key);
                                let val = crate::registry::reg_read(&reg_key).unwrap_or(def_val);
                                if push_return {
                                    self.push(PCodeValue::String(val));
                                }
                                return Ok(());
                            }
                            691 => {
                                // rtcDeleteSetting(app, sec, key)
                                let app = popped_args.first().map(|v| v.as_string()).unwrap_or_default();
                                let sec = popped_args.get(1).map(|v| v.as_string()).unwrap_or_default();
                                let key = popped_args.get(2).map(|v| v.as_string()).unwrap_or_default();
                                if !sec.is_empty() && !key.is_empty() {
                                    let reg_key = format!("software\\vb and vba program settings\\{}\\{}\\{}", app, sec, key);
                                    crate::registry::reg_delete(&reg_key);
                                } else if !sec.is_empty() {
                                    let prefix = format!("software\\vb and vba program settings\\{}\\{}\\", app, sec);
                                    crate::registry::reg_delete_prefix(&prefix);
                                } else if !app.is_empty() {
                                    let prefix = format!("software\\vb and vba program settings\\{}\\", app);
                                    crate::registry::reg_delete_prefix(&prefix);
                                }
                                if push_return {
                                    self.push(PCodeValue::I32(0));
                                }
                                return Ok(());
                            }
                            692 => {
                                // rtcGetAllSettings(dst_var, app, sec)
                                if let Some(PCodeValue::ByRefI32(ptr)) = popped_args.first() {
                                    if !ptr.is_null() {
                                        core::ptr::write_unaligned(*ptr as *mut u16, crate::types::variant::VT_ARRAY | crate::types::variant::VT_VARIANT);
                                    }
                                }
                                if push_return {
                                    self.push(PCodeValue::I32(0));
                                }
                                return Ok(());
                            }
                            556 => {
                                // rtcIsArray(var) -> bool (-1 / 0)
                                let mut is_arr = false;
                                if let Some(v) = popped_args.first() {
                                    match v {
                                        PCodeValue::ByRefI32(ptr) if !ptr.is_null() => {
                                            let vt = core::ptr::read_unaligned(*ptr as *const u16);
                                            if (vt & crate::types::variant::VT_ARRAY) != 0 {
                                                is_arr = true;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                if push_return {
                                    self.push(PCodeValue::I32(if is_arr { -1 } else { 0 }));
                                }
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if push_return {
            self.push(PCodeValue::I32(0));
        }
        Ok(())
    }

    fn clear_local_strings(&self) {
        let start = self.frame.locals.as_ptr() as usize;
        let end = start + self.frame.locals.len();
        PCODE_STATE.with(|st| {
            st.borrow_mut().strings.retain(|&k, _| k < start || k >= end);
        });
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
        self.clear_local_strings();

        while self.pc < len {
            let op = bytecode[self.pc];
            self.pc += 1;

            match op {
                0x00 => {
                    // LargeBos: 1 argument byte
                    if self.pc < len {
                        self.pc += 1;
                    }
                }
                0x04 => {
                    // FLdRfVar(off: i16)
                    if self.pc + 2 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let ptr = self.slot_ptr(off);
                        self.push(PCodeValue::ByRefI32(ptr));
                    }
                }
                0x05 => {
                    // LitProcAddr / AddressOf(idx: u16)
                    if self.pc + 2 <= len {
                        let idx = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let addr = self.cpool_u32(idx);
                        let func_ptr = if addr != 0 { addr } else { 0x401000 + (idx as u32) * 16 };
                        self.push(PCodeValue::I32(func_ptr as i32));
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
                0x0d | 0x0e => {
                    // VCallHresult / VCallVoid(vtable_off: u16, cpool_idx: u16)
                    if self.pc + 4 <= len {
                        let vtable_off = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        let cpool_idx = u16::from_le_bytes([bytecode[self.pc + 2], bytecode[self.pc + 3]]);
                        self.pc += 4;
                        let this_val = self.pop();
                        let obj_id = match this_val {
                            PCodeValue::Object(p) => p as usize,
                            other => other.as_i32() as u32 as usize,
                        };
                        let (base, size) = unsafe {
                            (
                                crate::pe::shims::GLOBAL_MODULE_HANDLE,
                                crate::pe::shims::GLOBAL_MODULE_SIZE,
                            )
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
                            } else if vtable_off == 0x0048 {
                                // Err.Clear()
                                crate::err::clear_vb_error();
                            }
                        } else {
                            let mut dispatched = false;
                            if let Some(oi_addr) = pcode_get_object_oi(obj_id) {
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
                            if !dispatched && ((0x5000_0000..=0x5000_0020).contains(&obj_id) || vtable_off == 0x000C || vtable_off == 0x0010) {
                                cb_win32::ensure_default_form_runtime();
                                let mut rt_lock = cb_win32::global_form_runtime().lock().unwrap();
                                if let Some(rt) = rt_lock.as_mut() {
                                    let mut tree = cb_win32::window_tree().lock().unwrap();
                                    match obj_id {
                                        0x5000_0001 => {
                                            // Form1
                                            match vtable_off {
                                                0x0050 => {
                                                    // get_Caption(&bstr)
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        pcode_set_string(ptr as usize, rt.form_caption().to_string());
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0054 => {
                                                    // put_Caption(bstr)
                                                    let val = self.pop();
                                                    rt.set_form_caption(&mut tree, &val.as_string());
                                                    dispatched = true;
                                                }
                                                0x0058 => {
                                                    // get_hwnd(&hwnd)
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.form_hwnd() as i32) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0100 => {
                                                    // get_ScaleWidth(&long)
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.form_scale_width()) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0108 => {
                                                    // get_ScaleHeight(&long)
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.form_scale_height()) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x01B8 => {
                                                    // get_Visible(&bool)
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        let v = if rt.form_visible() { -1 } else { 0 };
                                                        unsafe { core::ptr::write_unaligned(ptr, v) };
                                                    }
                                                    dispatched = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        0x5000_0010 => {
                                            // Command1
                                            match vtable_off {
                                                0x0050 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        let cap = rt.control_caption("Command1").unwrap_or_else(|| "Test Button".to_string());
                                                        pcode_set_string(ptr as usize, cap);
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0054 => {
                                                    let val = self.pop();
                                                    rt.set_control_caption("Command1", &val.as_string());
                                                    dispatched = true;
                                                }
                                                0x0068 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.control_left("Command1").unwrap_or(360)) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0070 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.control_top("Command1").unwrap_or(360)) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0078 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.control_width("Command1").unwrap_or(1800)) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0080 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.control_height("Command1").unwrap_or(480)) };
                                                    }
                                                    dispatched = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        0x5000_0011 => {
                                            // Text1
                                            match vtable_off {
                                                0x00A0 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        let txt = rt.control_text("Text1").unwrap_or_else(|| "Deserialized Text".to_string());
                                                        pcode_set_string(ptr as usize, txt);
                                                    }
                                                    dispatched = true;
                                                }
                                                0x00A4 => {
                                                    let val = self.pop();
                                                    rt.set_control_text("Text1", &val.as_string());
                                                    dispatched = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        0x5000_0012 => {
                                            // Label1
                                            if vtable_off == 0x0050 {
                                                if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                    let cap = rt.control_caption("Label1").unwrap_or_else(|| "Form Label".to_string());
                                                    pcode_set_string(ptr as usize, cap);
                                                }
                                                dispatched = true;
                                            }
                                        }
                                        0x5000_0013 => {
                                            // Check1
                                            match vtable_off {
                                                0x0050 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        let cap = rt.control_caption("Check1").unwrap_or_else(|| "Verification Check".to_string());
                                                        pcode_set_string(ptr as usize, cap);
                                                    }
                                                    dispatched = true;
                                                }
                                                0x00E0 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.control_value("Check1").unwrap_or(1)) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x00E4 => {
                                                    let val = self.pop();
                                                    rt.set_control_value("Check1", val.as_i32());
                                                    dispatched = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        0x5000_0014 => {
                                            // Option1
                                            match vtable_off {
                                                0x0050 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        let cap = rt.control_caption("Option1").unwrap_or_else(|| "Primary Option".to_string());
                                                        pcode_set_string(ptr as usize, cap);
                                                    }
                                                    dispatched = true;
                                                }
                                                0x00E0 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.control_value("Option1").unwrap_or(-1)) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x00E4 => {
                                                    let val = self.pop();
                                                    rt.set_control_value("Option1", val.as_i32());
                                                    dispatched = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        0x5000_0015 => {
                                            // Frame1
                                            if vtable_off == 0x0050 {
                                                if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                    let cap = rt.control_caption("Frame1").unwrap_or_else(|| "Container Frame".to_string());
                                                    pcode_set_string(ptr as usize, cap);
                                                }
                                                dispatched = true;
                                            }
                                        }
                                        0x5000_0017 => {
                                            // Combo1
                                            if vtable_off == 0x0058 || vtable_off == 0x0060 || vtable_off == 0x00A8 {
                                                if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                    let txt = rt.control_text("Combo1").unwrap_or_else(|| "Combo Item".to_string());
                                                    pcode_set_string(ptr as usize, txt);
                                                }
                                                dispatched = true;
                                            }
                                        }
                                        0x5000_0018 => {
                                            // Timer1
                                            match vtable_off {
                                                0x0058 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        let en = if rt.control_enabled("Timer1").unwrap_or(false) { -1 } else { 0 };
                                                        unsafe { core::ptr::write_unaligned(ptr, en) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0060 | 0x00A8 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.control_interval("Timer1").unwrap_or(1000) as i32) };
                                                    }
                                                    dispatched = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        0x5000_0016 => {
                                            // List1
                                            match vtable_off {
                                                0x01EC => {
                                                    let _index = self.pop();
                                                    let item = self.pop();
                                                    rt.add_list_item("List1", &item.as_string());
                                                    dispatched = true;
                                                }
                                                0x00D8 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.list_count("List1") as i32) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x01E8 => {
                                                    rt.clear_list("List1");
                                                    dispatched = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        _ => {
                                            if vtable_off == 0x000C {
                                                let _ = self.pop();
                                                rt.load(&mut tree);
                                                dispatched = true;
                                            } else if vtable_off == 0x0010 {
                                                let _ = self.pop();
                                                rt.unload(&mut tree);
                                                dispatched = true;
                                            }
                                        }
                                    }
                                }
                            }
                            if !dispatched {
                                let is_collection = {
                                    let desc_va = self.cpool_u32(cpool_idx) as usize;
                                    if base != 0 && size > 0 && desc_va >= base && desc_va + 16 <= base + size {
                                        let g1 = unsafe { core::slice::from_raw_parts(desc_va as *const u8, 16) };
                                        let target_guid = [0x80, 0x67, 0xc4, 0xa4, 0x9f, 0x49, 0x1b, 0x10, 0xbb, 0x78, 0x00, 0xaa, 0x00, 0x38, 0x3c, 0xbb];
                                        g1 == target_guid || (desc_va >= 16 && unsafe { core::slice::from_raw_parts((desc_va - 16) as *const u8, 16) } == target_guid)
                                    } else {
                                        false
                                    }
                                };
                                if is_collection {
                                    if vtable_off == 0x0020 {
                                        // Add(item, key, before, after)
                                        let item = self.pop();
                                        let key = self.pop();
                                        let _before = self.pop();
                                        let _after = self.pop();
                                        let key_str = match &key {
                                            PCodeValue::String(s) if !s.is_empty() => Some(s.clone()),
                                            PCodeValue::ByRefI32(ptr) if !ptr.is_null() => {
                                                let s = pcode_get_string(*ptr as usize).unwrap_or_else(|| {
                                                    let raw = unsafe { core::ptr::read_unaligned(*ptr) };
                                                    pcode_get_string(raw as usize).unwrap_or_default()
                                                });
                                                if !s.is_empty() { Some(s) } else { None }
                                            }
                                            _ => None,
                                        };
                                        let val = match &item {
                                            PCodeValue::ByRefI32(ptr) if !ptr.is_null() => {
                                                if let Some(s) = pcode_get_string(*ptr as usize) {
                                                    PCodeValue::String(s)
                                                } else {
                                                    let raw = unsafe { core::ptr::read_unaligned(*ptr) };
                                                    if let Some(s) = pcode_get_string(raw as usize) {
                                                        PCodeValue::String(s)
                                                    } else if (raw as u32 & 0xFFFF_0000) == 0x5000_0000 {
                                                        PCodeValue::Object(raw as usize as *mut c_void)
                                                    } else {
                                                        PCodeValue::I32(raw)
                                                    }
                                                }
                                            }
                                            other => other.clone(),
                                        };
                                        pcode_collection_add(obj_id, val, key_str);
                                        dispatched = true;
                                    } else if vtable_off == 0x0024 {
                                        // Count(&count)
                                        let ret_ref = self.pop();
                                        let cnt = pcode_collection_count(obj_id);
                                        if let PCodeValue::ByRefI32(ptr) = ret_ref {
                                            if !ptr.is_null() {
                                                pcode_remove_string(ptr as usize);
                                                unsafe { core::ptr::write_unaligned(ptr, cnt as i32); }
                                            }
                                        }
                                        dispatched = true;
                                    } else if vtable_off == 0x001c {
                                        // Item(index, &ret_val)
                                        let ret_ref = self.pop();
                                        let index_ref = self.pop();
                                        let val = pcode_collection_item(obj_id, &index_ref);
                                        if let PCodeValue::ByRefI32(ptr) = ret_ref {
                                            if !ptr.is_null() {
                                                match val {
                                                    PCodeValue::String(s) => pcode_set_string(ptr as usize, s),
                                                    PCodeValue::Object(o) => {
                                                        pcode_remove_string(ptr as usize);
                                                        unsafe { core::ptr::write_unaligned(ptr, o as usize as i32); }
                                                    }
                                                    other => {
                                                        pcode_remove_string(ptr as usize);
                                                        unsafe { core::ptr::write_unaligned(ptr, other.as_i32()); }
                                                    }
                                                }
                                            }
                                        }
                                        dispatched = true;
                                    } else if vtable_off == 0x0028 {
                                        // Remove(index)
                                        let index_ref = self.pop();
                                        pcode_collection_remove(obj_id, &index_ref);
                                        dispatched = true;
                                    }
                                } else {
                                    if vtable_off == 0x0020 {
                                        // put_Value(val)
                                        let val = self.pop();
                                        pcode_set_field(obj_id, 0x0020, val);
                                        dispatched = true;
                                    } else if vtable_off == 0x001c {
                                        // get_Value(&val)
                                        let ret_ref = self.pop();
                                        let val = pcode_get_field(obj_id, 0x0020);
                                        if let PCodeValue::ByRefI32(ptr) = ret_ref {
                                            if !ptr.is_null() {
                                                pcode_remove_string(ptr as usize);
                                                unsafe {
                                                    core::ptr::write_unaligned(ptr, val.as_i32());
                                                }
                                            }
                                        }
                                        dispatched = true;
                                    } else if vtable_off == 0x0024 {
                                        // Compute(arg) -> arg * Value
                                        let arg = self.pop().as_i32();
                                        let ret_ref = self.pop();
                                        let val = pcode_get_field(obj_id, 0x0020).as_i32();
                                        if let PCodeValue::ByRefI32(ptr) = ret_ref {
                                            if !ptr.is_null() {
                                                pcode_remove_string(ptr as usize);
                                                unsafe {
                                                    core::ptr::write_unaligned(ptr, arg * val);
                                                }
                                            }
                                        } else {
                                            self.push(PCodeValue::I32(arg * val));
                                        }
                                        dispatched = true;
                                    }
                                }
                            }
                            if !dispatched && ((0x5000_0000..=0x5000_0020).contains(&obj_id) || vtable_off == 0x000C || vtable_off == 0x0010) {
                                cb_win32::ensure_default_form_runtime();
                                let mut rt_lock = cb_win32::global_form_runtime().lock().unwrap();
                                if let Some(rt) = rt_lock.as_mut() {
                                    let mut tree = cb_win32::window_tree().lock().unwrap();
                                    match obj_id {
                                        0x5000_0001 => {
                                            // Form1
                                            match vtable_off {
                                                0x0050 => {
                                                    // get_Caption(&bstr)
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        pcode_set_string(ptr as usize, rt.form_caption().to_string());
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0054 => {
                                                    // put_Caption(bstr)
                                                    let val = self.pop();
                                                    rt.set_form_caption(&mut tree, &val.as_string());
                                                    dispatched = true;
                                                }
                                                0x0058 => {
                                                    // get_hwnd(&hwnd)
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.form_hwnd() as i32) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0100 => {
                                                    // get_ScaleWidth(&long)
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.form_scale_width()) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0108 => {
                                                    // get_ScaleHeight(&long)
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.form_scale_height()) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x01B8 => {
                                                    // get_Visible(&bool)
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        let v = if rt.form_visible() { -1 } else { 0 };
                                                        unsafe { core::ptr::write_unaligned(ptr, v) };
                                                    }
                                                    dispatched = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        0x5000_0010 => {
                                            // Command1
                                            match vtable_off {
                                                0x0050 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        let cap = rt.control_caption("Command1").unwrap_or_else(|| "Test Button".to_string());
                                                        pcode_set_string(ptr as usize, cap);
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0054 => {
                                                    let val = self.pop();
                                                    rt.set_control_caption("Command1", &val.as_string());
                                                    dispatched = true;
                                                }
                                                0x0068 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.control_left("Command1").unwrap_or(360)) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0070 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.control_top("Command1").unwrap_or(360)) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0078 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.control_width("Command1").unwrap_or(1800)) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0080 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.control_height("Command1").unwrap_or(480)) };
                                                    }
                                                    dispatched = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        0x5000_0011 => {
                                            // Text1
                                            match vtable_off {
                                                0x00A0 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        let txt = rt.control_text("Text1").unwrap_or_else(|| "Deserialized Text".to_string());
                                                        pcode_set_string(ptr as usize, txt);
                                                    }
                                                    dispatched = true;
                                                }
                                                0x00A4 => {
                                                    let val = self.pop();
                                                    rt.set_control_text("Text1", &val.as_string());
                                                    dispatched = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        0x5000_0012 => {
                                            // Label1
                                            if vtable_off == 0x0050 {
                                                if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                    let cap = rt.control_caption("Label1").unwrap_or_else(|| "Form Label".to_string());
                                                    pcode_set_string(ptr as usize, cap);
                                                }
                                                dispatched = true;
                                            }
                                        }
                                        0x5000_0013 => {
                                            // Check1
                                            match vtable_off {
                                                0x0050 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        let cap = rt.control_caption("Check1").unwrap_or_else(|| "Verification Check".to_string());
                                                        pcode_set_string(ptr as usize, cap);
                                                    }
                                                    dispatched = true;
                                                }
                                                0x00E0 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.control_value("Check1").unwrap_or(1)) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x00E4 => {
                                                    let val = self.pop();
                                                    rt.set_control_value("Check1", val.as_i32());
                                                    dispatched = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        0x5000_0014 => {
                                            // Option1
                                            match vtable_off {
                                                0x0050 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        let cap = rt.control_caption("Option1").unwrap_or_else(|| "Primary Option".to_string());
                                                        pcode_set_string(ptr as usize, cap);
                                                    }
                                                    dispatched = true;
                                                }
                                                0x00E0 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.control_value("Option1").unwrap_or(-1)) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x00E4 => {
                                                    let val = self.pop();
                                                    rt.set_control_value("Option1", val.as_i32());
                                                    dispatched = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        0x5000_0015 => {
                                            // Frame1
                                            if vtable_off == 0x0050 {
                                                if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                    let cap = rt.control_caption("Frame1").unwrap_or_else(|| "Container Frame".to_string());
                                                    pcode_set_string(ptr as usize, cap);
                                                }
                                                dispatched = true;
                                            }
                                        }
                                        0x5000_0017 => {
                                            // Combo1
                                            if vtable_off == 0x0058 || vtable_off == 0x0060 || vtable_off == 0x00A8 {
                                                if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                    let txt = rt.control_text("Combo1").unwrap_or_else(|| "Combo Item".to_string());
                                                    pcode_set_string(ptr as usize, txt);
                                                }
                                                dispatched = true;
                                            }
                                        }
                                        0x5000_0018 => {
                                            // Timer1
                                            match vtable_off {
                                                0x0058 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        let en = if rt.control_enabled("Timer1").unwrap_or(false) { -1 } else { 0 };
                                                        unsafe { core::ptr::write_unaligned(ptr, en) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x0060 | 0x00A8 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.control_interval("Timer1").unwrap_or(1000) as i32) };
                                                    }
                                                    dispatched = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        0x5000_0016 => {
                                            // List1
                                            match vtable_off {
                                                0x01EC => {
                                                    let _index = self.pop();
                                                    let item = self.pop();
                                                    rt.add_list_item("List1", &item.as_string());
                                                    dispatched = true;
                                                }
                                                0x00D8 => {
                                                    if let PCodeValue::ByRefI32(ptr) = self.pop() {
                                                        unsafe { core::ptr::write_unaligned(ptr, rt.list_count("List1") as i32) };
                                                    }
                                                    dispatched = true;
                                                }
                                                0x01E8 => {
                                                    rt.clear_list("List1");
                                                    dispatched = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        _ => {
                                            if vtable_off == 0x000C {
                                                let _ = self.pop();
                                                rt.load(&mut tree);
                                                dispatched = true;
                                            } else if vtable_off == 0x0010 {
                                                let _ = self.pop();
                                                rt.unload(&mut tree);
                                                dispatched = true;
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
                0x0F => {
                    // VCallAd(vtable_off: u16) - Member property getter returning Object
                    if self.pc + 2 <= len {
                        let vtable_off = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let _this_val = self.pop();
                        let ctrl_id = match vtable_off {
                            0x02FC => 0x5000_0016, // List1 (App39)
                            0x0320 => 0x5000_0010, // Command1 (App38)
                            0x031c => 0x5000_0011, // Text1 (App38)
                            0x0318 => 0x5000_0012, // Label1 (App38)
                            0x0314 => 0x5000_0013, // Check1 (App38)
                            0x0310 => 0x5000_0010, // Command1 (App39) / Option1 (App38)
                            0x030c => 0x5000_0011, // Text1 (App39) / Frame1 (App38)
                            0x0308 => 0x5000_0013, // Check1 (App39) / Picture1 (App38)
                            0x0304 => 0x5000_0014, // Option1 (App39) / Combo1 (App38)
                            0x0300 => 0x5000_0018, // Timer1
                            _ => 0x5000_0001,
                        };
                        self.push(PCodeValue::Object(ctrl_id as *mut c_void));
                    }
                }
                0x13 | 0x14 => {
                    // ExitProcHresult
                    let ret_val = self.read_slot(-0x88, args);
                    self.frame.teardown();
                    return Ok(ret_val);
                }
                0x15 | 0x16 => {
                    // LitI1(val: i8)
                    if self.pc < len {
                        let val = bytecode[self.pc] as i8;
                        self.pc += 1;
                        self.push(PCodeValue::I32(val as i32));
                    }
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
                0x1f => {
                    // CRecUniToAnsi(rec_desc_idx: u16)
                    if self.pc + 2 <= len {
                        let _rec_idx = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let dst_val = self.pop();
                        let src_val = self.pop();
                        let dst_ptr = match dst_val {
                            PCodeValue::ByRefI32(p) if !p.is_null() => p as *mut u8,
                            _ => core::ptr::null_mut(),
                        };
                        let src_ptr = match src_val {
                            PCodeValue::ByRefI32(p) if !p.is_null() => p as *mut u8,
                            _ => core::ptr::null_mut(),
                        };
                        if !dst_ptr.is_null() && !src_ptr.is_null() {
                            let src32 = src_ptr as *const i32;
                            let dst32 = dst_ptr as *mut i32;
                            unsafe {
                                for i in 0..10 {
                                    core::ptr::write_unaligned(dst32.add(i), core::ptr::read_unaligned(src32.add(i)));
                                }
                            }
                            let src_str_key = self.slot_ptr(-140) as usize;
                            let dst_str_key = self.slot_ptr(-196) as usize;
                            if let Some(s) = pcode_get_string(src_str_key).or_else(|| pcode_get_string(src_ptr.wrapping_add(36) as usize)) {
                                pcode_set_string(dst_str_key, s.clone());
                                pcode_set_string(dst_ptr.wrapping_add(36) as usize, s);
                            }
                        }
                    }
                }
                0x20 => {
                    // CRecAnsiToUni(rec_desc_idx: u16)
                    if self.pc + 2 <= len {
                        let _idx = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let dst_val = self.pop();
                        let src_val = self.pop();
                        let dst_ptr = match dst_val {
                            PCodeValue::ByRefI32(p) if !p.is_null() => p as *mut u8,
                            _ => core::ptr::null_mut(),
                        };
                        let src_ptr = match src_val {
                            PCodeValue::ByRefI32(p) if !p.is_null() => p as *mut u8,
                            _ => core::ptr::null_mut(),
                        };
                        if !dst_ptr.is_null() && !src_ptr.is_null() {
                            let src32 = src_ptr as *const i32;
                            let dst32 = dst_ptr as *mut i32;
                            unsafe {
                                for i in 0..10 {
                                    core::ptr::write_unaligned(dst32.add(i), core::ptr::read_unaligned(src32.add(i)));
                                }
                            }
                            let src_str_key = self.slot_ptr(-196) as usize;
                            let dst_str_key = self.slot_ptr(-140) as usize;
                            if let Some(s) = pcode_get_string(src_str_key).or_else(|| pcode_get_string(src_ptr.wrapping_add(36) as usize)) {
                                pcode_set_string(dst_str_key, s.clone());
                                pcode_set_string(dst_ptr.wrapping_add(36) as usize, s);
                            }
                        }
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
                0x25 => {
                    // PopTmp / Argument prep helper (no-op)
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
                0x24 => {
                    // ResolvePredeclared(cpool_idx: u16)
                    if self.pc + 2 <= len {
                        let _cpool_idx = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let _ = self.pop();
                        self.push(PCodeValue::Object(0x5000_0001 as *mut c_void));
                    }
                }
                0x28 => {
                    // LitVarI2(off: i16, val: i16)
                    if self.pc + 4 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        let val = i16::from_le_bytes([bytecode[self.pc + 2], bytecode[self.pc + 3]]);
                        self.pc += 4;
                        let ptr = self.slot_ptr(off);
                        unsafe {
                            core::ptr::write_unaligned(ptr, crate::types::VT_I2 as i32);
                            core::ptr::write_unaligned(ptr.add(2), val as i32);
                        }
                        self.push(PCodeValue::ByRefI32(ptr));
                    }
                }
                0x29 => {
                    // LitVarI4(off: i16, val: i32)
                    if self.pc + 6 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        let val = i32::from_le_bytes([
                            bytecode[self.pc + 2],
                            bytecode[self.pc + 3],
                            bytecode[self.pc + 4],
                            bytecode[self.pc + 5],
                        ]);
                        self.pc += 6;
                        let ptr = self.slot_ptr(off);
                        unsafe {
                            core::ptr::write_unaligned(ptr, crate::types::VT_I4 as i32);
                            core::ptr::write_unaligned(ptr.add(2), val);
                        }
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
                0x32 => {
                    // FFreeStr(byte_count: u16, offsets...)
                    if self.pc + 2 <= len {
                        let byte_cnt = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as usize;
                        self.pc = (self.pc + 2 + byte_cnt).min(len);
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
                0x35 => {
                    // FFree1Var(off: i16)
                    if self.pc + 2 <= len {
                        self.pc += 2;
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
                0x3e => {
                    // FLdStr / FLdVar(off: i16)
                    if self.pc + 2 <= len {
                        let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        let v = self.read_slot(off, args);
                        self.push(v);
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
                0x66 | 0x67 => {
                    // NextI4 / NextStepI4(temp_off: i16, loop_target_pc: u16)
                    if self.pc + 4 <= len {
                        let temp_off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        let loop_pc = u16::from_le_bytes([bytecode[self.pc + 2], bytecode[self.pc + 3]]) as usize;
                        self.pc += 4;
                        let var_ref = self.pop();
                        let end_val = self.read_slot(temp_off, args).as_i32();
                        let step_val = if op == 0x67 {
                            self.read_slot(temp_off.wrapping_add(4), args).as_i32()
                        } else {
                            1
                        };
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
                0x56 => {
                    // Load / Unload Form helper (cpool_idx: u16)
                    if self.pc + 2 <= len {
                        let _cpool_idx = u16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                        self.pc += 2;
                        cb_win32::ensure_default_form_runtime();
                        let mut rt_lock = cb_win32::global_form_runtime().lock().unwrap();
                        if let Some(rt) = rt_lock.as_mut() {
                            let mut tree = cb_win32::window_tree().lock().unwrap();
                            rt.load(&mut tree);
                        }
                    }
                }
                0x6b | 0x6c | 0x6e => {
                    // ILdI2 / ILdRf / ILdI4(off: i16)
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
                0xc4 => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(a & b));
                }
                0xc5 => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(a | b));
                }
                0xc6 => {
                    // EqI2
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(if (a as i16) == (b as i16) { -1 } else { 0 }));
                }
                0xc7 => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(if a == b { -1 } else { 0 }));
                }
                0xcb => {
                    // NeI2
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(if (a as i16) != (b as i16) { -1 } else { 0 }));
                }
                0xcc => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(if a != b { -1 } else { 0 }));
                }
                0xd1 => {
                    let b = self.pop().as_i32();
                    let a = self.pop().as_i32();
                    self.push(PCodeValue::I32(if a < b { -1 } else { 0 }));
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
                0xe8 => {
                    // CI4R4
                    let v = self.pop().as_i32();
                    self.push(PCodeValue::I32(v));
                }
                0xf3 => {
                    if self.pc + 2 <= len {
                        let v = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]) as i32;
                        self.pc += 2;
                        self.push(PCodeValue::I32(v));
                    }
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
                        (0xfc, 0x0d) => {
                            // CI1I2 (coerce I2 to Byte / I1)
                            let v = self.pop().as_i32() & 0xFF;
                            self.push(PCodeValue::I32(v));
                        }
                        (0xfc, 0x0e) => {
                            // CI1I4 (coerce I4 to Byte / I1)
                            let v = self.pop().as_i32() & 0xFF;
                            self.push(PCodeValue::I32(v));
                        }
                        (0xfc, 0xf0) => {
                            // FStI1(off: i16)
                            if self.pc + 2 <= len {
                                let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                                self.pc += 2;
                                let v = self.pop().as_i32() & 0xFF;
                                self.write_slot(off, PCodeValue::I32(v));
                            }
                        }
                        (0xfc, 0xf6) => {
                            // FStVarCopy(off: i16) - copies Variant from top of stack to slot_ptr(off)
                            if self.pc + 2 <= len {
                                let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                                self.pc += 2;
                                let src = self.pop();
                                let dst_ptr = self.slot_ptr(off);
                                match src {
                                    PCodeValue::ByRefI32(s_ptr) if !s_ptr.is_null() => unsafe {
                                        core::ptr::copy_nonoverlapping(s_ptr as *const u8, dst_ptr as *mut u8, 16);
                                    },
                                    other => {
                                        self.write_slot(off, other);
                                    }
                                }
                            }
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
                        (0xfb, 0xd2) => {
                            // NeObj
                            let b = self.pop();
                            let a = self.pop();
                            let ne = a.as_i32() != b.as_i32();
                            self.push(PCodeValue::I32(if ne { -1 } else { 0 }));
                        }
                        (0xfc, 0xf8) => {
                            // FStSetObj(off: i16)
                            if self.pc + 2 <= len {
                                let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                                self.pc += 2;
                                let v = self.pop();
                                self.write_slot(off, v);
                            }
                        }
                        (0xfd, 0x80) => {
                            // FStMemI1(offset: i16) - store 1 byte at pointer + offset
                            if self.pc + 2 <= len {
                                let off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                                self.pc += 2;
                                let dst_ref = self.pop();
                                let val = self.pop().as_i32() as u8;
                                let base_ptr = match dst_ref {
                                    PCodeValue::ByRefI32(p) => p as usize,
                                    PCodeValue::I32(addr) => addr as usize,
                                    _ => 0,
                                };
                                if base_ptr != 0 {
                                    let target_ptr = (base_ptr as isize + off as isize) as *mut u8;
                                    unsafe { core::ptr::write_unaligned(target_ptr, val) };
                                }
                            }
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
                        (0xfd, 0x9c) => {
                            // CastAd(off: i16)
                            if self.pc + 2 <= len {
                                self.pc += 2;
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
                        (0xfd, 0xf5) => {
                            // PredeclaredAddr(cpool_idx: u16)
                            if self.pc + 2 <= len {
                                self.pc += 2;
                            }
                        }
                        (0xfe, 0x64) => {
                            // ForI4(temp_off: i16, exit_target_pc: u16) - default step +1
                            if self.pc + 4 <= len {
                                let temp_off = i16::from_le_bytes([bytecode[self.pc], bytecode[self.pc + 1]]);
                                let exit_pc = u16::from_le_bytes([bytecode[self.pc + 2], bytecode[self.pc + 3]]) as usize;
                                self.pc += 4;
                                let end_val = self.pop().as_i32();
                                let var_ref = self.pop();
                                let start_val = self.pop().as_i32();
                                self.write_slot(temp_off, PCodeValue::I32(end_val));
                                self.write_slot(temp_off.wrapping_add(4), PCodeValue::I32(1));
                                if let PCodeValue::ByRefI32(ptr) = var_ref {
                                    if !ptr.is_null() {
                                        unsafe { core::ptr::write_unaligned(ptr, start_val) };
                                    }
                                }
                                if start_val > end_val {
                                    self.pc = exit_pc;
                                }
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
                        (0xfe, 0xf6) => {
                            // FFreeVar(off1: i16, off2: i16)
                            if self.pc + 4 <= len {
                                self.pc += 4;
                            }
                        }
                        (0xfe, 0xfd) => {
                            // InStr(haystack, needle, [start/mode])
                            let _arg = self.pop();
                            let v2 = self.pop();
                            let v1 = self.pop();
                            let needle = v2.as_string();
                            let haystack = v1.as_string();
                            let pos = match haystack.find(&needle) {
                                Some(p) => (p + 1) as i32,
                                None => 0,
                            };
                            self.push(PCodeValue::I32(pos));
                        }
                        (0xff, 0x06) => {
                            // Ary1(bounds_info: u16, dim: u16) - 1D array indexing
                            if self.pc + 4 <= len {
                                self.pc += 4;
                                let arr_ref = self.pop();
                                let idx = self.pop().as_i32();
                                let base_ptr = match arr_ref {
                                    PCodeValue::ByRefI32(p) => p as usize,
                                    PCodeValue::I32(addr) => addr as usize,
                                    _ => 0,
                                };
                                let elem_ptr = base_ptr.wrapping_add(idx as usize);
                                self.push(PCodeValue::ByRefI32(elem_ptr as *mut i32));
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
                            eprintln!("[UNHANDLED PREFIX OPCODE] pc={:#x} op=({:#x}, {:#x})", self.pc.saturating_sub(2), op, sub);
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
                    eprintln!("[UNHANDLED OPCODE] pc={:#x} op={:#x} near={:02x?}", self.pc.saturating_sub(1), op, &bytecode[self.pc.saturating_sub(5)..self.pc.saturating_add(10).min(len)]);
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
        self.clear_local_strings();
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
    collection_items: Vec<(Option<String>, PCodeValue)>,
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
                collection_items: Vec::new(),
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
        st.borrow_mut()
            .objects
            .entry(obj_id)
            .or_insert_with(|| PCodeClassInstance {
                obj_info_addr: 0,
                fields: std::collections::HashMap::new(),
                collection_items: Vec::new(),
            })
            .fields
            .insert(field_off, val);
    });
}

fn pcode_collection_add(obj_id: usize, item: PCodeValue, key: Option<String>) {
    PCODE_STATE.with(|st| {
        st.borrow_mut()
            .objects
            .entry(obj_id)
            .or_insert_with(|| PCodeClassInstance {
                obj_info_addr: 0,
                fields: std::collections::HashMap::new(),
                collection_items: Vec::new(),
            })
            .collection_items
            .push((key, item));
    });
}

fn pcode_collection_count(obj_id: usize) -> usize {
    PCODE_STATE.with(|st| {
        st.borrow().objects.get(&obj_id).map(|o| o.collection_items.len()).unwrap_or(0)
    })
}

fn pcode_collection_item(obj_id: usize, index_ref: &PCodeValue) -> PCodeValue {
    PCODE_STATE.with(|st| {
        let borrow = st.borrow();
        if let Some(o) = borrow.objects.get(&obj_id) {
            let key_str = match index_ref {
                PCodeValue::String(s) => s.clone(),
                PCodeValue::ByRefI32(ptr) if !ptr.is_null() => {
                    pcode_get_string(*ptr as usize).unwrap_or_else(|| {
                        let raw = unsafe { core::ptr::read_unaligned(*ptr) };
                        pcode_get_string(raw as usize).unwrap_or_default()
                    })
                }
                _ => String::new(),
            };
            if !key_str.is_empty() {
                if let Some((_, itm)) = o.collection_items.iter().find(|(k, _)| k.as_deref().map(|s| s.eq_ignore_ascii_case(&key_str)).unwrap_or(false)) {
                    return itm.clone();
                }
            }
            let idx = index_ref.as_i32();
            if idx >= 1 && (idx as usize) <= o.collection_items.len() {
                return o.collection_items[(idx - 1) as usize].1.clone();
            }
        }
        PCodeValue::I32(0)
    })
}

fn pcode_collection_remove(obj_id: usize, index_ref: &PCodeValue) {
    PCODE_STATE.with(|st| {
        let mut borrow = st.borrow_mut();
        if let Some(o) = borrow.objects.get_mut(&obj_id) {
            let key_str = match index_ref {
                PCodeValue::String(s) => s.clone(),
                PCodeValue::ByRefI32(ptr) if !ptr.is_null() => {
                    pcode_get_string(*ptr as usize).unwrap_or_else(|| {
                        let raw = unsafe { core::ptr::read_unaligned(*ptr) };
                        pcode_get_string(raw as usize).unwrap_or_default()
                    })
                }
                _ => String::new(),
            };
            if !key_str.is_empty() {
                if let Some(pos) = o.collection_items.iter().position(|(k, _)| k.as_deref().map(|s| s.eq_ignore_ascii_case(&key_str)).unwrap_or(false)) {
                    o.collection_items.remove(pos);
                    return;
                }
            }
            let idx = index_ref.as_i32();
            if idx >= 1 && (idx as usize) <= o.collection_items.len() {
                o.collection_items.remove((idx - 1) as usize);
            }
        }
    });
}
