// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # cb-win32: Win32 OS Subsystem, Windowing & GDI Foundation
//!
//! Provides Win32 API emulation (`KERNEL32.DLL`, `ADVAPI32.DLL`, `USER32.DLL`, `GDI32.DLL`),
//! along with the Milestone 3 Window Manager (`user32`) and GDI 32-bit RGBA software
//! rasterizer (`gdi32`) foundations.

#![allow(non_snake_case, clippy::too_many_arguments)]

pub mod gdi32;
pub mod handles;
pub mod shims;
pub mod user32;

pub use gdi32::*;
pub use handles::*;
pub use shims::*;
pub use user32::*;
