//! Regression Suite v0.1.0 (Phase 1.1): PE32 Loader & Execution Engine Tests
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
        &mut *(pe_data
            .as_mut_ptr()
            .add(132 + core::mem::size_of::<ImageFileHeader>())
            as *mut ImageOptionalHeader32)
    };
    opt_hdr.magic = IMAGE_NT_OPTIONAL_HDR32_MAGIC;
    opt_hdr.image_base = 0x00400000;
    opt_hdr.section_alignment = 4096;
    opt_hdr.file_alignment = 512;
    opt_hdr.size_of_image = 8192;
    opt_hdr.size_of_headers = 512;
    opt_hdr.address_of_entry_point = 0x1000; // in section 1

    // 5. Section Header
    let sec_offset = 132
        + core::mem::size_of::<ImageFileHeader>()
        + core::mem::size_of::<ImageOptionalHeader32>();
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

fn get_proc_map_perms(addr: usize) -> Option<String> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    for line in maps.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let range: Vec<&str> = parts[0].split('-').collect();
            if range.len() == 2 {
                if let (Ok(start), Ok(end)) = (
                    usize::from_str_radix(range[0], 16),
                    usize::from_str_radix(range[1], 16),
                ) {
                    if addr >= start && addr < end {
                        return Some(parts[1].to_string());
                    }
                }
            }
        }
    }
    None
}

#[test]
fn test_pe32_maps_sections_with_correct_page_permissions() {
    let mut pe_data = vec![0u8; 2048];

    // 1. DOS Header
    let dos = unsafe { &mut *(pe_data.as_mut_ptr() as *mut ImageDosHeader) };
    dos.e_magic = IMAGE_DOS_SIGNATURE;
    dos.e_lfanew = 128;

    // 2. NT Signature
    unsafe {
        *(pe_data.as_mut_ptr().add(128) as *mut u32) = IMAGE_NT_SIGNATURE;
    }

    // 3. File Header
    let file_hdr = unsafe { &mut *(pe_data.as_mut_ptr().add(132) as *mut ImageFileHeader) };
    file_hdr.machine = IMAGE_FILE_MACHINE_I386;
    file_hdr.number_of_sections = 2;
    file_hdr.size_of_optional_header = core::mem::size_of::<ImageOptionalHeader32>() as u16;

    // 4. Optional Header
    let opt_hdr = unsafe {
        &mut *(pe_data
            .as_mut_ptr()
            .add(132 + core::mem::size_of::<ImageFileHeader>())
            as *mut ImageOptionalHeader32)
    };
    opt_hdr.magic = IMAGE_NT_OPTIONAL_HDR32_MAGIC;
    opt_hdr.image_base = 0x00400000;
    opt_hdr.section_alignment = 4096;
    opt_hdr.file_alignment = 512;
    opt_hdr.size_of_image = 12288;
    opt_hdr.size_of_headers = 512;
    opt_hdr.address_of_entry_point = 0x1000;

    // 5. Section Headers: .text (RX) and .data (RW)
    let sec_offset = 132
        + core::mem::size_of::<ImageFileHeader>()
        + core::mem::size_of::<ImageOptionalHeader32>();
    let sec_text =
        unsafe { &mut *(pe_data.as_mut_ptr().add(sec_offset) as *mut ImageSectionHeader) };
    sec_text.name = *b".text\0\0\0";
    sec_text.virtual_address = 0x1000;
    sec_text.virtual_size = 512;
    sec_text.pointer_to_raw_data = 512;
    sec_text.size_of_raw_data = 512;
    sec_text.characteristics = IMAGE_SCN_MEM_READ | IMAGE_SCN_MEM_EXECUTE;

    let sec_data = unsafe {
        &mut *(pe_data
            .as_mut_ptr()
            .add(sec_offset + core::mem::size_of::<ImageSectionHeader>())
            as *mut ImageSectionHeader)
    };
    sec_data.name = *b".data\0\0\0";
    sec_data.virtual_address = 0x2000;
    sec_data.virtual_size = 512;
    sec_data.pointer_to_raw_data = 1024;
    sec_data.size_of_raw_data = 512;
    sec_data.characteristics = IMAGE_SCN_MEM_READ | IMAGE_SCN_MEM_WRITE;

    // Put valid code and VBHeader in .text
    pe_data[512] = 0x68; // PUSH imm32
    let vb_va = 0x00400000 + 0x1010;
    pe_data[513..517].copy_from_slice(&(vb_va as u32).to_le_bytes());
    pe_data[517] = 0xC3; // RET

    let vb_hdr = unsafe { &mut *(pe_data.as_mut_ptr().add(512 + 16) as *mut VBHeader) };
    vb_hdr.sz_magic = *b"VB5!";
    vb_hdr.runtime_build_version = 0x1234;

    let loaded = LoadedPe::load(&pe_data).expect("PE loading should succeed");
    let text_addr = unsafe { loaded.mapped_base.add(0x1000) } as usize;
    let data_addr = unsafe { loaded.mapped_base.add(0x2000) } as usize;

    if let Some(perms) = get_proc_map_perms(text_addr) {
        let chars: Vec<char> = perms.chars().collect();
        assert_eq!(chars[0], 'r', ".text must be readable");
        assert_eq!(chars[1], '-', ".text must NOT be writable");
        assert_eq!(chars[2], 'x', ".text must be executable");
    }

    if let Some(perms) = get_proc_map_perms(data_addr) {
        let chars: Vec<char> = perms.chars().collect();
        assert_eq!(chars[0], 'r', ".data must be readable");
        assert_eq!(chars[1], 'w', ".data must be writable");
        assert_eq!(chars[2], '-', ".data must NOT be executable");
    }
}

