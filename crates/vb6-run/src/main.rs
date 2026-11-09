//! vb6-run: Standalone Visual Basic 6.0 PE Runner and Inspector for Linux / macOS.
//!
//! Loads legacy Win32 VB6 executables into executable memory, resolves imports
//! against msvbvm60 and Win32 shims, and dispatches execution directly.
//!
//! ## Notice
//! Microsoft®, Windows®, and Visual Basic® are trademarks of Microsoft Corporation.
//! ClassicBasic is an independent open-source project and is not affiliated with Microsoft.

use std::env;
use std::fs;
use std::path::Path;
use std::process;

use msvbvm60::pe::loader::LoadedPe;
use msvbvm60::thun_rt::ExecutionMode;

/// Prints the application banner to standard output.
fn print_banner() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       VB6-RUN: Visual Basic 6.0 Linux / Native Runner         ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
}

/// Prints command-line syntax and usage options to standard output.
///
/// # Arguments
/// * `prog_name` - Program invocation name (typically `argv[0]`).
fn print_usage(prog_name: &str) {
    println!("\nUsage: {} [options] <path-to-vb6-exe>", prog_name);
    println!("       {} <path-to-vb6-exe> [options]", prog_name);
    println!("\nOptions:");
    println!("  --gui-web         Execute in interactive Web GUI mode (WebSocket / HTML5 canvas)");
    println!("  --port <PORT>     Port to bind web server (default: 8080 or PORT env)");
    println!("  --headless        Execute in headless batch mode (default for CLI, batch, daemons)");
    println!("  --inspect         Inspect PE headers, sections, and metadata without running");
    println!("  --dump-info       Dump full VBHeader, ProjectInfo, and object tables");
    println!("  --quiet, -q       Suppress console diagnostic logs (default when --gui-web is active)");
    println!("  --verbose, -v     Enable verbose debug and diagnostic output");
    println!("  --help, -h        Display this help message\n");
    println!("Note: Microsoft, Windows, and Visual Basic are trademarks of Microsoft Corporation.");
    println!(
        "      ClassicBasic is an independent clean-room project not affiliated with Microsoft.\n"
    );
}

struct DynamicTextBox {
    hwnd: u32,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    rel_x: usize,
    rel_y: usize,
    text: String,
    back_color: u32,
    fore_color: u32,
}

impl DynamicTextBox {
    fn render(&self, server: &cb_webserver::ServerHandle) {
        let mut patch = vec![0u8; self.w * self.h * 4];
        let r = (self.back_color & 0xFF) as u8;
        let g = ((self.back_color >> 8) & 0xFF) as u8;
        let b = ((self.back_color >> 16) & 0xFF) as u8;

        for chunk in patch.as_chunks_mut::<4>().0 {
            chunk[0] = b;
            chunk[1] = g;
            chunk[2] = r;
            chunk[3] = 0xFF;
        }

        // Sunken 3D border
        for x in 0..self.w {
            let idx_top = x * 4;
            let idx_bot = ((self.h - 1) * self.w + x) * 4;
            patch[idx_top] = 0x80;
            patch[idx_top + 1] = 0x80;
            patch[idx_top + 2] = 0x80;
            patch[idx_bot] = 0xFF;
            patch[idx_bot + 1] = 0xFF;
            patch[idx_bot + 2] = 0xFF;
        }
        for y in 0..self.h {
            let idx_left = (y * self.w) * 4;
            let idx_right = (y * self.w + self.w - 1) * 4;
            patch[idx_left] = 0x80;
            patch[idx_left + 1] = 0x80;
            patch[idx_left + 2] = 0x80;
            patch[idx_right] = 0xFF;
            patch[idx_right + 1] = 0xFF;
            patch[idx_right + 2] = 0xFF;
        }

        server.broadcast_frame(&cb_webserver::DisplayFrame::BitmapRect(
            cb_webserver::BitmapRectFrame {
                hwnd: self.hwnd,
                x: self.x as u16,
                y: self.y as u16,
                width: self.w as u16,
                height: self.h as u16,
                pixels: patch,
            },
        ));

        let char_w = 11;
        let text_w = self.text.len() * char_w;
        let tx = if text_w < self.w.saturating_sub(16) {
            (self.x + self.w - 12 - text_w) as i16
        } else {
            (self.x + 8) as i16
        };
        let ty = (self.y + (self.h * 2 / 3)) as i16;

        // Shadow
        server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: self.hwnd,
            x: tx + 1,
            y: ty + 1,
            color_rgb: 0x00303030,
            bk_mode: 1,
            font_height: -20,
            text: self.text.clone(),
        }));

        // Text
        server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: self.hwnd,
            x: tx,
            y: ty,
            color_rgb: self.fore_color,
            bk_mode: 1,
            font_height: -20,
            text: self.text.clone(),
        }));
    }
}

struct DynamicLabel {
    hwnd: u32,
    name: String,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    rel_x: usize,
    rel_y: usize,
    caption: String,
    fore_color: u32,
}

impl DynamicLabel {
    #[allow(dead_code)]
    fn render(&self, server: &cb_webserver::ServerHandle) {
        let mut patch = vec![0u8; self.w * self.h * 4];
        for chunk in patch.as_chunks_mut::<4>().0 {
            chunk[0] = 0xC8;
            chunk[1] = 0xD0;
            chunk[2] = 0xD4;
            chunk[3] = 0xFF;
        }

        server.broadcast_frame(&cb_webserver::DisplayFrame::BitmapRect(
            cb_webserver::BitmapRectFrame {
                hwnd: self.hwnd,
                x: self.x as u16,
                y: self.y as u16,
                width: self.w as u16,
                height: self.h as u16,
                pixels: patch,
            },
        ));

        let tx = (self.x + 4) as i16;
        let ty = (self.y + (self.h * 2 / 3).max(14)) as i16;
        let font_h = -((self.h * 2 / 3).clamp(12, 22) as i16);

        server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: self.hwnd,
            x: tx,
            y: ty,
            color_rgb: self.fore_color,
            bk_mode: 1,
            font_height: font_h,
            text: self.caption.clone(),
        }));
    }
}

struct ActiveTimer {
    name: String,
    interval_ms: u64,
    last_tick: std::time::Instant,
}

#[allow(dead_code)]
struct AppInstance {
    id: usize,
    exe_path: String,
    name: String,
    caption: String,
    client_w: usize,
    client_h: usize,
    win_w: usize,
    win_h: usize,
    win_x: usize,
    win_y: usize,
    open: bool,
    minimized: bool,
    system_menu_open: bool,
    max_button_enabled: bool,
    icon_16: Option<Vec<u8>>,
    icon_32: Option<Vec<u8>>,
    buttons: Vec<cb_win32::Win32ButtonControl>,
    button_rel_coords: Vec<(usize, usize)>,
    textboxes: Vec<DynamicTextBox>,
    labels: Vec<DynamicLabel>,
    timers: Vec<ActiveTimer>,
    runtime: cb_win32::FormRuntime,
    loaded_pe: LoadedPe,
}

#[allow(dead_code)]
fn handle_calc_action(action: &str, tb: &mut DynamicTextBox, server: &cb_webserver::ServerHandle) {
    match action {
        "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
            if tb.text == "Erro" || tb.text == "0" {
                tb.text = action.to_string();
            } else {
                tb.text.push_str(action);
            }
            tb.render(server);
        }
        "+" | "-" | "*" | "/" => {
            if tb.text == "Erro" {
                tb.text = "0".to_string();
            }
            tb.text.push_str(&format!(" {} ", action));
            tb.render(server);
        }
        "=" => {
            println!("[msvbvm60] Command_equals_Click: sc.Eval(\"{}\")", tb.text);
            match msvbvm60::activex::evaluate_vbscript_expression(&tb.text) {
                Ok(res) => {
                    tb.text = if res.fract() == 0.0 && res.abs() < 1e14 {
                        format!("{}", res as i64)
                    } else {
                        format!("{}", res)
                    };
                    println!("[msvbvm60] sc.Eval -> {}", tb.text);
                }
                Err(e) => {
                    println!("[msvbvm60] sc.Eval error: {} -> 'Erro'", e);
                    tb.text = "Erro".to_string();
                }
            }
            tb.render(server);
        }
        "C" | "CE" | "Clear" => {
            tb.text = "0".to_string();
            tb.render(server);
        }
        "." | "," if !tb.text.ends_with('.') && !tb.text.contains('.') => {
            tb.text.push('.');
            tb.render(server);
        }
        _ => {}
    }
}

fn broadcast_button_state(server: &cb_webserver::ServerHandle, btn: &cb_win32::Win32ButtonControl) {
    server.broadcast_frame(&cb_webserver::DisplayFrame::BitmapRect(
        cb_webserver::BitmapRectFrame {
            hwnd: btn.hwnd,
            x: btn.x as u16,
            y: btn.y as u16,
            width: btn.width as u16,
            height: btn.height as u16,
            pixels: btn.render_pixels(),
        },
    ));
    let (dx, dy) = btn.text_offset();
    let text_w = btn.caption.len() as i16 * 8;
    let tx = btn.x as i16 + (btn.width as i16 / 2) - (text_w / 2).min(btn.width as i16 / 2 - 2) + dx;
    let ty = btn.y as i16 + (btn.height as i16 / 2) + 5 + dy;
    server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
        hwnd: btn.hwnd,
        x: tx,
        y: ty,
        color_rgb: 0x00000000,
        bk_mode: 1,
        font_height: -14,
        text: btn.caption.clone(),
    }));
}

#[allow(dead_code)]
fn broadcast_msgbox_dialog(server: &cb_webserver::ServerHandle, title: &str, prompt: &str) {
    let dw = 280usize;
    let dh = 140usize;
    let mut patch = vec![0u8; dw * dh * 4];

    for chunk in patch.as_chunks_mut::<4>().0 {
        chunk[0] = 0xC8;
        chunk[1] = 0xD0;
        chunk[2] = 0xD4;
        chunk[3] = 0xFF;
    }

    let set_px = |buf: &mut [u8], px: usize, py: usize, b: u8, g: u8, r: u8| {
        if px < dw && py < dh {
            let idx = (py * dw + px) * 4;
            buf[idx] = b;
            buf[idx + 1] = g;
            buf[idx + 2] = r;
            buf[idx + 3] = 0xFF;
        }
    };

    let fill_box =
        |buf: &mut [u8], rx: usize, ry: usize, rw: usize, rh: usize, b: u8, g: u8, r: u8| {
            for y in ry..(ry + rh).min(dh) {
                for x in rx..(rx + rw).min(dw) {
                    let idx = (y * dw + x) * 4;
                    buf[idx] = b;
                    buf[idx + 1] = g;
                    buf[idx + 2] = r;
                    buf[idx + 3] = 0xFF;
                }
            }
        };

    let draw_bevel = |buf: &mut [u8], rx: usize, ry: usize, rw: usize, rh: usize, raised: bool| {
        let (top_b, top_g, top_r, bot_b, bot_g, bot_r) = if raised {
            (0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00)
        } else {
            (0x80, 0x80, 0x80, 0xFF, 0xFF, 0xFF)
        };
        for x in rx..(rx + rw) {
            set_px(buf, x, ry, top_b, top_g, top_r);
            set_px(buf, x, ry + rh - 1, bot_b, bot_g, bot_r);
        }
        for y in ry..(ry + rh) {
            set_px(buf, rx, y, top_b, top_g, top_r);
            set_px(buf, rx + rw - 1, y, bot_b, bot_g, bot_r);
        }
    };

    let draw_btn = |buf: &mut [u8], rx: usize, ry: usize, rw: usize, rh: usize| {
        fill_box(buf, rx, ry, rw, rh, 0xC8, 0xD0, 0xD4);
        draw_bevel(buf, rx, ry, rw, rh, true);
    };

    draw_bevel(&mut patch, 0, 0, dw, dh, true);
    draw_bevel(&mut patch, 1, 1, dw - 2, dh - 2, true);
    fill_box(&mut patch, 4, 4, 272, 22, 0x80, 0x00, 0x00);
    draw_btn(&mut patch, 254, 6, 18, 18);
    draw_btn(&mut patch, 105, 95, 70, 25);

    server.broadcast_frame(&cb_webserver::DisplayFrame::BitmapRect(
        cb_webserver::BitmapRectFrame {
            hwnd: 100,
            x: 260,
            y: 220,
            width: dw as u16,
            height: dh as u16,
            pixels: patch,
        },
    ));

    server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
        hwnd: 100,
        x: 270,
        y: 236,
        color_rgb: 0x00FFFFFF,
        bk_mode: 1,
        font_height: -12,
        text: title.to_string(),
    }));

    server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
        hwnd: 100,
        x: 518,
        y: 237,
        color_rgb: 0x00000000,
        bk_mode: 1,
        font_height: -11,
        text: "X".to_string(),
    }));

    server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
        hwnd: 100,
        x: 345,
        y: 275,
        color_rgb: 0x00000000,
        bk_mode: 1,
        font_height: -16,
        text: prompt.to_string(),
    }));

    server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
        hwnd: 100,
        x: 390,
        y: 332,
        color_rgb: 0x00000000,
        bk_mode: 1,
        font_height: -13,
        text: "OK".to_string(),
    }));
}

#[allow(dead_code)]
fn dismiss_msgbox_dialog(
    server: &cb_webserver::ServerHandle,
    buttons: &[cb_win32::Win32ButtonControl],
    textboxes: &[DynamicTextBox],
    labels: &[DynamicLabel],
) {
    let dw = 280usize;
    let dh = 140usize;
    let mut patch = vec![0u8; dw * dh * 4];
    for chunk in patch.as_chunks_mut::<4>().0 {
        chunk[0] = 0xC8;
        chunk[1] = 0xD0;
        chunk[2] = 0xD4;
        chunk[3] = 0xFF;
    }
    server.broadcast_frame(&cb_webserver::DisplayFrame::BitmapRect(
        cb_webserver::BitmapRectFrame {
            hwnd: 100,
            x: 260,
            y: 220,
            width: dw as u16,
            height: dh as u16,
            pixels: patch,
        },
    ));
    for b in buttons {
        if b.x + b.width as i32 >= 260 && b.x <= 260 + dw as i32
            && b.y + b.height as i32 >= 220 && b.y <= 220 + dh as i32
        {
            broadcast_button_state(server, b);
        }
    }
    for tb in textboxes {
        if tb.x + tb.w >= 260 && tb.x <= 260 + dw
            && tb.y + tb.h >= 220 && tb.y <= 220 + dh
        {
            tb.render(server);
        }
    }
    for lbl in labels {
        if lbl.x + lbl.w >= 260 && lbl.x <= 260 + dw
            && lbl.y + lbl.h >= 220 && lbl.y <= 220 + dh
        {
            lbl.render(server);
        }
    }
}

struct DesktopCanvas<'a> {
    buf: &'a mut [u8],
    w: usize,
    h: usize,
}

impl<'a> DesktopCanvas<'a> {
    #[inline]
    pub fn new(buf: &'a mut [u8], w: usize, h: usize) -> Self {
        Self { buf, w, h }
    }

    #[inline]
    pub fn set_px(&mut self, px: usize, py: usize, b: u8, g: u8, r: u8) {
        if px < self.w && py < self.h {
            let idx = (py * self.w + px) * 4;
            if idx + 3 < self.buf.len() {
                self.buf[idx] = b;
                self.buf[idx + 1] = g;
                self.buf[idx + 2] = r;
                self.buf[idx + 3] = 0xFF;
            }
        }
    }

