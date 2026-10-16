// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Math Intrinsics, CRT Floating Point helpers, Virtual x87 FPU, and Pentium FDIV workarounds.
//!
//! Visual Basic 6 executables compiled with MSVC backends include calls to
//! MSVC CRT math intrinsics (`_CI*`) and the historic Intel Pentium FDIV bug
//! software workaround routines (`_adj_fdiv_*`).

use core::cell::RefCell;

#[cfg(target_os = "linux")]
#[link(name = "m")]
extern "C" {}

/// Rounds a 64-bit float to the nearest integer, breaking ties to the nearest even number
/// in accordance with IEEE 754 Round-to-Nearest-Even (Banker's Rounding).
///
/// # Arguments
/// * `val` - 64-bit floating point number.
///
/// # Returns
/// Rounded 64-bit float value.
pub fn round_to_even(val: f64) -> f64 {
    if val.is_nan() || val.is_infinite() {
        return val;
    }
    let floor = val.floor();
    let diff = val - floor;
    if (diff - 0.5).abs() < 1e-12 {
        let f_int = floor as i64;
        if f_int % 2 == 0 {
            floor
        } else {
            floor + 1.0
        }
    } else if diff < 0.5 {
        floor
    } else {
        floor + 1.0
    }
}

/// Rounds a 64-bit float to a specified number of decimal places using Banker's Rounding.
///
/// # Arguments
/// * `val` - Floating point number to round.
/// * `decimals` - Number of decimal places to preserve.
///
/// # Returns
/// Rounded 64-bit float value.
pub fn bankers_round(val: f64, decimals: i32) -> f64 {
    if decimals <= 0 {
        round_to_even(val)
    } else {
        let factor = 10f64.powi(decimals);
        round_to_even(val * factor) / factor
    }
}

/// Emulated x87 Floating Point Unit state for CRT intrinsics and C2 compiler helpers.
#[derive(Debug, Clone)]
pub struct VirtualFpu {
    /// 8-register circular data stack (`st(0)` through `st(7)`).
    pub regs: [f64; 8],
    /// Current top of stack pointer (0..7).
    pub top: usize,
    /// x87 FPU Status Word condition code bits (C0, C1, C2, C3).
    pub status_word: u16,
}

impl Default for VirtualFpu {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualFpu {
    /// Condition Code 0 (bit 8).
    pub const C0: u16 = 0x0100;
    /// Condition Code 1 (bit 9).
    pub const C1: u16 = 0x0200;
    /// Condition Code 2 (bit 10).
    pub const C2: u16 = 0x0400;
    /// Condition Code 3 (bit 14).
    pub const C3: u16 = 0x4000;

    /// Creates an initialized x87 Virtual FPU instance with all registers zeroed.
    pub const fn new() -> Self {
        Self {
            regs: [0.0; 8],
            top: 0,
            status_word: 0,
        }
    }

    /// Pushes a floating point value onto the top of the x87 register stack (`st(0)`).
    pub fn push(&mut self, val: f64) {
        self.top = (self.top + 7) % 8;
        self.regs[self.top] = val;
    }

    /// Pops the value at `st(0)` from the x87 register stack.
    pub fn pop(&mut self) -> f64 {
        let val = self.regs[self.top];
        self.top = (self.top + 1) % 8;
        val
    }

    /// Reads the value at relative stack index `st(i)`.
    pub fn st(&self, i: usize) -> f64 {
        self.regs[(self.top + i) % 8]
    }

    /// Writes a value to relative stack index `st(i)`.
    pub fn set_st(&mut self, i: usize, val: f64) {
        let idx = (self.top + i) % 8;
        self.regs[idx] = val;
    }

    /// `_adj_fdiv_m32`: divides `st(0)` by a 32-bit single-precision float divisor.
    pub fn fdiv_m32(&mut self, divisor: f32) {
        let cur = self.st(0);
        self.set_st(0, cur / (divisor as f64));
    }

    /// `_adj_fdiv_m64`: divides `st(0)` by a 64-bit double-precision float divisor.
    pub fn fdiv_m64(&mut self, divisor: f64) {
        let cur = self.st(0);
        self.set_st(0, cur / divisor);
    }

