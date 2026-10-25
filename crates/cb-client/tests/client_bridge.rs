// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Unit & Integration Tests for `cb-client` Crate

use cb_client::*;

#[test]
fn test_canvas_client_state() {
    let mut state = CanvasClientState::new(1920, 1080);
    assert_eq!(state.desktop_width, 1920);
    assert_eq!(state.desktop_height, 1080);
    assert_eq!(state.frames_processed, 0);

    state.frames_processed += 1;
    assert_eq!(state.frames_processed, 1);
}
