// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Win32 `GDI32.DLL` Software Rasterizer, `HDC` & `Surface32` Foundation (Milestone 3).

/// 32-bit Device Context Handle (`HDC`).
pub type Hdc = u32;

/// 32-bit GDI Object Handle (`HGDIOBJ`: `HBRUSH`, `HPEN`, `HFONT`, `HBITMAP`).
pub type HGdioBj = u32;

/// Bounding box for dirty region tracking on a `Surface32` framebuffer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirtyRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// 32-bit RGBA in-memory software framebuffer for a window or off-screen bitmap.
#[derive(Debug, Clone)]
pub struct Surface32 {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
    pub dirty_regions: Vec<DirtyRect>,
}

impl Surface32 {
    /// Creates a new `width x height` 32-bit RGBA surface initialized to transparent black.
    pub fn new(width: u16, height: u16) -> Self {
        let len = (width as usize) * (height as usize) * 4;
        Self {
            width,
            height,
            pixels: vec![0u8; len],
            dirty_regions: Vec::new(),
        }
    }
}
