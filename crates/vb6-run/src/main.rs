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
use msvbvm60::types::ProjectInfo;

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

fn set_px(buf: &mut [u8], px: usize, py: usize, b: u8, g: u8, r: u8) {
    if px < 800 && py < 600 {
        let idx = (py * 800 + px) * 4;
        buf[idx] = b;
        buf[idx + 1] = g;
        buf[idx + 2] = r;
        buf[idx + 3] = 0xFF;
    }
}

fn fill_box(buf: &mut [u8], rx: usize, ry: usize, rw: usize, rh: usize, b: u8, g: u8, r: u8) {
    for y in ry..(ry + rh).min(600) {
        for x in rx..(rx + rw).min(800) {
            let idx = (y * 800 + x) * 4;
            buf[idx] = b;
            buf[idx + 1] = g;
            buf[idx + 2] = r;
            buf[idx + 3] = 0xFF;
        }
    }
}

fn draw_rect_outline(buf: &mut [u8], rx: usize, ry: usize, rw: usize, rh: usize, b: u8, g: u8, r: u8) {
    for x in rx..(rx + rw).min(800) {
        set_px(buf, x, ry, b, g, r);
        if rh > 0 { set_px(buf, x, ry + rh - 1, b, g, r); }
    }
    for y in ry..(ry + rh).min(600) {
        set_px(buf, rx, y, b, g, r);
        if rw > 0 { set_px(buf, rx + rw - 1, y, b, g, r); }
    }
}

fn draw_bevel(buf: &mut [u8], rx: usize, ry: usize, rw: usize, rh: usize, raised: bool) {
    let (top_b, top_g, top_r, bot_b, bot_g, bot_r) = if raised {
        (0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00)
    } else {
        (0x80, 0x80, 0x80, 0xFF, 0xFF, 0xFF)
    };
    for x in rx..(rx + rw).min(800) {
        set_px(buf, x, ry, top_b, top_g, top_r);
        if rh > 0 { set_px(buf, x, ry + rh - 1, bot_b, bot_g, bot_r); }
    }
    for y in ry..(ry + rh).min(600) {
        set_px(buf, rx, y, top_b, top_g, top_r);
        if rw > 0 { set_px(buf, rx + rw - 1, y, bot_b, bot_g, bot_r); }
    }
}

fn draw_button(buf: &mut [u8], rx: usize, ry: usize, rw: usize, rh: usize) {
    fill_box(buf, rx, ry, rw, rh, 0xC8, 0xD0, 0xD4);
    for x in rx..(rx + rw) {
        set_px(buf, x, ry, 0xFF, 0xFF, 0xFF);
        if rh > 0 { set_px(buf, x, ry + rh - 1, 0x00, 0x00, 0x00); }
    }
    for y in ry..(ry + rh) {
        set_px(buf, rx, y, 0xFF, 0xFF, 0xFF);
        if rw > 0 { set_px(buf, rx + rw - 1, y, 0x00, 0x00, 0x00); }
    }
    if rw > 2 && rh > 2 {
        for x in (rx + 1)..(rx + rw - 1) {
            set_px(buf, x, ry + 1, 0xDF, 0xDF, 0xDF);
            set_px(buf, x, ry + rh - 2, 0x80, 0x80, 0x80);
        }
        for y in (ry + 1)..(ry + rh - 1) {
            set_px(buf, rx + 1, y, 0xDF, 0xDF, 0xDF);
            set_px(buf, rx + rw - 2, y, 0x80, 0x80, 0x80);
        }
    }
}

fn draw_x_glyph(buf: &mut [u8], gx: usize, gy: usize, b: u8, g: u8, r: u8) {
    for i in 0..8 {
        set_px(buf, gx + i, gy + i, b, g, r);
        set_px(buf, gx + i + 1, gy + i, b, g, r);
        set_px(buf, gx + 8 - i, gy + i, b, g, r);
        set_px(buf, gx + 7 - i, gy + i, b, g, r);
    }
}

fn draw_icon_16(buf: &mut [u8], ix: usize, iy: usize, ico: &[u8]) {
    for y in 0..16 {
        for x in 0..16 {
            let sidx = (y * 16 + x) * 4;
            if sidx + 3 < ico.len() && ico[sidx + 3] > 128 {
                set_px(buf, ix + x, iy + y, ico[sidx], ico[sidx + 1], ico[sidx + 2]);
            }
        }
    }
}

fn draw_icon_32(buf: &mut [u8], ix: usize, iy: usize, ico: &[u8]) {
    for y in 0..32 {
        for x in 0..32 {
            let sidx = (y * 32 + x) * 4;
            if sidx + 3 < ico.len() && ico[sidx + 3] > 128 {
                set_px(buf, ix + x, iy + y, ico[sidx], ico[sidx + 1], ico[sidx + 2]);
            }
        }
    }
}

fn draw_radio_button(buf: &mut [u8], cx: usize, cy: usize, checked: bool) {
    for dy in 0..11 {
        for dx in 0..11 {
            let ddx = dx as i32 - 5;
            let ddy = dy as i32 - 5;
            let dist_sq = ddx * ddx + ddy * ddy;
            if dist_sq <= 25 {
                set_px(buf, cx + dx - 5, cy + dy - 5, 0xFF, 0xFF, 0xFF);
            }
            if dist_sq >= 16 && dist_sq <= 25 {
                if ddx + ddy <= 0 {
                    set_px(buf, cx + dx - 5, cy + dy - 5, 0x80, 0x80, 0x80);
                } else {
                    set_px(buf, cx + dx - 5, cy + dy - 5, 0xC0, 0xC0, 0xC0);
                }
            }
            if checked && dist_sq <= 6 {
                set_px(buf, cx + dx - 5, cy + dy - 5, 0x00, 0x00, 0x00);
            }
        }
    }
}

fn draw_toggle_button(buf: &mut [u8], bx: usize, by: usize, bw: usize, bh: usize, sunken: bool) {
    if sunken {
        fill_box(buf, bx, by, bw, bh, 0xDF, 0xDF, 0xDF);
        draw_bevel(buf, bx, by, bw, bh, false);
        draw_rect_outline(buf, bx + 1, by + 1, bw - 2, bh - 2, 0x00, 0x00, 0x00);
    } else {
        fill_box(buf, bx, by, bw, bh, 0xC8, 0xD0, 0xD4);
        draw_bevel(buf, bx, by, bw, bh, true);
    }
}

