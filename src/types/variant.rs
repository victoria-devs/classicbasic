//! COM and Visual Basic 6.0 VARIANT structure and vartype constants.
//!
//! A VB6 `Variant` is a 16-byte tagged union used for dynamic typing across
//! Visual Basic and COM Automation.

/// Unsigned 16-bit integer representing a `Variant` subtype tag (`VT_*`).
pub type VarType = u16;

/// Empty variant (uninitialized).
pub const VT_EMPTY: VarType = 0;
/// Null variant (SQL database NULL).
pub const VT_NULL: VarType = 1;
/// 2-byte signed integer (VB6 `Integer`).
pub const VT_I2: VarType = 2;
/// 4-byte signed integer (VB6 `Long`).
pub const VT_I4: VarType = 3;
/// 4-byte single-precision float (VB6 `Single`).
pub const VT_R4: VarType = 4;
/// 8-byte double-precision float (VB6 `Double`).
pub const VT_R8: VarType = 5;
/// 8-byte fixed-point currency (VB6 `Currency`).
pub const VT_CY: VarType = 6;
/// 8-byte floating point date (VB6 `Date`).
pub const VT_DATE: VarType = 7;
/// OLE Automation string pointer (VB6 `String`).
pub const VT_BSTR: VarType = 8;
/// `IDispatch` interface pointer (VB6 `Object`).
pub const VT_DISPATCH: VarType = 9;
/// SCODE error code (VB6 `Error`).
pub const VT_ERROR: VarType = 10;
/// Boolean: -1 for True, 0 for False (VB6 `Boolean`).
pub const VT_BOOL: VarType = 11;
/// Pointer to another `Variant` structure.
pub const VT_VARIANT: VarType = 12;
/// `IUnknown` interface pointer.
pub const VT_UNKNOWN: VarType = 13;
/// 16-byte fixed-point Decimal (VB6 `Decimal`).
pub const VT_DECIMAL: VarType = 14;
/// 1-byte signed char.
pub const VT_I1: VarType = 16;
/// 1-byte unsigned char (VB6 `Byte`).
pub const VT_UI1: VarType = 17;
/// 2-byte unsigned integer.
pub const VT_UI2: VarType = 18;
/// 4-byte unsigned integer.
pub const VT_UI4: VarType = 19;
/// 8-byte signed integer.
pub const VT_I8: VarType = 20;
/// 8-byte unsigned integer.
pub const VT_UI8: VarType = 21;
/// Integer.
pub const VT_INT: VarType = 22;
/// Unsigned integer.
pub const VT_UINT: VarType = 23;
/// C-style void.
pub const VT_VOID: VarType = 24;
/// Standard COM `HRESULT` return value.
pub const VT_HRESULT: VarType = 25;
/// Pointer type.
pub const VT_PTR: VarType = 26;
/// SAFEARRAY pointer.
pub const VT_SAFEARRAY: VarType = 27;
/// C-style fixed array.
pub const VT_CARRAY: VarType = 28;
/// User Defined Type (UDT).
pub const VT_USERDEFINED: VarType = 29;
/// Null-terminated ANSI string.
pub const VT_LPSTR: VarType = 30;
/// Null-terminated wide string.
pub const VT_LPWSTR: VarType = 31;
/// User defined record.
pub const VT_RECORD: VarType = 36;
/// Flag bit indicating that the variant contains an array of the specified subtype.
pub const VT_ARRAY: VarType = 0x2000;
/// Flag bit indicating that the variant data is passed by reference (pointer).
pub const VT_BYREF: VarType = 0x4000;

/// 16-byte COM Automation `VARIANT` tagged union.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Variant {
    /// Type descriptor tag indicating which union member is active (`VT_*`).
    pub vt: VarType,
    /// Reserved word 1.
    pub w_reserved1: u16,
    /// Reserved word 2.
    pub w_reserved2: u16,
    /// Reserved word 3.
    pub w_reserved3: u16,
    /// 8-byte data payload storing primitive values, BSTR pointers, or interface addresses.
    pub data: [u8; 8],
}

impl core::fmt::Debug for Variant {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Variant")
            .field("vt", &self.vt)
            .field("data", &self.data)
            .finish()
    }
}

impl Variant {
    /// Creates an empty, uninitialized `Variant` with tag `VT_EMPTY`.
    ///
    /// # Returns
    /// An empty `Variant`.
    pub const fn empty() -> Self {
        Self {
            vt: VT_EMPTY,
            w_reserved1: 0,
            w_reserved2: 0,
            w_reserved3: 0,
            data: [0; 8],
        }
    }

    /// Creates a 16-bit integer `Variant` (`VT_I2`).
    ///
    /// # Arguments
    /// * `val` - 16-bit signed integer value.
    ///
    /// # Returns
    /// Initialized `Variant`.
    pub fn from_i16(val: i16) -> Self {
        let mut v = Self::empty();
        v.vt = VT_I2;
        let bytes = val.to_ne_bytes();
        v.data[0] = bytes[0];
        v.data[1] = bytes[1];
        v
    }

    /// Creates a 32-bit integer `Variant` (`VT_I4`).
    ///
    /// # Arguments
    /// * `val` - 32-bit signed integer value.
    ///
    /// # Returns
    /// Initialized `Variant`.
    pub fn from_i32(val: i32) -> Self {
        let mut v = Self::empty();
        v.vt = VT_I4;
        let bytes = val.to_ne_bytes();
        v.data[..4].copy_from_slice(&bytes);
        v
    }

    /// Creates a 32-bit single-precision floating-point `Variant` (`VT_R4`).
    ///
    /// # Arguments
    /// * `val` - 32-bit float value.
    ///
    /// # Returns
    /// Initialized `Variant`.
    pub fn from_f32(val: f32) -> Self {
        let mut v = Self::empty();
        v.vt = VT_R4;
        let bytes = val.to_ne_bytes();
        v.data[..4].copy_from_slice(&bytes);
        v
    }

    /// Creates a 64-bit double-precision floating-point `Variant` (`VT_R8`).
    ///
    /// # Arguments
    /// * `val` - 64-bit float value.
    ///
    /// # Returns
    /// Initialized `Variant`.
    pub fn from_f64(val: f64) -> Self {
        let mut v = Self::empty();
        v.vt = VT_R8;
        let bytes = val.to_ne_bytes();
        v.data.copy_from_slice(&bytes);
        v
    }

    /// Extracts a 32-bit integer from this `Variant` if it holds an integer or boolean.
    ///
    /// Automatically converts `VT_I2` and `VT_BOOL` values into standard `i32`.
    ///
    /// # Returns
    /// `Some(i32)` if the variant holds an integer subtype, or `None` otherwise.
    pub fn as_i32(&self) -> Option<i32> {
        match self.vt {
            VT_I2 => {
                let bytes = [self.data[0], self.data[1]];
                Some(i16::from_ne_bytes(bytes) as i32)
            }
            VT_I4 => {
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&self.data[..4]);
                Some(i32::from_ne_bytes(bytes))
            }
            VT_BOOL => {
                let bytes = [self.data[0], self.data[1]];
                Some(i16::from_ne_bytes(bytes) as i32)
            }
            _ => None,
        }
    }
}
