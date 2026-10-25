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
                        println!("  Total Objects: {}", { obj_tbl.dw_total_objects_count });
                        println!("  Compiled Objects: {}", {
                            obj_tbl.dw_compiled_objects_count
                        });
                        println!("  Objects in Use: {}", { obj_tbl.dw_objects_in_use_count });
                    }
                }
            }

            if inspect_only || dump_info {
                println!("\n[+] Inspection finished successfully.");
                return;
            }

            let server_handle = if execution_mode == ExecutionMode::GuiWebStream {
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
                initial_frames.push(cb_webserver::DisplayFrame::Form(cb_webserver::FormSnapshotFrame {
                    hwnd: 100,
                    x: 50,
                    y: 50,
                    width: 700,
                    height: 500,
                    style: 0x00CF0000,
                    caption: "ClassicBasic - Form1 (Active Web Canvas)".to_string(),
                    controls: vec![
                        cb_webserver::FormControl {
                            hwnd: 101,
                            kind: 1, // CommandButton
                            x: 30,
                            y: 400,
                            width: 120,
                            height: 35,
                            text: "Apply".to_string(),
                        },
                        cb_webserver::FormControl {
                            hwnd: 102,
                            kind: 2, // TextBox
                            x: 30,
                            y: 50,
                            width: 250,
                            height: 25,
                            text: "Synchronized State".to_string(),
                        },
                        cb_webserver::FormControl {
                            hwnd: 103,
                            kind: 3, // Label
                            x: 30,
                            y: 90,
                            width: 250,
                            height: 20,
                            text: "State: Ready".to_string(),
                        },
                    ],
                }));

                // 640x480 bitmap surface
                let mut pixels = [0xC0, 0xC0, 0xC0, 0xFF].repeat(640 * 480);
                // Windows Navy title bar (0x000080 in BGR is B=0x80, G=0, R=0)
                for y in 0..28 {
                    for x in 0..640 {
                        let idx = (y * 640 + x) * 4;
                        pixels[idx] = 0x80;
                        pixels[idx + 1] = 0x00;
                        pixels[idx + 2] = 0x00;
                        pixels[idx + 3] = 0xFF;
                    }
                }
                initial_frames.push(cb_webserver::DisplayFrame::BitmapRect(cb_webserver::BitmapRectFrame {
                    hwnd: 100,
                    x: 0,
                    y: 0,
                    width: 640,
                    height: 480,
                    pixels,
                }));

                initial_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                    hwnd: 100,
                    x: 10,
                    y: 20,
                    color_rgb: 0x00FFFFFF, // White text on navy title bar
                    bk_mode: 1,
                    font_height: -14,
                    text: "ClassicBasic App41 Web Canvas".to_string(),
                }));

                initial_frames.push(cb_webserver::DisplayFrame::Text(cb_webserver::TextFrame {
                    hwnd: 100,
                    x: 30,
                    y: 80,
                    color_rgb: 0x00000000, // Black text
                    bk_mode: 1,
                    font_height: -14,
                    text: "WebAssembly Canvas Client Connected (Phase 3.7)".to_string(),
                }));

                let config = cb_webserver::ServerConfig {
                    bind_addr: format!("0.0.0.0:{}", port),
                    static_assets,
                    initial_frames,
                };

                let queue = std::sync::Arc::new(cb_win32::VirtualMessageQueue::new());
                match cb_webserver::start_server(config, queue, None::<fn(_)>) {
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
                }
            } else {
                None
            };

            println!("\n[*] Starting execution via ThunRTMain...");
            let exit_code = match loaded_pe.execute_with_mode(execution_mode) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("Execution error: {}", e);
                    process::exit(1);
                }
            };

            if let Some(_server) = server_handle {
                println!("\n[+] Binary execution finished with return code: {}", exit_code);
                println!("[*] Web GUI server is active. Press Ctrl+C to terminate session...");
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
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
