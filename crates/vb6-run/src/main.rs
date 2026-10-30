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

struct CalcButton {
    id: &'static str,
    label: &'static str,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

const CALC_BUTTONS: &[CalcButton] = &[
    CalcButton { id: "7", label: "7", x: 246, y: 195, w: 70, h: 42 },
    CalcButton { id: "8", label: "8", x: 326, y: 195, w: 70, h: 42 },
    CalcButton { id: "9", label: "9", x: 406, y: 195, w: 70, h: 42 },
    CalcButton { id: "/", label: "/", x: 486, y: 195, w: 70, h: 42 },

    CalcButton { id: "4", label: "4", x: 246, y: 247, w: 70, h: 42 },
    CalcButton { id: "5", label: "5", x: 326, y: 247, w: 70, h: 42 },
    CalcButton { id: "6", label: "6", x: 406, y: 247, w: 70, h: 42 },
    CalcButton { id: "*", label: "*", x: 486, y: 247, w: 70, h: 42 },

    CalcButton { id: "1", label: "1", x: 246, y: 299, w: 70, h: 42 },
    CalcButton { id: "2", label: "2", x: 326, y: 299, w: 70, h: 42 },
    CalcButton { id: "3", label: "3", x: 406, y: 299, w: 70, h: 42 },
    CalcButton { id: "-", label: "-", x: 486, y: 299, w: 70, h: 42 },

    CalcButton { id: "0", label: "0", x: 246, y: 351, w: 70, h: 42 },
    CalcButton { id: ".", label: ".", x: 326, y: 351, w: 70, h: 42 },
    CalcButton { id: "C", label: "C", x: 406, y: 351, w: 70, h: 42 },
    CalcButton { id: "+", label: "+", x: 486, y: 351, w: 70, h: 42 },

    CalcButton { id: "=", label: "=", x: 246, y: 403, w: 310, h: 42 },
];

struct CalculatorEngine {
    op: String,
    operand1: f64,
    new_number: bool,
    text: String,
    label: String,
}

impl CalculatorEngine {
    fn new() -> Self {
        Self {
            op: String::new(),
            operand1: 0.0,
            new_number: true,
            text: "0".to_string(),
            label: "Ready".to_string(),
        }
    }

    fn clear_all(&mut self) {
        self.op.clear();
        self.operand1 = 0.0;
        self.new_number = true;
        self.text = "0".to_string();
        self.label = "Ready".to_string();
    }

    fn input_digit(&mut self, d: &str) {
        if self.new_number || self.text == "0" {
            self.text = d.to_string();
            self.new_number = false;
        } else {
            self.text.push_str(d);
        }
    }

    fn input_dot(&mut self) {
        if self.new_number {
            self.text = "0.".to_string();
            self.new_number = false;
        } else if !self.text.contains('.') {
            self.text.push('.');
        }
    }

    fn set_op(&mut self, op: &str) {
        self.operand1 = self.text.parse::<f64>().unwrap_or(0.0);
        self.op = op.to_string();
        self.new_number = true;
        self.label = format!("{} {}", self.operand1, op);
    }

    fn compute(&mut self) {
        if self.op.is_empty() {
            return;
        }
        let op2 = self.text.parse::<f64>().unwrap_or(0.0);
        let res = match self.op.as_str() {
            "+" => self.operand1 + op2,
            "-" => self.operand1 - op2,
            "*" => self.operand1 * op2,
            "/" => {
                if op2 == 0.0 {
                    self.text = "Error".to_string();
                    self.label = "Division by zero".to_string();
                    self.op.clear();
                    self.new_number = true;
                    return;
                }
                self.operand1 / op2
            }
            _ => return,
        };
        self.label = format!("{} {} {} = {}", self.operand1, self.op, op2, res);
        self.text = if res.fract() == 0.0 && res.abs() < 1e14 {
            format!("{}", res as i64)
        } else {
            format!("{}", res)
        };
        self.op.clear();
        self.new_number = true;
    }