    #[inline]
    pub fn fill_box(&mut self, rx: usize, ry: usize, rw: usize, rh: usize, b: u8, g: u8, r: u8) {
        let max_y = (ry + rh).min(self.h);
        let max_x = (rx + rw).min(self.w);
        for y in ry..max_y {
            let row_start = y * self.w;
            for x in rx..max_x {
                let idx = (row_start + x) * 4;
                if idx + 3 < self.buf.len() {
                    self.buf[idx] = b;
                    self.buf[idx + 1] = g;
                    self.buf[idx + 2] = r;
                    self.buf[idx + 3] = 0xFF;
                }
            }
        }
    }

    pub fn draw_rect_outline(&mut self, rx: usize, ry: usize, rw: usize, rh: usize, b: u8, g: u8, r: u8) {
        let max_x = (rx + rw).min(self.w);
        let max_y = (ry + rh).min(self.h);
        for x in rx..max_x {
            self.set_px(x, ry, b, g, r);
            if rh > 0 { self.set_px(x, ry + rh - 1, b, g, r); }
        }
        for y in ry..max_y {
            self.set_px(rx, y, b, g, r);
            if rw > 0 { self.set_px(rx + rw - 1, y, b, g, r); }
        }
    }

    pub fn draw_bevel(&mut self, rx: usize, ry: usize, rw: usize, rh: usize, raised: bool) {
        let (top_b, top_g, top_r, bot_b, bot_g, bot_r) = if raised {
            (0xFF, 0xFF, 0xFF, 0x80, 0x80, 0x80) // Soft shadow (#808080) for authentic Windows 95/98
        } else {
            (0x80, 0x80, 0x80, 0xFF, 0xFF, 0xFF)
        };
        let max_x = (rx + rw).min(self.w);
        let max_y = (ry + rh).min(self.h);
        for x in rx..max_x {
            self.set_px(x, ry, top_b, top_g, top_r);
            if rh > 0 { self.set_px(x, ry + rh - 1, bot_b, bot_g, bot_r); }
        }
        for y in ry..max_y {
            self.set_px(rx, y, top_b, top_g, top_r);
            if rw > 0 { self.set_px(rx + rw - 1, y, bot_b, bot_g, bot_r); }
        }
    }
}

fn draw_win98_button(canvas: &mut DesktopCanvas, rx: usize, ry: usize, rw: usize, rh: usize, pressed: bool) {
    canvas.fill_box(rx, ry, rw, rh, 0xC8, 0xD0, 0xD4);
    if !pressed {
        // Outer Top/Left = White (#FFFFFF), Outer Bottom/Right = Dark Shadow (#404040, soft dark gray)
        for x in rx..(rx + rw) {
            canvas.set_px(x, ry, 0xFF, 0xFF, 0xFF);
            if rh > 0 { canvas.set_px(x, ry + rh - 1, 0x40, 0x40, 0x40); }
        }
        for y in ry..(ry + rh) {
            canvas.set_px(rx, y, 0xFF, 0xFF, 0xFF);
            if rw > 0 { canvas.set_px(rx + rw - 1, y, 0x40, 0x40, 0x40); }
        }
        // Inner Top/Left = Light face (#DFDFDF), Inner Bottom/Right = Medium Shadow (#808080)
        if rw > 2 && rh > 2 {
            for x in (rx + 1)..(rx + rw - 1) {
                canvas.set_px(x, ry + 1, 0xDF, 0xDF, 0xDF);
                canvas.set_px(x, ry + rh - 2, 0x80, 0x80, 0x80);
            }
            for y in (ry + 1)..(ry + rh - 1) {
                canvas.set_px(rx + 1, y, 0xDF, 0xDF, 0xDF);
                canvas.set_px(rx + rw - 2, y, 0x80, 0x80, 0x80);
            }
        }
    } else {
        // Sunken button state
        // Outer Top/Left = Dark Shadow (#404040), Outer Bottom/Right = White (#FFFFFF)
        for x in rx..(rx + rw) {
            canvas.set_px(x, ry, 0x40, 0x40, 0x40);
            if rh > 0 { canvas.set_px(x, ry + rh - 1, 0xFF, 0xFF, 0xFF); }
        }
        for y in ry..(ry + rh) {
            canvas.set_px(rx, y, 0x40, 0x40, 0x40);
            if rw > 0 { canvas.set_px(rx + rw - 1, y, 0xFF, 0xFF, 0xFF); }
        }
        // Inner Top/Left = Medium Shadow (#808080), Inner Bottom/Right = Face Light (#D4D0C8)
        if rw > 2 && rh > 2 {
            for x in (rx + 1)..(rx + rw - 1) {
                canvas.set_px(x, ry + 1, 0x80, 0x80, 0x80);
                canvas.set_px(x, ry + rh - 2, 0xC8, 0xD0, 0xD4);
            }
            for y in (ry + 1)..(ry + rh - 1) {
                canvas.set_px(rx + 1, y, 0x80, 0x80, 0x80);
                canvas.set_px(rx + rw - 2, y, 0xC8, 0xD0, 0xD4);
            }
        }
    }
}

fn draw_x_glyph(canvas: &mut DesktopCanvas, gx: usize, gy: usize, b: u8, g: u8, r: u8) {
    for i in 0..8 {
        canvas.set_px(gx + i, gy + i, b, g, r);
        canvas.set_px(gx + i + 1, gy + i, b, g, r);
        canvas.set_px(gx + 8 - i, gy + i, b, g, r);
        canvas.set_px(gx + 7 - i, gy + i, b, g, r);
    }
}

fn draw_icon_16(canvas: &mut DesktopCanvas, ix: usize, iy: usize, ico: &[u8]) {
    for y in 0..16 {
        for x in 0..16 {
            let sidx = (y * 16 + x) * 4;
            if sidx + 3 < ico.len() && ico[sidx + 3] > 128 {
                canvas.set_px(ix + x, iy + y, ico[sidx], ico[sidx + 1], ico[sidx + 2]);
            }
        }
    }
}

fn draw_icon_32(canvas: &mut DesktopCanvas, ix: usize, iy: usize, ico: &[u8]) {
    for y in 0..32 {
        for x in 0..32 {
            let sidx = (y * 32 + x) * 4;
            if sidx + 3 < ico.len() && ico[sidx + 3] > 128 {
                canvas.set_px(ix + x, iy + y, ico[sidx], ico[sidx + 1], ico[sidx + 2]);
            }
        }
    }
}

fn draw_radio_button(canvas: &mut DesktopCanvas, cx: usize, cy: usize, checked: bool) {
    for dy in 0..11 {
        for dx in 0..11 {
            let ddx = dx as i32 - 5;
            let ddy = dy as i32 - 5;
            let dist_sq = ddx * ddx + ddy * ddy;
            if dist_sq <= 25 {
                canvas.set_px(cx + dx - 5, cy + dy - 5, 0xFF, 0xFF, 0xFF);
            }
            if dist_sq >= 16 && dist_sq <= 25 {
                if ddx + ddy <= 0 {
                    canvas.set_px(cx + dx - 5, cy + dy - 5, 0x80, 0x80, 0x80);
                } else {
                    canvas.set_px(cx + dx - 5, cy + dy - 5, 0xC0, 0xC0, 0xC0);
                }
            }
            if checked && dist_sq <= 6 {
                canvas.set_px(cx + dx - 5, cy + dy - 5, 0x00, 0x00, 0x00);
            }
        }
    }
}

fn draw_group_box(canvas: &mut DesktopCanvas, gx: usize, gy: usize, gw: usize, gh: usize, theme: cb_win32::WindowTheme) {
    let (bg_b, bg_g, bg_r) = theme.client_bg_color();
    match theme {
        cb_win32::WindowTheme::Windows311 => {
            canvas.draw_rect_outline(gx, gy, gw, gh, 0, 0, 0);
            canvas.fill_box(gx + 8, gy, 175, 2, bg_b, bg_g, bg_r);
        }
        cb_win32::WindowTheme::Windows98 => {
            canvas.draw_bevel(gx, gy, gw, gh, false);
            canvas.fill_box(gx + 8, gy, 175, 2, bg_b, bg_g, bg_r);
        }
        cb_win32::WindowTheme::WindowsXp => {
            canvas.draw_rect_outline(gx, gy, gw, gh, 0xD0, 0xB8, 0xA0);
            canvas.fill_box(gx + 8, gy, 175, 2, bg_b, bg_g, bg_r);
        }
    }
}

fn draw_themed_radio_button(canvas: &mut DesktopCanvas, cx: usize, cy: usize, theme: cb_win32::WindowTheme, checked: bool) {
    match theme {
        cb_win32::WindowTheme::Windows311 => {
            for dy in 0..11 {
                for dx in 0..11 {
                    let ddx = dx as i32 - 5;
                    let ddy = dy as i32 - 5;
                    let dist_sq = ddx * ddx + ddy * ddy;
                    if dist_sq <= 25 {
                        canvas.set_px(cx + dx - 5, cy + dy - 5, 0xFF, 0xFF, 0xFF);
                    }
                    if dist_sq >= 16 && dist_sq <= 25 {
                        canvas.set_px(cx + dx - 5, cy + dy - 5, 0, 0, 0);
                    }
                    if checked && dist_sq <= 6 {
                        canvas.set_px(cx + dx - 5, cy + dy - 5, 0, 0, 0);
                    }
                }
            }
        }
        _ => {
            draw_radio_button(canvas, cx, cy, checked);
        }
    }
}

