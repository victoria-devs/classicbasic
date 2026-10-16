//! Clean-room MSFT TypeLib Binary Parser and DISPID Introspection Cache.
//!
//! Provides parsing for Microsoft Type Library (MSFT) binary blocks embedded within
//! Win32 PE `.rsrc` sections or standalone `.tlb`/`.olb` files.
//!
//! # Architecture & Clean-Room Specification
//! - **Header (`MSFT_Header`)**:
//!   Magic `0x5446534D` ("MSFT"), version `0x00010002`, `nrtypeinfos` count,
//!   `NameOffset`, and flags (`0x100` signifies presence of help string DLL offset).
//! - **Segment Directory (`MSFT_SegDir`)**:
//!   Directory of 15 segments containing `pTypeInfoTab`, `pNametab`, `pGuidTab`,
//!   `pStringtab`, `pRefTab`, etc.
//! - **Type Information (`TypeInfo`)**:
//!   Encodes coclasses, interfaces, dispatch dual interfaces, enums, etc.
//!   Maps member names to DISPIDs and parameter signature metadata.
//! - **DISPID Cache (`DispidCache`)**:
//!   Fast in-memory index mapping `(IID, member_name) -> DISPID` and
//!   `(IID, DISPID) -> FuncDesc` for dynamic dispatch and late-binding engines.

use crate::types::Guid;
use std::collections::HashMap;
use std::path::Path;

/// Magic signature for MSFT type libraries: `"MSFT"` (`0x5446534D`).
pub const MSFT_MAGIC: u32 = 0x5446534D;

/// Type kind representation within COM Type Libraries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    Enum,
    Record,
    Module,
    Interface,
    Dispatch,
    CoClass,
    Alias,
    Union,
    Unknown(u32),
}

impl From<u32> for TypeKind {
    fn from(val: u32) -> Self {
        match val & 0xF {
            0 => TypeKind::Enum,
            1 => TypeKind::Record,
            2 => TypeKind::Module,
            3 => TypeKind::Interface,
            4 => TypeKind::Dispatch,
            5 => TypeKind::CoClass,
            6 => TypeKind::Alias,
            7 => TypeKind::Union,
            other => TypeKind::Unknown(other),
        }
    }
}

/// Invocation kind representation for method/property descriptors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvokeKind {
    Func,
    PropertyGet,
    PropertyPut,
    PropertyPutRef,
    Unknown(u32),
}

impl From<u32> for InvokeKind {
    fn from(val: u32) -> Self {
        match val {
            1 => InvokeKind::Func,
            2 => InvokeKind::PropertyGet,
            4 => InvokeKind::PropertyPut,
            8 => InvokeKind::PropertyPutRef,
            other => InvokeKind::Unknown(other),
        }
    }
}

/// In-memory descriptor of a TypeLib function / property method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncDesc {
    /// Member DISPID (Dispatch Identifier).
    pub memid: i32,
    /// Function / property name.
    pub name: String,
    /// Invocation kind (Method call, Property Get, Property Put, etc.).
    pub inv_kind: InvokeKind,
    /// Offset in the virtual function table (VTable).
    pub vtable_offset: u16,
    /// Number of parameters accepted by the method.
    pub c_params: u16,
}

/// In-memory descriptor of a TypeLib variable / constant member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarDesc {
    /// Member DISPID or variable ID.
    pub memid: i32,
    /// Variable or constant name.
    pub name: String,
    /// Offset in the data table or VTable.
    pub vtable_offset: u16,
}

/// In-memory representation of a COM TypeInfo (`ITypeInfo`).
#[derive(Debug, Clone)]
pub struct TypeInfo {
    /// Type name (e.g. `Dictionary`, `IDictionary`, `FileSystemObject`).
    pub name: String,
    /// Interface ID or Class ID (GUID).
    pub guid: Option<Guid>,
    /// Classification of this type (Interface, Dispatch, CoClass, etc.).
    pub type_kind: TypeKind,
    /// Number of functions / methods exposed by the type.
    pub c_funcs: u16,
    /// Number of variables / constants exposed by the type.
    pub c_vars: u16,
    /// Number of implemented or inherited interfaces.
    pub c_impl_types: u16,
    /// Total virtual function table size in bytes.
    pub cb_size_vft: u16,
    /// Array of method and property descriptors.
    pub funcs: Vec<FuncDesc>,
    /// Array of variable and constant descriptors.
    pub vars: Vec<VarDesc>,
}

