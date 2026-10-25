// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Win32 `GDI32.DLL` Software Rasterizer, `HDC` & `Surface32` Foundation (Milestone 3).

use crate::handles::HandleTable;
use crate::user32::{window_tree, Hwnd, Point};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// 32-bit Device Context Handle (`HDC`).
pub type Hdc = u32;

/// 32-bit GDI Object Handle (`HGDIOBJ`: `HBRUSH`, `HPEN`, `HFONT`, `HBITMAP`).
pub type HGdioBj = u32;

// ---------------------------------------------------------------------------
// Standard Stock Object Constants
// ---------------------------------------------------------------------------
pub const WHITE_BRUSH: u32 = 0;
pub const LTGRAY_BRUSH: u32 = 1;
pub const GRAY_BRUSH: u32 = 2;
pub const DKGRAY_BRUSH: u32 = 3;
pub const BLACK_BRUSH: u32 = 4;
pub const NULL_BRUSH: u32 = 5;
pub const WHITE_PEN: u32 = 6;
pub const BLACK_PEN: u32 = 7;
pub const NULL_PEN: u32 = 8;

pub const STOCK_OBJECT_BASE: u32 = 0x8000_0000;

// ---------------------------------------------------------------------------
// Pen Styles
// ---------------------------------------------------------------------------
pub const PS_SOLID: u32 = 0;
pub const PS_DASH: u32 = 1;
pub const PS_DOT: u32 = 2;
pub const PS_NULL: u32 = 5;

// ---------------------------------------------------------------------------
// Raster Operations (ROP)
// ---------------------------------------------------------------------------
pub const BLACKNESS: u32 = 0x0000_0042;
pub const WHITENESS: u32 = 0x00FF_0062;
pub const SRCCOPY: u32 = 0x00CC_0020;
pub const SRCPAINT: u32 = 0x00EE_0086;
pub const SRCAND: u32 = 0x0088_00C6;
pub const SRCINVERT: u32 = 0x0066_0046;
pub const PATCOPY: u32 = 0x00F0_0021;

