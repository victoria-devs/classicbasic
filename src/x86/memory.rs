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
    /// Bump allocator pointer for dynamic heap allocations
    pub heap_ptr: u32,
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
            heap_ptr: 0x00800000,
        }
    }
}

impl X86Memory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocates `size` bytes from the emulator 32-bit heap and returns its virtual address.
    pub fn alloc_bytes(&mut self, size: usize) -> u32 {
        let aligned = (size + 3) & !3;
        let ptr = self.heap_ptr;
        self.heap_ptr = self.heap_ptr.wrapping_add(aligned as u32);
        self.allocate_pages(ptr, aligned);
        ptr
    }

    /// Allocates a 32-bit BSTR from the given UTF-16 slice.
    pub fn alloc_bstr(&mut self, text: &[u16]) -> u32 {
        let byte_len = (text.len() * 2) as u32;
        let raw = self.alloc_bytes(4 + byte_len as usize + 2);
        self.write_u32(raw, byte_len);
        let bstr_ptr = raw + 4;
        for (i, &ch) in text.iter().enumerate() {
            self.write_u16(bstr_ptr.wrapping_add(i as u32 * 2), ch);
        }
        self.write_u16(bstr_ptr.wrapping_add(byte_len), 0);
        bstr_ptr
    }

    /// Reads a 64-bit unsigned integer from emulator memory at `addr`.
    pub fn read_u64(&self, addr: u32) -> u64 {
        let lo = self.read_u32(addr) as u64;
        let hi = self.read_u32(addr.wrapping_add(4)) as u64;
        lo | (hi << 32)
    }

    /// Writes a 64-bit unsigned integer to emulator memory at `addr`.
    pub fn write_u64(&mut self, addr: u32, val: u64) {
        self.write_u32(addr, val as u32);
        self.write_u32(addr.wrapping_add(4), (val >> 32) as u32);
    }

    /// Reads a 32-bit BSTR from emulator memory as a Rust String.
    pub fn read_bstr_string(&self, bstr_ptr: u32) -> String {
        let chars = self.read_bstr(bstr_ptr);
        String::from_utf16_lossy(&chars)
    }

    /// Allocates a 32-bit BSTR from a Rust &str.
    pub fn alloc_bstr_from_str(&mut self, s: &str) -> u32 {
        let chars: Vec<u16> = s.encode_utf16().collect();
        self.alloc_bstr(&chars)
    }

    /// Reads a 16-byte Variant from emulator memory at `addr`.
    pub fn read_variant(&self, addr: u32) -> crate::types::Variant {
        if addr == 0 {
            return crate::types::Variant::empty();
        }
        let vt = self.read_u16(addr);
        let reserved1 = self.read_u16(addr.wrapping_add(2));
        let reserved2 = self.read_u16(addr.wrapping_add(4));
        let reserved3 = self.read_u16(addr.wrapping_add(6));
        let mut data = [0u8; 8];
        for i in 0..8 {
            data[i] = self.read_u8(addr.wrapping_add(8 + i as u32));
        }
        crate::types::Variant {
            vt,
            w_reserved1: reserved1,
            w_reserved2: reserved2,
            w_reserved3: reserved3,
            data,
        }
    }

    /// Reads a 16-byte Variant from emulator memory at `addr` and dereferences any VT_BYREF pointers.
    pub fn read_variant_deref(&self, addr: u32) -> crate::types::Variant {
        let mut v = self.read_variant(addr);
        while (v.vt & crate::types::variant::VT_BYREF) != 0 {
            let ref_ptr = u32::from_le_bytes(v.data[..4].try_into().unwrap());
            if ref_ptr == 0 {
                break;
            }
            let base_vt = v.vt & !crate::types::variant::VT_BYREF;
            if base_vt == crate::types::VT_VARIANT {
                v = self.read_variant(ref_ptr);
            } else {
                v.vt = base_vt;
                match base_vt {
                    crate::types::VT_I1 | crate::types::VT_UI1 => {
                        let b = self.read_u8(ref_ptr);
                        v.data = [0u8; 8];
                        v.data[0] = b;
                    }
                    crate::types::VT_I2 | crate::types::VT_UI2 | crate::types::VT_BOOL => {
                        let w = self.read_u16(ref_ptr);
                        v.data = [0u8; 8];
                        v.data[..2].copy_from_slice(&w.to_le_bytes());
                    }
                    crate::types::VT_I4
                    | crate::types::VT_UI4
                    | crate::types::VT_ERROR
                    | crate::types::VT_BSTR
                    | crate::types::VT_DISPATCH
                    | crate::types::VT_UNKNOWN => {
                        let d = self.read_u32(ref_ptr);
                        v.data = [0u8; 8];
                        v.data[..4].copy_from_slice(&d.to_le_bytes());
                    }
                    crate::types::VT_R4 => {
                        let d = self.read_u32(ref_ptr);
                        v.data = [0u8; 8];
                        v.data[..4].copy_from_slice(&d.to_le_bytes());
                    }
                    crate::types::VT_R8 | crate::types::VT_DATE | crate::types::VT_CY => {
                        let q = self.read_u64(ref_ptr);
                        v.data = q.to_ne_bytes();
                    }
                    _ => break,
                }
                break;
            }
        }
        v
    }

    /// Writes a 16-byte Variant to emulator memory at `addr`.
    pub fn write_variant(&mut self, addr: u32, var: &crate::types::Variant) {
        if addr == 0 {
            return;
        }
        self.write_u16(addr, var.vt);
        self.write_u16(addr.wrapping_add(2), var.w_reserved1);
        self.write_u16(addr.wrapping_add(4), var.w_reserved2);
        self.write_u16(addr.wrapping_add(6), var.w_reserved3);
        for i in 0..8 {
            self.write_u8(addr.wrapping_add(8 + i as u32), var.data[i]);
        }
    }

    /// Reads a 32-bit BSTR from emulator memory at `bstr_ptr`.
    pub fn read_bstr(&self, bstr_ptr: u32) -> Vec<u16> {
        if bstr_ptr == 0 {
            return Vec::new();
        }
        let byte_len = self.read_u32(bstr_ptr.wrapping_sub(4));
        let char_len = (byte_len / 2) as usize;
        let mut chars = Vec::with_capacity(char_len);
        for i in 0..char_len {
            chars.push(self.read_u16(bstr_ptr.wrapping_add(i as u32 * 2)));
        }
        chars
    }

    /// Allocates an ANSI C-string (null-terminated) in emulator heap.
    pub fn alloc_ansi(&mut self, text: &[u8]) -> u32 {
        let raw = self.alloc_bytes(text.len() + 1);
        for (i, &b) in text.iter().enumerate() {
            self.write_u8(raw.wrapping_add(i as u32), b);
        }
        self.write_u8(raw.wrapping_add(text.len() as u32), 0);
        raw
    }

    /// Reads a null-terminated ANSI string from emulator memory at `ptr`.
    pub fn read_ansi(&self, ptr: u32) -> Vec<u8> {
        if ptr == 0 {
            return Vec::new();
        }
        let mut bytes = Vec::new();
        let mut p = ptr;
        loop {
            let b = self.read_u8(p);
            if b == 0 {
                break;
            }
            bytes.push(b);
            p = p.wrapping_add(1);
        }
        bytes
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
            #[cfg(feature = "std")]
            unsafe {
                let pe_base = crate::pe::shims::GLOBAL_MODULE_HANDLE;
                let pe_size = crate::pe::shims::GLOBAL_MODULE_SIZE;
                if pe_base != 0 && (addr as usize) >= pe_base && (addr as usize) < pe_base + pe_size {
                    return *(addr as *const u8);
                }
            }
            let page_addr = addr & !0xFFF;
            if let Some(page) = self.pages.get(&page_addr) {
                let offset = (addr & 0xFFF) as usize;
                page[offset]
            } else {
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
            #[cfg(feature = "std")]
            unsafe {
                let pe_base = crate::pe::shims::GLOBAL_MODULE_HANDLE;
                let pe_size = crate::pe::shims::GLOBAL_MODULE_SIZE;
                if pe_base != 0 && (addr as usize) >= pe_base && (addr as usize) < pe_base + pe_size {
                    let page_start = (addr as usize) & !0xFFF;
                    libc::mprotect(
                        page_start as *mut libc::c_void,
                        4096,
                        libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
                    );
                    *(addr as *mut u8) = val;
                    return;
                }
            }
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
