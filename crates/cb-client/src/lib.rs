// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # cb-client: WebAssembly HTML5 Canvas 2D Client Foundation
//!
//! Decodes binary display frames (`0x02`..`0x07`) streamed from `vb6-run --gui-web`
//! or `cb-webserver` and renders them onto an HTML5 Canvas 2D surface via linear memory
//! RGBA blitting (`ctx.putImageData`) and delegated vector text (`ctx.fillText`).
//! Captures browser DOM pointer/keyboard/resize events to send back to the Win32 message queue.

pub mod decoder;
pub mod surface;
pub mod text;
pub mod input;
pub mod state;
pub mod ffi;
pub mod assets;

pub use decoder::*;
pub use surface::*;
pub use text::*;
pub use input::*;
pub use state::*;
pub use ffi::*;
pub use assets::*;

/// Backwards-compatible client state representation.
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
