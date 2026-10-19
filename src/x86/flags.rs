// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! 32-bit x86 EFLAGS Register and ALU Condition Logic.

pub const FLAG_CF: u32 = 1 << 0;  // Carry Flag
pub const FLAG_PF: u32 = 1 << 2;  // Parity Flag
pub const FLAG_AF: u32 = 1 << 4;  // Auxiliary Flag
pub const FLAG_ZF: u32 = 1 << 6;  // Zero Flag
pub const FLAG_SF: u32 = 1 << 7;  // Sign Flag
pub const FLAG_TF: u32 = 1 << 8;  // Trap Flag
pub const FLAG_IF: u32 = 1 << 9;  // Interrupt Flag
pub const FLAG_DF: u32 = 1 << 10; // Direction Flag
pub const FLAG_OF: u32 = 1 << 11; // Overflow Flag

/// Encapsulates the 32-bit x86 EFLAGS register state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flags {
    pub eflags: u32,
}

impl Default for Flags {
    fn default() -> Self {
        // Standard initialized EFLAGS (bit 1 is reserved and always 1)
        Self { eflags: 0x00000002 }
    }
}

impl Flags {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn has_flag(&self, mask: u32) -> bool {
        (self.eflags & mask) != 0
    }

    #[inline]
    pub fn set_flag(&mut self, mask: u32, val: bool) {
        if val {
            self.eflags |= mask;
        } else {
            self.eflags &= !mask;
        }
    }

    #[inline]
    pub fn cf(&self) -> bool {
        self.has_flag(FLAG_CF)
    }

    #[inline]
    pub fn pf(&self) -> bool {
        self.has_flag(FLAG_PF)
    }

    #[inline]
    pub fn af(&self) -> bool {
        self.has_flag(FLAG_AF)
    }

    #[inline]
    pub fn zf(&self) -> bool {
        self.has_flag(FLAG_ZF)
    }

    #[inline]
    pub fn sf(&self) -> bool {
        self.has_flag(FLAG_SF)
    }

    #[inline]
    pub fn df(&self) -> bool {
        self.has_flag(FLAG_DF)
    }

    #[inline]
    pub fn of(&self) -> bool {
        self.has_flag(FLAG_OF)
    }

    /// Evaluates parity of the lowest 8 bits (true if even number of 1s).
    #[inline]
    pub fn compute_parity(val: u8) -> bool {
        (val.count_ones() % 2) == 0
    }

    /// Updates flags for a 32-bit addition: `res = a + b`.
    pub fn update_add_32(&mut self, a: u32, b: u32, res: u32) {
        let cf = res < a;
        let zf = res == 0;
        let sf = (res & 0x80000000) != 0;
        let of = ((a ^ res) & (b ^ res) & 0x80000000) != 0;
        let af = ((a & 0x0F) + (b & 0x0F)) > 0x0F;
        let pf = Self::compute_parity(res as u8);

        self.set_flag(FLAG_CF, cf);
        self.set_flag(FLAG_ZF, zf);
        self.set_flag(FLAG_SF, sf);
        self.set_flag(FLAG_OF, of);
        self.set_flag(FLAG_AF, af);
        self.set_flag(FLAG_PF, pf);
    }

    /// Updates flags for a 32-bit addition with carry: `res = a + b + cin`.
    pub fn update_adc_32(&mut self, a: u32, b: u32, cin: u32, res: u32) {
        let cf = if cin != 0 { res <= a } else { res < a };
        let zf = res == 0;
        let sf = (res & 0x80000000) != 0;
        let of = ((a ^ res) & (b ^ res) & 0x80000000) != 0;
        let af = ((a & 0x0F) + (b & 0x0F) + cin) > 0x0F;
        let pf = Self::compute_parity(res as u8);

        self.set_flag(FLAG_CF, cf);
        self.set_flag(FLAG_ZF, zf);
        self.set_flag(FLAG_SF, sf);
        self.set_flag(FLAG_OF, of);
        self.set_flag(FLAG_AF, af);
        self.set_flag(FLAG_PF, pf);
    }

