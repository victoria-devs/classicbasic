// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! PE32 `.rsrc` Resource Directory Traversal & Resource Types.

/// Standard Win32 Resource Types.
pub const RT_CURSOR: u32 = 1;
pub const RT_BITMAP: u32 = 2;
pub const RT_ICON: u32 = 3;
pub const RT_MENU: u32 = 4;
pub const RT_DIALOG: u32 = 5;
pub const RT_STRING: u32 = 6;
pub const RT_FONTDIR: u32 = 7;
pub const RT_FONT: u32 = 8;
pub const RT_ACCELERATOR: u32 = 9;
pub const RT_RCDATA: u32 = 10;
pub const RT_MESSAGETABLE: u32 = 11;
pub const RT_GROUP_CURSOR: u32 = 12;
pub const RT_GROUP_ICON: u32 = 14;
pub const RT_VERSION: u32 = 16;
pub const RT_MANIFEST: u32 = 24;

/// PE32 Resource Directory Table (`IMAGE_RESOURCE_DIRECTORY`).
#[repr(C, packed)]
#[derive(Debug, Copy, Clone, Default)]
pub struct ImageResourceDirectory {
    pub characteristics: u32,
    pub time_date_stamp: u32,
    pub major_version: u16,
    pub minor_version: u16,
    pub number_of_named_entries: u16,
    pub number_of_id_entries: u16,
}

/// PE32 Resource Directory Entry (`IMAGE_RESOURCE_DIRECTORY_ENTRY`).
#[repr(C, packed)]
#[derive(Debug, Copy, Clone, Default)]
pub struct ImageResourceDirectoryEntry {
    pub name_or_id: u32,
    pub offset_to_data_or_directory: u32,
}

impl ImageResourceDirectoryEntry {
    /// Returns true if this entry points to a child resource subdirectory.
    pub fn is_directory(&self) -> bool {
        (self.offset_to_data_or_directory & 0x8000_0000) != 0
    }

    /// Returns the offset relative to the start of the resource section.
    pub fn offset(&self) -> usize {
        (self.offset_to_data_or_directory & 0x7FFF_FFFF) as usize
    }

    /// Returns true if the entry identifier is a string name (high bit set).
    pub fn is_named(&self) -> bool {
        (self.name_or_id & 0x8000_0000) != 0
    }

    /// Returns the numerical integer ID.
    pub fn id(&self) -> u32 {
        self.name_or_id & 0x7FFF_FFFF
    }
}

/// PE32 Resource Data Entry (`IMAGE_RESOURCE_DATA_ENTRY`).
#[repr(C, packed)]
#[derive(Debug, Copy, Clone, Default)]
pub struct ImageResourceDataEntry {
    pub offset_to_data: u32,
    pub size: u32,
    pub code_page: u32,
    pub reserved: u32,
}

/// A parsed resource entry extracted from a PE32 `.rsrc` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedResource {
    pub type_id: u32,
    pub name_id: u32,
    pub language_id: u32,
    pub rva: u32,
    pub size: u32,
}

/// Traverses a PE32 resource section bytes and returns all leaf resource descriptors.
pub fn parse_resource_directory(rsrc_bytes: &[u8], _rsrc_rva: u32) -> Vec<ParsedResource> {
    let mut resources = Vec::new();
    let total_len = rsrc_bytes.len();
    if total_len < std::mem::size_of::<ImageResourceDirectory>() {
        return resources;
    }

    let root_hdr = unsafe {
        core::ptr::read_unaligned(rsrc_bytes.as_ptr() as *const ImageResourceDirectory)
    };
    let num_root_entries = (root_hdr.number_of_named_entries + root_hdr.number_of_id_entries) as usize;
    let mut root_entry_offset = std::mem::size_of::<ImageResourceDirectory>();

    for _ in 0..num_root_entries {
        if root_entry_offset + std::mem::size_of::<ImageResourceDirectoryEntry>() > total_len {
            break;
        }
        let root_entry = unsafe {
            core::ptr::read_unaligned(
                rsrc_bytes.as_ptr().add(root_entry_offset) as *const ImageResourceDirectoryEntry,
            )
        };
        root_entry_offset += std::mem::size_of::<ImageResourceDirectoryEntry>();

        let type_id = root_entry.id();
        if !root_entry.is_directory() {
            continue;
        }

        // Level 2: Name / ID Directory
        let lvl2_dir_offset = root_entry.offset();
        if lvl2_dir_offset + std::mem::size_of::<ImageResourceDirectory>() > total_len {
            continue;
        }
        let lvl2_hdr = unsafe {
            core::ptr::read_unaligned(
                rsrc_bytes.as_ptr().add(lvl2_dir_offset) as *const ImageResourceDirectory,
            )
        };
        let num_lvl2_entries = (lvl2_hdr.number_of_named_entries + lvl2_hdr.number_of_id_entries) as usize;
        let mut lvl2_entry_offset = lvl2_dir_offset + std::mem::size_of::<ImageResourceDirectory>();

        for _ in 0..num_lvl2_entries {
            if lvl2_entry_offset + std::mem::size_of::<ImageResourceDirectoryEntry>() > total_len {
                break;
            }
            let lvl2_entry = unsafe {
                core::ptr::read_unaligned(
                    rsrc_bytes.as_ptr().add(lvl2_entry_offset) as *const ImageResourceDirectoryEntry,
                )
            };
            lvl2_entry_offset += std::mem::size_of::<ImageResourceDirectoryEntry>();

            let name_id = lvl2_entry.id();
            if !lvl2_entry.is_directory() {
                continue;
            }

            // Level 3: Language Directory
            let lvl3_dir_offset = lvl2_entry.offset();
            if lvl3_dir_offset + std::mem::size_of::<ImageResourceDirectory>() > total_len {
                continue;
            }
            let lvl3_hdr = unsafe {
                core::ptr::read_unaligned(
                    rsrc_bytes.as_ptr().add(lvl3_dir_offset) as *const ImageResourceDirectory,
                )
            };
            let num_lvl3_entries = (lvl3_hdr.number_of_named_entries + lvl3_hdr.number_of_id_entries) as usize;
            let mut lvl3_entry_offset = lvl3_dir_offset + std::mem::size_of::<ImageResourceDirectory>();

            for _ in 0..num_lvl3_entries {
                if lvl3_entry_offset + std::mem::size_of::<ImageResourceDirectoryEntry>() > total_len {
                    break;
                }
                let lvl3_entry = unsafe {
                    core::ptr::read_unaligned(
                        rsrc_bytes.as_ptr().add(lvl3_entry_offset) as *const ImageResourceDirectoryEntry,
                    )
                };
                lvl3_entry_offset += std::mem::size_of::<ImageResourceDirectoryEntry>();

                let language_id = lvl3_entry.id();
                if lvl3_entry.is_directory() {
                    continue;
                }

                // Data Entry
                let data_entry_offset = lvl3_entry.offset();
                if data_entry_offset + std::mem::size_of::<ImageResourceDataEntry>() > total_len {
                    continue;
                }
                let data_entry = unsafe {
                    core::ptr::read_unaligned(
                        rsrc_bytes.as_ptr().add(data_entry_offset) as *const ImageResourceDataEntry,
                    )
                };

                resources.push(ParsedResource {
                    type_id,
                    name_id,
                    language_id,
                    rva: data_entry.offset_to_data,
                    size: data_entry.size,
                });
            }
        }
    }

    resources
}
