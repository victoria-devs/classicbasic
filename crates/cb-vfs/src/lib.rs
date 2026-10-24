// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # cb-vfs: Virtual FileSystem, Virtual Registry & COM GUID Foundation
//!
//! Provides sandboxed Windows Registry emulation (`Registry` backed by `registry.json`),
//! 128-bit COM identifiers (`Guid`, `Clsid`, `Iid`), and virtualized Windows-to-POSIX
//! path resolution (`resolve_virtual_path`, `SandboxProvider`).

pub mod fs;
pub mod guid;
pub mod registry;

pub use fs::*;
pub use guid::*;
pub use registry::*;