    /// `_adj_fdiv_m16i`: divides `st(0)` by a 16-bit integer divisor.
    pub fn fdiv_m16i(&mut self, divisor: i16) {
        let cur = self.st(0);
        self.set_st(0, cur / (divisor as f64));
    }

    /// `_adj_fdiv_m32i`: divides `st(0)` by a 32-bit integer divisor.
    pub fn fdiv_m32i(&mut self, divisor: i32) {
        let cur = self.st(0);
        self.set_st(0, cur / (divisor as f64));
    }

    /// `_adj_fdiv_r`: divides `st(0)` by `st(1)` with register operand.
    pub fn fdiv_r(&mut self) {
        let a = self.st(0);
        let b = self.st(1);
        self.set_st(0, a / b);
    }

    /// `_adj_fdivr_m32`: reverse division, divides 32-bit float by `st(0)`.
    pub fn fdivr_m32(&mut self, divisor: f32) {
        let cur = self.st(0);
        self.set_st(0, (divisor as f64) / cur);
    }

    /// `_adj_fdivr_m64`: reverse division, divides 64-bit double by `st(0)`.
    pub fn fdivr_m64(&mut self, divisor: f64) {
        let cur = self.st(0);
        self.set_st(0, divisor / cur);
    }

    /// `_adj_fdivr_m16i`: reverse division, divides 16-bit integer by `st(0)`.
    pub fn fdivr_m16i(&mut self, divisor: i16) {
        let cur = self.st(0);
        self.set_st(0, (divisor as f64) / cur);
    }

    /// `_adj_fdivr_m32i`: reverse division, divides 32-bit integer by `st(0)`.
    pub fn fdivr_m32i(&mut self, divisor: i32) {
        let cur = self.st(0);
        self.set_st(0, (divisor as f64) / cur);
    }

    /// `_adj_fprem`: partial remainder operation.
    ///
    /// Computes `st(0) % st(1)` (truncated towards zero) and sets condition codes
    /// C0, C1, C3 to the lower 3 bits of quotient, and clears C2.
    pub fn fprem(&mut self) {
        let a = self.st(0);
        let b = self.st(1);
        if b != 0.0 {
            let q = (a / b).trunc() as i64;
            let rem = a - (q as f64) * b;
            self.set_st(0, rem);

            // Clear condition codes C0, C1, C2, C3
            self.status_word &= !(Self::C0 | Self::C1 | Self::C2 | Self::C3);
            if (q & 4) != 0 {
                self.status_word |= Self::C0;
            }
            if (q & 2) != 0 {
                self.status_word |= Self::C3;
            }
            if (q & 1) != 0 {
                self.status_word |= Self::C1;
            }
        }
    }

    /// `_adj_fprem1`: IEEE 754 partial remainder operation.
    ///
    /// Computes IEEE remainder using Banker's Rounding: `st(0) - round_to_even(st(0)/st(1)) * st(1)`.
    pub fn fprem1(&mut self) {
        let a = self.st(0);
        let b = self.st(1);
        if b != 0.0 {
            let q = round_to_even(a / b) as i64;
            let rem = a - (q as f64) * b;
            self.set_st(0, rem);

            self.status_word &= !(Self::C0 | Self::C1 | Self::C2 | Self::C3);
            if (q & 4) != 0 {
                self.status_word |= Self::C0;
            }
            if (q & 2) != 0 {
                self.status_word |= Self::C3;
            }
            if (q & 1) != 0 {
                self.status_word |= Self::C1;
            }
        }
    }

    /// `_adj_fpatan`: partial arctangent.
    pub fn fpatan(&mut self) {
        let x = self.pop();
        let y = self.st(0);
        self.set_st(0, y.atan2(x));
    }

    /// `_adj_fptan`: partial tangent.
    pub fn fptan(&mut self) {
        let cur = self.st(0);
        self.set_st(0, cur.tan());
        self.push(1.0);
    }

    /// Computes cosine of `st(0)`.
    pub fn cos(&mut self) {
        let cur = self.st(0);
        self.set_st(0, cur.cos());
    }

    /// Computes sine of `st(0)`.
    pub fn sin(&mut self) {
        let cur = self.st(0);
        self.set_st(0, cur.sin());
    }

