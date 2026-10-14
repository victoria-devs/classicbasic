//! Visual Basic 6.0 Currency (CY) data type.
//!
//! Visual Basic's `Currency` is an 8-byte (64-bit) two's complement integer,
//! scaled by 10,000 to give a fixed-point number with 15 digits to the left
//! of the decimal point and 4 digits to the right.

#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Currency(pub i64);

impl Currency {
    pub const SCALE: i64 = 10_000;

    pub fn from_scaled(val: i64) -> Self {
        Self(val)
    }

    pub fn from_f64(val: f64) -> Self {
        Self((val * Self::SCALE as f64) as i64)
    }

    pub fn to_f64(self) -> f64 {
        self.0 as f64 / Self::SCALE as f64
    }
}
