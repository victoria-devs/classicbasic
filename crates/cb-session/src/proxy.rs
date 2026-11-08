// Clean-room re-implementation. Behavior derived from publicly documented
// RFC 6455 The WebSocket Protocol and standard socket programming. No Microsoft or third-party source used.

//! # Multi-Tenant Reverse Proxy & Connection Router
//!
//! Handles client HTTP API requests and transparently proxies RFC 6455
//! WebSocket traffic directly to the isolated per-tenant `vb6-run` runner instances.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::manager::SessionManager;
use crate::tenant::SessionStatus;

/// Controller handle for a running `SessionBrokerServer`.
pub struct BrokerServerHandle {
    addr: SocketAddr,
    shutdown: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
}

impl BrokerServerHandle {
    /// Returns the bound local socket address.
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Signals server shutdown and joins background worker threads.
    pub fn shutdown(mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(th) = self.accept_thread.take() {
            let _ = th.join();
        }
    }
}

impl Drop for BrokerServerHandle {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(th) = self.accept_thread.take() {
            let _ = th.join();
        }
    }
}

/// Reverse proxy server routing connections to tenant runner processes.
pub struct SessionBrokerServer {
    manager: SessionManager,
    bind_addr: SocketAddr,
}

impl SessionBrokerServer {
    /// Constructs a new `SessionBrokerServer`.
    pub fn new(manager: SessionManager, bind_addr: SocketAddr) -> Self {
        Self { manager, bind_addr }
    }

    /// Starts the broker server on a background thread.
    pub fn start(self) -> std::io::Result<BrokerServerHandle> {
        let listener = TcpListener::bind(self.bind_addr)?;
        let addr = listener.local_addr()?;
        let shutdown = Arc::new(AtomicBool::new(false));

        let shutdown_clone = Arc::clone(&shutdown);
        let manager = self.manager;

        let accept_thread = thread::Builder::new()
            .name("cb-session-broker".into())
            .spawn(move || {
                while !shutdown_clone.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, client_addr)) => {
                            if shutdown_clone.load(Ordering::SeqCst) {
                                break;
                            }
                            let mgr = manager.clone();
                            thread::Builder::new()
                                .name(format!("proxy-{}", client_addr))
                                .spawn(move || {
                                    handle_incoming_connection(stream, mgr);
                                })
                                .ok();
                        }
                        Err(_) => {
                            if shutdown_clone.load(Ordering::SeqCst) {
                                break;
                            }
                        }
                    }
                }
            })?;

        Ok(BrokerServerHandle {
            addr,
            shutdown,
            accept_thread: Some(accept_thread),
        })
    }
}