#[test]
fn test_pe32_base_relocation_delta_applied_correctly_when_rebasing() {
    let mut pe_data = vec![0u8; 2048];

    // 1. DOS Header
    let dos = unsafe { &mut *(pe_data.as_mut_ptr() as *mut ImageDosHeader) };
    dos.e_magic = IMAGE_DOS_SIGNATURE;
    dos.e_lfanew = 128;

    // 2. NT Signature
    unsafe {
        *(pe_data.as_mut_ptr().add(128) as *mut u32) = IMAGE_NT_SIGNATURE;
    }

    // 3. File Header
    let file_hdr = unsafe { &mut *(pe_data.as_mut_ptr().add(132) as *mut ImageFileHeader) };
    file_hdr.machine = IMAGE_FILE_MACHINE_I386;
    file_hdr.number_of_sections = 2;
    file_hdr.size_of_optional_header = core::mem::size_of::<ImageOptionalHeader32>() as u16;

    // 4. Optional Header
    let opt_hdr = unsafe {
        &mut *(pe_data
            .as_mut_ptr()
            .add(132 + core::mem::size_of::<ImageFileHeader>())
            as *mut ImageOptionalHeader32)
    };
    opt_hdr.magic = IMAGE_NT_OPTIONAL_HDR32_MAGIC;
    opt_hdr.image_base = 0x00400000;
    opt_hdr.section_alignment = 4096;
    opt_hdr.file_alignment = 512;
    opt_hdr.size_of_image = 12288;
    opt_hdr.size_of_headers = 512;
    opt_hdr.address_of_entry_point = 0x1000;

    // Relocation directory: RVA 0x2000, size 12
    opt_hdr.data_directory[IMAGE_DIRECTORY_ENTRY_BASERELOC].virtual_address = 0x2000;
    opt_hdr.data_directory[IMAGE_DIRECTORY_ENTRY_BASERELOC].size = 12;

    // 5. Section Headers: .text (RVA 0x1000) and .reloc (RVA 0x2000)
    let sec_offset = 132
        + core::mem::size_of::<ImageFileHeader>()
        + core::mem::size_of::<ImageOptionalHeader32>();
    let sec_text =
        unsafe { &mut *(pe_data.as_mut_ptr().add(sec_offset) as *mut ImageSectionHeader) };
    sec_text.name = *b".text\0\0\0";
    sec_text.virtual_address = 0x1000;
    sec_text.virtual_size = 512;
    sec_text.pointer_to_raw_data = 512;
    sec_text.size_of_raw_data = 512;
    sec_text.characteristics = IMAGE_SCN_MEM_READ | IMAGE_SCN_MEM_EXECUTE;

    let sec_reloc = unsafe {
        &mut *(pe_data
            .as_mut_ptr()
            .add(sec_offset + core::mem::size_of::<ImageSectionHeader>())
            as *mut ImageSectionHeader)
    };
    sec_reloc.name = *b".reloc\0\0";
    sec_reloc.virtual_address = 0x2000;
    sec_reloc.virtual_size = 512;
    sec_reloc.pointer_to_raw_data = 1024;
    sec_reloc.size_of_raw_data = 512;
    sec_reloc.characteristics = IMAGE_SCN_MEM_READ;

    // Code at file offset 512: PUSH 0x00401010, RET
    pe_data[512] = 0x68;
    let original_va: u32 = 0x00401010;
    pe_data[513..517].copy_from_slice(&original_va.to_le_bytes());
    pe_data[517] = 0xC3;

    // VBHeader at offset 512 + 16 (RVA 0x1010)
    let vb_hdr = unsafe { &mut *(pe_data.as_mut_ptr().add(512 + 16) as *mut VBHeader) };
    vb_hdr.sz_magic = *b"VB5!";
    vb_hdr.runtime_build_version = 0x7777;

    // .reloc table at file offset 1024 (RVA 0x2000):
    // Block: virtual_address = 0x1000, size_of_block = 12
    // Entry 0: (IMAGE_REL_BASED_HIGHLOW << 12) | 1 (offset 1 for imm32 of PUSH)
    // Entry 1: 0 (padding)
    let reloc_block = unsafe { &mut *(pe_data.as_mut_ptr().add(1024) as *mut ImageBaseRelocation) };
    reloc_block.virtual_address = 0x1000;
    reloc_block.size_of_block = 12;

    let entry_val: u16 = (IMAGE_REL_BASED_HIGHLOW as u16) << 12 | 1;
    pe_data[1024 + 8..1024 + 10].copy_from_slice(&entry_val.to_le_bytes());
    pe_data[1024 + 10..1024 + 12].copy_from_slice(&0u16.to_le_bytes());

    // Force load at rebased address 0x00500000 (Delta = +0x00100000)
    let forced_base = 0x00500000;
    let loaded = LoadedPe::load_with_forced_base(&pe_data, forced_base)
        .expect("PE loading with forced rebase should succeed");

    assert_eq!(loaded.delta, 0x00100000);

    // Verify pointer in code is adjusted by delta
    let patched_va =
        unsafe { core::ptr::read_unaligned(loaded.mapped_base.add(0x1001) as *const u32) };
    assert_eq!(
        patched_va, 0x00501010,
        "Target reference adjusted by exactly delta +0x00100000"
    );

    // Verify VBHeader discovery returns the rebased pointer
    let found_hdr = loaded
        .find_vb_header()
        .expect("VBHeader should be found at rebased address");
    assert_eq!(
        found_hdr as usize, 0x00501010,
        "VBHeader pointer must point to rebased address"
    );
}

