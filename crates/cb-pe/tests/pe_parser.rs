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
