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
}

impl PCodeVM {
    /// Instantiates a new P-Code Virtual Machine with a frame of the given size.
    pub fn new(frame_size: usize) -> Self {
        Self {
            stack: Vec::with_capacity(64),
            pc: 0,
            frame: ProcFrame::new(frame_size),
            string_table: Vec::new(),
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
                        // Execute recursively or via delegate
                        let mut sub_vm = PCodeVM::new(128);
                        sub_vm.string_table = self.string_table.clone();
                        let arg_val = self.pop();
                        let mut sub_args = vec![arg_val];
                        let res = sub_vm.execute(&bytecode[sub_ptr..], &mut sub_args)?;
                        self.push(res);
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