#[test]
fn test_pe32_iat_resolves_all_msvbvm60_exports_by_ordinal_and_name() {
    let mut pe_data = vec![0u8; 4096];

    // 1. DOS Header
    let dos = unsafe { &mut *(pe_data.as_mut_ptr() as *mut ImageDosHeader) };
    dos.e_magic = IMAGE_DOS_SIGNATURE;
    dos.e_lfanew = 128;

    // 2. NT Signature
    unsafe {
        *(pe_data.as_mut_ptr().add(128) as *mut u32) = IMAGE_NT_SIGNATURE;
    }

    // 3. File Header
    let file_hdr = unsafe { &mut *(pe_data.as_mut_ptr().add(132) as *mut ImageFileHeader) };
    file_hdr.machine = IMAGE_FILE_MACHINE_I386;
    file_hdr.number_of_sections = 2;
    file_hdr.size_of_optional_header = core::mem::size_of::<ImageOptionalHeader32>() as u16;

    // 4. Optional Header
    let opt_hdr = unsafe {
        &mut *(pe_data
            .as_mut_ptr()
            .add(132 + core::mem::size_of::<ImageFileHeader>())
            as *mut ImageOptionalHeader32)
    };
    opt_hdr.magic = IMAGE_NT_OPTIONAL_HDR32_MAGIC;
    opt_hdr.image_base = 0x00400000;
    opt_hdr.section_alignment = 4096;
    opt_hdr.file_alignment = 512;
    opt_hdr.size_of_image = 12288;
    opt_hdr.size_of_headers = 512;
    opt_hdr.address_of_entry_point = 0x1000;

    // Import directory at RVA 0x2000 (in section 2)
    opt_hdr.data_directory[IMAGE_DIRECTORY_ENTRY_IMPORT].virtual_address = 0x2000;
    opt_hdr.data_directory[IMAGE_DIRECTORY_ENTRY_IMPORT].size = 40;

    // 5. Sections: .text (RVA 0x1000, raw 512) and .rdata (RVA 0x2000, raw 1024)
    let sec_offset = 132
        + core::mem::size_of::<ImageFileHeader>()
        + core::mem::size_of::<ImageOptionalHeader32>();
    let sec_text =
        unsafe { &mut *(pe_data.as_mut_ptr().add(sec_offset) as *mut ImageSectionHeader) };
    sec_text.name = *b".text\0\0\0";
    sec_text.virtual_address = 0x1000;
    sec_text.virtual_size = 512;
    sec_text.pointer_to_raw_data = 512;
    sec_text.size_of_raw_data = 512;
    sec_text.characteristics = IMAGE_SCN_MEM_READ | IMAGE_SCN_MEM_EXECUTE;

    let sec_rdata = unsafe {
        &mut *(pe_data
            .as_mut_ptr()
            .add(sec_offset + core::mem::size_of::<ImageSectionHeader>())
            as *mut ImageSectionHeader)
    };
    sec_rdata.name = *b".rdata\0\0";
    sec_rdata.virtual_address = 0x2000;
    sec_rdata.virtual_size = 1024;
    sec_rdata.pointer_to_raw_data = 1024;
    sec_rdata.size_of_raw_data = 1024;
    sec_rdata.characteristics = IMAGE_SCN_MEM_READ | IMAGE_SCN_MEM_WRITE;

    // Setup Import Descriptor at file offset 1024 (RVA 0x2000):
    // original_first_thunk: RVA 0x2050 (offset 1024 + 0x50)
    // name: RVA 0x2100 (offset 1024 + 0x100) -> "msvbvm60.dll\0"
    // first_thunk: RVA 0x2080 (offset 1024 + 0x80)
    let imp_desc = unsafe { &mut *(pe_data.as_mut_ptr().add(1024) as *mut ImageImportDescriptor) };
    imp_desc.original_first_thunk = 0x2050;
    imp_desc.name = 0x2100;
    imp_desc.first_thunk = 0x2080;

    // DLL name string at 1024 + 0x100
    let name_bytes = b"msvbvm60.dll\0";
    pe_data[1024 + 0x100..1024 + 0x100 + name_bytes.len()].copy_from_slice(name_bytes);

    // Hint/Name for "ThunRTMain" at RVA 0x2150 (offset 1024 + 0x150)
    // 2 bytes hint (0) + name + \0
    pe_data[1024 + 0x152..1024 + 0x152 + b"ThunRTMain\0".len()].copy_from_slice(b"ThunRTMain\0");

    // Thunk entries at original_first_thunk (1024 + 0x50):
    // 1. Ordinal #100: 0x80000064
    // 2. Ordinal #534: 0x80000216 (rtcBeep)
    // 3. Ordinal #600: 0x80000258
    // 4. Name "ThunRTMain": RVA 0x2150
    // 5. 0 (null terminator)
    let thunk_vals: [u32; 5] = [
        0x80000000 | 100,
        0x80000000 | 534,
        0x80000000 | 600,
        0x00002150,
        0,
    ];
    for (i, &tv) in thunk_vals.iter().enumerate() {
        pe_data[1024 + 0x50 + i * 4..1024 + 0x50 + (i + 1) * 4].copy_from_slice(&tv.to_le_bytes());
        // Copy same to FirstThunk table at 1024 + 0x80
        pe_data[1024 + 0x80 + i * 4..1024 + 0x80 + (i + 1) * 4].copy_from_slice(&tv.to_le_bytes());
    }

    let loaded = LoadedPe::load(&pe_data).expect("PE loading should succeed");

    // Read resolved IAT at RVA 0x2080
    let iat_ptr = unsafe { loaded.mapped_base.add(0x2080) as *const u32 };
    let iat_0 = unsafe { *iat_ptr };
    let iat_1 = unsafe { *iat_ptr.add(1) };
    let iat_2 = unsafe { *iat_ptr.add(2) };
    let iat_3 = unsafe { *iat_ptr.add(3) };

    assert_ne!(iat_0, 0, "Ordinal #100 must resolve to non-null");
    assert_ne!(iat_1, 0, "Ordinal #534 must resolve to non-null");
    assert_ne!(iat_2, 0, "Ordinal #600 must resolve to non-null");
    assert_ne!(iat_3, 0, "Name 'ThunRTMain' must resolve to non-null");

    assert_eq!(iat_0, 0xDEAD0000 | 100, "Ordinal #100 must resolve to 0xDEAD0064");
    assert_eq!(iat_1, 0xDEAD0000 | 534, "Ordinal #534 must resolve to 0xDEAD0216");
    assert_eq!(iat_3, 0xDEAD0000 | 100, "Name 'ThunRTMain' must resolve to 0xDEAD0064");
}

