// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Session Broker Configuration Definitions
//!
//! Provides configuration structs for isolated multi-tenant execution
//! environments, resource boundaries, and network reverse-proxy endpoints.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Configuration for an individual isolated tenant execution session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Unique tenant session identifier (e.g. "tenant-user123").
    pub session_id: String,
    /// Absolute or relative path to the VB6 executable binary.
    pub binary_path: PathBuf,
    /// Base filesystem root where the tenant's isolated sandbox lives.
    pub sandbox_root: PathBuf,
    /// Maximum virtual memory allocation limit in bytes (default: 128 MB).
    pub memory_limit_bytes: usize,
    /// Dedicated local port assigned for this tenant's runner instance.
    pub port: u16,
    /// Optional authentication bearer token for connection authorization.
    pub auth_token: Option<String>,
    /// Additional command-line arguments passed to the runner.
    pub extra_args: Vec<String>,
    /// Isolated environment variables exported to the child process.
    pub env_vars: HashMap<String, String>,
    /// Maximum allowable idle duration in seconds before reaping.
    pub max_idle_seconds: u64,
}

impl SessionConfig {
    /// Constructs a new `SessionConfig` with production defaults.
    pub fn new(session_id: impl Into<String>, binary_path: impl Into<PathBuf>, sandbox_root: impl Into<PathBuf>) -> Self {
        Self {
            session_id: session_id.into(),
            binary_path: binary_path.into(),
            sandbox_root: sandbox_root.into(),
            memory_limit_bytes: 128 * 1024 * 1024, // 128 MB default
            port: 0,                               // Dynamic allocation
            auth_token: None,
            extra_args: Vec::new(),
            env_vars: HashMap::new(),
            max_idle_seconds: 3600,                // 1 hour default
        }
    }

    /// Sets the assigned local port.
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Sets an authorization token for the tenant.
    pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    /// Sets the memory limit in bytes.
    pub fn with_memory_limit(mut self, bytes: usize) -> Self {
        self.memory_limit_bytes = bytes;
        self
    }
}

/// Global configuration for the multi-tenant session broker daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionBrokerConfig {
    /// Network address and port to bind the broker reverse proxy.
    pub listen_addr: SocketAddr,
    /// Base directory holding all tenant sandboxes.
    pub base_sandbox_dir: PathBuf,
    /// Default executable binary path if not explicitly provided per session.
    pub default_binary_path: PathBuf,
    /// Range of local ports available for allocation to tenant child processes.
    pub port_range_start: u16,
    pub port_range_end: u16,
    /// Whether authentication tokens are strictly required for session access.
    pub auth_required: bool,
    /// Optional master API secret for broker administrative operations.
    pub master_secret: Option<String>,
    /// Maximum concurrent active tenant sessions permitted.
    pub max_sessions: usize,
}

impl Default for SessionBrokerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8080".parse().unwrap(),
            base_sandbox_dir: PathBuf::from("/tmp/classicbasic-sandboxes"),
            default_binary_path: PathBuf::new(),
            port_range_start: 10000,
            port_range_end: 20000,
            auth_required: false,
            master_secret: None,
            max_sessions: 128,
        }
    }
}