fn draw_theme_button(
    canvas: &mut DesktopCanvas,
    bx: usize,
    by: usize,
    bw: usize,
    bh: usize,
    theme: cb_win32::WindowTheme,
    selected: bool,
    pressed: bool,
) {
    let sunken = selected || pressed;
    match theme {
        cb_win32::WindowTheme::Windows311 => {
            canvas.fill_box(bx, by, bw, bh, 0xC0, 0xC0, 0xC0);
            canvas.draw_rect_outline(bx, by, bw, bh, 0, 0, 0);
            if sunken {
                canvas.draw_rect_outline(bx + 1, by + 1, bw.saturating_sub(2), bh.saturating_sub(2), 0, 0, 0);
                canvas.draw_bevel(bx + 2, by + 2, bw.saturating_sub(4), bh.saturating_sub(4), false);
            } else {
                canvas.draw_bevel(bx + 1, by + 1, bw.saturating_sub(2), bh.saturating_sub(2), true);
            }
        }
        cb_win32::WindowTheme::Windows98 => {
            draw_win98_button(canvas, bx, by, bw, bh, sunken);
        }
        cb_win32::WindowTheme::WindowsXp => {
            let (b, g, r) = if sunken {
                (0xD0, 0xA0, 0x70)
            } else {
                (0xF0, 0xEA, 0xE4)
            };
            canvas.fill_box(bx, by, bw, bh, b, g, r);
            canvas.draw_rect_outline(bx, by, bw, bh, 0x90, 0x60, 0x00);
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowId {
    App(usize),
    ControlPanel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressedTarget {
    FormButton(usize, usize),
    AppMinimize(usize),
    AppMaximize(usize),
    AppClose(usize),
    AppSysMenu(usize),
    CpMinimize,
    CpMaximize,
    CpClose,
    CpSysMenu,
    CpFormat24,
    CpFormat12,
    CpTheme311,
    CpTheme98,
    CpThemeXp,
}

fn generate_default_app_icon_32() -> Vec<u8> {
    let mut px = vec![0u8; 32 * 32 * 4];
    for y in 4..28 {
        for x in 4..28 {
            let idx = (y * 32 + x) * 4;
            px[idx] = 0xC0;
            px[idx + 1] = 0xC0;
            px[idx + 2] = 0xC0;
            px[idx + 3] = 0xFF;
        }
    }
    // Blue Title Bar
    for y in 5..10 {
        for x in 5..27 {
            let idx = (y * 32 + x) * 4;
            px[idx] = 0xA0;
            px[idx + 1] = 0x00;
            px[idx + 2] = 0x00;
            px[idx + 3] = 0xFF;
        }
    }
    // White Client Area
    for y in 10..27 {
        for x in 5..27 {
            let idx = (y * 32 + x) * 4;
            px[idx] = 0xFF;
            px[idx + 1] = 0xFF;
            px[idx + 2] = 0xFF;
            px[idx + 3] = 0xFF;
        }
    }
    // Outlines
    for x in 4..28 {
        for &y in &[4, 27, 9] {
            let idx = (y * 32 + x) * 4;
            px[idx] = 0; px[idx + 1] = 0; px[idx + 2] = 0; px[idx + 3] = 0xFF;
        }
    }
    for y in 4..28 {
        for &x in &[4, 27] {
            let idx = (y * 32 + x) * 4;
            px[idx] = 0; px[idx + 1] = 0; px[idx + 2] = 0; px[idx + 3] = 0xFF;
        }
    }
    px
}

fn draw_system_menu(
    canvas: &mut DesktopCanvas,
    text_frames: &mut Vec<cb_webserver::DisplayFrame>,
    menu_x: usize,
    menu_y: usize,
    max_button_enabled: bool,
    theme: cb_win32::WindowTheme,
) {
    let menu_w = 145;
    let menu_h = 132;
    let (bg_b, bg_g, bg_r) = match theme {
        cb_win32::WindowTheme::Windows311 => (0xFF, 0xFF, 0xFF),
        cb_win32::WindowTheme::Windows98 => (0xD4, 0xD0, 0xC8),
        cb_win32::WindowTheme::WindowsXp => (0xFF, 0xFF, 0xFF),
    };
    canvas.fill_box(menu_x, menu_y, menu_w, menu_h, bg_b, bg_g, bg_r);
    canvas.draw_rect_outline(menu_x, menu_y, menu_w, menu_h, 0, 0, 0);
    // Drop shadow
    canvas.fill_box(menu_x + 2, menu_y + menu_h, menu_w, 2, 0x40, 0x40, 0x40);
    canvas.fill_box(menu_x + menu_w, menu_y + 2, 2, menu_h, 0x40, 0x40, 0x40);

    // Separator line between Maximizar and Fechar (at y = menu_y + 95)
    canvas.fill_box(menu_x + 2, menu_y + 95, menu_w - 4, 1, 0x80, 0x80, 0x80);
    canvas.fill_box(menu_x + 2, menu_y + 96, menu_w - 4, 1, 0xFF, 0xFF, 0xFF);

    let items = [
        ("Restaurar", false, menu_y + 16),
        ("Mover", false, menu_y + 34),
        ("Tamanho", false, menu_y + 52),
        ("Minimizar", true, menu_y + 70),
        ("Maximizar", max_button_enabled, menu_y + 88),
        ("Fechar", true, menu_y + 116),
    ];

    for (label, enabled, text_y) in items {
        let color = if enabled { 0x00000000 } else { 0x00808080 };
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 999,
            x: (menu_x + 14) as i16,
            y: text_y as i16,
            color_rgb: color,
            bk_mode: 1,
            font_height: -12,
            text: label.to_string(),
        }));
    }
}

fn draw_maximize_glyph(canvas: &mut DesktopCanvas, gx: usize, gy: usize, enabled: bool) {
    if enabled {
        canvas.fill_box(gx, gy, 9, 2, 0, 0, 0);
        canvas.fill_box(gx, gy + 7, 9, 1, 0, 0, 0);
        canvas.fill_box(gx, gy, 1, 8, 0, 0, 0);
        canvas.fill_box(gx + 8, gy, 1, 8, 0, 0, 0);
    } else {
        canvas.fill_box(gx + 1, gy + 1, 9, 2, 0xFF, 0xFF, 0xFF);
        canvas.fill_box(gx + 1, gy + 8, 9, 1, 0xFF, 0xFF, 0xFF);
        canvas.fill_box(gx + 1, gy + 1, 1, 8, 0xFF, 0xFF, 0xFF);
        canvas.fill_box(gx + 9, gy + 1, 1, 8, 0xFF, 0xFF, 0xFF);
        canvas.fill_box(gx, gy, 9, 2, 0x80, 0x80, 0x80);
        canvas.fill_box(gx, gy + 7, 9, 1, 0x80, 0x80, 0x80);
        canvas.fill_box(gx, gy, 1, 8, 0x80, 0x80, 0x80);
        canvas.fill_box(gx + 8, gy, 1, 8, 0x80, 0x80, 0x80);
    }
}

fn draw_xp_maximize_glyph(canvas: &mut DesktopCanvas, gx: usize, gy: usize, enabled: bool) {
    let (b, g, r) = if enabled {
        (0xFF, 0xFF, 0xFF)
    } else {
        (0xD0, 0xB8, 0xA0)
    };
    canvas.fill_box(gx, gy, 10, 2, b, g, r);
    canvas.fill_box(gx, gy + 8, 10, 1, b, g, r);
    canvas.fill_box(gx, gy, 1, 9, b, g, r);
    canvas.fill_box(gx + 9, gy, 1, 9, b, g, r);
}

fn draw_drag_frame(canvas: &mut DesktopCanvas, rx: i32, ry: i32, rw: usize, rh: usize, title_h: usize) {
    let x1 = rx;
    let y1 = ry;
    let x2 = rx + rw as i32 - 1;
    let y2 = ry + rh as i32 - 1;
    let tb_y = ry + title_h as i32;

    let set_stipple = |canvas: &mut DesktopCanvas, px: i32, py: i32| {
        if px >= 0 && (px as usize) < canvas.w && py >= 0 && (py as usize) < canvas.h {
            let is_white = (px + py) % 2 == 0;
            let col = if is_white { 0xFF } else { 0x00 };
            canvas.set_px(px as usize, py as usize, col, col, col);
        }
    };

    // 2px thick outer boundary with classic 50% stipple (DrawFocusRect / XOR rubber band)
    for x in x1..=x2 {
        set_stipple(canvas, x, y1);
        set_stipple(canvas, x, y1 + 1);
        set_stipple(canvas, x, y2 - 1);
        set_stipple(canvas, x, y2);
        if tb_y > y1 && tb_y < y2 {
            set_stipple(canvas, x, tb_y);
        }
    }
    for y in y1..=y2 {
        set_stipple(canvas, x1, y);
        set_stipple(canvas, x1 + 1, y);
        set_stipple(canvas, x2 - 1, y);
        set_stipple(canvas, x2, y);
    }
}

fn draw_themed_window(
    canvas: &mut DesktopCanvas,
    wx: usize,
    wy: usize,
    ww: usize,
    wh: usize,
    theme: cb_win32::WindowTheme,
    icon_16: Option<&[u8]>,
    is_active: bool,
    max_button_enabled: bool,
    is_min_pressed: bool,
    is_max_pressed: bool,
    is_close_pressed: bool,
) {
    let (bg_b, bg_g, bg_r) = theme.client_bg_color();
    match theme {
        cb_win32::WindowTheme::Windows311 => {
            // Sizing frame: Light gray (#C0C0C0) 5px
            canvas.fill_box(wx, wy, ww, wh, 0xC0, 0xC0, 0xC0);
            // Outer 1px solid black border
            canvas.draw_rect_outline(wx, wy, ww, wh, 0, 0, 0);

            // Perpendicular cut tick marks (1px black) cutting the 5px border at 18px from corners
            // Forming the authentic Windows 3.11 L-shaped corner resize brackets
            if ww > 38 && wh > 38 {
                // Top border vertical cuts
                canvas.fill_box(wx + 18, wy, 1, 5, 0, 0, 0);
                canvas.fill_box(wx + ww - 19, wy, 1, 5, 0, 0, 0);
                // Bottom border vertical cuts
                canvas.fill_box(wx + 18, wy + wh - 5, 1, 5, 0, 0, 0);
                canvas.fill_box(wx + ww - 19, wy + wh - 5, 1, 5, 0, 0, 0);
                // Left border horizontal cuts
                canvas.fill_box(wx, wy + 18, 5, 1, 0, 0, 0);
                canvas.fill_box(wx, wy + wh - 19, 5, 1, 0, 0, 0);
                // Right border horizontal cuts
                canvas.fill_box(wx + ww - 5, wy + 18, 5, 1, 0, 0, 0);
                canvas.fill_box(wx + ww - 5, wy + wh - 19, 5, 1, 0, 0, 0);
            }

            // Title bar row is 24px thick (wy + 5 to wy + 29)
            let tb_w = ww.saturating_sub(10);
            let tb_h = 24;
            // The entire title bar row inside the 5px border is filled with the title bar color:
            // Active: Dark Blue (#0000A0 -> BGR 0xA0, 0x00, 0x00)
            // Inactive: White (#FFFFFF -> BGR 0xFF, 0xFF, 0xFF)
            let (tb_b, tb_g, tb_r) = if is_active {
                (0xA0, 0x00, 0x00)
            } else {
                (0xFF, 0xFF, 0xFF)
            };
            canvas.fill_box(wx + 5, wy + 5, tb_w, tb_h, tb_b, tb_g, tb_r);
            canvas.draw_rect_outline(wx + 5, wy + 5, tb_w, tb_h, 0, 0, 0);

            // Title bar buttons: Authentic larger Windows 3.11 size: 22px wide x 20px high
            // Vertically centered in 24px bar (wy + 5 + 2 = wy + 7 to wy + 27, leaving 2px blue padding above and below)
            let btn_w = 22;
            let btn_h = 20;
            let by = wy + 7;

            // Left System Menu Box (22x20 at wx + 8, wy + 7) - 3px spacing from left inner border wx + 5
            let sm_x = wx + 8;
            let sm_off = if is_close_pressed { 1 } else { 0 };
            canvas.fill_box(sm_x, by, btn_w, btn_h, 0xC0, 0xC0, 0xC0);
            canvas.draw_rect_outline(sm_x, by, btn_w, btn_h, 0, 0, 0);
            canvas.draw_bevel(sm_x + 1, by + 1, btn_w - 2, btn_h - 2, !is_close_pressed);
            // Centered 14px horizontal slot bar for Windows 3.11 System Menu
            let slot_x = sm_x + 4 + sm_off;
            let slot_y = by + 8 + sm_off;
            canvas.fill_box(slot_x, slot_y, 14, 1, 0, 0, 0);             // Black top line
            canvas.fill_box(slot_x, slot_y + 1, 14, 1, 0x80, 0x80, 0x80); // Gray shadow
            canvas.fill_box(slot_x, slot_y + 2, 14, 1, 0xFF, 0xFF, 0xFF); // White highlight
            canvas.fill_box(slot_x, slot_y, 1, 3, 0, 0, 0);              // Left black cap
            canvas.fill_box(slot_x + 13, slot_y, 1, 3, 0, 0, 0);         // Right black cap

            // Right Maximize button ▲ (22x20 at max_x, wy + 7) - 2px spacing from right inner border wx + ww - 5
            let max_x = wx + ww.saturating_sub(29);
            let max_off = if is_max_pressed { 1 } else { 0 };
            canvas.fill_box(max_x, by, btn_w, btn_h, 0xC0, 0xC0, 0xC0);
            canvas.draw_rect_outline(max_x, by, btn_w, btn_h, 0, 0, 0);
            canvas.draw_bevel(max_x + 1, by + 1, btn_w - 2, btn_h - 2, !is_max_pressed);
            let max_cx = max_x + 11 + max_off;
            let max_cy = by + 10 + max_off;
            if max_button_enabled {
                for (row, width) in [1, 3, 5, 7, 9, 11, 13].iter().enumerate() {
                    let start_x = max_cx - width / 2;
                    canvas.fill_box(start_x, max_cy - 3 + row, *width, 1, 0, 0, 0);
                }
            } else {
                for (row, width) in [1, 3, 5, 7, 9, 11, 13].iter().enumerate() {
                    let start_x = max_cx - width / 2;
                    canvas.fill_box(start_x + 1, max_cy - 2 + row, *width, 1, 0xFF, 0xFF, 0xFF);
                    canvas.fill_box(start_x, max_cy - 3 + row, *width, 1, 0x80, 0x80, 0x80);
                }
            }

            // Right Minimize button ▼ (22x20 at min_x, wy + 7) - separated by 2px from maximize button
            let min_x = wx + ww.saturating_sub(53);
            let min_off = if is_min_pressed { 1 } else { 0 };
            canvas.fill_box(min_x, by, btn_w, btn_h, 0xC0, 0xC0, 0xC0);
            canvas.draw_rect_outline(min_x, by, btn_w, btn_h, 0, 0, 0);
            canvas.draw_bevel(min_x + 1, by + 1, btn_w - 2, btn_h - 2, !is_min_pressed);
            let min_cx = min_x + 11 + min_off;
            let min_cy = by + 10 + min_off;
            for (row, width) in [13, 11, 9, 7, 5, 3, 1].iter().enumerate() {
                let start_x = min_cx - width / 2;
                canvas.fill_box(start_x, min_cy - 3 + row, *width, 1, 0, 0, 0);
            }

            // Client area: Pure white background with black outline (5px border, 29px top)
            if ww > 10 && wh > 34 {
                canvas.fill_box(wx + 5, wy + 29, ww - 10, wh - 34, bg_b, bg_g, bg_r);
                canvas.draw_rect_outline(wx + 5, wy + 29, ww - 10, wh - 34, 0, 0, 0);
            }
        }
        cb_win32::WindowTheme::Windows98 => {
            canvas.fill_box(wx, wy, ww, wh, bg_b, bg_g, bg_r);
            // Sizing frame with soft shadows (NO PITCH BLACK!)
            for x in wx..(wx + ww) {
                canvas.set_px(x, wy, 0xFF, 0xFF, 0xFF);
                if wh > 0 { canvas.set_px(x, wy + wh - 1, 0x40, 0x40, 0x40); }
            }
            for y in wy..(wy + wh) {
                canvas.set_px(wx, y, 0xFF, 0xFF, 0xFF);
                if ww > 0 { canvas.set_px(wx + ww - 1, y, 0x40, 0x40, 0x40); }
            }
            if ww > 2 && wh > 2 {
                for x in (wx + 1)..(wx + ww - 1) {
                    canvas.set_px(x, wy + 1, 0xC8, 0xD0, 0xD4);
                    canvas.set_px(x, wy + wh - 2, 0x80, 0x80, 0x80);
                }
                for y in (wy + 1)..(wy + wh - 1) {
                    canvas.set_px(wx + 1, y, 0xC8, 0xD0, 0xD4);
                    canvas.set_px(wx + ww - 2, y, 0x80, 0x80, 0x80);
                }
            }

            // Title bar: Navy gradient (#000080 -> #1084D0) if active, Gray gradient (#808080 -> #B5B5B5) if inactive
            let tb_w = ww.saturating_sub(6);
            for x in 0..tb_w {
                let fraction = x as f32 / tb_w.max(1) as f32;
                let (b, g, r) = if is_active {
                    let b = (0x80 as f32 + fraction * (0xD0 as f32 - 0x80 as f32)) as u8;
                    let g = (fraction * 0x84 as f32) as u8;
                    let r = (fraction * 0x10 as f32) as u8;
                    (b, g, r)
                } else {
                    let b = (0x80 as f32 + fraction * (0xB5 as f32 - 0x80 as f32)) as u8;
                    let g = (0x80 as f32 + fraction * (0xB5 as f32 - 0x80 as f32)) as u8;
                    let r = (0x80 as f32 + fraction * (0xB5 as f32 - 0x80 as f32)) as u8;
                    (b, g, r)
                };
                for y in 0..18 {
                    canvas.set_px(wx + 3 + x, wy + 3 + y, b, g, r);
                }
            }

            if let Some(ico) = icon_16 {
                draw_icon_16(canvas, wx + 5, wy + 4, ico);
            }

            // Minimize button _ (16x14 at wx + ww - 54, wy + 4)
            draw_win98_button(canvas, wx + ww - 54, wy + 4, 16, 14, is_min_pressed);
            let min_off = if is_min_pressed { 1 } else { 0 };
            canvas.fill_box(wx + ww - 50 + min_off, wy + 13 + min_off, 6, 2, 0, 0, 0);

            // Maximize button [□] (16x14 at wx + ww - 38, wy + 4)
            draw_win98_button(canvas, wx + ww - 38, wy + 4, 16, 14, is_max_pressed);
            let max_off = if is_max_pressed { 1 } else { 0 };
            draw_maximize_glyph(canvas, wx + ww - 35 + max_off, wy + 7 + max_off, max_button_enabled);

            // Close button X (16x14 at wx + ww - 20, wy + 4)
            draw_win98_button(canvas, wx + ww - 20, wy + 4, 16, 14, is_close_pressed);
            let cls_off = if is_close_pressed { 1 } else { 0 };
            draw_x_glyph(canvas, wx + ww - 16 + cls_off, wy + 8 + cls_off, 0, 0, 0);
        }
        cb_win32::WindowTheme::WindowsXp => {
            // Outer 3px border: Royal steel blue (#0045B8) if active, Muted slate (#7080A8) if inactive
            let (bord_b, bord_g, bord_r) = if is_active {
                (0xB8, 0x45, 0x00)
            } else {
                (0xA8, 0x80, 0x70)
            };
            canvas.fill_box(wx, wy, ww, wh, bord_b, bord_g, bord_r);
            if ww > 6 && wh > 32 {
                canvas.fill_box(wx + 3, wy + 29, ww - 6, wh - 32, bg_b, bg_g, bg_r);
            }

            // Title bar (26px high)
            let tb_w = ww.saturating_sub(6);
            for x in 0..tb_w {
                let fraction_y = |y: usize| y as f32 / 26.0;
                if is_active {
                    canvas.set_px(wx + 3 + x, wy + 3, 0xF9, 0x99, 0x3E);
                    for y in 1..26 {
                        let fraction = fraction_y(y);
                        let b = (0xEA as f32 + fraction * (0xD7 as f32 - 0xEA as f32)) as u8;
                        let g = (0x55 as f32 + fraction * (0x65 as f32 - 0x55 as f32)) as u8;
                        let r = (0x00 as f32 + fraction * (0x1E as f32 - 0x00 as f32)) as u8;
                        canvas.set_px(wx + 3 + x, wy + 3 + y, b, g, r);
                    }
                } else {
                    canvas.set_px(wx + 3 + x, wy + 3, 0xE8, 0xC8, 0xB5);
                    for y in 1..26 {
                        let fraction = fraction_y(y);
                        let b = (0xD7 as f32 + fraction * (0xDC as f32 - 0xD7 as f32)) as u8;
                        let g = (0x97 as f32 + fraction * (0xA6 as f32 - 0x97 as f32)) as u8;
                        let r = (0x76 as f32 + fraction * (0x8F as f32 - 0x76 as f32)) as u8;
                        canvas.set_px(wx + 3 + x, wy + 3 + y, b, g, r);
                    }
                }
            }

            // Rounded corners for title bar (2px clip)
            canvas.set_px(wx, wy, 0x80, 0x80, 0x00);
            canvas.set_px(wx + 1, wy, 0x80, 0x80, 0x00);
            canvas.set_px(wx, wy + 1, 0x80, 0x80, 0x00);
            canvas.set_px(wx + ww - 1, wy, 0x80, 0x80, 0x00);
            canvas.set_px(wx + ww - 2, wy, 0x80, 0x80, 0x00);
            canvas.set_px(wx + ww - 1, wy + 1, 0x80, 0x80, 0x00);

            if let Some(ico) = icon_16 {
                draw_icon_16(canvas, wx + 7, wy + 8, ico);
            }

            // XP Minimize button _ (21x21 at wx + ww - 72, wy + 5)
            let (min_b, min_g, min_r) = if is_min_pressed {
                (0xC0, 0x58, 0x18)
            } else if is_active {
                (0xE8, 0x6F, 0x28)
            } else {
                (0xC0, 0x90, 0x7A)
            };
            canvas.fill_box(wx + ww - 72, wy + 5, 21, 21, min_b, min_g, min_r);
            canvas.draw_rect_outline(wx + ww - 72, wy + 5, 21, 21, if is_min_pressed { 0x80 } else { 0xFF }, if is_min_pressed { 0x38 } else { 0xFF }, if is_min_pressed { 0x08 } else { 0xFF });
            let min_off = if is_min_pressed { 1 } else { 0 };
            canvas.fill_box(wx + ww - 66 + min_off, wy + 19 + min_off, 8, 2, 0xFF, 0xFF, 0xFF);

            // XP Maximize button [□] (21x21 at wx + ww - 49, wy + 5)
            let (max_b, max_g, max_r) = if is_max_pressed {
                (0xC0, 0x58, 0x18)
            } else if is_active {
                (0xE8, 0x6F, 0x28)
            } else {
                (0xC0, 0x90, 0x7A)
            };
            canvas.fill_box(wx + ww - 49, wy + 5, 21, 21, max_b, max_g, max_r);
            canvas.draw_rect_outline(wx + ww - 49, wy + 5, 21, 21, if is_max_pressed { 0x80 } else { 0xFF }, if is_max_pressed { 0x38 } else { 0xFF }, if is_max_pressed { 0x08 } else { 0xFF });
            let max_off = if is_max_pressed { 1 } else { 0 };
            draw_xp_maximize_glyph(canvas, wx + ww - 44 + max_off, wy + 11 + max_off, max_button_enabled);

            // XP Close button X (21x21 at wx + ww - 26, wy + 5)
            let (cls_b, cls_g, cls_r) = if is_close_pressed {
                (0x18, 0x30, 0xB0)
            } else if is_active {
                (0x28, 0x44, 0xD8)
            } else {
                (0x78, 0x78, 0xAA)
            };
            canvas.fill_box(wx + ww - 26, wy + 5, 21, 21, cls_b, cls_g, cls_r);
            canvas.draw_rect_outline(wx + ww - 26, wy + 5, 21, 21, if is_close_pressed { 0x70 } else { 0xE8 }, if is_close_pressed { 0x18 } else { 0x65 }, if is_close_pressed { 0x08 } else { 0x50 });
            let cls_off = if is_close_pressed { 1 } else { 0 };
            draw_x_glyph(canvas, wx + ww - 20 + cls_off, wy + 11 + cls_off, 0xFF, 0xFF, 0xFF);
        }
    }
}

fn build_desktop_frames(
    desktop_w: usize,
    desktop_h: usize,
    theme: cb_win32::WindowTheme,
    apps: &[AppInstance],
    selected_icon: Option<usize>,
    active_window: Option<WindowId>,
    z_order: &[WindowId],
    cp_open: bool,
    cp_minimized: bool,
    cp_system_menu_open: bool,
    cp_x: usize,
    cp_y: usize,
    cp_w: usize,
    cp_h: usize,
    cp_icon_16: &[u8],
    cp_icon_32: &[u8],
    time_format_12h: bool,
    pressed_target: Option<PressedTarget>,
    drag_frame: Option<(i32, i32, usize, usize, usize)>,
) -> (Vec<u8>, Vec<cb_webserver::DisplayFrame>) {
    let mut pixels = vec![0u8; desktop_w * desktop_h * 4];
    // 1. Clear desktop to Teal (#008080: B=0x80, G=0x80, R=0x00)
    for chunk in pixels.as_chunks_mut::<4>().0 {
        chunk[0] = 0x80;
        chunk[1] = 0x80;
        chunk[2] = 0x00;
        chunk[3] = 0xFF;
    }

    let mut canvas = DesktopCanvas::new(&mut pixels, desktop_w, desktop_h);
    let mut text_frames = Vec::new();

    // 2. Desktop Icons:
    // When ANY window has active focus, desktop icons are inactive (no selection box)
    let icons_interactive = active_window.is_none();

    // App Desktop Icons along left column
    for (i, app) in apps.iter().enumerate() {
        let icon_y = 26 + i * 90;
        if let Some(ico32) = app.icon_32.as_ref() {
            draw_icon_32(&mut canvas, 40, icon_y, ico32);
        } else {
            let def_ico = generate_default_app_icon_32();
            draw_icon_32(&mut canvas, 40, icon_y, &def_ico);
        }

        let label_text = if !app.caption.is_empty() {
            &app.caption
        } else {
            &app.name
        };
        let text_w = (label_text.len() * 7).clamp(24, 120);
        let text_x = 56i16 - (text_w as i16 / 2);
        let text_y = (icon_y + 49) as i16;
        if icons_interactive && selected_icon == Some(i) {
            let box_w = (text_w + 12).clamp(40, 140);
            let box_x = 56usize.saturating_sub(box_w / 2);
            canvas.fill_box(box_x, icon_y + 36, box_w, 18, 0x80, 0x00, 0x00);
            canvas.draw_rect_outline(box_x, icon_y + 36, box_w, 18, 0xFF, 0xFF, 0xFF);
            text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                hwnd: 0,
                x: text_x,
                y: text_y,
                color_rgb: 0x00FFFFFF,
                bk_mode: 1,
                font_height: -12,
                text: label_text.to_string(),
            }));
        } else {
            text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                hwnd: 0,
                x: text_x + 1,
                y: text_y + 1,
                color_rgb: 0x00000000,
                bk_mode: 1,
                font_height: -12,
                text: label_text.to_string(),
            }));
            text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                hwnd: 0,
                x: text_x,
                y: text_y,
                color_rgb: 0x00FFFFFF,
                bk_mode: 1,
                font_height: -12,
                text: label_text.to_string(),
            }));
        }
    }

    // Control Panel Desktop Icon
    let cp_icon_y = 26 + apps.len() * 90;
    draw_icon_32(&mut canvas, 40, cp_icon_y, cp_icon_32);
    if icons_interactive && selected_icon == Some(apps.len()) {
        canvas.fill_box(18, cp_icon_y + 36, 76, 32, 0x80, 0x00, 0x00);
        canvas.draw_rect_outline(18, cp_icon_y + 36, 76, 32, 0xFF, 0xFF, 0xFF);
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 27,
            y: (cp_icon_y + 49) as i16,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -11,
            text: "Painel de".to_string(),
        }));
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 30,
            y: (cp_icon_y + 62) as i16,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -11,
            text: "Controle".to_string(),
        }));
    } else {
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 28,
            y: (cp_icon_y + 50) as i16,
            color_rgb: 0x00000000,
            bk_mode: 1,
            font_height: -11,
            text: "Painel de".to_string(),
        }));
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 27,
            y: (cp_icon_y + 49) as i16,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -11,
            text: "Painel de".to_string(),
        }));
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 31,
            y: (cp_icon_y + 63) as i16,
            color_rgb: 0x00000000,
            bk_mode: 1,
            font_height: -11,
            text: "Controle".to_string(),
        }));
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 30,
            y: (cp_icon_y + 62) as i16,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -11,
            text: "Controle".to_string(),
        }));
    }

    // Helper closures to verify whether a position or bounding rect is occluded by higher visible windows
    let is_occluded = |tx: i16, ty: i16, higher_rects: &[(usize, usize, usize, usize)]| -> bool {
        for &(hx, hy, hw, hh) in higher_rects {
            if tx >= hx as i16 && tx < (hx + hw) as i16 && ty >= hy as i16 && ty < (hy + hh) as i16 {
                return true;
            }
        }
        false
    };
    let is_occluded_rect = |rx: i16, ry: i16, rw: u16, rh: u16, higher_rects: &[(usize, usize, usize, usize)]| -> bool {
        for &(hx, hy, hw, hh) in higher_rects {
            let hx1 = hx as i16;
            let hy1 = hy as i16;
            let hx2 = (hx + hw) as i16;
            let hy2 = (hy + hh) as i16;
            let rx1 = rx;
            let ry1 = ry;
            let rx2 = rx + rw as i16;
            let ry2 = ry + rh as i16;
            if rx1 < hx2 && rx2 > hx1 && ry1 < hy2 && ry2 > hy1 {
                return true;
            }
        }
        false
    };

    // 3. Render Windows in Z-Order (from bottom to top)
    for (idx, &wid) in z_order.iter().enumerate() {
        // Collect bounding boxes of all higher visible windows in the Z-order
        let mut higher_rects = Vec::new();
        for &hw in &z_order[idx + 1..] {
            match hw {
                WindowId::App(ai) => {
                    if let Some(app) = apps.get(ai) {
                        if app.open && !app.minimized {
                            higher_rects.push((app.win_x, app.win_y, app.win_w, app.win_h));
                        }
                    }
                }
                WindowId::ControlPanel => {
                    if cp_open && !cp_minimized {
                        higher_rects.push((cp_x, cp_y, cp_w, cp_h));
                    }
                }
            }
        }

        match wid {
            WindowId::App(ai) => {
                if let Some(app) = apps.get(ai) {
                    if app.open && !app.minimized {
                        let is_active = active_window == Some(WindowId::App(ai));
                        let is_app_min_pressed = pressed_target == Some(PressedTarget::AppMinimize(ai));
                        let is_app_max_pressed = pressed_target == Some(PressedTarget::AppMaximize(ai));
                        let is_app_close_pressed = pressed_target == Some(PressedTarget::AppClose(ai)) || pressed_target == Some(PressedTarget::AppSysMenu(ai));
                        draw_themed_window(
                            &mut canvas,
                            app.win_x,
                            app.win_y,
                            app.win_w,
                            app.win_h,
                            theme,
                            app.icon_16.as_deref(),
                            is_active,
                            app.max_button_enabled,
                            is_app_min_pressed,
                            is_app_max_pressed,
                            is_app_close_pressed,
                        );

                        let (title_x, title_y) = match theme {
                            cb_win32::WindowTheme::Windows311 => {
                                let text_len = app.caption.len() as i16;
                                let approx_w = text_len * 7;
                                let center_x = (app.win_x + app.win_w / 2) as i16;
                                ((center_x - approx_w / 2).max((app.win_x + 34) as i16), (app.win_y + 21) as i16)
                            }
                            cb_win32::WindowTheme::Windows98 => (
                                if app.icon_16.is_some() { (app.win_x + 26) as i16 } else { (app.win_x + 8) as i16 },
                                (app.win_y + 17) as i16,
                            ),
                            cb_win32::WindowTheme::WindowsXp => (
                                (app.win_x + 28) as i16,
                                (app.win_y + 21) as i16,
                            ),
                        };
                        let title_h = match theme {
                            cb_win32::WindowTheme::Windows311 => 29,
                            cb_win32::WindowTheme::Windows98 => 21,
                            cb_win32::WindowTheme::WindowsXp => 29,
                        };
                        let title_color = theme.title_text_color(is_active);

                        if !is_occluded(title_x, title_y, &higher_rects) && !is_occluded_rect(app.win_x as i16, app.win_y as i16, app.win_w as u16, title_h, &higher_rects) {
                            text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                                hwnd: (100 + ai * 100) as u32,
                                x: title_x,
                                y: title_y,
                                color_rgb: title_color,
                                bk_mode: 1,
                                font_height: -13,
                                text: app.caption.clone(),
                            }));
                        }

                        let (bg_b, bg_g, bg_r) = theme.client_bg_color();
                        for btn in &app.buttons {
                            let b_pixels = btn.render_pixels();
                            for by in 0..btn.height as usize {
                                for bx in 0..btn.width as usize {
                                    let sidx = (by * btn.width as usize + bx) * 4;
                                    canvas.set_px(
                                        btn.x as usize + bx,
                                        btn.y as usize + by,
                                        b_pixels[sidx],
                                        b_pixels[sidx + 1],
                                        b_pixels[sidx + 2],
                                    );
                                }
                            }
                            let tx = (btn.x + 8) as i16;
                            let ty = (btn.y + (btn.height as i32 * 2 / 3).max(14)) as i16;
                            if !is_occluded(tx, ty, &higher_rects)
                                && !is_occluded_rect(btn.x as i16, btn.y as i16, btn.width as u16, btn.height as u16, &higher_rects)
                            {
                                text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                                    hwnd: btn.hwnd,
                                    x: tx,
                                    y: ty,
                                    color_rgb: 0x00000000,
                                    bk_mode: 1,
                                    font_height: -13,
                                    text: btn.caption.clone(),
                                }));
                            }
                        }

                        for tb in &app.textboxes {
                            let (b, g, r) = (tb.back_color as u8, (tb.back_color >> 8) as u8, (tb.back_color >> 16) as u8);
                            canvas.fill_box(tb.x, tb.y, tb.w, tb.h, b, g, r);
                            canvas.draw_bevel(tb.x, tb.y, tb.w, tb.h, false);
                            let tx = (tb.x + 4) as i16;
                            let ty = (tb.y + (tb.h * 2 / 3).max(14)) as i16;
                            if !is_occluded(tx, ty, &higher_rects)
                                && !is_occluded_rect(tb.x as i16, tb.y as i16, tb.w as u16, tb.h as u16, &higher_rects)
                            {
                                text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                                    hwnd: tb.hwnd,
                                    x: tx,
                                    y: ty,
                                    color_rgb: tb.fore_color,
                                    bk_mode: 1,
                                    font_height: -((tb.h * 2 / 3).clamp(12, 22) as i16),
                                    text: tb.text.clone(),
                                }));
                            }
                        }

                        for lbl in &app.labels {
                            canvas.fill_box(lbl.x, lbl.y, lbl.w, lbl.h, bg_b, bg_g, bg_r);
                            let display_caption = cb_win32::format_time_label(&lbl.caption, time_format_12h);
                            let tx = (lbl.x + 4) as i16;
                            let ty = (lbl.y + (lbl.h * 2 / 3).max(14)) as i16;
                            if !is_occluded(tx, ty, &higher_rects)
                                && !is_occluded_rect(lbl.x as i16, lbl.y as i16, lbl.w as u16, lbl.h as u16, &higher_rects)
                            {
                                text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                                    hwnd: lbl.hwnd,
                                    x: tx,
                                    y: ty,
                                    color_rgb: lbl.fore_color,
                                    bk_mode: 1,
                                    font_height: -((lbl.h * 2 / 3).clamp(12, 22) as i16),
                                    text: display_caption,
                                }));
                            }
                        }

                        if app.system_menu_open {
                            let sm_y = match theme {
                                cb_win32::WindowTheme::Windows311 => app.win_y + 27,
                                cb_win32::WindowTheme::Windows98 => app.win_y + 21,
                                cb_win32::WindowTheme::WindowsXp => app.win_y + 29,
                            };
                            draw_system_menu(&mut canvas, &mut text_frames, app.win_x + 8, sm_y, app.max_button_enabled, theme);
                        }
                    }
                }
            }
            WindowId::ControlPanel if cp_open && !cp_minimized => {
                let is_active = active_window == Some(WindowId::ControlPanel);
                let is_cp_min_pressed = pressed_target == Some(PressedTarget::CpMinimize);
                let is_cp_max_pressed = pressed_target == Some(PressedTarget::CpMaximize);
                let is_cp_close_pressed = pressed_target == Some(PressedTarget::CpClose) || pressed_target == Some(PressedTarget::CpSysMenu);
                draw_themed_window(
                    &mut canvas,
                    cp_x,
                    cp_y,
                    cp_w,
                    cp_h,
                    theme,
                    Some(cp_icon_16),
                    is_active,
                    false, // Control Panel maximize is always disabled
                    is_cp_min_pressed,
                    is_cp_max_pressed,
                    is_cp_close_pressed,
                );

                let (cp_title_x, cp_title_y) = match theme {
                    cb_win32::WindowTheme::Windows311 => {
                        let text_len = "Painel de Controle".len() as i16;
                        let approx_w = text_len * 7;
                        let center_x = (cp_x + cp_w / 2) as i16;
                        ((center_x - approx_w / 2).max((cp_x + 34) as i16), (cp_y + 21) as i16)
                    }
                    cb_win32::WindowTheme::Windows98 => ((cp_x + 26) as i16, (cp_y + 17) as i16),
                    cb_win32::WindowTheme::WindowsXp => ((cp_x + 28) as i16, (cp_y + 21) as i16),
                };
                let cp_title_h = match theme {
                    cb_win32::WindowTheme::Windows311 => 29,
                    cb_win32::WindowTheme::Windows98 => 21,
                    cb_win32::WindowTheme::WindowsXp => 29,
                };
                let cp_title_color = theme.title_text_color(is_active);
                if !is_occluded(cp_title_x, cp_title_y, &higher_rects)
                    && !is_occluded_rect(cp_x as i16, cp_y as i16, cp_w as u16, cp_title_h, &higher_rects)
                {
                    text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 500,
                        x: cp_title_x,
                        y: cp_title_y,
                        color_rgb: cp_title_color,
                        bk_mode: 1,
                        font_height: -13,
                        text: "Painel de Controle".to_string(),
                    }));
                }

                // --- Group 1: Formato de Data e Hora ---
                draw_group_box(&mut canvas, cp_x + 14, cp_y + 36, cp_w - 28, 84, theme);
                let g1_x = (cp_x + 24) as i16;
                let g1_y = (cp_y + 40) as i16;
                if !is_occluded(g1_x, g1_y, &higher_rects) {
                    text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 501,
                        x: g1_x,
                        y: g1_y,
                        color_rgb: 0x00000000,
                        bk_mode: 1,
                        font_height: -12,
                        text: "Formato de Data e Hora".to_string(),
                    }));
                }

                let is_r1_pressed = pressed_target == Some(PressedTarget::CpFormat24);
                let is_r2_pressed = pressed_target == Some(PressedTarget::CpFormat12);
                draw_themed_radio_button(&mut canvas, cp_x + 30, cp_y + 60, theme, !time_format_12h || is_r1_pressed);
                let r1_x = (cp_x + 44) as i16;
                let r1_y = (cp_y + 64) as i16;
                if !is_occluded(r1_x, r1_y, &higher_rects) {
                    text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 502,
                        x: r1_x,
                        y: r1_y,
                        color_rgb: 0x00000000,
                        bk_mode: 1,
                        font_height: -12,
                        text: "Formato 24 Horas (ex: 17:30:00)".to_string(),
                    }));
                }

                draw_themed_radio_button(&mut canvas, cp_x + 30, cp_y + 88, theme, time_format_12h || is_r2_pressed);
                let r2_x = (cp_x + 44) as i16;
                let r2_y = (cp_y + 92) as i16;
                if !is_occluded(r2_x, r2_y, &higher_rects) {
                    text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 503,
                        x: r2_x,
                        y: r2_y,
                        color_rgb: 0x00000000,
                        bk_mode: 1,
                        font_height: -12,
                        text: "Formato 12 Horas (AM/PM)".to_string(),
                    }));
                }

                // --- Group 2: Tema das Janelas ---
                draw_group_box(&mut canvas, cp_x + 14, cp_y + 130, cp_w - 28, 126, theme);
                let g2_x = (cp_x + 24) as i16;
                let g2_y = (cp_y + 134) as i16;
                if !is_occluded(g2_x, g2_y, &higher_rects) {
                    text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 504,
                        x: g2_x,
                        y: g2_y,
                        color_rgb: 0x00000000,
                        bk_mode: 1,
                        font_height: -12,
                        text: "Tema Visual do Ambiente".to_string(),
                    }));
                }

                let is_w311 = theme == cb_win32::WindowTheme::Windows311;
                let is_b1_pressed = pressed_target == Some(PressedTarget::CpTheme311);
                draw_theme_button(&mut canvas, cp_x + 22, cp_y + 152, 94, 30, theme, is_w311, is_b1_pressed);
                let b1_off = if is_b1_pressed { 1 } else { 0 };
                let b1_x = (cp_x + 28 + b1_off) as i16;
                let b1_y = (cp_y + 172 + b1_off) as i16;
                if !is_occluded(b1_x, b1_y, &higher_rects) {
                    text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 505,
                        x: b1_x,
                        y: b1_y,
                        color_rgb: 0x00000000,
                        bk_mode: 1,
                        font_height: -12,
                        text: if is_w311 { "• Win 3.11 •".to_string() } else { "Windows 3.11".to_string() },
                    }));
                }

                let is_w98 = theme == cb_win32::WindowTheme::Windows98;
                let is_b2_pressed = pressed_target == Some(PressedTarget::CpTheme98);
                draw_theme_button(&mut canvas, cp_x + 126, cp_y + 152, 94, 30, theme, is_w98, is_b2_pressed);
                let b2_off = if is_b2_pressed { 1 } else { 0 };
                let b2_x = (cp_x + 136 + b2_off) as i16;
                let b2_y = (cp_y + 172 + b2_off) as i16;
                if !is_occluded(b2_x, b2_y, &higher_rects) {
                    text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 506,
                        x: b2_x,
                        y: b2_y,
                        color_rgb: 0x00000000,
                        bk_mode: 1,
                        font_height: -12,
                        text: if is_w98 { "• Win 98 •".to_string() } else { "Windows 98".to_string() },
                    }));
                }

                let is_wxp = theme == cb_win32::WindowTheme::WindowsXp;
                let is_b3_pressed = pressed_target == Some(PressedTarget::CpThemeXp);
                draw_theme_button(&mut canvas, cp_x + 230, cp_y + 152, 94, 30, theme, is_wxp, is_b3_pressed);
                let b3_off = if is_b3_pressed { 1 } else { 0 };
                let b3_x = (cp_x + 242 + b3_off) as i16;
                let b3_y = (cp_y + 172 + b3_off) as i16;
                if !is_occluded(b3_x, b3_y, &higher_rects) {
                    text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 507,
                        x: b3_x,
                        y: b3_y,
                        color_rgb: 0x00000000,
                        bk_mode: 1,
                        font_height: -12,
                        text: if is_wxp { "• Win XP •".to_string() } else { "Windows XP".to_string() },
                    }));
                }

                let desc = match theme {
                    cb_win32::WindowTheme::Windows311 => "Estilo retrô: Windows 3.11 / Win16 (1993)",
                    cb_win32::WindowTheme::Windows98 => "Estilo clássico: Windows 98 / Win32 (1998)",
                    cb_win32::WindowTheme::WindowsXp => "Estilo moderno: Windows XP Luna Azul (2001)",
                };
                let d1_x = (cp_x + 22) as i16;
                let d1_y = (cp_y + 208) as i16;
                if !is_occluded(d1_x, d1_y, &higher_rects) {
                    text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 508,
                        x: d1_x,
                        y: d1_y,
                        color_rgb: 0x00000080,
                        bk_mode: 1,
                        font_height: -11,
                        text: desc.to_string(),
                    }));
                }
                let d2_x = (cp_x + 22) as i16;
                let d2_y = (cp_y + 228) as i16;
                if !is_occluded(d2_x, d2_y, &higher_rects) {
                    text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 509,
                        x: d2_x,
                        y: d2_y,
                        color_rgb: 0x00555555,
                        bk_mode: 1,
                        font_height: -11,
                        text: "Clique nos botões para mudar o tema das janelas.".to_string(),
                    }));
                }

                if cp_system_menu_open {
                    let sm_y = match theme {
                        cb_win32::WindowTheme::Windows311 => cp_y + 27,
                        cb_win32::WindowTheme::Windows98 => cp_y + 21,
                        cb_win32::WindowTheme::WindowsXp => cp_y + 29,
                    };
                    draw_system_menu(&mut canvas, &mut text_frames, cp_x + 8, sm_y, false, theme);
                }
            }
            _ => {}
        }
    }

    if let Some((dx, dy, dw, dh, dtb)) = drag_frame {
        draw_drag_frame(&mut canvas, dx, dy, dw, dh, dtb);
    }

    (pixels, text_frames)
}

