// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! 32-bit x86 General Purpose & Segment Registers.

/// Standard 32-bit x86 register state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registers {
    pub eax: u32,
    pub ecx: u32,
    pub edx: u32,
    pub ebx: u32,
    pub esp: u32,
    pub ebp: u32,
    pub esi: u32,
    pub edi: u32,
    pub eip: u32,
    /// Win32 FS segment base (points to Thread Information Block - TIB).
    pub fs_base: u32,
}

impl Default for Registers {
    fn default() -> Self {
        Self {
            eax: 0,
            ecx: 0,
            edx: 0,
            ebx: 0,
            esp: 0x001FFF00, // Top of 1MB stack
            ebp: 0x001FFF00,
            esi: 0,
            edi: 0,
            eip: 0,
            fs_base: 0x000F0000, // Win32 TIB base
        }
    }
}

impl Registers {
    /// Creates a new register state initialized with stack and TIB pointers.
    pub fn new(initial_eip: u32, initial_esp: u32, fs_base: u32) -> Self {
        Self {
            eax: 0,
            ecx: 0,
            edx: 0,
            ebx: 0,
            esp: initial_esp,
            ebp: initial_esp,
            esi: 0,
            edi: 0,
            eip: initial_eip,
            fs_base,
        }
    }

    /// Gets a 32-bit register value by standard x86 ModR/M register index (0..7).
    pub fn get_reg32(&self, idx: u8) -> u32 {
        match idx & 0x07 {
            0 => self.eax,
            1 => self.ecx,
            2 => self.edx,
            3 => self.ebx,
            4 => self.esp,
            5 => self.ebp,
            6 => self.esi,
            7 => self.edi,
            _ => unreachable!(),
        }
    }

    /// Sets a 32-bit register value by standard x86 ModR/M register index (0..7).
    pub fn set_reg32(&mut self, idx: u8, val: u32) {
        match idx & 0x07 {
            0 => self.eax = val,
            1 => self.ecx = val,
            2 => self.edx = val,
            3 => self.ebx = val,
            4 => self.esp = val,
            5 => self.ebp = val,
            6 => self.esi = val,
            7 => self.edi = val,
            _ => unreachable!(),
        }
    }

    /// Gets a 16-bit register value by standard x86 register index (0..7).
    pub fn get_reg16(&self, idx: u8) -> u16 {
        self.get_reg32(idx) as u16
    }

    /// Sets a 16-bit register value, preserving the upper 16 bits of the 32-bit register.
    pub fn set_reg16(&mut self, idx: u8, val: u16) {
        let old = self.get_reg32(idx);
        let new_val = (old & 0xFFFF0000) | (val as u32);
        self.set_reg32(idx, new_val);
    }

    /// Gets an 8-bit register value:
    /// 0: AL, 1: CL, 2: DL, 3: BL, 4: AH, 5: CH, 6: DH, 7: BH.
    pub fn get_reg8(&self, idx: u8) -> u8 {
        match idx & 0x07 {
            0 => (self.eax & 0xFF) as u8,
            1 => (self.ecx & 0xFF) as u8,
            2 => (self.edx & 0xFF) as u8,
            3 => (self.ebx & 0xFF) as u8,
            4 => ((self.eax >> 8) & 0xFF) as u8,
            5 => ((self.ecx >> 8) & 0xFF) as u8,
            6 => ((self.edx >> 8) & 0xFF) as u8,
            7 => ((self.ebx >> 8) & 0xFF) as u8,
            _ => unreachable!(),
        }
    }

    /// Sets an 8-bit register value:
    /// 0: AL, 1: CL, 2: DL, 3: BL, 4: AH, 5: CH, 6: DH, 7: BH.
    pub fn set_reg8(&mut self, idx: u8, val: u8) {
        let v = val as u32;
        match idx & 0x07 {
            0 => self.eax = (self.eax & 0xFFFFFF00) | v,
            1 => self.ecx = (self.ecx & 0xFFFFFF00) | v,
            2 => self.edx = (self.edx & 0xFFFFFF00) | v,
            3 => self.ebx = (self.ebx & 0xFFFFFF00) | v,
            4 => self.eax = (self.eax & 0xFFFF00FF) | (v << 8),
            5 => self.ecx = (self.ecx & 0xFFFF00FF) | (v << 8),
            6 => self.edx = (self.edx & 0xFFFF00FF) | (v << 8),
            7 => self.ebx = (self.ebx & 0xFFFF00FF) | (v << 8),
            _ => unreachable!(),
        }
    }
}
