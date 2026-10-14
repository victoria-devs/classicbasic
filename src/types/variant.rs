//! COM and Visual Basic 6.0 VARIANT structure and vartype constants.
//!
//! A VB6 `Variant` is a 16-byte tagged union used for dynamic typing across
//! Visual Basic and COM Automation.

pub type VarType = u16;

pub const VT_EMPTY: VarType = 0;
pub const VT_NULL: VarType = 1;
pub const VT_I2: VarType = 2;
pub const VT_I4: VarType = 3;
pub const VT_R4: VarType = 4;
pub const VT_R8: VarType = 5;
pub const VT_CY: VarType = 6;
pub const VT_DATE: VarType = 7;
pub const VT_BSTR: VarType = 8;
pub const VT_DISPATCH: VarType = 9;
pub const VT_ERROR: VarType = 10;
pub const VT_BOOL: VarType = 11;
pub const VT_VARIANT: VarType = 12;
pub const VT_UNKNOWN: VarType = 13;
pub const VT_DECIMAL: VarType = 14;
pub const VT_I1: VarType = 16;
pub const VT_UI1: VarType = 17;
pub const VT_UI2: VarType = 18;
pub const VT_UI4: VarType = 19;
pub const VT_I8: VarType = 20;
pub const VT_UI8: VarType = 21;
pub const VT_INT: VarType = 22;
pub const VT_UINT: VarType = 23;
pub const VT_VOID: VarType = 24;
pub const VT_HRESULT: VarType = 25;
pub const VT_PTR: VarType = 26;
pub const VT_SAFEARRAY: VarType = 27;
pub const VT_CARRAY: VarType = 28;
pub const VT_USERDEFINED: VarType = 29;
pub const VT_LPSTR: VarType = 30;
pub const VT_LPWSTR: VarType = 31;
pub const VT_RECORD: VarType = 36;
pub const VT_ARRAY: VarType = 0x2000;
pub const VT_BYREF: VarType = 0x4000;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Variant {
    pub vt: VarType,
    pub w_reserved1: u16,
    pub w_reserved2: u16,
    pub w_reserved3: u16,
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
    pub const fn empty() -> Self {
        Self {
            vt: VT_EMPTY,
            w_reserved1: 0,
            w_reserved2: 0,
            w_reserved3: 0,
            data: [0; 8],
        }
    }

    pub fn from_i16(val: i16) -> Self {
        let mut v = Self::empty();
        v.vt = VT_I2;
        let bytes = val.to_ne_bytes();
        v.data[0] = bytes[0];
        v.data[1] = bytes[1];
        v
    }

    pub fn from_i32(val: i32) -> Self {
        let mut v = Self::empty();
        v.vt = VT_I4;
        let bytes = val.to_ne_bytes();
        v.data[..4].copy_from_slice(&bytes);
        v
    }

    pub fn from_f32(val: f32) -> Self {
        let mut v = Self::empty();
        v.vt = VT_R4;
        let bytes = val.to_ne_bytes();
        v.data[..4].copy_from_slice(&bytes);
        v
    }

    pub fn from_f64(val: f64) -> Self {
        let mut v = Self::empty();
        v.vt = VT_R8;
        let bytes = val.to_ne_bytes();
        v.data.copy_from_slice(&bytes);
        v
    }

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