#[test]
fn test_dll_function_call_resolves_external_win32_apis_dynamically() {
    let dll_name = std::ffi::CString::new("kernel32.dll").unwrap();
    let func_name = std::ffi::CString::new("GetTickCount").unwrap();
    let mut fn_cache: usize = 0;

    let mut info = msvbvm60::com::DllFunctionCallInfo {
        lp_dll_name: dll_name.as_ptr(),
        lp_function_name: func_name.as_ptr(),
        dw_flags: 0,
        lp_fn_cache: &mut fn_cache,
    };

    let resolved_ptr = unsafe { msvbvm60::com::__vbaDllFunctionCall(&mut info) };
    assert_ne!(
        resolved_ptr, 0,
        "Dynamic DLL call must resolve GetTickCount"
    );
    assert_eq!(fn_cache, resolved_ptr, "Cache pointer must be updated");

    // Verify it is a valid callable function pointer
    let tick_fn: unsafe extern "system" fn() -> u32 = unsafe { core::mem::transmute(resolved_ptr) };
    let ticks = unsafe { tick_fn() };
    assert!(ticks < u32::MAX);
}

#[test]
fn test_vbheader_valid_magic() {
    let hdr = VBHeader {
        sz_magic: *b"VB5!",
        runtime_build_version: 0x1234,
        sz_language_dll: [0; 14],
        sz_sec_lang_dll: [0; 14],
        runtime_revision: 1,
        lcid: 0x409,
        sec_lcid: 0,
        lp_sub_main: 0,
        lp_project_data: 0,
        mdl_int_pool_cache: 0,
        mdl_int_pool_cache2: 0,
        dw_thread_flags: 0,
        dw_thread_count: 1,
        w_forms_count: 1,
        w_external_component_count: 0,
        dw_thunk_count: 0,
        lp_gui_table: 0,
        lp_ide_data: 0,
        lp_project_dependencies: 0,
        lp_import_table: 0,
        lp_export_table: 0,
    };
    assert!(hdr.is_valid());
}

