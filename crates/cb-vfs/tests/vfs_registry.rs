// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Unit & Integration Tests for `cb-vfs` Crate

use cb_vfs::*;
use std::path::Path;

#[test]
fn test_guid_parse() {
    let raw = "{00020400-0000-0000-C000-000000000046}";
    let parsed = Guid::parse(raw).expect("Failed to parse IID_IDispatch");
    assert_eq!(parsed, IID_IDispatch);
}

#[test]
fn test_guid_well_known_constants() {
    assert_eq!(IID_IUnknown.data1, 0x00000000);
    assert_eq!(IID_IDispatch.data1, 0x00020400);
    assert_eq!(IID_NULL.data1, 0x00000000);
}

#[test]
fn test_resolve_virtual_path() {
    let p1 = resolve_virtual_path("C:\\Windows\\System32\\msvbvm60.dll", None);
    assert_eq!(p1, Path::new("Windows/System32/msvbvm60.dll"));

    let root = Path::new("/var/sandbox");
    let p2 = resolve_virtual_path("D:\\Data\\file.txt", Some(root));
    assert_eq!(p2, Path::new("/var/sandbox/Data/file.txt"));
}

#[test]
fn test_registry_hive() {
    let mut reg = Registry::with_defaults();
    assert_eq!(reg.resolve_progid("Scripting.Dictionary"), Some(CLSID_Dictionary));
    assert_eq!(reg.resolve_progid("NonExistent.ProgID"), None);

    reg.set_value("Software\\ClassicBasic\\Version", "0.3.1");
    assert_eq!(reg.get_value("Software\\ClassicBasic\\Version"), Some("0.3.1".to_string()));
}
