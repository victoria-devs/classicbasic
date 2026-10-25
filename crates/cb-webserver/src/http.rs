// Clean-room re-implementation. Behavior derived from publicly documented
// RFC 7230 Hypertext Transfer Protocol (HTTP/1.1) and RFC 6455 The WebSocket Protocol.
// No Microsoft or third-party source used.

//! HTTP/1.1 request-line parser, header extractor, and WebSocket handshake validator.

use std::collections::HashMap;
use crate::sha1::sha1_digest;
use crate::base64::base64_encode;

/// Canonical RFC 6455 WebSocket GUID string.
pub const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// Parsed HTTP/1.1 request.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// HTTP method (e.g. "GET").
    pub method: String,
    /// Request URI target (e.g. "/ws" or "/index.html").
    pub path: String,
    /// HTTP version string (e.g. "HTTP/1.1").
    pub version: String,
    /// Header key-value map (keys normalized to lowercase).
    pub headers: HashMap<String, String>,
}

impl HttpRequest {
    /// Attempts to parse an HTTP/1.1 request from the initial incoming bytes.
    ///
    /// Returns `Ok(Some((req, consumed_bytes)))` if a complete header block (`\r\n\r\n`)
    /// is present. Returns `Ok(None)` if the buffer is incomplete, or `Err(String)` if malformed.
    pub fn parse(buf: &[u8]) -> Result<Option<(Self, usize)>, String> {
        let header_end = buf
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|pos| pos + 4)
            .or_else(|| {
                buf.windows(2)
                    .position(|w| w == b"\n\n")
                    .map(|pos| pos + 2)
            });

        let end_idx = match header_end {
            Some(idx) => idx,
            None => return Ok(None),
        };

        let header_str = std::str::from_utf8(&buf[..end_idx])
            .map_err(|e| format!("Invalid UTF-8 in HTTP headers: {}", e))?;

        let mut lines = header_str.split("\n");
        let request_line = lines.next().ok_or("Empty HTTP request")?.trim_end();
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(format!("Malformed request line: {}", request_line));
        }

        let method = parts[0].to_uppercase();
        let path = parts[1].to_string();
        let version = parts[2].to_uppercase();

        let mut headers = HashMap::new();
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            if let Some(colon_pos) = trimmed.find(':') {
                let key = trimmed[..colon_pos].trim().to_lowercase();
                let val = trimmed[colon_pos + 1..].trim().to_string();
                headers.insert(key, val);
            }
        }

        Ok(Some((
            Self {
                method,
                path,
                version,
                headers,
            },
            end_idx,
        )))
    }

    /// Checks if this request is a valid RFC 6455 WebSocket Upgrade request.
    ///
    /// Validates:
    /// - Method is `GET`
    /// - `Upgrade` header contains `websocket`
    /// - `Connection` header contains `Upgrade`
    /// - `Sec-WebSocket-Version` is `13`
    /// - `Sec-WebSocket-Key` is present and valid
    pub fn is_websocket_upgrade(&self) -> bool {
        if self.method != "GET" {
            return false;
        }

        let upgrade = self
            .headers
            .get("upgrade")
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        if !upgrade.contains("websocket") {
            return false;
        }

        let connection = self
            .headers
            .get("connection")
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        if !connection.contains("upgrade") {
            return false;
        }

        let version = self
            .headers
            .get("sec-websocket-version")
            .map(|s| s.trim())
            .unwrap_or_default();
        if version != "13" {
            return false;
        }

        self.headers.contains_key("sec-websocket-key")
    }

    /// Computes the RFC 6455 `Sec-WebSocket-Accept` value from `Sec-WebSocket-Key`.
    pub fn sec_websocket_accept(&self) -> Option<String> {
        let key = self.headers.get("sec-websocket-key")?.trim();
        let concat = format!("{}{}", key, WS_GUID);
        let digest = sha1_digest(concat.as_bytes());
        Some(base64_encode(&digest))
    }
}

/// Generates the standard HTTP/1.1 101 Switching Protocols upgrade response.
pub fn build_websocket_upgrade_response(accept_key: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {}\r\n\r\n",
        accept_key
    )
    .into_bytes()
}

/// Generates an HTTP 400 Bad Request response.
pub fn build_bad_request_response(msg: &str) -> Vec<u8> {
    let body = format!("Bad Request: {}\n", msg);
    format!(
        "HTTP/1.1 400 Bad Request\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n{}",
        body.len(),
        body
    )
    .into_bytes()
}

/// Generates an HTTP 200 OK static content response.
pub fn build_ok_response(content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut resp = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Connection: keep-alive\r\n\r\n",
        content_type,
        body.len()
    )
    .into_bytes();
    resp.extend_from_slice(body);
    resp
}

/// Generates an HTTP 404 Not Found response.
pub fn build_not_found_response() -> Vec<u8> {
    let body = "404 Not Found\n";
    format!(
        "HTTP/1.1 404 Not Found\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n{}",
        body.len(),
        body
    )
    .into_bytes()
}
