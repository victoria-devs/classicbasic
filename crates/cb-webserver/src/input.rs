// Clean-room re-implementation. Behavior derived from publicly documented
// RFC 8259 The JavaScript Object Notation (JSON) Data Interchange Format and Win32 message semantics.
// No Microsoft or third-party source used.

//! Hand-rolled minimal JSON input parser and Win32 message queue injector.

use std::collections::HashMap;
use cb_win32::{
    Win32Msg, VirtualMessageQueue,
    WM_MOUSEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_KEYDOWN, WM_KEYUP, WM_CHAR, WM_SIZE,
};

/// High-level parsed client input event.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientInputEvent {
    /// Inbound Win32 message to be posted to the message queue.
    Message(Win32Msg),
    /// Inbound modal dialog completion result.
    DialogResult(DialogResultEvent),
}

/// Dialog result event from a browser response.
#[derive(Debug, Clone, PartialEq)]
pub struct DialogResultEvent {
    pub dialog_id: u32,
    pub result: i32,
    pub text: Option<String>,
}

/// Parses a JSON text frame from the client into a `ClientInputEvent`.
pub fn parse_client_input(json_text: &str) -> Result<ClientInputEvent, String> {
    let val = parse_json_value(json_text)?;
    let obj = match val {
        JsonValue::Object(map) => map,
        _ => return Err("Expected JSON object at root".to_string()),
    };

    let event_type = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required 'type' field".to_string())?;

    match event_type {
        "mouse.move" => {
            let hwnd = obj.get("hwnd").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
            let x = obj.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = obj.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let buttons = obj.get("buttons").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
            let l_param = make_lparam(x, y);

            Ok(ClientInputEvent::Message(Win32Msg {
                hwnd,
                message: WM_MOUSEMOVE,
                w_param: buttons as usize,
                l_param: l_param as usize,
                time: 0,
                pt_x: x,
                pt_y: y,
            }))
        }
        "mouse.down" | "mouse.up" => {
            let hwnd = obj.get("hwnd").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
            let x = obj.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = obj.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let button = obj.get("button").and_then(|v| v.as_i64()).unwrap_or(0); // 0=Left, 1=Middle, 2=Right
            let is_down = event_type == "mouse.down";

            let (msg, w_param) = match button {
                0 => (
                    if is_down { WM_LBUTTONDOWN } else { WM_LBUTTONUP },
                    if is_down { 1usize } else { 0usize }, // MK_LBUTTON
                ),
                1 => (
                    if is_down { WM_MBUTTONDOWN } else { WM_MBUTTONUP },
                    if is_down { 0x10usize } else { 0usize }, // MK_MBUTTON
                ),
                _ => (
                    if is_down { WM_RBUTTONDOWN } else { WM_RBUTTONUP },
                    if is_down { 2usize } else { 0usize }, // MK_RBUTTON
                ),
            };

            let l_param = make_lparam(x, y);
            Ok(ClientInputEvent::Message(Win32Msg {
                hwnd,
                message: msg,
                w_param,
                l_param: l_param as usize,
                time: 0,
                pt_x: x,
                pt_y: y,
            }))
        }
        "key.down" => {
            let hwnd = obj.get("hwnd").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
            let vk = obj.get("vk").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            let repeat = obj.get("repeat").and_then(|v| v.as_i64()).unwrap_or(1) as usize;

            Ok(ClientInputEvent::Message(Win32Msg {
                hwnd,
                message: WM_KEYDOWN,
                w_param: vk,
                l_param: repeat,
                time: 0,
                pt_x: 0,
                pt_y: 0,
            }))
        }
        "key.up" => {
            let hwnd = obj.get("hwnd").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
            let vk = obj.get("vk").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            let l_param = 0xC0000001usize;

            Ok(ClientInputEvent::Message(Win32Msg {
                hwnd,
                message: WM_KEYUP,
                w_param: vk,
                l_param,
                time: 0,
                pt_x: 0,
                pt_y: 0,
            }))
        }
        "char" => {
            let hwnd = obj.get("hwnd").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
            let code = obj.get("code").and_then(|v| v.as_i64()).unwrap_or(0) as usize;

            Ok(ClientInputEvent::Message(Win32Msg {
                hwnd,
                message: WM_CHAR,
                w_param: code,
                l_param: 1,
                time: 0,
                pt_x: 0,
                pt_y: 0,
            }))
        }
        "resize" => {
            let hwnd = obj.get("hwnd").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
            let width = obj.get("width").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let height = obj.get("height").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let l_param = make_lparam(width, height);

            Ok(ClientInputEvent::Message(Win32Msg {
                hwnd,
                message: WM_SIZE,
                w_param: 0, // SIZE_RESTORED
                l_param: l_param as usize,
                time: 0,
                pt_x: 0,
                pt_y: 0,
            }))
        }
        "dialog.result" => {
            let dialog_id = obj.get("dialog_id").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
            let result = obj.get("result").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let text = obj.get("text").and_then(|v| v.as_str()).map(|s| s.to_string());

            Ok(ClientInputEvent::DialogResult(DialogResultEvent {
                dialog_id,
                result,
                text,
            }))
        }
        other => Err(format!("Unknown input event type: '{}'", other)),
    }
}

