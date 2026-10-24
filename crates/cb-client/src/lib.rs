// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # cb-client: WebAssembly HTML5 Canvas 2D Client Foundation
//!
//! Decodes binary display frames (`cb_net::DisplayFrame`) streamed from `vb6-run --gui-web`
//! or `cb-session` and renders them onto an HTML5 Canvas 2D surface while capturing DOM
//! mouse/keyboard events for transmission back to the Win32 message queue.

pub use cb_net::protocol::*;

/// Client-side state for a remote Win32 session canvas.
#[derive(Debug, Default)]
pub struct CanvasClientState {
    pub desktop_width: u16,
    pub desktop_height: u16,
    pub frames_processed: u64,
}

impl CanvasClientState {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            desktop_width: width,
            desktop_height: height,
            frames_processed: 0,
        }
    }
}