fn hit_test_target(
    target: PressedTarget,
    px: i32,
    py: i32,
    apps: &[AppInstance],
    cp_x: usize,
    cp_y: usize,
    cp_w: usize,
    theme: cb_win32::WindowTheme,
) -> bool {
    match target {
        PressedTarget::AppClose(i) => {
            if let Some(app) = apps.get(i) {
                is_close_click(app.win_x, app.win_y, app.win_w, theme, px, py)
            } else {
                false
            }
        }
        PressedTarget::AppSysMenu(i) => {
            if let Some(app) = apps.get(i) {
                is_sysmenu_click(app.win_x, app.win_y, app.win_w, theme, px, py)
            } else {
                false
            }
        }
        PressedTarget::AppMinimize(i) => {
            if let Some(app) = apps.get(i) {
                is_minimize_click(app.win_x, app.win_y, app.win_w, theme, px, py)
            } else {
                false
            }
        }
        PressedTarget::AppMaximize(i) => {
            if let Some(app) = apps.get(i) {
                is_maximize_click(app.win_x, app.win_y, app.win_w, theme, px, py)
            } else {
                false
            }
        }
        PressedTarget::CpClose => is_close_click(cp_x, cp_y, cp_w, theme, px, py),
        PressedTarget::CpSysMenu => is_sysmenu_click(cp_x, cp_y, cp_w, theme, px, py),
        PressedTarget::CpMinimize => is_minimize_click(cp_x, cp_y, cp_w, theme, px, py),
        PressedTarget::CpMaximize => is_maximize_click(cp_x, cp_y, cp_w, theme, px, py),
        PressedTarget::CpFormat24 => px >= (cp_x + 24) as i32 && px < (cp_x + 320) as i32 && py >= (cp_y + 54) as i32 && py < (cp_y + 78) as i32,
        PressedTarget::CpFormat12 => px >= (cp_x + 24) as i32 && px < (cp_x + 320) as i32 && py >= (cp_y + 82) as i32 && py < (cp_y + 108) as i32,
        PressedTarget::CpTheme311 => px >= (cp_x + 22) as i32 && px < (cp_x + 116) as i32 && py >= (cp_y + 152) as i32 && py < (cp_y + 182) as i32,
        PressedTarget::CpTheme98 => px >= (cp_x + 126) as i32 && px < (cp_x + 220) as i32 && py >= (cp_y + 152) as i32 && py < (cp_y + 182) as i32,
        PressedTarget::CpThemeXp => px >= (cp_x + 230) as i32 && px < (cp_x + 324) as i32 && py >= (cp_y + 152) as i32 && py < (cp_y + 182) as i32,
        PressedTarget::FormButton(app_idx, btn_idx) => {
            if let Some(app) = apps.get(app_idx) {
                if let Some(btn) = app.buttons.get(btn_idx) {
                    btn.hit_test(px, py)
                } else {
                    false
                }
            } else {
                false
            }
        }
    }
}