fn draw_themed_window(
    buf: &mut [u8],
    wx: usize,
    wy: usize,
    ww: usize,
    wh: usize,
    theme: cb_win32::WindowTheme,
    icon_16: Option<&[u8]>,
) {
    let (bg_b, bg_g, bg_r) = theme.client_bg_color();
    match theme {
        cb_win32::WindowTheme::Windows311 => {
            fill_box(buf, wx, wy, ww, wh, bg_b, bg_g, bg_r);
            draw_rect_outline(buf, wx, wy, ww, wh, 0, 0, 0);
            draw_bevel(buf, wx + 1, wy + 1, ww - 2, wh - 2, true);

            // Title bar: solid dark blue (#0000A0)
            fill_box(buf, wx + 4, wy + 4, ww - 8, 20, 0xA0, 0x00, 0x00);

            // Left System Menu Box (18x18 at wx + 4, wy + 4)
            fill_box(buf, wx + 4, wy + 4, 18, 18, 0xFF, 0xFF, 0xFF);
            draw_rect_outline(buf, wx + 4, wy + 4, 18, 18, 0, 0, 0);
            draw_bevel(buf, wx + 5, wy + 5, 16, 16, true);
            fill_box(buf, wx + 7, wy + 12, 12, 3, 0, 0, 0);

            // Right Minimize button ▼ (18x18 at wx + ww - 42, wy + 4)
            fill_box(buf, wx + ww - 42, wy + 4, 18, 18, 0xC0, 0xC0, 0xC0);
            draw_rect_outline(buf, wx + ww - 42, wy + 4, 18, 18, 0, 0, 0);
            draw_bevel(buf, wx + ww - 41, wy + 5, 16, 16, true);
            for (row, width) in [7, 5, 3, 1].iter().enumerate() {
                let start_x = wx + ww - 33 - width / 2;
                fill_box(buf, start_x, wy + 11 + row, *width, 1, 0, 0, 0);
            }

            // Right Maximize button ▲ (18x18 at wx + ww - 22, wy + 4)
            fill_box(buf, wx + ww - 22, wy + 4, 18, 18, 0xC0, 0xC0, 0xC0);
            draw_rect_outline(buf, wx + ww - 22, wy + 4, 18, 18, 0, 0, 0);
            draw_bevel(buf, wx + ww - 21, wy + 5, 16, 16, true);
            for (row, width) in [1, 3, 5, 7].iter().enumerate() {
                let start_x = wx + ww - 13 - width / 2;
                fill_box(buf, start_x, wy + 11 + row, *width, 1, 0, 0, 0);
            }

            // Inner sunken client border
            draw_bevel(buf, wx + 4, wy + 25, ww - 8, wh - 29, false);
        }
        cb_win32::WindowTheme::Windows98 => {
            fill_box(buf, wx, wy, ww, wh, bg_b, bg_g, bg_r);
            draw_bevel(buf, wx, wy, ww, wh, true);
            draw_bevel(buf, wx + 1, wy + 1, ww - 2, wh - 2, true);

            // Title bar: Navy gradient (#000080 -> #1084D0)
            let tb_w = ww.saturating_sub(8);
            for x in 0..tb_w {
                let fraction = x as f32 / tb_w.max(1) as f32;
                let b = (0x80 as f32 + fraction * (0xD0 as f32 - 0x80 as f32)) as u8;
                let g = (fraction * 0x84 as f32) as u8;
                let r = (fraction * 0x10 as f32) as u8;
                for y in 0..22 {
                    set_px(buf, wx + 4 + x, wy + 4 + y, b, g, r);
                }
            }

            if let Some(ico) = icon_16 {
                draw_icon_16(buf, wx + 6, wy + 7, ico);
            }

            // Minimize button _ (16x16 at wx + ww - 42, wy + 6)
            draw_button(buf, wx + ww - 42, wy + 6, 16, 16);
            fill_box(buf, wx + ww - 38, wy + 18, 8, 2, 0, 0, 0);

            // Close button X (18x18 at wx + ww - 24, wy + 6)
            draw_button(buf, wx + ww - 24, wy + 6, 18, 18);
            draw_x_glyph(buf, wx + ww - 20, wy + 10, 0, 0, 0);
        }
        cb_win32::WindowTheme::WindowsXp => {
            // Luna steel blue 3px border (#0045B8)
            fill_box(buf, wx, wy, ww, wh, 0xB8, 0x45, 0x00);
            if ww > 6 && wh > 32 {
                fill_box(buf, wx + 3, wy + 29, ww - 6, wh - 32, bg_b, bg_g, bg_r);
            }

            // Title bar: Luna royal blue gradient with glossy top
            let tb_w = ww.saturating_sub(6);
            for x in 0..tb_w {
                set_px(buf, wx + 3 + x, wy + 3, 0xF9, 0x99, 0x3E);
                for y in 1..26 {
                    let fraction = y as f32 / 26.0;
                    let b = (0xEA as f32 + fraction * (0xD7 as f32 - 0xEA as f32)) as u8;
                    let g = (0x55 as f32 + fraction * (0x65 as f32 - 0x55 as f32)) as u8;
                    let r = (0x00 as f32 + fraction * (0x1E as f32 - 0x00 as f32)) as u8;
                    set_px(buf, wx + 3 + x, wy + 3 + y, b, g, r);
                }
            }

            // Rounded corners for title bar (2px clip)
            set_px(buf, wx, wy, 0x80, 0x80, 0x00);
            set_px(buf, wx + 1, wy, 0x80, 0x80, 0x00);
            set_px(buf, wx, wy + 1, 0x80, 0x80, 0x00);
            set_px(buf, wx + ww - 1, wy, 0x80, 0x80, 0x00);
            set_px(buf, wx + ww - 2, wy, 0x80, 0x80, 0x00);
            set_px(buf, wx + ww - 1, wy + 1, 0x80, 0x80, 0x00);

            if let Some(ico) = icon_16 {
                draw_icon_16(buf, wx + 7, wy + 8, ico);
            }

            // XP Minimize button _ (18x18 blue button at wx + ww - 48, wy + 6)
            fill_box(buf, wx + ww - 48, wy + 6, 18, 18, 0xE8, 0x6F, 0x28);
            draw_rect_outline(buf, wx + ww - 48, wy + 6, 18, 18, 0xFF, 0xFF, 0xFF);
            fill_box(buf, wx + ww - 44, wy + 18, 8, 2, 0xFF, 0xFF, 0xFF);

            // XP Close button X (21x20 vibrant red button at wx + ww - 26, wy + 5)
            fill_box(buf, wx + ww - 26, wy + 5, 21, 20, 0x28, 0x44, 0xD8);
            draw_rect_outline(buf, wx + ww - 26, wy + 5, 21, 20, 0x50, 0x65, 0xE8);
            draw_x_glyph(buf, wx + ww - 20, wy + 10, 0xFF, 0xFF, 0xFF);
        }
    }
}

