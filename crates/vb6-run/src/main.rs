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
        (0xFF, 0xFF, 0xFF, 0x80, 0x80, 0x80) // Soft shadow (#808080) for authentic Windows 95/98
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

fn draw_win98_button(buf: &mut [u8], rx: usize, ry: usize, rw: usize, rh: usize, pressed: bool) {
    fill_box(buf, rx, ry, rw, rh, 0xC8, 0xD0, 0xD4);
    if !pressed {
        // Outer Top/Left = White (#FFFFFF), Outer Bottom/Right = Dark Shadow (#404040, soft dark gray)
        for x in rx..(rx + rw) {
            set_px(buf, x, ry, 0xFF, 0xFF, 0xFF);
            if rh > 0 { set_px(buf, x, ry + rh - 1, 0x40, 0x40, 0x40); }
        }
        for y in ry..(ry + rh) {
            set_px(buf, rx, y, 0xFF, 0xFF, 0xFF);
            if rw > 0 { set_px(buf, rx + rw - 1, y, 0x40, 0x40, 0x40); }
        }
        // Inner Top/Left = Light face (#DFDFDF), Inner Bottom/Right = Medium Shadow (#808080)
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
    } else {
        // Sunken button state
        // Outer Top/Left = Dark Shadow (#404040), Outer Bottom/Right = White (#FFFFFF)
        for x in rx..(rx + rw) {
            set_px(buf, x, ry, 0x40, 0x40, 0x40);
            if rh > 0 { set_px(buf, x, ry + rh - 1, 0xFF, 0xFF, 0xFF); }
        }
        for y in ry..(ry + rh) {
            set_px(buf, rx, y, 0x40, 0x40, 0x40);
            if rw > 0 { set_px(buf, rx + rw - 1, y, 0xFF, 0xFF, 0xFF); }
        }
        // Inner Top/Left = Medium Shadow (#808080), Inner Bottom/Right = Face Light (#D4D0C8)
        if rw > 2 && rh > 2 {
            for x in (rx + 1)..(rx + rw - 1) {
                set_px(buf, x, ry + 1, 0x80, 0x80, 0x80);
                set_px(buf, x, ry + rh - 2, 0xC8, 0xD0, 0xD4);
            }
            for y in (ry + 1)..(ry + rh - 1) {
                set_px(buf, rx + 1, y, 0x80, 0x80, 0x80);
                set_px(buf, rx + rw - 2, y, 0xC8, 0xD0, 0xD4);
            }
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

fn draw_group_box(buf: &mut [u8], gx: usize, gy: usize, gw: usize, gh: usize, theme: cb_win32::WindowTheme) {
    let (bg_b, bg_g, bg_r) = theme.client_bg_color();
    match theme {
        cb_win32::WindowTheme::Windows311 => {
            draw_rect_outline(buf, gx, gy, gw, gh, 0, 0, 0);
            fill_box(buf, gx + 8, gy, 175, 2, bg_b, bg_g, bg_r);
        }
        cb_win32::WindowTheme::Windows98 => {
            draw_bevel(buf, gx, gy, gw, gh, false);
            fill_box(buf, gx + 8, gy, 175, 2, bg_b, bg_g, bg_r);
        }
        cb_win32::WindowTheme::WindowsXp => {
            draw_rect_outline(buf, gx, gy, gw, gh, 0xD0, 0xB8, 0xA0);
            fill_box(buf, gx + 8, gy, 175, 2, bg_b, bg_g, bg_r);
        }
    }
}

fn draw_themed_radio_button(buf: &mut [u8], cx: usize, cy: usize, theme: cb_win32::WindowTheme, checked: bool) {
    match theme {
        cb_win32::WindowTheme::Windows311 => {
            for dy in 0..11 {
                for dx in 0..11 {
                    let ddx = dx as i32 - 5;
                    let ddy = dy as i32 - 5;
                    let dist_sq = ddx * ddx + ddy * ddy;
                    if dist_sq <= 25 {
                        set_px(buf, cx + dx - 5, cy + dy - 5, 0xFF, 0xFF, 0xFF);
                    }
                    if dist_sq >= 16 && dist_sq <= 25 {
                        set_px(buf, cx + dx - 5, cy + dy - 5, 0, 0, 0);
                    }
                    if checked && dist_sq <= 6 {
                        set_px(buf, cx + dx - 5, cy + dy - 5, 0, 0, 0);
                    }
                }
            }
        }
        _ => {
            draw_radio_button(buf, cx, cy, checked);
        }
    }
}

fn draw_theme_button(
    buf: &mut [u8],
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
            fill_box(buf, bx, by, bw, bh, 0xC0, 0xC0, 0xC0);
            draw_rect_outline(buf, bx, by, bw, bh, 0, 0, 0);
            if sunken {
                draw_rect_outline(buf, bx + 1, by + 1, bw.saturating_sub(2), bh.saturating_sub(2), 0, 0, 0);
                draw_bevel(buf, bx + 2, by + 2, bw.saturating_sub(4), bh.saturating_sub(4), false);
            } else {
                draw_bevel(buf, bx + 1, by + 1, bw.saturating_sub(2), bh.saturating_sub(2), true);
            }
        }
        cb_win32::WindowTheme::Windows98 => {
            draw_win98_button(buf, bx, by, bw, bh, sunken);
        }
        cb_win32::WindowTheme::WindowsXp => {
            let (b, g, r) = if sunken {
                (0xD0, 0xA0, 0x70)
            } else {
                (0xF0, 0xEA, 0xE4)
            };
            fill_box(buf, bx, by, bw, bh, b, g, r);
            draw_rect_outline(buf, bx, by, bw, bh, 0x90, 0x60, 0x00);
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowId {
    App,
    ControlPanel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressedTarget {
    FormButton(usize),
    AppMinimize,
    AppMaximize,
    AppClose,
    CpMinimize,
    CpMaximize,
    CpClose,
    CpFormat24,
    CpFormat12,
    CpTheme311,
    CpTheme98,
    CpThemeXp,
}

fn draw_maximize_glyph(buf: &mut [u8], gx: usize, gy: usize, enabled: bool) {
    if enabled {
        fill_box(buf, gx, gy, 9, 2, 0, 0, 0);
        fill_box(buf, gx, gy + 7, 9, 1, 0, 0, 0);
        fill_box(buf, gx, gy, 1, 8, 0, 0, 0);
        fill_box(buf, gx + 8, gy, 1, 8, 0, 0, 0);
    } else {
        fill_box(buf, gx + 1, gy + 1, 9, 2, 0xFF, 0xFF, 0xFF);
        fill_box(buf, gx + 1, gy + 8, 9, 1, 0xFF, 0xFF, 0xFF);
        fill_box(buf, gx + 1, gy + 1, 1, 8, 0xFF, 0xFF, 0xFF);
        fill_box(buf, gx + 9, gy + 1, 1, 8, 0xFF, 0xFF, 0xFF);
        fill_box(buf, gx, gy, 9, 2, 0x80, 0x80, 0x80);
        fill_box(buf, gx, gy + 7, 9, 1, 0x80, 0x80, 0x80);
        fill_box(buf, gx, gy, 1, 8, 0x80, 0x80, 0x80);
        fill_box(buf, gx + 8, gy, 1, 8, 0x80, 0x80, 0x80);
    }
}


fn draw_xp_maximize_glyph(buf: &mut [u8], gx: usize, gy: usize, enabled: bool) {
    let (b, g, r) = if enabled {
        (0xFF, 0xFF, 0xFF)
    } else {
        (0xD0, 0xB8, 0xA0)
    };
    fill_box(buf, gx, gy, 10, 2, b, g, r);
    fill_box(buf, gx, gy + 8, 10, 1, b, g, r);
    fill_box(buf, gx, gy, 1, 9, b, g, r);
    fill_box(buf, gx + 9, gy, 1, 9, b, g, r);
}

fn draw_drag_frame(buf: &mut [u8], rx: i32, ry: i32, rw: usize, rh: usize, title_h: usize) {
    let x1 = rx;
    let y1 = ry;
    let x2 = rx + rw as i32 - 1;
    let y2 = ry + rh as i32 - 1;
    let tb_y = ry + title_h as i32;

    let set_stipple = |buf: &mut [u8], px: i32, py: i32| {
        if px >= 0 && px < 800 && py >= 0 && py < 600 {
            let is_white = (px + py) % 2 == 0;
            let col = if is_white { 0xFF } else { 0x00 };
            set_px(buf, px as usize, py as usize, col, col, col);
        }
    };

    // 2px thick outer boundary with classic 50% stipple (DrawFocusRect / XOR rubber band)
    for x in x1..=x2 {
        set_stipple(buf, x, y1);
        set_stipple(buf, x, y1 + 1);
        set_stipple(buf, x, y2 - 1);
        set_stipple(buf, x, y2);
        if tb_y > y1 && tb_y < y2 {
            set_stipple(buf, x, tb_y);
        }
    }
    for y in y1..=y2 {
        set_stipple(buf, x1, y);
        set_stipple(buf, x1 + 1, y);
        set_stipple(buf, x2 - 1, y);
        set_stipple(buf, x2, y);
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
            fill_box(buf, wx, wy, ww, wh, 0xC0, 0xC0, 0xC0);
            // Outer 1px solid black border
            draw_rect_outline(buf, wx, wy, ww, wh, 0, 0, 0);

            // Authentic Windows 3.11 4 corner cuts (45-degree diagonal seams at corners)
            for i in 0..5 {
                set_px(buf, wx + i, wy + i, 0, 0, 0); // Top-left
                if ww >= 5 {
                    set_px(buf, wx + ww - 1 - i, wy + i, 0, 0, 0); // Top-right
                }
                if wh >= 5 {
                    set_px(buf, wx + i, wy + wh - 1 - i, 0, 0, 0); // Bottom-left
                }
                if ww >= 5 && wh >= 5 {
                    set_px(buf, wx + ww - 1 - i, wy + wh - 1 - i, 0, 0, 0); // Bottom-right
                }
            }

            // Windows 3.11 sizing tick cut marks separating the corner resize handles from the edges (at 18px from corners)
            if ww > 36 {
                fill_box(buf, wx + 18, wy, 1, 5, 0, 0, 0);
                fill_box(buf, wx + ww - 19, wy, 1, 5, 0, 0, 0);
                if wh >= 5 {
                    fill_box(buf, wx + 18, wy + wh - 5, 1, 5, 0, 0, 0);
                    fill_box(buf, wx + ww - 19, wy + wh - 5, 1, 5, 0, 0, 0);
                }
            }
            if wh > 36 {
                fill_box(buf, wx, wy + 18, 5, 1, 0, 0, 0);
                fill_box(buf, wx, wy + wh - 19, 5, 1, 0, 0, 0);
                if ww >= 5 {
                    fill_box(buf, wx + ww - 5, wy + 18, 5, 1, 0, 0, 0);
                    fill_box(buf, wx + ww - 5, wy + wh - 19, 5, 1, 0, 0, 0);
                }
            }

            // Title bar row is 24px thick (wy + 5 to wy + 29)
            let tb_w = ww.saturating_sub(10);
            let tb_h = 24;
            draw_rect_outline(buf, wx + 5, wy + 5, tb_w, tb_h, 0, 0, 0);

            // Left System Menu Box (21x21 at wx + 6, wy + 6)
            let btn_size = 21;
            let sm_off = if is_close_pressed { 1 } else { 0 };
            fill_box(buf, wx + 6, wy + 6, btn_size, btn_size, 0xC0, 0xC0, 0xC0);
            draw_rect_outline(buf, wx + 6, wy + 6, btn_size, btn_size, 0, 0, 0);
            draw_bevel(buf, wx + 7, wy + 7, btn_size - 2, btn_size - 2, !is_close_pressed);
            // Authentic 3D beveled horizontal slot bar for Windows 3.11 System Menu
            let slot_x = wx + 10 + sm_off;
            let slot_y = wy + 15 + sm_off;
            fill_box(buf, slot_x, slot_y, 13, 1, 0, 0, 0);             // Black top line
            fill_box(buf, slot_x, slot_y + 1, 13, 1, 0x60, 0x60, 0x60); // Dark gray shadow
            fill_box(buf, slot_x, slot_y + 2, 13, 1, 0xDF, 0xDF, 0xDF); // Light gray
            fill_box(buf, slot_x, slot_y + 3, 13, 1, 0xFF, 0xFF, 0xFF); // White highlight

            // Right Minimize button ▼ (21x21 at min_x, wy + 6)
            let min_x = wx + ww.saturating_sub(50);
            let min_off = if is_min_pressed { 1 } else { 0 };
            fill_box(buf, min_x, wy + 6, btn_size, btn_size, 0xC0, 0xC0, 0xC0);
            draw_rect_outline(buf, min_x, wy + 6, btn_size, btn_size, 0, 0, 0);
            draw_bevel(buf, min_x + 1, wy + 7, btn_size - 2, btn_size - 2, !is_min_pressed);
            let min_cx = min_x + 10 + min_off;
            let min_cy = wy + 16 + min_off;
            for (row, width) in [9, 7, 5, 3, 1].iter().enumerate() {
                let start_x = min_cx - width / 2;
                fill_box(buf, start_x, min_cy - 2 + row, *width, 1, 0, 0, 0);
            }

            // Right Maximize button ▲ (21x21 at max_x, wy + 6)
            let max_x = wx + ww.saturating_sub(28);
            let max_off = if is_max_pressed { 1 } else { 0 };
            fill_box(buf, max_x, wy + 6, btn_size, btn_size, 0xC0, 0xC0, 0xC0);
            draw_rect_outline(buf, max_x, wy + 6, btn_size, btn_size, 0, 0, 0);
            draw_bevel(buf, max_x + 1, wy + 7, btn_size - 2, btn_size - 2, !is_max_pressed);
            let max_cx = max_x + 10 + max_off;
            let max_cy = wy + 16 + max_off;
            if max_button_enabled {
                for (row, width) in [1, 3, 5, 7, 9].iter().enumerate() {
                    let start_x = max_cx - width / 2;
                    fill_box(buf, start_x, max_cy - 2 + row, *width, 1, 0, 0, 0);
                }
            } else {
                for (row, width) in [1, 3, 5, 7, 9].iter().enumerate() {
                    let start_x = max_cx - width / 2;
                    fill_box(buf, start_x + 1, max_cy - 1 + row, *width, 1, 0xFF, 0xFF, 0xFF);
                    fill_box(buf, start_x, max_cy - 2 + row, *width, 1, 0x80, 0x80, 0x80);
                }
            }

            // Central Title Bar with clear 2px padding between buttons and the blue title bar
            // System menu ends at wx + 27; blue bar starts at wx + 29 (2px gray padding).
            // Minimize button starts at min_x; blue bar ends at min_x - 2 (2px gray padding).
            let blue_x = wx + 29;
            let blue_end = min_x.saturating_sub(2);
            if blue_end > blue_x {
                let blue_w = blue_end - blue_x;
                let blue_y = wy + 6;
                let blue_h = 22;
                if is_active {
                    fill_box(buf, blue_x, blue_y, blue_w, blue_h, 0xA0, 0x00, 0x00);
                } else {
                    fill_box(buf, blue_x, blue_y, blue_w, blue_h, 0xFF, 0xFF, 0xFF);
                }
                draw_rect_outline(buf, blue_x, blue_y, blue_w, blue_h, 0, 0, 0);
            }

            // Client area: Pure white background with black outline (5px border, 29px top)
            if ww > 10 && wh > 34 {
                fill_box(buf, wx + 5, wy + 29, ww - 10, wh - 34, bg_b, bg_g, bg_r);
                draw_rect_outline(buf, wx + 5, wy + 29, ww - 10, wh - 34, 0, 0, 0);
            }
        }
        cb_win32::WindowTheme::Windows98 => {
            fill_box(buf, wx, wy, ww, wh, bg_b, bg_g, bg_r);
            // Sizing frame with soft shadows (NO PITCH BLACK!)
            for x in wx..(wx + ww) {
                set_px(buf, x, wy, 0xFF, 0xFF, 0xFF);
                if wh > 0 { set_px(buf, x, wy + wh - 1, 0x40, 0x40, 0x40); }
            }
            for y in wy..(wy + wh) {
                set_px(buf, wx, y, 0xFF, 0xFF, 0xFF);
                if ww > 0 { set_px(buf, wx + ww - 1, y, 0x40, 0x40, 0x40); }
            }
            if ww > 2 && wh > 2 {
                for x in (wx + 1)..(wx + ww - 1) {
                    set_px(buf, x, wy + 1, 0xC8, 0xD0, 0xD4);
                    set_px(buf, x, wy + wh - 2, 0x80, 0x80, 0x80);
                }
                for y in (wy + 1)..(wy + wh - 1) {
                    set_px(buf, wx + 1, y, 0xC8, 0xD0, 0xD4);
                    set_px(buf, wx + ww - 2, y, 0x80, 0x80, 0x80);
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
                    set_px(buf, wx + 3 + x, wy + 3 + y, b, g, r);
                }
            }

            if let Some(ico) = icon_16 {
                draw_icon_16(buf, wx + 5, wy + 4, ico);
            }

            // Minimize button _ (16x14 at wx + ww - 54, wy + 4)
            draw_win98_button(buf, wx + ww - 54, wy + 4, 16, 14, is_min_pressed);
            let min_off = if is_min_pressed { 1 } else { 0 };
            fill_box(buf, wx + ww - 50 + min_off, wy + 13 + min_off, 6, 2, 0, 0, 0);

            // Maximize button [□] (16x14 at wx + ww - 38, wy + 4)
            draw_win98_button(buf, wx + ww - 38, wy + 4, 16, 14, is_max_pressed);
            let max_off = if is_max_pressed { 1 } else { 0 };
            draw_maximize_glyph(buf, wx + ww - 35 + max_off, wy + 7 + max_off, max_button_enabled);

            // Close button X (16x14 at wx + ww - 20, wy + 4)
            draw_win98_button(buf, wx + ww - 20, wy + 4, 16, 14, is_close_pressed);
            let cls_off = if is_close_pressed { 1 } else { 0 };
            draw_x_glyph(buf, wx + ww - 16 + cls_off, wy + 8 + cls_off, 0, 0, 0);
        }
        cb_win32::WindowTheme::WindowsXp => {
            // Outer 3px border: Royal steel blue (#0045B8) if active, Muted slate (#7080A8) if inactive
            let (bord_b, bord_g, bord_r) = if is_active {
                (0xB8, 0x45, 0x00)
            } else {
                (0xA8, 0x80, 0x70)
            };
            fill_box(buf, wx, wy, ww, wh, bord_b, bord_g, bord_r);
            if ww > 6 && wh > 32 {
                fill_box(buf, wx + 3, wy + 29, ww - 6, wh - 32, bg_b, bg_g, bg_r);
            }

            // Title bar (26px high)
            let tb_w = ww.saturating_sub(6);
            for x in 0..tb_w {
                let fraction_y = |y: usize| y as f32 / 26.0;
                if is_active {
                    set_px(buf, wx + 3 + x, wy + 3, 0xF9, 0x99, 0x3E);
                    for y in 1..26 {
                        let fraction = fraction_y(y);
                        let b = (0xEA as f32 + fraction * (0xD7 as f32 - 0xEA as f32)) as u8;
                        let g = (0x55 as f32 + fraction * (0x65 as f32 - 0x55 as f32)) as u8;
                        let r = (0x00 as f32 + fraction * (0x1E as f32 - 0x00 as f32)) as u8;
                        set_px(buf, wx + 3 + x, wy + 3 + y, b, g, r);
                    }
                } else {
                    set_px(buf, wx + 3 + x, wy + 3, 0xE8, 0xC8, 0xB5);
                    for y in 1..26 {
                        let fraction = fraction_y(y);
                        let b = (0xD7 as f32 + fraction * (0xDC as f32 - 0xD7 as f32)) as u8;
                        let g = (0x97 as f32 + fraction * (0xA6 as f32 - 0x97 as f32)) as u8;
                        let r = (0x76 as f32 + fraction * (0x8F as f32 - 0x76 as f32)) as u8;
                        set_px(buf, wx + 3 + x, wy + 3 + y, b, g, r);
                    }
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

            // XP Minimize button _ (21x21 at wx + ww - 72, wy + 5)
            let (min_b, min_g, min_r) = if is_min_pressed {
                (0xC0, 0x58, 0x18)
            } else if is_active {
                (0xE8, 0x6F, 0x28)
            } else {
                (0xC0, 0x90, 0x7A)
            };
            fill_box(buf, wx + ww - 72, wy + 5, 21, 21, min_b, min_g, min_r);
            draw_rect_outline(buf, wx + ww - 72, wy + 5, 21, 21, if is_min_pressed { 0x80 } else { 0xFF }, if is_min_pressed { 0x38 } else { 0xFF }, if is_min_pressed { 0x08 } else { 0xFF });
            let min_off = if is_min_pressed { 1 } else { 0 };
            fill_box(buf, wx + ww - 66 + min_off, wy + 19 + min_off, 8, 2, 0xFF, 0xFF, 0xFF);

            // XP Maximize button [□] (21x21 at wx + ww - 49, wy + 5)
            let (max_b, max_g, max_r) = if is_max_pressed {
                (0xC0, 0x58, 0x18)
            } else if is_active {
                (0xE8, 0x6F, 0x28)
            } else {
                (0xC0, 0x90, 0x7A)
            };
            fill_box(buf, wx + ww - 49, wy + 5, 21, 21, max_b, max_g, max_r);
            draw_rect_outline(buf, wx + ww - 49, wy + 5, 21, 21, if is_max_pressed { 0x80 } else { 0xFF }, if is_max_pressed { 0x38 } else { 0xFF }, if is_max_pressed { 0x08 } else { 0xFF });
            let max_off = if is_max_pressed { 1 } else { 0 };
            draw_xp_maximize_glyph(buf, wx + ww - 44 + max_off, wy + 11 + max_off, max_button_enabled);

            // XP Close button X (21x21 at wx + ww - 26, wy + 5)
            let (cls_b, cls_g, cls_r) = if is_close_pressed {
                (0x18, 0x30, 0xB0)
            } else if is_active {
                (0x28, 0x44, 0xD8)
            } else {
                (0x78, 0x78, 0xAA)
            };
            fill_box(buf, wx + ww - 26, wy + 5, 21, 21, cls_b, cls_g, cls_r);
            draw_rect_outline(buf, wx + ww - 26, wy + 5, 21, 21, if is_close_pressed { 0x70 } else { 0xE8 }, if is_close_pressed { 0x18 } else { 0x65 }, if is_close_pressed { 0x08 } else { 0x50 });
            let cls_off = if is_close_pressed { 1 } else { 0 };
            draw_x_glyph(buf, wx + ww - 20 + cls_off, wy + 11 + cls_off, 0xFF, 0xFF, 0xFF);
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
    max_button_enabled: bool,
    selected_icon: Option<u32>,
    active_window: Option<WindowId>,
    z_order: &[WindowId],
    cp_open: bool,
    cp_minimized: bool,
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
    // When ANY window has active focus, desktop icons are inactive (no selection box)
    let icons_interactive = active_window.is_none();

    // Icon 1: Running App
    if let Some(ico32) = app_icon_32 {
        draw_icon_32(&mut pixels, 40, 26, ico32);
    }
    // Icon 1 Centered Label
    let text_w = (form_caption.len() * 7).clamp(24, 120);
    let text_x = 56i16 - (text_w as i16 / 2);
    if icons_interactive && selected_icon == Some(1) {
        let box_w = (text_w + 12).clamp(40, 140);
        let box_x = 56usize.saturating_sub(box_w / 2);
        fill_box(&mut pixels, box_x, 62, box_w, 18, 0x80, 0x00, 0x00);
        draw_rect_outline(&mut pixels, box_x, 62, box_w, 18, 0xFF, 0xFF, 0xFF);
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: text_x,
            y: 75,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -12,
            text: form_caption.to_string(),
        }));
    } else {
        // Inactive: drop shadow in black, text in white, no box
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: text_x + 1,
            y: 76,
            color_rgb: 0x00000000,
            bk_mode: 1,
            font_height: -12,
            text: form_caption.to_string(),
        }));
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: text_x,
            y: 75,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -12,
            text: form_caption.to_string(),
        }));
    }

    // Icon 2: Control Panel
    draw_icon_32(&mut pixels, 40, 116, cp_icon_32);
    // Icon 2 Centered Label ("Painel de Controle")
    if icons_interactive && selected_icon == Some(2) {
        fill_box(&mut pixels, 18, 152, 76, 32, 0x80, 0x00, 0x00);
        draw_rect_outline(&mut pixels, 18, 152, 76, 32, 0xFF, 0xFF, 0xFF);
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 27,
            y: 165,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -11,
            text: "Painel de".to_string(),
        }));
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 30,
            y: 178,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -11,
            text: "Controle".to_string(),
        }));
    } else {
        // Inactive: drop shadow in black, text in white, no box
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 28,
            y: 166,
            color_rgb: 0x00000000,
            bk_mode: 1,
            font_height: -11,
            text: "Painel de".to_string(),
        }));
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 27,
            y: 165,
            color_rgb: 0x00FFFFFF,
            bk_mode: 1,
            font_height: -11,
            text: "Painel de".to_string(),
        }));
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 31,
            y: 179,
            color_rgb: 0x00000000,
            bk_mode: 1,
            font_height: -11,
            text: "Controle".to_string(),
        }));
        text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
            hwnd: 0,
            x: 30,
            y: 178,
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
                WindowId::App if app_open && !app_minimized => {
                    higher_rects.push((app_win_x, app_win_y, app_win_w, app_win_h));
                }
                WindowId::ControlPanel if cp_open && !cp_minimized => {
                    higher_rects.push((cp_x, cp_y, cp_w, cp_h));
                }
                _ => {}
            }
        }

        match wid {
            WindowId::App if app_open && !app_minimized => {
                let is_active = active_window == Some(WindowId::App);
                let is_app_min_pressed = pressed_target == Some(PressedTarget::AppMinimize);
                let is_app_max_pressed = pressed_target == Some(PressedTarget::AppMaximize);
                let is_app_close_pressed = pressed_target == Some(PressedTarget::AppClose);
                draw_themed_window(
                    &mut pixels,
                    app_win_x,
                    app_win_y,
                    app_win_w,
                    app_win_h,
                    theme,
                    app_icon_16,
                    is_active,
                    max_button_enabled,
                    is_app_min_pressed,
                    is_app_max_pressed,
                    is_app_close_pressed,
                );

                let (title_x, title_y) = match theme {
                    cb_win32::WindowTheme::Windows311 => {
                        let text_len = form_caption.len() as i16;
                        let approx_w = text_len * 7;
                        let center_x = (app_win_x + app_win_w / 2) as i16;
                        ((center_x - approx_w / 2).max((app_win_x + 32) as i16), (app_win_y + 22) as i16)
                    }
                    cb_win32::WindowTheme::Windows98 => (
                        if app_icon_16.is_some() { (app_win_x + 26) as i16 } else { (app_win_x + 8) as i16 },
                        (app_win_y + 17) as i16,
                    ),
                    cb_win32::WindowTheme::WindowsXp => (
                        (app_win_x + 28) as i16,
                        (app_win_y + 21) as i16,
                    ),
                };
                let title_h = match theme {
                    cb_win32::WindowTheme::Windows311 => 29,
                    cb_win32::WindowTheme::Windows98 => 21,
                    cb_win32::WindowTheme::WindowsXp => 29,
                };
                let title_color = theme.title_text_color(is_active);

                if !is_occluded(title_x, title_y, &higher_rects) && !is_occluded_rect(app_win_x as i16, app_win_y as i16, app_win_w as u16, title_h, &higher_rects) {
                    text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 100,
                        x: title_x,
                        y: title_y,
                        color_rgb: title_color,
                        bk_mode: 1,
                        font_height: -13,
                        text: form_caption.to_string(),
                    }));
                }

                let (bg_b, bg_g, bg_r) = theme.client_bg_color();
                for btn in buttons {
                    let b_pixels = btn.render_pixels();
                    for by in 0..btn.height as usize {
                        for bx in 0..btn.width as usize {
                            let sidx = (by * btn.width as usize + bx) * 4;
                            set_px(
                                &mut pixels,
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

                for tb in textboxes {
                    let (b, g, r) = (tb.back_color as u8, (tb.back_color >> 8) as u8, (tb.back_color >> 16) as u8);
                    fill_box(&mut pixels, tb.x, tb.y, tb.w, tb.h, b, g, r);
                    draw_bevel(&mut pixels, tb.x, tb.y, tb.w, tb.h, false);
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

                for lbl in labels {
                    fill_box(&mut pixels, lbl.x, lbl.y, lbl.w, lbl.h, bg_b, bg_g, bg_r);
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
            }
            WindowId::ControlPanel if cp_open && !cp_minimized => {
                let is_active = active_window == Some(WindowId::ControlPanel);
                let is_cp_min_pressed = pressed_target == Some(PressedTarget::CpMinimize);
                let is_cp_max_pressed = pressed_target == Some(PressedTarget::CpMaximize);
                let is_cp_close_pressed = pressed_target == Some(PressedTarget::CpClose);
                draw_themed_window(
                    &mut pixels,
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
                        ((center_x - approx_w / 2).max((cp_x + 32) as i16), (cp_y + 22) as i16)
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
                draw_group_box(&mut pixels, cp_x + 14, cp_y + 36, cp_w - 28, 84, theme);
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
                draw_themed_radio_button(&mut pixels, cp_x + 30, cp_y + 60, theme, !time_format_12h || is_r1_pressed);
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

                draw_themed_radio_button(&mut pixels, cp_x + 30, cp_y + 88, theme, time_format_12h || is_r2_pressed);
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
                draw_group_box(&mut pixels, cp_x + 14, cp_y + 130, cp_w - 28, 126, theme);
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
                draw_theme_button(&mut pixels, cp_x + 22, cp_y + 152, 94, 30, theme, is_w311, is_b1_pressed);
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
                draw_theme_button(&mut pixels, cp_x + 126, cp_y + 152, 94, 30, theme, is_w98, is_b2_pressed);
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
                draw_theme_button(&mut pixels, cp_x + 230, cp_y + 152, 94, 30, theme, is_wxp, is_b3_pressed);
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
            }
            _ => {}
        }
    }

    if let Some((dx, dy, dw, dh, dtb)) = drag_frame {
        draw_drag_frame(&mut pixels, dx, dy, dw, dh, dtb);
    }

    (pixels, text_frames)
}

fn hit_test_target(
    target: PressedTarget,
    px: i32,
    py: i32,
    win_x: usize,
    win_y: usize,
    win_w: usize,
    cp_x: usize,
    cp_y: usize,
    cp_w: usize,
    theme: cb_win32::WindowTheme,
    buttons: &[cb_win32::Win32ButtonControl],
) -> bool {
    match target {
        PressedTarget::AppClose => is_close_click(win_x, win_y, win_w, theme, px, py),
        PressedTarget::AppMinimize => is_minimize_click(win_x, win_y, win_w, theme, px, py),
        PressedTarget::AppMaximize => is_maximize_click(win_x, win_y, win_w, theme, px, py),
        PressedTarget::CpClose => is_close_click(cp_x, cp_y, cp_w, theme, px, py),
        PressedTarget::CpMinimize => is_minimize_click(cp_x, cp_y, cp_w, theme, px, py),
        PressedTarget::CpMaximize => is_maximize_click(cp_x, cp_y, cp_w, theme, px, py),
        PressedTarget::CpFormat24 => px >= (cp_x + 24) as i32 && px < (cp_x + 320) as i32 && py >= (cp_y + 54) as i32 && py < (cp_y + 78) as i32,
        PressedTarget::CpFormat12 => px >= (cp_x + 24) as i32 && px < (cp_x + 320) as i32 && py >= (cp_y + 82) as i32 && py < (cp_y + 108) as i32,
        PressedTarget::CpTheme311 => px >= (cp_x + 22) as i32 && px < (cp_x + 116) as i32 && py >= (cp_y + 152) as i32 && py < (cp_y + 182) as i32,
        PressedTarget::CpTheme98 => px >= (cp_x + 126) as i32 && px < (cp_x + 220) as i32 && py >= (cp_y + 152) as i32 && py < (cp_y + 182) as i32,
        PressedTarget::CpThemeXp => px >= (cp_x + 230) as i32 && px < (cp_x + 324) as i32 && py >= (cp_y + 152) as i32 && py < (cp_y + 182) as i32,
        PressedTarget::FormButton(idx) => {
            if let Some(btn) = buttons.get(idx) {
                btn.hit_test(px, py)
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
            px >= wx + 5 && px < wx + 28 && py >= wy + 5 && py < wy + 28
        }
        cb_win32::WindowTheme::Windows98 => {
            px >= wx + ww.saturating_sub(20) && px < wx + ww.saturating_sub(4) && py >= wy + 4 && py < wy + 18
        }
        cb_win32::WindowTheme::WindowsXp => {
            px >= wx + ww.saturating_sub(26) && px < wx + ww.saturating_sub(5) && py >= wy + 5 && py < wy + 26
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
            px >= wx + ww.saturating_sub(28) && px < wx + ww.saturating_sub(5) && py >= wy + 5 && py < wy + 28
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
            px >= wx + ww.saturating_sub(50) && px < wx + ww.saturating_sub(28) && py >= wy + 5 && py < wy + 28
        }
        cb_win32::WindowTheme::Windows98 => {
            px >= wx + ww.saturating_sub(54) && px < wx + ww.saturating_sub(38) && py >= wy + 4 && py < wy + 18
        }
        cb_win32::WindowTheme::WindowsXp => {
            px >= wx + ww.saturating_sub(72) && px < wx + ww.saturating_sub(51) && py >= wy + 5 && py < wy + 26
        }
    }
}

fn apply_theme_layout(
    theme: cb_win32::WindowTheme,
    client_w: usize,
    client_h: usize,
    win_w: &mut usize,
    win_h: &mut usize,
    win_x: &mut usize,
    win_y: &mut usize,
    cp_w: &mut usize,
    cp_h: &mut usize,
    cp_x: &mut usize,
    cp_y: &mut usize,
    buttons: &mut [cb_win32::Win32ButtonControl],
    button_rel_coords: &[(usize, usize)],
    textboxes: &mut [DynamicTextBox],
    labels: &mut [DynamicLabel],
) {
    let (nw, nh) = theme.window_size(client_w, client_h);
    *win_w = nw.clamp(120, 1600);
    *win_h = nh.clamp(60, 1200);
    *win_x = (*win_x).min(800usize.saturating_sub(*win_w));
    *win_y = (*win_y).min(600usize.saturating_sub(*win_h));

    let (cp_nw, cp_nh) = theme.window_size(350, 240);
    *cp_w = cp_nw.clamp(120, 800);
    *cp_h = cp_nh.clamp(60, 600);
    *cp_x = (*cp_x).min(800usize.saturating_sub(*cp_w));
    *cp_y = (*cp_y).min(600usize.saturating_sub(*cp_h));

    let (off_x, off_y) = theme.client_offset();
    for (btn, &(rx, ry)) in buttons.iter_mut().zip(button_rel_coords.iter()) {
        btn.x = (*win_x + off_x + rx) as i32;
        btn.y = (*win_y + off_y + ry) as i32;
    }
    for tb in textboxes.iter_mut() {
        tb.x = *win_x + off_x + tb.rel_x;
        tb.y = *win_y + off_y + tb.rel_y;
    }
    for lbl in labels.iter_mut() {
        lbl.x = *win_x + off_x + lbl.rel_x;
        lbl.y = *win_y + off_y + lbl.rel_y;
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

            let mut theme = cb_win32::WindowTheme::Windows98;
            let mut time_format_12h = false;
            let mut app_open = true;
            let mut app_minimized = false;
            let mut cp_open = false;
            let mut cp_minimized = false;
            let mut selected_icon: Option<u32> = None;

            let (mut win_w, mut win_h) = theme.window_size(client_w, client_h);
            let mut win_x = (800usize.saturating_sub(win_w)) / 2;
            let mut win_y = (600usize.saturating_sub(win_h)) / 2;

            let (mut cp_w, mut cp_h) = theme.window_size(350, 240);
            let mut cp_x = (800usize.saturating_sub(cp_w)) / 2;
            let mut cp_y = (600usize.saturating_sub(cp_h)) / 2;

            let (off_x, off_y) = theme.client_offset();

            let mut button_rel_coords: Vec<(usize, usize)> = Vec::new();
            let mut buttons: Vec<cb_win32::Win32ButtonControl> = form
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
                    let rel_x = (c.left / 15).max(0) as usize;
                    let rel_y = (c.top / 15).max(0) as usize;
                    let abs_x = win_x + off_x + rel_x;
                    let abs_y = win_y + off_y + rel_y;
                    let abs_w = (c.width / 15).max(1) as usize;
                    let abs_h = (c.height / 15).max(1) as usize;
                    DynamicTextBox {
                        hwnd: (200 + idx) as u32,
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

            let mut labels: Vec<DynamicLabel> = form
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
                        hwnd: (200 + idx) as u32,
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
            let max_button_enabled = form.max_button;
            let mut z_order = vec![WindowId::App];
            let mut active_window: Option<WindowId> = Some(WindowId::App);

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
                    let abs_x = (win_x as i16 + off_x as i16) + (ctrl.left / 15) as i16;
                    let abs_y = (win_y as i16 + off_y as i16) + (ctrl.top / 15) as i16;
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
                    max_button_enabled,
                    selected_icon,
                    active_window,
                    &z_order,
                    cp_open,
                    cp_minimized,
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
                                      win_x: usize,
                                      win_y: usize,
                                      win_w: usize,
                                      win_h: usize,
                                      cp_x: usize,
                                      cp_y: usize,
                                      cp_w: usize,
                                      cp_h: usize,
                                      max_button_enabled: bool,
                                      selected_icon: Option<u32>,
                                      active_window: Option<WindowId>,
                                      z_order: &[WindowId],
                                      cp_open: bool,
                                      cp_minimized: bool,
                                      time_format_12h: bool,
                                      server: &cb_webserver::ServerHandle,
                                      buttons: &[cb_win32::Win32ButtonControl],
                                      textboxes: &[DynamicTextBox],
                                      labels: &[DynamicLabel],
                                      pressed_target: Option<PressedTarget>,
                                      drag_frame: Option<(i32, i32, usize, usize, usize)>| {
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
                        max_button_enabled,
                        selected_icon,
                        active_window,
                        z_order,
                        cp_open,
                        cp_minimized,
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
                let mut pressed_target: Option<PressedTarget> = None;
                let mut dragging_window: Option<WindowId> = None;
                let mut drag_offset: (i32, i32) = (0, 0);
                let mut drag_curr_rect: Option<(i32, i32, usize, usize)> = None;

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
                                            win_x,
                                            win_y,
                                            win_w,
                                            win_h,
                                            cp_x,
                                            cp_y,
                                            cp_w,
                                            cp_h,
                                            max_button_enabled,
                                            selected_icon,
                                            active_window,
                                            &z_order,
                                            cp_open,
                                            cp_minimized,
                                            time_format_12h,
                                            &server,
                                            &buttons,
                                            &textboxes,
                                            &labels,
                                            pressed_target,
                                            None,
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
                                        // App Desktop Icon double clicked: unminimize / reopen and bring to top
                                        app_open = true;
                                        app_minimized = false;
                                        z_order.retain(|&w| w != WindowId::App);
                                        z_order.push(WindowId::App);
                                        active_window = Some(WindowId::App);
                                        selected_icon = None;
                                        redraw_desktop(theme, app_open, app_minimized, win_x, win_y, win_w, win_h, cp_x, cp_y, cp_w, cp_h, max_button_enabled, selected_icon, active_window, &z_order, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels, None, None);
                                        continue;
                                    } else if px >= 16 && px < 100 && py >= 106 && py < 192 {
                                        // Control Panel Desktop Icon double clicked: open / unminimize and bring to top
                                        cp_open = true;
                                        cp_minimized = false;
                                        z_order.retain(|&w| w != WindowId::ControlPanel);
                                        z_order.push(WindowId::ControlPanel);
                                        active_window = Some(WindowId::ControlPanel);
                                        selected_icon = None;
                                        redraw_desktop(theme, app_open, app_minimized, win_x, win_y, win_w, win_h, cp_x, cp_y, cp_w, cp_h, max_button_enabled, selected_icon, active_window, &z_order, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels, None, None);
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
                                        redraw_desktop(theme, app_open, app_minimized, win_x, win_y, win_w, win_h, cp_x, cp_y, cp_w, cp_h, max_button_enabled, selected_icon, active_window, &z_order, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels, None, None);
                                    }
                                } else {
                                    let mut window_hit = false;
                                    let mut activate_target = None;

                                    // Hit-test in reverse Z-order (top window gets priority)
                                    for &wid in z_order.iter().rev() {
                                        match wid {
                                            WindowId::ControlPanel if cp_open && !cp_minimized => {
                                                if px >= cp_x as i32 && px < (cp_x + cp_w) as i32 && py >= cp_y as i32 && py < (cp_y + cp_h) as i32 {
                                                    window_hit = true;
                                                    activate_target = Some(WindowId::ControlPanel);

                                                    if is_close_click(cp_x, cp_y, cp_w, theme, px, py) {
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
                                            WindowId::App if app_open && !app_minimized => {
                                                if px >= win_x as i32 && px < (win_x + win_w) as i32 && py >= win_y as i32 && py < (win_y + win_h) as i32 {
                                                    window_hit = true;
                                                    activate_target = Some(WindowId::App);

                                                    if is_close_click(win_x, win_y, win_w, theme, px, py) {
                                                        pressed_target = Some(PressedTarget::AppClose);
                                                    } else if is_minimize_click(win_x, win_y, win_w, theme, px, py) {
                                                        pressed_target = Some(PressedTarget::AppMinimize);
                                                    } else if is_maximize_click(win_x, win_y, win_w, theme, px, py) {
                                                        if max_button_enabled {
                                                            pressed_target = Some(PressedTarget::AppMaximize);
                                                        }
                                                    } else if py < (win_y + theme.client_offset().1) as i32 {
                                                        dragging_window = Some(WindowId::App);
                                                        drag_offset = (px - win_x as i32, py - win_y as i32);
                                                        drag_curr_rect = Some((win_x as i32, win_y as i32, win_w, win_h));
                                                    } else {
                                                        for (idx, btn) in buttons.iter_mut().enumerate() {
                                                            if btn.on_lbutton_down(px, py) {
                                                                pressed_target = Some(PressedTarget::FormButton(idx));
                                                                break;
                                                            }
                                                        }
                                                    }
                                                    break;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }

                                    if let Some(target) = activate_target {
                                        z_order.retain(|&w| w != target);
                                        z_order.push(target);
                                        active_window = Some(target);
                                        selected_icon = None; // Desktop icons are inactive when a window is focused
                                        redraw_desktop(theme, app_open, app_minimized, win_x, win_y, win_w, win_h, cp_x, cp_y, cp_w, cp_h, max_button_enabled, selected_icon, active_window, &z_order, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels, pressed_target, None);
                                    } else if !window_hit {
                                        // Clicked empty desktop or desktop icon: window focus lost
                                        active_window = None;
                                        if px >= 16 && px < 96 && py >= 16 && py < 96 {
                                            selected_icon = Some(1);
                                        } else if px >= 16 && px < 100 && py >= 106 && py < 192 {
                                            selected_icon = Some(2);
                                        } else {
                                            selected_icon = None;
                                        }
                                        redraw_desktop(theme, app_open, app_minimized, win_x, win_y, win_w, win_h, cp_x, cp_y, cp_w, cp_h, max_button_enabled, selected_icon, active_window, &z_order, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels, pressed_target, None);
                                    }
                                }
                            }
                            cb_win32::WM_MOUSEMOVE => {
                                let px = msg.pt_x;
                                let py = msg.pt_y;
                                if let Some(drag_win) = dragging_window {
                                    let (curr_w, curr_h) = match drag_win {
                                        WindowId::App => (win_w, win_h),
                                        WindowId::ControlPanel => (cp_w, cp_h),
                                    };
                                    let new_x = (px - drag_offset.0).clamp(-((curr_w as i32) - 40), 800 - 40);
                                    let new_y = (py - drag_offset.1).clamp(0, 600 - 30);
                                    drag_curr_rect = Some((new_x, new_y, curr_w, curr_h));
                                    let drag_param = Some((new_x, new_y, curr_w, curr_h, theme.client_offset().1));
                                    redraw_desktop(theme, app_open, app_minimized, win_x, win_y, win_w, win_h, cp_x, cp_y, cp_w, cp_h, max_button_enabled, selected_icon, active_window, &z_order, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels, pressed_target, drag_param);
                                } else if !dialog_active && app_open && !app_minimized {
                                    if let Some(PressedTarget::FormButton(idx)) = pressed_target {
                                        if let Some(btn) = buttons.get_mut(idx) {
                                            if btn.has_capture && btn.on_mouse_move(px, py) {
                                                redraw_desktop(theme, app_open, app_minimized, win_x, win_y, win_w, win_h, cp_x, cp_y, cp_w, cp_h, max_button_enabled, selected_icon, active_window, &z_order, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels, pressed_target, None);
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
                                            WindowId::App => (win_w, win_h),
                                            WindowId::ControlPanel => (cp_w, cp_h),
                                        };
                                        let clamped_x = nx.clamp(0, 800usize.saturating_sub(curr_w) as i32) as usize;
                                        let clamped_y = ny.clamp(0, 600usize.saturating_sub(curr_h) as i32) as usize;
                                        match drag_win {
                                            WindowId::App => {
                                                let d_x = clamped_x as i32 - win_x as i32;
                                                let d_y = clamped_y as i32 - win_y as i32;
                                                win_x = clamped_x;
                                                win_y = clamped_y;
                                                for btn in &mut buttons {
                                                    btn.x += d_x;
                                                    btn.y += d_y;
                                                }
                                                for tb in &mut textboxes {
                                                    tb.x = (tb.x as i32 + d_x).max(0) as usize;
                                                    tb.y = (tb.y as i32 + d_y).max(0) as usize;
                                                }
                                                for lbl in &mut labels {
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
                                    redraw_desktop(theme, app_open, app_minimized, win_x, win_y, win_w, win_h, cp_x, cp_y, cp_w, cp_h, max_button_enabled, selected_icon, active_window, &z_order, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels, None, None);
                                } else if let Some(target) = pressed_target.take() {
                                    let inside = hit_test_target(target, px, py, win_x, win_y, win_w, cp_x, cp_y, cp_w, theme, &buttons);
                                    if inside {
                                        match target {
                                            PressedTarget::AppClose => {
                                                app_open = false;
                                                active_window = if cp_open && !cp_minimized { Some(WindowId::ControlPanel) } else { None };
                                            }
                                            PressedTarget::AppMinimize => {
                                                app_minimized = true;
                                                active_window = if cp_open && !cp_minimized { Some(WindowId::ControlPanel) } else { None };
                                            }
                                            PressedTarget::AppMaximize => {}
                                            PressedTarget::CpClose => {
                                                cp_open = false;
                                                active_window = if app_open && !app_minimized { Some(WindowId::App) } else { None };
                                            }
                                            PressedTarget::CpMinimize => {
                                                cp_minimized = true;
                                                active_window = if app_open && !app_minimized { Some(WindowId::App) } else { None };
                                            }
                                            PressedTarget::CpMaximize => {}
                                            PressedTarget::CpFormat24 => {
                                                time_format_12h = false;
                                                msvbvm60::rtc::set_time_format_12h(false);
                                            }
                                            PressedTarget::CpFormat12 => {
                                                time_format_12h = true;
                                                msvbvm60::rtc::set_time_format_12h(true);
                                            }
                                            PressedTarget::CpTheme311 => {
                                                theme = cb_win32::WindowTheme::Windows311;
                                                apply_theme_layout(
                                                    theme,
                                                    client_w,
                                                    client_h,
                                                    &mut win_w,
                                                    &mut win_h,
                                                    &mut win_x,
                                                    &mut win_y,
                                                    &mut cp_w,
                                                    &mut cp_h,
                                                    &mut cp_x,
                                                    &mut cp_y,
                                                    &mut buttons,
                                                    &button_rel_coords,
                                                    &mut textboxes,
                                                    &mut labels,
                                                );
                                            }
                                            PressedTarget::CpTheme98 => {
                                                theme = cb_win32::WindowTheme::Windows98;
                                                apply_theme_layout(
                                                    theme,
                                                    client_w,
                                                    client_h,
                                                    &mut win_w,
                                                    &mut win_h,
                                                    &mut win_x,
                                                    &mut win_y,
                                                    &mut cp_w,
                                                    &mut cp_h,
                                                    &mut cp_x,
                                                    &mut cp_y,
                                                    &mut buttons,
                                                    &button_rel_coords,
                                                    &mut textboxes,
                                                    &mut labels,
                                                );
                                            }
                                            PressedTarget::CpThemeXp => {
                                                theme = cb_win32::WindowTheme::WindowsXp;
                                                apply_theme_layout(
                                                    theme,
                                                    client_w,
                                                    client_h,
                                                    &mut win_w,
                                                    &mut win_h,
                                                    &mut win_x,
                                                    &mut win_y,
                                                    &mut cp_w,
                                                    &mut cp_h,
                                                    &mut cp_x,
                                                    &mut cp_y,
                                                    &mut buttons,
                                                    &button_rel_coords,
                                                    &mut textboxes,
                                                    &mut labels,
                                                );
                                            }
                                            PressedTarget::FormButton(idx) => {
                                                if let Some(btn) = buttons.get_mut(idx) {
                                                    let (_redraw, click_fired) = btn.on_lbutton_up(px, py);
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
                                                }
                                            }
                                        }
                                    } else {
                                        if let PressedTarget::FormButton(idx) = target {
                                            if let Some(btn) = buttons.get_mut(idx) {
                                                btn.on_lbutton_up(px, py);
                                            }
                                        }
                                    }
                                    redraw_desktop(theme, app_open, app_minimized, win_x, win_y, win_w, win_h, cp_x, cp_y, cp_w, cp_h, max_button_enabled, selected_icon, active_window, &z_order, cp_open, cp_minimized, time_format_12h, &server, &buttons, &textboxes, &labels, None, None);
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
                                        redraw_desktop(
                                            theme,
                                            app_open,
                                            app_minimized,
                                            win_x,
                                            win_y,
                                            win_w,
                                            win_h,
                                            cp_x,
                                            cp_y,
                                            cp_w,
                                            cp_h,
                                            max_button_enabled,
                                            selected_icon,
                                            active_window,
                                            &z_order,
                                            cp_open,
                                            cp_minimized,
                                            time_format_12h,
                                            &server,
                                            &buttons,
                                            &textboxes,
                                            &labels,
                                            pressed_target,
                                            None,
                                        );
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
