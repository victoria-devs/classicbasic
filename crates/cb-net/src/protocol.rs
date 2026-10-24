// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Binary Display & Input Wire Protocol Definitions (Milestone 3).

/// Server -> Client frame opcode: Create or update window geometry/title.
pub const FRAME_CREATE_WINDOW: u8 = 0x02;
/// Server -> Client frame opcode: Destroy window.
pub const FRAME_DESTROY_WINDOW: u8 = 0x03;
/// Server -> Client frame opcode: Dirty RGBA bitmap rectangle update.
pub const FRAME_BITMAP_RECT: u8 = 0x04;
/// Server -> Client frame opcode: Solid rectangle fill primitive.
pub const FRAME_FILL_RECT: u8 = 0x05;
/// Server -> Client frame opcode: Classic 3D border bevel primitive.
pub const FRAME_DRAW_EDGE: u8 = 0x06;
/// Server -> Client frame opcode: Vector text draw primitive.
pub const FRAME_TEXT: u8 = 0x07;

/// Client -> Server input opcode: Mouse button down/up/move.
pub const INPUT_MOUSE_EVENT: u8 = 0x81;
/// Client -> Server input opcode: Keyboard key down/up/char.
pub const INPUT_KEY_EVENT: u8 = 0x82;
/// Client -> Server input opcode: Window close/resize/focus notification.
pub const INPUT_WINDOW_EVENT: u8 = 0x83;

/// High-level display frame emitted by the `cb-win32` compositor over `cb-net`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayFrame {
    CreateWindow {
        hwnd: u32,
        parent: u32,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
        style: u32,
        caption: String,
    },
    DestroyWindow {
        hwnd: u32,
    },
    BitmapRect {
        hwnd: u32,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        rgba: Vec<u8>,
    },
    FillRect {
        hwnd: u32,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
        color_rgba: u32,
    },
}
