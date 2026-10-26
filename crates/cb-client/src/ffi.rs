// Clean-room re-implementation. Behavior derived from publicly documented
// WebAssembly Linear Memory C ABI & HTML5 Canvas 2D bridge semantics. No Microsoft or third-party source used.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

//! WebAssembly / C ABI exports for zero-dependency JavaScript interoperability.
//!
//! Provides bare FFI functions callable directly via `WebAssembly.Instance.exports`
//! without any runtime framework or heavy dependencies.

use crate::state::CanvasClientEngine;
use crate::input::InputEncoder;

/// Allocates raw byte buffer in linear memory with 8-byte alignment for host -> WASM data transfer.
#[no_mangle]
pub extern "C" fn cb_client_alloc(size: u32) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    unsafe {
        let layout = std::alloc::Layout::from_size_align_unchecked(size as usize, 8);
        std::alloc::alloc(layout)
    }
}

/// Frees a previously allocated linear memory buffer.
#[no_mangle]
pub extern "C" fn cb_client_free(ptr: *mut u8, size: u32) {
    if !ptr.is_null() && size > 0 {
        unsafe {
            let layout = std::alloc::Layout::from_size_align_unchecked(size as usize, 8);
            std::alloc::dealloc(ptr, layout);
        }
    }
}

/// Creates a new `CanvasClientEngine` instance.
#[no_mangle]
pub extern "C" fn cb_client_create(width: u32, height: u32) -> *mut CanvasClientEngine {
    let engine = Box::new(CanvasClientEngine::new(width, height));
    Box::into_raw(engine)
}

/// Destroys a previously instantiated `CanvasClientEngine`.
#[no_mangle]
pub extern "C" fn cb_client_destroy(ptr: *mut CanvasClientEngine) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Processes a binary display frame payload from JavaScript.
/// Returns 0 on success, -1 on invalid pointer/empty, -2 on decode error.
#[no_mangle]
pub extern "C" fn cb_client_process_message(
    ptr: *mut CanvasClientEngine,
    data: *const u8,
    len: u32,
) -> i32 {
    if ptr.is_null() || data.is_null() || len == 0 {
        return -1;
    }
    let engine = unsafe { &mut *ptr };
    let slice = unsafe { std::slice::from_raw_parts(data, len as usize) };
    match engine.process_binary_message(slice) {
        Ok(_) => 0,
        Err(_) => -2,
    }
}

/// Returns the raw pointer to the linear RGBA framebuffer for `ctx.putImageData`.
#[no_mangle]
pub extern "C" fn cb_client_framebuffer_ptr(ptr: *const CanvasClientEngine) -> *const u8 {
    if ptr.is_null() {
        return std::ptr::null();
    }
    let engine = unsafe { &*ptr };
    engine.surface().as_rgba_slice().as_ptr()
}

/// Returns the byte length of the linear RGBA framebuffer (`width * height * 4`).
#[no_mangle]
pub extern "C" fn cb_client_framebuffer_len(ptr: *const CanvasClientEngine) -> u32 {
    if ptr.is_null() {
        return 0;
    }
    let engine = unsafe { &*ptr };
    engine.surface().as_rgba_slice().len() as u32
}

/// Returns the surface desktop width.
#[no_mangle]
pub extern "C" fn cb_client_width(ptr: *const CanvasClientEngine) -> u32 {
    if ptr.is_null() {
        return 0;
    }
    let engine = unsafe { &*ptr };
    engine.surface().width()
}

/// Returns the surface desktop height.
#[no_mangle]
pub extern "C" fn cb_client_height(ptr: *const CanvasClientEngine) -> u32 {
    if ptr.is_null() {
        return 0;
    }
    let engine = unsafe { &*ptr };
    engine.surface().height()
}

/// Retrieves and clears the accumulated dirty rectangle.
/// Writes `[x, y, width, height]` to `out_rect` (4 x u32).
/// Returns 1 if a dirty rectangle existed, 0 if clean.
#[no_mangle]
pub extern "C" fn cb_client_dirty_rect(
    ptr: *mut CanvasClientEngine,
    out_rect: *mut u32,
) -> i32 {
    if ptr.is_null() || out_rect.is_null() {
        return 0;
    }
    let engine = unsafe { &mut *ptr };
    if let Some(rect) = engine.take_dirty_rect() {
        unsafe {
            *out_rect.add(0) = rect.x;
            *out_rect.add(1) = rect.y;
            *out_rect.add(2) = rect.width;
            *out_rect.add(3) = rect.height;
        }
        1
    } else {
        0
    }
}