/// Bounding box for dirty region tracking on a `Surface32` framebuffer.
#[repr(C)]
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
        let w = width.max(1);
        let h = height.max(1);
        let len = (w as usize) * (h as usize) * 4;
        Self {
            width: w,
            height: h,
            pixels: vec![0u8; len],
            dirty_regions: Vec::new(),
        }
    }

    /// Records a region as dirty.
    pub fn mark_dirty(&mut self, x: i32, y: i32, w: i32, h: i32) {
        if w <= 0 || h <= 0 {
            return;
        }
        let clamp_x = x.clamp(0, self.width as i32) as u16;
        let clamp_y = y.clamp(0, self.height as i32) as u16;
        let clamp_w = (w as u16).min(self.width.saturating_sub(clamp_x));
        let clamp_h = (h as u16).min(self.height.saturating_sub(clamp_y));
        if clamp_w > 0 && clamp_h > 0 {
            self.dirty_regions.push(DirtyRect {
                x: clamp_x,
                y: clamp_y,
                width: clamp_w,
                height: clamp_h,
            });
        }
    }

    /// Clears recorded dirty regions.
    pub fn clear_dirty(&mut self) {
        self.dirty_regions.clear();
    }

    /// Sets pixel at `(x, y)` to COLORREF `cr_color` (`0x00BBGGRR`).
    pub fn set_pixel(&mut self, x: i32, y: i32, cr_color: u32) -> bool {
        if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
            return false;
        }
        let offset = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        let r = (cr_color & 0xFF) as u8;
        let g = ((cr_color >> 8) & 0xFF) as u8;
        let b = ((cr_color >> 16) & 0xFF) as u8;
        self.pixels[offset] = r;
        self.pixels[offset + 1] = g;
        self.pixels[offset + 2] = b;
        self.pixels[offset + 3] = 0xFF;
        self.mark_dirty(x, y, 1, 1);
        true
    }

    /// Gets pixel at `(x, y)` as COLORREF `0x00BBGGRR`. Returns `0xFFFFFFFF` if out of bounds.
    pub fn get_pixel(&self, x: i32, y: i32) -> u32 {
        if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
            return 0xFFFF_FFFF;
        }
        let offset = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        let r = self.pixels[offset] as u32;
        let g = self.pixels[offset + 1] as u32;
        let b = self.pixels[offset + 2] as u32;
        r | (g << 8) | (b << 16)
    }

    /// Draws a line from `(x0, y0)` to `(x1, y1)` using Bresenham's algorithm.
    pub fn draw_line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u32) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.set_pixel(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                if x0 == x1 {
                    break;
                }
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                if y0 == y1 {
                    break;
                }
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Draws a rectangle `[left, top]` to `[right, bottom]`.
    pub fn draw_rectangle(
        &mut self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        pen_color: Option<u32>,
        brush_color: Option<u32>,
    ) {
        let min_x = left.min(right);
        let max_x = left.max(right);
        let min_y = top.min(bottom);
        let max_y = top.max(bottom);

        // Fill interior
        if let Some(bc) = brush_color {
            for y in (min_y + 1)..max_y {
                for x in (min_x + 1)..max_x {
                    self.set_pixel(x, y, bc);
                }
            }
        }

        // Draw border
        if let Some(pc) = pen_color {
            for x in min_x..max_x {
                self.set_pixel(x, min_y, pc);
                self.set_pixel(x, (max_y - 1).max(min_y), pc);
            }
            for y in min_y..max_y {
                self.set_pixel(min_x, y, pc);
                self.set_pixel((max_x - 1).max(min_x), y, pc);
            }
        }
    }

    /// Draws an ellipse inside bounding box `[left, top]` to `[right, bottom]`.
    pub fn draw_ellipse(
        &mut self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        pen_color: Option<u32>,
        brush_color: Option<u32>,
    ) {
        let min_x = left.min(right);
        let max_x = left.max(right);
        let min_y = top.min(bottom);
        let max_y = top.max(bottom);

        let rx = (max_x - min_x) as f64 / 2.0;
        let ry = (max_y - min_y) as f64 / 2.0;
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }
        let cx = min_x as f64 + rx;
        let cy = min_y as f64 + ry;

        let border_dist = 1.0 - (1.0 / rx.min(ry)).min(0.5);

        for y in min_y..max_y {
            for x in min_x..max_x {
                let dx = (x as f64 + 0.5 - cx) / rx;
                let dy = (y as f64 + 0.5 - cy) / ry;
                let d = dx * dx + dy * dy;
                if d <= 1.0 {
                    if let Some(pc) = pen_color {
                        if d >= border_dist * border_dist {
                            self.set_pixel(x, y, pc);
                            continue;
                        }
                    }
                    if let Some(bc) = brush_color {
                        self.set_pixel(x, y, bc);
                    }
                }
            }
        }
    }

    /// Draws a rounded rectangle.
    pub fn draw_round_rect(
        &mut self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        _w: i32,
        _h: i32,
        pen_color: Option<u32>,
        brush_color: Option<u32>,
    ) {
        // Approximated as rectangle with clamped border
        self.draw_rectangle(left, top, right, bottom, pen_color, brush_color);
    }

    /// Fills rectangle `[left..right, top..bottom]` with solid color.
    pub fn fill_rect(&mut self, left: i32, top: i32, right: i32, bottom: i32, color: u32) {
        let min_x = left.min(right);
        let max_x = left.max(right);
        let min_y = top.min(bottom);
        let max_y = top.max(bottom);
        for y in min_y..max_y {
            for x in min_x..max_x {
                self.set_pixel(x, y, color);
            }
        }
    }

    /// Frames rectangle `[left..right, top..bottom]` with a 1-pixel border.
    pub fn frame_rect(&mut self, left: i32, top: i32, right: i32, bottom: i32, color: u32) {
        let min_x = left.min(right);
        let max_x = left.max(right);
        let min_y = top.min(bottom);
        let max_y = top.max(bottom);
        for x in min_x..max_x {
            self.set_pixel(x, min_y, color);
            self.set_pixel(x, (max_y - 1).max(min_y), color);
        }
        for y in min_y..max_y {
            self.set_pixel(min_x, y, color);
            self.set_pixel((max_x - 1).max(min_x), y, color);
        }
    }

    /// Pattern blit with ROP.
    pub fn pat_blt(&mut self, x: i32, y: i32, w: i32, h: i32, rop: u32, brush_color: u32) {
        let fill_color = match rop {
            WHITENESS => 0x00FF_FFFF,
            BLACKNESS => 0x0000_0000,
            _ => brush_color, // PATCOPY
        };
        self.fill_rect(x, y, x + w, y + h, fill_color);
    }

    /// Bit blit with ROP.
    pub fn bit_blt(
        &mut self,
        dst_x: i32,
        dst_y: i32,
        w: i32,
        h: i32,
        src: &Surface32,
        src_x: i32,
        src_y: i32,
        rop: u32,
    ) {
        for j in 0..h {
            for i in 0..w {
                let s_x = src_x + i;
                let s_y = src_y + j;
                let d_x = dst_x + i;
                let d_y = dst_y + j;

                let src_c = src.get_pixel(s_x, s_y);
                let dst_c = self.get_pixel(d_x, d_y);

                let final_c = match rop {
                    WHITENESS => 0x00FF_FFFF,
                    BLACKNESS => 0x0000_0000,
                    SRCPAINT => dst_c | src_c,
                    SRCAND => dst_c & src_c,
                    SRCINVERT => dst_c ^ src_c,
                    _ => src_c, // SRCCOPY
                };
                self.set_pixel(d_x, d_y, final_c);
            }
        }
    }
}