/// Inspects the initial request line and headers to route API or WebSocket requests.
fn handle_incoming_connection(mut client_stream: TcpStream, manager: SessionManager) {
    client_stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .ok();

    let mut buf = [0u8; 4096];
    let n = match client_stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let header_text = String::from_utf8_lossy(&buf[..n]);
    let first_line = match header_text.lines().next() {
        Some(l) => l,
        None => return,
    };

    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        let _ = client_stream.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n");
        let _ = client_stream.flush();
        return;
    }

    let method = parts[0];
    let uri = parts[1];

    // Extract authorization bearer token if provided
    let mut token: Option<String> = None;
    for line in header_text.lines() {
        if line.to_ascii_lowercase().starts_with("authorization: bearer ") {
            token = Some(line[21..].trim().to_string());
        }
    }

    // Health check endpoint
    if method == "GET" && uri == "/health" {
        let active = manager.active_count();
        let body = format!("{{\"status\":\"ok\",\"active_sessions\":{}}}", active);
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = client_stream.write_all(resp.as_bytes());
        let _ = client_stream.flush();
        let _ = client_stream.shutdown(std::net::Shutdown::Both);
        return;
    }

    // Session list API
    if method == "GET" && uri == "/api/sessions" {
        let sessions = manager.list_sessions();
        let body = serde_json::to_string(&sessions).unwrap_or_else(|_| "[]".to_string());
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = client_stream.write_all(resp.as_bytes());
        let _ = client_stream.flush();
        let _ = client_stream.shutdown(std::net::Shutdown::Both);
        return;
    }

    // Create session API
    if method == "POST" && uri.starts_with("/api/sessions") {
        // Query param or random session ID
        let session_id = uri
            .split('?')
            .nth(1)
            .and_then(|q| {
                q.split('&')
                    .find(|p| p.starts_with("id="))
                    .map(|p| p[3..].to_string())
            })
            .unwrap_or_else(|| format!("tenant-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));

        match manager.create_session(&session_id, None, token) {
            Ok(snap) => {
                let body = serde_json::to_string(&snap).unwrap_or_default();
                let resp = format!(
                    "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = client_stream.write_all(resp.as_bytes());
                let _ = client_stream.flush();
                let _ = client_stream.shutdown(std::net::Shutdown::Both);
            }
            Err(e) => {
                let body = format!("{{\"error\":\"{}\"}}", e);
                let resp = format!(
                    "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = client_stream.write_all(resp.as_bytes());
                let _ = client_stream.flush();
                let _ = client_stream.shutdown(std::net::Shutdown::Both);
            }
        }
        return;
    }

    // Reverse Proxy for WebSocket connections: /session/{session_id}/ws or /session/{session_id}
    if uri.starts_with("/session/") {
        let trimmed = &uri["/session/".len()..];
        let session_id = trimmed
            .split('/')
            .next()
            .unwrap_or("")
            .split('?')
            .next()
            .unwrap_or("");

        if session_id.is_empty() {
            let _ = client_stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\nSession ID missing");
            let _ = client_stream.flush();
            let _ = client_stream.shutdown(std::net::Shutdown::Both);
            return;
        }

        // Authenticate request
        if !manager.authenticate(session_id, token.as_deref()) {
            let _ = client_stream.write_all(b"HTTP/1.1 401 Unauthorized\r\n\r\nInvalid or missing tenant token");
            let _ = client_stream.flush();
            let _ = client_stream.shutdown(std::net::Shutdown::Both);
            return;
        }

        // Query session port
        let snap = match manager.get_session(session_id) {
            Some(s) => s,
            None => {
                let _ = client_stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\nSession not found");
                let _ = client_stream.flush();
                let _ = client_stream.shutdown(std::net::Shutdown::Both);
                return;
            }
        };

        if !matches!(snap.status, SessionStatus::Running { .. }) {
            let _ = client_stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\n\r\nSession process not running");
            let _ = client_stream.flush();
            let _ = client_stream.shutdown(std::net::Shutdown::Both);
            return;
        }

        // Establish proxy connection to the tenant's dedicated runner upstream
        let upstream_addr = format!("127.0.0.1:{}", snap.port);
        let mut upstream_stream = match TcpStream::connect(&upstream_addr) {
            Ok(s) => s,
            Err(_) => {
                let _ = client_stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\nTenant runner unreachable");
                let _ = client_stream.flush();
                let _ = client_stream.shutdown(std::net::Shutdown::Both);
                return;
            }
        };

        // Forward the initial handshake bytes that we already read
        if upstream_stream.write_all(&buf[..n]).is_err() {
            let _ = client_stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\nFailed to write to upstream");
            let _ = client_stream.flush();
            let _ = client_stream.shutdown(std::net::Shutdown::Both);
            return;
        }
        let _ = upstream_stream.flush();

        // Reset timeouts for streaming WebSocket communication
        client_stream.set_read_timeout(None).ok();
        upstream_stream.set_read_timeout(None).ok();

        // Bi-directional transparent stream proxying
        proxy_streams(client_stream, upstream_stream, manager, session_id.to_string());
        return;
    }

    // Default fallback
    let _ = client_stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\nEndpoint not found");
    let _ = client_stream.flush();
    let _ = client_stream.shutdown(std::net::Shutdown::Both);
}

/// Performs bi-directional TCP streaming between client and tenant runner upstream.
fn proxy_streams(
    mut client: TcpStream,
    mut upstream: TcpStream,
    manager: SessionManager,
    session_id: String,
) {
    let mut client_read = match client.try_clone() {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut upstream_write = match upstream.try_clone() {
        Ok(u) => u,
        Err(_) => return,
    };

    let mgr_c = manager.clone();
    let sid_c = session_id.clone();

    // Client -> Upstream forwarding thread
    let c2u = thread::Builder::new()
        .name(format!("c2u-{}", session_id))
        .spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match client_read.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        mgr_c.touch_session(&sid_c);
                        if upstream_write.write_all(&buf[..n]).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = upstream_write.shutdown(std::net::Shutdown::Both);
        });

    // Upstream -> Client forwarding (current thread)
    let mut buf = [0u8; 8192];
    loop {
        match upstream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                manager.touch_session(&session_id);
                if client.write_all(&buf[..n]).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let _ = client.shutdown(std::net::Shutdown::Both);
    if let Ok(th) = c2u {
        let _ = th.join();
    }
}
