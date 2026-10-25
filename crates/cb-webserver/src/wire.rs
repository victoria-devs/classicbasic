// Clean-room re-implementation. Behavior derived from publicly documented
// Milestone 3 Display Wire Protocol (Phase 3.6). No Microsoft or third-party source used.

//! Atomic binary display frame codecs (`0x02`..`0x07`) for streaming virtual Win32
//! state over RFC 6455 binary WebSocket messages.

pub const OPCODE_BITMAP_RECT: u8 = 0x02;
pub const OPCODE_FORM: u8 = 0x03;
pub const OPCODE_DIALOG: u8 = 0x04;
pub const OPCODE_CURSOR: u8 = 0x05;
pub const OPCODE_CLIPBOARD: u8 = 0x06;
pub const OPCODE_TEXT: u8 = 0x07;

/// High-level representation of an atomic display frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayFrame {
    /// 0x02: Dirty rectangle pixel update (BGRA 32-bit).
    BitmapRect(BitmapRectFrame),
    /// 0x03: Form and child controls snapshot.
    Form(FormSnapshotFrame),
    /// 0x04: Modal MsgBox / InputBox dialog request.
    Dialog(DialogFrame),
    /// 0x05: Host cursor shape and position update.
    Cursor(CursorFrame),
    /// 0x06: System clipboard text update.
    Clipboard(ClipboardFrame),
    /// 0x07: Immediate vector text draw operation.
    Text(TextFrame),
}

/// 0x02 FRAME_BITMAP_RECT: dirty rectangular bitmap blit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitmapRectFrame {
    pub hwnd: u32,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    /// 32-bit BGRA (or RGBA) pixel buffer of length `width * height * 4`.
    pub pixels: Vec<u8>,
}

/// A control descriptor embedded in a 0x03 FRAME_FORM payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormControl {
    pub hwnd: u32,
    pub kind: u8,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub text: String,
}

/// 0x03 FRAME_FORM: window geometry, caption, and controls snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormSnapshotFrame {
    pub hwnd: u32,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub style: u32,
    pub caption: String,
    pub controls: Vec<FormControl>,
}

/// 0x04 FRAME_DIALOG: modal MsgBox / InputBox request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogFrame {
    pub dialog_id: u32,
    /// 0 = MsgBox, 1 = InputBox.
    pub kind: u8,
    pub buttons: u32,
    pub title: String,
    pub prompt: String,
    pub default_text: String,
}

/// 0x05 FRAME_CURSOR: active mouse cursor handle/ID and position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorFrame {
    pub cursor_id: u16,
    pub x: i16,
    pub y: i16,
}

/// 0x06 FRAME_CLIPBOARD: clipboard content payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardFrame {
    /// 1 = CF_TEXT.
    pub format: u16,
    pub data: String,
}

/// 0x07 FRAME_TEXT: vector text rendering run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFrame {
    pub hwnd: u32,
    pub x: i16,
    pub y: i16,
    pub color_rgb: u32,
    /// 1 = TRANSPARENT, 2 = OPAQUE.
    pub bk_mode: u8,
    pub font_height: i16,
    pub text: String,
}

// -----------------------------------------------------------------------------
// Encoding & Decoding Implementation
// -----------------------------------------------------------------------------