fn is_close_click(wx: usize, wy: usize, ww: usize, theme: cb_win32::WindowTheme, px: i32, py: i32) -> bool {
    if px < 0 || py < 0 {
        return false;
    }
    let px = px as usize;
    let py = py as usize;
    match theme {
        cb_win32::WindowTheme::Windows311 => {
            // In Windows 3.11, the top-left button (System Menu) acts as close on double-click
            px >= wx + 8 && px < wx + 30 && py >= wy + 7 && py < wy + 27
        }
        cb_win32::WindowTheme::Windows98 => {
            px >= wx + ww.saturating_sub(20) && px < wx + ww.saturating_sub(4) && py >= wy + 4 && py < wy + 18
        }
        cb_win32::WindowTheme::WindowsXp => {
            px >= wx + ww.saturating_sub(26) && px < wx + ww.saturating_sub(5) && py >= wy + 5 && py < wy + 26
        }
    }
}

fn is_sysmenu_click(wx: usize, wy: usize, _ww: usize, theme: cb_win32::WindowTheme, px: i32, py: i32) -> bool {
    if px < 0 || py < 0 {
        return false;
    }
    let px = px as usize;
    let py = py as usize;
    match theme {
        cb_win32::WindowTheme::Windows311 => {
            px >= wx + 8 && px < wx + 30 && py >= wy + 7 && py < wy + 27
        }
        cb_win32::WindowTheme::Windows98 => {
            px >= wx + 4 && px < wx + 22 && py >= wy + 3 && py < wy + 20
        }
        cb_win32::WindowTheme::WindowsXp => {
            px >= wx + 4 && px < wx + 24 && py >= wy + 4 && py < wy + 24
        }
    }
}

fn is_maximize_click(wx: usize, wy: usize, ww: usize, theme: cb_win32::WindowTheme, px: i32, py: i32) -> bool {
    if px < 0 || py < 0 {
        return false;
    }
    let px = px as usize;
    let py = py as usize;
    match theme {
        cb_win32::WindowTheme::Windows311 => {
            px >= wx + ww.saturating_sub(29) && px < wx + ww.saturating_sub(7) && py >= wy + 7 && py < wy + 27
        }
        cb_win32::WindowTheme::Windows98 => {
            px >= wx + ww.saturating_sub(38) && px < wx + ww.saturating_sub(22) && py >= wy + 4 && py < wy + 18
        }
        cb_win32::WindowTheme::WindowsXp => {
            px >= wx + ww.saturating_sub(49) && px < wx + ww.saturating_sub(28) && py >= wy + 5 && py < wy + 26
        }
    }
}

fn is_minimize_click(wx: usize, wy: usize, ww: usize, theme: cb_win32::WindowTheme, px: i32, py: i32) -> bool {
    if px < 0 || py < 0 {
        return false;
    }
    let px = px as usize;
    let py = py as usize;
    match theme {
        cb_win32::WindowTheme::Windows311 => {
            px >= wx + ww.saturating_sub(53) && px < wx + ww.saturating_sub(31) && py >= wy + 7 && py < wy + 27
        }
        cb_win32::WindowTheme::Windows98 => {
            px >= wx + ww.saturating_sub(54) && px < wx + ww.saturating_sub(38) && py >= wy + 4 && py < wy + 18
        }
        cb_win32::WindowTheme::WindowsXp => {
            px >= wx + ww.saturating_sub(72) && px < wx + ww.saturating_sub(51) && py >= wy + 5 && py < wy + 26
        }
    }
}

fn apply_theme_to_app(
    theme: cb_win32::WindowTheme,
    desktop_w: usize,
    desktop_h: usize,
    app: &mut AppInstance,
) {
    let (nw, nh) = theme.window_size(app.client_w, app.client_h);
    app.win_w = nw.clamp(120, 1600);
    app.win_h = nh.clamp(60, 1200);
    app.win_x = app.win_x.min(desktop_w.saturating_sub(app.win_w));
    app.win_y = app.win_y.min(desktop_h.saturating_sub(app.win_h));
    let (off_x, off_y) = theme.client_offset();
    for (btn, &(rx, ry)) in app.buttons.iter_mut().zip(app.button_rel_coords.iter()) {
        btn.x = (app.win_x + off_x + rx) as i32;
        btn.y = (app.win_y + off_y + ry) as i32;
    }
    for tb in app.textboxes.iter_mut() {
        tb.x = app.win_x + off_x + tb.rel_x;
        tb.y = app.win_y + off_y + tb.rel_y;
    }
    for lbl in app.labels.iter_mut() {
        lbl.x = app.win_x + off_x + lbl.rel_x;
        lbl.y = app.win_y + off_y + lbl.rel_y;
    }
}

fn apply_theme_to_cp(
    theme: cb_win32::WindowTheme,
    desktop_w: usize,
    desktop_h: usize,
    cp_w: &mut usize,
    cp_h: &mut usize,
    cp_x: &mut usize,
    cp_y: &mut usize,
) {
    let (cp_nw, cp_nh) = theme.window_size(350, 240);
    *cp_w = cp_nw.clamp(120, 800);
    *cp_h = cp_nh.clamp(60, 600);
    *cp_x = (*cp_x).min(desktop_w.saturating_sub(*cp_w));
    *cp_y = (*cp_y).min(desktop_h.saturating_sub(*cp_h));
}

