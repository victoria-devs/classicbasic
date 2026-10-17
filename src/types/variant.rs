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

    /// Creates a null `Variant` with tag `VT_NULL`.
    ///
    /// # Returns
    /// A null `Variant`.
    pub const fn null() -> Self {
        Self {
            vt: VT_NULL,
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

    /// Creates a ByRef Long reference (`VT_BYREF | VT_I4`).
    pub fn byref_i32(target: *mut i32) -> Self {
        let mut v = Self::empty();
        v.vt = VT_BYREF | VT_I4;
        let bytes = (target as usize).to_ne_bytes();
        let copy_len = bytes.len().min(8);
        v.data[..copy_len].copy_from_slice(&bytes[..copy_len]);
        v
    }

    /// Maximum 96-bit integer value that can be held by `VT_DECIMAL` (2^96 - 1).
    pub const DECIMAL_MAX: u128 = 79228162514264337593543950335u128;

    /// Creates a Decimal `Variant` (`VT_DECIMAL`).
    ///
    /// In COM Automation:
    /// - bytes 0..2: vt (VT_DECIMAL = 14)
    /// - byte 2: scale (0..28)
    /// - byte 3: sign (0x00 = positive, 0x80 = negative)
    /// - bytes 4..8: Hi32
    /// - bytes 8..16: Lo64
    pub fn from_decimal(value_96: u128, scale: u8, is_negative: bool) -> Self {
        let mut v = Self::empty();
        v.vt = VT_DECIMAL;
        let scale_byte = scale;
        let sign_byte = if is_negative { 0x80u8 } else { 0x00u8 };
        v.w_reserved1 = (scale_byte as u16) | ((sign_byte as u16) << 8);
        let hi32 = (value_96 >> 64) as u32;
        v.w_reserved2 = (hi32 & 0xFFFF) as u16;
        v.w_reserved3 = ((hi32 >> 16) & 0xFFFF) as u16;
        let lo64 = value_96 as u64;
        v.data.copy_from_slice(&lo64.to_ne_bytes());
        v
    }

    /// Extracts `(value_96, scale, is_negative)` from a `VT_DECIMAL` variant.
    pub fn as_decimal(&self) -> Option<(u128, u8, bool)> {
        let target = self.deref_variant();
        if target.vt == VT_DECIMAL {
            let scale = (target.w_reserved1 & 0xFF) as u8;
            let sign = ((target.w_reserved1 >> 8) & 0xFF) as u8;
            let hi32 = (target.w_reserved2 as u32) | ((target.w_reserved3 as u32) << 16);
            let lo64 = u64::from_ne_bytes(target.data);
            let val_96 = ((hi32 as u128) << 64) | (lo64 as u128);
            Some((val_96, scale, (sign & 0x80) != 0))
        } else {
            None
        }
    }

    /// Creates a ByRef Variant reference (`VT_BYREF | VT_VARIANT`).
    pub fn byref_variant(target: *const Variant) -> Self {
        let mut v = Self::empty();
        v.vt = VT_BYREF | VT_VARIANT;
        let bytes = (target as usize).to_ne_bytes();
        let copy_len = bytes.len().min(8);
        v.data[..copy_len].copy_from_slice(&bytes[..copy_len]);
        v
    }

    /// Retrieves the pointer/address value stored in the 8-byte data payload.
    pub fn data_as_ptr(&self) -> usize {
        let mut bytes = [0u8; core::mem::size_of::<usize>()];
        let copy_len = core::mem::size_of::<usize>().min(8);
        bytes[..copy_len].copy_from_slice(&self.data[..copy_len]);
        usize::from_ne_bytes(bytes)
    }

    /// Stores a pointer/address into the 8-byte data payload.
    pub fn set_data_ptr(&mut self, ptr: usize) {
        let bytes = ptr.to_ne_bytes();
        let copy_len = bytes.len().min(8);
        self.data[..copy_len].copy_from_slice(&bytes[..copy_len]);
    }

    /// Recursively unwraps any `VT_BYREF | VT_VARIANT` pointers until reaching a concrete Variant value.
    pub fn deref_variant(&self) -> &Self {
        let mut curr = self;
        while curr.vt == (VT_BYREF | VT_VARIANT) {
            let mut bytes = [0u8; core::mem::size_of::<usize>()];
            bytes.copy_from_slice(&curr.data[..core::mem::size_of::<usize>()]);
            let p = usize::from_ne_bytes(bytes) as *const Variant;
            if p.is_null() {
                break;
            }
            curr = unsafe { &*p };
        }
        curr
    }

    /// Fully resolves all levels of indirection (including `VT_BYREF | VT_VARIANT` and `VT_BYREF | VT_*`)
    /// into an owned, primitive ByVal `Variant`.
    pub fn to_byval(&self) -> Self {
        let concrete = self.deref_variant();
        if (concrete.vt & VT_BYREF) != 0 {
            let base_vt = concrete.vt & !VT_BYREF;
            let mut ptr_bytes = [0u8; core::mem::size_of::<usize>()];
            ptr_bytes.copy_from_slice(&concrete.data[..core::mem::size_of::<usize>()]);
            let raw_ptr = usize::from_ne_bytes(ptr_bytes);
            if raw_ptr == 0 {
                return Self::empty();
            }
            match base_vt {
                VT_I1 | VT_UI1 => {
                    let val = unsafe { *(raw_ptr as *const u8) };
                    let mut res = Self::empty();
                    res.vt = base_vt;
                    res.data[0] = val;
                    res
                }
                VT_I2 | VT_BOOL => {
                    let val = unsafe { *(raw_ptr as *const i16) };
                    let mut res = Self::empty();
                    res.vt = base_vt;
                    res.data[..2].copy_from_slice(&val.to_ne_bytes());
                    res
                }
                VT_I4 | VT_R4 => {
                    let val = unsafe { *(raw_ptr as *const i32) };
                    let mut res = Self::empty();
                    res.vt = base_vt;
                    res.data[..4].copy_from_slice(&val.to_ne_bytes());
                    res
                }
                VT_R8 | VT_CY | VT_DATE => {
                    let val = unsafe { *(raw_ptr as *const u64) };
                    let mut res = Self::empty();
                    res.vt = base_vt;
                    res.data[..8].copy_from_slice(&val.to_ne_bytes());
                    res
                }
                VT_BSTR => {
                    let bstr_ptr = unsafe { *(raw_ptr as *const crate::types::bstr::Bstr) };
                    if !bstr_ptr.is_null() {
                        let s = unsafe { crate::types::bstr_to_string(bstr_ptr) };
                        Self::from_bstr(crate::types::sys_alloc_string(&s))
                    } else {
                        Self::from_bstr(core::ptr::null_mut())
                    }
                }
                _ => *concrete,
            }
        } else {
            *concrete
        }
    }

    /// Extracts a 32-bit integer from this `Variant` if it holds an integer or boolean.
    ///
    /// Automatically converts `VT_I2` and `VT_BOOL` values into standard `i32`.
    ///
    /// # Returns
    /// `Some(i32)` if the variant holds an integer subtype, or `None` otherwise.
    pub fn as_i32(&self) -> Option<i32> {
        let target = self.deref_variant();
        match target.vt {
            VT_I2 => {
                let bytes = [target.data[0], target.data[1]];
                Some(i16::from_ne_bytes(bytes) as i32)
            }
            VT_I4 => {
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&target.data[..4]);
                Some(i32::from_ne_bytes(bytes))
            }
            VT_BOOL => {
                let bytes = [target.data[0], target.data[1]];
                Some(i16::from_ne_bytes(bytes) as i32)
            }
            VT_UI1 => Some(target.data[0] as i32),
            VT_R4 => {
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&target.data[..4]);
                let f = f32::from_ne_bytes(bytes) as f64;
                Some(crate::math::round_to_even(f) as i32)
            }
            VT_R8 => {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&target.data[..8]);
                let f = f64::from_ne_bytes(bytes);
                Some(crate::math::round_to_even(f) as i32)
            }
            VT_DECIMAL => {
                if let Some((val_96, scale, is_neg)) = target.as_decimal() {
                    let mut divisor = 1u128;
                    for _ in 0..scale {
                        divisor = divisor.saturating_mul(10);
                    }
                    let int_part = (val_96 / divisor) as i32;
                    let rem = val_96 % divisor;
                    let half = divisor / 2;
                    let round_up = rem > half || (rem == half && (int_part % 2 != 0));
                    let rounded = if round_up { int_part + 1 } else { int_part };
                    Some(if is_neg { -rounded } else { rounded })
                } else {
                    None
                }
            }
            VT_EMPTY => Some(0),
            VT_BSTR => {
                if let Some(b) = target.as_bstr() {
                    let s = unsafe { crate::types::bstr_to_string(b) };
                    s.trim().parse::<i32>().ok().or_else(|| {
                        s.trim().parse::<f64>().ok().map(|f| crate::math::round_to_even(f) as i32)
                    })
                } else {
                    None
                }
            }
            vt if vt == (VT_BYREF | VT_I2) => {
                let mut bytes = [0u8; core::mem::size_of::<usize>()];
                bytes.copy_from_slice(&target.data[..core::mem::size_of::<usize>()]);
                let p = usize::from_ne_bytes(bytes) as *const i16;
                if !p.is_null() {
                    unsafe { Some(*p as i32) }
                } else {
                    None
                }
            }
            vt if vt == (VT_BYREF | VT_I4) => {
                let mut bytes = [0u8; core::mem::size_of::<usize>()];
                bytes.copy_from_slice(&target.data[..core::mem::size_of::<usize>()]);
                let p = usize::from_ne_bytes(bytes) as *const i32;
                if !p.is_null() {
                    unsafe { Some(*p) }
                } else {
                    None
                }
            }
            vt if vt == (VT_BYREF | VT_BSTR) => {
                if let Some(b) = target.as_bstr() {
                    let s = unsafe { crate::types::bstr_to_string(b) };
                    s.trim().parse::<i32>().ok()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Converts this `Variant` into a signed 32-bit integer (`i32` / VB6 `Long`).
    pub fn to_i32(&self) -> Option<i32> {
        self.as_i32()
    }

    /// Creates a Boolean `Variant` (`VT_BOOL`).
    ///
    /// In VB6, True is represented as -1 (0xFFFF) and False as 0 (0x0000).
    ///
    /// # Arguments
    /// * `val` - Boolean value.
    ///
    /// # Returns
    /// Initialized `Variant`.
    pub fn from_bool(val: bool) -> Self {
        let mut v = Self::empty();
        v.vt = VT_BOOL;
        let bool_val: i16 = if val { -1 } else { 0 };
        let bytes = bool_val.to_ne_bytes();
        v.data[0] = bytes[0];
        v.data[1] = bytes[1];
        v
    }

    /// Creates a BSTR string `Variant` (`VT_BSTR`).
    ///
    /// # Arguments
    /// * `b` - Allocated BSTR string pointer.
    ///
    /// # Returns
    /// Initialized `Variant`.
    pub fn from_bstr(b: crate::types::bstr::Bstr) -> Self {
        let mut v = Self::empty();
        v.vt = VT_BSTR;
        let ptr_val = b as usize;
        let bytes = ptr_val.to_ne_bytes();
        let copy_len = bytes.len().min(8);
        v.data[..copy_len].copy_from_slice(&bytes[..copy_len]);
        v
    }

    /// Creates a Currency `Variant` (`VT_CY`).
    pub fn from_cy(cy: crate::types::currency::Currency) -> Self {
        let mut v = Self::empty();
        v.vt = VT_CY;
        let bytes = cy.0.to_ne_bytes();
        v.data[..8].copy_from_slice(&bytes);
        v
    }

    /// Extracts a `Currency` from this `Variant` if it holds `VT_CY`.
    pub fn as_cy(&self) -> Option<crate::types::currency::Currency> {
        if self.vt == VT_CY {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&self.data[..8]);
            Some(crate::types::currency::Currency(i64::from_ne_bytes(bytes)))
        } else {
            None
        }
    }

    /// Extracts a boolean value from this `Variant` if it holds a boolean or integer.
    ///
    /// # Returns
    /// `Some(bool)` if convertible, or `None` otherwise.
    pub fn as_bool(&self) -> Option<bool> {
        match self.vt {
            VT_BOOL | VT_I2 => {
                let bytes = [self.data[0], self.data[1]];
                Some(i16::from_ne_bytes(bytes) != 0)
            }
            VT_I4 => {
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&self.data[..4]);
                Some(i32::from_ne_bytes(bytes) != 0)
            }
            _ => None,
        }
    }

    /// Extracts a BSTR pointer from this `Variant` if `vt == VT_BSTR` or `VT_BYREF | VT_BSTR`.
    ///
    /// # Returns
    /// `Some(Bstr)` if this variant holds a string pointer, or `None` otherwise.
    pub fn as_bstr(&self) -> Option<crate::types::bstr::Bstr> {
        let target = self.deref_variant();
        if target.vt == VT_BSTR {
            let mut bytes = [0u8; core::mem::size_of::<usize>()];
            bytes.copy_from_slice(&target.data[..core::mem::size_of::<usize>()]);
            Some(usize::from_ne_bytes(bytes) as crate::types::bstr::Bstr)
        } else if target.vt == (VT_BYREF | VT_BSTR) {
            let mut bytes = [0u8; core::mem::size_of::<usize>()];
            bytes.copy_from_slice(&target.data[..core::mem::size_of::<usize>()]);
            let pbstr = usize::from_ne_bytes(bytes) as *const crate::types::bstr::Bstr;
            if !pbstr.is_null() {
                unsafe { Some(*pbstr) }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Coerces any variant into an `f64` number if compatible (numbers, booleans, or numeric strings).
    ///
    /// # Returns
    /// `Some(f64)` on successful coercion, or `None` otherwise.
    pub fn as_f64(&self) -> Option<f64> {
        let target = self.deref_variant();
        match target.vt {
            VT_I2 => {
                let bytes = [target.data[0], target.data[1]];
                Some(i16::from_ne_bytes(bytes) as f64)
            }
            VT_I4 => {
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&target.data[..4]);
                Some(i32::from_ne_bytes(bytes) as f64)
            }
            VT_R4 => {
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&target.data[..4]);
                Some(f32::from_ne_bytes(bytes) as f64)
            }
            VT_R8 | VT_DATE => {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&target.data[..8]);
                Some(f64::from_ne_bytes(bytes))
            }
            VT_BOOL => {
                let bytes = [target.data[0], target.data[1]];
                Some(if i16::from_ne_bytes(bytes) != 0 {
                    -1.0
                } else {
                    0.0
                })
            }
            VT_BSTR => {
                if let Some(b) = target.as_bstr() {
                    let s = unsafe { crate::types::bstr_to_string(b) };
                    s.trim().parse::<f64>().ok()
                } else {
                    None
                }
            }
            VT_UI1 => Some(target.data[0] as f64),
            VT_CY => target.as_cy().map(|c| c.to_f64()),
            VT_DECIMAL => {
                if let Some((val_96, scale, is_neg)) = target.as_decimal() {
                    let mut f = val_96 as f64;
                    if scale > 0 {
                        f /= 10f64.powi(scale as i32);
                    }
                    Some(if is_neg { -f } else { f })
                } else {
                    None
                }
            }
            VT_EMPTY => Some(0.0),
            _ => None,
        }
    }

    /// Converts this `Variant` into its standard VB6 string representation.
    ///
    /// # Returns
    /// An owned Rust `String`.
    pub fn to_string_representation(&self) -> String {
        let target = self.deref_variant();
        match target.vt {
            VT_EMPTY | VT_NULL => String::new(),
            VT_DECIMAL => {
                if let Some((val_96, scale, is_neg)) = target.as_decimal() {
                    let mut s = format!("{}", val_96);
                    if scale > 0 {
                        let scale_usize = scale as usize;
                        if s.len() <= scale_usize {
                            let zeros = scale_usize + 1 - s.len();
                            s = format!("{}{}", "0".repeat(zeros), s);
                        }
                        let insert_idx = s.len() - scale_usize;
                        s.insert(insert_idx, '.');
                    }
                    if is_neg {
                        format!("-{}", s)
                    } else {
                        s
                    }
                } else {
                    String::new()
                }
            }
            VT_CY => {
                if let Some(c) = target.as_cy() {
                    let whole = c.0 / 10000;
                    let frac = (c.0 % 10000).abs();
                    if frac == 0 {
                        format!("{}", whole)
                    } else {
                        let s = format!("{}.{:04}", whole, frac);
                        s.trim_end_matches('0').to_string()
                    }
                } else {
                    String::new()
                }
            }
            VT_I2 => {
                let bytes = [target.data[0], target.data[1]];
                format!("{}", i16::from_ne_bytes(bytes))
            }
            VT_I4 => {
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&target.data[..4]);
                format!("{}", i32::from_ne_bytes(bytes))
            }
            VT_R4 => {
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&target.data[..4]);
                let f = f32::from_ne_bytes(bytes);
                format_clean_float(f as f64)
            }
            VT_R8 => {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&target.data[..8]);
                let f = f64::from_ne_bytes(bytes);
                format_clean_float(f)
            }
            VT_BOOL => {
                let bytes = [target.data[0], target.data[1]];
                if i16::from_ne_bytes(bytes) != 0 {
                    "True".to_string()
                } else {
                    "False".to_string()
                }
            }
            VT_BSTR => {
                if let Some(b) = target.as_bstr() {
                    unsafe { crate::types::bstr_to_string(b) }
                } else {
                    String::new()
                }
            }
            vt if vt == (VT_BYREF | VT_BSTR) => {
                if let Some(b) = target.as_bstr() {
                    unsafe { crate::types::bstr_to_string(b) }
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        }
    }
}

/// Helper to format floating-point values cleanly without unnecessary scientific notation.
fn format_clean_float(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e12 {
        format!("{:.0}", f)
    } else {
        format!("{}", f)
    }
}
