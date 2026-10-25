// Clean-room re-implementation. Behavior derived from publicly documented
// RFC 6455 The WebSocket Protocol and standard socket programming. No Microsoft or third-party source used.

//! RFC 6455 WebSocket Server and HTTP static asset host over `std::net::TcpListener`.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread::{self, JoinHandle};

use cb_win32::VirtualMessageQueue;
use crate::http::{HttpRequest, build_websocket_upgrade_response, build_bad_request_response, build_ok_response, build_not_found_response};
use crate::frame::{WebSocketFrame, WebSocketOpcode, MessageReassembler};
use crate::wire::DisplayFrame;
use crate::input::{parse_client_input, inject_client_input, DialogResultEvent};

/// Handle to a running `WebServer` allowing control, port querying, and graceful shutdown.
pub struct ServerHandle {
    addr: SocketAddr,
    shutdown: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
    sessions: Arc<Mutex<Vec<Arc<SessionHandle>>>>,
}

impl ServerHandle {
    /// Returns the local bound socket address (useful when bound to port 0).
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Broadcasts a display frame to all connected active WebSocket sessions.
    pub fn broadcast_frame(&self, frame: &DisplayFrame) {
        let wire_bytes = frame.encode();
        let ws_frame = WebSocketFrame::binary(wire_bytes);
        let encoded = ws_frame.encode();

        let mut sessions = self.sessions.lock().unwrap();
        sessions.retain(|s| s.send_raw(&encoded));
    }

    /// Triggers shutdown and waits for the accept loop to terminate.
    pub fn shutdown(mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        // Connect to unblock accept()
        let _ = TcpStream::connect(self.addr);
        if let Some(th) = self.accept_thread.take() {
            let _ = th.join();
        }
        let sessions = self.sessions.lock().unwrap();
        for s in sessions.iter() {
            s.close();
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(th) = self.accept_thread.take() {
            let _ = th.join();
        }
    }
}

struct SessionHandle {
    stream: Mutex<TcpStream>,
    active: AtomicBool,
}

impl SessionHandle {
    fn new(stream: TcpStream) -> Self {
        Self {
            stream: Mutex::new(stream),
            active: AtomicBool::new(true),
        }
    }

    fn send_raw(&self, data: &[u8]) -> bool {
        if !self.active.load(Ordering::SeqCst) {
            return false;
        }
        if let Ok(mut s) = self.stream.lock() {
            if s.write_all(data).is_err() {
                self.active.store(false, Ordering::SeqCst);
                return false;
            }
            let _ = s.flush();
            true
        } else {
            false
        }
    }

    fn close(&self) {
        self.active.store(false, Ordering::SeqCst);
        if let Ok(mut s) = self.stream.lock() {
            let close_frame = WebSocketFrame::close(Some(1000), "Normal Closure").encode();
            let _ = s.write_all(&close_frame);
            let _ = s.flush();
            let _ = s.shutdown(std::net::Shutdown::Both);
        }
    }
}

/// WebServer configuration.
#[derive(Clone)]
pub struct ServerConfig {
    /// Bind address (e.g. `"127.0.0.1:0"` for random free port).
    pub bind_addr: String,
    /// Static assets map: path -> (mime_type, content).
    pub static_assets: HashMap<String, (&'static str, Vec<u8>)>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:0".to_string(),
            static_assets: HashMap::new(),
        }
    }
}

/// Starts a new HTTP/1.1 + RFC 6455 WebSocket server.
///
/// Inbound client inputs are delivered directly into `msg_queue`.
/// Dialog results (if any) can optionally be captured via `on_dialog_result`.
pub fn start_server<F>(
    config: ServerConfig,
    msg_queue: Arc<VirtualMessageQueue>,
    on_dialog_result: Option<F>,
) -> std::io::Result<ServerHandle>
where
    F: Fn(DialogResultEvent) + Send + Sync + 'static,
{
    let listener = TcpListener::bind(&config.bind_addr)?;
    let addr = listener.local_addr()?;
    let shutdown = Arc::new(AtomicBool::new(false));
    let sessions: Arc<Mutex<Vec<Arc<SessionHandle>>>> = Arc::new(Mutex::new(Vec::new()));

    let dialog_cb: Option<Arc<dyn Fn(DialogResultEvent) + Send + Sync>> =
        on_dialog_result.map(|cb| Arc::new(cb) as Arc<dyn Fn(DialogResultEvent) + Send + Sync>);

    let th_shutdown = Arc::clone(&shutdown);
    let th_sessions = Arc::clone(&sessions);
    let th_assets = config.static_assets;

    let accept_thread = thread::spawn(move || {
        while !th_shutdown.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    if th_shutdown.load(Ordering::SeqCst) {
                        break;
                    }
                    let session = Arc::new(SessionHandle::new(stream));
                    {
                        let mut s_list = th_sessions.lock().unwrap();
                        s_list.retain(|s| s.active.load(Ordering::SeqCst));
                        s_list.push(Arc::clone(&session));
                    }

                    let s_session = Arc::clone(&session);
                    let s_queue = Arc::clone(&msg_queue);
                    let s_dialog = dialog_cb.clone();
                    let s_assets = th_assets.clone();

                    thread::spawn(move || {
                        handle_connection(s_session, s_queue, s_dialog, s_assets);
                    });
                }
                Err(_) => {
                    if th_shutdown.load(Ordering::SeqCst) {
                        break;
                    }
                }
            }
        }
    });

    Ok(ServerHandle {
        addr,
        shutdown,
        accept_thread: Some(accept_thread),
        sessions,
    })
}

