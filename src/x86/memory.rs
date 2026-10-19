// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! 32-bit Flat Virtual Address Space with Win32 TIB & Stack.

use std::collections::BTreeMap;

pub const STACK_BASE: u32 = 0x00200000;
pub const STACK_LIMIT: u32 = 0x00100000; // 1MB Stack
pub const STACK_SIZE: usize = (STACK_BASE - STACK_LIMIT) as usize;

pub const TIB_BASE: u32 = 0x000F0000;
pub const TIB_SIZE: usize = 0x1000; // 4KB Page

/// Flat 32-bit virtual memory subsystem for the x86 CPU emulator.
pub struct X86Memory {
    /// 1MB stack memory [0x00100000 .. 0x00200000)
    pub stack: Vec<u8>,
    /// 4KB Win32 Thread Information Block (TIB) at 0x000F0000
    pub tib: [u8; TIB_SIZE],
    /// Dynamically mapped pages (4KB aligned)
    pub pages: BTreeMap<u32, Vec<u8>>,
    /// Read-only or mapped code buffer
    pub code_base: u32,
    pub code_data: Vec<u8>,
}

impl Default for X86Memory {
    fn default() -> Self {
        let mut tib = [0u8; TIB_SIZE];
        // Win32 TIB layout:
        // FS:[0x00] = ExceptionList (pointer to EXCEPTION_REGISTRATION_RECORD) -> 0xFFFFFFFF (End of list)
        tib[0..4].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes());
        // FS:[0x04] = StackBase
        tib[4..8].copy_from_slice(&STACK_BASE.to_le_bytes());
        // FS:[0x08] = StackLimit
        tib[8..12].copy_from_slice(&STACK_LIMIT.to_le_bytes());
        // FS:[0x18] = Self (Linear address of TIB)
        tib[0x18..0x1C].copy_from_slice(&TIB_BASE.to_le_bytes());
        // FS:[0x20] = Process ID
        tib[0x20..0x24].copy_from_slice(&1000u32.to_le_bytes());
        // FS:[0x24] = Thread ID
        tib[0x24..0x28].copy_from_slice(&1u32.to_le_bytes());

        Self {
            stack: vec![0u8; STACK_SIZE],
            tib,
            pages: BTreeMap::new(),
            code_base: 0x00400000,
            code_data: Vec::new(),
        }
    }
}

impl X86Memory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads code bytes into the virtual memory pages at the specified address.
    pub fn load_code(&mut self, base: u32, code: &[u8]) {
        for (i, &b) in code.iter().enumerate() {
            self.write_u8(base.wrapping_add(i as u32), b);
        }
    }

    /// Allocates a virtual memory page block of given size at or above base.
    pub fn allocate_pages(&mut self, base: u32, size: usize) -> u32 {
        let aligned_base = base & !0xFFF;
        let mut current = aligned_base;
        let end = aligned_base + ((size + 0xFFF) & !0xFFF) as u32;
        while current < end {
            self.pages.entry(current).or_insert_with(|| vec![0u8; 4096]);
            current += 4096;
        }
        base
    }

    /// Translates an address to a mutable reference or slice.
    fn get_page_mut(&mut self, addr: u32) -> &mut [u8] {
        let page_addr = addr & !0xFFF;
        self.pages.entry(page_addr).or_insert_with(|| vec![0u8; 4096])
    }

    /// Reads a single byte at virtual address `addr`.
    pub fn read_u8(&self, addr: u32) -> u8 {
        if addr >= STACK_LIMIT && addr < STACK_BASE {
            let offset = (addr - STACK_LIMIT) as usize;
            self.stack[offset]
        } else if addr >= TIB_BASE && addr < TIB_BASE + TIB_SIZE as u32 {
            let offset = (addr - TIB_BASE) as usize;
            self.tib[offset]
        } else {
            let page_addr = addr & !0xFFF;
            if let Some(page) = self.pages.get(&page_addr) {
                let offset = (addr & 0xFFF) as usize;
                page[offset]
            } else {
                // If mapped to host PE image memory
                #[cfg(feature = "std")]
                unsafe {
                    let pe_base = crate::pe::shims::GLOBAL_MODULE_HANDLE;
                    let pe_size = crate::pe::shims::GLOBAL_MODULE_SIZE;
                    if pe_base != 0 && (addr as usize) >= pe_base && (addr as usize) < pe_base + pe_size {
                        return *(addr as *const u8);
                    }
                }
                0
            }
        }
    }

    /// Writes a single byte at virtual address `addr`.
    pub fn write_u8(&mut self, addr: u32, val: u8) {
        if addr >= STACK_LIMIT && addr < STACK_BASE {
            let offset = (addr - STACK_LIMIT) as usize;
            self.stack[offset] = val;
        } else if addr >= TIB_BASE && addr < TIB_BASE + TIB_SIZE as u32 {
            let offset = (addr - TIB_BASE) as usize;
            self.tib[offset] = val;
        } else {
            let page = self.get_page_mut(addr);
            let offset = (addr & 0xFFF) as usize;
            page[offset] = val;
        }
    }

    pub fn read_u16(&self, addr: u32) -> u16 {
        let b0 = self.read_u8(addr) as u16;
        let b1 = self.read_u8(addr.wrapping_add(1)) as u16;
        b0 | (b1 << 8)
    }

    pub fn write_u16(&mut self, addr: u32, val: u16) {
        self.write_u8(addr, (val & 0xFF) as u8);
        self.write_u8(addr.wrapping_add(1), ((val >> 8) & 0xFF) as u8);
    }

    pub fn read_u32(&self, addr: u32) -> u32 {
        let b0 = self.read_u8(addr) as u32;
        let b1 = self.read_u8(addr.wrapping_add(1)) as u32;
        let b2 = self.read_u8(addr.wrapping_add(2)) as u32;
        let b3 = self.read_u8(addr.wrapping_add(3)) as u32;
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    pub fn write_u32(&mut self, addr: u32, val: u32) {
        let bytes = val.to_le_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            self.write_u8(addr.wrapping_add(i as u32), b);
        }
    }

    pub fn read_i32(&self, addr: u32) -> i32 {
        self.read_u32(addr) as i32
    }

    pub fn write_i32(&mut self, addr: u32, val: i32) {
        self.write_u32(addr, val as u32);
    }

    pub fn read_f32(&self, addr: u32) -> f32 {
        f32::from_bits(self.read_u32(addr))
    }

    pub fn write_f32(&mut self, addr: u32, val: f32) {
        self.write_u32(addr, val.to_bits());
    }

    pub fn read_f64(&self, addr: u32) -> f64 {
        let lo = self.read_u32(addr) as u64;
        let hi = self.read_u32(addr.wrapping_add(4)) as u64;
        f64::from_bits(lo | (hi << 32))
    }

    pub fn write_f64(&mut self, addr: u32, val: f64) {
        let bits = val.to_bits();
        self.write_u32(addr, bits as u32);
        self.write_u32(addr.wrapping_add(4), (bits >> 32) as u32);
    }
}
