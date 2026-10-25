// Clean-room re-implementation. Behavior derived from publicly documented
// Milestone 3 Display Wire Protocol (Phase 3.6 & 3.7). No Microsoft or third-party source used.

//! Binary display frame decoder for the ClassicBasic WebAssembly Canvas client.
//!
//! Decodes server-streamed binary WebSocket frames (`0x02`..`0x07`):
//! - `0x02 FRAME_BITMAP_RECT`: BGRA pixel rects blitted into client framebuffer.
//! - `0x03 FRAME_FORM`: Window geometry, caption, and controls snapshot.
//! - `0x04 FRAME_DIALOG`: Modal MsgBox / InputBox request.
//! - `0x05 FRAME_CURSOR`: Mouse cursor shape and coordinates.
//! - `0x06 FRAME_CLIPBOARD`: Text clipboard synchronization payload.
//! - `0x07 FRAME_TEXT`: Delegated vector text draw command.

pub const OPCODE_BITMAP_RECT: u8 = 0x02;
pub const OPCODE_FORM: u8 = 0x03;
pub const OPCODE_DIALOG: u8 = 0x04;
pub const OPCODE_CURSOR: u8 = 0x05;
pub const OPCODE_CLIPBOARD: u8 = 0x06;
pub const OPCODE_TEXT: u8 = 0x07;

/// 0x02 FRAME_BITMAP_RECT: dirty rectangular bitmap blit with 32-bit BGRA pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientBitmapRect {
    pub hwnd: u32,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub bgra_pixels: Vec<u8>,
}

/// A control descriptor embedded in a 0x03 FRAME_FORM snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientFormControl {
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
pub struct ClientFormSnapshot {
    pub hwnd: u32,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub style: u32,
    pub caption: String,
    pub controls: Vec<ClientFormControl>,
}

/// 0x04 FRAME_DIALOG: modal MsgBox / InputBox request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDialog {
    pub dialog_id: u32,
    pub kind: u8, // 0 = MsgBox, 1 = InputBox
    pub buttons: u32,
    pub title: String,
    pub prompt: String,
    pub default_text: String,
}

/// 0x05 FRAME_CURSOR: active mouse cursor ID and position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientCursor {
    pub cursor_id: u16,
    pub x: i16,
    pub y: i16,
}

/// 0x06 FRAME_CLIPBOARD: clipboard content payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientClipboard {
    pub format: u16, // 1 = CF_TEXT
    pub text: String,
}

/// 0x07 FRAME_TEXT: vector text rendering run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientTextRun {
    pub hwnd: u32,
    pub x: i16,
    pub y: i16,
    pub color_rgb: u32,
    pub bk_mode: u8, // 1 = TRANSPARENT, 2 = OPAQUE
    pub font_height: i16,
    pub text: String,
}

/// High-level parsed binary display frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientFrame {
    BitmapRect(ClientBitmapRect),
    Form(ClientFormSnapshot),
    Dialog(ClientDialog),
    Cursor(ClientCursor),
    Clipboard(ClientClipboard),
    Text(ClientTextRun),
}

