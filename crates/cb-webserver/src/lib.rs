// Clean-room re-implementation. Behavior derived from publicly documented
// RFC 6455 / RFC 7230 / FIPS 180-4 / RFC 4648 and Win32 ABI semantics.
// No Microsoft or third-party source used.

//! # cb-webserver: HTTP/1.1 + RFC 6455 WebSocket Server & GUI Wire Protocol
//!
//! `cb-webserver` is the network edge of the ClassicBasic Web-Streamed GUI
//! (Phase 3.6, `v0.3.6`). It streams the state of the virtual Win32 desktop
//! held by `cb-win32` to a browser and injects browser input back into the
//! `VirtualMessageQueue`, using only `std` and `cb-win32` (zero external crates).

pub mod sha1;
pub mod base64;
pub mod http;
pub mod frame;
pub mod wire;
pub mod input;
pub mod server;

pub use sha1::sha1_digest;
pub use base64::{base64_encode, base64_decode};
pub use http::{HttpRequest, WS_GUID};
pub use frame::{WebSocketFrame, WebSocketOpcode, MessageReassembler};
pub use wire::{
    DisplayFrame, BitmapRectFrame, FormSnapshotFrame, FormControl, DialogFrame,
    CursorFrame, ClipboardFrame, TextFrame,
    OPCODE_BITMAP_RECT, OPCODE_FORM, OPCODE_DIALOG, OPCODE_CURSOR, OPCODE_CLIPBOARD, OPCODE_TEXT,
};
pub use input::{
    ClientInputEvent, DialogResultEvent, parse_client_input, inject_client_input,
    JsonValue, parse_json_value,
};
pub use server::{ServerHandle, ServerConfig, start_server};
