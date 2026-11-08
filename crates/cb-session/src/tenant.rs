// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Tenant Session Lifecycle & Process Isolation
//!
//! Represents a single sandboxed tenant session running an isolated VB6 executable
//! container or process instance with its own virtual filesystem and allocated port.

use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Instant;
use serde::{Deserialize, Serialize};

use crate::config::SessionConfig;

/// Operational status of an isolated tenant session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// Session configured; sandbox directory created, process not yet spawned.
    Initialized,
    /// Tenant process running and listening on its dedicated port.
    Running {
        pid: u32,
        port: u16,
        uptime_secs: u64,
    },
    /// Client disconnected, process still active waiting for reconnection.
    Disconnected,
    /// Process exited or was explicitly terminated.
    Terminated {
        exit_code: Option<i32>,
    },
    /// Process failed to start or crashed unexpectedly.
    Failed(String),
}

/// A lightweight, serializable snapshot of a tenant session's current state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantSessionSnapshot {
    pub session_id: String,
    pub status: SessionStatus,
    pub port: u16,
    pub sandbox_path: PathBuf,
    pub binary_path: PathBuf,
    pub memory_limit_bytes: usize,
    pub has_auth_token: bool,
}

/// Represents an active or managed tenant session instance.
pub struct TenantSession {
    /// Unique tenant identifier.
    pub session_id: String,
    /// Session configuration parameters.
    pub config: SessionConfig,
    /// Full path to the isolated sandbox directory for this tenant.
    pub sandbox_path: PathBuf,
    /// Handle to the spawned child process, if active.
    pub(crate) process: Option<Child>,
    /// System instant when this session was initialized.
    pub created_at: Instant,
    /// System instant when this session last handled traffic or heartbeat.
    pub last_activity: Instant,
    /// Cached operational status.
    pub status: SessionStatus,
}

impl TenantSession {
    /// Initializes a new tenant session and prepares its isolated directory structure.
    ///
    /// Sandboxes are established at:
    /// `<sandbox_root>/<session_id>/fs/`   - Virtual filesystem root
    /// `<sandbox_root>/<session_id>/tmp/`  - Temporary workspace
    /// `<sandbox_root>/<session_id>/vfs/`  - Win32 Virtual Registry & State
    pub fn new(config: SessionConfig) -> io::Result<Self> {
        let sandbox_path = config.sandbox_root.join(&config.session_id);

        let fs_dir = sandbox_path.join("fs");
        let tmp_dir = sandbox_path.join("tmp");
        let vfs_dir = sandbox_path.join("vfs");

        fs::create_dir_all(&fs_dir)?;
        fs::create_dir_all(&tmp_dir)?;
        fs::create_dir_all(&vfs_dir)?;

        let now = Instant::now();
        Ok(Self {
            session_id: config.session_id.clone(),
            config,
            sandbox_path,
            process: None,
            created_at: now,
            last_activity: now,
            status: SessionStatus::Initialized,
        })
    }

    /// Spawns the dedicated `vb6-run` runner process for this tenant session.
    pub fn spawn(&mut self, runner_binary: &Path) -> io::Result<()> {
        if self.process.is_some() {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                format!("Session '{}' is already running a process", self.session_id),
            ));
        }

        if !runner_binary.exists() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Runner binary not found at {}", runner_binary.display()),
            ));
        }

        let mut cmd = Command::new(runner_binary);
        cmd.arg("--gui-web");
        cmd.arg("--port").arg(self.config.port.to_string());
        cmd.arg(&self.config.binary_path);

        for arg in &self.config.extra_args {
            cmd.arg(arg);
        }

        // Set isolated execution working directory to tenant sandbox
        cmd.current_dir(&self.sandbox_path);

        // Configure tenant-scoped environment variables
        cmd.env("CB_TENANT_ID", &self.session_id);
        cmd.env("CB_SANDBOX_ROOT", &self.sandbox_path);
        cmd.env("CB_MEMORY_LIMIT", self.config.memory_limit_bytes.to_string());
        for (k, v) in &self.config.env_vars {
            cmd.env(k, v);
        }

        // Suppress or redirect standard streams
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                self.process = Some(child);
                self.status = SessionStatus::Running {
                    pid,
                    port: self.config.port,
                    uptime_secs: 0,
                };
                self.last_activity = Instant::now();
                Ok(())
            }
            Err(e) => {
                self.status = SessionStatus::Failed(e.to_string());
                Err(e)
            }
        }
    }

    /// Checks the health of the tenant process, updating internal status.
    pub fn check_health(&mut self) -> SessionStatus {
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let code = status.code();
                    self.status = SessionStatus::Terminated { exit_code: code };
                    self.process = None;
                }
                Ok(None) => {
                    let uptime = self.created_at.elapsed().as_secs();
                    if let SessionStatus::Running { pid, port, .. } = self.status {
                        self.status = SessionStatus::Running {
                            pid,
                            port,
                            uptime_secs: uptime,
                        };
                    }
                }
                Err(e) => {
                    self.status = SessionStatus::Failed(e.to_string());
                    self.process = None;
                }
            }
        }
        self.status.clone()
    }

    /// Refreshes the last active timestamp for idle tracking.
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Determines if the session process is currently alive.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.check_health(), SessionStatus::Running { .. })
    }

    /// Gracefully terminates the running process instance.
    pub fn terminate(&mut self) -> io::Result<()> {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
            let exit_status = child.wait()?;
            self.status = SessionStatus::Terminated {
                exit_code: exit_status.code(),
            };
        } else if self.status == SessionStatus::Initialized {
            self.status = SessionStatus::Terminated { exit_code: Some(0) };
        }
        Ok(())
    }

    /// Performs cleanup of sandbox directories on disk.
    pub fn cleanup_sandbox(&self) -> io::Result<()> {
        if self.sandbox_path.exists() {
            fs::remove_dir_all(&self.sandbox_path)?;
        }
        Ok(())
    }

    /// Resolves a sandboxed file path inside the tenant's isolated `fs/` directory.
    pub fn resolve_sandbox_path(&self, relative_path: &str) -> io::Result<PathBuf> {
        let clean = relative_path.trim_start_matches('/').trim_start_matches('\\');
        // Prevent path traversal escapes
        if clean.contains("..") {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                "Path traversal prohibited in tenant sandbox",
            ));
        }
        Ok(self.sandbox_path.join("fs").join(clean))
    }

    /// Writes data to an isolated tenant file.
    pub fn write_file(&self, relative_path: &str, data: &[u8]) -> io::Result<PathBuf> {
        let target = self.resolve_sandbox_path(relative_path)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, data)?;
        Ok(target)
    }

    /// Reads data from an isolated tenant file.
    pub fn read_file(&self, relative_path: &str) -> io::Result<Vec<u8>> {
        let target = self.resolve_sandbox_path(relative_path)?;
        fs::read(target)
    }

    /// Checks if a file exists within the tenant sandbox.
    pub fn file_exists(&self, relative_path: &str) -> bool {
        match self.resolve_sandbox_path(relative_path) {
            Ok(p) => p.exists(),
            Err(_) => false,
        }
    }

    /// Creates an immutable snapshot of this tenant session.
    pub fn snapshot(&self) -> TenantSessionSnapshot {
        TenantSessionSnapshot {
            session_id: self.session_id.clone(),
            status: self.status.clone(),
            port: self.config.port,
            sandbox_path: self.sandbox_path.clone(),
            binary_path: self.config.binary_path.clone(),
            memory_limit_bytes: self.config.memory_limit_bytes,
            has_auth_token: self.config.auth_token.is_some(),
        }
    }
}

impl Drop for TenantSession {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}
