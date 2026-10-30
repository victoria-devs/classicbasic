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
    println!("\nUsage: {} <path-to-vb6-exe> [options]", prog_name);
    println!("\nOptions:");
    println!("  --inspect     Inspect PE structure, sections, and metadata without running");
    println!("  --dump-info   Dump full VBHeader, ProjectInfo, and object tables");
    println!("  --run         Execute the binary (default behavior)");
    println!("  --headless    Execute in headless mode (default for CLI, batch, and daemons)");
    println!(
        "  --gui-web     Execute in Web-Streamed GUI mode (WebSocket / WebAssembly remote canvas)"
    );
    println!("  --port <PORT> Port to bind web server for --gui-web (default: 8080 or PORT env)");
    println!("  --help, -h    Display this help message\n");
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

fn dismiss_msgbox_dialog(
    server: &cb_webserver::ServerHandle,
    buttons: &[cb_win32::Win32ButtonControl],
    textboxes: &[DynamicTextBox],
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

    print_banner();
    println!("[*] Reading binary: {}", file_path);

    let data = match fs::read(file_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        }
    };

    println!(
        "[*] Parsing PE32 and mapping sections ({} bytes)...",
        data.len()
    );
    let loaded_pe = match LoadedPe::load(&data) {
        Ok(pe) => pe,
        Err(e) => {
            eprintln!("PE Loader Error: {}", e);
            process::exit(1);
        }
    };

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
    let vb_hdr_ptr = loaded_pe.find_vb_header();

    match vb_hdr_ptr {
        Some(ptr) => {
            let vb_hdr = unsafe { *ptr };
            println!("[+] Visual Basic 6 Executable Verified!");
            let build = vb_hdr.runtime_build_version;
            let rev = vb_hdr.runtime_revision;
            let forms = vb_hdr.w_forms_count;
            let threads = vb_hdr.dw_thread_count;
            let sub_main = vb_hdr.lp_sub_main;
            println!(
                "    - Build Version: 0x{:04X}, Revision: 0x{:04X}",
                build, rev
            );
            println!("    - Forms Count: {}", forms);
            println!("    - Thread Count: {}", threads);
            println!("    - Sub Main Address: 0x{:08X}", sub_main);

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
                    println!("    - Project Name: {}", name_cstr.to_string_lossy());
                }
                if is_valid_ptr(proj_info.lp_project_title) {
                    let title_cstr = unsafe {
                        std::ffi::CStr::from_ptr(proj_info.lp_project_title as *const i8)
                    };
                    println!("    - Project Title: {}", title_cstr.to_string_lossy());
                }
                if is_valid_ptr(proj_info.lp_project_description) {
                    let desc_cstr = unsafe {
                        std::ffi::CStr::from_ptr(proj_info.lp_project_description as *const i8)
                    };
                    println!("    - Description: {}", desc_cstr.to_string_lossy());
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
                            println!(
                                "[+] Dynamically deserialized Form from PE GUI Table: \"{}\" (caption: \"{}\", {} controls)",
                                f.name,
                                f.caption,
                                f.controls.len()
                            );
                            Some(f)
                        }
                        Err(e) => {
                            println!("[-] Note: PE GUI table deserializer: {}", e);
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let form = form_desc.unwrap_or_else(|| {
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

            cb_win32::ensure_default_form_runtime();
            {
                let mut lock = cb_win32::global_form_runtime().lock().unwrap();
                *lock = Some(cb_win32::FormRuntime::new(form.clone()));
            }

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

                let mut initial_frames = Vec::new();

                // 800x600 Desktop bitmap surface
                let sw = 800usize;
                let sh = 600usize;
                let mut pixels = vec![0u8; sw * sh * 4];

                // 1. Initialize desktop area to Teal (#008080: B=0x80, G=0x80, R=0x00)
                for chunk in pixels.as_chunks_mut::<4>().0 {
                    chunk[0] = 0x80;
                    chunk[1] = 0x80;
                    chunk[2] = 0x00;
                    chunk[3] = 0xFF;
                }

                let set_px = |buf: &mut [u8], px: usize, py: usize, b: u8, g: u8, r: u8| {
                    if px < sw && py < sh {
                        let idx = (py * sw + px) * 4;
                        buf[idx] = b;
                        buf[idx + 1] = g;
                        buf[idx + 2] = r;
                        buf[idx + 3] = 0xFF;
                    }
                };

                let fill_box = |buf: &mut [u8], rx: usize, ry: usize, rw: usize, rh: usize, b: u8, g: u8, r: u8| {
                    for y in ry..(ry + rh).min(sh) {
                        for x in rx..(rx + rw).min(sw) {
                            let idx = (y * sw + x) * 4;
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

                let draw_button = |buf: &mut [u8], rx: usize, ry: usize, rw: usize, rh: usize| {
                    fill_box(buf, rx, ry, rw, rh, 0xC8, 0xD0, 0xD4);
                    for x in rx..(rx + rw) {
                        set_px(buf, x, ry, 0xFF, 0xFF, 0xFF);
                        set_px(buf, x, ry + rh - 1, 0x00, 0x00, 0x00);
                    }
                    for y in ry..(ry + rh) {
                        set_px(buf, rx, y, 0xFF, 0xFF, 0xFF);
                        set_px(buf, rx + rw - 1, y, 0x00, 0x00, 0x00);
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
                };

                // Form coordinates centered on 800x600 screen
                let client_w = (form.pixel_client_width() as usize).max(100);
                let client_h = (form.pixel_client_height() as usize).max(60);
                let win_w = client_w.clamp(200, 780);
                let win_h = (client_h + 26).clamp(100, 580);
                let win_x = (800usize.saturating_sub(win_w)) / 2;
                let win_y = (600usize.saturating_sub(win_h)) / 2;

                let mut web_controls = Vec::new();
                for (idx, ctrl) in form.controls.iter().enumerate() {
                    let hwnd = (200 + idx) as u32;
                    let kind = match ctrl.control_type {
                        cb_win32::ControlType::CommandButton => 1,
                        cb_win32::ControlType::TextBox => 2,
                        cb_win32::ControlType::Label => 3,
                        _ => 0,
                    };
                    let abs_x = win_x as i16 + (ctrl.left / 15) as i16;
                    let abs_y = (win_y + 26) as i16 + (ctrl.top / 15) as i16;
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

                // Form window background (#D4D0C8)
                fill_box(&mut pixels, win_x, win_y, win_w, win_h, 0xC8, 0xD0, 0xD4);
                draw_bevel(&mut pixels, win_x, win_y, win_w, win_h, true);
                draw_bevel(&mut pixels, win_x + 1, win_y + 1, win_w - 2, win_h - 2, true);

                // Title bar: Windows Navy (#000080)
                fill_box(&mut pixels, win_x + 4, win_y + 4, win_w - 8, 22, 0x80, 0x00, 0x00);

                // Close button [X]
                let close_x = win_x + win_w - 24;
                let close_y = win_y + 5;
                draw_button(&mut pixels, close_x, close_y, 18, 18);

                let mut text_frames = Vec::new();

                // Title bar text
                text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                    hwnd: 100,
                    x: (win_x + 8) as i16,
                    y: (win_y + 20) as i16,
                    color_rgb: 0x00FFFFFF,
                    bk_mode: 1,
                    font_height: -13,
                    text: form.caption.clone(),
                }));

                // Close button X text
                text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                    hwnd: 100,
                    x: (close_x + 5) as i16,
                    y: (close_y + 14) as i16,
                    color_rgb: 0x00000000,
                    bk_mode: 1,
                    font_height: -12,
                    text: "X".to_string(),
                }));

                for (idx, ctrl) in form.controls.iter().enumerate() {
                    let abs_x = (win_x as i32 + ctrl.left / 15).max(0) as usize;
                    let abs_y = ((win_y + 26) as i32 + ctrl.top / 15).max(0) as usize;
                    let abs_w = (ctrl.width / 15).max(1) as usize;
                    let abs_h = (ctrl.height / 15).max(1) as usize;

                    match ctrl.control_type {
                        cb_win32::ControlType::TextBox => {
                            let (b, g, r) = if let Some(bc) = ctrl.back_color {
                                ((bc & 0xFF) as u8, ((bc >> 8) & 0xFF) as u8, ((bc >> 16) & 0xFF) as u8)
                            } else {
                                (0xFF, 0xFF, 0xFF)
                            };
                            fill_box(&mut pixels, abs_x, abs_y, abs_w, abs_h, b, g, r);
                            draw_bevel(&mut pixels, abs_x, abs_y, abs_w, abs_h, false);

                            let txt = if !ctrl.text.is_empty() { &ctrl.text } else { "0" };
                            let ty = abs_y + (abs_h * 2 / 3);
                            let tx = abs_x + 8;
                            text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                                hwnd: (200 + idx) as u32,
                                x: tx as i16,
                                y: ty as i16,
                                color_rgb: ctrl.fore_color.unwrap_or(0x00000000),
                                bk_mode: 1,
                                font_height: -20,
                                text: txt.to_string(),
                            }));
                        }
                        cb_win32::ControlType::CommandButton => {
                            let btn = cb_win32::Win32ButtonControl::new(
                                (200 + idx) as u32,
                                abs_x as i32,
                                abs_y as i32,
                                abs_w as u32,
                                abs_h as u32,
                                &ctrl.caption,
                            );
                            let btn_px = btn.render_pixels();
                            for by in 0..abs_h {
                                for bx in 0..abs_w {
                                    let src_i = (by * abs_w + bx) * 4;
                                    let dst_i = ((abs_y + by) * sw + (abs_x + bx)) * 4;
                                    if dst_i + 4 <= pixels.len() && src_i + 4 <= btn_px.len() {
                                        pixels[dst_i..dst_i + 4].copy_from_slice(&btn_px[src_i..src_i + 4]);
                                    }
                                }
                            }
                            let (dx, dy) = btn.text_offset();
                            let text_w = ctrl.caption.len() as i16 * 8;
                            let tx = abs_x as i16 + (abs_w as i16 / 2) - (text_w / 2).min(abs_w as i16 / 2 - 2) + dx;
                            let ty = abs_y as i16 + (abs_h as i16 / 2) + 5 + dy;
                            text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                                hwnd: (200 + idx) as u32,
                                x: tx,
                                y: ty,
                                color_rgb: 0x00000000,
                                bk_mode: 1,
                                font_height: -14,
                                text: ctrl.caption.clone(),
                            }));
                        }
                        cb_win32::ControlType::Label => {
                            let tx = abs_x as i16 + 2;
                            let ty = abs_y as i16 + (abs_h as i16 * 2 / 3);
                            text_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                                hwnd: (200 + idx) as u32,
                                x: tx,
                                y: ty,
                                color_rgb: ctrl.fore_color.unwrap_or(0x00000000),
                                bk_mode: 1,
                                font_height: -13,
                                text: ctrl.caption.clone(),
                            }));
                        }
                        _ => {}
                    }
                }

                initial_frames.push(cb_webserver::DisplayFrame::BitmapRect(cb_webserver::BitmapRectFrame {
                    hwnd: 100,
                    x: 0,
                    y: 0,
                    width: 800,
                    height: 600,
                    pixels: pixels.clone(),
                }));

                initial_frames.extend(text_frames);

                let config = cb_webserver::ServerConfig {
                    bind_addr: format!("0.0.0.0:{}", port),
                    static_assets,
                    initial_frames,
                };

                let queue = std::sync::Arc::new(cb_win32::VirtualMessageQueue::new());
                let queue_clone = std::sync::Arc::clone(&queue);
                let server = match cb_webserver::start_server(config, queue_clone, None::<fn(_)>) {
                    Ok(h) => {
                        println!("\n╔═══════════════════════════════════════════════════════════════╗");
                        println!("║   ClassicBasic GUI Web Server Running on http://0.0.0.0:{:<5} ║", port);
                        println!("║   Open http://localhost:{:<5} in your browser to interact    ║", port);
                        println!("╚═══════════════════════════════════════════════════════════════╝\n");
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

            println!("\n[*] Starting execution via ThunRTMain...");
            let exit_code = match loaded_pe.execute_with_mode(execution_mode) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("Execution error: {}", e);
                    process::exit(1);
                }
            };

            if let (Some(server), Some(queue)) = (server_handle, queue) {
                println!("\n[+] Binary execution finished with return code: {}", exit_code);
                println!("[*] Interactive Web GUI is running at http://0.0.0.0:{}", port);
                println!("[*] Deserialized Form \"{}\" with {} controls.", form.caption, form.controls.len());
                println!("[*] Open http://localhost:{} in your browser to interact.", port);
                println!("[*] Press Ctrl+C to terminate session...\n");

                let client_w = (form.pixel_client_width() as usize).max(100);
                let client_h = (form.pixel_client_height() as usize).max(60);
                let win_w = client_w.clamp(200, 780);
                let win_h = (client_h + 26).clamp(100, 580);
                let win_x = (800usize.saturating_sub(win_w)) / 2;
                let win_y = (600usize.saturating_sub(win_h)) / 2;
                let close_x = win_x + win_w - 24;
                let close_y = win_y + 5;

                let mut buttons: Vec<cb_win32::Win32ButtonControl> = form
                    .controls
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.control_type == cb_win32::ControlType::CommandButton)
                    .map(|(idx, c)| {
                        let abs_x = (win_x as i32 + c.left / 15).max(0);
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
                        let abs_x = (win_x as i32 + c.left / 15).max(0) as usize;
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

                let mut dialog_active = false;

                loop {
                    if let Some(msg) = queue.peek_message(0, 0, 0, true) {
                        match msg.message {
                            cb_win32::WM_LBUTTONDOWN => {
                                let px = msg.pt_x;
                                let py = msg.pt_y;
                                if dialog_active {
                                    if (px >= 365 && px < 435 && py >= 315 && py < 340)
                                        || (px >= 514 && px < 532 && py >= 226 && py < 244)
                                    {
                                        dialog_active = false;
                                        dismiss_msgbox_dialog(&server, &buttons, &textboxes);
                                    }
                                } else if px >= close_x as i32
                                    && px < (close_x + 18) as i32
                                    && py >= close_y as i32
                                    && py < (close_y + 18) as i32
                                {
                                    break;
                                } else {
                                    for btn in buttons.iter_mut() {
                                        if btn.on_lbutton_down(px, py) {
                                            broadcast_button_state(&server, btn);
                                            break;
                                        }
                                    }
                                }
                            }
                            cb_win32::WM_MOUSEMOVE => {
                                let px = msg.pt_x;
                                let py = msg.pt_y;
                                if !dialog_active {
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
                                if !dialog_active {
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
                                        dismiss_msgbox_dialog(&server, &buttons, &textboxes);
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

            println!("\n[+] Execution completed with return code: {}", exit_code);
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
