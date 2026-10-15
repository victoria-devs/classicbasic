//! vb6-run: Standalone Visual Basic 6.0 PE Runner and Inspector for Linux / macOS.
//!
//! Loads legacy Win32 VB6 executables into executable memory, resolves imports
//! against msvbvm60 and Win32 shims, and dispatches execution directly.

use std::env;
use std::fs;
use std::path::Path;
use std::process;

use msvbvm60::pe::loader::LoadedPe;
use msvbvm60::types::ProjectInfo;

/// Prints the application banner to standard output.
fn print_banner() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       VB6-RUN: Visual Basic 6.0 Linux / Native Runner        ║");
    println!("║             Powered by Native Rust MSVBVM60 Runtime           ║");
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
    println!("  --help, -h    Display this help message\n");
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

    for arg in &args[1..] {
        match arg.as_str() {
            "--help" | "-h" => {
                print_banner();
                print_usage(&args[0]);
                process::exit(0);
            }
            "--inspect" => inspect_only = true,
            "--dump-info" => dump_info = true,
            "--run" => {}
            other if !other.starts_with("--") => exe_path = Some(other),
            unknown => {
                eprintln!("Unknown option: {}", unknown);
                print_usage(&args[0]);
                process::exit(1);
            }
        }
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

    println!("[*] Parsing PE32 and mapping sections ({} bytes)...", data.len());
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
            println!("    - Build Version: 0x{:04X}, Revision: 0x{:04X}", build, rev);
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
                    let name_cstr = unsafe { std::ffi::CStr::from_ptr(proj_info.lp_project_name as *const i8) };
                    println!("    - Project Name: {}", name_cstr.to_string_lossy());
                }
                if is_valid_ptr(proj_info.lp_project_title) {
                    let title_cstr = unsafe { std::ffi::CStr::from_ptr(proj_info.lp_project_title as *const i8) };
                    println!("    - Project Title: {}", title_cstr.to_string_lossy());
                }
                if is_valid_ptr(proj_info.lp_project_description) {
                    let desc_cstr = unsafe { std::ffi::CStr::from_ptr(proj_info.lp_project_description as *const i8) };
                    println!("    - Description: {}", desc_cstr.to_string_lossy());
                }
            }

            if dump_info {
                println!("\n--- [VBHeader Detailed Dump] ---");
                println!("  Magic: {:?}", std::str::from_utf8(&vb_hdr.sz_magic).unwrap_or("????"));
                println!("  Language DLL: {:?}", std::str::from_utf8(&vb_hdr.sz_language_dll).unwrap_or(""));
                println!("  LCID: 0x{:08X}", { vb_hdr.lcid });
                println!("  Thread Flags: 0x{:08X}", { vb_hdr.dw_thread_flags });
                println!("  GUI Table: 0x{:08X}", { vb_hdr.lp_gui_table });
                println!("  Import Table: 0x{:08X}", { vb_hdr.lp_import_table });
                println!("  Export Table: 0x{:08X}", { vb_hdr.lp_export_table });

                if vb_hdr.lp_project_data != 0 {
                    let proj_info = unsafe { &*(vb_hdr.lp_project_data as *const ProjectInfo) };
                    println!("\n--- [ProjectInfo Dump] ---");
                    println!("  Version: 0x{:08X}", { proj_info.dw_version });
                    println!("  ObjectTable Pointer: 0x{:08X}", { proj_info.lp_object_table });
                    println!("  Code Depot: 0x{:08X}", { proj_info.lp_code_depot });

                    if proj_info.lp_object_table != 0 {
                        let obj_tbl = unsafe { &*(proj_info.lp_object_table as *const msvbvm60::types::ObjectTable) };
                        println!("\n--- [ObjectTable Dump] ---");
                        println!("  Total Objects: {}", { obj_tbl.dw_total_objects_count });
                        println!("  Compiled Objects: {}", { obj_tbl.dw_compiled_objects_count });
                        println!("  Objects in Use: {}", { obj_tbl.dw_objects_in_use_count });
                    }
                }
            }

            if inspect_only || dump_info {
                println!("\n[+] Inspection finished successfully.");
                return;
            }

            println!("\n[*] Starting execution via ThunRTMain...");
            let exit_code = match loaded_pe.execute() {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("Execution error: {}", e);
                    process::exit(1);
                }
            };

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