fn load_app_instance(
    file_path: &str,
    app_id: usize,
    theme: cb_win32::WindowTheme,
    quiet: bool,
) -> Result<AppInstance, String> {
    let data = fs::read(file_path).map_err(|e| format!("Failed to read '{}': {}", file_path, e))?;
    let loaded_pe = LoadedPe::load(&data).map_err(|e| format!("PE loader error on '{}': {}", file_path, e))?;
    let vb_hdr_ptr = loaded_pe.find_vb_header();

    let _base = loaded_pe.mapped_base as usize;
    let form_desc = if let Some(vb_hdr_ptr) = vb_hdr_ptr {
        let vb_hdr = unsafe { *vb_hdr_ptr };
        if vb_hdr.lp_gui_table != 0 {
            match unsafe {
                cb_win32::FormDescriptor::from_pe_gui_table(
                    loaded_pe.mapped_base as *const u8,
                    loaded_pe.mapped_size,
                    loaded_pe.preferred_base,
                    vb_hdr.lp_gui_table,
                )
            } {
                Ok(f) => Some(f),
                Err(_) => None,
            }
        } else {
            None
        }
    } else {
        None
    };

    let app_stem = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("App");
    let display_title = app_stem.split('-').next().unwrap_or(app_stem).to_string();

    let mut form = form_desc.unwrap_or_else(|| {
        cb_win32::FormDescriptor {
            name: format!("Form{}", app_id + 1),
            caption: display_title.clone(),
            client_width: 7320,
            client_height: 4530,
            controls: vec![cb_win32::ControlDescriptor {
                name: "Command1".to_string(),
                control_type: cb_win32::ControlType::CommandButton,
                caption: "OK".to_string(),
                left: 2880,
                top: 1800,
                width: 1560,
                height: 540,
                ..Default::default()
            }],
            ..Default::default()
        }
    });

    if form.caption.is_empty() {
        form.caption = display_title.clone();
    }

    if form.icon_data.is_none() {
        for sec in &loaded_pe.sections {
            if sec.name.starts_with(".rsrc") {
                let rsrc_slice = unsafe {
                    core::slice::from_raw_parts(
                        loaded_pe.mapped_base.add(sec.virtual_address as usize),
                        sec.virtual_size as usize,
                    )
                };
                let resources = cb_pe::parse_resource_directory(rsrc_slice, sec.virtual_address);
                if let Some(icon_res) = resources.iter().find(|r| r.type_id == cb_pe::RT_ICON) {
                    let icon_slice = unsafe {
                        core::slice::from_raw_parts(
                            loaded_pe.mapped_base.add(icon_res.rva as usize),
                            icon_res.size as usize,
                        )
                    };
                    form.icon_data = Some(icon_slice.to_vec());
                    break;
                }
            }
        }
    }

    let runtime = cb_win32::FormRuntime::new(form.clone());

    let (icon_16, icon_32) = if let Some((iw, ih, ipx)) = form.decode_icon() {
        if iw == 32 && ih == 32 {
            let ico16 = cb_win32::downsample_32_to_16(&ipx);
            (Some(ico16), Some(ipx))
        } else if iw == 16 && ih == 16 {
            (Some(ipx.clone()), Some(ipx))
        } else {
            let ico16 = cb_win32::downsample_32_to_16(&ipx);
            (Some(ico16), Some(ipx))
        }
    } else {
        (None, None)
    };

    let client_w = (form.pixel_client_width() as usize).max(80);
    let client_h = (form.pixel_client_height() as usize).max(40);
    let (win_w, win_h) = theme.window_size(client_w, client_h);
    let (off_x, off_y) = theme.client_offset();

    let win_x = 100 + app_id * 35;
    let win_y = 50 + app_id * 35;

    let mut button_rel_coords: Vec<(usize, usize)> = Vec::new();
    let buttons: Vec<cb_win32::Win32ButtonControl> = form
        .controls
        .iter()
        .enumerate()
        .filter(|(_, c)| c.control_type == cb_win32::ControlType::CommandButton)
        .map(|(idx, c)| {
            let rel_x = (c.left / 15).max(0) as usize;
            let rel_y = (c.top / 15).max(0) as usize;
            button_rel_coords.push((rel_x, rel_y));
            let abs_x = (win_x + off_x + rel_x) as i32;
            let abs_y = (win_y + off_y + rel_y) as i32;
            let abs_w = (c.width / 15).max(1) as u32;
            let abs_h = (c.height / 15).max(1) as u32;
            cb_win32::Win32ButtonControl::new(
                (200 + app_id * 100 + idx) as u32,
                abs_x,
                abs_y,
                abs_w,
                abs_h,
                &c.caption,
            )
        })
        .collect();

    let textboxes: Vec<DynamicTextBox> = form
        .controls
        .iter()
        .enumerate()
        .filter(|(_, c)| c.control_type == cb_win32::ControlType::TextBox)
        .map(|(idx, c)| {
            let rel_x = (c.left / 15).max(0) as usize;
            let rel_y = (c.top / 15).max(0) as usize;
            let abs_x = win_x + off_x + rel_x;
            let abs_y = win_y + off_y + rel_y;
            let abs_w = (c.width / 15).max(1) as usize;
            let abs_h = (c.height / 15).max(1) as usize;
            DynamicTextBox {
                hwnd: (200 + app_id * 100 + idx) as u32,
                x: abs_x,
                y: abs_y,
                w: abs_w,
                h: abs_h,
                rel_x,
                rel_y,
                text: if !c.text.is_empty() { c.text.clone() } else { "0".to_string() },
                back_color: c.back_color.unwrap_or(0x00FFFFFF),
                fore_color: c.fore_color.unwrap_or(0x00000000),
            }
        })
        .collect();

    let labels: Vec<DynamicLabel> = form
        .controls
        .iter()
        .enumerate()
        .filter(|(_, c)| c.control_type == cb_win32::ControlType::Label)
        .map(|(idx, c)| {
            let rel_x = (c.left / 15).max(0) as usize;
            let rel_y = (c.top / 15).max(0) as usize;
            let abs_x = win_x + off_x + rel_x;
            let abs_y = win_y + off_y + rel_y;
            let abs_w = (c.width / 15).max(1) as usize;
            let abs_h = (c.height / 15).max(1) as usize;
            DynamicLabel {
                hwnd: (200 + app_id * 100 + idx) as u32,
                name: c.name.clone(),
                x: abs_x,
                y: abs_y,
                w: abs_w,
                h: abs_h,
                rel_x,
                rel_y,
                caption: c.caption.clone(),
                fore_color: c.fore_color.unwrap_or(0x00000000),
            }
        })
        .collect();

    let timers: Vec<ActiveTimer> = form
        .controls
        .iter()
        .filter(|c| c.control_type == cb_win32::ControlType::Timer)
        .map(|c| {
            let interval = runtime.control_interval(&c.name).unwrap_or(1000).max(50) as u64;
            ActiveTimer {
                name: c.name.clone(),
                interval_ms: interval,
                last_tick: std::time::Instant::now(),
            }
        })
        .collect();

    let max_button_enabled = form.max_button;

    let _ = loaded_pe.execute_with_mode(ExecutionMode::GuiWebStream);

    if !quiet {
        println!("[+] Initialized app #{}: \"{}\" ({}) with {} controls", app_id, form.caption, file_path, form.controls.len());
    }

    Ok(AppInstance {
        id: app_id,
        exe_path: file_path.to_string(),
        name: display_title,
        caption: form.caption.clone(),
        client_w,
        client_h,
        win_w,
        win_h,
        win_x,
        win_y,
        open: true,
        minimized: false,
        system_menu_open: false,
        max_button_enabled,
        icon_16,
        icon_32,
        buttons,
        button_rel_coords,
        textboxes,
        labels,
        timers,
        runtime,
        loaded_pe,
    })
}

