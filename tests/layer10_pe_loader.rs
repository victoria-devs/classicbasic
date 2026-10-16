//! Layer 10: PE32 Loader & Execution Engine Tests
//!
//! Granular tests for DOS header validation, PE32 header parsing, section RVA mapping,
//! VBHeader detection ("VB5!"), and native execution engine.

use msvbvm60::pe::*;
use msvbvm60::types::VBHeader;

#[test]
fn test_pe_loader_invalid_dos_signature_rejection() {
    let dummy_data = vec![0u8; 512];
    let result = LoadedPe::load(&dummy_data);
    assert!(result.is_err());
    match result.err().unwrap() {
        PeError::InvalidDosHeader => {}
        other => panic!("Expected InvalidDosHeader, got {:?}", other),
    }
}

#[test]
fn test_pe_loader_synthetic_vb6_binary_execution() {
    let mut pe_data = vec![0u8; 1024];

    // 1. DOS Header
    let dos = unsafe { &mut *(pe_data.as_mut_ptr() as *mut ImageDosHeader) };
    dos.e_magic = IMAGE_DOS_SIGNATURE; // "MZ"
    dos.e_lfanew = 128; // NT Header offset

    // 2. NT Signature
    unsafe {
        *(pe_data.as_mut_ptr().add(128) as *mut u32) = IMAGE_NT_SIGNATURE; // "PE\0\0"
    }

    // 3. File Header
    let file_hdr = unsafe { &mut *(pe_data.as_mut_ptr().add(132) as *mut ImageFileHeader) };
    file_hdr.machine = IMAGE_FILE_MACHINE_I386;
    file_hdr.number_of_sections = 1;
    file_hdr.size_of_optional_header = core::mem::size_of::<ImageOptionalHeader32>() as u16;

    // 4. Optional Header
    let opt_hdr = unsafe {
        &mut *(pe_data.as_mut_ptr().add(132 + core::mem::size_of::<ImageFileHeader>()) as *mut ImageOptionalHeader32)
    };
    opt_hdr.magic = IMAGE_NT_OPTIONAL_HDR32_MAGIC;
    opt_hdr.image_base = 0x00400000;
    opt_hdr.section_alignment = 4096;
    opt_hdr.file_alignment = 512;
    opt_hdr.size_of_image = 8192;
    opt_hdr.size_of_headers = 512;
    opt_hdr.address_of_entry_point = 0x1000; // in section 1

    // 5. Section Header
    let sec_offset = 132 + core::mem::size_of::<ImageFileHeader>() + core::mem::size_of::<ImageOptionalHeader32>();
    let sec = unsafe { &mut *(pe_data.as_mut_ptr().add(sec_offset) as *mut ImageSectionHeader) };
    sec.name = *b".text\0\0\0";
    sec.virtual_address = 0x1000;
    sec.virtual_size = 512;
    sec.pointer_to_raw_data = 512;
    sec.size_of_raw_data = 512;
    sec.characteristics = IMAGE_SCN_MEM_READ | IMAGE_SCN_MEM_EXECUTE;

    // 6. Section Data at file offset 512:
    // Entry point is at RVA 0x1000 (which maps to offset 512 in file).
    // Entry point instructions:
    // PUSH <pointer to VBHeader> (0x68 imm32)
    // RET (0xC3)
    // VBHeader is placed at offset 512 + 16 (RVA 0x1010).
    pe_data[512] = 0x68; // PUSH imm32
    let vb_va = 0x00400000 + 0x1010;
    pe_data[513..517].copy_from_slice(&(vb_va as u32).to_le_bytes());
    pe_data[517] = 0xC3; // RET

    // Put a valid VBHeader at 512 + 16:
    let vb_hdr = unsafe { &mut *(pe_data.as_mut_ptr().add(512 + 16) as *mut VBHeader) };
    vb_hdr.sz_magic = *b"VB5!";
    vb_hdr.runtime_build_version = 0x1234;
    vb_hdr.runtime_revision = 0x0001;
    vb_hdr.w_forms_count = 3;
    vb_hdr.dw_thread_count = 1;
    vb_hdr.lp_sub_main = 0; // GUI startup

    // Load and verify!
    let loaded = LoadedPe::load(&pe_data).expect("PE loading should succeed");
    assert_eq!(loaded.entry_point_rva, 0x1000);
    assert_eq!(loaded.sections.len(), 1);
    assert_eq!(loaded.sections[0].name, ".text");

    let found_hdr_ptr = loaded.find_vb_header();
    assert!(found_hdr_ptr.is_some(), "VBHeader should be discovered");

    let found_hdr = unsafe { *found_hdr_ptr.unwrap() };
    assert!(found_hdr.is_valid());
    assert_eq!({ found_hdr.runtime_build_version }, 0x1234);
    assert_eq!({ found_hdr.w_forms_count }, 3);

    let exec_result = loaded.execute();
    assert!(exec_result.is_ok());
}
