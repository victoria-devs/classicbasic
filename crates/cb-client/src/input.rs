// Clean-room re-implementation. Behavior derived from publicly documented
// RFC 8259 The JavaScript Object Notation (JSON) Data Interchange Format & Win32 message semantics.
// No Microsoft or third-party source used.

//! Browser DOM input event encoder to JSON message wire payloads.
//!
//! Encodes pointer, keyboard, window resize, and dialog interaction events into
//! compact JSON text frames consumed by `cb-webserver::input::parse_client_input`.

/// Utility for serializing client events to JSON text messages.
#[derive(Debug, Default, Clone, Copy)]
pub struct InputEncoder;

impl InputEncoder {
    /// Encodes a mouse pointer move event: `mouse.move`.
    pub fn mouse_move(hwnd: u32, x: i32, y: i32, buttons: u32) -> String {
        format!(
            r#"{{"type":"mouse.move","hwnd":{},"x":{},"y":{},"buttons":{}}}"#,
            hwnd, x, y, buttons
        )
    }

    /// Encodes a mouse button down event: `mouse.down`.
    /// `button`: 0 = Left, 1 = Middle, 2 = Right.
    pub fn mouse_down(hwnd: u32, x: i32, y: i32, button: u32) -> String {
        format!(
            r#"{{"type":"mouse.down","hwnd":{},"x":{},"y":{},"button":{}}}"#,
            hwnd, x, y, button
        )
    }

    /// Encodes a mouse button up event: `mouse.up`.
    /// `button`: 0 = Left, 1 = Middle, 2 = Right.
    pub fn mouse_up(hwnd: u32, x: i32, y: i32, button: u32) -> String {
        format!(
            r#"{{"type":"mouse.up","hwnd":{},"x":{},"y":{},"button":{}}}"#,
            hwnd, x, y, button
        )
    }

    /// Encodes a keyboard key down event: `key.down`.
    pub fn key_down(hwnd: u32, vk: usize, repeat: usize) -> String {
        format!(
            r#"{{"type":"key.down","hwnd":{},"vk":{},"repeat":{}}}"#,
            hwnd, vk, repeat
        )
    }

    /// Encodes a keyboard key up event: `key.up`.
    pub fn key_up(hwnd: u32, vk: usize) -> String {
        format!(
            r#"{{"type":"key.up","hwnd":{},"vk":{}}}"#,
            hwnd, vk
        )
    }

    /// Encodes a character input event: `char`.
    pub fn char_input(hwnd: u32, code: usize) -> String {
        format!(
            r#"{{"type":"char","hwnd":{},"code":{}}}"#,
            hwnd, code
        )
    }

    /// Encodes a canvas/window resize notification: `resize`.
    pub fn resize(hwnd: u32, width: i32, height: i32) -> String {
        format!(
            r#"{{"type":"resize","hwnd":{},"width":{},"height":{}}}"#,
            hwnd, width, height
        )
    }

    /// Encodes a modal dialog result response: `dialog.result`.
    pub fn dialog_result(dialog_id: u32, result: i32, text: Option<&str>) -> String {
        if let Some(t) = text {
            format!(
                r#"{{"type":"dialog.result","dialog_id":{},"result":{},"text":"{}"}}"#,
                dialog_id,
                result,
                escape_json(t)
            )
        } else {
            format!(
                r#"{{"type":"dialog.result","dialog_id":{},"result":{}}}"#,
                dialog_id, result
            )
        }
    }
}

/// Escapes special JSON control characters in string literals.
fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}