fn handle_connection(
    session: Arc<SessionHandle>,
    queue: Arc<VirtualMessageQueue>,
    dialog_cb: Option<Arc<dyn Fn(DialogResultEvent) + Send + Sync>>,
    static_assets: HashMap<String, (&'static str, Vec<u8>)>,
) {
    let mut stream_clone = match session.stream.lock() {
        Ok(s) => match s.try_clone() {
            Ok(c) => c,
            Err(_) => return,
        },
        Err(_) => return,
    };

    let mut buf = vec![0u8; 4096];
    let mut filled = 0;

    // Phase 1: Read HTTP Handshake
    loop {
        let n = match stream_clone.read(&mut buf[filled..]) {
            Ok(0) => return,
            Ok(n) => n,
            Err(_) => return,
        };
        filled += n;

        match HttpRequest::parse(&buf[..filled]) {
            Ok(Some((req, consumed))) => {
                if req.is_websocket_upgrade() {
                    let accept_key = match req.sec_websocket_accept() {
                        Some(k) => k,
                        None => {
                            let resp = build_bad_request_response("Missing Sec-WebSocket-Key");
                            let _ = stream_clone.write_all(&resp);
                            return;
                        }
                    };
                    let resp = build_websocket_upgrade_response(&accept_key);
                    if stream_clone.write_all(&resp).is_err() {
                        return;
                    }
                    let _ = stream_clone.flush();

                    // Shift remaining bytes
                    buf.copy_within(consumed..filled, 0);
                    filled -= consumed;
                    break;
                } else if req.method == "GET" {
                    // Static asset request
                    let path = req.path.split('?').next().unwrap_or(&req.path);
                    let path = if path == "/" { "/index.html" } else { path };

                    if let Some((content_type, body)) = static_assets.get(path) {
                        let resp = build_ok_response(content_type, body);
                        let _ = stream_clone.write_all(&resp);
                    } else {
                        let resp = build_not_found_response();
                        let _ = stream_clone.write_all(&resp);
                    }
                    let _ = stream_clone.flush();
                    return;
                } else {
                    let resp = build_bad_request_response("Expected WebSocket Upgrade");
                    let _ = stream_clone.write_all(&resp);
                    return;
                }
            }
            Ok(None) => {
                if filled >= buf.len() {
                    buf.resize(buf.len() * 2, 0);
                }
            }
            Err(e) => {
                let resp = build_bad_request_response(&e);
                let _ = stream_clone.write_all(&resp);
                return;
            }
        }
    }

    // Phase 2: WebSocket Streaming Loop
    let mut reassembler = MessageReassembler::new();

    loop {
        // Try parsing frame from available buffer
        loop {
            match WebSocketFrame::parse(&buf[..filled], true) {
                Ok(Some((frame, consumed))) => {
                    buf.copy_within(consumed..filled, 0);
                    filled -= consumed;

                    match frame.opcode {
                        WebSocketOpcode::Close => {
                            let reply = WebSocketFrame::close(Some(1000), "").encode();
                            let _ = session.send_raw(&reply);
                            session.active.store(false, Ordering::SeqCst);
                            return;
                        }
                        WebSocketOpcode::Ping => {
                            let pong = WebSocketFrame::pong(frame.payload).encode();
                            let _ = session.send_raw(&pong);
                        }
                        WebSocketOpcode::Pong => {}
                        _ => {
                            // Text or Binary data
                            if let Ok(Some((opcode, full_payload))) = reassembler.process_frame(frame) {
                                if opcode == WebSocketOpcode::Text {
                                    if let Ok(text) = std::str::from_utf8(&full_payload) {
                                        if let Ok(ev) = parse_client_input(text) {
                                            if let Some(diag_res) = inject_client_input(ev, &queue) {
                                                if let Some(ref cb) = dialog_cb {
                                                    cb(diag_res);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(None) => break, // Need more bytes from network
                Err(_) => {
                    // Protocol error: send 1002 protocol error close
                    let close_err = WebSocketFrame::close(Some(1002), "Protocol error").encode();
                    let _ = session.send_raw(&close_err);
                    session.active.store(false, Ordering::SeqCst);
                    return;
                }
            }
        }

        if filled >= buf.len() {
            buf.resize(buf.len() * 2, 0);
        }

        let n = match stream_clone.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        filled += n;
    }

    session.active.store(false, Ordering::SeqCst);
}
