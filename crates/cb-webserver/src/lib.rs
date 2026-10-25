// Clean-room re-implementation. Behavior derived from publicly documented
// RFC 6455 / RFC 7230 / FIPS 180-4 / RFC 4648 and Win32 ABI semantics.
// No Microsoft or third-party source used.

//! # cb-webserver: HTTP/1.1 + RFC 6455 WebSocket Server & GUI Wire Protocol
//!
//! `cb-webserver` is the network edge of the ClassicBasic Web-Streamed GUI
//! (Phase 3.6, `v0.3.6`). It streams the state of the virtual Win32 desktop
//! held by `cb-win32` to a browser and injects browser input back into the
//! `VirtualMessageQueue`, using only `std` (no external crates).
//!
//! ## Planned module layout (Phase 3.6)
//!
//! | Module      | Responsibility |
//! |-------------|----------------|
//! | `sha1`      | FIPS 180-4 SHA-1 digest (handshake `Sec-WebSocket-Accept`). |
//! | `base64`    | RFC 4648 Base64 encoder/decoder. |
//! | `http`      | HTTP/1.1 request-line/header parser and `101 Switching Protocols` upgrade. |
//! | `frame`     | RFC 6455 frame codec: FIN/opcode, 7/16/64-bit lengths, client masking, fragmentation, ping/pong/close. |
//! | `wire`      | Atomic binary display frames `0x02 FRAME_BITMAP_RECT` .. `0x07 FRAME_TEXT`. |
//! | `input`     | Minimal JSON input parser (`mouse.*`, `key.*`, `char`, `resize`, `dialog.result`) to `Win32Msg`. |
//! | `server`    | `std::net::TcpListener` accept loop, per-connection session, `Condvar` wake-up. |
//!
//! See `README.md` in this crate for the full protocol specification.
