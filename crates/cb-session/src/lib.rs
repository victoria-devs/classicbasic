// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # cb-session: Multi-Tenant Session Broker & Container Reverse Proxy
//!
//! Provides orchestrator facilities for spawning and managing isolated `vb6-run --gui-web`
//! container/process instances per authenticated client session, health checking,
//! resource bounding, and zero-leakage sandboxed reverse proxying.

pub mod config;
pub mod manager;
pub mod proxy;
pub mod tenant;

pub use config::*;
pub use manager::*;
pub use proxy::*;
pub use tenant::*;
