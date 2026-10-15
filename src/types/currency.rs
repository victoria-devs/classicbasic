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
    ///
    /// # Arguments
    /// * `val` - 64-bit integer already multiplied by 10,000.
    ///
    /// # Returns
    /// Initialized `Currency` instance.
    pub fn from_scaled(val: i64) -> Self {
        Self(val)
    }

    /// Converts a floating-point number into a fixed-point `Currency` by multiplying by 10,000.
    ///
    /// # Arguments
    /// * `val` - 64-bit float value.
    ///
    /// # Returns
    /// Initialized `Currency` instance.
    pub fn from_f64(val: f64) -> Self {
        Self((val * Self::SCALE as f64) as i64)
    }

    /// Converts the fixed-point `Currency` value into a 64-bit floating-point number.
    ///
    /// # Returns
    /// 64-bit float representation.
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / Self::SCALE as f64
    }
}
