// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # Multi-Tenant Session Broker & State Isolation Suite (Phase 3.9)
//!
//! Validates multi-tenant orchestration, concurrent process sandbox isolation,
//! transparent WebSocket reverse proxying, and deterministic session lifecycles.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use cb_session::*;

#[test]
fn test_concurrent_multi_tenant_state_isolation() {
    let tmp_root = std::env::temp_dir().join(format!(
        "cb_isolation_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let mut broker_config = SessionBrokerConfig::default();
    broker_config.base_sandbox_dir = tmp_root.clone();
    broker_config.auth_required = false;

    let manager = SessionManager::new(broker_config, PathBuf::from("vb6-run"));

    // Spawn 10 concurrent tenant sessions
    let tenant_count = 10;
    let mut handles = Vec::new();

    for i in 0..tenant_count {
        let mgr = manager.clone();
        let sid = format!("tenant-{:03}", i);
        let content = format!("[TenantState]\nId={:03}\nCounter={}\nToken=tok_{}", i, i * 42, i);

        let handle = thread::spawn(move || {
            let snap = mgr
                .create_session(&sid, None, None)
                .expect("Failed to create session");
            assert_eq!(snap.session_id, sid);

            // Write state into isolated tenant sandbox
            mgr.with_session(&sid, |tenant| {
                tenant.write_file("data/state.ini", content.as_bytes())?;
                let read_back = tenant.read_file("data/state.ini")?;
                assert_eq!(String::from_utf8_lossy(&read_back), content);
                Ok(())
            })
            .expect("Tenant with_session failed");
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(manager.active_count(), tenant_count);

    // Cross-verify all 10 sandboxes to ensure absolute state separation
    for i in 0..tenant_count {
        let sid = format!("tenant-{:03}", i);
        let expected_content = format!("[TenantState]\nId={:03}\nCounter={}\nToken=tok_{}", i, i * 42, i);

        manager
            .with_session(&sid, |tenant| {
                let bytes = tenant.read_file("data/state.ini")?;
                assert_eq!(String::from_utf8_lossy(&bytes), expected_content);

                // Ensure no neighbor state exists in this sandbox
                for other_i in 0..tenant_count {
                    if other_i != i {
                        let neighbor_token = format!("Token=tok_{}", other_i);
                        assert!(
                            !String::from_utf8_lossy(&bytes).contains(&neighbor_token),
                            "Sandbox leak detected: {} found inside {}",
                            neighbor_token,
                            sid
                        );
                    }
                }
                Ok(())
            })
            .unwrap();
    }

    // Deterministic teardown
    manager.terminate_all().unwrap();
    assert_eq!(manager.active_count(), 0);

    let _ = std::fs::remove_dir_all(&tmp_root);
}

#[test]
fn test_reverse_proxy_bidirectional_tunnel() {
    let tmp_root = std::env::temp_dir().join(format!(
        "cb_proxy_tunnel_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    // 1. Start a mock upstream tenant runner on an ephemeral port
    let mock_upstream = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock upstream");
    let upstream_port = mock_upstream.local_addr().unwrap().port();
    let upstream_running = Arc::new(AtomicBool::new(true));
    let ur_clone = Arc::clone(&upstream_running);

    let upstream_thread = thread::spawn(move || {
        while ur_clone.load(Ordering::SeqCst) {
            match mock_upstream.accept() {
                Ok((mut stream, _)) => {
                    // Echo back with prefix
                    let mut buf = [0u8; 1024];
                    match stream.read(&mut buf) {
                        Ok(n) if n > 0 => {
                            let _ = stream.write_all(b"UPSTREAM-ECHO: ");
                            let _ = stream.write_all(&buf[..n]);
                            let _ = stream.flush();
                        }
                        _ => {}
                    }
                }
                Err(_) => break,
            }
        }
    });

    // 2. Initialize broker manager
    let mut broker_config = SessionBrokerConfig::default();
    broker_config.base_sandbox_dir = tmp_root.clone();
    broker_config.auth_required = false;

    let manager = SessionManager::new(broker_config, PathBuf::from("mock_runner"));
    let server = SessionBrokerServer::new(manager.clone(), "127.0.0.1:0".parse().unwrap());
    let handle = server.start().expect("Failed to start SessionBrokerServer");
    let broker_addr = handle.local_addr();

    // 3. Register a session pointing directly to our mock upstream port and mark Running
    let test_session_id = "tenant-tunnel-test";
    manager
        .create_session(test_session_id, None, None)
        .expect("Failed to create session");

    // Override the session port and status for the test
    manager
        .with_session_mut(test_session_id, |tenant| {
            // Update port to our mock upstream
            tenant.config.port = upstream_port;
            tenant.status = SessionStatus::Running {
                pid: 12345,
                port: upstream_port,
                uptime_secs: 10,
            };
            Ok(())
        })
        .unwrap();

    // 4. Client connects to broker via /session/{id}/ws
    {
        let mut client = TcpStream::connect(broker_addr).expect("Failed to connect to broker");
        let req = format!("GET /session/{}/ws HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n", test_session_id);
        client.write_all(req.as_bytes()).unwrap();

        // Read response from upstream through broker proxy
        let mut resp = [0u8; 1024];
        let n = client.read(&mut resp).expect("Failed to read from broker");
        let resp_str = String::from_utf8_lossy(&resp[..n]);

        assert!(
            resp_str.contains("UPSTREAM-ECHO: GET /session/tenant-tunnel-test/ws"),
            "Expected upstream echo response, got: {}",
            resp_str
        );
    }

    // 5. Cleanup
    upstream_running.store(false, Ordering::SeqCst);
    let _ = TcpStream::connect(format!("127.0.0.1:{}", upstream_port));
    let _ = upstream_thread.join();
    handle.shutdown();
    let _ = std::fs::remove_dir_all(&tmp_root);
}
