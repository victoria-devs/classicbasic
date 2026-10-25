// Clean-room re-implementation. Behavior derived from publicly documented
// Milestone 3 Display Wire Protocol (Phase 3.6 & 3.7). No Microsoft or third-party source used.

//! Unit & Integration Tests for the Display Frame Decoder in `cb-client`.

use cb_client::decoder::*;

#[test]
fn test_decode_bitmap_rect() {
    let mut payload = Vec::new();
    payload.push(OPCODE_BITMAP_RECT);
    payload.extend_from_slice(&1001u32.to_le_bytes()); // hwnd
    payload.extend_from_slice(&10u16.to_le_bytes());   // x
    payload.extend_from_slice(&20u16.to_le_bytes());   // y
    payload.extend_from_slice(&2u16.to_le_bytes());    // width
    payload.extend_from_slice(&2u16.to_le_bytes());    // height
    // 2x2 BGRA pixels
    let bgra = vec![
        0x11, 0x22, 0x33, 0xFF,
        0x44, 0x55, 0x66, 0xFF,
        0x77, 0x88, 0x99, 0xFF,
        0xAA, 0xBB, 0xCC, 0xFF,
    ];
    payload.extend_from_slice(&bgra);

    let frame = decode_frame(&payload).expect("Failed to decode valid BitmapRect");
    match frame {
        ClientFrame::BitmapRect(bmp) => {
            assert_eq!(bmp.hwnd, 1001);
            assert_eq!(bmp.x, 10);
            assert_eq!(bmp.y, 20);
            assert_eq!(bmp.width, 2);
            assert_eq!(bmp.height, 2);
            assert_eq!(bmp.bgra_pixels, bgra);
        }
        other => panic!("Unexpected frame variant: {:?}", other),
    }
}

#[test]
fn test_decode_truncated_bitmap_rect_errors() {
    let mut payload = Vec::new();
    payload.push(OPCODE_BITMAP_RECT);
    payload.extend_from_slice(&1001u32.to_le_bytes());
    payload.extend_from_slice(&10u16.to_le_bytes());
    payload.extend_from_slice(&20u16.to_le_bytes());
    payload.extend_from_slice(&2u16.to_le_bytes());
    payload.extend_from_slice(&2u16.to_le_bytes());
    // Missing pixels (only 4 bytes instead of 16)
    payload.extend_from_slice(&[0, 0, 0, 0]);

    let err = decode_frame(&payload).unwrap_err();
    assert!(err.contains("BitmapRect pixel shortfall"));
}

#[test]
fn test_decode_form_snapshot() {
    let mut payload = Vec::new();
    payload.push(OPCODE_FORM);
    payload.extend_from_slice(&500u32.to_le_bytes());  // hwnd
    payload.extend_from_slice(&100i16.to_le_bytes());  // x
    payload.extend_from_slice(&150i16.to_le_bytes());  // y
    payload.extend_from_slice(&640u16.to_le_bytes());  // width
    payload.extend_from_slice(&480u16.to_le_bytes());  // height
    payload.extend_from_slice(&0x00CF0000u32.to_le_bytes()); // style
    // Caption: "Main Window"
    let caption = "Main Window";
    payload.extend_from_slice(&(caption.len() as u16).to_le_bytes());
    payload.extend_from_slice(caption.as_bytes());

    // 1 Control
    payload.extend_from_slice(&1u16.to_le_bytes());
    payload.extend_from_slice(&501u32.to_le_bytes());  // c_hwnd
    payload.push(1); // Button
    payload.extend_from_slice(&10i16.to_le_bytes());
    payload.extend_from_slice(&20i16.to_le_bytes());
    payload.extend_from_slice(&80u16.to_le_bytes());
    payload.extend_from_slice(&30u16.to_le_bytes());
    let btn_text = "OK";
    payload.extend_from_slice(&(btn_text.len() as u16).to_le_bytes());
    payload.extend_from_slice(btn_text.as_bytes());

    let frame = decode_frame(&payload).expect("Failed to decode valid Form");
    match frame {
        ClientFrame::Form(f) => {
            assert_eq!(f.hwnd, 500);
            assert_eq!(f.x, 100);
            assert_eq!(f.y, 150);
            assert_eq!(f.width, 640);
            assert_eq!(f.height, 480);
            assert_eq!(f.caption, "Main Window");
            assert_eq!(f.controls.len(), 1);
            let c = &f.controls[0];
            assert_eq!(c.hwnd, 501);
            assert_eq!(c.kind, 1);
            assert_eq!(c.x, 10);
            assert_eq!(c.y, 20);
            assert_eq!(c.width, 80);
            assert_eq!(c.height, 30);
            assert_eq!(c.text, "OK");
        }
        other => panic!("Unexpected frame variant: {:?}", other),
    }
}