/// Decodes a binary WebSocket display frame payload.
pub fn decode_frame(buf: &[u8]) -> Result<ClientFrame, String> {
    if buf.is_empty() {
        return Err("Buffer is empty, expected wire opcode".to_string());
    }

    let opcode = buf[0];
    let mut cur = &buf[1..];

    match opcode {
        OPCODE_BITMAP_RECT => {
            if cur.len() < 12 {
                return Err("Incomplete BitmapRect frame header".to_string());
            }
            let hwnd = read_u32_le(&mut cur)?;
            let x = read_u16_le(&mut cur)?;
            let y = read_u16_le(&mut cur)?;
            let width = read_u16_le(&mut cur)?;
            let height = read_u16_le(&mut cur)?;
            let expected_bytes = (width as usize) * (height as usize) * 4;
            if cur.len() < expected_bytes {
                return Err(format!(
                    "BitmapRect pixel shortfall: expected {} bytes, got {}",
                    expected_bytes,
                    cur.len()
                ));
            }
            let bgra_pixels = cur[..expected_bytes].to_vec();
            Ok(ClientFrame::BitmapRect(ClientBitmapRect {
                hwnd,
                x,
                y,
                width,
                height,
                bgra_pixels,
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
                controls.push(ClientFormControl {
                    hwnd: c_hwnd,
                    kind,
                    x: c_x,
                    y: c_y,
                    width: c_w,
                    height: c_h,
                    text,
                });
            }
            Ok(ClientFrame::Form(ClientFormSnapshot {
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
            Ok(ClientFrame::Dialog(ClientDialog {
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
            Ok(ClientFrame::Cursor(ClientCursor { cursor_id, x, y }))
        }
        OPCODE_CLIPBOARD => {
            let format = read_u16_le(&mut cur)?;
            let text = read_str32(&mut cur)?;
            Ok(ClientFrame::Clipboard(ClientClipboard { format, text }))
        }
        OPCODE_TEXT => {
            let hwnd = read_u32_le(&mut cur)?;
            let x = read_i16_le(&mut cur)?;
            let y = read_i16_le(&mut cur)?;
            let color_rgb = read_u32_le(&mut cur)?;
            let bk_mode = read_u8(&mut cur)?;
            let font_height = read_i16_le(&mut cur)?;
            let text = read_str16(&mut cur)?;
            Ok(ClientFrame::Text(ClientTextRun {
                hwnd,
                x,
                y,
                color_rgb,
                bk_mode,
                font_height,
                text,
            }))
        }
        unknown => Err(format!("Unknown wire opcode: 0x{:02X}", unknown)),
    }
}

// -----------------------------------------------------------------------------
// Stream Reading Helpers
// -----------------------------------------------------------------------------

fn read_u8(buf: &mut &[u8]) -> Result<u8, String> {
    if buf.is_empty() {
        return Err("Unexpected EOF reading u8".to_string());
    }
    let b = buf[0];
    *buf = &buf[1..];
    Ok(b)
}

fn read_u16_le(buf: &mut &[u8]) -> Result<u16, String> {
    if buf.len() < 2 {
        return Err("Unexpected EOF reading u16".to_string());
    }
    let val = u16::from_le_bytes([buf[0], buf[1]]);
    *buf = &buf[2..];
    Ok(val)
}

fn read_i16_le(buf: &mut &[u8]) -> Result<i16, String> {
    if buf.len() < 2 {
        return Err("Unexpected EOF reading i16".to_string());
    }
    let val = i16::from_le_bytes([buf[0], buf[1]]);
    *buf = &buf[2..];
    Ok(val)
}

fn read_u32_le(buf: &mut &[u8]) -> Result<u32, String> {
    if buf.len() < 4 {
        return Err("Unexpected EOF reading u32".to_string());
    }
    let val = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    *buf = &buf[4..];
    Ok(val)
}

fn read_str16(buf: &mut &[u8]) -> Result<String, String> {
    let len = read_u16_le(buf)? as usize;
    if buf.len() < len {
        return Err(format!(
            "Unexpected EOF reading string: expected {} bytes, got {}",
            len,
            buf.len()
        ));
    }
    let s = String::from_utf8_lossy(&buf[..len]).to_string();
    *buf = &buf[len..];
    Ok(s)
}

fn read_str32(buf: &mut &[u8]) -> Result<String, String> {
    let len = read_u32_le(buf)? as usize;
    if buf.len() < len {
        return Err(format!(
            "Unexpected EOF reading string: expected {} bytes, got {}",
            len,
            buf.len()
        ));
    }
    let s = String::from_utf8_lossy(&buf[..len]).to_string();
    *buf = &buf[len..];
    Ok(s)
}
