//! PE32 Loader and Execution Engine for Linux, macOS, and Windows.

use std::ffi::CStr;
use std::fmt;
use crate::pe::headers::*;
use crate::pe::resolver::resolve_symbol;
use crate::pe::shims::{GLOBAL_MODULE_HANDLE, GLOBAL_MODULE_SIZE};
use crate::types::VBHeader;
use crate::thun_rt::{ExecutionMode, ThunRTMain, set_execution_mode};

#[derive(Debug)]
pub enum PeError {
    FileTooSmall,
    InvalidDosHeader,
    InvalidNtSignature,
    UnsupportedArchitecture(u16),
    NotPe32(u16),
    AllocationFailed,
    SectionOutOfBounds,
    RelocationError,
    ImportError(String),
}

impl fmt::Display for PeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PeError::FileTooSmall => write!(f, "File is too small to contain valid PE headers"),
            PeError::InvalidDosHeader => write!(f, "Invalid DOS header (missing 'MZ' signature)"),
            PeError::InvalidNtSignature => write!(f, "Invalid NT signature (missing 'PE\\0\\0')"),
            PeError::UnsupportedArchitecture(m) => write!(f, "Unsupported architecture: 0x{:04X} (expected x86 / i386)", m),
            PeError::NotPe32(m) => write!(f, "Not a 32-bit PE binary: magic 0x{:04X}", m),
            PeError::AllocationFailed => write!(f, "Failed to allocate virtual memory for PE image"),
            PeError::SectionOutOfBounds => write!(f, "Section raw data exceeds file size"),
            PeError::RelocationError => write!(f, "Error applying base relocations"),
            PeError::ImportError(msg) => write!(f, "Error resolving imports: {}", msg),
        }
    }
}

impl std::error::Error for PeError {}

#[derive(Debug, Clone)]
pub struct SectionInfo {
    pub name: String,
    pub virtual_address: u32,
    pub virtual_size: u32,
    pub size_of_raw_data: u32,
    pub characteristics: u32,
}

pub struct LoadedPe {
    pub mapped_base: *mut u8,
    pub mapped_size: usize,
    pub preferred_base: usize,
    pub delta: i64,
    pub entry_point_rva: u32,
    pub sections: Vec<SectionInfo>,
}

impl LoadedPe {
    /// Loads a PE32 executable from a byte buffer into executable memory,
    /// performs base relocations, and resolves imports against msvbvm60 and Win32 shims.
    pub fn load(data: &[u8]) -> Result<Self, PeError> {
        if data.len() < core::mem::size_of::<ImageDosHeader>() {
            return Err(PeError::FileTooSmall);
        }

        // 1. Verify DOS Header
        let dos_header = unsafe { core::ptr::read_unaligned(data.as_ptr() as *const ImageDosHeader) };
        if dos_header.e_magic != IMAGE_DOS_SIGNATURE {
            return Err(PeError::InvalidDosHeader);
        }

        let nt_offset = dos_header.e_lfanew as usize;
        if data.len() < nt_offset + 4 + core::mem::size_of::<ImageFileHeader>() {
            return Err(PeError::FileTooSmall);
        }

        // 2. Verify NT Headers
        let nt_sig = unsafe { core::ptr::read_unaligned(data.as_ptr().add(nt_offset) as *const u32) };
        if nt_sig != IMAGE_NT_SIGNATURE {
            return Err(PeError::InvalidNtSignature);
        }

        let file_header = unsafe { core::ptr::read_unaligned(data.as_ptr().add(nt_offset + 4) as *const ImageFileHeader) };

        if file_header.machine != IMAGE_FILE_MACHINE_I386 {
            return Err(PeError::UnsupportedArchitecture(file_header.machine));
        }

        let opt_offset = nt_offset + 4 + core::mem::size_of::<ImageFileHeader>();
        let opt_header = unsafe { core::ptr::read_unaligned(data.as_ptr().add(opt_offset) as *const ImageOptionalHeader32) };

        if opt_header.magic != IMAGE_NT_OPTIONAL_HDR32_MAGIC {
            return Err(PeError::NotPe32(opt_header.magic));
        }

        let preferred_base = opt_header.image_base as usize;
        let image_size = opt_header.size_of_image as usize;
        let entry_point_rva = opt_header.address_of_entry_point;

        // 3. Allocate Virtual Memory
        let mapped_base = unsafe { allocate_executable_memory(preferred_base, image_size)? };
        let delta = (mapped_base as i64) - (preferred_base as i64);

        unsafe {
            GLOBAL_MODULE_HANDLE = mapped_base as usize;
            GLOBAL_MODULE_SIZE = image_size;
        }

        // 4. Copy Headers
        let headers_size = (opt_header.size_of_headers as usize).min(data.len());
        unsafe {
            core::ptr::copy_nonoverlapping(data.as_ptr(), mapped_base, headers_size);
        }

        // 5. Copy Sections
        let sections_offset = opt_offset + file_header.size_of_optional_header as usize;
        let mut sections = Vec::with_capacity(file_header.number_of_sections as usize);

        for i in 0..file_header.number_of_sections as usize {
            let sec = unsafe {
                core::ptr::read_unaligned(
                    data.as_ptr().add(sections_offset + i * core::mem::size_of::<ImageSectionHeader>())
                        as *const ImageSectionHeader,
                )
            };

            let name_bytes = sec.name.split(|&b| b == 0).next().unwrap_or(&sec.name);
            let name_str = String::from_utf8_lossy(name_bytes).to_string();

            let sec_va = sec.virtual_address as usize;
            let raw_ptr = sec.pointer_to_raw_data as usize;
            let raw_size = sec.size_of_raw_data as usize;

            if raw_size > 0 && raw_ptr < data.len() {
                let copy_len = raw_size.min(data.len() - raw_ptr);
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        data.as_ptr().add(raw_ptr),
                        mapped_base.add(sec_va),
                        copy_len,
                    );
                }
            }