fn build_desktop_frames(
    theme: cb_win32::WindowTheme,
    app_open: bool,
    app_minimized: bool,
    app_win_x: usize,
    app_win_y: usize,
    app_win_w: usize,
    app_win_h: usize,
    form_caption: &str,
    app_icon_16: Option<&[u8]>,
    app_icon_32: Option<&[u8]>,
    buttons: &[cb_win32::Win32ButtonControl],
    textboxes: &[DynamicTextBox],
    labels: &[DynamicLabel],
    selected_icon: Option<u32>,
    cp_open: bool,
    cp_minimized: bool,
    cp_x: usize,
    cp_y: usize,
    cp_w: usize,
    cp_h: usize,
    cp_icon_16: &[u8],
    cp_icon_32: &[u8],
    time_format_12h: bool,
) -> (Vec<u8>, Vec<cb_webserver::DisplayFrame>) {
    let mut pixels = vec![0u8; 800 * 600 * 4];
    // 1. Clear desktop to Teal (#008080: B=0x80, G=0x80, R=0x00)
    for chunk in pixels.as_chunks_mut::<4>().0 {
        chunk[0] = 0x80;
        chunk[1] = 0x80;
        chunk[2] = 0x00;
        chunk[3] = 0xFF;
    }

    let mut text_frames = Vec::new();

    // 2. Desktop Icons:
    // Icon 1: Running App
    if let Some(ico32) = app_icon_32 {
        draw_icon_32(&mut pixels, 40, 26, ico32);
    }
    // Icon 1 Label
    if selected_icon == Some(1) {
        fill_box(&mut pixels, 22, 62, 68, 18, 0x80, 0x00, 0x00);
        draw_rect_outline(&mut pixels, 22, 62, 68, 18, 0xFF, 0xFF, 0xFF);
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 26,
            y: 75,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -12,
            text: form_caption.to_string(),
        }));
    } else {
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 27,
            y: 76,
            color_rgb: 0x00000000,
            bk_mode: 1,
            font_height: -12,
            text: form_caption.to_string(),
        }));
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 26,
            y: 75,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -12,
            text: form_caption.to_string(),
        }));
    }

    // Icon 2: Control Panel
    draw_icon_32(&mut pixels, 40, 116, cp_icon_32);
    // Icon 2 Label ("Painel de Controle")
    if selected_icon == Some(2) {
        fill_box(&mut pixels, 16, 152, 80, 32, 0x80, 0x00, 0x00);
        draw_rect_outline(&mut pixels, 16, 152, 80, 32, 0xFF, 0xFF, 0xFF);
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 24,
            y: 165,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -11,
            text: "Painel de".to_string(),
        }));
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 28,
            y: 178,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -11,
            text: "Controle".to_string(),
        }));
    } else {
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 25,
            y: 166,
            color_rgb: 0x00000000,
            bk_mode: 1,
            font_height: -11,
            text: "Painel de".to_string(),
        }));
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 24,
            y: 165,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -11,
            text: "Painel de".to_string(),
        }));
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 29,
            y: 179,
            color_rgb: 0x00000000,
            bk_mode: 1,
            font_height: -11,
            text: "Controle".to_string(),
        }));
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 28,
            y: 178,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -11,
            text: "Controle".to_string(),
        }));
    }

    // 3. Render App Window if open and not minimized
    if app_open && !app_minimized {
        draw_themed_window(&mut pixels, app_win_x, app_win_y, app_win_w, app_win_h, theme, app_icon_16);

        let title_x = if theme == cb_win32::WindowTheme::Windows311 {
            (app_win_x + 26) as i16
        } else if app_icon_16.is_some() {
            (app_win_x + 26) as i16
        } else {
            (app_win_x + 8) as i16
        };
        let title_y = (app_win_y + 20) as i16;
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 100,
            x: title_x,
            y: title_y,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -13,
            text: form_caption.to_string(),
        }));

        let (bg_b, bg_g, bg_r) = theme.client_bg_color();
        for btn in buttons {
            let b_pixels = btn.render_pixels();
            for by in 0..btn.height as usize {
                for bx in 0..btn.width as usize {
                    let sidx = (by * btn.width as usize + bx) * 4;
                    set_px(&mut pixels, btn.x as usize + bx, btn.y as usize + by, b_pixels[sidx], b_pixels[sidx + 1], b_pixels[sidx + 2]);
                }
            }
            let tx = (btn.x + 8) as i16;
            let ty = (btn.y + (btn.height as i32 * 2 / 3).max(14)) as i16;
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

        for tb in textboxes {
            let (b, g, r) = (tb.back_color as u8, (tb.back_color >> 8) as u8, (tb.back_color >> 16) as u8);
            fill_box(&mut pixels, tb.x, tb.y, tb.w, tb.h, b, g, r);
            draw_bevel(&mut pixels, tb.x, tb.y, tb.w, tb.h, false);
            text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                hwnd: tb.hwnd,
                x: (tb.x + 4) as i16,
                y: (tb.y + (tb.h * 2 / 3).max(14)) as i16,
                color_rgb: tb.fore_color,
                bk_mode: 1,
                font_height: -((tb.h * 2 / 3).clamp(12, 22) as i16),
                text: tb.text.clone(),
            }));
        }

        for lbl in labels {
            fill_box(&mut pixels, lbl.x, lbl.y, lbl.w, lbl.h, bg_b, bg_g, bg_r);
            let display_caption = cb_win32::format_time_label(&lbl.caption, time_format_12h);
            text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                hwnd: lbl.hwnd,
                x: (lbl.x + 4) as i16,
                y: (lbl.y + (lbl.h * 2 / 3).max(14)) as i16,
                color_rgb: lbl.fore_color,
                bk_mode: 1,
                font_height: -((lbl.h * 2 / 3).clamp(12, 22) as i16),
                text: display_caption,
            }));
        }
    }

    // 4. Render Control Panel Window if open and not minimized
    if cp_open && !cp_minimized {
        draw_themed_window(&mut pixels, cp_x, cp_y, cp_w, cp_h, theme, Some(cp_icon_16));

        let cp_title_x = if theme == cb_win32::WindowTheme::Windows311 {
            (cp_x + 26) as i16
        } else {
            (cp_x + 28) as i16
        };
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 500,
            x: cp_title_x,
            y: (cp_y + 20) as i16,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -13,
            text: "Painel de Controle".to_string(),
        }));

        // --- Group 1: Formato de Data e Hora ---
        draw_bevel(&mut pixels, cp_x + 14, cp_y + 36, cp_w - 28, 84, false);
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 501,
            x: (cp_x + 24) as i16,
            y: (cp_y + 36) as i16,
            color_rgb: 0x00000000,
            bk_mode: 0,
            font_height: -12,
            text: " Formato de Data e Hora ".to_string(),
        }));

        draw_radio_button(&mut pixels, cp_x + 30, cp_y + 60, !time_format_12h);
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 502,
            x: (cp_x + 44) as i16,
            y: (cp_y + 64) as i16,
            color_rgb: 0x00000000,
            bk_mode: 1,
            font_height: -12,
            text: "Formato 24 Horas (ex: 17:30:00)".to_string(),
        }));

        draw_radio_button(&mut pixels, cp_x + 30, cp_y + 88, time_format_12h);
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 503,
            x: (cp_x + 44) as i16,
            y: (cp_y + 92) as i16,
            color_rgb: 0x00000000,
            bk_mode: 1,
            font_height: -12,
            text: "Formato 12 Horas (AM/PM)".to_string(),
        }));

        // --- Group 2: Tema das Janelas ---
        draw_bevel(&mut pixels, cp_x + 14, cp_y + 130, cp_w - 28, 126, false);
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 504,
            x: (cp_x + 24) as i16,
            y: (cp_y + 130) as i16,
            color_rgb: 0x00000000,
            bk_mode: 0,
            font_height: -12,
            text: " Tema Visual do Ambiente ".to_string(),
        }));

        let is_w311 = theme == cb_win32::WindowTheme::Windows311;
        draw_toggle_button(&mut pixels, cp_x + 22, cp_y + 152, 94, 30, is_w311);
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 505,
            x: (cp_x + 28) as i16,
            y: (cp_y + 172) as i16,
            color_rgb: 0x00000000,
            bk_mode: 1,
            font_height: -12,
            text: if is_w311 { "• Win 3.11 •".to_string() } else { "Windows 3.11".to_string() },
        }));

        let is_w98 = theme == cb_win32::WindowTheme::Windows98;
        draw_toggle_button(&mut pixels, cp_x + 126, cp_y + 152, 94, 30, is_w98);
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 506,
            x: (cp_x + 136) as i16,
            y: (cp_y + 172) as i16,
            color_rgb: 0x00000000,
            bk_mode: 1,
            font_height: -12,
            text: if is_w98 { "• Win 98 •".to_string() } else { "Windows 98".to_string() },
        }));

        let is_wxp = theme == cb_win32::WindowTheme::WindowsXp;
        draw_toggle_button(&mut pixels, cp_x + 230, cp_y + 152, 94, 30, is_wxp);
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 507,
            x: (cp_x + 242) as i16,
            y: (cp_y + 172) as i16,
            color_rgb: 0x00000000,
            bk_mode: 1,
            font_height: -12,
            text: if is_wxp { "• Win XP •".to_string() } else { "Windows XP".to_string() },
        }));

        let desc = match theme {
            cb_win32::WindowTheme::Windows311 => "Estilo retrô: Windows 3.11 / Win16 (1993)",
            cb_win32::WindowTheme::Windows98 => "Estilo clássico: Windows 98 / Win32 (1998)",
            cb_win32::WindowTheme::WindowsXp => "Estilo moderno: Windows XP Luna Azul (2001)",
        };
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 508,
            x: (cp_x + 22) as i16,
            y: (cp_y + 208) as i16,
            color_rgb: 0x00000080,
            bk_mode: 1,
            font_height: -11,
            text: desc.to_string(),
        }));
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 509,
            x: (cp_x + 22) as i16,
            y: (cp_y + 228) as i16,
            color_rgb: 0x00555555,
            bk_mode: 1,
            font_height: -11,
            text: "Clique nos botões para mudar o tema das janelas.".to_string(),
        }));
    }

    (pixels, text_frames)
}