/// GDI Object representation (`Pen`, `Brush`, `Bitmap`).
#[derive(Debug, Clone)]
pub enum GdiObject {
    Pen {
        style: u32,
        width: i32,
        color: u32,
    },
    Brush {
        style: u32,
        color: u32,
    },
    Bitmap {
        width: u16,
        height: u16,
        surface: Surface32,
    },
}

/// Virtual Device Context (`HDC`).
#[derive(Debug, Clone)]
pub struct VirtualDC {
    pub hdc: Hdc,
    pub hwnd: Option<Hwnd>,
    pub selected_pen: u32,
    pub selected_brush: u32,
    pub selected_bitmap: u32,
    pub pen_pos: (i32, i32),
    pub mem_surface: Option<Surface32>,
}

/// Global GDI State managing DCs and GDI Objects.
pub struct GdiState {
    pub dcs: HandleTable<VirtualDC>,
    pub objects: HandleTable<GdiObject>,
    pub stock_objects: HashMap<u32, GdiObject>,
    pub screen_surface: Surface32,
}

impl GdiState {
    pub fn new() -> Self {
        let mut stock_objects = HashMap::new();
        stock_objects.insert(WHITE_BRUSH, GdiObject::Brush { style: 0, color: 0x00FF_FFFF });
        stock_objects.insert(LTGRAY_BRUSH, GdiObject::Brush { style: 0, color: 0x00C0_C0C0 });
        stock_objects.insert(GRAY_BRUSH, GdiObject::Brush { style: 0, color: 0x0080_8080 });
        stock_objects.insert(DKGRAY_BRUSH, GdiObject::Brush { style: 0, color: 0x0040_4040 });
        stock_objects.insert(BLACK_BRUSH, GdiObject::Brush { style: 0, color: 0x0000_0000 });
        stock_objects.insert(NULL_BRUSH, GdiObject::Brush { style: 1, color: 0x0000_0000 });
        stock_objects.insert(WHITE_PEN, GdiObject::Pen { style: PS_SOLID, width: 1, color: 0x00FF_FFFF });
        stock_objects.insert(BLACK_PEN, GdiObject::Pen { style: PS_SOLID, width: 1, color: 0x0000_0000 });
        stock_objects.insert(NULL_PEN, GdiObject::Pen { style: PS_NULL, width: 1, color: 0x0000_0000 });

        Self {
            dcs: HandleTable::new(0x0002_0001),
            objects: HandleTable::new(0x0003_0001),
            stock_objects,
            screen_surface: Surface32::new(1024, 768),
        }
    }

    pub fn get_object(&self, handle: u32) -> Option<GdiObject> {
        if (STOCK_OBJECT_BASE..=STOCK_OBJECT_BASE + 8).contains(&handle) {
            let id = handle - STOCK_OBJECT_BASE;
            self.stock_objects.get(&id).cloned()
        } else {
            self.objects.get(handle).cloned()
        }
    }
}

impl Default for GdiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Global singleton for the GDI state.
pub fn gdi_state() -> &'static Mutex<GdiState> {
    static GDI_STATE: OnceLock<Mutex<GdiState>> = OnceLock::new();
    GDI_STATE.get_or_init(|| Mutex::new(GdiState::new()))
}

