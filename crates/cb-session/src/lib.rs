// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # cb-session: Multi-Tenant Session Broker & Sandboxed Orchestrator Foundation
//!
//! Manages isolated `vb6-run` tenant processes, per-session virtual filesystems (`cb-vfs`),
//! and WebSocket display/input multiplexing (`cb-net`).

use std::path::PathBuf;

/// Configuration for an isolated tenant execution session.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub session_id: String,
    pub binary_path: PathBuf,
    pub sandbox_root: PathBuf,
    pub memory_limit_bytes: usize,
}
