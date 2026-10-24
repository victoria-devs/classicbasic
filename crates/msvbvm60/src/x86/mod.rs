// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Native 32-bit x86 Execution Engine re-exported from `cb-x86` plus MSVBVM60 IAT Ordinal Bridge.

pub mod bridge;

pub use bridge::*;
pub use cb_x86::*;