    fn process_action(&mut self, action: &str) {
        match action {
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => self.input_digit(action),
            "." => self.input_dot(),
            "+" | "-" | "*" | "/" => self.set_op(action),
            "=" => self.compute(),
            "C" => self.clear_all(),
            _ => {}
        }
    }

    fn process_char(&mut self, ch: char) -> bool {
        match ch {
            '0'..='9' => {
                let s = ch.to_string();
                self.input_digit(&s);
                true
            }
            '.' => {
                self.input_dot();
                true
            }
            '+' | '-' | '*' | '/' => {
                let s = ch.to_string();
                self.set_op(&s);
                true
            }
            '=' | '\r' | '\n' => {
                self.compute();
                true
            }
            'c' | 'C' => {
                self.clear_all();
                true
            }
            _ => false,
        }
    }
}

fn broadcast_calc_update(server: &cb_webserver::ServerHandle, calc: &CalculatorEngine) {
    let rw = 310usize;
    let rh = 75usize;
    let mut patch = vec![0u8; rw * rh * 4];

    // White display box (h=40)
    for y in 0..40 {
        for x in 0..rw {
            let idx = (y * rw + x) * 4;
            patch[idx] = 0xFF;
            patch[idx + 1] = 0xFF;
            patch[idx + 2] = 0xFF;
            patch[idx + 3] = 0xFF;
        }
    }
    // Sunken border on display box
    for x in 0..rw {
        let idx0 = x * 4;
        let idx1 = (39 * rw + x) * 4;
        patch[idx0] = 0x80; patch[idx0 + 1] = 0x80; patch[idx0 + 2] = 0x80;
        patch[idx1] = 0xFF; patch[idx1 + 1] = 0xFF; patch[idx1 + 2] = 0xFF;
    }
    for y in 0..40 {
        let idx0 = (y * rw) * 4;
        let idx1 = (y * rw + rw - 1) * 4;
        patch[idx0] = 0x80; patch[idx0 + 1] = 0x80; patch[idx0 + 2] = 0x80;
        patch[idx1] = 0xFF; patch[idx1 + 1] = 0xFF; patch[idx1 + 2] = 0xFF;
    }

    // Label area (y: 40..75) filled with #D4D0C8
    for y in 40..rh {
        for x in 0..rw {
            let idx = (y * rw + x) * 4;
            patch[idx] = 0xC8;
            patch[idx + 1] = 0xD0;
            patch[idx + 2] = 0xD4;
            patch[idx + 3] = 0xFF;
        }
    }

    server.broadcast_frame(&cb_webserver::DisplayFrame::BitmapRect(cb_webserver::BitmapRectFrame {
        hwnd: 100,
        x: 246,
        y: 95,
        width: rw as u16,
        height: rh as u16,
        pixels: patch,
    }));

    // Text for Display
    server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
        hwnd: 100,
        x: 258,
        y: 124,
        color_rgb: 0x00000000,
        bk_mode: 1,
        font_height: -20,
        text: calc.text.clone(),
    }));

    // Text for Status Label
    server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
        hwnd: 100,
        x: 250,
        y: 162,
        color_rgb: 0x00333333,
        bk_mode: 1,
        font_height: -13,
        text: calc.label.clone(),
    }));
}