#[test]
fn test_vbheader_invalid_magic() {
    let mut hdr = VBHeader {
        sz_magic: *b"BAD!",
        runtime_build_version: 0,
        sz_language_dll: [0; 14],
        sz_sec_lang_dll: [0; 14],
        runtime_revision: 0,
        lcid: 0,
        sec_lcid: 0,
        lp_sub_main: 0,
        lp_project_data: 0,
        mdl_int_pool_cache: 0,
        mdl_int_pool_cache2: 0,
        dw_thread_flags: 0,
        dw_thread_count: 0,
        w_forms_count: 0,
        w_external_component_count: 0,
        dw_thunk_count: 0,
        lp_gui_table: 0,
        lp_ide_data: 0,
        lp_project_dependencies: 0,
        lp_import_table: 0,
        lp_export_table: 0,
    };
    assert!(!hdr.is_valid());
    hdr.sz_magic = *b"VB5!";
    assert!(hdr.is_valid());
}

#[test]
fn test_dllmain_process_attach_and_detach() {
    unsafe {
        assert_eq!(msvbvm60::DllMain(core::ptr::null_mut(), 1, core::ptr::null_mut()), 1);
        assert_eq!(msvbvm60::DllMain(core::ptr::null_mut(), 0, core::ptr::null_mut()), 1);
    }
}

