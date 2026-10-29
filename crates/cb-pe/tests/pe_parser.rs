// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Unit & Integration Tests for `cb-pe` Crate

use cb_pe::*;

#[test]
fn test_dos_header_fields() {
    let mut dos: ImageDosHeader = unsafe { std::mem::zeroed() };
    dos.e_magic = IMAGE_DOS_SIGNATURE; // 'MZ' (0x5A4D)
    dos.e_lfanew = 0x0080;
    let magic = dos.e_magic;
    let lfanew = dos.e_lfanew;
    assert_eq!(magic, 0x5A4D);
    assert_eq!(lfanew, 0x0080);
}

#[test]
fn test_image_constants() {
    assert_eq!(IMAGE_DOS_SIGNATURE, 0x5A4D);
    assert_eq!(IMAGE_NT_SIGNATURE, 0x00004550);
    assert_eq!(IMAGE_FILE_MACHINE_I386, 0x014C);
    assert_eq!(IMAGE_NT_OPTIONAL_HDR32_MAGIC, 0x010B);
}

#[test]
fn test_msvbvm60_ordinal_lookup() {
    assert_eq!(msvbvm60_name_to_ordinal("ThunRTMain"), Some(100));
    assert_eq!(msvbvm60_name_to_ordinal("__vbaVarAdd"), Some(450));
    assert_eq!(msvbvm60_name_to_ordinal("__vbaStrCat"), Some(405));
    assert_eq!(msvbvm60_name_to_ordinal("rtcTrimBstr"), Some(519));
    assert_eq!(msvbvm60_name_to_ordinal("NonExistentExport"), None);
}

#[test]
fn test_vb_header_magic() {
    let mut vbh: VBHeader = unsafe { std::mem::zeroed() };
    vbh.sz_magic = *VB_MAGIC;
    assert_eq!(&vbh.sz_magic, b"VB5!");
}

#[test]
fn test_rsrc_constants_and_directory_parsing() {
    assert_eq!(RT_BITMAP, 2);
    assert_eq!(RT_ICON, 3);
    assert_eq!(RT_RCDATA, 10);
    assert_eq!(RT_VERSION, 16);

    // Empty or truncated bytes return empty vec without crashing
    let empty_res = parse_resource_directory(&[], 0x5000);
    assert!(empty_res.is_empty());
}

#[test]
fn test_object_table_layout() {
    assert_eq!(std::mem::size_of::<ObjectTable>(), 52);
    assert_eq!(std::mem::offset_of!(ObjectTable, lp_object_array), 48);
    assert_eq!(std::mem::offset_of!(ObjectTable, guid), 24);
    assert_eq!(std::mem::offset_of!(ObjectTable, w_compiled_objects_count), 42);
}

