//! Visual Basic 6.0 internal header structures.
//!
//! When a native-compiled Visual Basic 6.0 executable starts, its entry point performs:
//! ```text
//!     push offset VBHeader
//!     call ThunRTMain
//!     ret
//! ```
//! The `VBHeader` struct contains the runtime version, language DLL references,
//! pointer to `Sub Main` (if any), thread configuration, and pointers to the
//! project tables (`ProjectInfo`, `ObjectTable`, GUI descriptions, etc.).

/// Magic signature present in all VB5 and VB6 executables: "VB5!" (0x21354256).
pub const VB_MAGIC: &[u8; 4] = b"VB5!";

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct VBHeader {
    /// Magic constant: ASCII "VB5!" (0x21354256)
    pub sz_magic: [u8; 4],
    /// Runtime build version (e.g. 0x0000 or specific build number)
    pub runtime_build_version: u16,
    /// Language DLL string (14 bytes, e.g. "*\G{...}" or null-padded)
    pub sz_language_dll: [u8; 14],
    /// Secondary language DLL string (14 bytes)
    pub sz_sec_lang_dll: [u8; 14],
    /// Runtime revision
    pub runtime_revision: u16,
    /// Primary Locale Identifier (LCID)
    pub lcid: u32,
    /// Secondary Locale Identifier
    pub sec_lcid: u32,
    /// Pointer to Sub Main address (0 if startup object is a Form)
    pub lp_sub_main: u32,
    /// Pointer to `ProjectInfo` structure
    pub lp_project_data: u32,
    /// Internal MDL pool cache 1
    pub mdl_int_pool_cache: u32,
    /// Internal MDL pool cache 2
    pub mdl_int_pool_cache2: u32,
    /// Thread flags
    pub dw_thread_flags: u32,
    /// Thread count
    pub dw_thread_count: u32,
    /// Number of forms in the project
    pub w_forms_count: u16,
    /// External component count
    pub w_external_component_count: u16,
    /// Thunk count
    pub dw_thunk_count: u32,
    /// Pointer to GUI Table
    pub lp_gui_table: u32,
    /// Pointer to IDE Data
    pub lp_ide_data: u32,
    /// Pointer to Project Dependencies
    pub lp_project_dependencies: u32,
    /// Pointer to Import Table
    pub lp_import_table: u32,
    /// Pointer to Export Table
    pub lp_export_table: u32,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct ProjectInfo {
    /// Version flag (typically 0x500 for VB5/VB6)
    pub dw_version: u32,
    /// Pointer to `ObjectTable`
    pub lp_object_table: u32,
    /// Reserved/null
    pub dw_null: u32,
    /// Pointer to code depot
    pub lp_code_depot: u32,
    /// Pointer to project data
    pub lp_project_data: u32,
    /// Pointer to GUID Table
    pub lp_guid_table: u32,
    /// Pointer to External Components Table
    pub lp_external_table: u32,
    /// Pointer to ASCII string: Project Title
    pub lp_project_title: u32,
    /// LCID
    pub lcid: u32,
    /// Secondary LCID
    pub lcid2: u32,
    /// Pointer to ASCII string: Help file path
    pub lp_help_file: u32,
    /// Pointer to ASCII string: Project Name
    pub lp_project_name: u32,
    /// Pointer to ASCII string: Project Description
    pub lp_project_description: u32,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct ObjectTable {
    pub lp_null1: u32,
    pub lp_code_depot: u32,
    pub lp_object_info_table: u32,
    pub dw_null2: u32,
    pub lp_gui_table: u32,
    pub lp_guid_table: u32,
    pub lp_object_descriptors: u32,
    pub dw_total_objects_count: u32,
    pub dw_compiled_objects_count: u32,
    pub dw_objects_in_use_count: u32,
    pub lp_object_array: u32,
}

impl VBHeader {
    /// Validates whether the magic bytes correspond to "VB5!".
    pub fn is_valid(&self) -> bool {
        &self.sz_magic == VB_MAGIC
    }
}
