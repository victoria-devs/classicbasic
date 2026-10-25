// Clean-room re-implementation. Behavior derived from publicly documented
// RFC 8259 and Win32 message semantics. No Microsoft or third-party source used.

//! Unit tests for JSON input frame parsing and Win32 message queue injection.

use cb_webserver::{parse_client_input, inject_client_input, ClientInputEvent};
use cb_win32::{
    VirtualMessageQueue,
    WM_MOUSEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_KEYDOWN, WM_KEYUP, WM_CHAR, WM_SIZE,
};

#[test]
fn test_parse_mouse_events() {
    let json_move = r#"{"type": "mouse.move", "hwnd": 65537, "x": 100, "y": 200, "buttons": 1}"#;
    let ev = parse_client_input(json_move).unwrap();
    if let ClientInputEvent::Message(msg) = ev {
        assert_eq!(msg.hwnd, 65537);
        assert_eq!(msg.message, WM_MOUSEMOVE);
        assert_eq!(msg.w_param, 1);
        assert_eq!(msg.pt_x, 100);
        assert_eq!(msg.pt_y, 200);
        let lo = (msg.l_param & 0xFFFF) as i16;
        let hi = ((msg.l_param >> 16) & 0xFFFF) as i16;
        assert_eq!(lo, 100);
        assert_eq!(hi, 200);
    } else {
        panic!("Expected message event");
    }

    let json_down = r#"{"type": "mouse.down", "hwnd": 65537, "x": 50, "y": 75, "button": 0}"#;
    let ev_down = parse_client_input(json_down).unwrap();
    if let ClientInputEvent::Message(msg) = ev_down {
        assert_eq!(msg.message, WM_LBUTTONDOWN);
        assert_eq!(msg.w_param, 1); // MK_LBUTTON
    } else {
        panic!("Expected mouse down event");
    }

    let json_up = r#"{"type": "mouse.up", "hwnd": 65537, "x": 50, "y": 75, "button": 0}"#;
    let ev_up = parse_client_input(json_up).unwrap();
    if let ClientInputEvent::Message(msg) = ev_up {
        assert_eq!(msg.message, WM_LBUTTONUP);
        assert_eq!(msg.w_param, 0);
    } else {
        panic!("Expected mouse up event");
    }
}

#[test]
fn test_parse_keyboard_and_char_events() {
    let json_key_down = r#"{"type": "key.down", "hwnd": 1234, "vk": 65, "repeat": 1}"#;
    let ev_kd = parse_client_input(json_key_down).unwrap();
    if let ClientInputEvent::Message(msg) = ev_kd {
        assert_eq!(msg.message, WM_KEYDOWN);
        assert_eq!(msg.w_param, 65);
        assert_eq!(msg.l_param, 1);
    } else {
        panic!("Expected key down event");
    }

    let json_key_up = r#"{"type": "key.up", "hwnd": 1234, "vk": 65}"#;
    let ev_ku = parse_client_input(json_key_up).unwrap();
    if let ClientInputEvent::Message(msg) = ev_ku {
        assert_eq!(msg.message, WM_KEYUP);
        assert_eq!(msg.w_param, 65);
        assert_eq!(msg.l_param, 0xC0000001);
    } else {
        panic!("Expected key up event");
    }

    let json_char = r#"{"type": "char", "hwnd": 1234, "code": 65}"#;
    let ev_char = parse_client_input(json_char).unwrap();
    if let ClientInputEvent::Message(msg) = ev_char {
        assert_eq!(msg.message, WM_CHAR);
        assert_eq!(msg.w_param, 65);
    } else {
        panic!("Expected char event");
    }
}

#[test]
fn test_parse_resize_and_dialog_result_events() {
    let json_resize = r#"{"type": "resize", "hwnd": 100, "width": 800, "height": 600}"#;
    let ev_res = parse_client_input(json_resize).unwrap();
    if let ClientInputEvent::Message(msg) = ev_res {
        assert_eq!(msg.message, WM_SIZE);
        let w = (msg.l_param & 0xFFFF) as i32;
        let h = ((msg.l_param >> 16) & 0xFFFF) as i32;
        assert_eq!(w, 800);
        assert_eq!(h, 600);
    } else {
        panic!("Expected resize message");
    }

    let json_dialog = r#"{"type": "dialog.result", "dialog_id": 99, "result": 1, "text": "Entered text"}"#;
    let ev_diag = parse_client_input(json_dialog).unwrap();
    if let ClientInputEvent::DialogResult(d) = ev_diag {
        assert_eq!(d.dialog_id, 99);
        assert_eq!(d.result, 1);
        assert_eq!(d.text.as_deref(), Some("Entered text"));
    } else {
        panic!("Expected dialog result");
    }
}

#[test]
fn test_injection_into_message_queue() {
    let queue = VirtualMessageQueue::new();
    let json_down = r#"{"type": "mouse.down", "hwnd": 42, "x": 10, "y": 20, "button": 0}"#;
    let ev = parse_client_input(json_down).unwrap();
    assert!(inject_client_input(ev, &queue).is_none());

    let out_msg = queue.peek_message(42, 0, 0, true).expect("Message should be in queue");
    assert_eq!(out_msg.hwnd, 42);
    assert_eq!(out_msg.message, WM_LBUTTONDOWN);
}
