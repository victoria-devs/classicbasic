// Clean-room re-implementation. Behavior derived from publicly documented
// Milestone 3 Display Wire Protocol specifications. No Microsoft or third-party source used.

//! Unit tests for atomic binary wire protocol frames (0x02..0x07) and Form snapshot.

use cb_webserver::{
    DisplayFrame, BitmapRectFrame, FormSnapshotFrame, FormControl, DialogFrame,
    CursorFrame, ClipboardFrame, TextFrame,
    OPCODE_BITMAP_RECT, OPCODE_FORM, OPCODE_DIALOG, OPCODE_CURSOR, OPCODE_CLIPBOARD, OPCODE_TEXT,
};
use cb_win32::{VirtualWindowTree, RegisteredClass};

#[test]
fn test_bitmap_rect_frame_roundtrip() {
    let frame = DisplayFrame::BitmapRect(BitmapRectFrame {
        hwnd: 0x10001,
        x: 10,
        y: 20,
        width: 2,
        height: 2,
        pixels: vec![
            255, 0, 0, 255,  // red
            0, 255, 0, 255,  // green
            0, 0, 255, 255,  // blue
            255, 255, 255, 255, // white
        ],
    });

    let wire = frame.encode();
    assert_eq!(wire[0], OPCODE_BITMAP_RECT);

    let decoded = DisplayFrame::decode(&wire).unwrap();
    assert_eq!(frame, decoded);
}

#[test]
fn test_form_frame_roundtrip() {
    let frame = DisplayFrame::Form(FormSnapshotFrame {
        hwnd: 0x20001,
        x: 50,
        y: 60,
        width: 640,
        height: 480,
        style: 0xCF0000,
        caption: "Main Form Title".to_string(),
        controls: vec![
            FormControl {
                hwnd: 0x20002,
                kind: 1, // CommandButton
                x: 10,
                y: 10,
                width: 100,
                height: 30,
                text: "Click Me".to_string(),
            },
            FormControl {
                hwnd: 0x20003,
                kind: 2, // TextBox
                x: 10,
                y: 50,
                width: 200,
                height: 25,
                text: "Sample Text".to_string(),
            },
        ],
    });

    let wire = frame.encode();
    assert_eq!(wire[0], OPCODE_FORM);

    let decoded = DisplayFrame::decode(&wire).unwrap();
    assert_eq!(frame, decoded);
}

#[test]
fn test_dialog_frame_roundtrip() {
    let frame = DisplayFrame::Dialog(DialogFrame {
        dialog_id: 42,
        kind: 0, // MsgBox
        buttons: 1, // vbOKCancel
        title: "Confirm".to_string(),
        prompt: "Are you sure?".to_string(),
        default_text: "".to_string(),
    });

    let wire = frame.encode();
    assert_eq!(wire[0], OPCODE_DIALOG);

    let decoded = DisplayFrame::decode(&wire).unwrap();
    assert_eq!(frame, decoded);
}

#[test]
fn test_cursor_frame_roundtrip() {
    let frame = DisplayFrame::Cursor(CursorFrame {
        cursor_id: 32512, // IDC_ARROW
        x: 150,
        y: 275,
    });

    let wire = frame.encode();
    assert_eq!(wire[0], OPCODE_CURSOR);

    let decoded = DisplayFrame::decode(&wire).unwrap();
    assert_eq!(frame, decoded);
}

#[test]
fn test_clipboard_frame_roundtrip() {
    let frame = DisplayFrame::Clipboard(ClipboardFrame {
        format: 1, // CF_TEXT
        data: "ClassicBasic Clipboard Sync Data".to_string(),
    });

    let wire = frame.encode();
    assert_eq!(wire[0], OPCODE_CLIPBOARD);

    let decoded = DisplayFrame::decode(&wire).unwrap();
    assert_eq!(frame, decoded);
}

#[test]
fn test_text_frame_roundtrip() {
    let frame = DisplayFrame::Text(TextFrame {
        hwnd: 0x30001,
        x: 25,
        y: 40,
        color_rgb: 0x00FF8000,
        bk_mode: 1, // TRANSPARENT
        font_height: 16,
        text: "Vector Streamed Text".to_string(),
    });

    let wire = frame.encode();
    assert_eq!(wire[0], OPCODE_TEXT);

    let decoded = DisplayFrame::decode(&wire).unwrap();
    assert_eq!(frame, decoded);
}

#[test]
fn test_snapshot_window_from_tree() {
    let mut tree = VirtualWindowTree::new();
    tree.register_class(RegisteredClass {
        name: "TestFormClass".to_string(),
        style: 0,
        wndproc: 0,
        cb_cls_extra: 0,
        cb_wnd_extra: 0,
        h_instance: 0x400000,
        atom: 0xC001,
    });

    let hwnd_form = tree.create_window(
        0,
        "TestFormClass".to_string(),
        "Window Title".to_string(),
        0,
        100,
        150,
        500,
        400,
        0,
        0,
        0x400000,
    );

    let snapshot = DisplayFrame::snapshot_window(&tree, hwnd_form).unwrap();
    if let DisplayFrame::Form(form) = snapshot {
        assert_eq!(form.hwnd, hwnd_form);
        assert_eq!(form.x, 100);
        assert_eq!(form.y, 150);
        assert_eq!(form.width, 500);
        assert_eq!(form.height, 400);
        assert_eq!(form.caption, "Window Title");
    } else {
        panic!("Expected Form frame");
    }
}
