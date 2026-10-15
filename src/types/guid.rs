//! COM GUID / IID / CLSID definitions.
//!
//! Visual Basic 6 and OLE Automation use 128-bit Globally Unique Identifiers (GUIDs)
//! to identify COM classes (`CLSID`) and interfaces (`IID`).

/// 128-bit COM Globally Unique Identifier (`GUID`).
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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
}

/// Interface Identifier (`IID`) alias for COM interfaces.
pub type Iid = Guid;

/// Class Identifier (`CLSID`) alias for COM coclasses.
pub type Clsid = Guid;