/// In-memory representation of a parsed COM Type Library (`ITypeLib`).
#[derive(Debug, Clone)]
pub struct TypeLib {
    /// Type library identifier name (e.g. `Scripting`).
    pub name: String,
    /// Type library GUID (LibID).
    pub guid: Option<Guid>,
    /// Locale ID (LCID).
    pub lcid: u32,
    /// Major version number.
    pub version_major: u16,
    /// Minor version number.
    pub version_minor: u16,
    /// All TypeInfos contained within this library.
    pub type_infos: Vec<TypeInfo>,
}

impl TypeLib {
    /// Finds a TypeInfo by case-insensitive name.
    pub fn get_type_info_by_name(&self, name: &str) -> Option<&TypeInfo> {
        let name_lower = name.to_ascii_lowercase();
        self.type_infos
            .iter()
            .find(|ti| ti.name.to_ascii_lowercase() == name_lower)
    }

    /// Finds a TypeInfo by its GUID (IID or CLSID).
    pub fn get_type_info_by_guid(&self, guid: &Guid) -> Option<&TypeInfo> {
        self.type_infos
            .iter()
            .find(|ti| ti.guid.as_ref() == Some(guid))
    }
}

/// High-performance cache for DISPID lookups and method introspection.
#[derive(Debug, Clone, Default)]
pub struct DispidCache {
    /// Lookup index mapping `(IID, lowercase_member_name) -> DISPID`.
    by_name: HashMap<(Guid, String), i32>,
    /// Reverse lookup index mapping `(IID, DISPID) -> FuncDesc`.
    by_id: HashMap<(Guid, i32), FuncDesc>,
}

impl DispidCache {
    /// Creates an empty DISPID cache.
    pub fn new() -> Self {
        Self {
            by_name: HashMap::new(),
            by_id: HashMap::new(),
        }
    }

    /// Indexes all types and methods from a parsed `TypeLib`.
    pub fn register_typelib(&mut self, typelib: &TypeLib) {
        for info in &typelib.type_infos {
            if let Some(guid) = &info.guid {
                for func in &info.funcs {
                    let key = (*guid, func.name.to_ascii_lowercase());
                    self.by_name.insert(key, func.memid);
                    self.by_id.insert((*guid, func.memid), func.clone());
                }
            }
        }
    }

    /// Looks up a DISPID by IID and case-insensitive member name.
    pub fn lookup_dispid(&self, iid: &Guid, name: &str) -> Option<i32> {
        self.by_name
            .get(&(*iid, name.to_ascii_lowercase()))
            .copied()
    }

    /// Looks up function metadata by IID and DISPID.
    pub fn lookup_func(&self, iid: &Guid, dispid: i32) -> Option<&FuncDesc> {
        self.by_id.get(&(*iid, dispid))
    }
}

// ----------------------------------------------------------------------------
// Clean-Room MSFT Binary Parser Implementation
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct SegmentRef {
    offset: i32,
    length: i32,
}

