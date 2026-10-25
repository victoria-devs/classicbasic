// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Unit & Integration Tests for `cb-session` Crate

use cb_session::*;
use std::path::PathBuf;

#[test]
fn test_session_config_initialization() {
    let cfg = SessionConfig {
        session_id: "tenant-001".to_string(),
        binary_path: PathBuf::from("/var/apps/App01.exe"),
        sandbox_root: PathBuf::from("/var/sandboxes/tenant-001"),
        memory_limit_bytes: 128 * 1024 * 1024,
    };

    assert_eq!(cfg.session_id, "tenant-001");
    assert_eq!(cfg.memory_limit_bytes, 134217728);
}
