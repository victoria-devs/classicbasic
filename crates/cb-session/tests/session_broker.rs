// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Unit & Integration Tests for `cb-session` Multi-Tenant Session Broker

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use cb_session::*;

#[test]
fn test_session_config_initialization() {
    let mut cfg = SessionConfig::new(
        "tenant-001",
        "/var/apps/App01.exe",
        "/var/sandboxes/tenant-001",
    );
    cfg.memory_limit_bytes = 128 * 1024 * 1024;
    cfg = cfg.with_port(18080).with_auth_token("sec-token-123");

    assert_eq!(cfg.session_id, "tenant-001");
    assert_eq!(cfg.memory_limit_bytes, 134217728);
    assert_eq!(cfg.port, 18080);
    assert_eq!(cfg.auth_token.as_deref(), Some("sec-token-123"));
}

#[test]
fn test_tenant_session_sandbox_and_isolation() {
    let tmp_root = std::env::temp_dir().join(format!(
        "cb_test_tenant_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let cfg_a = SessionConfig::new("tenant-alpha", "App.exe", &tmp_root);
    let cfg_b = SessionConfig::new("tenant-beta", "App.exe", &tmp_root);

    let session_a = TenantSession::new(cfg_a).expect("Session A failed to initialize");
    let session_b = TenantSession::new(cfg_b).expect("Session B failed to initialize");

    assert!(session_a.sandbox_path.exists());
    assert!(session_b.sandbox_path.exists());
    assert!(session_a.sandbox_path.join("fs").exists());
    assert!(session_b.sandbox_path.join("fs").exists());

    // Write file in session A
    let test_file = "accounting/ledger.txt";
    session_a
        .write_file(test_file, b"Tenant Alpha Balance: 1000")
        .expect("Session A write failed");

    // Write file in session B with same relative path
    session_b
        .write_file(test_file, b"Tenant Beta Balance: 2000")
        .expect("Session B write failed");

    // Verify complete state isolation: Session A sees Alpha data, Session B sees Beta data
    let read_a = session_a.read_file(test_file).expect("Session A read failed");
    let read_b = session_b.read_file(test_file).expect("Session B read failed");

    assert_eq!(read_a, b"Tenant Alpha Balance: 1000");
    assert_eq!(read_b, b"Tenant Beta Balance: 2000");
    assert_ne!(read_a, read_b);

    // Verify path traversal is strictly prohibited
    assert!(session_a.write_file("../escape.txt", b"malicious").is_err());
    assert!(session_a.read_file("../../etc/passwd").is_err());

    // Cleanup
    let _ = session_a.cleanup_sandbox();
    let _ = session_b.cleanup_sandbox();
    let _ = std::fs::remove_dir_all(&tmp_root);
}

#[test]
fn test_session_manager_lifecycle_and_auth() {
    let tmp_root = std::env::temp_dir().join(format!(
        "cb_test_mgr_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let mut broker_config = SessionBrokerConfig::default();
    broker_config.base_sandbox_dir = tmp_root.clone();
    broker_config.auth_required = true;
    broker_config.master_secret = Some("admin-master-key".to_string());

    let manager = SessionManager::new(broker_config, PathBuf::from("dummy_runner"));

    // Create session 1 with dedicated token
    let s1 = manager
        .create_session("tenant-one", None, Some("user1-key".to_string()))
        .expect("Failed to create tenant-one");
    assert_eq!(s1.session_id, "tenant-one");
    assert!(s1.port > 0);

    // Create session 2 without dedicated token
    let s2 = manager
        .create_session("tenant-two", None, None)
        .expect("Failed to create tenant-two");
    assert_eq!(s2.session_id, "tenant-two");
    assert!(s2.port > 0);
    assert_ne!(s1.port, s2.port);

    // Duplicate creation rejected
    assert!(manager
        .create_session("tenant-one", None, None)
        .is_err());

    // Verify authentication
    assert!(manager.authenticate("tenant-one", Some("user1-key")));
    assert!(!manager.authenticate("tenant-one", Some("wrong-key")));
    assert!(!manager.authenticate("tenant-one", None));

    // Master secret authenticates any session
    assert!(manager.authenticate("tenant-one", Some("admin-master-key")));
    assert!(manager.authenticate("tenant-two", Some("admin-master-key")));

    // List and active count
    assert_eq!(manager.active_count(), 2);
    let list = manager.list_sessions();
    assert_eq!(list.len(), 2);

    // Prune idle (immediate should keep them)
    let pruned = manager.prune_idle(Duration::from_secs(3600));
    assert_eq!(pruned, 0);

    // Terminate one
    manager
        .terminate_session("tenant-one")
        .expect("Failed to terminate tenant-one");
    assert_eq!(manager.active_count(), 1);
    assert!(manager.get_session("tenant-one").is_none());

    // Terminate all
    manager.terminate_all().expect("Failed to terminate all");
    assert_eq!(manager.active_count(), 0);

    let _ = std::fs::remove_dir_all(&tmp_root);
}

#[test]
fn test_reverse_proxy_http_api_and_health() {
    let tmp_root = std::env::temp_dir().join(format!(
        "cb_test_proxy_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let mut broker_config = SessionBrokerConfig::default();
    broker_config.base_sandbox_dir = tmp_root.clone();
    broker_config.auth_required = true;

    let manager = SessionManager::new(broker_config, PathBuf::from("dummy_runner"));
    let server = SessionBrokerServer::new(manager.clone(), "127.0.0.1:0".parse().unwrap());
    let handle = server.start().expect("Failed to start SessionBrokerServer");
    let server_addr = handle.local_addr();

    // 1. Test GET /health
    {
        let mut stream = TcpStream::connect(server_addr).expect("Failed to connect");
        stream.write_all(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).unwrap();
        assert!(resp.starts_with("HTTP/1.1 200 OK"));
        assert!(resp.contains("\"status\":\"ok\""));
        assert!(resp.contains("\"active_sessions\":0"));
    }

    // 2. Test POST /api/sessions?id=web-tenant-1
    {
        let mut stream = TcpStream::connect(server_addr).expect("Failed to connect");
        stream.write_all(b"POST /api/sessions?id=web-tenant-1 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).unwrap();
        assert!(resp.starts_with("HTTP/1.1 201 Created"));
        assert!(resp.contains("\"session_id\":\"web-tenant-1\""));
    }

    // 3. Test GET /api/sessions
    {
        let mut stream = TcpStream::connect(server_addr).expect("Failed to connect");
        stream.write_all(b"GET /api/sessions HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).unwrap();
        assert!(resp.starts_with("HTTP/1.1 200 OK"));
        assert!(resp.contains("web-tenant-1"));
    }

    // 4. Test unauthorized WebSocket upgrade request to /session/web-tenant-1/ws
    {
        let mut stream = TcpStream::connect(server_addr).expect("Failed to connect");
        stream.write_all(b"GET /session/web-tenant-1/ws HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).unwrap();
        // Since auth_required is true and no token was sent, expect 401 Unauthorized
        assert!(resp.starts_with("HTTP/1.1 401 Unauthorized"));
    }

    // 5. Test nonexistent session request
    {
        let mut stream = TcpStream::connect(server_addr).expect("Failed to connect");
        stream.write_all(b"GET /session/nonexistent-session/ws HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).unwrap();
        assert!(resp.starts_with("HTTP/1.1 401 Unauthorized") || resp.starts_with("HTTP/1.1 404 Not Found"));
    }

    handle.shutdown();
    let _ = std::fs::remove_dir_all(&tmp_root);
}