            sections.push(SectionInfo {
                name: name_str,
                virtual_address: sec.virtual_address,
                virtual_size: sec.virtual_size,
                size_of_raw_data: sec.size_of_raw_data,
                characteristics: sec.characteristics,
            });
        }

        let mut loaded = LoadedPe {
            mapped_base,
            mapped_size: image_size,
            preferred_base,
            delta,
            entry_point_rva,
            sections,
        };

        // 6. Apply Base Relocations if loaded at a different address
        if delta != 0 {
            loaded.apply_relocations(&opt_header)?;
        }

        // 7. Resolve Imports
        loaded.resolve_imports(&opt_header)?;

        Ok(loaded)
    }

    /// Applies base relocations (.reloc) when the image is not mapped at its preferred ImageBase.
    fn apply_relocations(&mut self, opt_header: &ImageOptionalHeader32) -> Result<(), PeError> {
        let reloc_dir = &opt_header.data_directory[IMAGE_DIRECTORY_ENTRY_BASERELOC];
        if reloc_dir.virtual_address == 0 || reloc_dir.size == 0 {
            return Ok(());
        }

        let mut offset = reloc_dir.virtual_address as usize;
        let end_offset = offset + reloc_dir.size as usize;

        while offset < end_offset && offset + core::mem::size_of::<ImageBaseRelocation>() <= self.mapped_size {
            let block = unsafe { core::ptr::read_unaligned(self.mapped_base.add(offset) as *const ImageBaseRelocation) };
            let page_rva = block.virtual_address as usize;
            let block_size = block.size_of_block as usize;

            if block_size < 8 {
                break;
            }

            let entries_count = (block_size - 8) / 2;

            for i in 0..entries_count {
                let entry = unsafe { core::ptr::read_unaligned(self.mapped_base.add(offset + 8 + i * 2) as *const u16) };
                let reloc_type = entry >> 12;
                let reloc_offset = (entry & 0x0FFF) as usize;

                if reloc_type == IMAGE_REL_BASED_HIGHLOW {
                    let target_rva = page_rva + reloc_offset;
                    if target_rva + 4 <= self.mapped_size {
                        unsafe {
                            let patch_ptr = self.mapped_base.add(target_rva) as *mut u32;
                            let val = core::ptr::read_unaligned(patch_ptr);
                            core::ptr::write_unaligned(patch_ptr, (val as i64 + self.delta) as u32);
                        }
                    }
                }
            }

            offset += block_size;
        }

        Ok(())
    }

    /// Resolves the Import Address Table (IAT) for all imported libraries.
    fn resolve_imports(&mut self, opt_header: &ImageOptionalHeader32) -> Result<(), PeError> {
        let import_dir = &opt_header.data_directory[IMAGE_DIRECTORY_ENTRY_IMPORT];
        if import_dir.virtual_address == 0 || import_dir.size == 0 {
            return Ok(());
        }

        let mut desc_offset = import_dir.virtual_address as usize;

        loop {
            if desc_offset + core::mem::size_of::<ImageImportDescriptor>() > self.mapped_size {
                break;
            }

            let desc = unsafe { core::ptr::read_unaligned(self.mapped_base.add(desc_offset) as *const ImageImportDescriptor) };
            if desc.name == 0 {
                break; // Null terminator descriptor
            }

            let dll_name_ptr = unsafe { self.mapped_base.add(desc.name as usize) as *const i8 };
            let dll_name = unsafe { CStr::from_ptr(dll_name_ptr) }.to_string_lossy();

            let thunk_rva = if desc.original_first_thunk != 0 {
                desc.original_first_thunk as usize
            } else {
                desc.first_thunk as usize
            };

            let iat_rva = desc.first_thunk as usize;
            let mut thunk_idx = 0;

            loop {
                let thunk_entry_offset = thunk_rva + thunk_idx * 4;
                let iat_entry_offset = iat_rva + thunk_idx * 4;

                if thunk_entry_offset + 4 > self.mapped_size || iat_entry_offset + 4 > self.mapped_size {
                    break;
                }

                let thunk_val = unsafe { core::ptr::read_unaligned(self.mapped_base.add(thunk_entry_offset) as *const u32) };
                if thunk_val == 0 {
                    break;
                }

                let fn_addr: usize;

                if (thunk_val & 0x80000000) != 0 {
                    // Import by Ordinal
                    let ordinal = (thunk_val & 0xFFFF) as u16;
                    fn_addr = resolve_symbol(&dll_name, None, Some(ordinal));
                } else {
                    // Import by Name
                    let name_rva = (thunk_val & 0x7FFFFFFF) as usize;
                    if name_rva + 2 < self.mapped_size {
                        let fn_name_ptr = unsafe { self.mapped_base.add(name_rva + 2) as *const i8 };
                        let fn_name = unsafe { CStr::from_ptr(fn_name_ptr) }.to_string_lossy();
                        fn_addr = resolve_symbol(&dll_name, Some(&fn_name), None);
                    } else {
                        fn_addr = resolve_symbol(&dll_name, None, None);
                    }
                }

                // Patch the IAT entry with the resolved function pointer
                unsafe {
                    core::ptr::write_unaligned(self.mapped_base.add(iat_entry_offset) as *mut u32, fn_addr as u32);
                }

                thunk_idx += 1;
            }

            desc_offset += core::mem::size_of::<ImageImportDescriptor>();
        }

        Ok(())
    }

    /// Searches for the VBHeader in the mapped image.
    ///
    /// In VB6 binaries, the entrypoint instruction is typically:
    /// `push <pointer_to_VBHeader>` (0x68 xx xx xx xx), followed by `call ThunRTMain`.
    pub fn find_vb_header(&self) -> Option<*const VBHeader> {
        let ep = self.entry_point_rva as usize;

        // Check if EntryPoint starts with 0x68 (PUSH imm32)
        if ep + 5 <= self.mapped_size {
            let first_byte = unsafe { *self.mapped_base.add(ep) };
            if first_byte == 0x68 {
                let imm_val = unsafe { core::ptr::read_unaligned(self.mapped_base.add(ep + 1) as *const u32) } as usize;
                
                // If it's already an absolute mapped pointer or a relocated VA
                let candidate_ptr = if imm_val >= (self.mapped_base as usize) && imm_val < (self.mapped_base as usize + self.mapped_size) {
                    imm_val as *const VBHeader
                } else if imm_val >= self.preferred_base && imm_val < (self.preferred_base + self.mapped_size) {
                    let rva = imm_val - self.preferred_base;
                    unsafe { self.mapped_base.add(rva) as *const VBHeader }
                } else {
                    core::ptr::null()
                };

                if !candidate_ptr.is_null() {
                    let candidate_hdr = unsafe { core::ptr::read_unaligned(candidate_ptr) };
                    if candidate_hdr.is_valid() {
                        return Some(candidate_ptr);
                    }
                }
            }
        }

        // Fallback: Scan memory for magic signature "VB5!" (0x21354256)
        for offset in (0..self.mapped_size.saturating_sub(core::mem::size_of::<VBHeader>())).step_by(4) {
            let candidate_ptr = unsafe { self.mapped_base.add(offset) as *const VBHeader };
            let candidate_hdr = unsafe { core::ptr::read_unaligned(candidate_ptr) };
            if candidate_hdr.is_valid() {
                return Some(candidate_ptr);
            }
        }

        None
    }

    /// Executes the Visual Basic runtime directly using the discovered VBHeader with default (Headless) mode.
    pub fn execute(&self) -> Result<i32, String> {
        self.execute_with_mode(ExecutionMode::Headless)
    }

    /// Executes the Visual Basic runtime directly using the discovered VBHeader with the specified execution mode.
    pub fn execute_with_mode(&self, mode: ExecutionMode) -> Result<i32, String> {
        if let Some(vb_hdr) = self.find_vb_header() {
            set_execution_mode(mode);
            unsafe {
                println!("[vb6-run] VBHeader located at {:p}. Invoking ThunRTMain (mode: {:?})...", vb_hdr, mode);
                let res = ThunRTMain(vb_hdr);
                Ok(res)
            }
        } else {
            Err("Failed to locate a valid VBHeader ('VB5!' magic) in the executable image".to_string())
        }
    }
}

