// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! 32-bit x86 CPU Execution Engine & Instruction Interpreter.

use super::flags::*;
use super::fpu::*;
use super::memory::*;
use super::registers::*;
use std::collections::HashMap;

/// Result of an instruction step or execution run.
#[derive(Debug, PartialEq, Eq)]
pub enum X86Error {
    InvalidOpcode(u8, u32),
    StackUnderflow,
    StackOverflow,
    DivisionByZero,
    MemoryFault(u32),
    UnhandledInterrupt(u8),
    Halted(i32),
}

/// Decoded ModR/M operand: either a CPU register or an effective memory address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    Reg(u8),
    Mem(u32),
}

/// Native hook handler signature for intercepting Win32 API and MSVBVM60 exports.
pub type HookCallback = Box<dyn Fn(&mut X86Cpu) -> Result<(), X86Error>>;

/// Clean-room 32-bit x86 CPU emulator.
pub struct X86Cpu {
    pub regs: Registers,
    pub flags: Flags,
    pub fpu: X87Fpu,
    pub mem: X86Memory,
    pub hooks: HashMap<u32, HookCallback>,
    pub is_halted: bool,
    pub exit_code: i32,
    pub instruction_count: u64,
}

impl Default for X86Cpu {
    fn default() -> Self {
        Self {
            regs: Registers::default(),
            flags: Flags::default(),
            fpu: X87Fpu::default(),
            mem: X86Memory::default(),
            hooks: HashMap::new(),
            is_halted: false,
            exit_code: 0,
            instruction_count: 0,
        }
    }
}