impl DisplayFrame {
    /// Encodes this display frame into a complete binary WebSocket message payload.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            Self::BitmapRect(f) => {
                out.push(OPCODE_BITMAP_RECT);
                out.extend_from_slice(&f.hwnd.to_le_bytes());
                out.extend_from_slice(&f.x.to_le_bytes());
                out.extend_from_slice(&f.y.to_le_bytes());
                out.extend_from_slice(&f.width.to_le_bytes());
                out.extend_from_slice(&f.height.to_le_bytes());
                out.extend_from_slice(&f.pixels);
            }
            Self::Form(f) => {
                out.push(OPCODE_FORM);
                out.extend_from_slice(&f.hwnd.to_le_bytes());
                out.extend_from_slice(&f.x.to_le_bytes());
                out.extend_from_slice(&f.y.to_le_bytes());
                out.extend_from_slice(&f.width.to_le_bytes());
                out.extend_from_slice(&f.height.to_le_bytes());
                out.extend_from_slice(&f.style.to_le_bytes());
                write_str16(&mut out, &f.caption);
                out.extend_from_slice(&(f.controls.len() as u16).to_le_bytes());
                for c in &f.controls {
                    out.extend_from_slice(&c.hwnd.to_le_bytes());
                    out.push(c.kind);
                    out.extend_from_slice(&c.x.to_le_bytes());
                    out.extend_from_slice(&c.y.to_le_bytes());
                    out.extend_from_slice(&c.width.to_le_bytes());
                    out.extend_from_slice(&c.height.to_le_bytes());
                    write_str16(&mut out, &c.text);
                }
            }
            Self::Dialog(f) => {
                out.push(OPCODE_DIALOG);
                out.extend_from_slice(&f.dialog_id.to_le_bytes());
                out.push(f.kind);
                out.extend_from_slice(&f.buttons.to_le_bytes());
                write_str16(&mut out, &f.title);
                write_str16(&mut out, &f.prompt);
                write_str16(&mut out, &f.default_text);
            }
            Self::Cursor(f) => {
                out.push(OPCODE_CURSOR);
                out.extend_from_slice(&f.cursor_id.to_le_bytes());
                out.extend_from_slice(&f.x.to_le_bytes());
                out.extend_from_slice(&f.y.to_le_bytes());
            }
            Self::Clipboard(f) => {
                out.push(OPCODE_CLIPBOARD);
                out.extend_from_slice(&f.format.to_le_bytes());
                write_str32(&mut out, &f.data);
            }
            Self::Text(f) => {
                out.push(OPCODE_TEXT);
                out.extend_from_slice(&f.hwnd.to_le_bytes());
                out.extend_from_slice(&f.x.to_le_bytes());
                out.extend_from_slice(&f.y.to_le_bytes());
                out.extend_from_slice(&f.color_rgb.to_le_bytes());
                out.push(f.bk_mode);
                out.extend_from_slice(&f.font_height.to_le_bytes());
                write_str16(&mut out, &f.text);
            }
        }
        out
    }

    /// Decodes a display frame from a binary message payload.
    pub fn decode(buf: &[u8]) -> Result<Self, String> {
        if buf.is_empty() {
            return Err("Empty wire frame buffer".to_string());
        }

        let opcode = buf[0];
        let mut cur = &buf[1..];

        match opcode {
            OPCODE_BITMAP_RECT => {
                if cur.len() < 12 {
                    return Err("Incomplete BitmapRect header".to_string());
                }
                let hwnd = read_u32_le(&mut cur)?;
                let x = read_u16_le(&mut cur)?;
                let y = read_u16_le(&mut cur)?;
                let width = read_u16_le(&mut cur)?;
                let height = read_u16_le(&mut cur)?;
                let expected_pixels = (width as usize) * (height as usize) * 4;
                if cur.len() < expected_pixels {
                    return Err(format!(
                        "BitmapRect payload shortfall: expected {} bytes, got {}",
                        expected_pixels,
                        cur.len()
                    ));
                }
                let pixels = cur[..expected_pixels].to_vec();
                Ok(Self::BitmapRect(BitmapRectFrame {
                    hwnd,
                    x,
                    y,
                    width,
                    height,
                    pixels,
                }))
            }
            OPCODE_FORM => {
                let hwnd = read_u32_le(&mut cur)?;
                let x = read_i16_le(&mut cur)?;
                let y = read_i16_le(&mut cur)?;
                let width = read_u16_le(&mut cur)?;
                let height = read_u16_le(&mut cur)?;
                let style = read_u32_le(&mut cur)?;
                let caption = read_str16(&mut cur)?;
                let n_controls = read_u16_le(&mut cur)? as usize;
                let mut controls = Vec::with_capacity(n_controls);
                for _ in 0..n_controls {
                    let c_hwnd = read_u32_le(&mut cur)?;
                    let kind = read_u8(&mut cur)?;
                    let c_x = read_i16_le(&mut cur)?;
                    let c_y = read_i16_le(&mut cur)?;
                    let c_w = read_u16_le(&mut cur)?;
                    let c_h = read_u16_le(&mut cur)?;
                    let text = read_str16(&mut cur)?;
                    controls.push(FormControl {
                        hwnd: c_hwnd,
                        kind,
                        x: c_x,
                        y: c_y,
                        width: c_w,
                        height: c_h,
                        text,
                    });
                }
                Ok(Self::Form(FormSnapshotFrame {
                    hwnd,
                    x,
                    y,
                    width,
                    height,
                    style,
                    caption,
                    controls,
                }))
            }
            OPCODE_DIALOG => {
                let dialog_id = read_u32_le(&mut cur)?;
                let kind = read_u8(&mut cur)?;
                let buttons = read_u32_le(&mut cur)?;
                let title = read_str16(&mut cur)?;
                let prompt = read_str16(&mut cur)?;
                let default_text = read_str16(&mut cur)?;
                Ok(Self::Dialog(DialogFrame {
                    dialog_id,
                    kind,
                    buttons,
                    title,
                    prompt,
                    default_text,
                }))
            }
            OPCODE_CURSOR => {
                let cursor_id = read_u16_le(&mut cur)?;
                let x = read_i16_le(&mut cur)?;
                let y = read_i16_le(&mut cur)?;
                Ok(Self::Cursor(CursorFrame { cursor_id, x, y }))
            }
            OPCODE_CLIPBOARD => {
                let format = read_u16_le(&mut cur)?;
                let data = read_str32(&mut cur)?;
                Ok(Self::Clipboard(ClipboardFrame { format, data }))
            }
            OPCODE_TEXT => {
                let hwnd = read_u32_le(&mut cur)?;
                let x = read_i16_le(&mut cur)?;
                let y = read_i16_le(&mut cur)?;
                let color_rgb = read_u32_le(&mut cur)?;
                let bk_mode = read_u8(&mut cur)?;
                let font_height = read_i16_le(&mut cur)?;
                let text = read_str16(&mut cur)?;
                Ok(Self::Text(TextFrame {
                    hwnd,
                    x,
                    y,
                    color_rgb,
                    bk_mode,
                    font_height,
                    text,
                }))
            }
            other => Err(format!("Unknown wire opcode: {:#x}", other)),
        }
    }

    /// Snapshots a window from `cb_win32::VirtualWindowTree` into a 0x03 FRAME_FORM.
    pub fn snapshot_window(tree: &cb_win32::VirtualWindowTree, hwnd: u32) -> Option<Self> {
        let win = tree.get_window(hwnd)?;
        let mut controls = Vec::new();
        for &child_hwnd in &win.children {
            if let Some(child) = tree.get_window(child_hwnd) {
                controls.push(FormControl {
                    hwnd: child.hwnd,
                    kind: 0,
                    x: child.x as i16,
                    y: child.y as i16,
                    width: child.width.max(0) as u16,
                    height: child.height.max(0) as u16,
                    text: child.caption.clone(),
                });
            }
        }

        Some(Self::Form(FormSnapshotFrame {
            hwnd: win.hwnd,
            x: win.x as i16,
            y: win.y as i16,
            width: win.width.max(0) as u16,
            height: win.height.max(0) as u16,
            style: win.style,
            caption: win.caption.clone(),
            controls,
        }))
    }
}

