// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Multi-Tenant Session Broker Manager
//!
//! Orchestrates the creation, lifecycle management, health monitoring,
//! and resource reclamation for sandboxed tenant sessions across the system.

use std::collections::HashMap;
use std::io::{self, Error, ErrorKind};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::config::{SessionBrokerConfig, SessionConfig};
use crate::tenant::{SessionStatus, TenantSession, TenantSessionSnapshot};

/// Thread-safe manager responsible for all tenant sessions.
#[derive(Clone)]
pub struct SessionManager {
    inner: Arc<SessionManagerInner>,
}

struct SessionManagerInner {
    config: SessionBrokerConfig,
    runner_binary: PathBuf,
    sessions: Mutex<HashMap<String, TenantSession>>,
}

impl SessionManager {
    /// Constructs a new `SessionManager` with the specified broker configuration
    /// and runner binary path.
    pub fn new(config: SessionBrokerConfig, runner_binary: PathBuf) -> Self {
        Self {
            inner: Arc::new(SessionManagerInner {
                config,
                runner_binary,
                sessions: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Allocates an available local TCP port within the configured range.
    pub fn allocate_port(&self) -> io::Result<u16> {
        let sessions = self.inner.sessions.lock().unwrap();
        let used_ports: Vec<u16> = sessions
            .values()
            .map(|s| s.config.port)
            .filter(|&p| p > 0)
            .collect();

        // Try binding ports in the configured range
        for port in self.inner.config.port_range_start..=self.inner.config.port_range_end {
            if used_ports.contains(&port) {
                continue;
            }
            if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) {
                drop(listener);
                return Ok(port);
            }
        }

        // Fallback: request an ephemeral OS-assigned port
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        drop(listener);
        Ok(port)
    }

    /// Creates and registers a new tenant session.
    pub fn create_session(
        &self,
        session_id: &str,
        binary_path: Option<&Path>,
        token: Option<String>,
    ) -> io::Result<TenantSessionSnapshot> {
        let mut sessions = self.inner.sessions.lock().unwrap();

        if sessions.len() >= self.inner.config.max_sessions {
            return Err(Error::new(
                ErrorKind::Other,
                "Maximum concurrent tenant sessions reached",
            ));
        }

        if sessions.contains_key(session_id) {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                format!("Session '{}' already exists", session_id),
            ));
        }

        let used_ports: Vec<u16> = sessions.values().map(|s| s.config.port).collect();
        let mut assigned_port = 0u16;
        for port in self.inner.config.port_range_start..=self.inner.config.port_range_end {
            if !used_ports.contains(&port) {
                if let Ok(l) = TcpListener::bind(("127.0.0.1", port)) {
                    drop(l);
                    assigned_port = port;
                    break;
                }
            }
        }
        if assigned_port == 0 {
            let l = TcpListener::bind("127.0.0.1:0")?;
            assigned_port = l.local_addr()?.port();
        }

        let bin = binary_path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.inner.config.default_binary_path.clone());

        let mut config = SessionConfig::new(
            session_id,
            bin,
            self.inner.config.base_sandbox_dir.clone(),
        );
        config.port = assigned_port;
        config.auth_token = token;

        let tenant = TenantSession::new(config)?;
        let snapshot = tenant.snapshot();
        sessions.insert(session_id.to_string(), tenant);

        Ok(snapshot)
    }