// ---------------------------------------------------------------------------
// GDI32 C-ABI Functions
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "system" fn gdi32_CreateCompatibleDC(_hdc: u32) -> u32 {
    let mut state = gdi_state().lock().unwrap();
    let dc = VirtualDC {
        hdc: 0,
        hwnd: None,
        selected_pen: STOCK_OBJECT_BASE + BLACK_PEN,
        selected_brush: STOCK_OBJECT_BASE + WHITE_BRUSH,
        selected_bitmap: 0,
        pen_pos: (0, 0),
        mem_surface: Some(Surface32::new(1, 1)),
    };
    let new_hdc = state.dcs.insert(dc);
    if let Some(d) = state.dcs.get_mut(new_hdc) {
        d.hdc = new_hdc;
    }
    new_hdc
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_DeleteDC(hdc: u32) -> i32 {
    let mut state = gdi_state().lock().unwrap();
    if state.dcs.remove(hdc).is_some() { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_CreateCompatibleBitmap(
    _hdc: u32,
    n_width: i32,
    n_height: i32,
) -> u32 {
    let mut state = gdi_state().lock().unwrap();
    let w = n_width.max(1) as u16;
    let h = n_height.max(1) as u16;
    let bmp = GdiObject::Bitmap {
        width: w,
        height: h,
        surface: Surface32::new(w, h),
    };
    state.objects.insert(bmp)
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_SelectObject(hdc: u32, hgdiobj: u32) -> u32 {
    let obj = {
        let state = gdi_state().lock().unwrap();
        state.get_object(hgdiobj)
    };

    if let Some(obj) = obj {
        let mut state = gdi_state().lock().unwrap();
        if let Some(dc) = state.dcs.get_mut(hdc) {
            match obj {
                GdiObject::Pen { .. } => {
                    let prev = dc.selected_pen;
                    dc.selected_pen = hgdiobj;
                    prev
                }
                GdiObject::Brush { .. } => {
                    let prev = dc.selected_brush;
                    dc.selected_brush = hgdiobj;
                    prev
                }
                GdiObject::Bitmap { .. } => {
                    let prev = dc.selected_bitmap;
                    dc.selected_bitmap = hgdiobj;
                    prev
                }
            }
        } else {
            0
        }
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_DeleteObject(h_object: u32) -> i32 {
    if (STOCK_OBJECT_BASE..=STOCK_OBJECT_BASE + 8).contains(&h_object) {
        return 1;
    }
    let mut state = gdi_state().lock().unwrap();
    if state.objects.remove(h_object).is_some() { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_GetStockObject(fn_object: i32) -> u32 {
    if (0..=8).contains(&fn_object) {
        STOCK_OBJECT_BASE + (fn_object as u32)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_CreatePen(
    fn_pen_style: i32,
    n_width: i32,
    cr_color: u32,
) -> u32 {
    let mut state = gdi_state().lock().unwrap();
    let pen = GdiObject::Pen {
        style: fn_pen_style as u32,
        width: n_width.max(1),
        color: cr_color,
    };
    state.objects.insert(pen)
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_CreateSolidBrush(cr_color: u32) -> u32 {
    let mut state = gdi_state().lock().unwrap();
    let brush = GdiObject::Brush {
        style: 0,
        color: cr_color,
    };
    state.objects.insert(brush)
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_SetPixel(hdc: u32, x: i32, y: i32, cr_color: u32) -> u32 {
    let (hwnd, selected_bitmap) = {
        let state = gdi_state().lock().unwrap();
        if let Some(dc) = state.dcs.get(hdc) {
            (dc.hwnd, dc.selected_bitmap)
        } else {
            return 0xFFFF_FFFF;
        }
    };

    if let Some(h) = hwnd {
        if h != 0 {
            let mut tree = window_tree().lock().unwrap();
            if let Some(win) = tree.get_window_mut(h) {
                win.surface.set_pixel(x, y, cr_color);
                return cr_color;
            }
        }
    }

    let mut state = gdi_state().lock().unwrap();
    if selected_bitmap != 0 {
        if let Some(GdiObject::Bitmap { surface, .. }) = state.objects.get_mut(selected_bitmap) {
            surface.set_pixel(x, y, cr_color);
            return cr_color;
        }
    }
    if let Some(dc) = state.dcs.get_mut(hdc) {
        if let Some(s) = &mut dc.mem_surface {
            s.set_pixel(x, y, cr_color);
            return cr_color;
        }
    }
    0xFFFF_FFFF
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_GetPixel(hdc: u32, x: i32, y: i32) -> u32 {
    let (hwnd, selected_bitmap) = {
        let state = gdi_state().lock().unwrap();
        if let Some(dc) = state.dcs.get(hdc) {
            (dc.hwnd, dc.selected_bitmap)
        } else {
            return 0xFFFF_FFFF;
        }
    };

    if let Some(h) = hwnd {
        if h != 0 {
            let tree = window_tree().lock().unwrap();
            if let Some(win) = tree.get_window(h) {
                return win.surface.get_pixel(x, y);
            }
        }
    }

    let state = gdi_state().lock().unwrap();
    if selected_bitmap != 0 {
        if let Some(GdiObject::Bitmap { surface, .. }) = state.objects.get(selected_bitmap) {
            return surface.get_pixel(x, y);
        }
    }
    if let Some(dc) = state.dcs.get(hdc) {
        if let Some(s) = &dc.mem_surface {
            return s.get_pixel(x, y);
        }
    }
    0xFFFF_FFFF
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_MoveToEx(
    hdc: u32,
    x: i32,
    y: i32,
    lp_point: *mut Point,
) -> i32 {
    let mut state = gdi_state().lock().unwrap();
    if let Some(dc) = state.dcs.get_mut(hdc) {
        if !lp_point.is_null() {
            (*lp_point).x = dc.pen_pos.0;
            (*lp_point).y = dc.pen_pos.1;
        }
        dc.pen_pos = (x, y);
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_LineTo(hdc: u32, x: i32, y: i32) -> i32 {
    let (hwnd, selected_pen, from_pos, selected_bitmap) = {
        let mut state = gdi_state().lock().unwrap();
        if let Some(dc) = state.dcs.get_mut(hdc) {
            let from = dc.pen_pos;
            dc.pen_pos = (x, y);
            (dc.hwnd, dc.selected_pen, from, dc.selected_bitmap)
        } else {
            return 0;
        }
    };

    let pen = gdi_state().lock().unwrap().get_object(selected_pen);
    let pen_color = match pen {
        Some(GdiObject::Pen { style, color, .. }) if style != PS_NULL => Some(color),
        _ => None,
    };

    if let Some(pc) = pen_color {
        if let Some(h) = hwnd {
            if h != 0 {
                let mut tree = window_tree().lock().unwrap();
                if let Some(win) = tree.get_window_mut(h) {
                    win.surface.draw_line(from_pos.0, from_pos.1, x, y, pc);
                    return 1;
                }
            }
        }
        let mut state = gdi_state().lock().unwrap();
        if selected_bitmap != 0 {
            if let Some(GdiObject::Bitmap { surface, .. }) = state.objects.get_mut(selected_bitmap) {
                surface.draw_line(from_pos.0, from_pos.1, x, y, pc);
                return 1;
            }
        }
        if let Some(dc) = state.dcs.get_mut(hdc) {
            if let Some(s) = &mut dc.mem_surface {
                s.draw_line(from_pos.0, from_pos.1, x, y, pc);
                return 1;
            }
        }
    }
    1
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_Rectangle(
    hdc: u32,
    n_left: i32,
    n_top: i32,
    n_right: i32,
    n_bottom: i32,
) -> i32 {
    let (hwnd, selected_pen, selected_brush, selected_bitmap) = {
        let state = gdi_state().lock().unwrap();
        if let Some(dc) = state.dcs.get(hdc) {
            (dc.hwnd, dc.selected_pen, dc.selected_brush, dc.selected_bitmap)
        } else {
            return 0;
        }
    };

    let (pen_color, brush_color) = {
        let state = gdi_state().lock().unwrap();
        let p = match state.get_object(selected_pen) {
            Some(GdiObject::Pen { style, color, .. }) if style != PS_NULL => Some(color),
            _ => None,
        };
        let b = match state.get_object(selected_brush) {
            Some(GdiObject::Brush { style: 0, color }) => Some(color),
            _ => None,
        };
        (p, b)
    };

    if let Some(h) = hwnd {
        if h != 0 {
            let mut tree = window_tree().lock().unwrap();
            if let Some(win) = tree.get_window_mut(h) {
                win.surface.draw_rectangle(n_left, n_top, n_right, n_bottom, pen_color, brush_color);
                return 1;
            }
        }
    }

    let mut state = gdi_state().lock().unwrap();
    if selected_bitmap != 0 {
        if let Some(GdiObject::Bitmap { surface, .. }) = state.objects.get_mut(selected_bitmap) {
            surface.draw_rectangle(n_left, n_top, n_right, n_bottom, pen_color, brush_color);
            return 1;
        }
    }
    if let Some(dc) = state.dcs.get_mut(hdc) {
        if let Some(s) = &mut dc.mem_surface {
            s.draw_rectangle(n_left, n_top, n_right, n_bottom, pen_color, brush_color);
            return 1;
        }
    }
    1
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_Ellipse(
    hdc: u32,
    n_left: i32,
    n_top: i32,
    n_right: i32,
    n_bottom: i32,
) -> i32 {
    let (hwnd, selected_pen, selected_brush, selected_bitmap) = {
        let state = gdi_state().lock().unwrap();
        if let Some(dc) = state.dcs.get(hdc) {
            (dc.hwnd, dc.selected_pen, dc.selected_brush, dc.selected_bitmap)
        } else {
            return 0;
        }
    };

    let (pen_color, brush_color) = {
        let state = gdi_state().lock().unwrap();
        let p = match state.get_object(selected_pen) {
            Some(GdiObject::Pen { style, color, .. }) if style != PS_NULL => Some(color),
            _ => None,
        };
        let b = match state.get_object(selected_brush) {
            Some(GdiObject::Brush { style: 0, color }) => Some(color),
            _ => None,
        };
        (p, b)
    };

    if let Some(h) = hwnd {
        if h != 0 {
            let mut tree = window_tree().lock().unwrap();
            if let Some(win) = tree.get_window_mut(h) {
                win.surface.draw_ellipse(n_left, n_top, n_right, n_bottom, pen_color, brush_color);
                return 1;
            }
        }
    }

    let mut state = gdi_state().lock().unwrap();
    if selected_bitmap != 0 {
        if let Some(GdiObject::Bitmap { surface, .. }) = state.objects.get_mut(selected_bitmap) {
            surface.draw_ellipse(n_left, n_top, n_right, n_bottom, pen_color, brush_color);
            return 1;
        }
    }
    if let Some(dc) = state.dcs.get_mut(hdc) {
        if let Some(s) = &mut dc.mem_surface {
            s.draw_ellipse(n_left, n_top, n_right, n_bottom, pen_color, brush_color);
            return 1;
        }
    }
    1
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_RoundRect(
    hdc: u32,
    n_left: i32,
    n_top: i32,
    n_right: i32,
    n_bottom: i32,
    n_width: i32,
    n_height: i32,
) -> i32 {
    let (hwnd, selected_pen, selected_brush, selected_bitmap) = {
        let state = gdi_state().lock().unwrap();
        if let Some(dc) = state.dcs.get(hdc) {
            (dc.hwnd, dc.selected_pen, dc.selected_brush, dc.selected_bitmap)
        } else {
            return 0;
        }
    };

    let (pen_color, brush_color) = {
        let state = gdi_state().lock().unwrap();
        let p = match state.get_object(selected_pen) {
            Some(GdiObject::Pen { style, color, .. }) if style != PS_NULL => Some(color),
            _ => None,
        };
        let b = match state.get_object(selected_brush) {
            Some(GdiObject::Brush { style: 0, color }) => Some(color),
            _ => None,
        };
        (p, b)
    };

    if let Some(h) = hwnd {
        if h != 0 {
            let mut tree = window_tree().lock().unwrap();
            if let Some(win) = tree.get_window_mut(h) {
                win.surface.draw_round_rect(n_left, n_top, n_right, n_bottom, n_width, n_height, pen_color, brush_color);
                return 1;
            }
        }
    }

    let mut state = gdi_state().lock().unwrap();
    if selected_bitmap != 0 {
        if let Some(GdiObject::Bitmap { surface, .. }) = state.objects.get_mut(selected_bitmap) {
            surface.draw_round_rect(n_left, n_top, n_right, n_bottom, n_width, n_height, pen_color, brush_color);
            return 1;
        }
    }
    if let Some(dc) = state.dcs.get_mut(hdc) {
        if let Some(s) = &mut dc.mem_surface {
            s.draw_round_rect(n_left, n_top, n_right, n_bottom, n_width, n_height, pen_color, brush_color);
            return 1;
        }
    }
    1
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_PatBlt(
    hdc: u32,
    n_x_left: i32,
    n_y_left: i32,
    n_width: i32,
    n_height: i32,
    dw_rop: u32,
) -> i32 {
    let (hwnd, selected_brush, selected_bitmap) = {
        let state = gdi_state().lock().unwrap();
        if let Some(dc) = state.dcs.get(hdc) {
            (dc.hwnd, dc.selected_brush, dc.selected_bitmap)
        } else {
            return 0;
        }
    };

    let brush_color = {
        let state = gdi_state().lock().unwrap();
        match state.get_object(selected_brush) {
            Some(GdiObject::Brush { color, .. }) => color,
            _ => 0x00FF_FFFF,
        }
    };

    if let Some(h) = hwnd {
        if h != 0 {
            let mut tree = window_tree().lock().unwrap();
            if let Some(win) = tree.get_window_mut(h) {
                win.surface.pat_blt(n_x_left, n_y_left, n_width, n_height, dw_rop, brush_color);
                return 1;
            }
        }
    }

    let mut state = gdi_state().lock().unwrap();
    if selected_bitmap != 0 {
        if let Some(GdiObject::Bitmap { surface, .. }) = state.objects.get_mut(selected_bitmap) {
            surface.pat_blt(n_x_left, n_y_left, n_width, n_height, dw_rop, brush_color);
            return 1;
        }
    }
    if let Some(dc) = state.dcs.get_mut(hdc) {
        if let Some(s) = &mut dc.mem_surface {
            s.pat_blt(n_x_left, n_y_left, n_width, n_height, dw_rop, brush_color);
            return 1;
        }
    }
    1
}

#[no_mangle]
pub unsafe extern "system" fn gdi32_BitBlt(
    hdc_dest: u32,
    n_x_dest: i32,
    n_y_dest: i32,
    n_width: i32,
    n_height: i32,
    hdc_src: u32,
    n_x_src: i32,
    n_y_src: i32,
    dw_rop: u32,
) -> i32 {
    // 1. Snapshot src surface
    let src_surface = {
        let (hwnd, selected_bitmap) = {
            let state = gdi_state().lock().unwrap();
            if let Some(dc) = state.dcs.get(hdc_src) {
                (dc.hwnd, dc.selected_bitmap)
            } else {
                return 0;
            }
        };

        if let Some(h) = hwnd {
            if h != 0 {
                let tree = window_tree().lock().unwrap();
                tree.get_window(h).map(|w| w.surface.clone())
            } else {
                None
            }
        } else {
            let state = gdi_state().lock().unwrap();
            if selected_bitmap != 0 {
                if let Some(GdiObject::Bitmap { surface, .. }) = state.objects.get(selected_bitmap) {
                    Some(surface.clone())
                } else {
                    None
                }
            } else if let Some(dc) = state.dcs.get(hdc_src) {
                dc.mem_surface.clone()
            } else {
                None
            }
        }
    };

    let src = match src_surface {
        Some(s) => s,
        None => return 0,
    };

    // 2. Blit to dest surface
    let (hwnd_dst, selected_bitmap_dst) = {
        let state = gdi_state().lock().unwrap();
        if let Some(dc) = state.dcs.get(hdc_dest) {
            (dc.hwnd, dc.selected_bitmap)
        } else {
            return 0;
        }
    };

    if let Some(h) = hwnd_dst {
        if h != 0 {
            let mut tree = window_tree().lock().unwrap();
            if let Some(win) = tree.get_window_mut(h) {
                win.surface.bit_blt(n_x_dest, n_y_dest, n_width, n_height, &src, n_x_src, n_y_src, dw_rop);
                return 1;
            }
        }
    }

    let mut state = gdi_state().lock().unwrap();
    if selected_bitmap_dst != 0 {
        if let Some(GdiObject::Bitmap { surface, .. }) = state.objects.get_mut(selected_bitmap_dst) {
            surface.bit_blt(n_x_dest, n_y_dest, n_width, n_height, &src, n_x_src, n_y_src, dw_rop);
            return 1;
        }
    }
    if let Some(dc) = state.dcs.get_mut(hdc_dest) {
        if let Some(s) = &mut dc.mem_surface {
            s.bit_blt(n_x_dest, n_y_dest, n_width, n_height, &src, n_x_src, n_y_src, dw_rop);
            return 1;
        }
    }
    1
}