fn is_close_click(wx: usize, wy: usize, ww: usize, theme: cb_win32::WindowTheme, px: i32, py: i32) -> bool {
    if px < 0 || py < 0 {
        return false;
    }
    let px = px as usize;
    let py = py as usize;
    match theme {
        cb_win32::WindowTheme::Windows311 => {
            (px >= wx + 4 && px < wx + 22 && py >= wy + 4 && py < wy + 22)
                || (px >= wx + ww.saturating_sub(22) && px < wx + ww.saturating_sub(4) && py >= wy + 4 && py < wy + 22)
        }
        cb_win32::WindowTheme::Windows98 => {
            px >= wx + ww.saturating_sub(24) && px < wx + ww.saturating_sub(6) && py >= wy + 6 && py < wy + 24
        }
        cb_win32::WindowTheme::WindowsXp => {
            px >= wx + ww.saturating_sub(26) && px < wx + ww.saturating_sub(5) && py >= wy + 5 && py < wy + 25
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
            px >= wx + ww.saturating_sub(42) && px < wx + ww.saturating_sub(24) && py >= wy + 4 && py < wy + 22
        }
        cb_win32::WindowTheme::Windows98 => {
            px >= wx + ww.saturating_sub(42) && px < wx + ww.saturating_sub(26) && py >= wy + 6 && py < wy + 22
        }
        cb_win32::WindowTheme::WindowsXp => {
            px >= wx + ww.saturating_sub(48) && px < wx + ww.saturating_sub(30) && py >= wy + 6 && py < wy + 24
        }
    }
}