    /// Computes tangent of `st(0)`.
    pub fn tan(&mut self) {
        let cur = self.st(0);
        self.set_st(0, cur.tan());
    }

    /// Computes arctangent of `st(0)`.
    pub fn atan(&mut self) {
        let cur = self.st(0);
        self.set_st(0, cur.atan());
    }

    /// Computes square root of `st(0)`.
    pub fn sqrt(&mut self) {
        let cur = self.st(0);
        if cur < 0.0 && cur != -0.0 {
            crate::err::set_vb_error(5, None, None);
        } else {
            self.set_st(0, cur.sqrt());
        }
    }

    /// Computes natural logarithm of `st(0)`.
    pub fn log(&mut self) {
        let cur = self.st(0);
        if cur <= 0.0 {
            crate::err::set_vb_error(5, None, None);
        } else {
            self.set_st(0, cur.ln());
        }
    }

    /// Computes natural exponential of `st(0)`.
    pub fn exp(&mut self) {
        let cur = self.st(0);
        if cur > 709.782712893384 {
            crate::err::set_vb_error(6, None, None);
        } else {
            self.set_st(0, cur.exp());
        }
    }
}

#[cfg(feature = "std")]
std::thread_local! {
    /// Thread-local virtual x87 FPU state for MSVC CRT and C2 math calls.
    pub static CURRENT_FPU: RefCell<VirtualFpu> = const { RefCell::new(VirtualFpu::new()) };
}

// ----------------------------------------------------------------------------
// CRT Math Intrinsics (x87 FPU Register Operations)
// ----------------------------------------------------------------------------

/// CRT intrinsic computing the arc tangent of the value in the x87 FPU top-of-stack register `st(0)`.
#[no_mangle]
pub unsafe extern "C" fn _CIatan() {
    #[cfg(feature = "std")]
    CURRENT_FPU.with(|f| f.borrow_mut().atan());
}

/// CRT intrinsic computing the cosine of the value in the x87 FPU top-of-stack register `st(0)`.
#[no_mangle]
pub unsafe extern "C" fn _CIcos() {
    #[cfg(feature = "std")]
    CURRENT_FPU.with(|f| f.borrow_mut().cos());
}

/// CRT intrinsic computing the natural exponential (`e^x`) of the value in `st(0)`.
#[no_mangle]
pub unsafe extern "C" fn _CIexp() {
    #[cfg(feature = "std")]
    CURRENT_FPU.with(|f| f.borrow_mut().exp());
}

/// CRT intrinsic computing the natural logarithm (`ln(x)`) of the value in `st(0)`.
#[no_mangle]
pub unsafe extern "C" fn _CIlog() {
    #[cfg(feature = "std")]
    CURRENT_FPU.with(|f| f.borrow_mut().log());
}

/// CRT intrinsic computing the sine of the value in the x87 FPU top-of-stack register `st(0)`.
#[no_mangle]
pub unsafe extern "C" fn _CIsin() {
    #[cfg(feature = "std")]
    CURRENT_FPU.with(|f| f.borrow_mut().sin());
}

/// CRT intrinsic computing the square root of the value in the x87 FPU top-of-stack register `st(0)`.
#[no_mangle]
pub unsafe extern "C" fn _CIsqrt() {
    #[cfg(feature = "std")]
    CURRENT_FPU.with(|f| f.borrow_mut().sqrt());
}

/// CRT intrinsic computing the tangent of the value in the x87 FPU top-of-stack register `st(0)`.
#[no_mangle]
pub unsafe extern "C" fn _CItan() {
    #[cfg(feature = "std")]
    CURRENT_FPU.with(|f| f.borrow_mut().tan());
}

// ----------------------------------------------------------------------------
// 64-bit Integer Multiplication Runtime Helper
// ----------------------------------------------------------------------------

/// 64-bit integer multiplication runtime helper emitted by MSVC backends for 32-bit x86 targets.
#[no_mangle]
pub unsafe extern "C" fn _allmul(a: i64, b: i64) -> i64 {
    a.wrapping_mul(b)
}

// ----------------------------------------------------------------------------
// Pentium FDIV Bug Workarounds (_adj_*)
// ----------------------------------------------------------------------------

/// Workaround helper for Intel Pentium FDIV bug: 16-bit integer divisor memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdiv_m16i(divisor_ptr: *const i16) {
    if !divisor_ptr.is_null() {
        #[cfg(feature = "std")]
        CURRENT_FPU.with(|f| f.borrow_mut().fdiv_m16i(*divisor_ptr));
    }
}