    /// Updates flags for a 32-bit subtraction / comparison: `res = a - b`.
    pub fn update_sub_32(&mut self, a: u32, b: u32, res: u32) {
        let cf = a < b;
        let zf = res == 0;
        let sf = (res & 0x80000000) != 0;
        let of = ((a ^ b) & (a ^ res) & 0x80000000) != 0;
        let af = (a & 0x0F) < (b & 0x0F);
        let pf = Self::compute_parity(res as u8);

        self.set_flag(FLAG_CF, cf);
        self.set_flag(FLAG_ZF, zf);
        self.set_flag(FLAG_SF, sf);
        self.set_flag(FLAG_OF, of);
        self.set_flag(FLAG_AF, af);
        self.set_flag(FLAG_PF, pf);
    }

    /// Updates flags for a 32-bit subtraction with borrow: `res = a - b - bin`.
    pub fn update_sbb_32(&mut self, a: u32, b: u32, bin: u32, res: u32) {
        let cf = if bin != 0 { a <= b } else { a < b };
        let zf = res == 0;
        let sf = (res & 0x80000000) != 0;
        let of = ((a ^ b) & (a ^ res) & 0x80000000) != 0;
        let af = (a & 0x0F) < ((b & 0x0F) + bin);
        let pf = Self::compute_parity(res as u8);

        self.set_flag(FLAG_CF, cf);
        self.set_flag(FLAG_ZF, zf);
        self.set_flag(FLAG_SF, sf);
        self.set_flag(FLAG_OF, of);
        self.set_flag(FLAG_AF, af);
        self.set_flag(FLAG_PF, pf);
    }

    /// Updates flags for logical operations (AND, OR, XOR, TEST): CF=0, OF=0, AF=0.
    pub fn update_logic_32(&mut self, res: u32) {
        self.set_flag(FLAG_CF, false);
        self.set_flag(FLAG_OF, false);
        self.set_flag(FLAG_AF, false);
        self.set_flag(FLAG_ZF, res == 0);
        self.set_flag(FLAG_SF, (res & 0x80000000) != 0);
        self.set_flag(FLAG_PF, Self::compute_parity(res as u8));
    }

    /// Updates flags for INC 32-bit (CF preserved).
    pub fn update_inc_32(&mut self, a: u32, res: u32) {
        let zf = res == 0;
        let sf = (res & 0x80000000) != 0;
        let of = a == 0x7FFFFFFF;
        let af = (a & 0x0F) == 0x0F;
        let pf = Self::compute_parity(res as u8);

        self.set_flag(FLAG_ZF, zf);
        self.set_flag(FLAG_SF, sf);
        self.set_flag(FLAG_OF, of);
        self.set_flag(FLAG_AF, af);
        self.set_flag(FLAG_PF, pf);
    }

    /// Updates flags for DEC 32-bit (CF preserved).
    pub fn update_dec_32(&mut self, a: u32, res: u32) {
        let zf = res == 0;
        let sf = (res & 0x80000000) != 0;
        let of = a == 0x80000000;
        let af = (a & 0x0F) == 0;
        let pf = Self::compute_parity(res as u8);

        self.set_flag(FLAG_ZF, zf);
        self.set_flag(FLAG_SF, sf);
        self.set_flag(FLAG_OF, of);
        self.set_flag(FLAG_AF, af);
        self.set_flag(FLAG_PF, pf);
    }

    /// Evaluates condition codes for Jcc and SETcc instructions (0x0..0xF).
    pub fn eval_condition(&self, cond: u8) -> bool {
        match cond & 0x0F {
            0x0 => self.of(),                                // O (overflow)
            0x1 => !self.of(),                               // NO
            0x2 => self.cf(),                                // B, C, NAE (below, carry)
            0x3 => !self.cf(),                               // NB, NC, AE
            0x4 => self.zf(),                                // E, Z (equal, zero)
            0x5 => !self.zf(),                               // NE, NZ
            0x6 => self.cf() || self.zf(),                   // BE, NA (below or equal)
            0x7 => !self.cf() && !self.zf(),                 // NBE, A (above)
            0x8 => self.sf(),                                // S (sign)
            0x9 => !self.sf(),                               // NS
            0xA => self.pf(),                                // P, PE (parity even)
            0xB => !self.pf(),                               // NP, PO (parity odd)
            0xC => self.sf() != self.of(),                   // L, NGE (less)
            0xD => self.sf() == self.of(),                   // NL, GE (greater or equal)
            0xE => self.zf() || (self.sf() != self.of()),    // LE, NG (less or equal)
            0xF => !self.zf() && (self.sf() == self.of()),   // NLE, G (greater)
            _ => unreachable!(),
        }
    }
}
