// Clean-room re-implementation. Behavior derived from publicly documented
// RFC 7230 and RFC 6455 specifications. No Microsoft or third-party source used.

//! Unit tests for HTTP request parsing and WebSocket upgrade validation.

use cb_webserver::http::{HttpRequest, build_websocket_upgrade_response, build_bad_request_response, build_ok_response, build_not_found_response};

#[test]
fn test_valid_websocket_upgrade_request() {
    let req_bytes = b"GET /ws HTTP/1.1\r\n\
Host: 127.0.0.1:8080\r\n\
Upgrade: websocket\r\n\
Connection: Upgrade\r\n\
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
Sec-WebSocket-Version: 13\r\n\r\n";

    let (req, consumed) = HttpRequest::parse(req_bytes).unwrap().unwrap();
    assert_eq!(consumed, req_bytes.len());
    assert_eq!(req.method, "GET");
    assert_eq!(req.path, "/ws");
    assert_eq!(req.version, "HTTP/1.1");
    assert!(req.is_websocket_upgrade());
    assert_eq!(
        req.sec_websocket_accept().unwrap(),
        "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
    );

    let resp = build_websocket_upgrade_response("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    let resp_str = String::from_utf8(resp).unwrap();
    assert!(resp_str.contains("101 Switching Protocols"));
    assert!(resp_str.contains("Upgrade: websocket"));
    assert!(resp_str.contains("Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));
}

#[test]
fn test_invalid_upgrade_missing_headers() {
    // Missing Sec-WebSocket-Version
    let req_bytes = b"GET /ws HTTP/1.1\r\n\
Upgrade: websocket\r\n\
Connection: Upgrade\r\n\
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n";

    let (req, _) = HttpRequest::parse(req_bytes).unwrap().unwrap();
    assert!(!req.is_websocket_upgrade());

    // Wrong method POST
    let req_bytes_post = b"POST /ws HTTP/1.1\r\n\
Upgrade: websocket\r\n\
Connection: Upgrade\r\n\
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
Sec-WebSocket-Version: 13\r\n\r\n";

    let (req_post, _) = HttpRequest::parse(req_bytes_post).unwrap().unwrap();
    assert!(!req_post.is_websocket_upgrade());
}

#[test]
fn test_static_asset_responses() {
    let ok = build_ok_response("text/html; charset=utf-8", b"<html>Hello</html>");
    let ok_str = String::from_utf8_lossy(&ok);
    assert!(ok_str.contains("200 OK"));
    assert!(ok_str.contains("Content-Length: 18"));
    assert!(ok_str.ends_with("<html>Hello</html>"));

    let nf = build_not_found_response();
    let nf_str = String::from_utf8_lossy(&nf);
    assert!(nf_str.contains("404 Not Found"));

    let bad = build_bad_request_response("Bad Header");
    let bad_str = String::from_utf8_lossy(&bad);
    assert!(bad_str.contains("400 Bad Request"));
}
