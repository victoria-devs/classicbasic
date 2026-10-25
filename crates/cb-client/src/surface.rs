// Clean-room re-implementation. Behavior derived from publicly documented
// HTML5 Canvas 2D ImageData & GDI 32-bit BGRA surface blitting semantics. No Microsoft or third-party source used.

//! Linear memory RGBA surface and high-performance BGRA -> RGBA pixel blitter.
//!
//! Converts 32-bit Win32 BGRA framebuffer scanlines received via `0x02 FRAME_BITMAP_RECT`
//! into standard HTML5 Canvas 2D linear RGBA memory for direct ingestion by `ctx.putImageData`.

/// Axis-aligned bounding rectangle for dirty/damage tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DirtyRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl DirtyRect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Computes the bounding union of this rectangle and another dirty rectangle.
    pub fn union(&mut self, other: &DirtyRect) {
        if other.width == 0 || other.height == 0 {
            return;
        }
        if self.width == 0 || self.height == 0 {
            *self = *other;
            return;
        }

        let left = self.x.min(other.x);
        let top = self.y.min(other.y);
        let right = (self.x + self.width).max(other.x + other.width);
        let bottom = (self.y + self.height).max(other.y + other.height);

        self.x = left;
        self.y = top;
        self.width = right.saturating_sub(left);
        self.height = bottom.saturating_sub(top);
    }
}

/// Client-side 32-bit RGBA surface for Canvas 2D rendering.
#[derive(Debug, Clone)]
pub struct ClientSurface {
    width: u32,
    height: u32,
    rgba_buffer: Vec<u8>,
    dirty_rect: Option<DirtyRect>,
    blitted_rects: u64,
}

impl ClientSurface {
    /// Creates a new RGBA surface with the specified dimensions, initialized to opaque black.
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width as usize) * (height as usize) * 4;
        let mut rgba_buffer = vec![0u8; size];
        // Default to opaque alpha
        for i in (3..size).step_by(4) {
            rgba_buffer[i] = 255;
        }

        Self {
            width,
            height,
            rgba_buffer,
            dirty_rect: Some(DirtyRect::new(0, 0, width, height)),
            blitted_rects: 0,
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    pub fn blitted_rects(&self) -> u64 {
        self.blitted_rects
    }

    #[inline]
    pub fn as_rgba_slice(&self) -> &[u8] {
        &self.rgba_buffer
    }

    #[inline]
    pub fn as_rgba_mut_slice(&mut self) -> &mut [u8] {
        &mut self.rgba_buffer
    }

    /// Retrieves the RGBA pixel value at (x, y) if within bounds.
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let offset = ((y * self.width + x) * 4) as usize;
        Some([
            self.rgba_buffer[offset],
            self.rgba_buffer[offset + 1],
            self.rgba_buffer[offset + 2],
            self.rgba_buffer[offset + 3],
        ])
    }

    /// Sets the RGBA pixel value at (x, y) if within bounds.
    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let offset = ((y * self.width + x) * 4) as usize;
        self.rgba_buffer[offset] = r;
        self.rgba_buffer[offset + 1] = g;
        self.rgba_buffer[offset + 2] = b;
        self.rgba_buffer[offset + 3] = a;

        let rect = DirtyRect::new(x, y, 1, 1);
        if let Some(ref mut d) = self.dirty_rect {
            d.union(&rect);
        } else {
            self.dirty_rect = Some(rect);
        }
    }

    /// Clears the entire surface with the given RGBA color and marks it dirty.
    pub fn clear(&mut self, r: u8, g: u8, b: u8, a: u8) {
        let len = self.rgba_buffer.len();
        let mut i = 0;
        while i + 3 < len {
            self.rgba_buffer[i] = r;
            self.rgba_buffer[i + 1] = g;
            self.rgba_buffer[i + 2] = b;
            self.rgba_buffer[i + 3] = a;
            i += 4;
        }
        self.dirty_rect = Some(DirtyRect::new(0, 0, self.width, self.height));
    }

    /// Resizes the surface to new dimensions. Preserves old content where overlapping.
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if self.width == new_width && self.height == new_height {
            return;
        }
        let new_size = (new_width as usize) * (new_height as usize) * 4;
        let mut new_buf = vec![0u8; new_size];
        for i in (3..new_size).step_by(4) {
            new_buf[i] = 255;
        }

        let copy_w = self.width.min(new_width);
        let copy_h = self.height.min(new_height);

        for row in 0..copy_h {
            let src_start = ((row * self.width) * 4) as usize;
            let src_end = src_start + (copy_w * 4) as usize;
            let dst_start = ((row * new_width) * 4) as usize;
            let dst_end = dst_start + (copy_w * 4) as usize;
            new_buf[dst_start..dst_end].copy_from_slice(&self.rgba_buffer[src_start..src_end]);
        }

        self.width = new_width;
        self.height = new_height;
        self.rgba_buffer = new_buf;
        self.dirty_rect = Some(DirtyRect::new(0, 0, new_width, new_height));
    }

    /// Blits a dirty rectangular block of 32-bit BGRA pixels into the RGBA framebuffer.
    ///
    /// Performs in-place BGRA -> RGBA channel swizzling:
    /// `R = BGRA[2]`, `G = BGRA[1]`, `B = BGRA[0]`, `A = BGRA[3]`.
    /// Clips rect to surface boundaries safely.
    pub fn blit_bgra_rect(&mut self, x: u16, y: u16, width: u16, height: u16, bgra_pixels: &[u8]) {
        if width == 0 || height == 0 || bgra_pixels.is_empty() {
            return;
        }

        let expected_len = (width as usize) * (height as usize) * 4;
        if bgra_pixels.len() < expected_len {
            return;
        }

        let x_u32 = x as u32;
        let y_u32 = y as u32;
        let w_u32 = width as u32;
        let h_u32 = height as u32;

        if x_u32 >= self.width || y_u32 >= self.height {
            return;
        }

        let visible_w = (self.width - x_u32).min(w_u32);
        let visible_h = (self.height - y_u32).min(h_u32);

        for row in 0..visible_h {
            let src_row_start = ((row * w_u32) * 4) as usize;
            let dst_row_start = (((y_u32 + row) * self.width + x_u32) * 4) as usize;

            for px in 0..visible_w {
                let s = src_row_start + (px * 4) as usize;
                let d = dst_row_start + (px * 4) as usize;

                let b = bgra_pixels[s];
                let g = bgra_pixels[s + 1];
                let r = bgra_pixels[s + 2];
                let a = bgra_pixels[s + 3];

                self.rgba_buffer[d] = r;
                self.rgba_buffer[d + 1] = g;
                self.rgba_buffer[d + 2] = b;
                self.rgba_buffer[d + 3] = a;
            }
        }

        let damaged = DirtyRect::new(x_u32, y_u32, visible_w, visible_h);
        if let Some(ref mut d) = self.dirty_rect {
            d.union(&damaged);
        } else {
            self.dirty_rect = Some(damaged);
        }

        self.blitted_rects += 1;
    }

    /// Takes the current accumulated dirty rectangle, resetting dirty tracking to `None`.
    pub fn take_dirty_rect(&mut self) -> Option<DirtyRect> {
        self.dirty_rect.take()
    }

    /// Queries the current dirty rectangle without consuming it.
    pub fn peek_dirty_rect(&self) -> Option<DirtyRect> {
        self.dirty_rect
    }
}