/// Parses an in-memory slice containing raw MSFT TypeLib binary data.
pub fn parse_msft_bytes(data: &[u8]) -> Result<TypeLib, String> {
    if data.len() < 84 {
        return Err("Data too short for MSFT header".to_string());
    }

    let magic1 = u32::from_le_bytes(data[0..4].try_into().unwrap());
    if magic1 != MSFT_MAGIC {
        return Err(format!("Invalid MSFT magic signature: 0x{:08X}", magic1));
    }

    let posguid = i32::from_le_bytes(data[8..12].try_into().unwrap());
    let lcid = u32::from_le_bytes(data[12..16].try_into().unwrap());
    let varflags = u32::from_le_bytes(data[20..24].try_into().unwrap());
    let version = u32::from_le_bytes(data[24..28].try_into().unwrap());
    let version_major = (version & 0xFFFF) as u16;
    let version_minor = ((version >> 16) & 0xFFFF) as u16;

    let nrtypeinfos = u32::from_le_bytes(data[32..36].try_into().unwrap()) as usize;
    let name_offset = i32::from_le_bytes(data[56..60].try_into().unwrap());

    // Check optional helpstring dll offset
    let mut hdr_len = 84;
    if (varflags & 0x100) != 0 {
        hdr_len += 4;
    }

    let segdir_offset = hdr_len + nrtypeinfos * 4;
    if data.len() < segdir_offset + 15 * 16 {
        return Err("Data too short for MSFT SegDir table".to_string());
    }

    // Read 15 segments
    let mut segments = Vec::with_capacity(15);
    for i in 0..15 {
        let seg_pos = segdir_offset + i * 16;
        let off = i32::from_le_bytes(data[seg_pos..seg_pos + 4].try_into().unwrap());
        let len = i32::from_le_bytes(data[seg_pos + 4..seg_pos + 8].try_into().unwrap());
        segments.push(SegmentRef {
            offset: off,
            length: len,
        });
    }

    let p_typeinfotab = segments[0];
    let p_guidtab = segments[5];
    let p_nametab = segments[7];

    let read_name = |name_off: i32| -> String {
        if name_off < 0 || p_nametab.offset < 0 {
            return String::new();
        }
        let pos = (p_nametab.offset + name_off) as usize;
        if pos + 12 > data.len() {
            return String::new();
        }
        let len = data[pos + 8] as usize;
        if pos + 12 + len > data.len() {
            return String::new();
        }
        String::from_utf8_lossy(&data[pos + 12..pos + 12 + len]).into_owned()
    };

    let read_guid = |guid_off: i32| -> Option<Guid> {
        if guid_off < 0 || p_guidtab.offset < 0 {
            return None;
        }
        let pos = (p_guidtab.offset + guid_off) as usize;
        if pos + 16 > data.len() {
            return None;
        }
        let bytes: [u8; 16] = data[pos..pos + 16].try_into().ok()?;
        Some(Guid::from_bytes_le(bytes))
    };

    let lib_name = read_name(name_offset);
    let lib_guid = read_guid(posguid);

    // Parse TypeInfos
    let mut type_infos = Vec::with_capacity(nrtypeinfos);

    for i in 0..nrtypeinfos {
        let t_pos = (p_typeinfotab.offset + (i as i32) * 100) as usize;
        if t_pos + 100 > data.len() {
            break;
        }

        let typekind_raw = u32::from_le_bytes(data[t_pos..t_pos + 4].try_into().unwrap());
        let memoffset = i32::from_le_bytes(data[t_pos + 4..t_pos + 8].try_into().unwrap());
        let c_element = u32::from_le_bytes(data[t_pos + 24..t_pos + 28].try_into().unwrap());
        let c_funcs = (c_element & 0xFFFF) as u16;
        let c_vars = ((c_element >> 16) & 0xFFFF) as u16;

        let t_posguid = i32::from_le_bytes(data[t_pos + 44..t_pos + 48].try_into().unwrap());
        let t_name_offset = i32::from_le_bytes(data[t_pos + 52..t_pos + 56].try_into().unwrap());

        let c_impl_types = u16::from_le_bytes(data[t_pos + 76..t_pos + 78].try_into().unwrap());
        let cb_size_vft = u16::from_le_bytes(data[t_pos + 78..t_pos + 80].try_into().unwrap());

        let t_name = read_name(t_name_offset);
        let t_guid = read_guid(t_posguid);
        let t_kind = TypeKind::from(typekind_raw);

        let mut funcs = Vec::with_capacity(c_funcs as usize);

        // If TypeInfo contains functions, parse the function table at memoffset
        if c_funcs > 0 && memoffset >= 0 && (memoffset as usize) < data.len() {
            let m_pos = memoffset as usize;
            if m_pos + 4 <= data.len() {
                let dispid_table_rel =
                    i32::from_le_bytes(data[m_pos..m_pos + 4].try_into().unwrap());
                if dispid_table_rel > 0 {
                    let d_pos = m_pos + dispid_table_rel as usize;
                    let num_funcs = c_funcs as usize;
                    // Check if dispid table and name offsets table exist
                    if d_pos + 4 + num_funcs * 8 <= data.len() {
                        let dispids_start = d_pos + 4;
                        let names_start = d_pos + 4 + num_funcs * 4;

                        // Also parse sequential MSFT_FuncRecord structures
                        let mut rec_ptr = m_pos + 4;

                        for f_idx in 0..num_funcs {
                            let memid = i32::from_le_bytes(
                                data[dispids_start + f_idx * 4..dispids_start + (f_idx + 1) * 4]
                                    .try_into()
                                    .unwrap(),
                            );
                            let f_name_off = i32::from_le_bytes(
                                data[names_start + f_idx * 4..names_start + (f_idx + 1) * 4]
                                    .try_into()
                                    .unwrap(),
                            );
                            let func_name = read_name(f_name_off);

                            let mut vtable_offset = 0u16;
                            let mut inv_kind = InvokeKind::Func;
                            let mut c_params = 0u16;

                            if rec_ptr + 24 <= data.len() {
                                let rec_size_raw = u32::from_le_bytes(
                                    data[rec_ptr..rec_ptr + 4].try_into().unwrap(),
                                );
                                let rec_size = (rec_size_raw & 0xFFFF) as usize;

                                let vt_off_raw = u32::from_le_bytes(
                                    data[rec_ptr + 12..rec_ptr + 16].try_into().unwrap(),
                                );
                                vtable_offset = (vt_off_raw & 0xFFFF) as u16;

                                let flags_raw = u32::from_le_bytes(
                                    data[rec_ptr + 16..rec_ptr + 20].try_into().unwrap(),
                                );
                                let inv_raw = (flags_raw >> 3) & 0xF;
                                inv_kind = if inv_raw != 0 {
                                    InvokeKind::from(inv_raw)
                                } else {
                                    InvokeKind::Func
                                };

                                let params_raw = u32::from_le_bytes(
                                    data[rec_ptr + 20..rec_ptr + 24].try_into().unwrap(),
                                );
                                c_params = (params_raw & 0xFFFF) as u16;

                                if rec_size >= 24 {
                                    rec_ptr += rec_size;
                                } else {
                                    rec_ptr += 24;
                                }
                            }

                            funcs.push(FuncDesc {
                                memid,
                                name: func_name,
                                inv_kind,
                                vtable_offset,
                                c_params,
                            });
                        }
                    }
                }
            }
        }

        type_infos.push(TypeInfo {
            name: t_name,
            guid: t_guid,
            type_kind: t_kind,
            c_funcs,
            c_vars,
            c_impl_types,
            cb_size_vft,
            funcs,
            vars: Vec::new(),
        });
    }

    Ok(TypeLib {
        name: lib_name,
        guid: lib_guid,
        lcid,
        version_major,
        version_minor,
        type_infos,
    })
}

/// Locates and extracts the raw MSFT TypeLib byte block from a PE32 executable or DLL.
pub fn extract_typelib_bytes_from_pe(data: &[u8]) -> Result<&[u8], String> {
    const SIG: [u8; 8] = [0x4D, 0x53, 0x46, 0x54, 0x02, 0x00, 0x01, 0x00];

    // Scan for MSFT header signature
    if let Some(pos) = data.windows(SIG.len()).position(|window| window == SIG) {
        return Ok(&data[pos..]);
    }

    Err("MSFT TypeLib resource signature not found in PE binary".to_string())
}

/// Loads and parses a TypeLib from a file path (either standalone `.tlb` or in-process DLL/OCX).
pub fn parse_from_file(path: impl AsRef<Path>) -> Result<TypeLib, String> {
    let bytes = std::fs::read(path.as_ref())
        .map_err(|e| format!("Failed to read file {}: {}", path.as_ref().display(), e))?;

    if bytes.starts_with(b"MZ") {
        let typelib_slice = extract_typelib_bytes_from_pe(&bytes)?;
        parse_msft_bytes(typelib_slice)
    } else {
        parse_msft_bytes(&bytes)
    }
}