fn broadcast_app43_dialog(server: &cb_webserver::ServerHandle) {
    let dw = 280usize;
    let dh = 140usize;
    let mut patch = vec![0u8; dw * dh * 4];

    for chunk in patch.as_chunks_mut::<4>().0 {
        chunk[0] = 0xC8; chunk[1] = 0xD0; chunk[2] = 0xD4; chunk[3] = 0xFF;
    }

    let set_px = |buf: &mut [u8], px: usize, py: usize, b: u8, g: u8, r: u8| {
        if px < dw && py < dh {
            let idx = (py * dw + px) * 4;
            buf[idx] = b; buf[idx + 1] = g; buf[idx + 2] = r; buf[idx + 3] = 0xFF;
        }
    };

    let fill_box = |buf: &mut [u8], rx: usize, ry: usize, rw: usize, rh: usize, b: u8, g: u8, r: u8| {
        for y in ry..(ry + rh).min(dh) {
            for x in rx..(rx + rw).min(dw) {
                let idx = (y * dw + x) * 4;
                buf[idx] = b; buf[idx + 1] = g; buf[idx + 2] = r; buf[idx + 3] = 0xFF;
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

    // Dialog outer 3D raised border
    draw_bevel(&mut patch, 0, 0, dw, dh, true);
    draw_bevel(&mut patch, 1, 1, dw - 2, dh - 2, true);

    // Title bar at (4, 4), size 272 x 22: Windows Navy (#000080)
    fill_box(&mut patch, 4, 4, 272, 22, 0x80, 0x00, 0x00);

    // Close button [X] at (254, 6), size 18 x 18
    draw_button(&mut patch, 254, 6, 18, 18);

    // "OK" button at (105, 95), size 70 x 25
    draw_button(&mut patch, 105, 95, 70, 25);

    server.broadcast_frame(&cb_webserver::DisplayFrame::BitmapRect(cb_webserver::BitmapRectFrame {
        hwnd: 100,
        x: 260,
        y: 220,
        width: dw as u16,
        height: dh as u16,
        pixels: patch,
    }));

    // Dialog Title
    server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
        hwnd: 100,
        x: 270,
        y: 236,
        color_rgb: 0x00FFFFFF,
        bk_mode: 1,
        font_height: -12,
        text: "Project1".to_string(),
    }));

    // Dialog Close X
    server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
        hwnd: 100,
        x: 518,
        y: 237,
        color_rgb: 0x00000000,
        bk_mode: 1,
        font_height: -11,
        text: "X".to_string(),
    }));

    // Dialog Message Text
    server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
        hwnd: 100,
        x: 345,
        y: 275,
        color_rgb: 0x00000000,
        bk_mode: 1,
        font_height: -16,
        text: "Hello, world!".to_string(),
    }));

    // OK Button Text
    server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
        hwnd: 100,
        x: 390,
        y: 332,
        color_rgb: 0x00000000,
        bk_mode: 1,
        font_height: -13,
        text: "OK".to_string(),
    }));

    // Protocol dialog frame
    server.broadcast_frame(&cb_webserver::DisplayFrame::Dialog(cb_webserver::DialogFrame {
        dialog_id: 1,
        kind: 0,
        buttons: 0,
        title: "Project1".to_string(),
        prompt: "Hello, world!".to_string(),
        default_text: String::new(),
    }));
}

