// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! x87 Floating Point Unit (FPU) Stack & Instructions.

/// Virtual x87 FPU stack (ST(0) through ST(7)).
#[derive(Debug, Clone)]
pub struct X87Fpu {
    pub st: [f64; 8],
    /// Top of stack index (0..7), ST(0) is at index `top`.
    pub top: usize,
    pub status_word: u16,
    pub control_word: u16,
    pub tag_word: u16,
}

impl Default for X87Fpu {
    fn default() -> Self {
        Self {
            st: [0.0; 8],
            top: 0,
            status_word: 0,
            control_word: 0x037F, // Standard initial FPU control word (exceptions masked)
            tag_word: 0xFFFF,     // All registers empty
        }
    }
}

impl X87Fpu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes a 64-bit float onto the FPU stack: ST(0) = val.
    pub fn push(&mut self, val: f64) {
        self.top = (self.top + 7) & 7;
        self.st[self.top] = val;
    }

    /// Pops the value at the top of the FPU stack ST(0).
    pub fn pop(&mut self) -> f64 {
        let val = self.st[self.top];
        self.st[self.top] = 0.0;
        self.top = (self.top + 1) & 7;
        val
    }

    /// Gets ST(i) relative to top of stack.
    pub fn get_st(&self, i: usize) -> f64 {
        self.st[(self.top + i) & 7]
    }

    /// Sets ST(i) relative to top of stack.
    pub fn set_st(&mut self, i: usize, val: f64) {
        self.st[(self.top + i) & 7] = val;
    }

    /// FADD ST(0) += ST(i)
    pub fn fadd(&mut self, i: usize) {
        let rhs = self.get_st(i);
        self.st[self.top] += rhs;
    }

    /// FSUB ST(0) -= ST(i)
    pub fn fsub(&mut self, i: usize) {
        let rhs = self.get_st(i);
        self.st[self.top] -= rhs;
    }

    /// FSUBR ST(0) = ST(i) - ST(0)
    pub fn fsubr(&mut self, i: usize) {
        let rhs = self.get_st(i);
        self.st[self.top] = rhs - self.st[self.top];
    }

    /// FMUL ST(0) *= ST(i)
    pub fn fmul(&mut self, i: usize) {
        let rhs = self.get_st(i);
        self.st[self.top] *= rhs;
    }

    /// FDIV ST(0) /= ST(i)
    pub fn fdiv(&mut self, i: usize) {
        let rhs = self.get_st(i);
        self.st[self.top] /= rhs;
    }

    /// FDIVR ST(0) = ST(i) / ST(0)
    pub fn fdivr(&mut self, i: usize) {
        let rhs = self.get_st(i);
        self.st[self.top] = rhs / self.st[self.top];
    }

    /// FSQRT: ST(0) = sqrt(ST(0))
    pub fn fsqrt(&mut self) {
        self.st[self.top] = self.st[self.top].sqrt();
    }

    /// FCHS: ST(0) = -ST(0)
    pub fn fchs(&mut self) {
        self.st[self.top] = -self.st[self.top];
    }

    /// FABS: ST(0) = |ST(0)|
    pub fn fabs(&mut self) {
        self.st[self.top] = self.st[self.top].abs();
    }

    /// FSIN: ST(0) = sin(ST(0))
    pub fn fsin(&mut self) {
        self.st[self.top] = self.st[self.top].sin();
    }

    /// FCOS: ST(0) = cos(ST(0))
    pub fn fcos(&mut self) {
        self.st[self.top] = self.st[self.top].cos();
    }

    /// FPATAN: ST(1) = atan2(ST(1), ST(0)); pop ST(0)
    pub fn fpatan(&mut self) {
        let x = self.pop();
        let y = self.st[self.top];
        self.st[self.top] = y.atan2(x);
    }

    /// Compares ST(0) with `val`, setting status word condition codes C3, C2, C0.
    pub fn fcom(&mut self, val: f64) {
        let lhs = self.st[self.top];
        // Clear C0 (bit 8), C2 (bit 10), C3 (bit 14)
        self.status_word &= !( (1 << 8) | (1 << 10) | (1 << 14) );
        if lhs.is_nan() || val.is_nan() {
            // Unordered: C3=1, C2=1, C0=1
            self.status_word |= (1 << 8) | (1 << 10) | (1 << 14);
        } else if lhs > val {
            // C3=0, C2=0, C0=0
        } else if lhs < val {
            // C3=0, C2=0, C0=1
            self.status_word |= 1 << 8;
        } else {
            // Equal: C3=1, C2=0, C0=0
            self.status_word |= 1 << 14;
        }
    }

    /// Reads FPU status word for `FNSTSW AX` instruction.
    pub fn fnstsw(&self) -> u16 {
        // Set TOP field in status word (bits 11..13)
        let top_bits = ((self.top as u16) & 0x7) << 11;
        (self.status_word & !(0x7 << 11)) | top_bits
    }
}