/// Injects a parsed `ClientInputEvent` into the provided `VirtualMessageQueue`.
pub fn inject_client_input(event: ClientInputEvent, queue: &VirtualMessageQueue) -> Option<DialogResultEvent> {
    match event {
        ClientInputEvent::Message(msg) => {
            queue.post_message(msg);
            None
        }
        ClientInputEvent::DialogResult(d) => Some(d),
    }
}

fn make_lparam(lo: i32, hi: i32) -> u32 {
    ((hi as u32 & 0xFFFF) << 16) | (lo as u32 & 0xFFFF)
}

// -----------------------------------------------------------------------------
// Minimal Hand-Rolled JSON Parser (RFC 8259)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(n) => Some(*n as i64),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// Parses a JSON value from a string.
pub fn parse_json_value(input: &str) -> Result<JsonValue, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut idx = 0;
    skip_whitespace(&chars, &mut idx);
    let val = parse_val(&chars, &mut idx)?;
    skip_whitespace(&chars, &mut idx);
    if idx < chars.len() {
        return Err(format!("Trailing characters after JSON value at pos {}", idx));
    }
    Ok(val)
}

fn skip_whitespace(chars: &[char], idx: &mut usize) {
    while *idx < chars.len() && chars[*idx].is_whitespace() {
        *idx += 1;
    }
}

fn parse_val(chars: &[char], idx: &mut usize) -> Result<JsonValue, String> {
    skip_whitespace(chars, idx);
    if *idx >= chars.len() {
        return Err("Unexpected end of JSON input".to_string());
    }

    match chars[*idx] {
        'n' => parse_literal(chars, idx, "null", JsonValue::Null),
        't' => parse_literal(chars, idx, "true", JsonValue::Bool(true)),
        'f' => parse_literal(chars, idx, "false", JsonValue::Bool(false)),
        '"' => parse_string(chars, idx).map(JsonValue::String),
        '[' => parse_array(chars, idx),
        '{' => parse_object(chars, idx),
        '-' | '0'..='9' => parse_number(chars, idx),
        other => Err(format!("Unexpected character '{}' at pos {}", other, idx)),
    }
}

fn parse_literal(chars: &[char], idx: &mut usize, expected: &str, ret: JsonValue) -> Result<JsonValue, String> {
    for expected_c in expected.chars() {
        if *idx >= chars.len() || chars[*idx] != expected_c {
            return Err(format!("Expected '{}' at pos {}", expected, idx));
        }
        *idx += 1;
    }
    Ok(ret)
}

