// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # cb-net: Networking, WebSocket & Display Wire Protocol Foundation
//!
//! Provides the binary wire protocol codec (`protocol`) and RFC 6455 WebSocket
//! transport definitions for streaming GDI/Window display frames (`0x02..0x07`)
//! and browser input events (`0x81..0x85`).

pub mod protocol;

pub use protocol::*;