impl Drop for LoadedPe {
    fn drop(&mut self) {
        if !self.mapped_base.is_null() {
            unsafe {
                free_executable_memory(self.mapped_base, self.mapped_size);
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Virtual Memory Allocation Helpers
// ----------------------------------------------------------------------------

/// Allocates executable virtual memory pages using `mmap(2)` on Unix systems.
///
/// Attempts to map memory at `preferred_base` first to minimize relocation overhead,
/// falling back to arbitrary addresses if the preferred address is occupied.
///
/// # Arguments
/// * `preferred_base` - Preferred virtual address base (e.g. `0x00400000`).
/// * `size` - Total size of the PE image in bytes.
///
/// # Returns
/// Pointer to allocated memory buffer, or `PeError::AllocationFailed`.
#[cfg(unix)]
unsafe fn allocate_executable_memory(preferred_base: usize, size: usize) -> Result<*mut u8, PeError> {
    use libc::{mmap, MAP_ANON, MAP_PRIVATE, PROT_EXEC, PROT_READ, PROT_WRITE};

    // Attempt allocation at preferred base first
    let mut addr = mmap(
        preferred_base as *mut libc::c_void,
        size,
        PROT_READ | PROT_WRITE | PROT_EXEC,
        MAP_PRIVATE | MAP_ANON,
        -1,
        0,
    );

    if addr == libc::MAP_FAILED {
        // Fallback: allocate anywhere with RWX
        addr = mmap(
            core::ptr::null_mut(),
            size,
            PROT_READ | PROT_WRITE | PROT_EXEC,
            MAP_PRIVATE | MAP_ANON,
            -1,
            0,
        );
    }

    if addr == libc::MAP_FAILED {
        // On macOS (Apple Silicon), W^X disallows anonymous PROT_WRITE | PROT_EXEC without MAP_JIT.
        // Fall back to PROT_READ | PROT_WRITE.
        addr = mmap(
            core::ptr::null_mut(),
            size,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANON,
            -1,
            0,
        );
    }

    if addr == libc::MAP_FAILED {
        Err(PeError::AllocationFailed)
    } else {
        // Zero memory
        core::ptr::write_bytes(addr as *mut u8, 0, size);
        Ok(addr as *mut u8)
    }
}

/// Frees virtual memory pages allocated with `mmap(2)` on Unix systems.
///
/// # Arguments
/// * `addr` - Pointer to base of mapped memory.
/// * `size` - Total size of the mapped memory region.
#[cfg(unix)]
unsafe fn free_executable_memory(addr: *mut u8, size: usize) {
    libc::munmap(addr as *mut libc::c_void, size);
}

/// Allocates executable virtual memory pages using `VirtualAlloc` on Windows systems.
///
/// # Arguments
/// * `preferred_base` - Preferred virtual address base.
/// * `size` - Total size of the PE image in bytes.
///
/// # Returns
/// Pointer to allocated memory buffer, or `PeError::AllocationFailed`.
#[cfg(windows)]
unsafe fn allocate_executable_memory(preferred_base: usize, size: usize) -> Result<*mut u8, PeError> {
    // Windows VirtualAlloc
    extern "system" {
        fn VirtualAlloc(lp_address: *mut core::ffi::c_void, dw_size: usize, fl_allocation_type: u32, fl_protect: u32) -> *mut core::ffi::c_void;
    }
    const MEM_COMMIT: u32 = 0x00001000;
    const MEM_RESERVE: u32 = 0x00002000;
    const PAGE_EXECUTE_READWRITE: u32 = 0x40;

    let mut addr = VirtualAlloc(preferred_base as *mut _, size, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
    if addr.is_null() {
        addr = VirtualAlloc(core::ptr::null_mut(), size, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
    }

    if addr.is_null() {
        Err(PeError::AllocationFailed)
    } else {
        Ok(addr as *mut u8)
    }
}

/// Frees virtual memory pages using `VirtualFree` on Windows systems.
///
/// # Arguments
/// * `addr` - Pointer to base of allocated memory.
/// * `_size` - Size parameter.
#[cfg(windows)]
unsafe fn free_executable_memory(addr: *mut u8, _size: usize) {
    extern "system" {
        fn VirtualFree(lp_address: *mut core::ffi::c_void, dw_size: usize, dw_free_type: u32) -> i32;
    }
    const MEM_RELEASE: u32 = 0x00008000;
    VirtualFree(addr as *mut _, 0, MEM_RELEASE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pe_loader_with_synthetic_vb_binary() {
        // Build a synthetic minimal valid PE32 in memory
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
}