/// Workaround helper for Intel Pentium FDIV bug: 32-bit single-precision float divisor memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdiv_m32(divisor_ptr: *const f32) {
    if !divisor_ptr.is_null() {
        #[cfg(feature = "std")]
        CURRENT_FPU.with(|f| f.borrow_mut().fdiv_m32(*divisor_ptr));
    }
}

/// Workaround helper for Intel Pentium FDIV bug: 32-bit integer divisor memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdiv_m32i(divisor_ptr: *const i32) {
    if !divisor_ptr.is_null() {
        #[cfg(feature = "std")]
        CURRENT_FPU.with(|f| f.borrow_mut().fdiv_m32i(*divisor_ptr));
    }
}

/// Workaround helper for Intel Pentium FDIV bug: 64-bit double-precision float divisor memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdiv_m64(divisor_ptr: *const f64) {
    if !divisor_ptr.is_null() {
        #[cfg(feature = "std")]
        CURRENT_FPU.with(|f| f.borrow_mut().fdiv_m64(*divisor_ptr));
    }
}

/// Workaround helper for Intel Pentium FDIV bug: floating-point division with register operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdiv_r() {
    #[cfg(feature = "std")]
    CURRENT_FPU.with(|f| f.borrow_mut().fdiv_r());
}

/// Workaround helper for Intel Pentium FDIV bug: reverse division with 16-bit integer memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdivr_m16i(divisor_ptr: *const i16) {
    if !divisor_ptr.is_null() {
        #[cfg(feature = "std")]
        CURRENT_FPU.with(|f| f.borrow_mut().fdivr_m16i(*divisor_ptr));
    }
}

/// Workaround helper for Intel Pentium FDIV bug: reverse division with 32-bit float memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdivr_m32(divisor_ptr: *const f32) {
    if !divisor_ptr.is_null() {
        #[cfg(feature = "std")]
        CURRENT_FPU.with(|f| f.borrow_mut().fdivr_m32(*divisor_ptr));
    }
}

/// Workaround helper for Intel Pentium FDIV bug: reverse division with 32-bit integer memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdivr_m32i(divisor_ptr: *const i32) {
    if !divisor_ptr.is_null() {
        #[cfg(feature = "std")]
        CURRENT_FPU.with(|f| f.borrow_mut().fdivr_m32i(*divisor_ptr));
    }
}

/// Workaround helper for Intel Pentium FDIV bug: reverse division with 64-bit double memory operand.
#[no_mangle]
pub unsafe extern "C" fn _adj_fdivr_m64(divisor_ptr: *const f64) {
    if !divisor_ptr.is_null() {
        #[cfg(feature = "std")]
        CURRENT_FPU.with(|f| f.borrow_mut().fdivr_m64(*divisor_ptr));
    }
}

/// Workaround helper for Intel Pentium FDIV bug: partial arc tangent operation.
#[no_mangle]
pub unsafe extern "C" fn _adj_fpatan() {
    #[cfg(feature = "std")]
    CURRENT_FPU.with(|f| f.borrow_mut().fpatan());
}

/// Workaround helper for Intel Pentium FDIV bug: partial remainder operation.
#[no_mangle]
pub unsafe extern "C" fn _adj_fprem() {
    #[cfg(feature = "std")]
    CURRENT_FPU.with(|f| f.borrow_mut().fprem());
}

/// Workaround helper for Intel Pentium FDIV bug: IEEE partial remainder operation.
#[no_mangle]
pub unsafe extern "C" fn _adj_fprem1() {
    #[cfg(feature = "std")]
    CURRENT_FPU.with(|f| f.borrow_mut().fprem1());
}

/// Workaround helper for Intel Pentium FDIV bug: partial tangent operation.
#[no_mangle]
pub unsafe extern "C" fn _adj_fptan() {
    #[cfg(feature = "std")]
    CURRENT_FPU.with(|f| f.borrow_mut().fptan());
}