#[test]
fn test_decode_dialog() {
    let mut payload = Vec::new();
    payload.push(OPCODE_DIALOG);
    payload.extend_from_slice(&42u32.to_le_bytes()); // dialog_id
    payload.push(0); // MsgBox
    payload.extend_from_slice(&0x40u32.to_le_bytes()); // buttons (Information)
    let title = "Confirm";
    payload.extend_from_slice(&(title.len() as u16).to_le_bytes());
    payload.extend_from_slice(title.as_bytes());
    let prompt = "Proceed with calculation?";
    payload.extend_from_slice(&(prompt.len() as u16).to_le_bytes());
    payload.extend_from_slice(prompt.as_bytes());
    let def = "";
    payload.extend_from_slice(&(def.len() as u16).to_le_bytes());

    let frame = decode_frame(&payload).expect("Failed to decode valid Dialog");
    match frame {
        ClientFrame::Dialog(d) => {
            assert_eq!(d.dialog_id, 42);
            assert_eq!(d.kind, 0);
            assert_eq!(d.buttons, 0x40);
            assert_eq!(d.title, "Confirm");
            assert_eq!(d.prompt, "Proceed with calculation?");
            assert_eq!(d.default_text, "");
        }
        other => panic!("Unexpected frame variant: {:?}", other),
    }
}

#[test]
fn test_decode_cursor_and_clipboard() {
    // Cursor
    let mut cur_buf = Vec::new();
    cur_buf.push(OPCODE_CURSOR);
    cur_buf.extend_from_slice(&32512u16.to_le_bytes()); // IDC_ARROW
    cur_buf.extend_from_slice(&250i16.to_le_bytes());
    cur_buf.extend_from_slice(&180i16.to_le_bytes());

    let cur_frame = decode_frame(&cur_buf).expect("Decode cursor");
    match cur_frame {
        ClientFrame::Cursor(c) => {
            assert_eq!(c.cursor_id, 32512);
            assert_eq!(c.x, 250);
            assert_eq!(c.y, 180);
        }
        other => panic!("Unexpected cursor: {:?}", other),
    }

    // Clipboard
    let mut clip_buf = Vec::new();
    clip_buf.push(OPCODE_CLIPBOARD);
    clip_buf.extend_from_slice(&1u16.to_le_bytes()); // CF_TEXT
    let txt = "Copied text content";
    clip_buf.extend_from_slice(&(txt.len() as u32).to_le_bytes());
    clip_buf.extend_from_slice(txt.as_bytes());

    let clip_frame = decode_frame(&clip_buf).expect("Decode clipboard");
    match clip_frame {
        ClientFrame::Clipboard(cb) => {
            assert_eq!(cb.format, 1);
            assert_eq!(cb.text, "Copied text content");
        }
        other => panic!("Unexpected clipboard: {:?}", other),
    }
}

#[test]
fn test_decode_text_run() {
    let mut payload = Vec::new();
    payload.push(OPCODE_TEXT);
    payload.extend_from_slice(&777u32.to_le_bytes()); // hwnd
    payload.extend_from_slice(&15i16.to_le_bytes());  // x
    payload.extend_from_slice(&35i16.to_le_bytes());  // y
    payload.extend_from_slice(&0x00FF8800u32.to_le_bytes()); // color_rgb
    payload.push(1); // TRANSPARENT
    payload.extend_from_slice(&(-14i16).to_le_bytes()); // font_height
    let txt = "ClassicBasic Running";
    payload.extend_from_slice(&(txt.len() as u16).to_le_bytes());
    payload.extend_from_slice(txt.as_bytes());

    let frame = decode_frame(&payload).expect("Decode text");
    match frame {
        ClientFrame::Text(t) => {
            assert_eq!(t.hwnd, 777);
            assert_eq!(t.x, 15);
            assert_eq!(t.y, 35);
            assert_eq!(t.color_rgb, 0x00FF8800);
            assert_eq!(t.bk_mode, 1);
            assert_eq!(t.font_height, -14);
            assert_eq!(t.text, "ClassicBasic Running");
        }
        other => panic!("Unexpected text: {:?}", other),
    }
}

#[test]
fn test_decode_empty_or_unknown_errors() {
    assert!(decode_frame(&[]).is_err());
    assert!(decode_frame(&[0xFF, 0x00]).is_err());
}