fn parse_string(chars: &[char], idx: &mut usize) -> Result<String, String> {
    *idx += 1; // skip leading '"'
    let mut s = String::new();
    while *idx < chars.len() {
        let c = chars[*idx];
        *idx += 1;
        match c {
            '"' => return Ok(s),
            '\\' => {
                if *idx >= chars.len() {
                    return Err("Unterminated escape sequence".to_string());
                }
                let esc = chars[*idx];
                *idx += 1;
                match esc {
                    '"' => s.push('"'),
                    '\\' => s.push('\\'),
                    '/' => s.push('/'),
                    'b' => s.push('\x08'),
                    'f' => s.push('\x0C'),
                    'n' => s.push('\n'),
                    'r' => s.push('\r'),
                    't' => s.push('\t'),
                    'u' => {
                        if *idx + 4 > chars.len() {
                            return Err("Incomplete \\u hex escape".to_string());
                        }
                        let hex_str: String = chars[*idx..*idx + 4].iter().collect();
                        *idx += 4;
                        let code = u16::from_str_radix(&hex_str, 16)
                            .map_err(|e| format!("Invalid \\u hex: {}", e))?;
                        if let Some(ch) = char::from_u32(code as u32) {
                            s.push(ch);
                        }
                    }
                    other => return Err(format!("Unsupported escape '\\{}'", other)),
                }
            }
            other => s.push(other),
        }
    }
    Err("Unterminated string literal".to_string())
}

fn parse_number(chars: &[char], idx: &mut usize) -> Result<JsonValue, String> {
    let start = *idx;
    if *idx < chars.len() && chars[*idx] == '-' {
        *idx += 1;
    }
    while *idx < chars.len() && chars[*idx].is_ascii_digit() {
        *idx += 1;
    }
    if *idx < chars.len() && chars[*idx] == '.' {
        *idx += 1;
        while *idx < chars.len() && chars[*idx].is_ascii_digit() {
            *idx += 1;
        }
    }
    if *idx < chars.len() && (chars[*idx] == 'e' || chars[*idx] == 'E') {
        *idx += 1;
        if *idx < chars.len() && (chars[*idx] == '+' || chars[*idx] == '-') {
            *idx += 1;
        }
        while *idx < chars.len() && chars[*idx].is_ascii_digit() {
            *idx += 1;
        }
    }
    let s: String = chars[start..*idx].iter().collect();
    let n = s
        .parse::<f64>()
        .map_err(|e| format!("Invalid number '{}': {}", s, e))?;
    Ok(JsonValue::Number(n))
}

fn parse_array(chars: &[char], idx: &mut usize) -> Result<JsonValue, String> {
    *idx += 1; // skip '['
    let mut arr = Vec::new();
    skip_whitespace(chars, idx);
    if *idx < chars.len() && chars[*idx] == ']' {
        *idx += 1;
        return Ok(JsonValue::Array(arr));
    }

    loop {
        let item = parse_val(chars, idx)?;
        arr.push(item);
        skip_whitespace(chars, idx);
        if *idx >= chars.len() {
            return Err("Unterminated array".to_string());
        }
        if chars[*idx] == ']' {
            *idx += 1;
            break;
        }
        if chars[*idx] == ',' {
            *idx += 1;
        } else {
            return Err(format!("Expected ',' or ']' at pos {}", idx));
        }
    }
    Ok(JsonValue::Array(arr))
}

fn parse_object(chars: &[char], idx: &mut usize) -> Result<JsonValue, String> {
    *idx += 1; // skip '{'
    let mut map = HashMap::new();
    skip_whitespace(chars, idx);
    if *idx < chars.len() && chars[*idx] == '}' {
        *idx += 1;
        return Ok(JsonValue::Object(map));
    }

    loop {
        skip_whitespace(chars, idx);
        if *idx >= chars.len() || chars[*idx] != '"' {
            return Err(format!("Expected string key at pos {}", idx));
        }
        let key = parse_string(chars, idx)?;
        skip_whitespace(chars, idx);
        if *idx >= chars.len() || chars[*idx] != ':' {
            return Err(format!("Expected ':' after key at pos {}", idx));
        }
        *idx += 1; // skip ':'
        let val = parse_val(chars, idx)?;
        map.insert(key, val);

        skip_whitespace(chars, idx);
        if *idx >= chars.len() {
            return Err("Unterminated object".to_string());
        }
        if chars[*idx] == '}' {
            *idx += 1;
            break;
        }
        if chars[*idx] == ',' {
            *idx += 1;
        } else {
            return Err(format!("Expected ',' or '}}' at pos {}", idx));
        }
    }
    Ok(JsonValue::Object(map))
}
