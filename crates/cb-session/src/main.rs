// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! `cb-session`: Multi-tenant session broker & container reverse proxy CLI.

use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process;

use cb_session::{SessionBrokerConfig, SessionBrokerServer, SessionManager};

fn print_usage(prog: &str) {
    println!("ClassicBasic Multi-Tenant Session Broker (Phase 3.9)");
    println!("Usage: {} [OPTIONS]", prog);
    println!();
    println!("Options:");
    println!("  --bind <ADDR:PORT>    Bind address and port (default: 127.0.0.1:8080)");
    println!("  --runner <PATH>       Path to vb6-run executable binary");
    println!("  --app <PATH>          Default VB6 application executable path");
    println!("  --sandbox-dir <PATH>  Root directory for tenant sandboxes (default: /tmp/classicbasic-sandboxes)");
    println!("  --max-sessions <NUM>  Maximum concurrent active sessions (default: 128)");
    println!("  --auth-token <TOKEN>  Shared secret token for authentication");
    println!("  --help, -h            Show this help text");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut bind_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let mut runner_path = PathBuf::from("target/release/vb6-run");
    let mut default_app = PathBuf::new();
    let mut sandbox_dir = PathBuf::from("/tmp/classicbasic-sandboxes");
    let mut max_sessions = 128usize;
    let mut auth_token = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--bind" => {
                i += 1;
                if i < args.len() {
                    if let Ok(addr) = args[i].parse() {
                        bind_addr = addr;
                    }
                }
            }
            "--runner" => {
                i += 1;
                if i < args.len() {
                    runner_path = PathBuf::from(&args[i]);
                }
            }
            "--app" => {
                i += 1;
                if i < args.len() {
                    default_app = PathBuf::from(&args[i]);
                }
            }
            "--sandbox-dir" => {
                i += 1;
                if i < args.len() {
                    sandbox_dir = PathBuf::from(&args[i]);
                }
            }
            "--max-sessions" => {
                i += 1;
                if i < args.len() {
                    if let Ok(num) = args[i].parse() {
                        max_sessions = num;
                    }
                }
            }
            "--auth-token" => {
                i += 1;
                if i < args.len() {
                    auth_token = Some(args[i].clone());
                }
            }
            "--help" | "-h" => {
                print_usage(&args[0]);
                process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_usage(&args[0]);
                process::exit(1);
            }
        }
        i += 1;
    }

    let mut config = SessionBrokerConfig::default();
    config.listen_addr = bind_addr;
    config.base_sandbox_dir = sandbox_dir;
    config.default_binary_path = default_app;
    config.max_sessions = max_sessions;
    config.master_secret = auth_token;

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       ClassicBasic: Multi-Tenant Session Broker (v0.3.9)      ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!("[*] Binding Session Broker on: {}", config.listen_addr);
    println!("[*] Tenant Sandbox Base: {}", config.base_sandbox_dir.display());
    println!("[*] Max Concurrent Sessions: {}", config.max_sessions);

    let manager = SessionManager::new(config.clone(), runner_path);
    let server = SessionBrokerServer::new(manager, bind_addr);

    match server.start() {
        Ok(handle) => {
            println!("[+] Session Broker successfully listening on http://{}", handle.local_addr());
            // Keep main thread alive
            let (tx, rx) = std::sync::mpsc::channel::<()>();
            ctrlc_or_signal(tx);
            let _ = rx.recv();
            println!("[*] Shutting down Session Broker...");
            handle.shutdown();
        }
        Err(e) => {
            eprintln!("[-] Failed to start Session Broker: {}", e);
            process::exit(1);
        }
    }
}

fn ctrlc_or_signal(tx: std::sync::mpsc::Sender<()>) {
    // Basic fallback channel keepalive
    std::thread::spawn(move || {
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        let _ = tx.send(());
    });
}
