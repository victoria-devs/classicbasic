// Clean-room re-implementation. Behavior derived from publicly documented
// Win32 Button Control specification. No Microsoft or third-party source used.

//! Unit tests for `Win32ButtonControl` state transitions, hit testing, and 3D rendering.

use cb_win32::{
    Win32ButtonControl, BST_UNCHECKED,
};

#[test]
fn test_button_initial_state() {
    let btn = Win32ButtonControl::new(101, 100, 50, 80, 25, "OK");
    assert_eq!(btn.hwnd, 101);
    assert_eq!(btn.x, 100);
    assert_eq!(btn.y, 50);
    assert_eq!(btn.width, 80);
    assert_eq!(btn.height, 25);
    assert_eq!(btn.caption, "OK");
    assert_eq!(btn.state, BST_UNCHECKED);
    assert!(!btn.is_pushed());
    assert!(!btn.has_capture);
    assert_eq!(btn.text_offset(), (0, 0));
}

#[test]
fn test_button_hit_testing() {
    let btn = Win32ButtonControl::new(101, 100, 50, 80, 25, "OK");
    assert!(btn.hit_test(100, 50));
    assert!(btn.hit_test(179, 74));
    assert!(!btn.hit_test(99, 50));
    assert!(!btn.hit_test(180, 50));
    assert!(!btn.hit_test(100, 49));
    assert!(!btn.hit_test(100, 75));
}

#[test]
fn test_mouse_down_captures_and_depresses() {
    let mut btn = Win32ButtonControl::new(101, 100, 50, 80, 25, "Click me!");
    
    // Mouse down outside does nothing
    let changed = btn.on_lbutton_down(50, 50);
    assert!(!changed);
    assert!(!btn.has_capture);
    assert!(!btn.is_pushed());

    // Mouse down inside captures and pushes
    let changed = btn.on_lbutton_down(120, 60);
    assert!(changed);
    assert!(btn.has_capture);
    assert!(btn.is_pushed());
    assert_eq!(btn.text_offset(), (1, 1));
}

#[test]
fn test_mouse_move_tracking_inside_outside() {
    let mut btn = Win32ButtonControl::new(101, 100, 50, 80, 25, "Click me!");
    btn.on_lbutton_down(120, 60);
    assert!(btn.is_pushed());

    // Drag outside: button visually springs up (unpressed), but mouse capture is retained
    let changed = btn.on_mouse_move(50, 50);
    assert!(changed);
    assert!(btn.has_capture);
    assert!(!btn.is_pushed());
    assert_eq!(btn.text_offset(), (0, 0));

    // Drag back inside: button visually depresses again
    let changed = btn.on_mouse_move(130, 65);
    assert!(changed);
    assert!(btn.has_capture);
    assert!(btn.is_pushed());
    assert_eq!(btn.text_offset(), (1, 1));
}

#[test]
fn test_mouse_up_fires_click_only_when_released_inside() {
    let mut btn = Win32ButtonControl::new(101, 100, 50, 80, 25, "Click me!");

    // Case 1: Click down, release outside -> NO click event
    btn.on_lbutton_down(120, 60);
    btn.on_mouse_move(20, 20);
    let (_redraw, click_fired) = btn.on_lbutton_up(20, 20);
    assert!(!btn.has_capture);
    assert!(!btn.is_pushed());
    assert!(!click_fired);

    // Case 2: Click down, release inside -> CLICK EVENT FIRED!
    btn.on_lbutton_down(120, 60);
    let (redraw, click_fired) = btn.on_lbutton_up(125, 62);
    assert!(redraw);
    assert!(!btn.has_capture);
    assert!(!btn.is_pushed());
    assert!(click_fired);
}

#[test]
fn test_keyboard_space_and_return() {
    let mut btn = Win32ButtonControl::new(101, 100, 50, 80, 25, "OK");

    // Spacebar down: depressed
    let (redraw, click) = btn.on_key_down(32);
    assert!(redraw);
    assert!(!click);
    assert!(btn.is_pushed());
    assert_eq!(btn.text_offset(), (1, 1));

    // Spacebar up: released & clicked
    let (redraw, click) = btn.on_key_up(32);
    assert!(redraw);
    assert!(click);
    assert!(!btn.is_pushed());
    assert_eq!(btn.text_offset(), (0, 0));

    // Enter down: immediate click
    let (redraw, click) = btn.on_key_down(13);
    assert!(!redraw);
    assert!(click);
}

#[test]
fn test_render_pixels_unpressed_vs_pressed() {
    let mut btn = Win32ButtonControl::new(101, 0, 0, 10, 10, "Test");

    // 1. Unpressed: top-left is White (0xFF, 0xFF, 0xFF), bottom-right is Black (0x00, 0x00, 0x00)
    let p_unpressed = btn.render_pixels();
    assert_eq!(p_unpressed.len(), 10 * 10 * 4);
    // (0, 0) top-left
    assert_eq!(p_unpressed[0], 0xFF); // B
    assert_eq!(p_unpressed[1], 0xFF); // G
    assert_eq!(p_unpressed[2], 0xFF); // R
    // (9, 9) bottom-right
    let br_idx = (9 * 10 + 9) * 4;
    assert_eq!(p_unpressed[br_idx], 0x00); // B
    assert_eq!(p_unpressed[br_idx + 1], 0x00); // G
    assert_eq!(p_unpressed[br_idx + 2], 0x00); // R

    // 2. Pressed: top-left is Black (0x00, 0x00, 0x00), bottom-right is White (0xFF, 0xFF, 0xFF)
    btn.set_pushed(true);
    let p_pressed = btn.render_pixels();
    assert_eq!(p_pressed[0], 0x00); // B
    assert_eq!(p_pressed[1], 0x00); // G
    assert_eq!(p_pressed[2], 0x00); // R

    assert_eq!(p_pressed[br_idx], 0xFF); // B
    assert_eq!(p_pressed[br_idx + 1], 0xFF); // G
    assert_eq!(p_pressed[br_idx + 2], 0xFF); // R
}
