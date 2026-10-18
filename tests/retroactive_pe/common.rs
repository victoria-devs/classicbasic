// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

use std::path::Path;
use std::process::Command;

pub fn run_binary_test(filename: &str) -> String {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("artifacts");
    let path = if base.join("bin").join(filename).exists() {
        base.join("bin").join(filename)
    } else {
        base.join(filename)
    };
    assert!(path.exists(), "Binary not found: {}", path.display());

    let runner = env!("CARGO_BIN_EXE_vb6-run");
    let output = Command::new(runner)
        .arg(&path)
        .output()
        .unwrap_or_else(|e| panic!("Failed to execute vb6-run with {}: {}", filename, e));

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "Binary {} failed with exit status {:?}\n--- STDOUT ---\n{}\n--- STDERR ---\n{}",
            filename, output.status, stdout, stderr
        );
    }
    stdout
}