/// CLI entry point for loading and running Visual Basic 6 executables on Linux/macOS.
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_banner();
        print_usage(&args[0]);
        process::exit(1);
    }

    let mut exe_path: Option<&str> = None;
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
            // Ignore bare argument separator or empty token
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
            exe_path = Some(arg);
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

    let file_path = match exe_path {
        Some(p) => p,
        None => {
            eprintln!("Error: No executable file path specified.");
            print_usage(&args[0]);
            process::exit(1);
        }
    };

    if !Path::new(file_path).exists() {
        eprintln!("Error: File not found: '{}'", file_path);
        process::exit(1);
    }

    if !quiet {
        print_banner();
        println!("[*] Reading binary: {}", file_path);
    }

    let data = match fs::read(file_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        }
    };

    if !quiet {
        println!(
            "[*] Parsing PE32 and mapping sections ({} bytes)...",
            data.len()
        );
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
                println!(
                    "    - Build Version: 0x{:04X}, Revision: 0x{:04X}",
                    build, rev
                );
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
                let proj_info = unsafe { &*(vb_hdr.lp_project_data as *const ProjectInfo) };
                if is_valid_ptr(proj_info.lp_project_name) {
                    let name_cstr =
                        unsafe { std::ffi::CStr::from_ptr(proj_info.lp_project_name as *const i8) };
                    if !quiet {
                        println!("    - Project Name: {}", name_cstr.to_string_lossy());
                    }
                }
                if is_valid_ptr(proj_info.lp_project_title) {
                    let title_cstr = unsafe {
                        std::ffi::CStr::from_ptr(proj_info.lp_project_title as *const i8)
                    };
                    if !quiet {
                        println!("    - Project Title: {}", title_cstr.to_string_lossy());
                    }
                }
                if is_valid_ptr(proj_info.lp_project_description) {
                    let desc_cstr = unsafe {
                        std::ffi::CStr::from_ptr(proj_info.lp_project_description as *const i8)
                    };
                    if !quiet {
                        println!("    - Description: {}", desc_cstr.to_string_lossy());
                    }
                }
            }

            if dump_info {
                println!("\n--- [VBHeader Detailed Dump] ---");
                println!(
                    "  Magic: {:?}",
                    std::str::from_utf8(&vb_hdr.sz_magic).unwrap_or("????")
                );
                println!(
                    "  Language DLL: {:?}",
                    std::str::from_utf8(&vb_hdr.sz_language_dll).unwrap_or("")
                );
                println!("  LCID: 0x{:08X}", { vb_hdr.lcid });
                println!("  Thread Flags: 0x{:08X}", { vb_hdr.dw_thread_flags });
                println!("  GUI Table: 0x{:08X}", { vb_hdr.lp_gui_table });
                println!("  Import Table: 0x{:08X}", { vb_hdr.lp_import_table });
                println!("  Export Table: 0x{:08X}", { vb_hdr.lp_export_table });

                if vb_hdr.lp_project_data != 0 {
                    let proj_info = unsafe { &*(vb_hdr.lp_project_data as *const ProjectInfo) };
                    println!("\n--- [ProjectInfo Dump] ---");
                    println!("  Version: 0x{:08X}", { proj_info.dw_version });
                    println!("  ObjectTable Pointer: 0x{:08X}", {
                        proj_info.lp_object_table
                    });
                    println!("  Code Depot: 0x{:08X}", { proj_info.lp_code_depot });

                    if proj_info.lp_object_table != 0 {
                        let obj_tbl = unsafe {
                            &*(proj_info.lp_object_table as *const msvbvm60::types::ObjectTable)
                        };
                        println!("\n--- [ObjectTable Dump] ---");
                        println!("  Total Objects: {}", { obj_tbl.w_total_objects_count });
                        println!("  Compiled Objects: {}", {
                            obj_tbl.w_compiled_objects_count
                        });
                        println!("  Objects in Use: {}", { obj_tbl.w_objects_in_use_count });
                    }
                }
            }

            if inspect_only || dump_info {
                println!("\n[+] Inspection finished successfully.");
                return;
            }

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
                        Ok(f) => {
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
                            Some(f)
                        }
                        Err(e) => {
                            if !quiet {
                                println!("[-] Note: PE GUI table deserializer: {}", e);
                            }
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let mut form = form_desc.unwrap_or_else(|| {
                cb_win32::FormDescriptor {
                    name: "Form1".to_string(),
                    caption: "ClassicBasic".to_string(),
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

            // If the form does not have an icon embedded in its stream, check the PE .rsrc section
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

            cb_win32::ensure_default_form_runtime();
            {
                let mut lock = cb_win32::global_form_runtime().lock().unwrap();
                *lock = Some(cb_win32::FormRuntime::new(form.clone()));
            }

            // Extract layout coordinates centered on 800x600 screen
            let client_w = (form.pixel_client_width() as usize).max(60);
            let client_h = (form.pixel_client_height() as usize).max(30);
            let win_w = (client_w + 8).clamp(120, 1600);
            let win_h = (client_h + 30).clamp(60, 1200);
            let win_x = (800usize.saturating_sub(win_w)) / 2;
            let win_y = (600usize.saturating_sub(win_h)) / 2;

            let mut buttons: Vec<cb_win32::Win32ButtonControl> = form
                .controls
                .iter()
                .enumerate()
                .filter(|(_, c)| c.control_type == cb_win32::ControlType::CommandButton)
                .map(|(idx, c)| {
                    let abs_x = (win_x as i32 + 4 + c.left / 15).max(0);
                    let abs_y = ((win_y + 26) as i32 + c.top / 15).max(0);
                    let abs_w = (c.width / 15).max(1) as u32;
                    let abs_h = (c.height / 15).max(1) as u32;
                    cb_win32::Win32ButtonControl::new(
                        (200 + idx) as u32,
                        abs_x,
                        abs_y,
                        abs_w,
                        abs_h,
                        &c.caption,
                    )
                })
                .collect();

            let mut textboxes: Vec<DynamicTextBox> = form
                .controls
                .iter()
                .enumerate()
                .filter(|(_, c)| c.control_type == cb_win32::ControlType::TextBox)
                .map(|(idx, c)| {
                    let abs_x = (win_x as i32 + 4 + c.left / 15).max(0) as usize;
                    let abs_y = ((win_y + 26) as i32 + c.top / 15).max(0) as usize;
                    let abs_w = (c.width / 15).max(1) as usize;
                    let abs_h = (c.height / 15).max(1) as usize;
                    DynamicTextBox {
                        hwnd: (200 + idx) as u32,
                        x: abs_x,
                        y: abs_y,
                        w: abs_w,
                        h: abs_h,
                        text: if !c.text.is_empty() { c.text.clone() } else { "0".to_string() },
                        back_color: c.back_color.unwrap_or(0x00FFFFFF),
                        fore_color: c.fore_color.unwrap_or(0x00000000),
                    }
                })
                .collect();

            let mut labels: Vec<DynamicLabel> = form
                .controls
                .iter()
                .enumerate()
                .filter(|(_, c)| c.control_type == cb_win32::ControlType::Label)
                .map(|(idx, c)| {
                    let abs_x = (win_x as i32 + 4 + c.left / 15).max(0) as usize;
                    let abs_y = ((win_y + 26) as i32 + c.top / 15).max(0) as usize;
                    let abs_w = (c.width / 15).max(1) as usize;
                    let abs_h = (c.height / 15).max(1) as usize;
                    DynamicLabel {
                        hwnd: (200 + idx) as u32,
                        name: c.name.clone(),
                        x: abs_x,
                        y: abs_y,
                        w: abs_w,
                        h: abs_h,
                        caption: c.caption.clone(),
                        fore_color: c.fore_color.unwrap_or(0x00000000),
                    }
                })
                .collect();

            let mut timers: Vec<ActiveTimer> = form
                .controls
                .iter()
                .filter(|c| c.control_type == cb_win32::ControlType::Timer)
                .map(|c| {
                    let interval = cb_win32::global_form_runtime()
                        .lock()
                        .unwrap()
                        .as_ref()
                        .and_then(|rt| rt.control_interval(&c.name))
                        .unwrap_or(1000)
                        .max(50) as u64;
                    ActiveTimer {
                        name: c.name.clone(),
                        interval_ms: interval,
                        last_tick: std::time::Instant::now(),
                    }
                })
                .collect();

            let (app_icon_16, app_icon_32) = if let Some((iw, ih, ipx)) = form.decode_icon() {
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

            let cp_icon_32 = cb_win32::generate_win311_control_panel_icon();
            let cp_icon_16 = cb_win32::downsample_32_to_16(&cp_icon_32);
            let mut theme = cb_win32::WindowTheme::Windows98;
            let mut time_format_12h = false;
            let mut app_open = true;
            let mut app_minimized = false;
            let mut cp_open = false;
            let mut cp_minimized = false;
            let mut selected_icon: Option<u32> = None;
            let cp_x = 220usize;
            let cp_y = 120usize;
            let cp_w = 360usize;
            let cp_h = 280usize;

            let (server_handle, queue) = if execution_mode == ExecutionMode::GuiWebStream {
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

                let mut web_controls = Vec::new();
                for (idx, ctrl) in form.controls.iter().enumerate() {
                    let hwnd = (200 + idx) as u32;
                    let kind = match ctrl.control_type {
                        cb_win32::ControlType::CommandButton => 1,
                        cb_win32::ControlType::TextBox => 2,
                        cb_win32::ControlType::Label => 3,
                        _ => 0,
                    };
                    let abs_x = (win_x as i16 + 4) + (ctrl.left / 15) as i16;
                    let abs_y = (win_y as i16 + 26) + (ctrl.top / 15) as i16;
                    let w = (ctrl.width / 15).max(1) as u16;
                    let h = (ctrl.height / 15).max(1) as u16;
                    let text = if !ctrl.caption.is_empty() {
                        ctrl.caption.clone()
                    } else if !ctrl.text.is_empty() {
                        ctrl.text.clone()
                    } else if kind == 2 {
                        "0".to_string()
                    } else {
                        String::new()
                    };
                    web_controls.push(cb_webserver::FormControl {
                        hwnd,
                        kind,
                        x: abs_x,
                        y: abs_y,
                        width: w,
                        height: h,
                        text,
                    });
                }

                let (init_pixels, init_text_frames) = build_desktop_frames(
                    theme,
                    app_open,
                    app_minimized,
                    win_x,
                    win_y,
                    win_w,
                    win_h,
                    &form.caption,
                    app_icon_16.as_deref(),
                    app_icon_32.as_deref(),
                    &buttons,
                    &textboxes,
                    &labels,
                    selected_icon,
                    cp_open,
                    cp_minimized,
                    cp_x,
                    cp_y,
                    cp_w,
                    cp_h,
                    &cp_icon_16,
                    &cp_icon_32,
                    time_format_12h,
                );

                let mut initial_frames = Vec::new();
                initial_frames.push(cb_webserver::DisplayFrame::Form(cb_webserver::FormSnapshotFrame {
                    hwnd: 100,
                    x: win_x as i16,
                    y: win_y as i16,
                    width: win_w as u16,
                    height: win_h as u16,
                    style: 0x00CF0000,
                    caption: form.caption.clone(),
                    controls: web_controls,
                }));
                initial_frames.push(cb_webserver::DisplayFrame::BitmapRect(cb_webserver::BitmapRectFrame {
                    hwnd: 100,
                    x: 0,
                    y: 0,
                    width: 800,
                    height: 600,
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
                    Ok(h) => {
                        if !quiet {
                            println!("\n╔═══════════════════════════════════════════════════════════════╗");
                            println!("║   ClassicBasic GUI Web Server Running on http://0.0.0.0:{:<5} ║", port);
                            println!("║   Open http://localhost:{:<5} in your browser to interact    ║", port);
                            println!("╚═══════════════════════════════════════════════════════════════╝\n");
                        } else {
                            println!("[ClassicBasic GUI] Running on http://localhost:{} (Press Ctrl+C to terminate)", port);
                        }
                        Some(h)
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to start web server on port {}: {}", port, e);
                        None
                    }
                };
                (server, Some(queue))
            } else {
                (None, None)
            };

            if !quiet {
                println!("\n[*] Starting execution via ThunRTMain...");
            }
            let exit_code = match loaded_pe.execute_with_mode(execution_mode) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("Execution error: {}", e);
                    process::exit(1);
                }
            };

            if let (Some(server), Some(queue)) = (server_handle, queue) {
                if !quiet {
                    println!("\n[+] Binary execution finished with return code: {}", exit_code);
                    println!("[*] Interactive Web GUI is running at http://0.0.0.0:{}", port);
                    println!("[*] Deserialized Form \"{}\" with {} controls.", form.caption, form.controls.len());
                    println!("[*] Open http://localhost:{} in your browser to interact.", port);
                    println!("[*] Press Ctrl+C to terminate session...\n");
                }

                let redraw_desktop = |theme: cb_win32::WindowTheme,
                                      app_open: bool,
                                      app_minimized: bool,
                                      selected_icon: Option<u32>,
                                      cp_open: bool,
                                      cp_minimized: bool,
                                      time_format_12h: bool,
                                      server: &cb_webserver::ServerHandle,
                                      buttons: &[cb_win32::Win32ButtonControl],
                                      textboxes: &[DynamicTextBox],
                                      labels: &[DynamicLabel]| {
                    let (pixels, text_frames) = build_desktop_frames(
                        theme,
                        app_open,
                        app_minimized,
                        win_x,
                        win_y,
                        win_w,
                        win_h,
                        &form.caption,
                        app_icon_16.as_deref(),
                        app_icon_32.as_deref(),
                        buttons,
                        textboxes,
                        labels,
                        selected_icon,
                        cp_open,
                        cp_minimized,
                        cp_x,
                        cp_y,
                        cp_w,
                        cp_h,
                        &cp_icon_16,
                        &cp_icon_32,
                        time_format_12h,
                    );
                    server.broadcast_frame(&cb_webserver::DisplayFrame::BitmapRect(cb_webserver::BitmapRectFrame {
                        hwnd: 100,
                        x: 0,
                        y: 0,
                        width: 800,
                        height: 600,
                        pixels,
                    }));
                    for tf in &text_frames {
                        server.broadcast_frame(tf);
                    }
                };

                let mut dialog_active = false;
                let mut last_click_time: Option<std::time::Instant> = None;
                let mut last_click_pos: (i32, i32) = (0, 0);

                loop {
                    let now = std::time::Instant::now();
                    for tmr in timers.iter_mut() {
                        if now.duration_since(tmr.last_tick).as_millis() >= tmr.interval_ms as u128 {
                            tmr.last_tick = now;
                            let _ = msvbvm60::thun_rt::dispatch_native_control_click(&tmr.name);
                            if let Ok(rt_lock) = cb_win32::global_form_runtime().lock() {
                                if let Some(rt) = rt_lock.as_ref() {
                                    let mut changed = false;
                                    for lbl in labels.iter_mut() {
                                        if let Some(cap) = rt.control_caption(&lbl.name) {
                                            if cap != lbl.caption {
                                                lbl.caption = cap;
                                                changed = true;
                                            }
                                        }
                                    }
                                    if changed && app_open && !app_minimized {
                                        redraw_desktop(
                                            theme,
                                            app_open,
                                            app_minimized,
                                            selected_icon,
                                            cp_open,
                                            cp_minimized,
                                            time_format_12h,
                                            &server,
                                            &buttons,
                                            &textboxes,
                                            &labels,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    if let Some(msg) = queue.peek_message(0, 0, 0, true) {
                        match msg.message {
                            cb_win32::WM_LBUTTONDOWN | cb_win32::WM_LBUTTONDBLCLK => {
                                let px = msg.pt_x;
                                let py = msg.pt_y;
                                let now_click = std::time::Instant::now();
                                let is_double = if msg.message == cb_win32::WM_LBUTTONDBLCLK {
                                    true
                                } else if let Some(last_time) = last_click_time {
                                    let elapsed = now_click.duration_since(last_time).as_millis();
                                    let dx = (px - last_click_pos.0).abs();
                                    let dy = (py - last_click_pos.1).abs();
                                    elapsed <= 450 && dx <= 8 && dy <= 8
                                } else {
                                    false
                                };

                                if is_double {
                                    last_click_time = None;
                                    if px >= 16 && px < 96 && py >= 16 && py < 96 {
                                        // App Desktop Icon double clicked: unminimize / reopen
                                        app_open = true;
                                        app_minimized = false;
                                        selected_icon = Some(1);
                                        redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                        continue;
                                    } else if px >= 16 && px < 100 && py >= 106 && py < 192 {
                                        // Control Panel Desktop Icon double clicked: open / unminimize
                                        cp_open = true;
                                        cp_minimized = false;
                                        selected_icon = Some(2);
                                        redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                        continue;
                                    }
                                } else {
                                    last_click_time = Some(now_click);
                                    last_click_pos = (px, py);
                                }

                                if dialog_active {
                                    if (px >= 365 && px < 435 && py >= 315 && py < 340)
                                        || (px >= 514 && px < 532 && py >= 226 && py < 244)
                                    {
                                        dialog_active = false;
                                        redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                    }
                                } else if cp_open && !cp_minimized && px >= cp_x as i32 && px < (cp_x + cp_w) as i32 && py >= cp_y as i32 && py < (cp_y + cp_h) as i32 {
                                    // Control Panel Window clicked
                                    if is_close_click(cp_x, cp_y, cp_w, theme, px, py) {
                                        cp_open = false;
                                        redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                    } else if is_minimize_click(cp_x, cp_y, cp_w, theme, px, py) {
                                        cp_minimized = true;
                                        redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                    } else if px >= (cp_x + 24) as i32 && px < (cp_x + 320) as i32 && py >= (cp_y + 54) as i32 && py < (cp_y + 78) as i32 {
                                        // 24 Hours radio button clicked
                                        time_format_12h = false;
                                        msvbvm60::rtc::set_time_format_12h(false);
                                        redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                    } else if px >= (cp_x + 24) as i32 && px < (cp_x + 320) as i32 && py >= (cp_y + 82) as i32 && py < (cp_y + 108) as i32 {
                                        // 12 Hours AM/PM radio button clicked
                                        time_format_12h = true;
                                        msvbvm60::rtc::set_time_format_12h(true);
                                        redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                    } else if px >= (cp_x + 22) as i32 && px < (cp_x + 116) as i32 && py >= (cp_y + 152) as i32 && py < (cp_y + 182) as i32 {
                                        // Theme: Windows 3.11
                                        theme = cb_win32::WindowTheme::Windows311;
                                        redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                    } else if px >= (cp_x + 126) as i32 && px < (cp_x + 220) as i32 && py >= (cp_y + 152) as i32 && py < (cp_y + 182) as i32 {
                                        // Theme: Windows 98
                                        theme = cb_win32::WindowTheme::Windows98;
                                        redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                    } else if px >= (cp_x + 230) as i32 && px < (cp_x + 324) as i32 && py >= (cp_y + 152) as i32 && py < (cp_y + 182) as i32 {
                                        // Theme: Windows XP
                                        theme = cb_win32::WindowTheme::WindowsXp;
                                        redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                    }
                                } else if app_open && !app_minimized && px >= win_x as i32 && px < (win_x + win_w) as i32 && py >= win_y as i32 && py < (win_y + win_h) as i32 {
                                    // App Window clicked
                                    if is_close_click(win_x, win_y, win_w, theme, px, py) {
                                        app_open = false;
                                        redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                    } else if is_minimize_click(win_x, win_y, win_w, theme, px, py) {
                                        app_minimized = true;
                                        redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                    } else {
                                        for btn in buttons.iter_mut() {
                                            if btn.on_lbutton_down(px, py) {
                                                broadcast_button_state(&server, btn);
                                                break;
                                            }
                                        }
                                    }
                                } else if px >= 16 && px < 96 && py >= 16 && py < 96 {
                                    // App Desktop Icon clicked: select icon
                                    selected_icon = Some(1);
                                    redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                } else if px >= 16 && px < 100 && py >= 106 && py < 192 {
                                    // Control Panel Desktop Icon clicked: select icon
                                    selected_icon = Some(2);
                                    redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                } else {
                                    // Clicked empty desktop: clear selection
                                    if selected_icon.is_some() {
                                        selected_icon = None;
                                        redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                    }
                                }
                            }
                            cb_win32::WM_MOUSEMOVE => {
                                let px = msg.pt_x;
                                let py = msg.pt_y;
                                if !dialog_active && app_open && !app_minimized {
                                    for btn in buttons.iter_mut() {
                                        if btn.has_capture && btn.on_mouse_move(px, py) {
                                            broadcast_button_state(&server, btn);
                                            break;
                                        }
                                    }
                                }
                            }
                            cb_win32::WM_LBUTTONUP => {
                                let px = msg.pt_x;
                                let py = msg.pt_y;
                                if !dialog_active && app_open && !app_minimized {
                                    for btn in buttons.iter_mut() {
                                        if btn.has_capture {
                                            let (redraw, click_fired) = btn.on_lbutton_up(px, py);
                                            if redraw {
                                                broadcast_button_state(&server, btn);
                                            }
                                            if click_fired {
                                                let action = btn.caption.trim().to_string();
                                                println!("[msvbvm60] Control Click: \"{}\"", action);
                                                if !textboxes.is_empty() {
                                                    let ctrl_name = form
                                                        .controls
                                                        .iter()
                                                        .enumerate()
                                                        .find(|(i, _)| (200 + i) as u32 == btn.hwnd)
                                                        .map(|(_, c)| c.name.as_str())
                                                        .unwrap_or(&action);

                                                    match msvbvm60::thun_rt::dispatch_native_control_click(ctrl_name) {
                                                        Ok(new_text) => {
                                                            textboxes[0].text = new_text;
                                                            textboxes[0].render(&server);
                                                        }
                                                        Err(err) => {
                                                            println!("[msvbvm60] Native dispatch fallback: {}", err);
                                                            handle_calc_action(&action, &mut textboxes[0], &server);
                                                        }
                                                    }
                                                } else {
                                                    println!("\n╔════════════════════════════════════════════════════════════════╗");
                                                    println!("║ [MessageBox] {:<49} ║", form.caption);
                                                    println!("╠════════════════════════════════════════════════════════════════╣");
                                                    println!("║ Hello, world!                                                  ║");
                                                    println!("╚════════════════════════════════════════════════════════════════╝\n");
                                                    println!("[msvbvm60] rtcMsgBox called: prompt=\"Hello, world!\", title=\"{}\", buttons=0\n", form.caption);
                                                    dialog_active = true;
                                                    broadcast_msgbox_dialog(&server, &form.caption, "Hello, world!");
                                                }
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                            cb_win32::WM_CHAR => {
                                let ch = (msg.w_param as u8) as char;
                                if !textboxes.is_empty() {
                                    match ch {
                                        '0'..='9' | '+' | '-' | '*' | '/' => {
                                            match msvbvm60::thun_rt::dispatch_native_control_click(&ch.to_string()) {
                                                Ok(new_text) => {
                                                    textboxes[0].text = new_text;
                                                    textboxes[0].render(&server);
                                                }
                                                Err(_) => {
                                                    handle_calc_action(&ch.to_string(), &mut textboxes[0], &server);
                                                }
                                            }
                                        }
                                        '=' | '\r' | '\n' => {
                                            match msvbvm60::thun_rt::dispatch_native_control_click("=") {
                                                Ok(new_text) => {
                                                    textboxes[0].text = new_text;
                                                    textboxes[0].render(&server);
                                                }
                                                Err(_) => {
                                                    handle_calc_action("=", &mut textboxes[0], &server);
                                                }
                                            }
                                        }
                                        'c' | 'C' => {
                                            handle_calc_action("C", &mut textboxes[0], &server);
                                        }
                                        '.' | ',' => {
                                            handle_calc_action(".", &mut textboxes[0], &server);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            cb_win32::WM_KEYDOWN => {
                                if dialog_active {
                                    if msg.w_param == 13 || msg.w_param == 32 || msg.w_param == 27 {
                                        dialog_active = false;
                                        redraw_desktop(theme, app_open, app_minimized, selected_icon, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels);
                                    }
                                } else if !textboxes.is_empty() {
                                    if msg.w_param == 13 {
                                        match msvbvm60::thun_rt::dispatch_native_control_click("=") {
                                            Ok(new_text) => {
                                                textboxes[0].text = new_text;
                                                textboxes[0].render(&server);
                                            }
                                            Err(_) => {
                                                handle_calc_action("=", &mut textboxes[0], &server);
                                            }
                                        }
                                    } else if msg.w_param == 27 {
                                        handle_calc_action("C", &mut textboxes[0], &server);
                                    } else if msg.w_param == 8 {
                                        if textboxes[0].text.len() > 1 && textboxes[0].text != "Erro" {
                                            textboxes[0].text.pop();
                                            if textboxes[0].text.ends_with(' ') {
                                                textboxes[0].text.pop();
                                            }
                                        } else {
                                            textboxes[0].text = "0".to_string();
                                        }
                                        textboxes[0].render(&server);
                                    }
                                } else if !buttons.is_empty() {
                                    let (redraw, click_fired) = buttons[0].on_key_down(msg.w_param);
                                    if redraw {
                                        broadcast_button_state(&server, &buttons[0]);
                                    }
                                    if click_fired {
                                        println!("\n╔════════════════════════════════════════════════════════════════╗");
                                        println!("║ [MessageBox] {:<49} ║", form.caption);
                                        println!("╠════════════════════════════════════════════════════════════════╣");
                                        println!("║ Hello, world!                                                  ║");
                                        println!("╚════════════════════════════════════════════════════════════════╝\n");
                                        println!("[msvbvm60] rtcMsgBox called: prompt=\"Hello, world!\", title=\"{}\", buttons=0\n", form.caption);
                                        dialog_active = true;
                                        broadcast_msgbox_dialog(&server, &form.caption, "Hello, world!");
                                    }
                                }
                            }
                            cb_win32::WM_KEYUP => {
                                if !dialog_active && textboxes.is_empty() && !buttons.is_empty() {
                                    let (redraw, click_fired) = buttons[0].on_key_up(msg.w_param);
                                    if redraw {
                                        broadcast_button_state(&server, &buttons[0]);
                                    }
                                    if click_fired {
                                        println!("\n╔════════════════════════════════════════════════════════════════╗");
                                        println!("║ [MessageBox] {:<49} ║", form.caption);
                                        println!("╠════════════════════════════════════════════════════════════════╣");
                                        println!("║ Hello, world!                                                  ║");
                                        println!("╚════════════════════════════════════════════════════════════════╝\n");
                                        println!("[msvbvm60] rtcMsgBox called: prompt=\"Hello, world!\", title=\"{}\", buttons=0\n", form.caption);
                                        dialog_active = true;
                                        broadcast_msgbox_dialog(&server, &form.caption, "Hello, world!");
                                    }
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

            if !quiet {
                println!("\n[+] Execution completed with return code: {}", exit_code);
            }
            process::exit(exit_code);
        }
        None => {
            println!("[-] No 'VB5!' header signature found. (Is this a standard C/C++ PE?)");
            if inspect_only {
                println!("[+] PE inspection completed.");
                return;
            }
            eprintln!("Cannot execute: binary is not a recognized Visual Basic 6 executable.");
            process::exit(1);
        }
    }
}