#[test]
fn test_vba_end_flushes_and_closes_all_active_virtual_file_channels() {
    unsafe {
        let p1 = msvbvm60::sys_alloc_string("test_end_chan1.txt");
        let p2 = msvbvm60::sys_alloc_string("test_end_chan2.txt");
        let p3 = msvbvm60::sys_alloc_string("test_end_chan3.txt");

        assert_eq!(msvbvm60::__vbaFileOpen(0x02, -1, 1, p1), 0);
        assert_eq!(msvbvm60::__vbaFileOpen(0x02, -1, 2, p2), 0);
        assert_eq!(msvbvm60::__vbaFileOpen(0x02, -1, 3, p3), 0);

        let text1 = msvbvm60::sys_alloc_string("Channel 1 pending data");
        assert_eq!(msvbvm60::__vbaPrintFile(msvbvm60::PRINT_NEWLINE as *const _, 1, text1), 0);
        msvbvm60::SysFreeString(text1);

        msvbvm60::set_runtime_state(msvbvm60::RuntimeState::Running);
        msvbvm60::set_exit_hook(Some(|_code: i32| {}));

        msvbvm60::__vbaEnd();

        msvbvm60::set_exit_hook(None::<fn(i32)>);

        let path1 = std::path::Path::new("test_end_chan1.txt");
        assert!(path1.exists());
        let content1 = std::fs::read_to_string(path1).unwrap();
        assert!(content1.contains("Channel 1 pending data"));

        let eof_res = msvbvm60::rtcEndOfFile(1);
        assert_eq!(eof_res, -1);
        assert_eq!(msvbvm60::err::get_vb_error_number(), 52);

        assert_eq!(msvbvm60::get_runtime_state(), msvbvm60::RuntimeState::Terminated);

        msvbvm60::rtcKillFiles(p1);
        msvbvm60::rtcKillFiles(p2);
        msvbvm60::rtcKillFiles(p3);
        msvbvm60::SysFreeString(p1);
        msvbvm60::SysFreeString(p2);
        msvbvm60::SysFreeString(p3);
    }
}

