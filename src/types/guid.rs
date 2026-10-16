//! COM GUID / IID / CLSID definitions.
//!
//! Visual Basic 6 and OLE Automation use 128-bit Globally Unique Identifiers (GUIDs)
//! to identify COM classes (`CLSID`) and interfaces (`IID`).

#![allow(non_upper_case_globals)]

/// 128-bit COM Globally Unique Identifier (`GUID`).
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Guid {
    /// First 32-bit component of the GUID.
    pub data1: u32,
    /// Second 16-bit component of the GUID.
    pub data2: u16,
    /// Third 16-bit component of the GUID.
    pub data3: u16,
    /// Final 8-byte array component of the GUID.
    pub data4: [u8; 8],
}

impl Guid {
    /// Constructs a constant `Guid` from its raw integer and byte array components.
    ///
    /// # Arguments
    /// * `data1` - 32-bit component.
    /// * `data2` - 16-bit component.
    /// * `data3` - 16-bit component.
    /// * `data4` - 8-byte array.
    ///
    /// # Returns
    /// An initialized `Guid`.
    pub const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self {
            data1,
            data2,
            data3,
            data4,
        }
    }

    /// Constructs a `Guid` from a 16-byte little-endian slice or array.
    pub fn from_bytes_le(bytes: [u8; 16]) -> Self {
        let d1 = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let d2 = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
        let d3 = u16::from_le_bytes(bytes[6..8].try_into().unwrap());
        let mut d4 = [0u8; 8];
        d4.copy_from_slice(&bytes[8..16]);
        Self::new(d1, d2, d3, d4)
    }

    /// Parses a GUID string in standard format, with or without curly braces:
    /// `"{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}"` or `"XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX"`.
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim().trim_start_matches('{').trim_end_matches('}');
        let parts: Vec<&str> = trimmed.split('-').collect();
        if parts.len() != 5 {
            return None;
        }

        let d1 = u32::from_str_radix(parts[0], 16).ok()?;
        let d2 = u16::from_str_radix(parts[1], 16).ok()?;
        let d3 = u16::from_str_radix(parts[2], 16).ok()?;

        if parts[3].len() != 4 || parts[4].len() != 12 {
            return None;
        }

        let d4_0 = u8::from_str_radix(&parts[3][0..2], 16).ok()?;
        let d4_1 = u8::from_str_radix(&parts[3][2..4], 16).ok()?;

        let mut d4 = [0u8; 8];
        d4[0] = d4_0;
        d4[1] = d4_1;
        for i in 0..6 {
            d4[2 + i] = u8::from_str_radix(&parts[4][i * 2..i * 2 + 2], 16).ok()?;
        }

        Some(Self::new(d1, d2, d3, d4))
    }

    /// Formats the GUID as a canonical string with uppercase hex and braces:
    /// `"{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}"`.
    pub fn to_string_with_braces(&self) -> String {
        format!(
            "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
            self.data1,
            self.data2,
            self.data3,
            self.data4[0],
            self.data4[1],
            self.data4[2],
            self.data4[3],
            self.data4[4],
            self.data4[5],
            self.data4[6],
            self.data4[7],
        )
    }
}

/// Interface Identifier (`IID`) alias for COM interfaces.
pub type Iid = Guid;

/// Class Identifier (`CLSID`) alias for COM coclasses.
pub type Clsid = Guid;

#[allow(non_upper_case_globals)]
/// Standard `IID_IUnknown`: `{00000000-0000-0000-C000-000000000046}`.
pub const IID_IUnknown: Iid = Guid::new(
    0x00000000,
    0x0000,
    0x0000,
    [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
);

/// Standard `IID_IClassFactory`: `{00000001-0000-0000-C000-000000000046}`.
pub const IID_IClassFactory: Iid = Guid::new(
    0x00000001,
    0x0000,
    0x0000,
    [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
);

/// Standard `IID_IDispatch`: `{00020400-0000-0000-C000-000000000046}`.
pub const IID_IDispatch: Iid = Guid::new(
    0x00020400,
    0x0000,
    0x0000,
    [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
);

/// Standard `CLSID_Dictionary` (`Scripting.Dictionary`): `{EE09B103-97E0-11CF-978F-00A02463E06F}`.
pub const CLSID_Dictionary: Clsid = Guid::new(
    0xEE09B103,
    0x97E0,
    0x11CF,
    [0x97, 0x8F, 0x00, 0xA0, 0x24, 0x63, 0xE0, 0x6F],
);

/// Standard `CLSID_FileSystemObject` (`Scripting.FileSystemObject`): `{0D43FE01-F093-11CF-8940-00A0C9054228}`.
pub const CLSID_FileSystemObject: Clsid = Guid::new(
    0x0D43FE01,
    0xF093,
    0x11CF,
    [0x89, 0x40, 0x00, 0xA0, 0xC9, 0x05, 0x42, 0x28],
);

/// Standard `CLSID_WshShell` (`WScript.Shell`): `{72C24DD5-D70A-438B-8A42-98424B88AFB8}`.
pub const CLSID_WshShell: Clsid = Guid::new(
    0x72C24DD5,
    0xD70A,
    0x438B,
    [0x8A, 0x42, 0x98, 0x42, 0x4B, 0x88, 0xAF, 0xB8],
);

/// Standard `CLSID_WshNetwork` (`WScript.Network`): `{093FF999-1EA0-4079-9525-9614C3504B74}`.
pub const CLSID_WshNetwork: Clsid = Guid::new(
    0x093FF999,
    0x1EA0,
    0x4079,
    [0x95, 0x25, 0x96, 0x14, 0xC3, 0x50, 0x4B, 0x74],
);

/// Standard `CLSID_Recordset` (`ADODB.Recordset`): `{00000535-0000-0010-8000-00AA006D2EA4}`.
pub const CLSID_Recordset: Clsid = Guid::new(
    0x00000535,
    0x0000,
    0x0010,
    [0x80, 0x00, 0x00, 0xAA, 0x00, 0x6D, 0x2E, 0xA4],
);

/// Standard `IID_IDictionary` (`Scripting.IDictionary`): `{42C642C1-97E1-11CF-978F-00A02463E06F}`.
pub const IID_IDictionary: Iid = Guid::new(
    0x42C642C1,
    0x97E1,
    0x11CF,
    [0x97, 0x8F, 0x00, 0xA0, 0x24, 0x63, 0xE0, 0x6F],
);

/// Standard `IID_NULL`: `{00000000-0000-0000-0000-000000000000}`.
pub const IID_NULL: Iid = Guid::new(0, 0, 0, [0; 8]);

/// Standard `IID_ITypeInfo`: `{00020401-0000-0000-C000-000000000046}`.
pub const IID_ITypeInfo: Iid = Guid::new(
    0x00020401,
    0x0000,
    0x0000,
    [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
);