/// Returns the number of pending text draw commands.
#[no_mangle]
pub extern "C" fn cb_client_text_command_count(ptr: *const CanvasClientEngine) -> u32 {
    if ptr.is_null() {
        return 0;
    }
    let engine = unsafe { &*ptr };
    engine.text_queue().len() as u32
}

/// Drains text commands and serializes them as a JSON array into `out_buf`.
/// Returns the number of bytes written, or -1 if buffer was too small.
#[no_mangle]
pub extern "C" fn cb_client_drain_text_json(
    ptr: *mut CanvasClientEngine,
    out_buf: *mut u8,
    max_len: u32,
) -> i32 {
    if ptr.is_null() || out_buf.is_null() || max_len == 0 {
        return -1;
    }
    let engine = unsafe { &mut *ptr };
    let cmds = engine.drain_text_commands();
    if cmds.is_empty() {
        let empty = b"[]";
        if (max_len as usize) < empty.len() {
            return -1;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(empty.as_ptr(), out_buf, empty.len());
        }
        return empty.len() as i32;
    }

    let mut json = String::from("[");
    for (i, c) in cmds.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        json.push_str(&format!(
            r#"{{"hwnd":{},"x":{},"y":{},"color":"{}","font":"{}","bk":{},"text":"{}"}}"#,
            c.hwnd,
            c.x,
            c.y,
            c.color_css,
            c.font_css,
            c.bk_mode,
            c.text.replace('\\', "\\\\").replace('"', "\\\"")
        ));
    }
    json.push(']');

    let bytes = json.as_bytes();
    if bytes.len() > max_len as usize {
        return -1;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf, bytes.len());
    }
    bytes.len() as i32
}

/// Encodes a mouse event into JSON in `out_buf`.
/// `event_type`: 0 = move, 1 = down, 2 = up.
#[no_mangle]
pub extern "C" fn cb_client_encode_mouse(
    event_type: u32,
    hwnd: u32,
    x: i32,
    y: i32,
    btn: u32,
    out_buf: *mut u8,
    max_len: u32,
) -> i32 {
    if out_buf.is_null() || max_len == 0 {
        return -1;
    }
    let json = match event_type {
        0 => InputEncoder::mouse_move(hwnd, x, y, btn),
        1 => InputEncoder::mouse_down(hwnd, x, y, btn),
        _ => InputEncoder::mouse_up(hwnd, x, y, btn),
    };
    write_str_to_buf(&json, out_buf, max_len)
}

/// Encodes a key event into JSON in `out_buf`.
/// `event_type`: 0 = down, 1 = up, 2 = char.
#[no_mangle]
pub extern "C" fn cb_client_encode_key(
    event_type: u32,
    hwnd: u32,
    vk_or_code: u32,
    repeat: u32,
    out_buf: *mut u8,
    max_len: u32,
) -> i32 {
    if out_buf.is_null() || max_len == 0 {
        return -1;
    }
    let json = match event_type {
        0 => InputEncoder::key_down(hwnd, vk_or_code as usize, repeat as usize),
        1 => InputEncoder::key_up(hwnd, vk_or_code as usize),
        _ => InputEncoder::char_input(hwnd, vk_or_code as usize),
    };
    write_str_to_buf(&json, out_buf, max_len)
}

/// Encodes a canvas resize event into JSON in `out_buf`.
#[no_mangle]
pub extern "C" fn cb_client_encode_resize(
    hwnd: u32,
    width: i32,
    height: i32,
    out_buf: *mut u8,
    max_len: u32,
) -> i32 {
    if out_buf.is_null() || max_len == 0 {
        return -1;
    }
    let json = InputEncoder::resize(hwnd, width, height);
    write_str_to_buf(&json, out_buf, max_len)
}

fn write_str_to_buf(s: &str, out_buf: *mut u8, max_len: u32) -> i32 {
    let bytes = s.as_bytes();
    if bytes.len() > max_len as usize {
        return -1;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf, bytes.len());
    }
    bytes.len() as i32
}
