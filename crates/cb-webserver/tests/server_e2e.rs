// Clean-room re-implementation. Behavior derived from publicly documented
// RFC 6455 and Win32 message queue semantics. No Microsoft or third-party source used.

//! End-to-end loopback integration test of HTTP/1.1 + RFC 6455 WebSocket server.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use cb_webserver::{
    start_server, ServerConfig, WebSocketFrame, WebSocketOpcode,
    DisplayFrame, CursorFrame,
};
use cb_win32::{VirtualMessageQueue, WM_LBUTTONDOWN};

#[test]
fn test_loopback_server_e2e_session() {
    let queue = Arc::new(VirtualMessageQueue::new());
    let mut config = ServerConfig {
        bind_addr: "127.0.0.1:0".to_string(),
        ..Default::default()
    };
    config.static_assets.insert(
        "/index.html".to_string(),
        ("text/html", b"<h1>ClassicBasic Web GUI</h1>".to_vec()),
    );

    let handle = start_server(config, Arc::clone(&queue), None::<fn(_)>).unwrap();
    let addr = handle.local_addr();

    // 1. Test Static Asset GET
    {
        let mut client = TcpStream::connect(addr).unwrap();
        client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let get_req = b"GET /index.html HTTP/1.1\r\nHost: localhost\r\n\r\n";
        client.write_all(get_req).unwrap();

        let mut buf = [0u8; 512];
        let n = client.read(&mut buf).unwrap();
        let resp = String::from_utf8_lossy(&buf[..n]);
        assert!(resp.contains("200 OK"));
        assert!(resp.contains("ClassicBasic Web GUI"));
    }

    // 2. Test WebSocket Handshake and Streaming
    {
        let mut client = TcpStream::connect(addr).unwrap();
        client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();

        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let upgrade_req = format!(
            "GET /ws HTTP/1.1\r\n\
             Host: localhost\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {}\r\n\
             Sec-WebSocket-Version: 13\r\n\r\n",
            key
        );
        client.write_all(upgrade_req.as_bytes()).unwrap();

        let mut buf = [0u8; 1024];
        let n = client.read(&mut buf).unwrap();
        let resp = String::from_utf8_lossy(&buf[..n]);
        assert!(resp.contains("101 Switching Protocols"));
        assert!(resp.contains("s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));

        // Send masked JSON input frame from client
        let json_input = r#"{"type": "mouse.down", "hwnd": 999, "x": 12, "y": 34, "button": 0}"#;
        let mut client_frame = WebSocketFrame::text(json_input);
        client_frame.mask = Some([0xAA, 0xBB, 0xCC, 0xDD]);
        client.write_all(&client_frame.encode()).unwrap();

        // Check that message was received on the server's VirtualMessageQueue
        std::thread::sleep(Duration::from_millis(50));
        let msg = queue.peek_message(999, 0, 0, true).expect("Message should be present in message queue");
        assert_eq!(msg.hwnd, 999);
        assert_eq!(msg.message, WM_LBUTTONDOWN);
        assert_eq!(msg.pt_x, 12);
        assert_eq!(msg.pt_y, 34);

        // Server broadcasts display frame to client
        let display_frame = DisplayFrame::Cursor(CursorFrame {
            cursor_id: 32512,
            x: 77,
            y: 88,
        });
        handle.broadcast_frame(&display_frame);

        // Client reads and verifies binary display frame
        let n = client.read(&mut buf).unwrap();
        let (parsed_ws, _) = WebSocketFrame::parse(&buf[..n], false).unwrap().unwrap();
        assert_eq!(parsed_ws.opcode, WebSocketOpcode::Binary);
        let parsed_display = DisplayFrame::decode(&parsed_ws.payload).unwrap();
        assert_eq!(parsed_display, display_frame);

        // Clean close handshake
        let mut close_frame = WebSocketFrame::close(Some(1000), "Bye");
        close_frame.mask = Some([1, 2, 3, 4]);
        client.write_all(&close_frame.encode()).unwrap();
    }

    // Graceful server shutdown
    handle.shutdown();
}

#[test]
fn test_http_keepalive_pipelined_requests() {
    let queue = Arc::new(VirtualMessageQueue::new());
    let mut config = ServerConfig {
        bind_addr: "127.0.0.1:0".to_string(),
        ..Default::default()
    };
    config.static_assets.insert(
        "/index.html".to_string(),
        ("text/html", b"<html>Hello</html>".to_vec()),
    );
    config.static_assets.insert(
        "/classicbasic.js".to_string(),
        ("application/javascript", b"console.log('cb');".to_vec()),
    );

    let handle = start_server(config, Arc::clone(&queue), None::<fn(_)>).unwrap();
    let addr = handle.local_addr();

    let mut client = TcpStream::connect(addr).unwrap();
    client.set_read_timeout(Some(Duration::from_secs(3))).unwrap();

    // 1. Send first request with keep-alive
    let req1 = b"GET /index.html HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n";
    client.write_all(req1).unwrap();

    let mut buf = [0u8; 1024];
    let n1 = client.read(&mut buf).unwrap();
    let resp1 = String::from_utf8_lossy(&buf[..n1]);
    assert!(resp1.contains("200 OK"));
    assert!(resp1.contains("Hello"));
    assert!(resp1.contains("connection: keep-alive") || resp1.contains("Connection: keep-alive"));

    // 2. Send second request on the SAME persistent connection
    let req2 = b"GET /classicbasic.js HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    client.write_all(req2).unwrap();

    let n2 = client.read(&mut buf).unwrap();
    let resp2 = String::from_utf8_lossy(&buf[..n2]);
    assert!(resp2.contains("200 OK"));
    assert!(resp2.contains("console.log('cb');"));

    handle.shutdown();
}