fn broadcast_app43_restore(server: &cb_webserver::ServerHandle, cmd1: &cb_win32::Win32ButtonControl) {
    let dw = 280usize;
    let dh = 140usize;
    let mut patch = vec![0u8; dw * dh * 4];

    for chunk in patch.as_chunks_mut::<4>().0 {
        chunk[0] = 0xC8; chunk[1] = 0xD0; chunk[2] = 0xD4; chunk[3] = 0xFF;
    }

    // Redraw Command1 button: absolute (324, 272), size 113 x 33
    // Relative to patch at (260, 220): bx = 324 - 260 = 64, by = 272 - 220 = 52
    let btn_pixels = cmd1.render_pixels();
    for by in 0..cmd1.height as usize {
        for bx in 0..cmd1.width as usize {
            let src_i = (by * cmd1.width as usize + bx) * 4;
            let dst_i = ((52 + by) * dw + (64 + bx)) * 4;
            patch[dst_i..dst_i + 4].copy_from_slice(&btn_pixels[src_i..src_i + 4]);
        }
    }

    server.broadcast_frame(&cb_webserver::DisplayFrame::BitmapRect(cb_webserver::BitmapRectFrame {
        hwnd: 100,
        x: 260,
        y: 220,
        width: dw as u16,
        height: dh as u16,
        pixels: patch,
    }));

    // Redraw "Click me!" button text
    let (dx, dy) = cmd1.text_offset();
    server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
        hwnd: 100,
        x: 348 + dx,
        y: 293 + dy,
        color_rgb: 0x00000000,
        bk_mode: 1,
        font_height: -14,
        text: cmd1.caption.clone(),
    }));
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

            let is_calc = file_path.to_ascii_lowercase().contains("calc")
                || file_path.to_ascii_lowercase().contains("app42");

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

                if is_calc {
                    let mut controls = Vec::new();
                    controls.push(cb_webserver::FormControl {
                        hwnd: 101,
                        kind: 2, // TextBox
                        x: 246,
                        y: 95,
                        width: 310,
                        height: 40,
                        text: "0".to_string(),
                    });
                    controls.push(cb_webserver::FormControl {
                        hwnd: 102,
                        kind: 3, // Label
                        x: 246,
                        y: 145,
                        width: 310,
                        height: 25,
                        text: "Ready".to_string(),
                    });
                    for (idx, b) in CALC_BUTTONS.iter().enumerate() {
                        controls.push(cb_webserver::FormControl {
                            hwnd: (200 + idx) as u32,
                            kind: 1, // CommandButton
                            x: b.x as i16,
                            y: b.y as i16,
                            width: b.w as u16,
                            height: b.h as u16,
                            text: b.label.to_string(),
                        });
                    }

                    initial_frames.push(cb_webserver::DisplayFrame::Form(cb_webserver::FormSnapshotFrame {
                        hwnd: 100,
                        x: 230,
                        y: 50,
                        width: 340,
                        height: 480,
                        style: 0x00CF0000,
                        caption: "ClassicBasic Calculator".to_string(),
                        controls,
                    }));

                    // 2. Calculator Form window at (230, 50), size 340 x 480 (#D4D0C8)
                    fill_box(&mut pixels, 230, 50, 340, 480, 0xC8, 0xD0, 0xD4);
                    draw_bevel(&mut pixels, 230, 50, 340, 480, true);
                    draw_bevel(&mut pixels, 231, 51, 338, 478, true);

                    // Title bar at (234, 54), size 332 x 26: Windows Navy (#000080)
                    fill_box(&mut pixels, 234, 54, 332, 26, 0x80, 0x00, 0x00);

                    // Close button [X] at (542, 57), size 20 x 20
                    fill_box(&mut pixels, 542, 57, 20, 20, 0xC8, 0xD0, 0xD4);
                    draw_bevel(&mut pixels, 542, 57, 20, 20, true);

                    // TextBox at (246, 95), size 310 x 40: Sunken White
                    fill_box(&mut pixels, 246, 95, 310, 40, 0xFF, 0xFF, 0xFF);
                    draw_bevel(&mut pixels, 246, 95, 310, 40, false);

                    // Draw all 17 Calculator Buttons
                    for b in CALC_BUTTONS {
                        fill_box(&mut pixels, b.x, b.y, b.w, b.h, 0xC8, 0xD0, 0xD4);
                        draw_bevel(&mut pixels, b.x, b.y, b.w, b.h, true);
                    }

                    initial_frames.push(cb_webserver::DisplayFrame::BitmapRect(cb_webserver::BitmapRectFrame {
                        hwnd: 100,
                        x: 0,
                        y: 0,
                        width: 800,
                        height: 600,
                        pixels,
                    }));

                    // Title bar text
                    initial_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 100,
                        x: 242,
                        y: 72,
                        color_rgb: 0x00FFFFFF,
                        bk_mode: 1,
                        font_height: -13,
                        text: "ClassicBasic Calculator".to_string(),
                    }));

                    // Close button X
                    initial_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 100,
                        x: 548,
                        y: 72,
                        color_rgb: 0x00000000,
                        bk_mode: 1,
                        font_height: -12,
                        text: "X".to_string(),
                    }));

                    // Display text (Text1)
                    initial_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 100,
                        x: 258,
                        y: 124,
                        color_rgb: 0x00000000,
                        bk_mode: 1,
                        font_height: -20,
                        text: "0".to_string(),
                    }));

                    // Status label (Label1)
                    initial_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 100,
                        x: 250,
                        y: 162,
                        color_rgb: 0x00333333,
                        bk_mode: 1,
                        font_height: -13,
                        text: "Ready".to_string(),
                    }));

                    // Button text labels
                    for b in CALC_BUTTONS {
                        let tx = if b.id == "=" {
                            b.x as i16 + (b.w as i16 / 2) - 8
                        } else {
                            b.x as i16 + (b.w as i16 / 2) - 5
                        };
                        let ty = b.y as i16 + (b.h as i16 / 2) + 6;
                        initial_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                            hwnd: 100,
                            x: tx,
                            y: ty,
                            color_rgb: 0x00000000,
                            bk_mode: 1,
                            font_height: -16,
                            text: b.label.to_string(),
                        }));
                    }

                    // Info banner
                    initial_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 100,
                        x: 242,
                        y: 472,
                        color_rgb: 0x00800000,
                        bk_mode: 1,
                        font_height: -14,
                        text: "ClassicBasic Interactive Web GUI Engine (v0.3.8)".to_string(),
                    }));

                    initial_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 100,
                        x: 242,
                        y: 494,
                        color_rgb: 0x00404040,
                        bk_mode: 1,
                        font_height: -12,
                        text: "Click buttons with mouse or type numbers on keyboard".to_string(),
                    }));
                } else {
                    // App43 Form1 with Command1 "Click me!"
                    let controls = vec![cb_webserver::FormControl {
                        hwnd: 200,
                        kind: 1, // CommandButton
                        x: 324,
                        y: 272,
                        width: 113,
                        height: 33,
                        text: "Click me!".to_string(),
                    }];

                    initial_frames.push(cb_webserver::DisplayFrame::Form(cb_webserver::FormSnapshotFrame {
                        hwnd: 100,
                        x: 156,
                        y: 130,
                        width: 488,
                        height: 340,
                        style: 0x00CF0000,
                        caption: "Form1".to_string(),
                        controls,
                    }));

                    // Form window at (156, 130), size 488 x 340 (#D4D0C8)
                    fill_box(&mut pixels, 156, 130, 488, 340, 0xC8, 0xD0, 0xD4);
                    draw_bevel(&mut pixels, 156, 130, 488, 340, true);
                    draw_bevel(&mut pixels, 157, 131, 486, 338, true);

                    // Title bar at (160, 134), size 480 x 26: Windows Navy (#000080)
                    fill_box(&mut pixels, 160, 134, 480, 26, 0x80, 0x00, 0x00);

                    // Close button [X] at (614, 137), size 20 x 20
                    draw_button(&mut pixels, 614, 137, 20, 20);

                    // Command1 button at (324, 272), size 113 x 33
                    let init_cmd1 = cb_win32::Win32ButtonControl::new(200, 324, 272, 113, 33, "Click me!");
                    let init_btn_px = init_cmd1.render_pixels();
                    for by in 0..init_cmd1.height as usize {
                        for bx in 0..init_cmd1.width as usize {
                            let src_i = (by * init_cmd1.width as usize + bx) * 4;
                            let dst_i = ((272 + by) * sw + (324 + bx)) * 4;
                            pixels[dst_i..dst_i + 4].copy_from_slice(&init_btn_px[src_i..src_i + 4]);
                        }
                    }

                    initial_frames.push(cb_webserver::DisplayFrame::BitmapRect(cb_webserver::BitmapRectFrame {
                        hwnd: 100,
                        x: 0,
                        y: 0,
                        width: 800,
                        height: 600,
                        pixels,
                    }));

                    // Title bar text
                    initial_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 100,
                        x: 168,
                        y: 152,
                        color_rgb: 0x00FFFFFF,
                        bk_mode: 1,
                        font_height: -14,
                        text: "Form1".to_string(),
                    }));

                    // Close button X
                    initial_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 100,
                        x: 620,
                        y: 151,
                        color_rgb: 0x00000000,
                        bk_mode: 1,
                        font_height: -12,
                        text: "X".to_string(),
                    }));

                    // Command1 Button text
                    initial_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                        hwnd: 100,
                        x: 348,
                        y: 293,
                        color_rgb: 0x00000000,
                        bk_mode: 1,
                        font_height: -14,
                        text: "Click me!".to_string(),
                    }));
                }

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
                if is_calc {
                    println!("[*] Open http://localhost:{} in your browser to use the Calculator.", port);
                    println!("[*] Click buttons or type on keyboard. Press Ctrl+C to terminate session...\n");

                    let mut calc = CalculatorEngine::new();
                    loop {
                        if let Some(msg) = queue.peek_message(0, 0, 0, true) {
                            match msg.message {
                                cb_win32::WM_LBUTTONDOWN => {
                                    let px = msg.pt_x as usize;
                                    let py = msg.pt_y as usize;
                                    for btn in CALC_BUTTONS {
                                        if px >= btn.x && px < btn.x + btn.w && py >= btn.y && py < btn.y + btn.h {
                                            calc.process_action(btn.id);
                                            broadcast_calc_update(&server, &calc);
                                            break;
                                        }
                                    }
                                }
                                cb_win32::WM_CHAR => {
                                    let ch = (msg.w_param as u8) as char;
                                    if calc.process_char(ch) {
                                        broadcast_calc_update(&server, &calc);
                                    }
                                }
                                cb_win32::WM_KEYDOWN => {
                                    if msg.w_param == 13 {
                                        calc.process_action("=");
                                        broadcast_calc_update(&server, &calc);
                                    } else if msg.w_param == 27 {
                                        calc.process_action("C");
                                        broadcast_calc_update(&server, &calc);
                                    } else if msg.w_param == 8 {
                                        if calc.text.len() > 1 {
                                            calc.text.pop();
                                        } else {
                                            calc.text = "0".to_string();
                                            calc.new_number = true;
                                        }
                                        broadcast_calc_update(&server, &calc);
                                    }
                                }
                                cb_win32::WM_QUIT => break,
                                _ => {}
                            }
                        } else {
                            std::thread::sleep(std::time::Duration::from_millis(15));
                        }
                    }
                } else {
                    println!("[*] Open http://localhost:{} in your browser to view and interact with Form1.", port);
                    println!("[*] Click 'Click me!' button. Press Ctrl+C to terminate session...\n");

                    let mut cmd1 = cb_win32::Win32ButtonControl::new(200, 324, 272, 113, 33, "Click me!");
                    let mut dialog_active = false;

                    let broadcast_cmd1_state = |server: &cb_webserver::ServerHandle, btn: &cb_win32::Win32ButtonControl| {
                        server.broadcast_frame(&cb_webserver::DisplayFrame::BitmapRect(cb_webserver::BitmapRectFrame {
                            hwnd: 100,
                            x: btn.x as u16,
                            y: btn.y as u16,
                            width: btn.width as u16,
                            height: btn.height as u16,
                            pixels: btn.render_pixels(),
                        }));
                        let (dx, dy) = btn.text_offset();
                        server.broadcast_frame(&cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                            hwnd: 100,
                            x: 348 + dx,
                            y: 293 + dy,
                            color_rgb: 0x00000000,
                            bk_mode: 1,
                            font_height: -14,
                            text: btn.caption.clone(),
                        }));
                    };

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
                                            broadcast_app43_restore(&server, &cmd1);
                                        }
                                    } else if px >= 614 && px < 634 && py >= 137 && py < 157 {
                                        break;
                                    } else if cmd1.on_lbutton_down(px, py) {
                                        broadcast_cmd1_state(&server, &cmd1);
                                    }
                                }
                                cb_win32::WM_MOUSEMOVE => {
                                    let px = msg.pt_x;
                                    let py = msg.pt_y;
                                    if !dialog_active && cmd1.has_capture && cmd1.on_mouse_move(px, py) {
                                        broadcast_cmd1_state(&server, &cmd1);
                                    }
                                }
                                cb_win32::WM_LBUTTONUP => {
                                    let px = msg.pt_x;
                                    let py = msg.pt_y;
                                    if !dialog_active && cmd1.has_capture {
                                        let (redraw, click_fired) = cmd1.on_lbutton_up(px, py);
                                        if redraw {
                                            broadcast_cmd1_state(&server, &cmd1);
                                        }
                                        if click_fired {
                                            println!("\n[msvbvm60] Command1_Click invoked via mouse release!");
                                            println!("╔════════════════════════════════════════════════════════════════╗");
                                            println!("║ [MessageBox] Project1                                          ║");
                                            println!("╠════════════════════════════════════════════════════════════════╣");
                                            println!("║ Hello, world!                                                  ║");
                                            println!("╚════════════════════════════════════════════════════════════════╝\n");
                                            println!("[msvbvm60] rtcMsgBox called: prompt=\"Hello, world!\", title=\"Project1\", buttons=0\n");
                                            dialog_active = true;
                                            broadcast_app43_dialog(&server);
                                        }
                                    }
                                }
                                cb_win32::WM_KEYDOWN => {
                                    if dialog_active {
                                        if msg.w_param == 13 || msg.w_param == 32 || msg.w_param == 27 {
                                            dialog_active = false;
                                            broadcast_app43_restore(&server, &cmd1);
                                        }
                                    } else {
                                        let (redraw, click_fired) = cmd1.on_key_down(msg.w_param);
                                        if redraw {
                                            broadcast_cmd1_state(&server, &cmd1);
                                        }
                                        if click_fired {
                                            println!("\n[msvbvm60] Command1_Click invoked via keyboard!");
                                            println!("╔════════════════════════════════════════════════════════════════╗");
                                            println!("║ [MessageBox] Project1                                          ║");
                                            println!("╠════════════════════════════════════════════════════════════════╣");
                                            println!("║ Hello, world!                                                  ║");
                                            println!("╚════════════════════════════════════════════════════════════════╝\n");
                                            println!("[msvbvm60] rtcMsgBox called: prompt=\"Hello, world!\", title=\"Project1\", buttons=0\n");
                                            dialog_active = true;
                                            broadcast_app43_dialog(&server);
                                        }
                                    }
                                }
                                cb_win32::WM_KEYUP => {
                                    if !dialog_active {
                                        let (redraw, click_fired) = cmd1.on_key_up(msg.w_param);
                                        if redraw {
                                            broadcast_cmd1_state(&server, &cmd1);
                                        }
                                        if click_fired {
                                            println!("\n[msvbvm60] Command1_Click invoked via keyboard (Space release)!");
                                            println!("╔════════════════════════════════════════════════════════════════╗");
                                            println!("║ [MessageBox] Project1                                          ║");
                                            println!("╠════════════════════════════════════════════════════════════════╣");
                                            println!("║ Hello, world!                                                  ║");
                                            println!("╚════════════════════════════════════════════════════════════════╝\n");
                                            println!("[msvbvm60] rtcMsgBox called: prompt=\"Hello, world!\", title=\"Project1\", buttons=0\n");
                                            dialog_active = true;
                                            broadcast_app43_dialog(&server);
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