// -----------------------------------------------------------------------------
// Binary Serialization Primitives
// -----------------------------------------------------------------------------

fn write_str16(out: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    let len = bytes.len().min(65535) as u16;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&bytes[..len as usize]);
}

fn write_str32(out: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    let len = bytes.len() as u32;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn read_u8(cur: &mut &[u8]) -> Result<u8, String> {
    if cur.is_empty() {
        return Err("Unexpected EOF reading u8".to_string());
    }
    let b = cur[0];
    *cur = &cur[1..];
    Ok(b)
}

fn read_u16_le(cur: &mut &[u8]) -> Result<u16, String> {
    if cur.len() < 2 {
        return Err("Unexpected EOF reading u16".to_string());
    }
    let v = u16::from_le_bytes([cur[0], cur[1]]);
    *cur = &cur[2..];
    Ok(v)
}

fn read_i16_le(cur: &mut &[u8]) -> Result<i16, String> {
    if cur.len() < 2 {
        return Err("Unexpected EOF reading i16".to_string());
    }
    let v = i16::from_le_bytes([cur[0], cur[1]]);
    *cur = &cur[2..];
    Ok(v)
}

fn read_u32_le(cur: &mut &[u8]) -> Result<u32, String> {
    if cur.len() < 4 {
        return Err("Unexpected EOF reading u32".to_string());
    }
    let v = u32::from_le_bytes([cur[0], cur[1], cur[2], cur[3]]);
    *cur = &cur[4..];
    Ok(v)
}

fn read_str16(cur: &mut &[u8]) -> Result<String, String> {
    let len = read_u16_le(cur)? as usize;
    if cur.len() < len {
        return Err("Unexpected EOF reading str16 data".to_string());
    }
    let s = std::str::from_utf8(&cur[..len])
        .map_err(|e| format!("Invalid UTF-8 in str16: {}", e))?
        .to_string();
    *cur = &cur[len..];
    Ok(s)
}

fn read_str32(cur: &mut &[u8]) -> Result<String, String> {
    let len = read_u32_le(cur)? as usize;
    if cur.len() < len {
        return Err("Unexpected EOF reading str32 data".to_string());
    }
    let s = std::str::from_utf8(&cur[..len])
        .map_err(|e| format!("Invalid UTF-8 in str32: {}", e))?
        .to_string();
    *cur = &cur[len..];
    Ok(s)
}
