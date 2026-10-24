// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Sandboxed Windows-to-POSIX Virtual FileSystem (VFS) path translation and provider traits.

use std::path::{Path, PathBuf};

/// Converts a null-terminated UTF-16 raw pointer into an owned Rust `String`.
///
/// # Safety
/// `ptr` must be either null or point to a valid null-terminated `u16` buffer.
pub unsafe fn utf16_ptr_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let slice = core::slice::from_raw_parts(ptr, len);
    String::from_utf16_lossy(slice)
}

/// Normalizes a legacy Windows path (e.g. `C:\Temp\file.txt` or `subdir\data.bin`)
/// into a sandboxed POSIX `PathBuf` relative to `sandbox_root` (or current working directory).
pub fn resolve_virtual_path(win_path: &str, sandbox_root: Option<&Path>) -> PathBuf {
    let normalized = win_path.replace('\\', "/");
    let stripped = if normalized.len() >= 2 && normalized.as_bytes()[1] == b':' {
        normalized[2..].trim_start_matches('/')
    } else {
        normalized.as_str()
    };
    if let Some(root) = sandbox_root {
        root.join(stripped)
    } else {
        PathBuf::from(stripped)
    }
}

/// Trait for multi-tenant sandbox providers (Milestone 4: Linux namespaces, cgroups v2, VFS).
pub trait SandboxProvider: Send + Sync {
    /// Returns the root directory of the tenant's isolated virtual drive (`C:\`).
    fn virtual_drive_root(&self) -> &Path;

    /// Resolves a guest Windows path to a host path inside the sandbox.
    fn resolve_path(&self, guest_path: &str) -> PathBuf {
        resolve_virtual_path(guest_path, Some(self.virtual_drive_root()))
    }
}
