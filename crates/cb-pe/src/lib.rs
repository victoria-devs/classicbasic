// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # cb-pe: PE32 Binary Format & VB6 Metadata Foundation
//!
//! Provides Portable Executable (PE32) structural headers (`ImageDosHeader`, `ImageFileHeader`,
//! `ImageOptionalHeader32`, `ImageSectionHeader`, `ImageImportDescriptor`, `ImageBaseRelocation`),
//! Visual Basic 6.0 executable metadata structures (`VBHeader`, `ProjectInfo`, `ObjectTable`),
//! and the canonical 635-entry `MSVBVM60.DLL` ordinal mapping (`msvbvm60_name_to_ordinal`).

pub mod headers;
pub mod ordinals;
pub mod rsrc;
pub mod vb_header;

pub use headers::*;
pub use ordinals::*;
pub use rsrc::*;
pub use vb_header::*;
