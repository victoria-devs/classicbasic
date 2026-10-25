// Clean-room re-implementation. Behavior derived from publicly documented
// RFC 8259 JSON & Win32 message semantics. No Microsoft or third-party source used.

//! Unit & Integration Tests for the Browser DOM Input Encoder.
//!
//! Verifies bidirectional compatibility between `cb-client` JSON output and
//! `cb-webserver::input::parse_client_input`.

use cb_client::input::InputEncoder;
use cb_webserver::input::{parse_client_input, ClientInputEvent};
use cb_win32::{
    WM_MOUSEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN,
    WM_KEYDOWN, WM_KEYUP, WM_CHAR, WM_SIZE,
};

#[test]
fn test_encode_and_parse_mouse_events() {
    // 1. Mouse Move
    let json_move = InputEncoder::mouse_move(101, 250, 175, 0);
    let parsed_move = parse_client_input(&json_move).expect("Parse mouse.move");
    match parsed_move {
        ClientInputEvent::Message(msg) => {
            assert_eq!(msg.hwnd, 101);
            assert_eq!(msg.message, WM_MOUSEMOVE);
            assert_eq!(msg.pt_x, 250);
            assert_eq!(msg.pt_y, 175);
            assert_eq!(msg.w_param, 0);
            let expected_lparam = (175 << 16) | (250 & 0xFFFF);
            assert_eq!(msg.l_param, expected_lparam as usize);
        }
        other => panic!("Unexpected event: {:?}", other),
    }

    // 2. Mouse Down (Left button = 0)
    let json_down = InputEncoder::mouse_down(101, 250, 175, 0);
    let parsed_down = parse_client_input(&json_down).expect("Parse mouse.down");
    match parsed_down {
        ClientInputEvent::Message(msg) => {
            assert_eq!(msg.hwnd, 101);
            assert_eq!(msg.message, WM_LBUTTONDOWN);
            assert_eq!(msg.w_param, 1); // MK_LBUTTON
        }
        other => panic!("Unexpected event: {:?}", other),
    }

    // 3. Mouse Up (Left button = 0)
    let json_up = InputEncoder::mouse_up(101, 250, 175, 0);
    let parsed_up = parse_client_input(&json_up).expect("Parse mouse.up");
    match parsed_up {
        ClientInputEvent::Message(msg) => {
            assert_eq!(msg.hwnd, 101);
            assert_eq!(msg.message, WM_LBUTTONUP);
            assert_eq!(msg.w_param, 0);
        }
        other => panic!("Unexpected event: {:?}", other),
    }

    // 4. Mouse Down (Right button = 2)
    let json_rdown = InputEncoder::mouse_down(101, 50, 50, 2);
    let parsed_rdown = parse_client_input(&json_rdown).expect("Parse right mouse.down");
    match parsed_rdown {
        ClientInputEvent::Message(msg) => {
            assert_eq!(msg.message, WM_RBUTTONDOWN);
            assert_eq!(msg.w_param, 2); // MK_RBUTTON
        }
        other => panic!("Unexpected event: {:?}", other),
    }
}

#[test]
fn test_encode_and_parse_keyboard_events() {
    // 1. Key Down (VK_RETURN = 13)
    let json_kd = InputEncoder::key_down(200, 13, 1);
    let parsed_kd = parse_client_input(&json_kd).expect("Parse key.down");
    match parsed_kd {
        ClientInputEvent::Message(msg) => {
            assert_eq!(msg.hwnd, 200);
            assert_eq!(msg.message, WM_KEYDOWN);
            assert_eq!(msg.w_param, 13);
            assert_eq!(msg.l_param, 1);
        }
        other => panic!("Unexpected event: {:?}", other),
    }

    // 2. Key Up (VK_RETURN = 13)
    let json_ku = InputEncoder::key_up(200, 13);
    let parsed_ku = parse_client_input(&json_ku).expect("Parse key.up");
    match parsed_ku {
        ClientInputEvent::Message(msg) => {
            assert_eq!(msg.hwnd, 200);
            assert_eq!(msg.message, WM_KEYUP);
            assert_eq!(msg.w_param, 13);
            assert_eq!(msg.l_param, 0xC0000001);
        }
        other => panic!("Unexpected event: {:?}", other),
    }

    // 3. Char event (ASCII 'A' = 65)
    let json_ch = InputEncoder::char_input(200, 65);
    let parsed_ch = parse_client_input(&json_ch).expect("Parse char");
    match parsed_ch {
        ClientInputEvent::Message(msg) => {
            assert_eq!(msg.hwnd, 200);
            assert_eq!(msg.message, WM_CHAR);
            assert_eq!(msg.w_param, 65);
        }
        other => panic!("Unexpected event: {:?}", other),
    }
}

#[test]
fn test_encode_and_parse_resize_and_dialog_events() {
    // Resize
    let json_resize = InputEncoder::resize(300, 1024, 768);
    let parsed_resize = parse_client_input(&json_resize).expect("Parse resize");
    match parsed_resize {
        ClientInputEvent::Message(msg) => {
            assert_eq!(msg.hwnd, 300);
            assert_eq!(msg.message, WM_SIZE);
            let expected_lparam = (768 << 16) | (1024 & 0xFFFF);
            assert_eq!(msg.l_param, expected_lparam as usize);
        }
        other => panic!("Unexpected event: {:?}", other),
    }

    // Dialog Result (MsgBox OK = 1)
    let json_diag = InputEncoder::dialog_result(55, 1, None);
    let parsed_diag = parse_client_input(&json_diag).expect("Parse dialog.result");
    match parsed_diag {
        ClientInputEvent::DialogResult(res) => {
            assert_eq!(res.dialog_id, 55);
            assert_eq!(res.result, 1);
            assert!(res.text.is_none());
        }
        other => panic!("Unexpected event: {:?}", other),
    }

    // Dialog Result with Text (InputBox)
    let json_diag_text = InputEncoder::dialog_result(56, 1, Some("Hello \"Antigravity\" \\ User\n"));
    let parsed_diag_text = parse_client_input(&json_diag_text).expect("Parse dialog.result text");
    match parsed_diag_text {
        ClientInputEvent::DialogResult(res) => {
            assert_eq!(res.dialog_id, 56);
            assert_eq!(res.result, 1);
            assert_eq!(res.text.as_deref(), Some("Hello \"Antigravity\" \\ User\n"));
        }
        other => panic!("Unexpected event: {:?}", other),
    }
}
