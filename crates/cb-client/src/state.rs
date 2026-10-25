// Clean-room re-implementation. Behavior derived from publicly documented
// Milestone 3 Display Wire Protocol (Phase 3.6 & 3.7). No Microsoft or third-party source used.

//! Canvas client session coordinator and virtual display engine.
//!
//! Orchestrates linear memory surface blitting, delegated text queues, active form/control
//! snapshots, modal dialog states, and DOM input translation.

use std::collections::HashMap;
use crate::decoder::{ClientFrame, decode_frame, ClientFormSnapshot, ClientDialog};
use crate::surface::{ClientSurface, DirtyRect};
use crate::text::{TextQueue, TextDrawCommand};

/// High-level client engine maintaining the virtual Win32 canvas state.
#[derive(Debug)]
pub struct CanvasClientEngine {
    surface: ClientSurface,
    text_queue: TextQueue,
    forms: HashMap<u32, ClientFormSnapshot>,
    active_dialog: Option<ClientDialog>,
    cursor: (u16, i16, i16), // (cursor_id, x, y)
    clipboard_text: Option<String>,
    frames_processed: u64,
    bytes_received: u64,
}

impl CanvasClientEngine {
    /// Initializes a client canvas engine with given viewport dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            surface: ClientSurface::new(width, height),
            text_queue: TextQueue::new(),
            forms: HashMap::new(),
            active_dialog: None,
            cursor: (0, 0, 0),
            clipboard_text: None,
            frames_processed: 0,
            bytes_received: 0,
        }
    }

    #[inline]
    pub fn surface(&self) -> &ClientSurface {
        &self.surface
    }

    #[inline]
    pub fn surface_mut(&mut self) -> &mut ClientSurface {
        &mut self.surface
    }

    #[inline]
    pub fn text_queue(&self) -> &TextQueue {
        &self.text_queue
    }

    #[inline]
    pub fn text_queue_mut(&mut self) -> &mut TextQueue {
        &mut self.text_queue
    }

    #[inline]
    pub fn forms(&self) -> &HashMap<u32, ClientFormSnapshot> {
        &self.forms
    }

    #[inline]
    pub fn active_dialog(&self) -> Option<&ClientDialog> {
        self.active_dialog.as_ref()
    }

    #[inline]
    pub fn cursor(&self) -> (u16, i16, i16) {
        self.cursor
    }

    #[inline]
    pub fn clipboard_text(&self) -> Option<&str> {
        self.clipboard_text.as_deref()
    }

    #[inline]
    pub fn frames_processed(&self) -> u64 {
        self.frames_processed
    }

    #[inline]
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received
    }

    /// Ingests and processes a binary display frame received over the WebSocket.
    pub fn process_binary_message(&mut self, payload: &[u8]) -> Result<ClientFrame, String> {
        self.bytes_received += payload.len() as u64;
        self.frames_processed += 1;

        let frame = decode_frame(payload)?;
        match &frame {
            ClientFrame::BitmapRect(bmp) => {
                self.surface.blit_bgra_rect(
                    bmp.x,
                    bmp.y,
                    bmp.width,
                    bmp.height,
                    &bmp.bgra_pixels,
                );
            }
            ClientFrame::Form(form) => {
                self.forms.insert(form.hwnd, form.clone());
            }
            ClientFrame::Dialog(dialog) => {
                self.active_dialog = Some(dialog.clone());
            }
            ClientFrame::Cursor(cur) => {
                self.cursor = (cur.cursor_id, cur.x, cur.y);
            }
            ClientFrame::Clipboard(cb) => {
                self.clipboard_text = Some(cb.text.clone());
            }
            ClientFrame::Text(txt) => {
                let cmd = TextDrawCommand::new(
                    txt.hwnd,
                    txt.x,
                    txt.y,
                    txt.color_rgb,
                    txt.bk_mode,
                    txt.font_height,
                    txt.text.clone(),
                );
                self.text_queue.push(cmd);
            }
        }

        Ok(frame)
    }

    /// Consumes the accumulated dirty rectangle since the last flush.
    #[inline]
    pub fn take_dirty_rect(&mut self) -> Option<DirtyRect> {
        self.surface.take_dirty_rect()
    }

    /// Drains all pending text draw commands.
    #[inline]
    pub fn drain_text_commands(&mut self) -> Vec<TextDrawCommand> {
        self.text_queue.drain()
    }

    /// Resizes the virtual canvas to new dimensions.
    #[inline]
    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface.resize(width, height);
    }

    /// Dismisses the active modal dialog.
    #[inline]
    pub fn dismiss_dialog(&mut self) {
        self.active_dialog = None;
    }
}