    /// Spawns the underlying process for an initialized session.
    pub fn start_session(&self, session_id: &str) -> io::Result<TenantSessionSnapshot> {
        let mut sessions = self.inner.sessions.lock().unwrap();
        let session = sessions.get_mut(session_id).ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!("Session '{}' not found", session_id),
            )
        })?;

        session.spawn(&self.inner.runner_binary)?;
        Ok(session.snapshot())
    }

    /// Returns a snapshot of the session state, or None if not registered.
    pub fn get_session(&self, session_id: &str) -> Option<TenantSessionSnapshot> {
        let mut sessions = self.inner.sessions.lock().unwrap();
        sessions.get_mut(session_id).map(|s| {
            s.check_health();
            s.snapshot()
        })
    }

    /// Authenticates a token for access to a given session.
    pub fn authenticate(&self, session_id: &str, token: Option<&str>) -> bool {
        // If master secret is provided and matches, grant access
        if let Some(ref master) = self.inner.config.master_secret {
            if token == Some(master.as_str()) {
                return true;
            }
        }

        let sessions = self.inner.sessions.lock().unwrap();
        match sessions.get(session_id) {
            Some(session) => {
                match session.config.auth_token {
                    Some(ref expected) => token == Some(expected.as_str()),
                    None => !self.inner.config.auth_required,
                }
            }
            None => false,
        }
    }

    /// Updates the session's last activity timestamp.
    pub fn touch_session(&self, session_id: &str) {
        let mut sessions = self.inner.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(session_id) {
            session.touch();
        }
    }

    /// Terminates a specific tenant session and frees its resources.
    pub fn terminate_session(&self, session_id: &str) -> io::Result<()> {
        let mut sessions = self.inner.sessions.lock().unwrap();
        if let Some(mut session) = sessions.remove(session_id) {
            session.terminate()?;
        }
        Ok(())
    }

    /// Terminates all currently active sessions.
    pub fn terminate_all(&self) -> io::Result<()> {
        let mut sessions = self.inner.sessions.lock().unwrap();
        for (_, mut session) in sessions.drain() {
            let _ = session.terminate();
        }
        Ok(())
    }

    /// Lists snapshots of all tracked tenant sessions.
    pub fn list_sessions(&self) -> Vec<TenantSessionSnapshot> {
        let mut sessions = self.inner.sessions.lock().unwrap();
        sessions
            .values_mut()
            .map(|s| {
                s.check_health();
                s.snapshot()
            })
            .collect()
    }

    /// Returns the number of currently tracked sessions.
    pub fn active_count(&self) -> usize {
        let sessions = self.inner.sessions.lock().unwrap();
        sessions.len()
    }

    /// Reaps sessions whose processes have terminated.
    pub fn prune_dead(&self) -> usize {
        let mut sessions = self.inner.sessions.lock().unwrap();
        let initial = sessions.len();
        sessions.retain(|_, s| {
            let status = s.check_health();
            !matches!(status, SessionStatus::Terminated { .. } | SessionStatus::Failed(_))
        });
        initial - sessions.len()
    }

    /// Reaps sessions that have exceeded the idle duration threshold.
    pub fn prune_idle(&self, max_idle: Duration) -> usize {
        let mut sessions = self.inner.sessions.lock().unwrap();
        let initial = sessions.len();
        sessions.retain(|_, s| {
            if s.last_activity.elapsed() > max_idle {
                let _ = s.terminate();
                false
            } else {
                true
            }
        });
        initial - sessions.len()
    }

    /// Accesses the underlying session immutably for disk/state inspection.
    pub fn with_session<F, R>(&self, session_id: &str, f: F) -> io::Result<R>
    where
        F: FnOnce(&TenantSession) -> io::Result<R>,
    {
        let sessions = self.inner.sessions.lock().unwrap();
        let session = sessions.get(session_id).ok_or_else(|| {
            Error::new(ErrorKind::NotFound, format!("Session '{}' not found", session_id))
        })?;
        f(session)
    }

    /// Accesses the underlying session mutably for lifecycle manipulation.
    pub fn with_session_mut<F, R>(&self, session_id: &str, f: F) -> io::Result<R>
    where
        F: FnOnce(&mut TenantSession) -> io::Result<R>,
    {
        let mut sessions = self.inner.sessions.lock().unwrap();
        let session = sessions.get_mut(session_id).ok_or_else(|| {
            Error::new(ErrorKind::NotFound, format!("Session '{}' not found", session_id))
        })?;
        f(session)
    }
}
