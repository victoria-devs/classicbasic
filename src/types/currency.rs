// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Visual Basic 6.0 Currency (CY) data type.
//!
//! Visual Basic's `Currency` is an 8-byte (64-bit) two's complement integer,
//! scaled by 10,000 to give a fixed-point number with 15 digits to the left
//! of the decimal point and 4 digits to the right.

/// Visual Basic 6.0 64-bit fixed-point Currency (`CY`) structure.
#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Currency(pub i64);

impl Currency {
    /// Fixed scale factor (10,000) representing four decimal digits.
    pub const SCALE: i64 = 10_000;

    /// Constructs a `Currency` from a pre-scaled 64-bit raw integer value.
    pub fn from_scaled(val: i64) -> Self {
        Self(val)
    }

    /// Constructs a `Currency` from a raw 64-bit integer.
    pub fn from_raw(val: i64) -> Self {
        Self(val)
    }

    /// Returns the raw 64-bit integer representation.
    pub fn as_raw(self) -> i64 {
        self.0
    }

    /// Checked multiplication with 128-bit intermediate math and Banker's Rounding.
    pub fn checked_mul(self, other: Self) -> Option<Self> {
        let prod = (self.0 as i128) * (other.0 as i128);
        let q = prod / (Self::SCALE as i128);
        if q > (i64::MAX as i128) || q < (i64::MIN as i128) {
            return None;
        }
        let rem = (prod % (Self::SCALE as i128)) as i64;
        let abs_rem = rem.abs();
        let half = Self::SCALE / 2;
        let round_up = abs_rem > half || (abs_rem == half && ((q as i64) % 2 != 0));
        let res = if round_up {
            if prod > 0 {
                (q as i64).checked_add(1)?
            } else {
                (q as i64).checked_sub(1)?
            }
        } else {
            q as i64
        };
        Some(Self(res))
    }

    /// Rounds the Currency value to the specified number of decimal digits using Banker's Rounding.
    pub fn round_to_decimals(self, decimals: u32) -> Self {
        let factor: i64 = match decimals {
            0 => 10_000,
            1 => 1_000,
            2 => 100,
            3 => 10,
            _ => 1,
        };
        let q = self.0 / factor;
        let rem = (self.0 % factor).abs();
        let half = factor / 2;
        let round_up = rem > half || (rem == half && (q % 2 != 0));
        let res = if round_up {
            if self.0 > 0 { q + 1 } else { q - 1 }
        } else {
            q
        };
        Self(res * factor)
    }

    /// Converts a floating-point number into a fixed-point `Currency` using Banker's Rounding.
    ///
    /// # Arguments
    /// * `val` - 64-bit float value.
    ///
    /// # Returns
    /// Initialized `Currency` instance.
    pub fn from_f64(val: f64) -> Self {
        let scaled = val * (Self::SCALE as f64);
        Self(crate::math::round_to_even(scaled) as i64)
    }

    /// Converts the fixed-point `Currency` value into a 64-bit floating-point number.
    ///
    /// # Returns
    /// 64-bit float representation.
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / Self::SCALE as f64
    }

    /// Multiplies two Currency values using 128-bit intermediate math to prevent
    /// premature 64-bit overflow and applies Banker's Rounding to the remainder.
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, other: Self) -> Self {
        let prod = (self.0 as i128) * (other.0 as i128);
        let q = prod / (Self::SCALE as i128);
        let rem = (prod % (Self::SCALE as i128)) as i64;
        let abs_rem = rem.abs();
        let half = Self::SCALE / 2;
        let round_up = abs_rem > half || (abs_rem == half && ((q as i64) % 2 != 0));
        let res = if round_up {
            if prod > 0 {
                (q as i64) + 1
            } else {
                (q as i64) - 1
            }
        } else {
            q as i64
        };
        Self(res)
    }

    /// Divides two Currency values using 128-bit intermediate math and Banker's Rounding.
    #[allow(clippy::should_implement_trait)]
    pub fn div(self, other: Self) -> Self {
        if other.0 == 0 {
            crate::err::set_vb_error(11, None, None);
            return Self(0);
        }
        let num = (self.0 as i128) * (Self::SCALE as i128);
        let den = other.0 as i128;
        let q = num / den;
        let rem = (num % den) as i64;
        let abs_den = den.abs() as i64;
        let half = abs_den / 2;
        let abs_rem = rem.abs();
        let round_up = abs_rem > half || (abs_rem == half && ((q as i64) % 2 != 0));
        let res = if round_up {
            if (num > 0 && den > 0) || (num < 0 && den < 0) {
                (q as i64) + 1
            } else {
                (q as i64) - 1
            }
        } else {
            q as i64
        };
        Self(res)
    }
}
