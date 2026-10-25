// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Unit & Integration Tests for `cb-net` Crate

use cb_net::*;

#[test]
fn test_wire_protocol_opcodes() {
    assert_eq!(FRAME_CREATE_WINDOW, 0x02);
    assert_eq!(FRAME_DESTROY_WINDOW, 0x03);
    assert_eq!(FRAME_BITMAP_RECT, 0x04);
    assert_eq!(FRAME_FILL_RECT, 0x05);
    assert_eq!(FRAME_DRAW_EDGE, 0x06);
    assert_eq!(FRAME_TEXT, 0x07);

    assert_eq!(INPUT_MOUSE_EVENT, 0x81);
    assert_eq!(INPUT_KEY_EVENT, 0x82);
    assert_eq!(INPUT_WINDOW_EVENT, 0x83);
}

#[test]
fn test_display_frame_variants() {
    let frame = DisplayFrame::CreateWindow {
        hwnd: 0x10001,
        parent: 0,
        x: 100,
        y: 100,
        width: 800,
        height: 600,
        style: 0xCF0000,
        caption: "Test Window".to_string(),
    };

    match frame {
        DisplayFrame::CreateWindow { hwnd, width, height, caption, .. } => {
            assert_eq!(hwnd, 0x10001);
            assert_eq!(width, 800);
            assert_eq!(height, 600);
            assert_eq!(caption, "Test Window");
        }
        _ => panic!("Unexpected frame variant"),
    }
}