impl X86Cpu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a native host hook at the specified 32-bit address.
    pub fn register_hook<F>(&mut self, addr: u32, callback: F)
    where
        F: Fn(&mut X86Cpu) -> Result<(), X86Error> + 'static,
    {
        self.hooks.insert(addr, Box::new(callback));
    }

    /// Pushes a 32-bit value onto the stack: `esp -= 4; [esp] = val`.
    pub fn push32(&mut self, val: u32) {
        self.regs.esp = self.regs.esp.wrapping_sub(4);
        self.mem.write_u32(self.regs.esp, val);
    }

    /// Pops a 32-bit value from the stack: `val = [esp]; esp += 4`.
    pub fn pop32(&mut self) -> u32 {
        let val = self.mem.read_u32(self.regs.esp);
        self.regs.esp = self.regs.esp.wrapping_add(4);
        val
    }

    /// Reads immediate byte from code at `EIP` and advances `EIP`.
    pub fn fetch_u8(&mut self) -> u8 {
        let byte = self.mem.read_u8(self.regs.eip);
        self.regs.eip = self.regs.eip.wrapping_add(1);
        byte
    }

    /// Reads immediate 16-bit word from code at `EIP` and advances `EIP`.
    pub fn fetch_u16(&mut self) -> u16 {
        let val = self.mem.read_u16(self.regs.eip);
        self.regs.eip = self.regs.eip.wrapping_add(2);
        val
    }

    /// Reads immediate 32-bit dword from code at `EIP` and advances `EIP`.
    pub fn fetch_u32(&mut self) -> u32 {
        let val = self.mem.read_u32(self.regs.eip);
        self.regs.eip = self.regs.eip.wrapping_add(4);
        val
    }

    /// Decodes ModR/M and optional SIB byte into `(reg_index, operand)`.
    pub fn decode_modrm(&mut self, is_fs_prefix: bool) -> (u8, Operand) {
        let modrm = self.fetch_u8();
        let mod_bits = (modrm >> 6) & 0x03;
        let reg_bits = (modrm >> 3) & 0x07;
        let rm_bits = modrm & 0x07;

        if mod_bits == 3 {
            return (reg_bits, Operand::Reg(rm_bits));
        }

        let mut base_addr = if rm_bits == 4 {
            // SIB byte follows
            let sib = self.fetch_u8();
            let scale_shift = (sib >> 6) & 0x03;
            let index_reg = (sib >> 3) & 0x07;
            let base_reg = sib & 0x07;

            let mut base = if base_reg == 5 && mod_bits == 0 {
                self.fetch_u32()
            } else {
                self.regs.get_reg32(base_reg)
            };

            if index_reg != 4 {
                let idx_val = self.regs.get_reg32(index_reg);
                base = base.wrapping_add(idx_val << scale_shift);
            }
            base
        } else if rm_bits == 5 && mod_bits == 0 {
            // Displacement-only 32-bit address
            self.fetch_u32()
        } else {
            self.regs.get_reg32(rm_bits)
        };

        if mod_bits == 1 {
            let disp8 = self.fetch_u8() as i8 as i32 as u32;
            base_addr = base_addr.wrapping_add(disp8);
        } else if mod_bits == 2 {
            let disp32 = self.fetch_u32();
            base_addr = base_addr.wrapping_add(disp32);
        }

        if is_fs_prefix {
            base_addr = base_addr.wrapping_add(self.regs.fs_base);
        }

        (reg_bits, Operand::Mem(base_addr))
    }

    /// Reads a 32-bit value from an operand.
    pub fn read_operand32(&self, op: &Operand) -> u32 {
        match *op {
            Operand::Reg(r) => self.regs.get_reg32(r),
            Operand::Mem(addr) => self.mem.read_u32(addr),
        }
    }

    /// Writes a 32-bit value to an operand.
    pub fn write_operand32(&mut self, op: &Operand, val: u32) {
        match *op {
            Operand::Reg(r) => self.regs.set_reg32(r, val),
            Operand::Mem(addr) => self.mem.write_u32(addr, val),
        }
    }

    /// Reads an 8-bit value from an operand.
    pub fn read_operand8(&self, op: &Operand) -> u8 {
        match *op {
            Operand::Reg(r) => self.regs.get_reg8(r),
            Operand::Mem(addr) => self.mem.read_u8(addr),
        }
    }

    /// Writes an 8-bit value to an operand.
    pub fn write_operand8(&mut self, op: &Operand, val: u8) {
        match *op {
            Operand::Reg(r) => self.regs.set_reg8(r, val),
            Operand::Mem(addr) => self.mem.write_u8(addr, val),
        }
    }

    /// Executes a single instruction at `EIP`.
    pub fn step(&mut self) -> Result<(), X86Error> {
        if self.is_halted {
            return Ok(());
        }

        // Check if current EIP is registered as a native runtime hook
        if let Some(callback) = self.hooks.get(&self.regs.eip) {
            let cb = callback as *const HookCallback;
            return unsafe { (*cb)(self) };
        }

        let mut is_fs_prefix = false;
        let mut _is_op_size_override = false;

        let mut opcode = self.fetch_u8();

        // Decode instruction prefixes
        while opcode == 0x64 || opcode == 0x66 || opcode == 0x67 || opcode == 0xF0 || opcode == 0xF2 || opcode == 0xF3 {
            match opcode {
                0x64 => is_fs_prefix = true,
                0x66 => _is_op_size_override = true,
                _ => {}
            }
            opcode = self.fetch_u8();
        }

        match opcode {
            // NOP
            0x90 => {}

            // MOV r32, imm32 (0xB8 .. 0xBF)
            0xB8..=0xBF => {
                let reg = opcode - 0xB8;
                let val = self.fetch_u32();
                self.regs.set_reg32(reg, val);
            }

            // MOV r8, imm8 (0xB0 .. 0xB7)
            0xB0..=0xB7 => {
                let reg = opcode - 0xB0;
                let val = self.fetch_u8();
                self.regs.set_reg8(reg, val);
            }

            // MOV r/m32, r32
            0x89 => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let val = self.regs.get_reg32(reg);
                self.write_operand32(&op, val);
            }

            // MOV r32, r/m32
            0x8B => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let val = self.read_operand32(&op);
                self.regs.set_reg32(reg, val);
            }

            // MOV r/m8, r8
            0x88 => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let val = self.regs.get_reg8(reg);
                self.write_operand8(&op, val);
            }

            // MOV r8, r/m8
            0x8A => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let val = self.read_operand8(&op);
                self.regs.set_reg8(reg, val);
            }

            // MOV r/m32, imm32
            0xC7 => {
                let (_, op) = self.decode_modrm(is_fs_prefix);
                let val = self.fetch_u32();
                self.write_operand32(&op, val);
            }

            // MOV r/m8, imm8
            0xC6 => {
                let (_, op) = self.decode_modrm(is_fs_prefix);
                let val = self.fetch_u8();
                self.write_operand8(&op, val);
            }

            // MOV al, [moffs8]
            0xA0 => {
                let addr = self.fetch_u32();
                let effective = if is_fs_prefix { addr.wrapping_add(self.regs.fs_base) } else { addr };
                let val = self.mem.read_u8(effective);
                self.regs.set_reg8(0, val);
            }

            // MOV eax, [moffs32]
            0xA1 => {
                let addr = self.fetch_u32();
                let effective = if is_fs_prefix { addr.wrapping_add(self.regs.fs_base) } else { addr };
                let val = self.mem.read_u32(effective);
                self.regs.eax = val;
            }

            // MOV [moffs32], eax
            0xA3 => {
                let addr = self.fetch_u32();
                let effective = if is_fs_prefix { addr.wrapping_add(self.regs.fs_base) } else { addr };
                self.mem.write_u32(effective, self.regs.eax);
            }

            // LEA r32, m
            0x8D => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                if let Operand::Mem(addr) = op {
                    self.regs.set_reg32(reg, addr);
                }
            }

            // PUSH r32 (0x50 .. 0x57)
            0x50..=0x57 => {
                let reg = opcode - 0x50;
                let val = self.regs.get_reg32(reg);
                self.push32(val);
            }

            // POP r32 (0x58 .. 0x5F)
            0x58..=0x5F => {
                let reg = opcode - 0x58;
                let val = self.pop32();
                self.regs.set_reg32(reg, val);
            }

            // PUSH imm32
            0x68 => {
                let val = self.fetch_u32();
                self.push32(val);
            }

            // PUSH imm8 (sign-extended to 32 bits)
            0x6A => {
                let val = self.fetch_u8() as i8 as i32 as u32;
                self.push32(val);
            }

            // INC r32 (0x40 .. 0x47)
            0x40..=0x47 => {
                let reg = opcode - 0x40;
                let old = self.regs.get_reg32(reg);
                let res = old.wrapping_add(1);
                self.regs.set_reg32(reg, res);
                self.flags.update_inc_32(old, res);
            }

            // DEC r32 (0x48 .. 0x4F)
            0x48..=0x4F => {
                let reg = opcode - 0x48;
                let old = self.regs.get_reg32(reg);
                let res = old.wrapping_sub(1);
                self.regs.set_reg32(reg, res);
                self.flags.update_dec_32(old, res);
            }

            // ADD r/m32, r32
            0x01 => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let a = self.read_operand32(&op);
                let b = self.regs.get_reg32(reg);
                let res = a.wrapping_add(b);
                self.write_operand32(&op, res);
                self.flags.update_add_32(a, b, res);
            }

            // ADD r32, r/m32
            0x03 => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let a = self.regs.get_reg32(reg);
                let b = self.read_operand32(&op);
                let res = a.wrapping_add(b);
                self.regs.set_reg32(reg, res);
                self.flags.update_add_32(a, b, res);
            }

            // SUB r/m32, r32
            0x29 => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let a = self.read_operand32(&op);
                let b = self.regs.get_reg32(reg);
                let res = a.wrapping_sub(b);
                self.write_operand32(&op, res);
                self.flags.update_sub_32(a, b, res);
            }

            // SUB r32, r/m32
            0x2B => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let a = self.regs.get_reg32(reg);
                let b = self.read_operand32(&op);
                let res = a.wrapping_sub(b);
                self.regs.set_reg32(reg, res);
                self.flags.update_sub_32(a, b, res);
            }

            // CMP r/m32, r32
            0x39 => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let a = self.read_operand32(&op);
                let b = self.regs.get_reg32(reg);
                let res = a.wrapping_sub(b);
                self.flags.update_sub_32(a, b, res);
            }

            // CMP r32, r/m32
            0x3B => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let a = self.regs.get_reg32(reg);
                let b = self.read_operand32(&op);
                let res = a.wrapping_sub(b);
                self.flags.update_sub_32(a, b, res);
            }

            // AND r/m32, r32
            0x21 => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let a = self.read_operand32(&op);
                let b = self.regs.get_reg32(reg);
                let res = a & b;
                self.write_operand32(&op, res);
                self.flags.update_logic_32(res);
            }

            // AND r32, r/m32
            0x23 => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let a = self.regs.get_reg32(reg);
                let b = self.read_operand32(&op);
                let res = a & b;
                self.regs.set_reg32(reg, res);
                self.flags.update_logic_32(res);
            }

            // OR r/m32, r32
            0x09 => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let a = self.read_operand32(&op);
                let b = self.regs.get_reg32(reg);
                let res = a | b;
                self.write_operand32(&op, res);
                self.flags.update_logic_32(res);
            }

            // OR r32, r/m32
            0x0B => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let a = self.regs.get_reg32(reg);
                let b = self.read_operand32(&op);
                let res = a | b;
                self.regs.set_reg32(reg, res);
                self.flags.update_logic_32(res);
            }

            // XOR r/m32, r32
            0x31 => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let a = self.read_operand32(&op);
                let b = self.regs.get_reg32(reg);
                let res = a ^ b;
                self.write_operand32(&op, res);
                self.flags.update_logic_32(res);
            }

            // XOR r32, r/m32
            0x33 => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let a = self.regs.get_reg32(reg);
                let b = self.read_operand32(&op);
                let res = a ^ b;
                self.regs.set_reg32(reg, res);
                self.flags.update_logic_32(res);
            }

            // TEST r/m32, r32
            0x85 => {
                let (reg, op) = self.decode_modrm(is_fs_prefix);
                let a = self.read_operand32(&op);
                let b = self.regs.get_reg32(reg);
                let res = a & b;
                self.flags.update_logic_32(res);
            }

            // Immediate ALU group: 0x81 (r/m32, imm32) and 0x83 (r/m32, imm8 sign-extended)
            0x81 | 0x83 => {
                let (sub_op, op) = self.decode_modrm(is_fs_prefix);
                let imm = if opcode == 0x81 {
                    self.fetch_u32()
                } else {
                    self.fetch_u8() as i8 as i32 as u32
                };
                let a = self.read_operand32(&op);
                match sub_op {
                    0 => { // ADD
                        let res = a.wrapping_add(imm);
                        self.write_operand32(&op, res);
                        self.flags.update_add_32(a, imm, res);
                    }
                    1 => { // OR
                        let res = a | imm;
                        self.write_operand32(&op, res);
                        self.flags.update_logic_32(res);
                    }
                    4 => { // AND
                        let res = a & imm;
                        self.write_operand32(&op, res);
                        self.flags.update_logic_32(res);
                    }
                    5 => { // SUB
                        let res = a.wrapping_sub(imm);
                        self.write_operand32(&op, res);
                        self.flags.update_sub_32(a, imm, res);
                    }
                    6 => { // XOR
                        let res = a ^ imm;
                        self.write_operand32(&op, res);
                        self.flags.update_logic_32(res);
                    }
                    7 => { // CMP
                        let res = a.wrapping_sub(imm);
                        self.flags.update_sub_32(a, imm, res);
                    }
                    _ => return Err(X86Error::InvalidOpcode(opcode, self.regs.eip)),
                }
            }

            // Shifts and Rotates: 0xC1 (r/m32, imm8)
            0xC1 => {
                let (sub_op, op) = self.decode_modrm(is_fs_prefix);
                let count = self.fetch_u8() & 0x1F;
                let val = self.read_operand32(&op);
                let res = match sub_op {
                    4 => val << count,                               // SHL
                    5 => val >> count,                               // SHR
                    7 => ((val as i32) >> count) as u32,             // SAR
                    _ => val,
                };
                self.write_operand32(&op, res);
                self.flags.update_logic_32(res);
            }

            // Unary & Multiply/Divide group: 0xF7
            0xF7 => {
                let (sub_op, op) = self.decode_modrm(is_fs_prefix);
                let val = self.read_operand32(&op);
                match sub_op {
                    0 => { // TEST r/m32, imm32
                        let imm = self.fetch_u32();
                        self.flags.update_logic_32(val & imm);
                    }
                    2 => { // NOT
                        let res = !val;
                        self.write_operand32(&op, res);
                    }
                    3 => { // NEG
                        let res = (0u32).wrapping_sub(val);
                        self.write_operand32(&op, res);
                        self.flags.update_sub_32(0, val, res);
                    }
                    4 => { // MUL EAX * val
                        let res = (self.regs.eax as u64) * (val as u64);
                        self.regs.eax = res as u32;
                        self.regs.edx = (res >> 32) as u32;
                    }
                    5 => { // IMUL EAX * val (signed)
                        let res = (self.regs.eax as i32 as i64) * (val as i32 as i64);
                        self.regs.eax = res as u32;
                        self.regs.edx = (res >> 32) as u32;
                    }
                    6 => { // DIV EDX:EAX / val
                        if val == 0 {
                            return Err(X86Error::DivisionByZero);
                        }
                        let num = ((self.regs.edx as u64) << 32) | (self.regs.eax as u64);
                        self.regs.eax = (num / (val as u64)) as u32;
                        self.regs.edx = (num % (val as u64)) as u32;
                    }
                    7 => { // IDIV EDX:EAX / val (signed)
                        if val == 0 {
                            return Err(X86Error::DivisionByZero);
                        }
                        let num = (((self.regs.edx as u64) << 32) | (self.regs.eax as u64)) as i64;
                        let divisor = val as i32 as i64;
                        self.regs.eax = (num / divisor) as i32 as u32;
                        self.regs.edx = (num % divisor) as i32 as u32;
                    }
                    _ => return Err(X86Error::InvalidOpcode(opcode, self.regs.eip)),
                }
            }

            // CDQ: Convert Doubleword to Quadword (sign-extend EAX into EDX:EAX)
            0x99 => {
                if (self.regs.eax & 0x80000000) != 0 {
                    self.regs.edx = 0xFFFFFFFF;
                } else {
                    self.regs.edx = 0;
                }
            }

            // CWDE: Convert Word to Doubleword Extended (sign-extend AX into EAX)
            0x98 => {
                self.regs.eax = (self.regs.eax as i16 as i32) as u32;
            }

            // JMP rel8
            0xEB => {
                let offset = self.fetch_u8() as i8 as i32;
                self.regs.eip = (self.regs.eip as i32 + offset) as u32;
            }

            // JMP rel32
            0xE9 => {
                let offset = self.fetch_u32() as i32;
                self.regs.eip = (self.regs.eip as i32 + offset) as u32;
            }

            // Jcc rel8 (0x70 .. 0x7F)
            0x70..=0x7F => {
                let cond = opcode & 0x0F;
                let offset = self.fetch_u8() as i8 as i32;
                if self.flags.eval_condition(cond) {
                    self.regs.eip = (self.regs.eip as i32 + offset) as u32;
                }
            }

            // CALL rel32
            0xE8 => {
                let offset = self.fetch_u32() as i32;
                let return_addr = self.regs.eip;
                self.push32(return_addr);
                self.regs.eip = (self.regs.eip as i32 + offset) as u32;
            }

            // RET
            0xC3 => {
                self.regs.eip = self.pop32();
            }

            // RET imm16
            0xC2 => {
                let imm16 = self.fetch_u16() as u32;
                self.regs.eip = self.pop32();
                self.regs.esp = self.regs.esp.wrapping_add(imm16);
            }

            // Indirect Group 5: 0xFF (INC, DEC, CALL m32, JMP m32, PUSH m32)
            0xFF => {
                let (sub_op, op) = self.decode_modrm(is_fs_prefix);
                match sub_op {
                    0 => { // INC r/m32
                        let a = self.read_operand32(&op);
                        let res = a.wrapping_add(1);
                        self.write_operand32(&op, res);
                        self.flags.update_inc_32(a, res);
                    }
                    1 => { // DEC r/m32
                        let a = self.read_operand32(&op);
                        let res = a.wrapping_sub(1);
                        self.write_operand32(&op, res);
                        self.flags.update_dec_32(a, res);
                    }
                    2 => { // CALL r/m32 (Indirect Call)
                        let target = self.read_operand32(&op);
                        let return_addr = self.regs.eip;
                        self.push32(return_addr);
                        self.regs.eip = target;
                    }
                    4 => { // JMP r/m32 (Indirect Jump)
                        let target = self.read_operand32(&op);
                        self.regs.eip = target;
                    }
                    6 => { // PUSH r/m32
                        let val = self.read_operand32(&op);
                        self.push32(val);
                    }
                    _ => return Err(X86Error::InvalidOpcode(opcode, self.regs.eip)),
                }
            }

            // Two-byte escape prefix 0x0F
            0x0F => {
                let sub_op = self.fetch_u8();
                match sub_op {
                    // Jcc rel32 (0x80 .. 0x8F)
                    0x80..=0x8F => {
                        let cond = sub_op & 0x0F;
                        let offset = self.fetch_u32() as i32;
                        if self.flags.eval_condition(cond) {
                            self.regs.eip = (self.regs.eip as i32 + offset) as u32;
                        }
                    }

                    // SETcc r/m8 (0x90 .. 0x9F)
                    0x90..=0x9F => {
                        let cond = sub_op & 0x0F;
                        let (_, op) = self.decode_modrm(is_fs_prefix);
                        let val = if self.flags.eval_condition(cond) { 1u8 } else { 0u8 };
                        self.write_operand8(&op, val);
                    }

                    // IMUL r32, r/m32 (0x0F 0xAF)
                    0xAF => {
                        let (reg, op) = self.decode_modrm(is_fs_prefix);
                        let a = self.regs.get_reg32(reg);
                        let b = self.read_operand32(&op);
                        let res = (a as i32).wrapping_mul(b as i32) as u32;
                        self.regs.set_reg32(reg, res);
                    }

                    // MOVZX r32, r/m8
                    0xB6 => {
                        let (reg, op) = self.decode_modrm(is_fs_prefix);
                        let val = self.read_operand8(&op) as u32;
                        self.regs.set_reg32(reg, val);
                    }

                    // MOVZX r32, r/m16
                    0xB7 => {
                        let (reg, op) = self.decode_modrm(is_fs_prefix);
                        let val = match op {
                            Operand::Reg(r) => self.regs.get_reg16(r) as u32,
                            Operand::Mem(addr) => self.mem.read_u16(addr) as u32,
                        };
                        self.regs.set_reg32(reg, val);
                    }

                    _ => return Err(X86Error::InvalidOpcode(sub_op, self.regs.eip)),
                }
            }

            // x87 FPU Instructions: 0xD8 .. 0xDF
            0xD8..=0xDF => {
                self.execute_fpu_instruction(opcode, is_fs_prefix)?;
            }

            // HLT
            0xF4 => {
                self.is_halted = true;
            }

            _ => return Err(X86Error::InvalidOpcode(opcode, self.regs.eip)),
        }

        self.instruction_count += 1;
        Ok(())
    }

    /// Executes x87 FPU instructions (FLD, FSTP, FADD, FSUB, FMUL, FDIV, FSQRT, etc.).
    fn execute_fpu_instruction(&mut self, opcode: u8, is_fs_prefix: bool) -> Result<(), X86Error> {
        let modrm = self.fetch_u8();
        let mod_bits = (modrm >> 6) & 0x03;
        let sub_op = (modrm >> 3) & 0x07;
        let rm_bits = modrm & 0x07;

        if mod_bits == 3 {
            // Register form
            match opcode {
                0xD9 => {
                    match modrm {
                        0xC0..=0xC7 => self.fpu.push(self.fpu.get_st((modrm & 7) as usize)), // FLD ST(i)
                        0xE0 => self.fpu.fchs(),                                              // FCHS
                        0xE1 => self.fpu.fabs(),                                              // FABS
                        0xE8 => self.fpu.push(1.0),                                           // FLD1
                        0xEE => self.fpu.push(0.0),                                           // FLDZ
                        0xEB => self.fpu.push(core::f64::consts::PI),                         // FLDPI
                        0xFA => self.fpu.fsqrt(),                                             // FSQRT
                        0xFE => self.fpu.fsin(),                                              // FSIN
                        0xFF => self.fpu.fcos(),                                              // FCOS
                        0xF3 => self.fpu.fpatan(),                                            // FPATAN
                        _ => {}
                    }
                }
                0xDD => {
                    if (0xD8..=0xDF).contains(&modrm) {
                        // FSTP ST(i)
                        let val = self.fpu.pop();
                        self.fpu.set_st((modrm & 7) as usize, val);
                    }
                }
                0xDE => {
                    match modrm {
                        0xC1 => { // FADDP
                            let val = self.fpu.pop();
                            self.fpu.st[self.fpu.top] += val;
                        }
                        0xC9 => { // FMULP
                            let val = self.fpu.pop();
                            self.fpu.st[self.fpu.top] *= val;
                        }
                        0xE1 => { // FSUBP
                            let val = self.fpu.pop();
                            self.fpu.st[self.fpu.top] -= val;
                        }
                        0xF1 => { // FDIVP
                            let val = self.fpu.pop();
                            self.fpu.st[self.fpu.top] /= val;
                        }
                        _ => {}
                    }
                }
                0xDF if modrm == 0xE0 => {
                    // FNSTSW AX
                    self.regs.eax = (self.regs.eax & 0xFFFF0000) | (self.fpu.fnstsw() as u32);
                }
                _ => {}
            }
        } else {
            // Memory form (ModR/M memory address)
            let mut base_addr = if rm_bits == 5 && mod_bits == 0 {
                self.fetch_u32()
            } else {
                self.regs.get_reg32(rm_bits)
            };
            if mod_bits == 1 {
                base_addr = base_addr.wrapping_add(self.fetch_u8() as i8 as i32 as u32);
            } else if mod_bits == 2 {
                base_addr = base_addr.wrapping_add(self.fetch_u32());
            }
            if is_fs_prefix {
                base_addr = base_addr.wrapping_add(self.regs.fs_base);
            }

            match opcode {
                // Single-precision (32-bit float)
                0xD9 => {
                    match sub_op {
                        0 => { // FLD m32fp
                            let val = self.mem.read_f32(base_addr) as f64;
                            self.fpu.push(val);
                        }
                        2 => { // FST m32fp
                            let val = self.fpu.get_st(0) as f32;
                            self.mem.write_f32(base_addr, val);
                        }
                        3 => { // FSTP m32fp
                            let val = self.fpu.pop() as f32;
                            self.mem.write_f32(base_addr, val);
                        }
                        _ => {}
                    }
                }
                // Double-precision (64-bit float)
                0xDD => {
                    match sub_op {
                        0 => { // FLD m64fp
                            let val = self.mem.read_f64(base_addr);
                            self.fpu.push(val);
                        }
                        2 => { // FST m64fp
                            let val = self.fpu.get_st(0);
                            self.mem.write_f64(base_addr, val);
                        }
                        3 => { // FSTP m64fp
                            let val = self.fpu.pop();
                            self.mem.write_f64(base_addr, val);
                        }
                        _ => {}
                    }
                }
                // Double-precision math (64-bit float)
                0xDC => {
                    let val = self.mem.read_f64(base_addr);
                    match sub_op {
                        0 => self.fpu.st[self.fpu.top] += val,
                        1 => self.fpu.st[self.fpu.top] *= val,
                        4 => self.fpu.st[self.fpu.top] -= val,
                        6 => self.fpu.st[self.fpu.top] /= val,
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Triggers a Win32 SEH exception dispatch through the `FS:[0]` TIB chain.
    pub fn trigger_seh_exception(&mut self, exception_code: u32) -> Result<(), X86Error> {
        let tib_exception_head = self.regs.fs_base; // FS:[0]
        let curr_record = self.mem.read_u32(tib_exception_head);

        if curr_record == 0xFFFFFFFF || curr_record == 0 {
            return Err(X86Error::Halted(exception_code as i32));
        }

        // Win32 EXCEPTION_REGISTRATION_RECORD layout:
        // [record + 0] = Next (pointer to previous record)
        // [record + 4] = Handler (function pointer to exception filter / handler)
        let handler_addr = self.mem.read_u32(curr_record.wrapping_add(4));

        // Setup stack frame for SEH handler invocation
        // stdcall(pExceptRecord, pEstablisherFrame, pContext, pDispatcherContext)
        self.regs.esp = curr_record;
        self.regs.eip = handler_addr;
        self.regs.eax = exception_code;

        Ok(())
    }

    /// Runs execution until halted or maximum instruction count reached.
    pub fn run(&mut self, max_instructions: u64) -> Result<(), X86Error> {
        let mut count = 0;
        while !self.is_halted && count < max_instructions {
            self.step()?;
            count += 1;
        }
        Ok(())
    }
}