#[test]
fn test_vba_end_exits_process_with_zero_status_code() {
    let runner = env!("CARGO_BIN_EXE_vb6-run");
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let base = if manifest_dir.join("artifacts").exists() {
        manifest_dir.join("artifacts")
    } else {
        manifest_dir.join("../../artifacts")
    };
    let binary_path = if base.join("bin").join("App10-core-primitives-native.exe").exists() {
        base.join("bin").join("App10-core-primitives-native.exe")
    } else {
        base.join("App10-core-primitives-native.exe")
    };

    let output = std::process::Command::new(runner)
        .arg(&binary_path)
        .output()
        .expect("Failed to execute runner");

    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_rtc_environ_bstr() {
    unsafe {
        let var_name = msvbvm60::sys_alloc_string("PATH");
        let val = msvbvm60::rtc::rtcEnvironBstr(var_name);
        let s = msvbvm60::bstr_to_string(val);
        assert!(!s.is_empty());
        msvbvm60::SysFreeString(var_name);
        msvbvm60::SysFreeString(val);
    }
}

#[test]
fn test_rtc_current_dir_bstr() {
    unsafe {
        let val = msvbvm60::rtc::rtcCurrentDirBstr(core::ptr::null_mut());
        let s = msvbvm60::bstr_to_string(val);
        assert!(!s.is_empty());
        msvbvm60::SysFreeString(val);
    }
}

#[test]
fn test_rtc_get_date_bstr_format() {
    unsafe {
        let val = msvbvm60::rtc::rtcGetDateBstr();
        let s = msvbvm60::bstr_to_string(val);
        // MM-DD-YYYY has 10 characters
        assert_eq!(s.len(), 10);
        msvbvm60::SysFreeString(val);
    }
}

#[test]
fn test_rtc_get_time_bstr_format() {
    unsafe {
        let val = msvbvm60::rtc::rtcGetTimeBstr();
        let s = msvbvm60::bstr_to_string(val);
        // HH:MM:SS has 8 characters
        assert_eq!(s.len(), 8);
        msvbvm60::SysFreeString(val);
    }
}

#[test]
fn test_rtc_get_timer_bounds() {
    unsafe {
        let t = msvbvm60::rtc::rtcGetTimer();
        // Seconds since midnight: 0.0 .. 86400.0
        assert!(t >= 0.0 && t <= 86400.0);
    }
}

#[test]
fn test_rtc_get_present_date_now() {
    unsafe {
        let mut v_now = msvbvm60::Variant::empty();
        msvbvm60::rtc::rtcGetPresentDate(&mut v_now);
        assert_eq!(v_now.vt, msvbvm60::VT_DATE);
        let serial = msvbvm60::__vbaDateVar(&v_now);
        assert!(serial > 25569.0); // Past 1970-01-01
    }
}

#[test]
fn test_rtc_make_and_remove_dir() {
    unsafe {
        let dir_name = msvbvm60::sys_alloc_string("test_layer07_dir");
        msvbvm60::rtc::rtcMakeDir(dir_name);
        assert!(std::path::Path::new("test_layer07_dir").is_dir());

        msvbvm60::rtc::rtcRemoveDir(dir_name);
        assert!(!std::path::Path::new("test_layer07_dir").exists());

        msvbvm60::SysFreeString(dir_name);
    }
}

#[test]
fn test_filesystem_virtual_path_resolution_normalizes_windows_separators() {
    let win_path = "C:\\APP\\DATA\\OUTPUT.TXT";
    let normalized = msvbvm60::vba::resolve_virtual_path(win_path);
    assert_eq!(normalized, "/app/data/output.txt");

    let sub_path = "DATA\\SUB\\CONFIG.INI";
    let normalized_sub = msvbvm60::vba::resolve_virtual_path(sub_path);
    assert_eq!(normalized_sub, "DATA/SUB/CONFIG.INI");

    unsafe {
        let b_win = msvbvm60::sys_alloc_string("C:\\APP\\DATA\\OUTPUT.TXT");
        let resolved = msvbvm60::vba::extract_bstr_or_variant_path(b_win);
        assert_eq!(resolved, "/app/data/output.txt");
        msvbvm60::SysFreeString(b_win);
    }
}

