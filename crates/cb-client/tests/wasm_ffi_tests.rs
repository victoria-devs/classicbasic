// Clean-room re-implementation. Behavior derived from publicly documented
// WebAssembly Linear Memory C ABI semantics. No Microsoft or third-party source used.

//! Unit & Integration Tests for the WebAssembly FFI layer in `cb-client`.

use cb_client::ffi::*;
use cb_client::decoder::OPCODE_BITMAP_RECT;

#[test]
fn test_ffi_client_lifecycle_and_dimensions() {
    let client = cb_client_create(800, 600);
    assert!(!client.is_null());

    assert_eq!(cb_client_width(client), 800);
    assert_eq!(cb_client_height(client), 600);
    assert_eq!(cb_client_framebuffer_len(client), 800 * 600 * 4);

    let fb_ptr = cb_client_framebuffer_ptr(client);
    assert!(!fb_ptr.is_null());

    cb_client_destroy(client);
}

#[test]
fn test_ffi_memory_allocation_and_free() {
    let size = 1024u32;
    let ptr = cb_client_alloc(size);
    assert!(!ptr.is_null());

    // Write test data into allocated linear memory
    unsafe {
        for i in 0..size as usize {
            *ptr.add(i) = (i % 256) as u8;
        }
        for i in 0..size as usize {
            assert_eq!(*ptr.add(i), (i % 256) as u8);
        }
    }

    cb_client_free(ptr, size);
}

#[test]
fn test_ffi_process_frame_and_dirty_rect() {
    let client = cb_client_create(320, 240);
    assert!(!client.is_null());

    // Consume initial full dirty rect
    let mut out_rect = [0u32; 4];
    let had_initial = cb_client_dirty_rect(client, out_rect.as_mut_ptr());
    assert_eq!(had_initial, 1);
    assert_eq!(out_rect, [0, 0, 320, 240]);

    // Build a 10x10 BitmapRect payload at (20, 30)
    let mut payload = Vec::new();
    payload.push(OPCODE_BITMAP_RECT);
    payload.extend_from_slice(&1u32.to_le_bytes()); // hwnd
    payload.extend_from_slice(&20u16.to_le_bytes()); // x
    payload.extend_from_slice(&30u16.to_le_bytes()); // y
    payload.extend_from_slice(&10u16.to_le_bytes()); // w
    payload.extend_from_slice(&10u16.to_le_bytes()); // h
    // 100 pixels BGRA (all blue: 255, 0, 0, 255)
    let pixels = [255, 0, 0, 255].repeat(100);
    payload.extend_from_slice(&pixels);

    let res = cb_client_process_message(client, payload.as_ptr(), payload.len() as u32);
    assert_eq!(res, 0);

    // Query dirty rect via FFI
    let mut damage = [0u32; 4];
    let has_damage = cb_client_dirty_rect(client, damage.as_mut_ptr());
    assert_eq!(has_damage, 1);
    assert_eq!(damage, [20, 30, 10, 10]);

    // Subsequent call returns 0 (clean)
    assert_eq!(cb_client_dirty_rect(client, damage.as_mut_ptr()), 0);

    cb_client_destroy(client);
}

#[test]
fn test_ffi_text_drain_json() {
    let client = cb_client_create(640, 480);
    assert!(!client.is_null());

    assert_eq!(cb_client_text_command_count(client), 0);

    // Build text frame (opcode 0x07)
    let mut payload = Vec::new();
    payload.push(0x07);
    payload.extend_from_slice(&10u32.to_le_bytes());  // hwnd
    payload.extend_from_slice(&50i16.to_le_bytes());  // x
    payload.extend_from_slice(&75i16.to_le_bytes());  // y
    payload.extend_from_slice(&0x0000FF00u32.to_le_bytes()); // Green COLORREF
    payload.push(1); // TRANSPARENT
    payload.extend_from_slice(&(-14i16).to_le_bytes()); // 14px
    let txt = "Canvas Text FFI";
    payload.extend_from_slice(&(txt.len() as u16).to_le_bytes());
    payload.extend_from_slice(txt.as_bytes());

    let res = cb_client_process_message(client, payload.as_ptr(), payload.len() as u32);
    assert_eq!(res, 0);
    assert_eq!(cb_client_text_command_count(client), 1);

    // Drain text JSON via FFI
    let mut out_buf = vec![0u8; 1024];
    let len = cb_client_drain_text_json(client, out_buf.as_mut_ptr(), out_buf.len() as u32);
    assert!(len > 0);

    let json_str = std::str::from_utf8(&out_buf[..len as usize]).expect("UTF-8 JSON");
    assert!(json_str.contains("Canvas Text FFI"));
    assert!(json_str.contains("rgb(0, 255, 0)"));
    assert!(json_str.contains("14px sans-serif"));

    // After drain, count is 0
    assert_eq!(cb_client_text_command_count(client), 0);

    cb_client_destroy(client);
}

#[test]
fn test_ffi_input_encoders() {
    let mut buf = vec![0u8; 256];

    // Mouse move
    let len_move = cb_client_encode_mouse(0, 10, 100, 200, 1, buf.as_mut_ptr(), buf.len() as u32);
    assert!(len_move > 0);
    let str_move = std::str::from_utf8(&buf[..len_move as usize]).unwrap();
    assert_eq!(str_move, r#"{"type":"mouse.move","hwnd":10,"x":100,"y":200,"buttons":1}"#);

    // Key down
    let len_key = cb_client_encode_key(0, 20, 87, 1, buf.as_mut_ptr(), buf.len() as u32);
    assert!(len_key > 0);
    let str_key = std::str::from_utf8(&buf[..len_key as usize]).unwrap();
    assert_eq!(str_key, r#"{"type":"key.down","hwnd":20,"vk":87,"repeat":1}"#);

    // Resize
    let len_res = cb_client_encode_resize(30, 1920, 1080, buf.as_mut_ptr(), buf.len() as u32);
    assert!(len_res > 0);
    let str_res = std::str::from_utf8(&buf[..len_res as usize]).unwrap();
    assert_eq!(str_res, r#"{"type":"resize","hwnd":30,"width":1920,"height":1080}"#);
}