/// CLI entry point for loading and running Visual Basic 6 executables on Linux/macOS.
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_banner();
        print_usage(&args[0]);
        process::exit(1);
    }

    let mut exe_paths: Vec<String> = Vec::new();
    let mut inspect_only = false;
    let mut dump_info = false;
    let mut execution_mode = ExecutionMode::Headless;
    let mut port: u16 = env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
    let mut quiet_flag = false;
    let mut verbose = false;

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--help" || arg == "-h" {
            print_banner();
            print_usage(&args[0]);
            process::exit(0);
        } else if arg == "--inspect" {
            inspect_only = true;
        } else if arg == "--dump-info" {
            dump_info = true;
        } else if arg == "--run" {
            // default run mode
        } else if arg == "--headless" {
            execution_mode = ExecutionMode::Headless;
        } else if arg == "--gui-web" {
            execution_mode = ExecutionMode::GuiWebStream;
        } else if arg == "--quiet" || arg == "-q" {
            quiet_flag = true;
        } else if arg == "--verbose" || arg == "-v" {
            verbose = true;
        } else if arg == "--" || arg.is_empty() {
            i += 1;
            continue;
        } else if arg == "--port" {
            i += 1;
            if i < args.len() {
                if let Ok(p) = args[i].parse::<u16>() {
                    port = p;
                }
            }
        } else if let Some(stripped) = arg.strip_prefix("--port=") {
            if let Ok(p) = stripped.parse::<u16>() {
                port = p;
            }
        } else if !arg.starts_with("--") {
            exe_paths.push(arg.clone());
        } else {
            eprintln!("Unknown option: {}", arg);
            print_usage(&args[0]);
            process::exit(1);
        }
        i += 1;
    }

    let quiet = if verbose {
        false
    } else {
        execution_mode == ExecutionMode::GuiWebStream || quiet_flag
    };

    if exe_paths.is_empty() {
        let default_candidates = [
            "artifacts/bin/Samples/Alarm/Alarm-native.exe",
            "artifacts/bin/Samples/Atm/ATM32-native.EXE",
            "artifacts/bin/Samples/Blanker/Blanker-native.exe",
        ];
        for c in &default_candidates {
            if Path::new(c).exists() {
                exe_paths.push(c.to_string());
            }
        }
        if exe_paths.is_empty() {
            if Path::new("tests/fixtures/Alarm-native.exe").exists() {
                exe_paths.push("tests/fixtures/Alarm-native.exe".to_string());
            } else {
                eprintln!("Error: No executable file specified and no default sample binaries found.");
                process::exit(1);
            }
        }
    } else if exe_paths.len() == 1 && execution_mode == ExecutionMode::GuiWebStream {
        let samples = [
            "artifacts/bin/Samples/Alarm/Alarm-native.exe",
            "artifacts/bin/Samples/Atm/ATM32-native.EXE",
            "artifacts/bin/Samples/Blanker/Blanker-native.exe",
        ];
        for s in &samples {
            if Path::new(s).exists() && !exe_paths.iter().any(|p| p == *s) {
                exe_paths.push(s.to_string());
            }
        }
    }

    if inspect_only || dump_info {
        for file_path in &exe_paths {
            if !Path::new(file_path).exists() {
                eprintln!("Error: File not found: '{}'", file_path);
                continue;
            }
            if !quiet {
                print_banner();
                println!("[*] Reading binary: {}", file_path);
            }
            let data = match fs::read(file_path) {
                Ok(bytes) => bytes,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    continue;
                }
            };
            let loaded_pe = match LoadedPe::load(&data) {
                Ok(pe) => pe,
                Err(e) => {
                    eprintln!("PE Loader Error: {}", e);
                    continue;
                }
            };
            if !quiet {
                println!("[+] PE32 successfully mapped: base=0x{:08X}", loaded_pe.preferred_base);
            }
            let vb_hdr_ptr = loaded_pe.find_vb_header();
            if let Some(ptr) = vb_hdr_ptr {
                let vb_hdr = unsafe { *ptr };
                let build = vb_hdr.runtime_build_version;
                let forms = vb_hdr.w_forms_count;
                let threads = vb_hdr.dw_thread_count;
                println!("[+] Visual Basic 6 Header found in '{}': Build=0x{:04X}, Forms={}, Threads={}", file_path, build, forms, threads);
            } else {
                println!("[-] No 'VB5!' header signature found in '{}'", file_path);
            }
        }
        return;
    }

    if execution_mode == ExecutionMode::Headless {
        let file_path = &exe_paths[0];
        if !Path::new(file_path).exists() {
            eprintln!("Error: File not found: '{}'", file_path);
            process::exit(1);
        }
        let data = match fs::read(file_path) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("Error reading file: {}", e);
                process::exit(1);
            }
        };
        if !quiet {
            print_banner();
            println!("[*] Reading binary: {}", file_path);
            println!("[*] Parsing PE32 and mapping sections ({} bytes)...", data.len());
        }
        let loaded_pe = match LoadedPe::load(&data) {
            Ok(pe) => pe,
            Err(e) => {
                eprintln!("PE Loader Error: {}", e);
                process::exit(1);
            }
        };
        if !quiet {
            println!("[+] PE32 successfully mapped into virtual memory:");
            println!("    - Preferred Base: 0x{:08X}", loaded_pe.preferred_base);
            println!("    - Allocated Base: {:p}", loaded_pe.mapped_base);
            println!("    - Relocation Delta: 0x{:X}", loaded_pe.delta);
            println!("    - Entry Point RVA: 0x{:08X}", loaded_pe.entry_point_rva);
            println!("    - Sections Count: {}", loaded_pe.sections.len());
            for (i, sec) in loaded_pe.sections.iter().enumerate() {
                println!(
                    "      [{}] {:<8} VA: 0x{:08X} (VSize: 0x{:06X}, RawSize: 0x{:06X})",
                    i, sec.name, sec.virtual_address, sec.virtual_size, sec.size_of_raw_data
                );
            }
            println!("\n[*] Scanning for Visual Basic 6 metadata ('VB5!' signature)...");
        }
        let vb_hdr_ptr = loaded_pe.find_vb_header();
        match vb_hdr_ptr {
            Some(ptr) => {
                let vb_hdr = unsafe { *ptr };
                let build = vb_hdr.runtime_build_version;
                let rev = vb_hdr.runtime_revision;
                let forms = vb_hdr.w_forms_count;
                let threads = vb_hdr.dw_thread_count;
                let sub_main = vb_hdr.lp_sub_main;
                if !quiet {
                    println!("[+] Visual Basic 6 Executable Verified!");
                    println!("    - Build Version: 0x{:04X}, Revision: 0x{:04X}", build, rev);
                    println!("    - Forms Count: {}", forms);
                    println!("    - Thread Count: {}", threads);
                    println!("    - Sub Main Address: 0x{:08X}", sub_main);
                }

                let base = loaded_pe.mapped_base as usize;
                let size = loaded_pe.mapped_size;
                let is_valid_ptr = |ptr: u32| -> bool {
                    let p = ptr as usize;
                    p >= base && p < base + size
                };

                if is_valid_ptr(vb_hdr.lp_project_data) {
                    let proj_info = unsafe { &*(vb_hdr.lp_project_data as *const msvbvm60::types::ProjectInfo) };
                    if is_valid_ptr(proj_info.lp_project_name) {
                        let name_cstr = unsafe { std::ffi::CStr::from_ptr(proj_info.lp_project_name as *const i8) };
                        if !quiet {
                            println!("    - Project Name: {}", name_cstr.to_string_lossy());
                        }
                    }
                    if is_valid_ptr(proj_info.lp_project_title) {
                        let title_cstr = unsafe { std::ffi::CStr::from_ptr(proj_info.lp_project_title as *const i8) };
                        if !quiet {
                            println!("    - Project Title: {}", title_cstr.to_string_lossy());
                        }
                    }
                    if is_valid_ptr(proj_info.lp_project_description) {
                        let desc_cstr = unsafe { std::ffi::CStr::from_ptr(proj_info.lp_project_description as *const i8) };
                        if !quiet {
                            println!("    - Description: {}", desc_cstr.to_string_lossy());
                        }
                    }
                }

                if vb_hdr.lp_gui_table != 0 {
                    if let Ok(f) = unsafe {
                        cb_win32::FormDescriptor::from_pe_gui_table(
                            loaded_pe.mapped_base as *const u8,
                            loaded_pe.mapped_size,
                            loaded_pe.preferred_base,
                            vb_hdr.lp_gui_table,
                        )
                    } {
                        if !quiet {
                            println!(
                                "[+] Dynamically deserialized Form from PE GUI Table: \"{}\" (caption: \"{}\", {} controls)",
                                f.name,
                                f.caption,
                                f.controls.len()
                            );
                            for c in &f.controls {
                                println!("    - Control: {} ({:?}) left={} top={} w={} h={} caption=\"{}\" text=\"{}\"", c.name, c.control_type, c.left, c.top, c.width, c.height, c.caption, c.text);
                            }
                        }
                        cb_win32::ensure_default_form_runtime();
                        {
                            let mut lock = cb_win32::global_form_runtime().lock().unwrap();
                            *lock = Some(cb_win32::FormRuntime::new(f));
                        }
                    }
                }

                let exit_code = match loaded_pe.execute_with_mode(ExecutionMode::Headless) {
                    Ok(code) => code,
                    Err(e) => {
                        eprintln!("Execution error: {}", e);
                        1
                    }
                };
                if !quiet {
                    println!("\n[+] Execution completed with return code: {}", exit_code);
                }
                process::exit(exit_code);
            }
            None => {
                println!("[-] No 'VB5!' header signature found. (Is this a standard C/C++ PE?)");
                process::exit(1);
            }
        }
    }

    // --- GUI WebStream Multi-Application Desktop Execution ---
    let mut theme = cb_win32::WindowTheme::Windows311;
    let mut time_format_12h = false;
    let mut desktop_w = 800usize;
    let mut desktop_h = 600usize;

    let (mut cp_w, mut cp_h) = theme.window_size(350, 240);
    let mut cp_x = (desktop_w.saturating_sub(cp_w)) / 2;
    let mut cp_y = (desktop_h.saturating_sub(cp_h)) / 2;
    let mut cp_open = false;
    let mut cp_minimized = false;
    let mut cp_system_menu_open = false;

    let cp_icon_32 = cb_win32::generate_win311_control_panel_icon();
    let cp_icon_16 = cb_win32::downsample_32_to_16(&cp_icon_32);

    let mut apps: Vec<AppInstance> = Vec::new();
    for (idx, path) in exe_paths.iter().enumerate() {
        if Path::new(path).exists() {
            match load_app_instance(path, idx, theme, quiet) {
                Ok(app) => apps.push(app),
                Err(e) => {
                    if !quiet {
                        eprintln!("Warning: Failed to load app '{}': {}", path, e);
                    }
                }
            }
        }
    }

    if apps.is_empty() {
        eprintln!("Error: No applications could be loaded.");
        process::exit(1);
    }

    let mut z_order: Vec<WindowId> = Vec::new();
    for i in 0..apps.len() {
        z_order.push(WindowId::App(i));
    }
    z_order.push(WindowId::ControlPanel);

    let mut active_window: Option<WindowId> = Some(WindowId::App(0));
    let mut selected_icon: Option<usize> = None;

    let mut static_assets = std::collections::HashMap::new();
    static_assets.insert(
        "/index.html".to_string(),
        ("text/html", cb_client::INDEX_HTML.as_bytes().to_vec()),
    );
    static_assets.insert(
        "/classicbasic.js".to_string(),
        ("application/javascript", cb_client::JS_SHIM.as_bytes().to_vec()),
    );
    static_assets.insert(
        "/classicbasic_client.wasm".to_string(),
        ("application/wasm", cb_client::WASM_BINARY.to_vec()),
    );

    let (init_pixels, init_text_frames) = build_desktop_frames(
        desktop_w,
        desktop_h,
        theme,
        &apps,
        selected_icon,
        active_window,
        &z_order,
        cp_open,
        cp_minimized,
        cp_system_menu_open,
        cp_x,
        cp_y,
        cp_w,
        cp_h,
        &cp_icon_16,
        &cp_icon_32,
        time_format_12h,
        None,
        None,
    );

    let mut initial_frames = Vec::new();
    initial_frames.push(cb_webserver::DisplayFrame::BitmapRect(cb_webserver::BitmapRectFrame {
        hwnd: 100,
        x: 0,
        y: 0,
        width: desktop_w as u16,
        height: desktop_h as u16,
        pixels: init_pixels,
    }));
    initial_frames.extend(init_text_frames);

    let config = cb_webserver::ServerConfig {
        bind_addr: format!("0.0.0.0:{}", port),
        static_assets,
        initial_frames,
    };
    let queue = std::sync::Arc::new(cb_win32::VirtualMessageQueue::new());
    let queue_clone = std::sync::Arc::clone(&queue);
    let server = match cb_webserver::start_server(config, queue_clone, None::<fn(_)>) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Failed to start web server: {}", e);
            process::exit(1);
        }
    };

    let redraw_desktop = |desktop_w: usize,
                          desktop_h: usize,
                          theme: cb_win32::WindowTheme,
                          apps: &[AppInstance],
                          selected_icon: Option<usize>,
                          active_window: Option<WindowId>,
                          z_order: &[WindowId],
                          cp_open: bool,
                          cp_minimized: bool,
                          cp_system_menu_open: bool,
                          cp_x: usize,
                          cp_y: usize,
                          cp_w: usize,
                          cp_h: usize,
                          time_format_12h: bool,
                          server: &cb_webserver::ServerHandle,
                          pressed_target: Option<PressedTarget>,
                          drag_frame: Option<(i32, i32, usize, usize, usize)>| {
        let (pixels, text_frames) = build_desktop_frames(
            desktop_w,
            desktop_h,
            theme,
            apps,
            selected_icon,
            active_window,
            z_order,
            cp_open,
            cp_minimized,
            cp_system_menu_open,
            cp_x,
            cp_y,
            cp_w,
            cp_h,
            &cp_icon_16,
            &cp_icon_32,
            time_format_12h,
            pressed_target,
            drag_frame,
        );
        server.broadcast_frame(&cb_webserver::DisplayFrame::BitmapRect(cb_webserver::BitmapRectFrame {
            hwnd: 100,
            x: 0,
            y: 0,
            width: desktop_w as u16,
            height: desktop_h as u16,
            pixels,
        }));
        for tf in text_frames {
            server.broadcast_frame(&tf);
        }
    };

    let mut pressed_target: Option<PressedTarget> = None;
    let mut dragging_window: Option<WindowId> = None;
    let mut drag_offset: (i32, i32) = (0, 0);
    let mut drag_curr_rect: Option<(i32, i32, usize, usize)> = None;
    let mut last_drag_redraw = std::time::Instant::now();
    let mut last_rendered_drag_pos = (-9999i32, -9999i32);

    if !quiet {
        println!("\n╔═══════════════════════════════════════════════════════════════╗");
        println!("║   ClassicBasic Multi-App Desktop on http://0.0.0.0:{:<5}       ║", port);
        println!("║   Open http://localhost:{:<5} in your browser to interact    ║", port);
        println!("╚═══════════════════════════════════════════════════════════════╝\n");
    } else {
        println!("[ClassicBasic Multi-App GUI] Running on http://localhost:{} (Ctrl+C to stop)", port);
    }

    loop {
        // 1. Dispatch periodic timers for all active applications
        let now = std::time::Instant::now();
        let mut timer_redraw_needed = false;
        for app in apps.iter_mut() {
            if !app.open || app.minimized {
                continue;
            }
            for timer in app.timers.iter_mut() {
                if now.duration_since(timer.last_tick).as_millis() >= timer.interval_ms as u128 {
                    timer.last_tick = now;
                    let _ = msvbvm60::thun_rt::dispatch_native_control_click(&timer.name);
                    if let Ok(rt_lock) = cb_win32::global_form_runtime().lock() {
                        if let Some(rt) = rt_lock.as_ref() {
                            for lbl in app.labels.iter_mut() {
                                if let Some(c) = rt.control_caption(&lbl.name) {
                                    if c != lbl.caption {
                                        lbl.caption = c;
                                        timer_redraw_needed = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if timer_redraw_needed && dragging_window.is_none() {
            redraw_desktop(
                desktop_w, desktop_h, theme, &apps, selected_icon, active_window,
                &z_order, cp_open, cp_minimized, cp_system_menu_open,
                cp_x, cp_y, cp_w, cp_h, time_format_12h, &server, pressed_target, None,
            );
        }

        // 2. Poll Win32 message queue for mouse / window / resize events
        if let Some(msg) = queue.peek_message(0, 0, 0, true) {
            match msg.message {
                cb_win32::WM_LBUTTONDOWN => {
                    let px = msg.pt_x;
                    let py = msg.pt_y;
                    let mut handled_menu_click = false;

                    // A. Check if user clicked inside an active popup system menu
                    let mut app_menu_action: Option<(usize, usize)> = None;
                    for (ai, app) in apps.iter_mut().enumerate() {
                        if app.system_menu_open {
                            let sm_x = app.win_x + 8;
                            let sm_y = match theme {
                                cb_win32::WindowTheme::Windows311 => app.win_y + 27,
                                cb_win32::WindowTheme::Windows98 => app.win_y + 21,
                                cb_win32::WindowTheme::WindowsXp => app.win_y + 29,
                            };
                            if px >= sm_x as i32 && px < (sm_x + 145) as i32 && py >= sm_y as i32 && py < (sm_y + 132) as i32 {
                                handled_menu_click = true;
                                app.system_menu_open = false;
                                let rel_y = py as usize - sm_y;
                                if rel_y >= 58 && rel_y < 76 {
                                    app_menu_action = Some((ai, 1));
                                } else if rel_y >= 98 && rel_y < 128 {
                                    app_menu_action = Some((ai, 2));
                                }
                            } else {
                                app.system_menu_open = false;
                            }
                        }
                    }

                    if let Some((ai, act)) = app_menu_action {
                        if act == 1 {
                            apps[ai].minimized = true;
                        } else if act == 2 {
                            apps[ai].open = false;
                        }
                        active_window = z_order.iter().rev().find(|&&w| match w {
                            WindowId::App(j) => apps[j].open && !apps[j].minimized,
                            WindowId::ControlPanel => cp_open && !cp_minimized,
                        }).copied();
                    }

                    if cp_system_menu_open {
                        let sm_x = cp_x + 8;
                        let sm_y = match theme {
                            cb_win32::WindowTheme::Windows311 => cp_y + 27,
                            cb_win32::WindowTheme::Windows98 => cp_y + 21,
                            cb_win32::WindowTheme::WindowsXp => cp_y + 29,
                        };
                        if px >= sm_x as i32 && px < (sm_x + 145) as i32 && py >= sm_y as i32 && py < (sm_y + 132) as i32 {
                            handled_menu_click = true;
                            cp_system_menu_open = false;
                            let rel_y = py as usize - sm_y;
                            if rel_y >= 58 && rel_y < 76 {
                                // Minimizar
                                cp_minimized = true;
                                active_window = z_order.iter().rev().find(|&&w| match w {
                                    WindowId::App(j) => apps[j].open && !apps[j].minimized,
                                    WindowId::ControlPanel => cp_open && !cp_minimized,
                                }).copied();
                            } else if rel_y >= 98 && rel_y < 128 {
                                // Fechar
                                cp_open = false;
                                active_window = z_order.iter().rev().find(|&&w| match w {
                                    WindowId::App(j) => apps[j].open && !apps[j].minimized,
                                    WindowId::ControlPanel => cp_open && !cp_minimized,
                                }).copied();
                            }
                        } else {
                            cp_system_menu_open = false;
                        }
                    }

                    if handled_menu_click {
                        redraw_desktop(
                            desktop_w, desktop_h, theme, &apps, selected_icon, active_window,
                            &z_order, cp_open, cp_minimized, cp_system_menu_open,
                            cp_x, cp_y, cp_w, cp_h, time_format_12h, &server, None, None,
                        );
                        continue;
                    }

                    // B. Hit test windows in reverse Z-order (topmost first)
                    let mut activate_target: Option<WindowId> = None;
                    let mut window_hit = false;

                    for &wid in z_order.iter().rev() {
                        match wid {
                            WindowId::ControlPanel if cp_open && !cp_minimized => {
                                if px >= cp_x as i32 && px < (cp_x + cp_w) as i32 && py >= cp_y as i32 && py < (cp_y + cp_h) as i32 {
                                    window_hit = true;
                                    activate_target = Some(WindowId::ControlPanel);

                                    if is_sysmenu_click(cp_x, cp_y, cp_w, theme, px, py) {
                                        pressed_target = Some(PressedTarget::CpSysMenu);
                                    } else if is_close_click(cp_x, cp_y, cp_w, theme, px, py) {
                                        pressed_target = Some(PressedTarget::CpClose);
                                    } else if is_minimize_click(cp_x, cp_y, cp_w, theme, px, py) {
                                        pressed_target = Some(PressedTarget::CpMinimize);
                                    } else if is_maximize_click(cp_x, cp_y, cp_w, theme, px, py) {
                                        pressed_target = Some(PressedTarget::CpMaximize);
                                    } else if py < (cp_y + theme.client_offset().1) as i32 {
                                        dragging_window = Some(WindowId::ControlPanel);
                                        drag_offset = (px - cp_x as i32, py - cp_y as i32);
                                        drag_curr_rect = Some((cp_x as i32, cp_y as i32, cp_w, cp_h));
                                    } else if px >= (cp_x + 24) as i32 && px < (cp_x + 320) as i32 && py >= (cp_y + 54) as i32 && py < (cp_y + 78) as i32 {
                                        pressed_target = Some(PressedTarget::CpFormat24);
                                    } else if px >= (cp_x + 24) as i32 && px < (cp_x + 320) as i32 && py >= (cp_y + 82) as i32 && py < (cp_y + 108) as i32 {
                                        pressed_target = Some(PressedTarget::CpFormat12);
                                    } else if px >= (cp_x + 22) as i32 && px < (cp_x + 116) as i32 && py >= (cp_y + 152) as i32 && py < (cp_y + 182) as i32 {
                                        pressed_target = Some(PressedTarget::CpTheme311);
                                    } else if px >= (cp_x + 126) as i32 && px < (cp_x + 220) as i32 && py >= (cp_y + 152) as i32 && py < (cp_y + 182) as i32 {
                                        pressed_target = Some(PressedTarget::CpTheme98);
                                    } else if px >= (cp_x + 230) as i32 && px < (cp_x + 324) as i32 && py >= (cp_y + 152) as i32 && py < (cp_y + 182) as i32 {
                                        pressed_target = Some(PressedTarget::CpThemeXp);
                                    }
                                    break;
                                }
                            }
                            WindowId::App(ai) => {
                                if let Some(app) = apps.get_mut(ai) {
                                    if app.open && !app.minimized {
                                        if px >= app.win_x as i32 && px < (app.win_x + app.win_w) as i32 && py >= app.win_y as i32 && py < (app.win_y + app.win_h) as i32 {
                                            window_hit = true;
                                            activate_target = Some(WindowId::App(ai));

                                            if is_sysmenu_click(app.win_x, app.win_y, app.win_w, theme, px, py) {
                                                pressed_target = Some(PressedTarget::AppSysMenu(ai));
                                            } else if is_close_click(app.win_x, app.win_y, app.win_w, theme, px, py) {
                                                pressed_target = Some(PressedTarget::AppClose(ai));
                                            } else if is_minimize_click(app.win_x, app.win_y, app.win_w, theme, px, py) {
                                                pressed_target = Some(PressedTarget::AppMinimize(ai));
                                            } else if is_maximize_click(app.win_x, app.win_y, app.win_w, theme, px, py) {
                                                if app.max_button_enabled {
                                                    pressed_target = Some(PressedTarget::AppMaximize(ai));
                                                }
                                            } else if py < (app.win_y + theme.client_offset().1) as i32 {
                                                dragging_window = Some(WindowId::App(ai));
                                                drag_offset = (px - app.win_x as i32, py - app.win_y as i32);
                                                drag_curr_rect = Some((app.win_x as i32, app.win_y as i32, app.win_w, app.win_h));
                                            } else {
                                                for (btn_idx, btn) in app.buttons.iter_mut().enumerate() {
                                                    if btn.on_lbutton_down(px, py) {
                                                        pressed_target = Some(PressedTarget::FormButton(ai, btn_idx));
                                                        break;
                                                    }
                                                }
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    if let Some(target) = activate_target {
                        z_order.retain(|&w| w != target);
                        z_order.push(target);
                        active_window = Some(target);
                        selected_icon = None; // Desktop icons are inactive when any window has focus
                        redraw_desktop(
                            desktop_w, desktop_h, theme, &apps, selected_icon, active_window,
                            &z_order, cp_open, cp_minimized, cp_system_menu_open,
                            cp_x, cp_y, cp_w, cp_h, time_format_12h, &server, pressed_target, None,
                        );
                    } else if !window_hit {
                        // Clicked desktop background or desktop icon
                        active_window = None;
                        selected_icon = None;

                        for (i, _) in apps.iter().enumerate() {
                            let icon_y = (26 + i * 90) as i32;
                            if px >= 16 && px < 96 && py >= icon_y - 10 && py < icon_y + 70 {
                                selected_icon = Some(i);
                                break;
                            }
                        }

                        let cp_icon_y = (26 + apps.len() * 90) as i32;
                        if px >= 16 && px < 100 && py >= cp_icon_y - 10 && py < cp_icon_y + 70 {
                            selected_icon = Some(apps.len());
                        }

                        redraw_desktop(
                            desktop_w, desktop_h, theme, &apps, selected_icon, active_window,
                            &z_order, cp_open, cp_minimized, cp_system_menu_open,
                            cp_x, cp_y, cp_w, cp_h, time_format_12h, &server, pressed_target, None,
                        );
                    }
                }

                cb_win32::WM_LBUTTONDBLCLK => {
                    let px = msg.pt_x;
                    let py = msg.pt_y;

                    // Double click on title bar system menu immediately closes window (Win 3.11 behavior)
                    if let Some(wid) = active_window {
                        match wid {
                            WindowId::App(ai) => {
                                if let Some(app) = apps.get_mut(ai) {
                                    if is_sysmenu_click(app.win_x, app.win_y, app.win_w, theme, px, py) {
                                        app.open = false;
                                        app.system_menu_open = false;
                                        active_window = z_order.iter().rev().find(|&&w| match w {
                                            WindowId::App(j) => apps[j].open && !apps[j].minimized,
                                            WindowId::ControlPanel => cp_open && !cp_minimized,
                                        }).copied();
                                        redraw_desktop(
                                            desktop_w, desktop_h, theme, &apps, selected_icon, active_window,
                                            &z_order, cp_open, cp_minimized, cp_system_menu_open,
                                            cp_x, cp_y, cp_w, cp_h, time_format_12h, &server, None, None,
                                        );
                                        continue;
                                    }
                                }
                            }
                            WindowId::ControlPanel => {
                                if is_sysmenu_click(cp_x, cp_y, cp_w, theme, px, py) {
                                    cp_open = false;
                                    cp_system_menu_open = false;
                                    active_window = z_order.iter().rev().find(|&&w| match w {
                                        WindowId::App(j) => apps[j].open && !apps[j].minimized,
                                        WindowId::ControlPanel => cp_open && !cp_minimized,
                                    }).copied();
                                    redraw_desktop(
                                        desktop_w, desktop_h, theme, &apps, selected_icon, active_window,
                                        &z_order, cp_open, cp_minimized, cp_system_menu_open,
                                        cp_x, cp_y, cp_w, cp_h, time_format_12h, &server, None, None,
                                    );
                                    continue;
                                }
                            }
                        }
                    }

                    // Double click desktop icons to open / restore applications
                    for (i, app) in apps.iter_mut().enumerate() {
                        let icon_y = (26 + i * 90) as i32;
                        if px >= 16 && px < 96 && py >= icon_y - 10 && py < icon_y + 70 {
                            app.open = true;
                            app.minimized = false;
                            z_order.retain(|&w| w != WindowId::App(i));
                            z_order.push(WindowId::App(i));
                            active_window = Some(WindowId::App(i));
                            selected_icon = None;
                            break;
                        }
                    }

                    let cp_icon_y = (26 + apps.len() * 90) as i32;
                    if px >= 16 && px < 100 && py >= cp_icon_y - 10 && py < cp_icon_y + 70 {
                        cp_open = true;
                        cp_minimized = false;
                        z_order.retain(|&w| w != WindowId::ControlPanel);
                        z_order.push(WindowId::ControlPanel);
                        active_window = Some(WindowId::ControlPanel);
                        selected_icon = None;
                    }

                    redraw_desktop(
                        desktop_w, desktop_h, theme, &apps, selected_icon, active_window,
                        &z_order, cp_open, cp_minimized, cp_system_menu_open,
                        cp_x, cp_y, cp_w, cp_h, time_format_12h, &server, None, None,
                    );
                }

                cb_win32::WM_MOUSEMOVE => {
                    let mut px = msg.pt_x;
                    let py = msg.pt_y;
                    // Coalesce mouse moves
                    while let Some(next_msg) = queue.peek_message(0, cb_win32::WM_MOUSEMOVE, cb_win32::WM_MOUSEMOVE, false) {
                        if next_msg.message == cb_win32::WM_MOUSEMOVE {
                            let removed = queue.peek_message(0, cb_win32::WM_MOUSEMOVE, cb_win32::WM_MOUSEMOVE, true).unwrap();
                            px = removed.pt_x;
                        } else {
                            break;
                        }
                    }

                    if let Some(drag_win) = dragging_window {
                        let (curr_w, curr_h) = match drag_win {
                            WindowId::App(ai) => (apps[ai].win_w, apps[ai].win_h),
                            WindowId::ControlPanel => (cp_w, cp_h),
                        };
                        let new_x = (px - drag_offset.0).clamp(-((curr_w as i32) - 40), desktop_w as i32 - 40);
                        let new_y = (py - drag_offset.1).clamp(0, desktop_h as i32 - 30);
                        drag_curr_rect = Some((new_x, new_y, curr_w, curr_h));

                        let now_drag = std::time::Instant::now();
                        let moved_dist = (new_x - last_rendered_drag_pos.0).abs() + (new_y - last_rendered_drag_pos.1).abs();
                        if moved_dist >= 8 && now_drag.duration_since(last_drag_redraw).as_millis() >= 60 {
                            last_drag_redraw = now_drag;
                            last_rendered_drag_pos = (new_x, new_y);
                            let drag_param = Some((new_x, new_y, curr_w, curr_h, theme.client_offset().1));
                            redraw_desktop(
                                desktop_w, desktop_h, theme, &apps, selected_icon, active_window,
                                &z_order, cp_open, cp_minimized, cp_system_menu_open,
                                cp_x, cp_y, cp_w, cp_h, time_format_12h, &server, pressed_target, drag_param,
                            );
                        }
                    } else if let Some(PressedTarget::FormButton(ai, bi)) = pressed_target {
                        if let Some(app) = apps.get_mut(ai) {
                            if let Some(btn) = app.buttons.get_mut(bi) {
                                if btn.has_capture && btn.on_mouse_move(px, py) {
                                    redraw_desktop(
                                        desktop_w, desktop_h, theme, &apps, selected_icon, active_window,
                                        &z_order, cp_open, cp_minimized, cp_system_menu_open,
                                        cp_x, cp_y, cp_w, cp_h, time_format_12h, &server, pressed_target, None,
                                    );
                                }
                            }
                        }
                    }
                }

                cb_win32::WM_LBUTTONUP => {
                    let px = msg.pt_x;
                    let py = msg.pt_y;

                    if let Some(drag_win) = dragging_window.take() {
                        if let Some((nx, ny, _, _)) = drag_curr_rect.take() {
                            let (curr_w, curr_h) = match drag_win {
                                WindowId::App(ai) => (apps[ai].win_w, apps[ai].win_h),
                                WindowId::ControlPanel => (cp_w, cp_h),
                            };
                            let clamped_x = nx.clamp(0, desktop_w.saturating_sub(curr_w) as i32) as usize;
                            let clamped_y = ny.clamp(0, desktop_h.saturating_sub(curr_h) as i32) as usize;
                            match drag_win {
                                WindowId::App(ai) => {
                                    let d_x = clamped_x as i32 - apps[ai].win_x as i32;
                                    let d_y = clamped_y as i32 - apps[ai].win_y as i32;
                                    apps[ai].win_x = clamped_x;
                                    apps[ai].win_y = clamped_y;
                                    for btn in &mut apps[ai].buttons {
                                        btn.x += d_x;
                                        btn.y += d_y;
                                    }
                                    for tb in &mut apps[ai].textboxes {
                                        tb.x = (tb.x as i32 + d_x).max(0) as usize;
                                        tb.y = (tb.y as i32 + d_y).max(0) as usize;
                                    }
                                    for lbl in &mut apps[ai].labels {
                                        lbl.x = (lbl.x as i32 + d_x).max(0) as usize;
                                        lbl.y = (lbl.y as i32 + d_y).max(0) as usize;
                                    }
                                }
                                WindowId::ControlPanel => {
                                    cp_x = clamped_x;
                                    cp_y = clamped_y;
                                }
                            }
                        }
                        redraw_desktop(
                            desktop_w, desktop_h, theme, &apps, selected_icon, active_window,
                            &z_order, cp_open, cp_minimized, cp_system_menu_open,
                            cp_x, cp_y, cp_w, cp_h, time_format_12h, &server, None, None,
                        );
                    } else if let Some(target) = pressed_target.take() {
                        let inside = hit_test_target(target, px, py, &apps, cp_x, cp_y, cp_w, theme);
                        if inside {
                            match target {
                                PressedTarget::AppClose(ai) => {
                                    apps[ai].open = false;
                                    apps[ai].system_menu_open = false;
                                    active_window = z_order.iter().rev().find(|&&w| match w {
                                        WindowId::App(j) => apps[j].open && !apps[j].minimized,
                                        WindowId::ControlPanel => cp_open && !cp_minimized,
                                    }).copied();
                                }
                                PressedTarget::AppMinimize(ai) => {
                                    apps[ai].minimized = true;
                                    apps[ai].system_menu_open = false;
                                    active_window = z_order.iter().rev().find(|&&w| match w {
                                        WindowId::App(j) => apps[j].open && !apps[j].minimized,
                                        WindowId::ControlPanel => cp_open && !cp_minimized,
                                    }).copied();
                                }
                                PressedTarget::AppMaximize(ai) => {
                                    apps[ai].system_menu_open = false;
                                }
                                PressedTarget::AppSysMenu(ai) => {
                                    apps[ai].system_menu_open = !apps[ai].system_menu_open;
                                }
                                PressedTarget::CpClose => {
                                    cp_open = false;
                                    cp_system_menu_open = false;
                                    active_window = z_order.iter().rev().find(|&&w| match w {
                                        WindowId::App(j) => apps[j].open && !apps[j].minimized,
                                        WindowId::ControlPanel => cp_open && !cp_minimized,
                                    }).copied();
                                }
                                PressedTarget::CpMinimize => {
                                    cp_minimized = true;
                                    cp_system_menu_open = false;
                                    active_window = z_order.iter().rev().find(|&&w| match w {
                                        WindowId::App(j) => apps[j].open && !apps[j].minimized,
                                        WindowId::ControlPanel => cp_open && !cp_minimized,
                                    }).copied();
                                }
                                PressedTarget::CpMaximize => {
                                    cp_system_menu_open = false;
                                }
                                PressedTarget::CpSysMenu => {
                                    cp_system_menu_open = !cp_system_menu_open;
                                }
                                PressedTarget::CpFormat24 => {
                                    time_format_12h = false;
                                }
                                PressedTarget::CpFormat12 => {
                                    time_format_12h = true;
                                }
                                PressedTarget::CpTheme311 => {
                                    theme = cb_win32::WindowTheme::Windows311;
                                    for app in &mut apps {
                                        apply_theme_to_app(theme, desktop_w, desktop_h, app);
                                    }
                                    apply_theme_to_cp(theme, desktop_w, desktop_h, &mut cp_w, &mut cp_h, &mut cp_x, &mut cp_y);
                                }
                                PressedTarget::CpTheme98 => {
                                    theme = cb_win32::WindowTheme::Windows98;
                                    for app in &mut apps {
                                        apply_theme_to_app(theme, desktop_w, desktop_h, app);
                                    }
                                    apply_theme_to_cp(theme, desktop_w, desktop_h, &mut cp_w, &mut cp_h, &mut cp_x, &mut cp_y);
                                }
                                PressedTarget::CpThemeXp => {
                                    theme = cb_win32::WindowTheme::WindowsXp;
                                    for app in &mut apps {
                                        apply_theme_to_app(theme, desktop_w, desktop_h, app);
                                    }
                                    apply_theme_to_cp(theme, desktop_w, desktop_h, &mut cp_w, &mut cp_h, &mut cp_x, &mut cp_y);
                                }
                                PressedTarget::FormButton(ai, bi) => {
                                    if let Some(app) = apps.get_mut(ai) {
                                        if let Some(btn) = app.buttons.get_mut(bi) {
                                            let (_redraw, click_fired) = btn.on_lbutton_up(px, py);
                                            if click_fired {
                                                let _ = msvbvm60::thun_rt::dispatch_native_control_click(&btn.caption);
                                                if let Ok(rt_lock) = cb_win32::global_form_runtime().lock() {
                                                    if let Some(rt) = rt_lock.as_ref() {
                                                        for lbl in app.labels.iter_mut() {
                                                            if let Some(c) = rt.control_caption(&lbl.name) {
                                                                lbl.caption = c;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        redraw_desktop(
                            desktop_w, desktop_h, theme, &apps, selected_icon, active_window,
                            &z_order, cp_open, cp_minimized, cp_system_menu_open,
                            cp_x, cp_y, cp_w, cp_h, time_format_12h, &server, None, None,
                        );
                    }
                }

                cb_win32::WM_SIZE => {
                    let new_w = (msg.w_param as usize).clamp(640, 2560);
                    let new_h = (msg.l_param as usize).clamp(480, 1600);
                    if new_w != desktop_w || new_h != desktop_h {
                        desktop_w = new_w;
                        desktop_h = new_h;
                        for app in apps.iter_mut() {
                            app.win_x = app.win_x.min(desktop_w.saturating_sub(app.win_w));
                            app.win_y = app.win_y.min(desktop_h.saturating_sub(app.win_h));
                        }
                        cp_x = cp_x.min(desktop_w.saturating_sub(cp_w));
                        cp_y = cp_y.min(desktop_h.saturating_sub(cp_h));
                        redraw_desktop(
                            desktop_w, desktop_h, theme, &apps, selected_icon, active_window,
                            &z_order, cp_open, cp_minimized, cp_system_menu_open,
                            cp_x, cp_y, cp_w, cp_h, time_format_12h, &server, pressed_target, None,
                        );
                    }
                }

                cb_win32::WM_QUIT => break,
                _ => {}
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
    }
}
